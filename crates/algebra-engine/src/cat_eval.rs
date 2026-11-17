//! Categorical Multi-Fidelity Evaluator bridging `ExprGraph` (`algebra-core`)
//! with `cat-core`, `cat-bridge` (`Lift ⊣ Lower`), and `cat-anytime` (`FidelityStream`).

use crate::calculus::SymbolicCalculus;
use crate::numeric::{BigValue, EvalContext, NumericalEval};
use crate::simplify::Simplifier;
use algebra_core::{ExprGraph, ExprId, ExprKind, Number};
use cat_anytime::{AnytimeBudget, FidelityStream, MultiFidelityOp, tier0_ball_result};
use cat_bridge::{
    BridgeError, LegacyAstTree, LegacyBridgeContext, MigrationVerifyStatus, RetrofittableAst,
    ShadowVerifyHarness,
};
use cat_core::{
    ApproxResult, CatRuntimeContext, CatTerm, DomainCondition, ErrorEnclosure, EvalForm,
    FidelityMode, UncertaintyKind, UniversalBall,
};
use std::collections::HashMap;
use std::fmt;

/// Error returned by [`CatExprEvaluator`].
#[derive(Debug, Clone)]
pub enum CatEvalError {
    Bridge(String),
    Simplification(String),
    Numeric(String),
}

impl fmt::Display for CatEvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bridge(msg) => write!(f, "cat-bridge error: {msg}"),
            Self::Simplification(msg) => write!(f, "simplification error: {msg}"),
            Self::Numeric(msg) => write!(f, "numeric evaluation error: {msg}"),
        }
    }
}

impl std::error::Error for CatEvalError {}

impl From<BridgeError> for CatEvalError {
    fn from(err: BridgeError) -> Self {
        Self::Bridge(err.to_string())
    }
}

/// Input payload for categorical multi-fidelity expression evaluation.
#[derive(Clone)]
pub struct CatEvalInput {
    pub graph: ExprGraph,
    pub expr: ExprId,
    pub var_hint: Option<String>,
    pub eval_point: Option<f64>,
}

impl CatEvalInput {
    pub fn new(graph: ExprGraph, expr: ExprId) -> Self {
        Self {
            graph,
            expr,
            var_hint: None,
            eval_point: None,
        }
    }

    pub fn with_var(mut self, var: impl Into<String>, point: Option<f64>) -> Self {
        self.var_hint = Some(var.into());
        self.eval_point = point;
        self
    }
}

/// Output payload for categorical multi-fidelity expression evaluation.
/// Note: intentionally omits `#[derive(Debug)]` because `CatTerm` is non-`Debug`.
#[derive(Clone)]
pub struct CatEvalOutput {
    pub graph: ExprGraph,
    pub expr: ExprId,
    pub term: Option<CatTerm>,
    pub roundtrip_status: Option<MigrationVerifyStatus>,
    pub numeric_value: Option<f64>,
}

/// Unified multi-fidelity evaluator routing `ExprGraph` expressions through
/// `cat-bridge` (`Lift ⊣ Lower`), `cat-anytime` (`FidelityStream`), and `Simplifier`.
#[derive(Debug, Default, Clone, Copy)]
pub struct CatExprEvaluator;

impl CatExprEvaluator {
    pub fn new() -> Self {
        Self
    }

    /// Evaluate an expression progressively from Tier-0 numerical ball up to Tier-3
    /// hybrid symbolic canonicalization within `budget`.
    pub fn eval_stream(
        &self,
        input: &CatEvalInput,
        form: EvalForm,
        budget: AnytimeBudget,
    ) -> Result<FidelityStream<CatEvalOutput>, CatEvalError> {
        FidelityStream::run_progressive(self, input, form, budget)
    }

    fn eval_numeric_ball(&self, input: &CatEvalInput) -> (Option<f64>, UniversalBall) {
        let mut bindings = HashMap::new();
        if let (Some(var), Some(pt)) = (&input.var_hint, input.eval_point) {
            bindings.insert(var.clone(), pt);
        }
        let ctx = EvalContext::with_bindings(53, bindings);
        match input.graph.evalf(input.expr, &ctx) {
            Ok(BigValue::Real(v)) if v.is_finite() => {
                let rad = v.abs() * 1e-12 + 1e-15;
                (Some(v), UniversalBall::real_with_radius(v, rad))
            }
            Ok(BigValue::Complex(re, im)) if re.is_finite() && im.is_finite() => {
                let rad = (re.hypot(im)) * 1e-12 + 1e-15;
                (
                    Some(re),
                    UniversalBall::ComplexDisk {
                        center_re: re,
                        center_im: im,
                        radius: rad,
                    },
                )
            }
            _ => (
                None,
                UniversalBall::RealInterval {
                    min: f64::NEG_INFINITY,
                    max: f64::INFINITY,
                },
            ),
        }
    }

    fn is_pure_constant_zero(graph: &ExprGraph, id: ExprId) -> bool {
        match &graph.get(id).kind {
            ExprKind::Number(Number::Integer(0)) => true,
            ExprKind::Number(Number::Float(bits)) => f64::from_bits(*bits) == 0.0,
            _ => false,
        }
    }
}

impl MultiFidelityOp<CatEvalInput, CatEvalOutput> for CatExprEvaluator {
    type Error = CatEvalError;

    /// Evaluate at the requested fidelity and report whether it overran `budget.hard_timeout`.
    ///
    /// A tier cannot be interrupted part-way (simplification and bridge verification run to
    /// completion), so the budget is enforced as a measurement: the result is returned in full
    /// and `budget_exceeded` says that it took longer than the hard timeout, letting a scheduler
    /// stop escalating to higher tiers.
    fn eval_with_fidelity(
        &self,
        input: &CatEvalInput,
        mode: FidelityMode,
        form: EvalForm,
        budget: &AnytimeBudget,
    ) -> Result<ApproxResult<CatEvalOutput>, Self::Error> {
        let started = std::time::Instant::now();
        let mut result = self.evaluate_tier(input, mode, form)?;
        if started.elapsed() > budget.hard_timeout {
            result.budget_exceeded = true;
        }
        Ok(result)
    }
}

impl CatExprEvaluator {
    fn evaluate_tier(
        &self,
        input: &CatEvalInput,
        mode: FidelityMode,
        form: EvalForm,
    ) -> Result<ApproxResult<CatEvalOutput>, CatEvalError> {
        match mode {
            FidelityMode::NumericalApprox => {
                let (numeric_value, ball) = self.eval_numeric_ball(input);
                let out = CatEvalOutput {
                    graph: input.graph.clone(),
                    expr: input.expr,
                    term: None,
                    roundtrip_status: None,
                    numeric_value,
                };
                Ok(tier0_ball_result(out, ball, "expr_graph::evalf"))
            }
            FidelityMode::SymbolicApprox => {
                if form == EvalForm::Hold {
                    return Ok(ApproxResult {
                        value: CatEvalOutput {
                            graph: input.graph.clone(),
                            expr: input.expr,
                            term: None,
                            roundtrip_status: None,
                            numeric_value: None,
                        },
                        mode_used: FidelityMode::SymbolicApprox,
                        form: EvalForm::Hold,
                        enclosure: ErrorEnclosure::exact("hold"),
                        remainder_term: None,
                        validity: DomainCondition::default(),
                        budget_exceeded: false,
                    });
                }

                let graph = input.graph.clone();
                let (approx_expr, linearised) = if let Some(var) = &input.var_hint {
                    let pt = input.eval_point.unwrap_or(0.0);
                    let pt_expr = graph.float(pt);
                    let x_sym = graph.symbol(var);
                    if let ExprKind::Symbol(sym_id) = graph.get(x_sym).kind {
                        let f_a = graph.substitute(input.expr, sym_id, pt_expr);
                        let df = graph.diff(input.expr, sym_id);
                        let df_a = graph.substitute(df, sym_id, pt_expr);
                        let dx = graph.sub(x_sym, pt_expr);
                        let term = graph.mul([df_a, dx]);
                        let lin = graph.add([f_a, term]);
                        (Simplifier::simplify(&graph, lin).unwrap_or(lin), true)
                    } else {
                        (
                            Simplifier::simplify(&graph, input.expr).unwrap_or(input.expr),
                            false,
                        )
                    }
                } else {
                    (
                        Simplifier::simplify(&graph, input.expr).unwrap_or(input.expr),
                        false,
                    )
                };

                let out = CatEvalOutput {
                    graph,
                    expr: approx_expr,
                    term: None,
                    roundtrip_status: None,
                    numeric_value: None,
                };
                Ok(ApproxResult {
                    value: out,
                    mode_used: FidelityMode::SymbolicApprox,
                    form: EvalForm::Canonical,
                    // The tangent line f(a) + f'(a)(x - a) leaves a remainder O((x - a)^2); with no
                    // expansion variable the expression is only simplified, which is exact.
                    enclosure: if linearised {
                        ErrorEnclosure {
                            kind: UncertaintyKind::Asymptotic {
                                order: 2,
                                constant_bound: None,
                            },
                            abs_err: None,
                            rel_err: None,
                            method: "taylor_order_1".to_string(),
                        }
                    } else {
                        ErrorEnclosure::exact("simplify")
                    },
                    remainder_term: None,
                    validity: DomainCondition {
                        operating_point: Some(format!(
                            "{}={}",
                            input.var_hint.as_deref().unwrap_or("_"),
                            input.eval_point.unwrap_or(0.0)
                        )),
                        assumptions: Vec::new(),
                        roc_summary: Some("local analytic neighborhood".to_string()),
                    },
                    budget_exceeded: false,
                })
            }
            FidelityMode::SymbolicExact => {
                let mut cat_ctx = CatRuntimeContext::default();
                let mut bridge_ctx = LegacyBridgeContext::default();
                let legacy = LegacyAstTree::from_graph(input.expr, &input.graph)?;
                let term = legacy.lift_to_cat(&mut cat_ctx, &mut bridge_ctx)?;
                let roundtrip_status =
                    ShadowVerifyHarness::verify_roundtrip(&legacy, &mut cat_ctx, &mut bridge_ctx)?;

                let out_expr = if form == EvalForm::Hold {
                    input.expr
                } else {
                    Simplifier::simplify(&input.graph, input.expr)
                        .map_err(|e| CatEvalError::Simplification(e.to_string()))?
                };

                Ok(ApproxResult {
                    value: CatEvalOutput {
                        graph: input.graph.clone(),
                        expr: out_expr,
                        term: Some(term),
                        roundtrip_status: Some(roundtrip_status),
                        numeric_value: None,
                    },
                    mode_used: FidelityMode::SymbolicExact,
                    form,
                    enclosure: ErrorEnclosure::exact(if form == EvalForm::Hold {
                        "cat_bridge+hold"
                    } else {
                        "cat_bridge+simplifier"
                    }),
                    remainder_term: None,
                    validity: DomainCondition::default(),
                    budget_exceeded: false,
                })
            }
            FidelityMode::SymbolicHybrid => {
                // Seed with fast Tier-0 check; if already exact zero, short-circuit cleanly.
                let (numeric_value, _ball) = self.eval_numeric_ball(input);
                if form != EvalForm::Hold && Self::is_pure_constant_zero(&input.graph, input.expr) {
                    let mut cat_ctx = CatRuntimeContext::default();
                    let mut bridge_ctx = LegacyBridgeContext::default();
                    let legacy = LegacyAstTree::from_graph(input.expr, &input.graph)?;
                    let term = legacy.lift_to_cat(&mut cat_ctx, &mut bridge_ctx)?;
                    return Ok(ApproxResult {
                        value: CatEvalOutput {
                            graph: input.graph.clone(),
                            expr: input.expr,
                            term: Some(term),
                            roundtrip_status: Some(MigrationVerifyStatus::ExactMatch),
                            numeric_value: Some(0.0),
                        },
                        mode_used: FidelityMode::SymbolicHybrid,
                        form,
                        enclosure: ErrorEnclosure::exact("hybrid_zero_short_circuit"),
                        remainder_term: None,
                        validity: DomainCondition::default(),
                        budget_exceeded: false,
                    });
                }

                let mut exact = self.evaluate_tier(input, FidelityMode::SymbolicExact, form)?;
                exact.mode_used = FidelityMode::SymbolicHybrid;
                exact.value.numeric_value = numeric_value;
                exact.enclosure.method = "hybrid_ball+cat_bridge+simplifier".to_string();
                Ok(exact)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use algebra_core::ExprParser;

    #[test]
    fn test_cat_eval_four_fidelities_and_hold() {
        let graph = ExprGraph::new();
        let expr = ExprParser::new(&graph).parse("(x + 1) * 2").expect("parse");
        let input = CatEvalInput::new(graph, expr).with_var("x", Some(3.0));
        let evaluator = CatExprEvaluator::new();
        let budget = AnytimeBudget::default();

        let r0 = evaluator
            .eval_with_fidelity(
                &input,
                FidelityMode::NumericalApprox,
                EvalForm::Canonical,
                &budget,
            )
            .expect("tier 0");
        assert_eq!(r0.mode_used, FidelityMode::NumericalApprox);
        assert!((r0.value.numeric_value.expect("val") - 8.0).abs() < 1e-12);

        let r1 = evaluator
            .eval_with_fidelity(
                &input,
                FidelityMode::SymbolicApprox,
                EvalForm::Canonical,
                &budget,
            )
            .expect("tier 1");
        assert_eq!(r1.mode_used, FidelityMode::SymbolicApprox);
        assert!(matches!(
            r1.enclosure.kind,
            UncertaintyKind::Asymptotic { order: 2, .. }
        ));

        let r2 = evaluator
            .eval_with_fidelity(
                &input,
                FidelityMode::SymbolicExact,
                EvalForm::Canonical,
                &budget,
            )
            .expect("tier 2");
        assert_eq!(r2.mode_used, FidelityMode::SymbolicExact);
        assert!(r2.value.term.is_some());
        assert_eq!(
            r2.value.roundtrip_status,
            Some(MigrationVerifyStatus::ExactMatch)
        );

        let r_hold = evaluator
            .eval_with_fidelity(&input, FidelityMode::SymbolicExact, EvalForm::Hold, &budget)
            .expect("hold");
        assert_eq!(r_hold.form, EvalForm::Hold);
        assert_eq!(r_hold.value.expr, expr);

        let stream = evaluator
            .eval_stream(&input, EvalForm::Canonical, budget)
            .expect("stream");
        assert!(stream.completed);
        assert!(!stream.stages.is_empty());
        assert_eq!(
            stream.latest().expect("latest").mode_used,
            FidelityMode::SymbolicHybrid
        );
    }
}
