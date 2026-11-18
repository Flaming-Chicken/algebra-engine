//! # `algebra_engine::executor`
//!
//! Central Mathematical Operation Dispatcher & Execution Engine.
//!
//! Executes any `MathOperation` AST against an `ExprGraph`, handling simplification,
//! calculus, solvers, linear algebra, transforms, polynomials, discrete combinatorics,
//! physics units, certified intervals, logic, number theory, topology, control systems,
//! and esoteric hyperoperations.

use crate::logic::SatSolver;
use crate::matrix::SymbolicMatrix;
use crate::poly::{
    UniPoly, collect_expr_symbols, expr_from_rational, symbolic_discriminant, symbolic_resultant,
};
use algebra_core::format::{Formatter, LatexFormatter, UnicodeFormatter};
use algebra_core::interval::RealInterval;
use algebra_core::operation::*;
use algebra_core::parser::ExprParser;
use algebra_core::{ExprGraph, ExprId, ExprKind};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::calculus::SymbolicCalculus;
use crate::cartan::{DifferentialForm, VectorField};
use crate::cartan_symbolic::SymbolicForm;
use crate::combinatorics::{
    bell_number, catalan_number, combinations, derangements, factorial, integer_partitions,
    permutations, stirling_second_kind,
};
use crate::control::StateSpaceSystem;
use crate::diffeq::{DiffEqClassifier, DiffEqKind};
use crate::hyperop::{ackermann, hyperoperation, knuth_up_arrow, pentation, super_log, tetration};
use crate::numbertheory::{
    ContinuedFraction, decompose_universal, extended_gcd, is_prime, legendre_symbol,
};
use crate::numeric::{EvalContext, NumericalEval};
use crate::simplify::{AlgebraicTransformations, Simplifier, SymbolicSimplifier};
use crate::solver::SymbolicSolver;
use crate::systems::{LinearSystemSolver, PolynomialSystemSolver};
use crate::topology::SimplicialComplex;
use crate::transforms::SymbolicTransforms;
use crate::tropical::{MaxPlus, MinPlus, TropicalMatrix, log_sum_exp};
use algebra_core::probabilistic::ProbabilisticVerifier;

/// Execution environment context (variable bindings, parameters, uncertainty, precision).
#[derive(Debug, Clone, Default)]
pub struct ExecutionContext {
    /// Active variable & parameter bindings for numerical evaluation.
    pub bindings: HashMap<String, f64>,
    /// Active 1-sigma uncertainties for declared parameters.
    pub uncertainties: HashMap<String, f64>,
    /// Active certified intervals for declared parameters.
    pub intervals: HashMap<String, RealInterval>,
    /// Active logical/domain assumptions on symbols (e.g. `x > 0`, `n in Integers`).
    pub assumptions: crate::logic::AssumptionsContext,
    /// Optional active Riemann sheet index `k` (None => principal branch `k = 0`).
    pub branch_sheet: Option<i64>,
    /// Summary of notebook variables: `(name, role, val, unit)`
    pub symbol_summary: Vec<(String, String, f64, String)>,
    /// Desired numerical decimal precision.
    pub precision_digits: usize,
}

impl ExecutionContext {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            uncertainties: HashMap::new(),
            intervals: HashMap::new(),
            assumptions: crate::logic::AssumptionsContext::new(),
            branch_sheet: None,
            symbol_summary: Vec::new(),
            precision_digits: 50,
        }
    }

    pub fn with_bindings(bindings: HashMap<String, f64>) -> Self {
        Self {
            bindings,
            uncertainties: HashMap::new(),
            intervals: HashMap::new(),
            assumptions: crate::logic::AssumptionsContext::new(),
            branch_sheet: None,
            symbol_summary: Vec::new(),
            precision_digits: 50,
        }
    }
}

/// Structured result output from an executed operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult {
    /// Primary human-readable formatted output.
    pub output_text: String,
    /// Rendered LaTeX representation.
    pub output_latex: String,
    /// Rendered 2D Unicode math representation.
    pub output_unicode: String,
    /// E-Graph or transformation simplified Unicode representation.
    pub simplified_unicode: Option<String>,
    /// Underlying primary AST expression handle.
    pub root_expr: Option<ExprId>,
    /// Domain and classification information.
    pub domain_info: Option<String>,
    /// Any evaluation warnings or errors.
    pub error_msg: Option<String>,
    /// Whether execution resulted in an error.
    pub is_error: bool,
    /// Key-value diagnostic metadata.
    pub metadata: HashMap<String, String>,
}

impl OperationResult {
    pub fn success(
        text: impl Into<String>,
        latex: impl Into<String>,
        unicode: impl Into<String>,
    ) -> Self {
        let t = text.into();
        let l = latex.into();
        let u = unicode.into();
        Self {
            output_text: t,
            output_latex: l,
            output_unicode: u,
            simplified_unicode: None,
            root_expr: None,
            domain_info: None,
            error_msg: None,
            is_error: false,
            metadata: HashMap::new(),
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        let m = msg.into();
        Self {
            output_text: format!("Error: {}", m),
            output_latex: format!("\\text{{Error: {}}}", m),
            output_unicode: format!("Error: {}", m),
            simplified_unicode: None,
            root_expr: None,
            domain_info: None,
            error_msg: Some(m),
            is_error: true,
            metadata: HashMap::new(),
        }
    }
}

/// Central mathematical operation execution engine.
pub struct OperationExecutor;

impl OperationExecutor {
    /// Execute any `MathOperation` against the provided `ExprGraph`.
    pub fn execute(
        graph: &ExprGraph,
        op: &MathOperation,
        ctx: &ExecutionContext,
    ) -> OperationResult {
        let parser = ExprParser::new(graph);
        let latex_fmt = LatexFormatter;
        let unicode_fmt = UnicodeFormatter;

        match op {
            MathOperation::Expression { raw } => {
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    return OperationResult::success("", "", "");
                }
                match parser.parse(trimmed) {
                    Ok(expr_id) => {
                        let latex = latex_fmt
                            .format(graph, expr_id)
                            .unwrap_or_else(|_| trimmed.to_string());
                        let unicode = unicode_fmt
                            .format(graph, expr_id)
                            .unwrap_or_else(|_| trimmed.to_string());
                        let mut res = OperationResult::success(trimmed, latex, unicode);
                        res.root_expr = Some(expr_id);
                        res
                    }
                    Err(e) => OperationResult::error(e.to_string()),
                }
            }

            MathOperation::Differentiate {
                expression,
                variable,
                order,
                is_total: _,
            } => match parser.parse(expression) {
                Ok(expr_id) => {
                    let mut subbed_id = expr_id;
                    for (param, &val) in &ctx.bindings {
                        if Some(param.as_str()) != variable.as_deref() {
                            let param_sym = graph.symbols.get_or_intern(param);
                            let val_node = graph.float(val);
                            subbed_id = graph.substitute(subbed_id, param_sym, val_node);
                        }
                    }

                    if let Some(var_str) = variable {
                        let wrt_sym = graph.symbols.get_or_intern(var_str);
                        let mut cur_id = subbed_id;
                        for _ in 0..*order {
                            cur_id = graph.diff(cur_id, wrt_sym);
                        }
                        let simp_id = Simplifier::simplify(graph, cur_id)
                            .unwrap_or_else(|_| graph.simplify(cur_id));
                        let latex = latex_fmt.format(graph, simp_id).unwrap_or_default();
                        let unicode = unicode_fmt.format(graph, simp_id).unwrap_or_default();
                        let mut res = OperationResult::success(unicode.clone(), latex, unicode);
                        res.root_expr = Some(simp_id);
                        res.simplified_unicode =
                            Some(unicode_fmt.format(graph, simp_id).unwrap_or_default());
                        res
                    } else {
                        let mut free_sym_ids = std::collections::BTreeSet::new();
                        collect_expr_symbols(graph, subbed_id, &mut free_sym_ids);
                        if free_sym_ids.len() <= 1 {
                            let wrt_sym = free_sym_ids
                                .into_iter()
                                .next()
                                .unwrap_or_else(|| graph.symbols.get_or_intern("x"));
                            let mut cur_id = subbed_id;
                            for _ in 0..*order {
                                cur_id = graph.diff(cur_id, wrt_sym);
                            }
                            let simp_id = Simplifier::simplify(graph, cur_id)
                                .unwrap_or_else(|_| graph.simplify(cur_id));
                            let latex = latex_fmt.format(graph, simp_id).unwrap_or_default();
                            let unicode = unicode_fmt.format(graph, simp_id).unwrap_or_default();
                            let mut res =
                                OperationResult::success(unicode.clone(), latex, unicode.clone());
                            res.root_expr = Some(simp_id);
                            res.simplified_unicode = Some(unicode);
                            res
                        } else {
                            let mut lines = Vec::new();
                            for (i, wrt_sym) in free_sym_ids.iter().enumerate() {
                                let v_name = graph.symbols.resolve(*wrt_sym).unwrap_or_default();
                                let mut cur_id = subbed_id;
                                for _ in 0..*order {
                                    cur_id = graph.diff(cur_id, *wrt_sym);
                                }
                                let simp_id = Simplifier::simplify(graph, cur_id)
                                    .unwrap_or_else(|_| graph.simplify(cur_id));
                                let out_fmt =
                                    unicode_fmt.format(graph, simp_id).unwrap_or_default();
                                lines.push(format!(
                                    "  [{}] d/d{} [{}] = {}",
                                    i + 1,
                                    v_name,
                                    expression,
                                    out_fmt
                                ));
                            }
                            let out =
                                format!("Derivatives for {}:\n{}", expression, lines.join("\n"));
                            OperationResult::success(out.clone(), format!("\\text{{{}}}", out), out)
                        }
                    }
                }
                Err(e) => OperationResult::error(e.to_string()),
            },

            MathOperation::Integrate {
                expression,
                variable,
                lower,
                upper,
            } => match parser.parse(expression) {
                Ok(expr_id) => {
                    let mut subbed_id = expr_id;
                    for (param, &val) in &ctx.bindings {
                        if Some(param.as_str()) != variable.as_deref() {
                            let param_sym = graph.symbols.get_or_intern(param);
                            let val_node = graph.float(val);
                            subbed_id = graph.substitute(subbed_id, param_sym, val_node);
                        }
                    }

                    if let Some(var_str) = variable {
                        let wrt_sym = graph.symbols.get_or_intern(var_str);
                        let int_res: Result<_, String> = match (lower, upper) {
                            (Some(l_str), Some(u_str)) => {
                                match (parser.parse(l_str), parser.parse(u_str)) {
                                    (Ok(l_id), Ok(u_id)) => graph
                                        .integrate_definite(subbed_id, wrt_sym, l_id, u_id)
                                        .map_err(|e| e.to_string()),
                                    (Err(e), _) | (_, Err(e)) => Err(e.to_string()),
                                }
                            }
                            _ => graph
                                .integrate(subbed_id, wrt_sym)
                                .map_err(|e| e.to_string()),
                        };
                        match int_res {
                            Ok(int_id) => {
                                let simp_id = graph.simplify(int_id);
                                let latex = latex_fmt.format(graph, simp_id).unwrap_or_default();
                                let unicode =
                                    unicode_fmt.format(graph, simp_id).unwrap_or_default();
                                let bounds_suffix = match (lower, upper) {
                                    (Some(l), Some(u)) => format!(" along [{}, {}]", l, u),
                                    _ => String::new(),
                                };
                                let text =
                                    format!("{} (w.r.t {}{})", unicode, var_str, bounds_suffix);
                                let mut res = OperationResult::success(text, latex, unicode);
                                res.root_expr = Some(simp_id);
                                res
                            }
                            Err(e) => OperationResult::error(e.to_string()),
                        }
                    } else {
                        let mut free_sym_ids = std::collections::BTreeSet::new();
                        collect_expr_symbols(graph, subbed_id, &mut free_sym_ids);
                        if free_sym_ids.len() <= 1 {
                            let wrt_sym = free_sym_ids
                                .into_iter()
                                .next()
                                .unwrap_or_else(|| graph.symbols.get_or_intern("x"));
                            match graph.integrate(subbed_id, wrt_sym) {
                                Ok(int_id) => {
                                    let simp_id = Simplifier::simplify(graph, int_id)
                                        .unwrap_or_else(|_| graph.simplify(int_id));
                                    let unicode =
                                        unicode_fmt.format(graph, simp_id).unwrap_or_default();
                                    let latex =
                                        latex_fmt.format(graph, simp_id).unwrap_or_default();
                                    let wrt_name =
                                        graph.symbols.resolve(wrt_sym).unwrap_or_default();
                                    let out = format!("{} (w.r.t {})", unicode, wrt_name);
                                    let mut res = OperationResult::success(out.clone(), latex, out);
                                    res.root_expr = Some(simp_id);
                                    res
                                }
                                Err(e) => OperationResult::error(e.to_string()),
                            }
                        } else {
                            let mut lines = Vec::new();
                            for (i, wrt_sym) in free_sym_ids.iter().enumerate() {
                                let v_name = graph.symbols.resolve(*wrt_sym).unwrap_or_default();
                                if let Ok(int_id) = graph.integrate(subbed_id, *wrt_sym) {
                                    let simp_id = Simplifier::simplify(graph, int_id)
                                        .unwrap_or_else(|_| graph.simplify(int_id));
                                    let out_fmt =
                                        unicode_fmt.format(graph, simp_id).unwrap_or_default();
                                    lines.push(format!(
                                        "  [{}] ∫ {} d{} = {}",
                                        i + 1,
                                        expression,
                                        v_name,
                                        out_fmt
                                    ));
                                }
                            }
                            let out =
                                format!("Integrals for {}:\n{}", expression, lines.join("\n"));
                            OperationResult::success(out.clone(), format!("\\text{{{}}}", out), out)
                        }
                    }
                }
                Err(e) => OperationResult::error(e.to_string()),
            },

            MathOperation::Solve { equation, variable } => {
                let var_name = variable.as_deref().unwrap_or("x");
                let wrt_sym = graph.symbols.get_or_intern(var_name);
                match parser.parse(equation) {
                    Ok(expr_id) => match graph.solveset(expr_id, wrt_sym) {
                        Ok(solutions) => {
                            let eval_ctx = crate::numeric::EvalContext::default();
                            let filtered: Vec<ExprId> = solutions
                                .into_iter()
                                .map(|sol| {
                                    Simplifier::simplify_with_assumptions(
                                        graph,
                                        sol,
                                        &ctx.assumptions,
                                    )
                                    .unwrap_or(sol)
                                })
                                .filter(|&sol| {
                                    solution_satisfies_assumptions(
                                        graph,
                                        sol,
                                        wrt_sym,
                                        &ctx.assumptions,
                                        &eval_ctx,
                                    )
                                })
                                .collect();
                            let mut sol_strs = Vec::new();
                            let mut sol_latex = Vec::new();
                            for &sol in &filtered {
                                sol_strs.push(unicode_fmt.format(graph, sol).unwrap_or_default());
                                sol_latex.push(latex_fmt.format(graph, sol).unwrap_or_default());
                            }
                            let out =
                                format!("Solutions for {}: [{}]", var_name, sol_strs.join(", "));
                            let latex = if sol_latex.is_empty() {
                                format!("{} \\in \\emptyset", var_name)
                            } else {
                                format!(
                                    "{} \\in \\left\\{{ {} \\right\\}}",
                                    var_name,
                                    sol_latex.join(", ")
                                )
                            };
                            let mut res = OperationResult::success(out.clone(), latex, out);
                            if filtered.len() == 1 {
                                res.root_expr = Some(filtered[0]);
                            }
                            res
                        }
                        Err(e) => OperationResult::error(e.to_string()),
                    },
                    Err(e) => OperationResult::error(e.to_string()),
                }
            }

            MathOperation::SolveSystem {
                equations,
                variables,
            } => execute_solve_system(graph, equations, variables, ctx),

            MathOperation::Simplify {
                expression,
                strategy,
            } => match parser.parse(expression) {
                Ok(expr_id) => {
                    let transformed_id = match strategy {
                        SimplifyStrategy::Standard => {
                            Simplifier::simplify_with_assumptions(graph, expr_id, &ctx.assumptions)
                                .unwrap_or(expr_id)
                        }
                        SimplifyStrategy::Expand => {
                            AlgebraicTransformations::expand(graph, expr_id)
                        }
                        SimplifyStrategy::Factor => {
                            AlgebraicTransformations::factor(graph, expr_id)
                        }
                        SimplifyStrategy::Together => {
                            AlgebraicTransformations::together(graph, expr_id)
                        }
                        SimplifyStrategy::Cancel => {
                            AlgebraicTransformations::cancel(graph, expr_id)
                        }
                        SimplifyStrategy::Horner { var } => {
                            let target_sym =
                                graph.symbols.get_or_intern(var.as_deref().unwrap_or("x"));
                            AlgebraicTransformations::horner(graph, expr_id, target_sym)
                        }
                        SimplifyStrategy::PartialFractions { var } => {
                            let target_sym =
                                graph.symbols.get_or_intern(var.as_deref().unwrap_or("x"));
                            AlgebraicTransformations::partial_fractions(graph, expr_id, target_sym)
                        }
                        SimplifyStrategy::Collect { var } => {
                            let target_sym = graph.symbols.get_or_intern(var);
                            AlgebraicTransformations::collect(graph, expr_id, target_sym)
                        }
                        SimplifyStrategy::MinTree => {
                            Simplifier::min_tree(graph, expr_id).unwrap_or(expr_id)
                        }
                        SimplifyStrategy::MinOps => {
                            Simplifier::min_ops(graph, expr_id).unwrap_or(expr_id)
                        }
                        SimplifyStrategy::MinLeaf => {
                            Simplifier::min_leaf(graph, expr_id).unwrap_or(expr_id)
                        }
                    };

                    let latex = latex_fmt.format(graph, transformed_id).unwrap_or_default();
                    let unicode = unicode_fmt
                        .format(graph, transformed_id)
                        .unwrap_or_default();
                    let mut res = OperationResult::success(unicode.clone(), latex, unicode.clone());
                    res.root_expr = Some(transformed_id);
                    res.simplified_unicode = Some(unicode);
                    #[cfg(any(test, feature = "shadow-verify"))]
                    if let Ok(legacy) = cat_bridge::LegacyAstTree::from_graph(transformed_id, graph)
                    {
                        let mut cat_ctx = cat_core::CatRuntimeContext::default();
                        let mut bridge_ctx = cat_bridge::LegacyBridgeContext::default();
                        if let Ok(status) = cat_bridge::ShadowVerifyHarness::verify_roundtrip(
                            &legacy,
                            &mut cat_ctx,
                            &mut bridge_ctx,
                        ) {
                            res.metadata
                                .insert("cat_bridge_status".to_string(), format!("{status:?}"));
                        }
                    }
                    res
                }
                Err(e) => OperationResult::error(e.to_string()),
            },

            MathOperation::Evaluate {
                expression,
                precision_digits,
                bindings,
            } => match parser.parse(expression) {
                Ok(expr_id) => {
                    let mut eval_bindings = ctx.bindings.clone();
                    for (k, v) in bindings {
                        eval_bindings.insert(k.clone(), *v);
                    }
                    let p = precision_digits.unwrap_or(ctx.precision_digits);
                    let eval_ctx = EvalContext::with_bindings(p, eval_bindings.clone())
                        .with_uncertainties(ctx.uncertainties.clone())
                        .with_intervals(ctx.intervals.clone())
                        .with_branch_sheet(ctx.branch_sheet);

                    if eval_ctx.has_uncertainty()
                        || crate::numeric::expr_has_uncertainty_nodes(graph, expr_id)
                    {
                        match crate::numeric::eval_uncertain(graph, expr_id, &eval_ctx) {
                            Ok(u) => {
                                let out = u.format_unicode();
                                let latex = u.format_latex();
                                let mut res = OperationResult::success(out.clone(), latex, out);
                                res.metadata
                                    .insert("value".to_string(), format!("{}", u.mean));
                                res.metadata
                                    .insert("uncertainty".to_string(), format!("{}", u.std_dev));
                                res.metadata.insert(
                                    "interval_inf".to_string(),
                                    format!("{}", u.interval.inf),
                                );
                                res.metadata.insert(
                                    "interval_sup".to_string(),
                                    format!("{}", u.interval.sup),
                                );
                                res
                            }
                            Err(_) => match graph.evalf(expr_id, &eval_ctx) {
                                Ok(val) => {
                                    let out = val.to_string();
                                    OperationResult::success(out.clone(), out.clone(), out)
                                }
                                Err(e) => OperationResult::error(e.to_string()),
                            },
                        }
                    } else if ctx.branch_sheet.is_some() {
                        match graph.evalf(expr_id, &eval_ctx) {
                            Ok(val) => {
                                let out = val.to_string();
                                OperationResult::success(out.clone(), out.clone(), out)
                            }
                            Err(e) => OperationResult::error(e.to_string()),
                        }
                    } else {
                        match crate::numeric::try_eval_exact(graph, expr_id, &eval_bindings) {
                            Ok(Some(q)) if precision_digits.is_none() || q.is_integer() => {
                                if q.is_integer() {
                                    let out = q.numer().to_string();
                                    OperationResult::success(out.clone(), out.clone(), out)
                                } else {
                                    let out = format!("{}/{}", q.numer(), q.denom());
                                    let latex = format!("\\frac{{{}}}{{{}}}", q.numer(), q.denom());
                                    OperationResult::success(out.clone(), latex, out)
                                }
                            }
                            Ok(Some(q)) => {
                                use num_traits::ToPrimitive as _;
                                if let Some(f) = q.to_f64()
                                    && f.is_finite()
                                {
                                    let out = format!("{}", f);
                                    OperationResult::success(out.clone(), out.clone(), out)
                                } else {
                                    let out = format!("{}/{}", q.numer(), q.denom());
                                    let latex = format!("\\frac{{{}}}{{{}}}", q.numer(), q.denom());
                                    OperationResult::success(out.clone(), latex, out)
                                }
                            }
                            Err(e) => OperationResult::error(e.to_string()),
                            Ok(None) => match graph.evalf(expr_id, &eval_ctx) {
                                Ok(val) => {
                                    let out = val.to_string();
                                    OperationResult::success(out.clone(), out.clone(), out)
                                }
                                Err(e) => OperationResult::error(e.to_string()),
                            },
                        }
                    }
                }
                Err(e) => OperationResult::error(e.to_string()),
            },

            MathOperation::Substitute {
                expression,
                variable,
                value,
            } => match (parser.parse(expression), parser.parse(value)) {
                (Ok(expr_id), Ok(val_id)) => {
                    let var_sym = graph.symbols.get_or_intern(variable);
                    let subbed_id = graph.substitute(expr_id, var_sym, val_id);
                    let simp_id = graph.simplify(subbed_id);
                    let latex = latex_fmt.format(graph, simp_id).unwrap_or_default();
                    let unicode = unicode_fmt.format(graph, simp_id).unwrap_or_default();
                    let mut res = OperationResult::success(unicode.clone(), latex, unicode);
                    res.root_expr = Some(simp_id);
                    res
                }
                (Err(e), _) | (_, Err(e)) => OperationResult::error(e.to_string()),
            },

            MathOperation::Limit {
                expression,
                variable,
                point,
                direction,
            } => match (parser.parse(expression), parser.parse(point)) {
                (Ok(expr_id), Ok(pt_id)) => {
                    let wrt_sym = graph.symbols.get_or_intern(variable);
                    let lim_res = match direction
                        .as_deref()
                        .map(|s| s.trim().to_ascii_lowercase())
                        .as_deref()
                    {
                        Some("+" | "right" | "r" | "+1") => {
                            crate::limits::LimitEngine::limit_directed(
                                graph,
                                expr_id,
                                wrt_sym,
                                pt_id,
                                crate::limits::LimitDirection::FromRight,
                            )
                        }
                        Some("-" | "left" | "l" | "-1") => {
                            crate::limits::LimitEngine::limit_directed(
                                graph,
                                expr_id,
                                wrt_sym,
                                pt_id,
                                crate::limits::LimitDirection::FromLeft,
                            )
                        }
                        _ => graph.limit(expr_id, wrt_sym, pt_id),
                    };
                    match lim_res {
                        Ok(lim_id) => {
                            let latex = latex_fmt.format(graph, lim_id).unwrap_or_default();
                            let unicode = unicode_fmt.format(graph, lim_id).unwrap_or_default();
                            let dir_s = direction.as_deref().unwrap_or("");
                            let out = format!(
                                "lim_{{{} -> {}{}}} ({}) = {}",
                                variable, point, dir_s, expression, unicode
                            );
                            let mut res = OperationResult::success(out, latex, unicode);
                            res.root_expr = Some(lim_id);
                            res
                        }
                        Err(e) => OperationResult::error(e.to_string()),
                    }
                }
                (Err(e), _) | (_, Err(e)) => OperationResult::error(e.to_string()),
            },

            MathOperation::Series {
                expression,
                variable,
                point,
                order,
            } => match (parser.parse(expression), parser.parse(point)) {
                (Ok(expr_id), Ok(pt_expr)) => {
                    let wrt_sym = graph.symbols.get_or_intern(variable);
                    use crate::series::SymbolicSeries;
                    match graph.laurent_series(expr_id, wrt_sym, pt_expr, 0, *order as u32) {
                        Ok(series_id) => {
                            let simp_id = graph.simplify(series_id);
                            let latex = latex_fmt.format(graph, simp_id).unwrap_or_default();
                            let unicode = unicode_fmt.format(graph, simp_id).unwrap_or_default();
                            let mut res = OperationResult::success(unicode.clone(), latex, unicode);
                            res.root_expr = Some(simp_id);
                            res
                        }
                        Err(e) => OperationResult::error(e.to_string()),
                    }
                }
                (Err(e), _) | (_, Err(e)) => OperationResult::error(e.to_string()),
            },

            MathOperation::Sum {
                expression,
                variable,
                lower,
                upper,
            } => match execute_symbolic_sum(graph, &parser, expression, variable, lower, upper) {
                Ok(res_id) => {
                    let simp_id = graph.simplify(res_id);
                    let latex = latex_fmt.format(graph, simp_id).unwrap_or_default();
                    let unicode = unicode_fmt.format(graph, simp_id).unwrap_or_default();
                    let mut res = OperationResult::success(unicode.clone(), latex, unicode);
                    res.root_expr = Some(simp_id);
                    res
                }
                Err(e) => OperationResult::error(e),
            },

            MathOperation::Matrix(kind) => match kind {
                MatrixOpKind::Determinant { matrix_str } => {
                    match SymbolicMatrix::from_expr_str(graph, matrix_str)
                        .and_then(|m| m.determinant(graph))
                    {
                        Ok(det_id) => {
                            let simp_id = graph.simplify(det_id);
                            let latex = latex_fmt.format(graph, simp_id).unwrap_or_default();
                            let unicode = unicode_fmt.format(graph, simp_id).unwrap_or_default();
                            let mut res = OperationResult::success(unicode.clone(), latex, unicode);
                            res.root_expr = Some(simp_id);
                            res
                        }
                        Err(e) => OperationResult::error(e.to_string()),
                    }
                }
                MatrixOpKind::Inverse { matrix_str } => {
                    match SymbolicMatrix::from_expr_str(graph, matrix_str)
                        .and_then(|m| m.inverse(graph))
                    {
                        Ok(inv) => {
                            let elems: Vec<ExprId> =
                                inv.elements.iter().map(|&e| graph.simplify(e)).collect();
                            match SymbolicMatrix::new(inv.rows, inv.cols, elems) {
                                Ok(simp_mat) => {
                                    let mat_id = simp_mat.to_expr_id(graph);
                                    let latex = latex_fmt.format(graph, mat_id).unwrap_or_default();
                                    let unicode =
                                        unicode_fmt.format(graph, mat_id).unwrap_or_default();
                                    let mut res =
                                        OperationResult::success(unicode.clone(), latex, unicode);
                                    res.root_expr = Some(mat_id);
                                    res
                                }
                                Err(e) => OperationResult::error(e.to_string()),
                            }
                        }
                        Err(e) => OperationResult::error(e.to_string()),
                    }
                }
                MatrixOpKind::Trace { matrix_str } => {
                    match SymbolicMatrix::from_expr_str(graph, matrix_str)
                        .and_then(|m| m.trace(graph))
                    {
                        Ok(tr_id) => {
                            let simp_id = graph.simplify(tr_id);
                            let latex = latex_fmt.format(graph, simp_id).unwrap_or_default();
                            let unicode = unicode_fmt.format(graph, simp_id).unwrap_or_default();
                            let mut res = OperationResult::success(unicode.clone(), latex, unicode);
                            res.root_expr = Some(simp_id);
                            res
                        }
                        Err(e) => OperationResult::error(e.to_string()),
                    }
                }
                MatrixOpKind::Transpose { matrix_str } => {
                    match SymbolicMatrix::from_expr_str(graph, matrix_str)
                        .and_then(|m| m.transpose(graph))
                    {
                        Ok(tr_mat) => {
                            let mat_id = tr_mat.to_expr_id(graph);
                            let latex = latex_fmt.format(graph, mat_id).unwrap_or_default();
                            let unicode = unicode_fmt.format(graph, mat_id).unwrap_or_default();
                            let mut res = OperationResult::success(unicode.clone(), latex, unicode);
                            res.root_expr = Some(mat_id);
                            res
                        }
                        Err(e) => OperationResult::error(e.to_string()),
                    }
                }
                MatrixOpKind::CharPoly { matrix_str, var } => {
                    match SymbolicMatrix::from_expr_str(graph, matrix_str)
                        .and_then(|m| m.charpoly(graph, var))
                    {
                        Ok(cp_id) => {
                            let simp_id = graph.simplify(cp_id);
                            let latex = latex_fmt.format(graph, simp_id).unwrap_or_default();
                            let unicode = unicode_fmt.format(graph, simp_id).unwrap_or_default();
                            let mut res = OperationResult::success(unicode.clone(), latex, unicode);
                            res.root_expr = Some(simp_id);
                            res
                        }
                        Err(e) => OperationResult::error(e.to_string()),
                    }
                }
                MatrixOpKind::Multiply { a_str, b_str } => {
                    match (
                        SymbolicMatrix::from_expr_str(graph, a_str),
                        SymbolicMatrix::from_expr_str(graph, b_str),
                    ) {
                        (Ok(ma), Ok(mb)) => match ma.multiply(graph, &mb) {
                            Ok(prod) => {
                                let elems: Vec<ExprId> =
                                    prod.elements.iter().map(|&e| graph.simplify(e)).collect();
                                match SymbolicMatrix::new(prod.rows, prod.cols, elems) {
                                    Ok(simp_mat) => {
                                        let mat_id = simp_mat.to_expr_id(graph);
                                        let latex =
                                            latex_fmt.format(graph, mat_id).unwrap_or_default();
                                        let unicode =
                                            unicode_fmt.format(graph, mat_id).unwrap_or_default();
                                        let mut res = OperationResult::success(
                                            unicode.clone(),
                                            latex,
                                            unicode,
                                        );
                                        res.root_expr = Some(mat_id);
                                        res
                                    }
                                    Err(e) => OperationResult::error(e.to_string()),
                                }
                            }
                            Err(e) => OperationResult::error(e.to_string()),
                        },
                        (Err(e), _) | (_, Err(e)) => OperationResult::error(e.to_string()),
                    }
                }
                MatrixOpKind::Rref { matrix_str } => {
                    match SymbolicMatrix::from_expr_str(graph, matrix_str)
                        .and_then(|m| m.rref(graph))
                    {
                        Ok((rref_mat, pivots)) => {
                            let mat_id = rref_mat.to_expr_id(graph);
                            let latex = latex_fmt.format(graph, mat_id).unwrap_or_default();
                            let unicode = unicode_fmt.format(graph, mat_id).unwrap_or_default();
                            let mut res = OperationResult::success(unicode.clone(), latex, unicode);
                            res.root_expr = Some(mat_id);
                            res.metadata
                                .insert("rank".to_string(), pivots.len().to_string());
                            res.metadata
                                .insert("pivot_columns".to_string(), format!("{:?}", pivots));
                            res
                        }
                        Err(e) => OperationResult::error(e.to_string()),
                    }
                }
                MatrixOpKind::Rank { matrix_str } => {
                    match SymbolicMatrix::from_expr_str(graph, matrix_str)
                        .and_then(|m| m.rank(graph))
                    {
                        Ok(rank) => {
                            let r_id = graph.integer(rank as i64);
                            let latex = latex_fmt
                                .format(graph, r_id)
                                .unwrap_or_else(|_| rank.to_string());
                            let unicode = unicode_fmt
                                .format(graph, r_id)
                                .unwrap_or_else(|_| rank.to_string());
                            let mut res = OperationResult::success(unicode.clone(), latex, unicode);
                            res.root_expr = Some(r_id);
                            res
                        }
                        Err(e) => OperationResult::error(e.to_string()),
                    }
                }
                MatrixOpKind::Nullspace { matrix_str } => {
                    match SymbolicMatrix::from_expr_str(graph, matrix_str)
                        .and_then(|m| m.nullspace(graph))
                    {
                        Ok(basis) => {
                            if basis.is_empty() {
                                let mut res = OperationResult::success(
                                    "[]".to_string(),
                                    "\\left[ \\right]".to_string(),
                                    "[]".to_string(),
                                );
                                res.metadata.insert("nullity".to_string(), "0".to_string());
                                res
                            } else {
                                let mut u_vecs = Vec::with_capacity(basis.len());
                                let mut l_vecs = Vec::with_capacity(basis.len());
                                for col_vec in &basis {
                                    if let Ok(cm) =
                                        SymbolicMatrix::new(col_vec.len(), 1, col_vec.clone())
                                    {
                                        let id = cm.to_expr_id(graph);
                                        u_vecs.push(
                                            unicode_fmt.format(graph, id).unwrap_or_default(),
                                        );
                                        l_vecs
                                            .push(latex_fmt.format(graph, id).unwrap_or_default());
                                    }
                                }
                                let text = format!("[{}]", u_vecs.join(", "));
                                let latex = format!("\\left[ {} \\right]", l_vecs.join(",\\; "));
                                let mut res = OperationResult::success(text.clone(), latex, text);
                                res.metadata
                                    .insert("nullity".to_string(), basis.len().to_string());
                                res
                            }
                        }
                        Err(e) => OperationResult::error(e.to_string()),
                    }
                }
                MatrixOpKind::Eigenvalues { matrix_str } => {
                    match SymbolicMatrix::from_expr_str(graph, matrix_str)
                        .and_then(|m| m.eigenvalues(graph))
                    {
                        Ok(evals) => {
                            let mut u_vals = Vec::with_capacity(evals.len());
                            let mut l_vals = Vec::with_capacity(evals.len());
                            let mut last_simp = None;
                            for &ev in &evals {
                                let simp = Simplifier::simplify_with_assumptions(
                                    graph,
                                    ev,
                                    &ctx.assumptions,
                                )
                                .unwrap_or(ev);
                                last_simp = Some(simp);
                                u_vals.push(unicode_fmt.format(graph, simp).unwrap_or_default());
                                l_vals.push(latex_fmt.format(graph, simp).unwrap_or_default());
                            }
                            let text = format!("[{}]", u_vals.join(", "));
                            let latex = format!("\\left\\{{ {} \\right\\}}", l_vals.join(",\\; "));
                            let mut res = OperationResult::success(text.clone(), latex, text);
                            if evals.len() == 1 {
                                res.root_expr = last_simp;
                            }
                            res
                        }
                        Err(e) => OperationResult::error(e.to_string()),
                    }
                }
            },

            MathOperation::Combinatorics(kind) => match kind {
                CombinatoricsOpKind::Factorial { n } => {
                    let val = factorial(*n);
                    let out = format!("{}! = {}", n, val);
                    OperationResult::success(out.clone(), format!("{}! = {}", n, val), out)
                }
                CombinatoricsOpKind::Combinations { n, k } => {
                    let val = combinations(*n, *k);
                    let out = format!("C({}, {}) = {}", n, k, val);
                    OperationResult::success(
                        out.clone(),
                        format!("\\binom{{{}}}{{{}}} = {}", n, k, val),
                        out,
                    )
                }
                CombinatoricsOpKind::Permutations { n, k } => {
                    let val = permutations(*n, *k);
                    let out = format!("P({}, {}) = {}", n, k, val);
                    OperationResult::success(
                        out.clone(),
                        format!("P_{{{}}}^{{{}}} = {}", k, n, val),
                        out,
                    )
                }
                CombinatoricsOpKind::Derangements { n } => {
                    let val = derangements(*n);
                    let out = format!("!{} = {}", n, val);
                    OperationResult::success(out.clone(), format!("!{} = {}", n, val), out)
                }
                CombinatoricsOpKind::Catalan { n } => {
                    let val = catalan_number(*n);
                    let out = format!("C_{} = {}", n, val);
                    OperationResult::success(out.clone(), format!("C_{{{}}} = {}", n, val), out)
                }
                CombinatoricsOpKind::Bell { n } => {
                    let val = bell_number(*n as u32);
                    let out = format!("B_{} = {}", n, val);
                    OperationResult::success(out.clone(), format!("B_{{{}}} = {}", n, val), out)
                }
                CombinatoricsOpKind::Stirling2 { n, k } => {
                    let val = stirling_second_kind(*n as u32, *k as u32);
                    let out = format!("S({}, {}) = {}", n, k, val);
                    OperationResult::success(
                        out.clone(),
                        format!("\\left\\{{{}\\atop {}\\right\\}} = {}", n, k, val),
                        out,
                    )
                }
                CombinatoricsOpKind::Partitions { n } => {
                    let val = integer_partitions(*n as usize);
                    let out = format!("p({}) = {}", n, val);
                    OperationResult::success(out.clone(), format!("p({}) = {}", n, val), out)
                }
            },

            MathOperation::Transform(kind) => match kind {
                TransformOpKind::Laplace {
                    expression,
                    time_var,
                    freq_var,
                } => match parser.parse(expression) {
                    Ok(expr_id) => {
                        let t_sym = graph.symbols.get_or_intern(time_var);
                        let s_sym = graph.symbols.get_or_intern(freq_var);
                        match graph.laplace_transform(expr_id, t_sym, s_sym) {
                            Ok(res_id) => {
                                let unicode = unicode_fmt.format(graph, res_id).unwrap_or_default();
                                let latex = latex_fmt.format(graph, res_id).unwrap_or_default();
                                let out =
                                    format!("L{{{}}}({}) = {}", expression, freq_var, unicode);
                                OperationResult::success(out, latex, unicode)
                            }
                            Err(e) => OperationResult::error(e.to_string()),
                        }
                    }
                    Err(e) => OperationResult::error(e.to_string()),
                },
                TransformOpKind::InverseLaplace {
                    expression,
                    freq_var,
                    time_var,
                } => match parser.parse(expression) {
                    Ok(expr_id) => {
                        let s_sym = graph.symbols.get_or_intern(freq_var);
                        let t_sym = graph.symbols.get_or_intern(time_var);
                        match graph.inverse_laplace_transform(expr_id, s_sym, t_sym) {
                            Ok(res_id) => {
                                let simp_id = graph.simplify(res_id);
                                let unicode =
                                    unicode_fmt.format(graph, simp_id).unwrap_or_default();
                                let latex = latex_fmt.format(graph, simp_id).unwrap_or_default();
                                let mut res =
                                    OperationResult::success(unicode.clone(), latex, unicode);
                                res.root_expr = Some(simp_id);
                                res
                            }
                            Err(e) => OperationResult::error(e.to_string()),
                        }
                    }
                    Err(e) => OperationResult::error(e.to_string()),
                },
                TransformOpKind::Fourier {
                    expression,
                    time_var,
                    freq_var,
                } => match parser.parse(expression) {
                    Ok(expr_id) => {
                        let t_sym = graph.symbols.get_or_intern(time_var);
                        let w_sym = graph.symbols.get_or_intern(freq_var);
                        match graph.fourier_transform(expr_id, t_sym, w_sym) {
                            Ok(res_id) => {
                                if matches!(graph.get(res_id).kind, ExprKind::Integral { .. }) {
                                    OperationResult::error(format!(
                                        "Fourier transform closed form unavailable for '{}'",
                                        expression
                                    ))
                                } else {
                                    let unicode =
                                        unicode_fmt.format(graph, res_id).unwrap_or_default();
                                    let latex = latex_fmt.format(graph, res_id).unwrap_or_default();
                                    let out =
                                        format!("F{{{}}}({}) = {}", expression, freq_var, unicode);
                                    let mut res = OperationResult::success(out, latex, unicode);
                                    res.root_expr = Some(res_id);
                                    res
                                }
                            }
                            Err(e) => OperationResult::error(e.to_string()),
                        }
                    }
                    Err(e) => OperationResult::error(e.to_string()),
                },
            },

            MathOperation::Polynomial(kind) => match kind {
                PolynomialOpKind::Roots { poly_str } => {
                    let target_var = if poly_str.contains('t') && !poly_str.contains('x') {
                        "t"
                    } else {
                        "x"
                    };
                    let target_sym = graph.symbols.get_or_intern(target_var);
                    match parser.parse(poly_str) {
                        Ok(poly_id) => match graph.solveset(poly_id, target_sym) {
                            Ok(roots) => {
                                let mut root_strs = Vec::new();
                                let mut latex_strs = Vec::new();
                                for &r in &roots {
                                    root_strs
                                        .push(unicode_fmt.format(graph, r).unwrap_or_default());
                                    latex_strs.push(latex_fmt.format(graph, r).unwrap_or_default());
                                }
                                let out = format!(
                                    "Roots for polynomial {}: [{}]",
                                    poly_str,
                                    root_strs.join(", ")
                                );
                                let latex =
                                    format!("\\left\\{{ {} \\right\\}}", latex_strs.join(", "));
                                OperationResult::success(out.clone(), latex, out)
                            }
                            Err(e) => OperationResult::error(e.to_string()),
                        },
                        Err(e) => OperationResult::error(e.to_string()),
                    }
                }
                PolynomialOpKind::Discriminant { poly_str, var } => match parser.parse(poly_str) {
                    Ok(poly_id) => {
                        let var_sym = graph.symbols.get_or_intern(var);
                        match symbolic_discriminant(graph, poly_id, var_sym) {
                            Ok(disc_id) => {
                                let simp_id = graph.simplify(disc_id);
                                let latex = latex_fmt.format(graph, simp_id).unwrap_or_default();
                                let unicode =
                                    unicode_fmt.format(graph, simp_id).unwrap_or_default();
                                let mut res =
                                    OperationResult::success(unicode.clone(), latex, unicode);
                                res.root_expr = Some(simp_id);
                                res
                            }
                            Err(e) => OperationResult::error(e.to_string()),
                        }
                    }
                    Err(e) => OperationResult::error(e.to_string()),
                },
                PolynomialOpKind::Resultant {
                    poly1_str,
                    poly2_str,
                    var,
                } => match (parser.parse(poly1_str), parser.parse(poly2_str)) {
                    (Ok(p1), Ok(p2)) => {
                        let var_sym = graph.symbols.get_or_intern(var);
                        match symbolic_resultant(graph, p1, p2, var_sym) {
                            Ok(res_id) => {
                                let simp_id = graph.simplify(res_id);
                                let latex = latex_fmt.format(graph, simp_id).unwrap_or_default();
                                let unicode =
                                    unicode_fmt.format(graph, simp_id).unwrap_or_default();
                                let mut res =
                                    OperationResult::success(unicode.clone(), latex, unicode);
                                res.root_expr = Some(simp_id);
                                res
                            }
                            Err(e) => OperationResult::error(e.to_string()),
                        }
                    }
                    (Err(e), _) | (_, Err(e)) => OperationResult::error(e.to_string()),
                },
                PolynomialOpKind::Gcd {
                    poly1_str,
                    poly2_str,
                } => match (parser.parse(poly1_str), parser.parse(poly2_str)) {
                    (Ok(p1), Ok(p2)) => {
                        let mut syms = std::collections::BTreeSet::new();
                        collect_expr_symbols(graph, p1, &mut syms);
                        collect_expr_symbols(graph, p2, &mut syms);
                        let var_sym = if syms.is_empty() {
                            graph.symbols.get_or_intern("x")
                        } else if let (1, Some(&only)) = (syms.len(), syms.iter().next()) {
                            only
                        } else {
                            return OperationResult::error(
                                "Univariate poly_gcd supports only a single variable across both polynomials"
                                    .to_string(),
                            );
                        };
                        match (
                            UniPoly::from_expr(graph, p1, var_sym),
                            UniPoly::from_expr(graph, p2, var_sym),
                        ) {
                            (Ok(u1), Ok(u2)) => match u1.gcd(&u2) {
                                Ok(g) => {
                                    let g_id = if g.is_zero() {
                                        expr_from_rational(
                                            graph,
                                            &num_rational::BigRational::from_integer(0.into()),
                                        )
                                    } else {
                                        g.to_expr(graph, var_sym)
                                    };
                                    let simp_id = graph.simplify(g_id);
                                    let latex =
                                        latex_fmt.format(graph, simp_id).unwrap_or_default();
                                    let unicode =
                                        unicode_fmt.format(graph, simp_id).unwrap_or_default();
                                    let mut res =
                                        OperationResult::success(unicode.clone(), latex, unicode);
                                    res.root_expr = Some(simp_id);
                                    res
                                }
                                Err(e) => OperationResult::error(e.to_string()),
                            },
                            (Err(e), _) | (_, Err(e)) => OperationResult::error(e.to_string()),
                        }
                    }
                    (Err(e), _) | (_, Err(e)) => OperationResult::error(e.to_string()),
                },
            },

            MathOperation::Physics(kind) => match kind {
                PhysicsOpKind::Quantity { val, unit } => {
                    let out = format!("{:.4} [{}]", val, unit);
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                PhysicsOpKind::Convert {
                    val,
                    from_unit,
                    to_unit,
                } => {
                    let converted = if (from_unit == "km/h" || from_unit == "kmh")
                        && (to_unit == "m/s" || to_unit == "ms")
                    {
                        val / 3.6
                    } else if (from_unit == "m/s" || from_unit == "ms")
                        && (to_unit == "km/h" || to_unit == "kmh")
                    {
                        val * 3.6
                    } else if from_unit == "m" && from_unit == "ft" {
                        val * 3.28084
                    } else {
                        *val
                    };
                    let out = format!(
                        "{:.4} [{}] = {:.4} [{}]",
                        val, from_unit, converted, to_unit
                    );
                    OperationResult::success(out.clone(), out.clone(), out)
                }
            },

            MathOperation::Interval(kind) => match kind {
                IntervalOpKind::Add {
                    a_lo,
                    a_hi,
                    b_lo,
                    b_hi,
                } => {
                    let res = RealInterval::new(*a_lo, *a_hi) + RealInterval::new(*b_lo, *b_hi);
                    let out = format!(
                        "[{}, {}] + [{}, {}] = [{:.4}, {:.4}]",
                        a_lo, a_hi, b_lo, b_hi, res.inf, res.sup
                    );
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                IntervalOpKind::Mul {
                    a_lo,
                    a_hi,
                    b_lo,
                    b_hi,
                } => {
                    let res = RealInterval::new(*a_lo, *a_hi) * RealInterval::new(*b_lo, *b_hi);
                    let out = format!(
                        "[{}, {}] * [{}, {}] = [{:.4}, {:.4}]",
                        a_lo, a_hi, b_lo, b_hi, res.inf, res.sup
                    );
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                IntervalOpKind::Sub {
                    a_lo,
                    a_hi,
                    b_lo,
                    b_hi,
                } => {
                    let res = RealInterval::new(*a_lo, *a_hi) - RealInterval::new(*b_lo, *b_hi);
                    let out = format!(
                        "[{}, {}] - [{}, {}] = [{:.4}, {:.4}]",
                        a_lo, a_hi, b_lo, b_hi, res.inf, res.sup
                    );
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                IntervalOpKind::Div {
                    a_lo,
                    a_hi,
                    b_lo,
                    b_hi,
                } => {
                    let res = RealInterval::new(*a_lo, *a_hi) / RealInterval::new(*b_lo, *b_hi);
                    let out = format!(
                        "[{}, {}] / [{}, {}] = [{:.4}, {:.4}]",
                        a_lo, a_hi, b_lo, b_hi, res.inf, res.sup
                    );
                    OperationResult::success(out.clone(), out.clone(), out)
                }
            },

            MathOperation::Logic(kind) => match kind {
                LogicOpKind::SatSolve { formula } => match SatSolver::parse_formula(formula) {
                    Ok((expr, vars)) => {
                        let out = if let Some(model) = SatSolver::solve_model(&expr) {
                            if vars.is_empty() {
                                format!("SAT for '{}': Satisfiable (Tautology)", formula)
                            } else {
                                let bindings: Vec<String> = vars
                                    .iter()
                                    .map(|(vid, name)| {
                                        let v = model.get(vid).copied().unwrap_or(false);
                                        format!("{}={}", name, v)
                                    })
                                    .collect();
                                format!(
                                    "SAT for '{}': Satisfiable ({})",
                                    formula,
                                    bindings.join(", ")
                                )
                            }
                        } else {
                            format!("SAT for '{}': Unsatisfiable", formula)
                        };
                        OperationResult::success(out.clone(), out.clone(), out)
                    }
                    Err(e) => OperationResult::error(e),
                },
                LogicOpKind::TruthTable { formula } => match SatSolver::truth_table(formula) {
                    Ok(table) => OperationResult::success(table.clone(), table.clone(), table),
                    Err(e) => OperationResult::error(e),
                },
            },

            MathOperation::HyperOp(kind) => match kind {
                HyperOpKind::KnuthUpArrow { a, arrows, b } => {
                    let val = knuth_up_arrow(*a, *arrows as u32, *b);
                    let out = format!("{} ↑^{} {} = {}", a, arrows, b, val);
                    OperationResult::success(
                        out.clone(),
                        format!("{} \\uparrow^{{{}}} {} = {}", a, arrows, b, val),
                        out,
                    )
                }
                HyperOpKind::Tetration { a, b } => {
                    let val = tetration(*a, *b);
                    let out = format!("{}^^{} = {}", a, b, val);
                    OperationResult::success(
                        out.clone(),
                        format!("{}^{{{}^{{...}}}} = {}", a, b, val),
                        out,
                    )
                }
                HyperOpKind::Pentation { a, b } => {
                    let val = pentation(*a, *b);
                    let out = format!("{}^^^{} = {}", a, b, val);
                    OperationResult::success(
                        out.clone(),
                        format!("{} \\uparrow\\uparrow\\uparrow {} = {}", a, b, val),
                        out,
                    )
                }
                HyperOpKind::General { n, a, b } => {
                    let val = hyperoperation(*n as u32, *a, *b);
                    let out = format!("H_{}({}, {}) = {}", n, a, b, val);
                    OperationResult::success(
                        out.clone(),
                        format!("H_{{{}}}({}, {}) = {}", n, a, b, val),
                        out,
                    )
                }
                HyperOpKind::Ackermann { m, n } => {
                    let val = ackermann(*m, *n);
                    let out = format!("A({}, {}) = {}", m, n, val);
                    OperationResult::success(out.clone(), format!("A({}, {}) = {}", m, n, val), out)
                }
                HyperOpKind::SuperLog { base, val } => {
                    let res = super_log(*base, *val);
                    let out = format!("slog_{}({}) = {:.6}", base, val, res);
                    OperationResult::success(
                        out.clone(),
                        format!("\\text{{slog}}_{{{}}}({}) = {:.6}", base, val, res),
                        out,
                    )
                }
            },

            MathOperation::Tropical(kind) => match kind {
                TropicalOpKind::MaxPlusAdd { a, b } => {
                    let res = MaxPlus::val(*a) + MaxPlus::val(*b);
                    let out = format!(
                        "{:?} ⊕_max {:?} = {:?}",
                        MaxPlus::val(*a),
                        MaxPlus::val(*b),
                        res
                    );
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                TropicalOpKind::MaxPlusMul { a, b } => {
                    let res = MaxPlus::val(*a) * MaxPlus::val(*b);
                    let out = format!(
                        "{:?} ⊗_max {:?} = {:?}",
                        MaxPlus::val(*a),
                        MaxPlus::val(*b),
                        res
                    );
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                TropicalOpKind::MinPlusAdd { a, b } => {
                    let res = MinPlus::val(*a) + MinPlus::val(*b);
                    let out = format!(
                        "{:?} ⊕_min {:?} = {:?}",
                        MinPlus::val(*a),
                        MinPlus::val(*b),
                        res
                    );
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                TropicalOpKind::MinPlusMul { a, b } => {
                    let res = MinPlus::val(*a) * MinPlus::val(*b);
                    let out = format!(
                        "{:?} ⊗_min {:?} = {:?}",
                        MinPlus::val(*a),
                        MinPlus::val(*b),
                        res
                    );
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                TropicalOpKind::LogSumExp { x, y, epsilon } => {
                    let res = log_sum_exp(*x, *y, *epsilon);
                    let out = format!("LSE_eps({}, {}) = {:.6}", x, y, res);
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                TropicalOpKind::MatrixMul {
                    semiring,
                    rows,
                    cols,
                    a_data,
                    b_data,
                } => {
                    let mut m1 = TropicalMatrix::new(*rows, *cols, MinPlus::zero());
                    let mut m2 = TropicalMatrix::new(*rows, *cols, MinPlus::zero());
                    for i in 0..*rows {
                        for j in 0..*cols {
                            m1.set(i, j, MinPlus::val(a_data[i * cols + j]));
                            m2.set(i, j, MinPlus::val(b_data[i * cols + j]));
                        }
                    }
                    let res = m1.mul(&m2);
                    let out = format!("Tropical ({}) Matrix Product: {:?}", semiring, res);
                    OperationResult::success(out.clone(), out.clone(), out)
                }
            },

            MathOperation::NumberTheory(kind) => match kind {
                NumberTheoryOpKind::ContinuedFraction { val_str, max_terms } => {
                    let x = val_str.parse::<f64>().unwrap_or(std::f64::consts::SQRT_2);
                    let cf = ContinuedFraction::from_f64(x, *max_terms);
                    let conv = cf.convergent();
                    let out = format!(
                        "Continued Fraction: {:?} -> Convergent: {}/{}",
                        cf.terms, conv.0, conv.1
                    );
                    OperationResult::success(
                        out.clone(),
                        format!(
                            "\\text{{CF: }} {:?} \\to \\frac{{{}}}{{{}}}",
                            cf.terms, conv.0, conv.1
                        ),
                        out,
                    )
                }
                NumberTheoryOpKind::IsPrime { n } => {
                    let prime = is_prime(*n);
                    let out = format!("is_prime({}) = {}", n, prime);
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                NumberTheoryOpKind::IsPrimeExpr { expr_str } => {
                    let eval_int = |s: &str| -> Result<num_bigint::BigInt, String> {
                        let e = parser.parse(s).map_err(|e| e.to_string())?;
                        let q = crate::numeric::try_eval_exact(
                            graph,
                            e,
                            &std::collections::HashMap::new(),
                        )
                        .map_err(|e| e.to_string())?
                        .ok_or_else(|| {
                            format!("Expression '{}' did not evaluate to an exact integer", s)
                        })?;
                        if q.is_integer() {
                            Ok(q.numer().clone())
                        } else {
                            Err(format!("Expression '{}' is not an integer", s))
                        }
                    };
                    match eval_int(expr_str) {
                        Ok(n) => {
                            use num_traits::Signed as _;
                            let prime = if n.is_negative() {
                                false
                            } else {
                                crate::numbertheory::is_prime_big(n.magnitude())
                            };
                            let out = format!("is_prime({}) = {}", n, prime);
                            OperationResult::success(out.clone(), out.clone(), out)
                        }
                        Err(err) => OperationResult::error(err),
                    }
                }
                NumberTheoryOpKind::ExtendedGcd { a, b } => {
                    let (g, x, y) = extended_gcd(*a, *b);
                    let out = format!("gcd({}, {}) = {} = {}*({}) + {}*({})", a, b, g, a, x, b, y);
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                NumberTheoryOpKind::ExtendedGcdExpr { a_str, b_str } => {
                    let eval_int = |s: &str| -> Result<num_bigint::BigInt, String> {
                        let e = parser.parse(s).map_err(|e| e.to_string())?;
                        let q = crate::numeric::try_eval_exact(
                            graph,
                            e,
                            &std::collections::HashMap::new(),
                        )
                        .map_err(|e| e.to_string())?
                        .ok_or_else(|| {
                            format!("Expression '{}' did not evaluate to an exact integer", s)
                        })?;
                        if q.is_integer() {
                            Ok(q.numer().clone())
                        } else {
                            Err(format!("Expression '{}' is not an integer", s))
                        }
                    };
                    match (eval_int(a_str), eval_int(b_str)) {
                        (Ok(a), Ok(b)) => {
                            let (g, x, y) = crate::numbertheory::extended_gcd_big(&a, &b);
                            let out = format!(
                                "gcd({}, {}) = {} = {}*({}) + {}*({})",
                                a, b, g, a, x, b, y
                            );
                            OperationResult::success(out.clone(), out.clone(), out)
                        }
                        (Err(e), _) | (_, Err(e)) => OperationResult::error(e),
                    }
                }
                NumberTheoryOpKind::LcmExpr { a_str, b_str } => {
                    let eval_int = |s: &str| -> Result<num_bigint::BigInt, String> {
                        let e = parser.parse(s).map_err(|e| e.to_string())?;
                        let q = crate::numeric::try_eval_exact(
                            graph,
                            e,
                            &std::collections::HashMap::new(),
                        )
                        .map_err(|e| e.to_string())?
                        .ok_or_else(|| {
                            format!("Expression '{}' did not evaluate to an exact integer", s)
                        })?;
                        if q.is_integer() {
                            Ok(q.numer().clone())
                        } else {
                            Err(format!("Expression '{}' is not an integer", s))
                        }
                    };
                    match (eval_int(a_str), eval_int(b_str)) {
                        (Ok(a), Ok(b)) => {
                            use num::Integer as _;
                            let l = a.lcm(&b);
                            let out = format!("lcm({}, {}) = {}", a, b, l);
                            OperationResult::success(out.clone(), out.clone(), out)
                        }
                        (Err(e), _) | (_, Err(e)) => OperationResult::error(e),
                    }
                }
                NumberTheoryOpKind::Legendre { a, p } => {
                    let leg = legendre_symbol(*a, *p);
                    let out = format!("Legendre Symbol ({}/{}) = {}", a, p, leg);
                    OperationResult::success(
                        out.clone(),
                        format!("\\left(\\frac{{{}}}{{{}}}\\right) = {}", a, p, leg),
                        out,
                    )
                }
                NumberTheoryOpKind::PrimeDecomposition {
                    expr_str,
                    domain_hint,
                } => {
                    // Evaluate pure integer arithmetic expressions first (e.g. `prime_factors 2^67 - 1`)
                    let resolved_expr = if domain_hint.is_none()
                        && let Ok(e_id) = parser.parse(expr_str)
                        && let Ok(Some(q)) = crate::numeric::try_eval_exact(
                            graph,
                            e_id,
                            &std::collections::HashMap::new(),
                        )
                        && q.is_integer()
                    {
                        q.numer().to_string()
                    } else {
                        expr_str.clone()
                    };
                    match decompose_universal(&resolved_expr, domain_hint.as_deref()) {
                        Ok(res) => {
                            let out = format!(
                                "{} [{}]\n  {}",
                                res.formatted_equation,
                                res.number_system,
                                res.classification_summary.join("\n  ")
                            );
                            OperationResult::success(
                                out,
                                res.latex_equation,
                                res.formatted_equation,
                            )
                        }
                        Err(err) => {
                            OperationResult::error(format!("Prime decomposition failed: {}", err))
                        }
                    }
                }
            },

            MathOperation::Control(kind) => match kind {
                ControlOpKind::StateSpace { a, b, c, d } => {
                    let sys = StateSpaceSystem::new(a.clone(), b.clone(), c.clone(), d.clone());
                    let stable = sys.is_stable_2x2().unwrap_or(false);
                    let out = format!("State-Space Continuous LTI System (Stable: {})", stable);
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                ControlOpKind::Stability2x2 { a11, a12, a21, a22 } => {
                    let a = vec![vec![*a11, *a12], vec![*a21, *a22]];
                    let b = vec![vec![1.0], vec![1.0]];
                    let c = vec![vec![1.0, 0.0]];
                    let d = vec![vec![0.0]];
                    let sys = StateSpaceSystem::new(a, b, c, d);
                    let stable = sys.is_stable_2x2().unwrap_or(false);
                    let out = format!("2x2 Matrix System Hurwitz Stability: {}", stable);
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                ControlOpKind::Controllability2x2 {
                    a11,
                    a12,
                    a21,
                    a22,
                    b1,
                    b2,
                } => {
                    let a = vec![vec![*a11, *a12], vec![*a21, *a22]];
                    let b = vec![vec![*b1], vec![*b2]];
                    let c = vec![vec![1.0, 0.0]];
                    let d = vec![vec![0.0]];
                    let sys = StateSpaceSystem::new(a, b, c, d);
                    let ctrl = sys.controllability_matrix_2x2();
                    let out = format!("Controllability Matrix: {:?}", ctrl);
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                ControlOpKind::DiscretizeZoh {
                    a,
                    b,
                    c,
                    d,
                    sample_time,
                } => {
                    let sys = StateSpaceSystem::new(a.clone(), b.clone(), c.clone(), d.clone());
                    let dsys = sys.discretize_zoh(*sample_time);
                    let out = format!(
                        "ZOH Discretized LTI System (Ts = {} s):\n  • State Transition Matrix Ad:\n    {:?}\n  • Input Coupling Matrix Bd:\n    {:?}",
                        sample_time, dsys.a, dsys.b
                    );
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                ControlOpKind::RankTests { a, b, c } => {
                    let dummy_d = vec![vec![0.0; b[0].len()]; c.len()];
                    let sys = StateSpaceSystem::new(a.clone(), b.clone(), c.clone(), dummy_d);
                    let n = sys.num_states();
                    let ctrl_rank = sys.controllability_rank();
                    let obs_rank = sys.observability_rank();
                    let is_ctrl = sys.is_controllable();
                    let is_obs = sys.is_observable();
                    let out = format!(
                        "Kalman Rank Analysis (n = {}):\n  • Controllability Matrix Rank: {}/{} (Controllable: {})\n  • Observability Matrix Rank: {}/{} (Observable: {})",
                        n, ctrl_rank, n, is_ctrl, obs_rank, n, is_obs
                    );
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                ControlOpKind::PolePlacement {
                    a,
                    b,
                    desired_poles,
                } => {
                    let dummy_c = vec![vec![1.0; a.len()]];
                    let dummy_d = vec![vec![0.0; b[0].len()]];
                    let sys = StateSpaceSystem::new(a.clone(), b.clone(), dummy_c, dummy_d);
                    match sys.pole_placement_ackermann(desired_poles) {
                        Ok(gain_k) => {
                            let out =
                                format!("Ackermann State-Feedback Gain Vector K:\n  {:?}", gain_k);
                            OperationResult::success(out.clone(), out.clone(), out)
                        }
                        Err(e) => OperationResult::error(e),
                    }
                }
                ControlOpKind::FrequencyResponse { a, b, c, d, omega } => {
                    let sys = StateSpaceSystem::new(a.clone(), b.clone(), c.clone(), d.clone());
                    match sys.frequency_response(*omega) {
                        Ok(resp) => {
                            let mut out =
                                format!("Bode Frequency Response at ω = {:.4} rad/s:\n", omega);
                            for (out_idx, row) in resp.iter().enumerate() {
                                for (in_idx, (mag_db, phase_deg)) in row.iter().enumerate() {
                                    out.push_str(&format!(
                                        "  Channel (u{} -> y{}): {:.2} dB, {:.2}°\n",
                                        in_idx + 1,
                                        out_idx + 1,
                                        mag_db,
                                        phase_deg
                                    ));
                                }
                            }
                            let text = out.trim_end().to_string();
                            OperationResult::success(text.clone(), text.clone(), text)
                        }
                        Err(e) => OperationResult::error(e),
                    }
                }
            },

            MathOperation::Topology(kind) => match kind {
                TopologyOpKind::EulerCharacteristic { simplices } => {
                    let mut complex = SimplicialComplex::new();
                    for s in simplices {
                        complex.add_simplex(s);
                    }
                    let chi = complex.euler_characteristic();
                    let out = format!("Euler Characteristic χ(K) = {}", chi);
                    OperationResult::success(out.clone(), format!("\\chi(K) = {}", chi), out)
                }
                TopologyOpKind::CountSimplices { simplices, dim } => {
                    let mut complex = SimplicialComplex::new();
                    for s in simplices {
                        complex.add_simplex(s);
                    }
                    let count = complex.count_simplices(*dim);
                    let out = format!("Count of {}-simplices: {}", dim, count);
                    OperationResult::success(out.clone(), out.clone(), out)
                }
            },

            MathOperation::StatMech(kind) => match kind {
                StatMechOpKind::PartitionFunction {
                    energy_levels,
                    beta,
                } => {
                    let z: f64 = energy_levels
                        .iter()
                        .map(|&(e, g)| (g as f64) * (-beta * e).exp())
                        .sum();
                    let out = format!("Canonical Partition Function Z(β={:.4}) = {:.6}", beta, z);
                    OperationResult::success(out.clone(), format!("Z(\\beta) = {:.6}", z), out)
                }
                StatMechOpKind::FreeEnergy {
                    energy_levels,
                    beta,
                } => {
                    let z: f64 = energy_levels
                        .iter()
                        .map(|&(e, g)| (g as f64) * (-beta * e).exp())
                        .sum();
                    let f = if z <= 0.0 { 0.0 } else { -z.ln() / beta };
                    let out = format!("Helmholtz Free Energy F(β={:.4}) = {:.6}", beta, f);
                    OperationResult::success(out.clone(), format!("F = {:.6}", f), out)
                }
                StatMechOpKind::InternalEnergy {
                    energy_levels,
                    beta,
                } => {
                    let z: f64 = energy_levels
                        .iter()
                        .map(|&(e, g)| (g as f64) * (-beta * e).exp())
                        .sum();
                    let num: f64 = energy_levels
                        .iter()
                        .map(|&(e, g)| (g as f64) * e * (-beta * e).exp())
                        .sum();
                    let u = if z <= 0.0 { 0.0 } else { num / z };
                    let out = format!("Mean Internal Energy U(β={:.4}) = {:.6}", beta, u);
                    OperationResult::success(out.clone(), format!("U = {:.6}", u), out)
                }
                StatMechOpKind::FermiDirac {
                    energy,
                    chemical_potential,
                    beta,
                } => {
                    let exp_term = (beta * (energy - chemical_potential)).exp();
                    let fd = 1.0 / (exp_term + 1.0);
                    let out = format!("Fermi-Dirac Distribution f(E={:.4}) = {:.6}", energy, fd);
                    OperationResult::success(out.clone(), format!("f(E) = {:.6}", fd), out)
                }
                StatMechOpKind::BoseEinstein {
                    energy,
                    chemical_potential,
                    beta,
                } => {
                    let out = if energy <= chemical_potential {
                        "Bose-Einstein Distribution undefined for E <= mu (Bose-Einstein condensation)".to_string()
                    } else {
                        let exp_term = (beta * (energy - chemical_potential)).exp();
                        let be = 1.0 / (exp_term - 1.0);
                        format!("Bose-Einstein Distribution n(E={:.4}) = {:.6}", energy, be)
                    };
                    OperationResult::success(out.clone(), format!("\\text{{{}}}", out), out)
                }
            },

            MathOperation::Galois(kind) => match kind {
                GaloisOpKind::QuadraticExtension { d } => {
                    let out = format!(
                        "Field Extension Q(sqrt({})) / Q (Degree: 2, Galois: true, Solvable: true)",
                        d
                    );
                    OperationResult::success(
                        out.clone(),
                        format!("\\mathbb{{Q}}(\\sqrt{{{}}})/\\mathbb{{Q}}", d),
                        out,
                    )
                }
                GaloisOpKind::CheckSolvability { group_name } => {
                    match classify_galois_group_solvability(group_name) {
                        Ok((solvable, detail)) => {
                            let out = format!(
                                "Galois Group {} ({}) Solvable by Radicals: {}",
                                group_name, detail, solvable
                            );
                            OperationResult::success(out.clone(), out.clone(), out)
                        }
                        Err(e) => OperationResult::error(e),
                    }
                }
            },

            MathOperation::Cartan(kind) => {
                let coords: [String; 4] = ["x".into(), "y".into(), "z".into(), "t".into()];
                let parse = |text: &str, dim: usize| -> Result<DifferentialForm, String> {
                    if dim == 0 || dim > coords.len() {
                        return Err(format!("dimension must be between 1 and {}", coords.len()));
                    }
                    DifferentialForm::parse(text, dim, &coords[..dim]).map_err(|e| e.to_string())
                };
                let report = |label: String, form: DifferentialForm, latex: String| {
                    let out = format!("{label} = {} (degree {})", form.to_text(), form.degree);
                    OperationResult::success(out.clone(), latex, out)
                };
                // Forms whose coefficients are expressions (`x^2 dy`) are handled by `SymbolicForm`.
                let sym_names: Vec<String> = coords.to_vec();
                let sym_parse = |text: &str, dim: usize| -> Result<SymbolicForm, String> {
                    if dim == 0 || dim > sym_names.len() {
                        return Err(format!(
                            "dimension must be between 1 and {}",
                            sym_names.len()
                        ));
                    }
                    SymbolicForm::parse(graph, text, &sym_names[..dim]).map_err(|e| e.to_string())
                };
                let sym_report = |label: String, form: &SymbolicForm, latex: &str| {
                    let out = format!("{label} = {} (degree {})", form.to_text(graph), form.degree);
                    OperationResult::success(out.clone(), latex.to_string(), out)
                };
                let sym_vector = |components: &[f64]| -> Vec<algebra_core::ExprId> {
                    components.iter().map(|&v| graph.float(v)).collect()
                };
                match kind {
                    CartanOpKind::Wedge {
                        form1_str,
                        form2_str,
                        dim,
                    } => match (parse(form1_str, *dim), parse(form2_str, *dim)) {
                        (Ok(f1), Ok(f2)) => match f1.wedge(&f2) {
                            Ok(w) => report(
                                format!("Wedge Product ({form1_str}) ∧ ({form2_str})"),
                                w,
                                "\\omega_1 \\wedge \\omega_2".to_string(),
                            ),
                            Err(e) => OperationResult::error(e.to_string()),
                        },
                        (Err(e), _) | (_, Err(e)) => {
                            match (sym_parse(form1_str, *dim), sym_parse(form2_str, *dim)) {
                                (Ok(f1), Ok(f2)) => match f1.wedge(graph, &f2) {
                                    Ok(w) => sym_report(
                                        format!("Wedge Product ({form1_str}) ∧ ({form2_str})"),
                                        &w,
                                        "\\omega_1 \\wedge \\omega_2",
                                    ),
                                    Err(err) => OperationResult::error(err.to_string()),
                                },
                                _ => OperationResult::error(e),
                            }
                        }
                    },
                    CartanOpKind::ExteriorDerivative { form_str, dim } => {
                        match parse(form_str, *dim) {
                            Ok(f) => match f.exterior_derivative() {
                                Ok(df) => report(
                                    format!("Exterior Derivative d({form_str})"),
                                    df,
                                    "\\mathrm{d}\\omega".to_string(),
                                ),
                                Err(e) => OperationResult::error(e.to_string()),
                            },
                            Err(e) => match sym_parse(form_str, *dim).and_then(|f| {
                                f.exterior_derivative(graph).map_err(|x| x.to_string())
                            }) {
                                Ok(df) => sym_report(
                                    format!("Exterior Derivative d({form_str})"),
                                    &df,
                                    "\\mathrm{d}\\omega",
                                ),
                                Err(_) => OperationResult::error(e),
                            },
                        }
                    }
                    CartanOpKind::InteriorProduct {
                        form_str,
                        vector_field,
                        dim,
                    } => match parse(form_str, *dim) {
                        Ok(f) => {
                            let vf = VectorField {
                                dim: *dim,
                                coord_names: coords[..*dim].to_vec(),
                                components: vector_field.clone(),
                            };
                            match f.interior_product(&vf) {
                                Ok(c) => report(
                                    format!("Interior Product i_X({form_str})"),
                                    c,
                                    "\\iota_X \\omega".to_string(),
                                ),
                                Err(e) => OperationResult::error(e.to_string()),
                            }
                        }
                        Err(e) => match sym_parse(form_str, *dim).and_then(|f| {
                            f.interior_product(graph, &sym_vector(vector_field))
                                .map_err(|x| x.to_string())
                        }) {
                            Ok(c) => sym_report(
                                format!("Interior Product i_X({form_str})"),
                                &c,
                                "\\iota_X \\omega",
                            ),
                            Err(_) => OperationResult::error(e),
                        },
                    },
                    CartanOpKind::LieDerivative {
                        form_str,
                        vector_field,
                        dim,
                    } => match parse(form_str, *dim) {
                        Ok(f) => {
                            let vf = VectorField {
                                dim: *dim,
                                coord_names: coords[..*dim].to_vec(),
                                components: vector_field.clone(),
                            };
                            match f.lie_derivative(&vf) {
                                Ok(l) => report(
                                    format!("Lie Derivative L_X({form_str})"),
                                    l,
                                    "\\mathcal{L}_X \\omega".to_string(),
                                ),
                                Err(e) => OperationResult::error(e.to_string()),
                            }
                        }
                        Err(e) => match sym_parse(form_str, *dim).and_then(|f| {
                            f.lie_derivative(graph, &sym_vector(vector_field))
                                .map_err(|x| x.to_string())
                        }) {
                            Ok(l) => sym_report(
                                format!("Lie Derivative L_X({form_str})"),
                                &l,
                                "\\mathcal{L}_X \\omega",
                            ),
                            Err(_) => OperationResult::error(e),
                        },
                    },
                    CartanOpKind::HodgeStar {
                        form_str,
                        dim,
                        is_minkowski,
                    } => match parse(form_str, *dim) {
                        Ok(f) => match f.hodge_star(*is_minkowski) {
                            Ok(star) => report(
                                format!("Hodge Star ⋆({form_str})"),
                                star,
                                "\\star\\omega".to_string(),
                            ),
                            Err(e) => OperationResult::error(e.to_string()),
                        },
                        Err(e) => match sym_parse(form_str, *dim).and_then(|f| {
                            f.hodge_star(graph, *is_minkowski)
                                .map_err(|x| x.to_string())
                        }) {
                            Ok(star) => sym_report(
                                format!("Hodge Star ⋆({form_str})"),
                                &star,
                                "\\star\\omega",
                            ),
                            Err(_) => OperationResult::error(e),
                        },
                    },
                }
            }

            MathOperation::Clifford(kind) => match kind {
                CliffordOpKind::GeometricProduct {
                    sig_p,
                    sig_q,
                    sig_r,
                    a_coords,
                    b_coords,
                } => {
                    let n = sig_p + sig_q + sig_r;
                    if a_coords.len() != n || b_coords.len() != n {
                        return OperationResult::error(format!(
                            "Cl({sig_p},{sig_q},{sig_r}) vectors need {n} coordinates; got {} and {}",
                            a_coords.len(),
                            b_coords.len()
                        ));
                    }
                    // Product of two vectors: a b = a . b + a ^ b, with the metric diagonal
                    // (+1 for the first p axes, -1 for the next q, 0 for the last r).
                    let metric = |i: usize| {
                        if i < *sig_p {
                            1.0
                        } else if i < sig_p + sig_q {
                            -1.0
                        } else {
                            0.0
                        }
                    };
                    let scalar: f64 = (0..n).map(|i| metric(i) * a_coords[i] * b_coords[i]).sum();
                    let mut parts = vec![format!("{scalar}")];
                    for i in 0..n {
                        for j in (i + 1)..n {
                            let c = a_coords[i] * b_coords[j] - a_coords[j] * b_coords[i];
                            if c != 0.0 {
                                parts.push(format!("{c} e{}e{}", i + 1, j + 1));
                            }
                        }
                    }
                    let out = format!(
                        "Cl({sig_p},{sig_q},{sig_r}) geometric product a b = {}",
                        parts.join(" + ")
                    );
                    OperationResult::success(
                        out.clone(),
                        format!(
                            "a b = a \\cdot b + a \\wedge b \\in \\operatorname{{Cl}}({sig_p},{sig_q},{sig_r})"
                        ),
                        out,
                    )
                }
                CliffordOpKind::RotorSandwich {
                    sig_p,
                    sig_q,
                    sig_r,
                    vector,
                    angle_rad,
                    bivector_plane,
                } => {
                    let n = sig_p + sig_q + sig_r;
                    let (i, j) = *bivector_plane;
                    if *sig_q != 0 || *sig_r != 0 {
                        return OperationResult::error(
                            "rotor rotation is implemented for Euclidean signature Cl(p,0,0) only"
                                .to_string(),
                        );
                    }
                    if vector.len() != n || i >= n || j >= n || i == j {
                        return OperationResult::error(format!(
                            "need a {n}-component vector and two distinct plane axes below {n}"
                        ));
                    }
                    // R = cos(t/2) - sin(t/2) e_i e_j, so R v R~ turns e_i toward e_j by t.
                    let (c, sn) = (angle_rad.cos(), angle_rad.sin());
                    let mut rotated = vector.clone();
                    rotated[i] = c * vector[i] - sn * vector[j];
                    rotated[j] = sn * vector[i] + c * vector[j];
                    let list = rotated
                        .iter()
                        .map(|v| format!("{v:.6}"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let out = format!(
                        "Clifford Rotor Rotation Cl({sig_p},{sig_q},{sig_r}): angle = {angle_rad:.4} rad in plane (e{}, e{}): v' = ({list})",
                        i + 1,
                        j + 1
                    );
                    OperationResult::success(
                        out.clone(),
                        format!(
                            "v' = R v R^\\dagger \\in \\operatorname{{Cl}}({sig_p},{sig_q},{sig_r})"
                        ),
                        out,
                    )
                }
            },

            MathOperation::Certified(kind) => match kind {
                CertifiedOpKind::Simplify { expression, eps } => match parser.parse(expression) {
                    Ok(expr_id) => match Simplifier::simplify_certified(graph, expr_id, *eps) {
                        Ok(cert_sol) => {
                            let simp_id = *cert_sol.as_ref();
                            let out_unicode =
                                unicode_fmt.format(graph, simp_id).unwrap_or_default();
                            let out_latex = latex_fmt.format(graph, simp_id).unwrap_or_default();
                            let text = format!("{} [Certified: {}]", out_unicode, cert_sol);
                            let mut res = OperationResult::success(text, out_latex, out_unicode);
                            res.root_expr = Some(simp_id);
                            res
                        }
                        Err(e) => OperationResult::error(e.to_string()),
                    },
                    Err(e) => OperationResult::error(e.to_string()),
                },
                CertifiedOpKind::Solve { equation, var, eps } => {
                    let wrt_sym = graph.symbols.get_or_intern(var);
                    match parser.parse(equation) {
                        Ok(eq_id) => {
                            match SymbolicSolver::solveset_certified(graph, eq_id, wrt_sym, *eps) {
                                Ok(cert_roots) => {
                                    let mut root_strs = Vec::new();
                                    for root in cert_roots.as_ref() {
                                        root_strs.push(
                                            unicode_fmt.format(graph, *root).unwrap_or_default(),
                                        );
                                    }
                                    let conf = cert_roots.confidence();
                                    let out = format!(
                                        "Certified Roots for {}: [{}] (Confidence: {:.2}%)",
                                        var,
                                        root_strs.join(", "),
                                        conf * 100.0
                                    );
                                    OperationResult::success(
                                        out.clone(),
                                        format!("\\text{{{}}}", out),
                                        out,
                                    )
                                }
                                Err(e) => OperationResult::error(e.to_string()),
                            }
                        }
                        Err(e) => OperationResult::error(e.to_string()),
                    }
                }
                CertifiedOpKind::VerifyZero { expression, eps } => match parser.parse(expression) {
                    Ok(expr_id) => {
                        let cert = ProbabilisticVerifier::verify_zero_schwartz_zippel(
                            graph, expr_id, *eps,
                        );
                        let is_zero = *cert.as_ref();
                        let out = format!("Expression == 0: {} ({})", is_zero, cert);
                        OperationResult::success(
                            out.clone(),
                            format!("f(x) \\stackrel{{?}}{{=}} 0: {} \\; ({})", is_zero, cert),
                            out,
                        )
                    }
                    Err(e) => OperationResult::error(e.to_string()),
                },
                CertifiedOpKind::VerifyEquivalence { lhs, rhs, eps } => {
                    match (parser.parse(lhs), parser.parse(rhs)) {
                        (Ok(lhs_id), Ok(rhs_id)) => {
                            let cert = ProbabilisticVerifier::verify_equivalence(
                                graph, lhs_id, rhs_id, *eps,
                            );
                            let is_equiv = *cert.as_ref();
                            let out = format!("{} ≡ {}: {} ({})", lhs, rhs, is_equiv, cert);
                            OperationResult::success(
                                out.clone(),
                                format!("{} \\equiv {}: {} \\; ({})", lhs, rhs, is_equiv, cert),
                                out,
                            )
                        }
                        (Err(e), _) | (_, Err(e)) => OperationResult::error(e.to_string()),
                    }
                }
            },

            MathOperation::BlockDiagram(kind) => match kind {
                BlockDiagramOpKind::Feedback { g_expr, h_expr } => {
                    match (parser.parse(g_expr), parser.parse(h_expr)) {
                        (Ok(g_id), Ok(h_id)) => {
                            let one = graph.integer(1);
                            let gh = graph.mul([g_id, h_id]);
                            let den = graph.add([one, gh]);
                            let closed_loop = graph.div(g_id, den);
                            let simp = graph.simplify(closed_loop);
                            let unicode = unicode_fmt.format(graph, simp).unwrap_or_default();
                            let latex = latex_fmt.format(graph, simp).unwrap_or_default();
                            let out = format!("Closed Loop T(s) = G / (1 + G H) = {}", unicode);
                            OperationResult::success(out, format!("T(s) = {}", latex), unicode)
                        }
                        (Err(e), _) | (_, Err(e)) => OperationResult::error(e.to_string()),
                    }
                }
                BlockDiagramOpKind::Series { g1_expr, g2_expr } => {
                    match (parser.parse(g1_expr), parser.parse(g2_expr)) {
                        (Ok(g1_id), Ok(g2_id)) => {
                            let series = graph.mul([g1_id, g2_id]);
                            let simp = graph.simplify(series);
                            let unicode = unicode_fmt.format(graph, simp).unwrap_or_default();
                            let latex = latex_fmt.format(graph, simp).unwrap_or_default();
                            let out = format!("Series Cascade T(s) = G1 · G2 = {}", unicode);
                            OperationResult::success(out, format!("T(s) = {}", latex), unicode)
                        }
                        (Err(e), _) | (_, Err(e)) => OperationResult::error(e.to_string()),
                    }
                }
                BlockDiagramOpKind::Parallel { g1_expr, g2_expr } => {
                    match (parser.parse(g1_expr), parser.parse(g2_expr)) {
                        (Ok(g1_id), Ok(g2_id)) => {
                            let parallel = graph.add([g1_id, g2_id]);
                            let simp = graph.simplify(parallel);
                            let unicode = unicode_fmt.format(graph, simp).unwrap_or_default();
                            let latex = latex_fmt.format(graph, simp).unwrap_or_default();
                            let out = format!("Parallel Sum T(s) = G1 + G2 = {}", unicode);
                            OperationResult::success(out, format!("T(s) = {}", latex), unicode)
                        }
                        (Err(e), _) | (_, Err(e)) => OperationResult::error(e.to_string()),
                    }
                }
            },

            MathOperation::Pde(kind) => match kind {
                PdeOpKind::Wave1D {
                    speed,
                    x_var,
                    t_var,
                    initial_pos,
                    initial_vel,
                } => {
                    let x_sym = graph.symbols.get_or_intern(x_var);
                    let t_sym = graph.symbols.get_or_intern(t_var);
                    let c_node = match parser.parse(speed) {
                        Ok(id) => id,
                        Err(_) => graph.symbol(speed),
                    };

                    if let Some(pos_str) = initial_pos {
                        match parser.parse(pos_str) {
                            Ok(f_id) => {
                                let g_id = initial_vel
                                    .as_ref()
                                    .and_then(|v_str| parser.parse(v_str).ok());
                                match graph
                                    .solve_pde_wave_dalembert(f_id, g_id, c_node, x_sym, t_sym)
                                {
                                    Ok(sol_id) => {
                                        let unicode =
                                            unicode_fmt.format(graph, sol_id).unwrap_or_default();
                                        let latex =
                                            latex_fmt.format(graph, sol_id).unwrap_or_default();
                                        let out = format!(
                                            "Wave Equation d'Alembert Solution u({}, {}): {}",
                                            x_var, t_var, unicode
                                        );
                                        OperationResult::success(
                                            out,
                                            format!("u({}, {}) = {}", x_var, t_var, latex),
                                            unicode,
                                        )
                                    }
                                    Err(e) => OperationResult::error(e.to_string()),
                                }
                            }
                            Err(e) => OperationResult::error(e.to_string()),
                        }
                    } else {
                        match graph.solve_pde_wave_1d(c_node, x_sym, t_sym) {
                            Ok((x_sol, t_sol)) => {
                                let total = graph.mul([x_sol, t_sol]);
                                let simp = graph.simplify(total);
                                let unicode = unicode_fmt.format(graph, simp).unwrap_or_default();
                                let latex = latex_fmt.format(graph, simp).unwrap_or_default();
                                let out = format!(
                                    "Wave Equation Separated Mode u({}, {}): {}",
                                    x_var, t_var, unicode
                                );
                                OperationResult::success(
                                    out,
                                    format!("u({}, {}) = {}", x_var, t_var, latex),
                                    unicode,
                                )
                            }
                            Err(e) => OperationResult::error(e.to_string()),
                        }
                    }
                }
                PdeOpKind::Heat1D {
                    alpha,
                    x_var,
                    t_var,
                    length: _,
                } => {
                    let x_sym = graph.symbols.get_or_intern(x_var);
                    let t_sym = graph.symbols.get_or_intern(t_var);
                    let alpha_node = match parser.parse(alpha) {
                        Ok(id) => id,
                        Err(_) => graph.symbol(alpha),
                    };
                    match graph.solve_pde_heat_1d(alpha_node, x_sym, t_sym) {
                        Ok((x_sol, t_sol)) => {
                            let total = graph.mul([x_sol, t_sol]);
                            let simp = graph.simplify(total);
                            let unicode = unicode_fmt.format(graph, simp).unwrap_or_default();
                            let latex = latex_fmt.format(graph, simp).unwrap_or_default();
                            let out = format!(
                                "Heat Equation Eigenmode u({}, {}): {}",
                                x_var, t_var, unicode
                            );
                            OperationResult::success(
                                out,
                                format!("u({}, {}) = {}", x_var, t_var, latex),
                                unicode,
                            )
                        }
                        Err(e) => OperationResult::error(e.to_string()),
                    }
                }
                PdeOpKind::Laplace2D {
                    x_var,
                    y_var,
                    a_bound,
                    b_bound: _,
                } => {
                    let x_sym = graph.symbols.get_or_intern(x_var);
                    let y_sym = graph.symbols.get_or_intern(y_var);
                    let a_node = match a_bound {
                        Some(a_str) => parser.parse(a_str).unwrap_or_else(|_| graph.symbol(a_str)),
                        None => graph.symbol("a"),
                    };
                    match graph.solve_pde_laplace_2d(a_node, x_sym, y_sym) {
                        Ok((x_sol, y_sol)) => {
                            let total = graph.mul([x_sol, y_sol]);
                            let simp = graph.simplify(total);
                            let unicode = unicode_fmt.format(graph, simp).unwrap_or_default();
                            let latex = latex_fmt.format(graph, simp).unwrap_or_default();
                            let out = format!(
                                "Laplace Equation Harmonic Mode u({}, {}): {}",
                                x_var, y_var, unicode
                            );
                            OperationResult::success(
                                out,
                                format!("u({}, {}) = {}", x_var, y_var, latex),
                                unicode,
                            )
                        }
                        Err(e) => OperationResult::error(e.to_string()),
                    }
                }
                PdeOpKind::Transport1D {
                    speed,
                    x_var,
                    t_var,
                    initial_state,
                } => {
                    let x_sym = graph.symbols.get_or_intern(x_var);
                    let t_sym = graph.symbols.get_or_intern(t_var);
                    let c_node = match parser.parse(speed) {
                        Ok(id) => id,
                        Err(_) => graph.symbol(speed),
                    };
                    let f_id = match initial_state {
                        Some(s) => parser.parse(s).unwrap_or_else(|_| graph.symbol(s)),
                        None => graph.function("f", [graph.symbol(x_var)]),
                    };
                    match graph.solve_pde_transport_1d(f_id, c_node, x_sym, t_sym) {
                        Ok(sol_id) => {
                            let unicode = unicode_fmt.format(graph, sol_id).unwrap_or_default();
                            let latex = latex_fmt.format(graph, sol_id).unwrap_or_default();
                            let out = format!(
                                "Transport Equation Characteristic u({}, {}): {}",
                                x_var, t_var, unicode
                            );
                            OperationResult::success(
                                out,
                                format!("u({}, {}) = {}", x_var, t_var, latex),
                                unicode,
                            )
                        }
                        Err(e) => OperationResult::error(e.to_string()),
                    }
                }
                PdeOpKind::RadialBessel {
                    wave_num,
                    r_var,
                    order,
                } => {
                    let r_sym = graph.symbols.get_or_intern(r_var);
                    let k_node = match parser.parse(wave_num) {
                        Ok(id) => id,
                        Err(_) => graph.symbol(wave_num),
                    };
                    match graph.solve_pde_radial_bessel(k_node, r_sym, *order) {
                        Ok(sol_id) => {
                            let unicode = unicode_fmt.format(graph, sol_id).unwrap_or_default();
                            let latex = latex_fmt.format(graph, sol_id).unwrap_or_default();
                            let out = format!(
                                "Radial Laplacian Bessel Eigenmode R_{}({}): {}",
                                order, r_var, unicode
                            );
                            OperationResult::success(
                                out,
                                format!("R_{{{}}}({}) = {}", order, r_var, latex),
                                unicode,
                            )
                        }
                        Err(e) => OperationResult::error(e.to_string()),
                    }
                }
            },

            MathOperation::DiffEq(kind) => match kind {
                DiffEqOpKind::ClassifyAndSolve {
                    equation,
                    dependent_var,
                    independent_vars,
                    override_kind,
                } => {
                    let manual_kind =
                        override_kind
                            .as_deref()
                            .and_then(|s| match s.to_lowercase().as_str() {
                                "ode" => Some(DiffEqKind::Ode),
                                "pde" => Some(DiffEqKind::Pde),
                                _ => None,
                            });
                    let dep_name = dependent_var.as_deref().unwrap_or("y");
                    let indep_name = independent_vars
                        .as_ref()
                        .and_then(|v| v.first().map(|s| s.as_str()))
                        .unwrap_or("x");
                    // Replace prime notation dep'' and dep' cleanly for classification and solving
                    let prime2_pat = format!("{}''", dep_name);
                    let prime1_pat = format!("{}'", dep_name);
                    let eq_for_solve = equation
                        .replace(&prime2_pat, "__ode_d2")
                        .replace(&prime1_pat, "__ode_d1");
                    let eq_for_cls = equation
                        .replace(
                            &prime2_pat,
                            &format!("diff({}, {}, 2)", dep_name, indep_name),
                        )
                        .replace(&prime1_pat, &format!("diff({}, {})", dep_name, indep_name));

                    let parse_side_by_side =
                        |s: &str| -> Result<ExprId, algebra_core::parser::ParseError> {
                            if let Some((lhs_str, rhs_str)) = s.split_once('=') {
                                let l = parser.parse(lhs_str.trim())?;
                                let r = parser.parse(rhs_str.trim())?;
                                Ok(graph.sub(l, r))
                            } else {
                                parser.parse(s.trim())
                            }
                        };

                    if let Ok(ode_res) = parse_side_by_side(&eq_for_solve) {
                        let d2_sym = graph.symbols.get_or_intern("__ode_d2");
                        let d1_sym = graph.symbols.get_or_intern("__ode_d1");
                        let d0_sym = graph.symbols.get_or_intern(dep_name);
                        let x_sym = graph.symbols.get_or_intern(indep_name);
                        let zero = graph.integer(0);
                        let one = graph.integer(1);

                        let eval_coeffs = |v2: ExprId, v1: ExprId, v0: ExprId| -> ExprId {
                            let e1 = graph.substitute(ode_res, d2_sym, v2);
                            let e2 = graph.substitute(e1, d1_sym, v1);
                            let e3 = graph.substitute(e2, d0_sym, v0);
                            Simplifier::simplify(graph, e3).unwrap_or(e3)
                        };
                        let base = eval_coeffs(zero, zero, zero);
                        let c2 = Simplifier::simplify(
                            graph,
                            graph.sub(eval_coeffs(one, zero, zero), base),
                        )
                        .unwrap_or_else(|_| graph.sub(eval_coeffs(one, zero, zero), base));
                        let c1 = Simplifier::simplify(
                            graph,
                            graph.sub(eval_coeffs(zero, one, zero), base),
                        )
                        .unwrap_or_else(|_| graph.sub(eval_coeffs(zero, one, zero), base));
                        let c0 = Simplifier::simplify(
                            graph,
                            graph.sub(eval_coeffs(zero, zero, one), base),
                        )
                        .unwrap_or_else(|_| graph.sub(eval_coeffs(zero, zero, one), base));

                        let eval_ctx = EvalContext::default();
                        let solved_opt = if c2 == zero && c1 != zero {
                            let p = Simplifier::simplify(graph, graph.div(c0, c1))
                                .unwrap_or_else(|_| graph.div(c0, c1));
                            let q = Simplifier::simplify(graph, graph.div(graph.neg(base), c1))
                                .unwrap_or_else(|_| graph.div(graph.neg(base), c1));
                            crate::ode::SymbolicOdeSolver::solve_first_order_linear(
                                graph, p, q, x_sym,
                            )
                            .ok()
                        } else if c2 != zero && base == zero {
                            if let (Ok(a_v), Ok(b_v), Ok(c_v)) = (
                                graph.evalf(c2, &eval_ctx).map(|v| v.to_f64()),
                                graph.evalf(c1, &eval_ctx).map(|v| v.to_f64()),
                                graph.evalf(c0, &eval_ctx).map(|v| v.to_f64()),
                            ) && a_v.is_finite()
                                && b_v.is_finite()
                                && c_v.is_finite()
                            {
                                crate::ode::SymbolicOdeSolver::solve_second_order_const_coeff_homogeneous(
                                    graph, a_v, b_v, c_v, x_sym,
                                )
                                .ok()
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        if let Some(sol_id) = solved_opt {
                            let simp_sol = Simplifier::simplify(graph, sol_id).unwrap_or(sol_id);
                            let u_sol = unicode_fmt.format(graph, simp_sol).unwrap_or_default();
                            let l_sol = latex_fmt.format(graph, simp_sol).unwrap_or_default();
                            let out = format!("{}({}) = {}", dep_name, indep_name, u_sol);
                            let latex = format!("{}({}) = {}", dep_name, indep_name, l_sol);
                            let mut res = OperationResult::success(out.clone(), latex, out);
                            res.root_expr = Some(simp_sol);
                            return res;
                        }
                    }

                    match parse_side_by_side(&eq_for_cls) {
                        Ok(eq_id) => {
                            let desc = DiffEqClassifier::classify(graph, eq_id, manual_kind);
                            let kind_str = match desc.kind {
                                DiffEqKind::Ode => "ODE (Ordinary Differential Equation)",
                                DiffEqKind::Pde => "PDE (Partial Differential Equation)",
                            };
                            let lin_str = match desc.linearity {
                                crate::diffeq::DiffEqLinearity::Linear => "Linear",
                                crate::diffeq::DiffEqLinearity::Semilinear => "Semilinear",
                                crate::diffeq::DiffEqLinearity::Quasilinear => "Quasilinear",
                                crate::diffeq::DiffEqLinearity::Nonlinear => "Nonlinear",
                            };
                            let out = format!(
                                "Differential Equation Analysis:\n  • Classification: {}\n  • Order: {}\n  • Linearity: {}\n  • Dependent Variable: {}\n  • Independent Variable(s): {:?}\n  • Recommended Strategy: {}",
                                kind_str,
                                desc.order,
                                lin_str,
                                desc.dependent_var,
                                desc.independent_vars,
                                desc.recommended_method
                            );
                            OperationResult::success(out.clone(), out.clone(), out)
                        }
                        Err(e) => OperationResult::error(e.to_string()),
                    }
                }
            },

            MathOperation::Proof {
                expression,
                target: _,
            } => {
                let (lhs_res, rhs_res) = if let Some((l, r)) = expression.split_once('=') {
                    (parser.parse(l.trim()), Some(parser.parse(r.trim())))
                } else {
                    (parser.parse(expression.trim()), None)
                };
                match (lhs_res, rhs_res) {
                    (Ok(lhs_id), Some(Ok(rhs_id))) => {
                        match verify_equivalence_pair(graph, lhs_id, rhs_id, &ctx.assumptions) {
                            Ok((sym_exact, detail)) => {
                                let out = format!("Verified: {} ({})", expression, detail);
                                let latex = format!(
                                    "\\text{{Verified ({}): }} {}",
                                    if sym_exact {
                                        "symbolic"
                                    } else {
                                        "Schwartz-Zippel"
                                    },
                                    expression
                                );
                                OperationResult::success(out.clone(), latex, out)
                            }
                            Err(msg) => OperationResult::error(msg),
                        }
                    }
                    (Ok(id), None) => {
                        let zero = graph.integer(0);
                        match verify_equivalence_pair(graph, id, zero, &ctx.assumptions) {
                            Ok((_, detail)) => {
                                let out = format!("Verified: {} == 0 ({})", expression, detail);
                                OperationResult::success(
                                    out.clone(),
                                    format!("\\text{{{}}}", out),
                                    out,
                                )
                            }
                            Err(msg) => OperationResult::error(msg),
                        }
                    }
                    (Err(e), _) | (_, Some(Err(e))) => OperationResult::error(e.to_string()),
                }
            }

            MathOperation::Compare { left, right, kind } => {
                match (parser.parse(left.trim()), parser.parse(right.trim())) {
                    (Ok(lhs_id), Ok(rhs_id)) => {
                        if *kind == ComparisonKind::Isomorphic {
                            let l_s = AlgebraicTransformations::expand(graph, lhs_id);
                            let r_s = AlgebraicTransformations::expand(graph, rhs_id);
                            let eq = l_s == r_s
                                || verify_equivalence_pair(graph, lhs_id, rhs_id, &ctx.assumptions)
                                    .is_ok();
                            let out = format!(
                                "{} {} {} ({})",
                                left,
                                if eq { "≅" } else { "≇" },
                                right,
                                if eq { "Equivalent" } else { "Not equivalent" }
                            );
                            return OperationResult::success(out.clone(), out.clone(), out);
                        }
                        match verify_equivalence_pair(graph, lhs_id, rhs_id, &ctx.assumptions) {
                            Ok((_, detail)) => {
                                let out = format!("Equal: {} == {} ({})", left, right, detail);
                                let latex = format!(
                                    "{} = {}",
                                    latex_fmt
                                        .format(graph, lhs_id)
                                        .unwrap_or_else(|_| left.clone()),
                                    latex_fmt
                                        .format(graph, rhs_id)
                                        .unwrap_or_else(|_| right.clone())
                                );
                                OperationResult::success(out.clone(), latex, out)
                            }
                            Err(detail) => {
                                let out = format!("Not equal: {} != {} ({})", left, right, detail);
                                OperationResult::success(out.clone(), out.clone(), out)
                            }
                        }
                    }
                    (Err(e), _) | (_, Err(e)) => OperationResult::error(e.to_string()),
                }
            }

            MathOperation::Distribution {
                name,
                dist_type,
                params,
            } => {
                let out = format!("{} ~ {}({})", name, dist_type, params.join(", "));
                OperationResult::success(
                    out.clone(),
                    format!(
                        "{} \\sim \\text{{{}}}({})",
                        name,
                        dist_type,
                        params.join(", ")
                    ),
                    out,
                )
            }

            MathOperation::Declaration {
                name,
                role,
                value,
                unit,
            } => {
                let unit_s = unit
                    .as_ref()
                    .map(|u| format!(" [{}]", u))
                    .unwrap_or_default();
                let val_s = value.map(|v| format!(" = {:.4}", v)).unwrap_or_default();
                let out = format!("{}: {:?}{}{}", name, role, val_s, unit_s);
                OperationResult::success(out.clone(), out.clone(), out)
            }

            MathOperation::UncertainDeclaration {
                name,
                role,
                value,
                uncertainty,
                unit,
            } => {
                let unit_s = unit
                    .as_ref()
                    .map(|u| format!(" [{}]", u))
                    .unwrap_or_default();
                let lo = value - uncertainty.abs();
                let hi = value + uncertainty.abs();
                let out = format!(
                    "{}: {:?} = {} ± {} (interval [{}, {}]){}",
                    name, role, value, uncertainty, lo, hi, unit_s
                );
                let latex = format!("{} = {} \\pm {}{}", name, value, uncertainty, unit_s);
                let mut res = OperationResult::success(out.clone(), latex, out);
                res.metadata
                    .insert("value".to_string(), format!("{}", value));
                res.metadata
                    .insert("uncertainty".to_string(), format!("{}", uncertainty));
                res.metadata
                    .insert("interval_inf".to_string(), format!("{}", lo));
                res.metadata
                    .insert("interval_sup".to_string(), format!("{}", hi));
                res
            }

            MathOperation::Branch(kind) => execute_branch_op(graph, &parser, kind, ctx),

            MathOperation::SetBuilder {
                var_name,
                domain_type,
                condition,
            } => {
                let out = format!("{{ {} in {} | {} }}", var_name, domain_type, condition);
                OperationResult::success(
                    out.clone(),
                    format!(
                        "\\{{ {} \\in {} \\mid {} \\}}",
                        var_name, domain_type, condition
                    ),
                    out,
                )
            }

            MathOperation::DomainRestriction(bound) => {
                let min_s = bound.min_val.unwrap_or(-10.0);
                let max_s = bound.max_val.unwrap_or(10.0);
                let display_unicode = match bound.domain_type.as_str() {
                    "Reals" if bound.min_val.is_some() && bound.max_val.is_some() => {
                        format!("∀ {} ∈ ℝ ∩ [{:.2}, {:.2}]", bound.name, min_s, max_s)
                    }
                    "Integers" if bound.min_val.is_some() && bound.max_val.is_some() => {
                        format!("∀ {} ∈ ℤ ∩ [{:.0}, {:.0}]", bound.name, min_s, max_s)
                    }
                    "Positive" => format!("∀ {} ∈ ℝ⁺", bound.name),
                    "NonNegative" => format!("∀ {} ∈ ℝ⁺₀", bound.name),
                    "Integers" => format!("∀ {} ∈ ℤ", bound.name),
                    "Complex" => format!("∀ {} ∈ ℂ", bound.name),
                    _ => format!("∀ {} ∈ {}", bound.name, bound.domain_type),
                };
                let mut res = OperationResult::success(
                    display_unicode.clone(),
                    format!("\\text{{{}}}", display_unicode),
                    display_unicode,
                );
                res.domain_info = Some(format!(
                    "Domain Bound: {} in {}",
                    bound.name, bound.domain_type
                ));
                res
            }

            MathOperation::SetContext { key, value } => {
                let out = format!("Active context '{}' set to '{}'.", key, value);
                OperationResult::success(out.clone(), out.clone(), out)
            }

            MathOperation::System(cmd) => match cmd {
                SystemCommandKind::Help => {
                    let help = [
                            "Universal Rust Algebra Engine (URAE) Command Reference:",
                            "  • diff <expr>, <var> [, <order>] - Symbolic differentiation (single/higher order)",
                            "  • integrate <expr>, <var>        - Symbolic integration (indefinite/definite)",
                            "  • solve <eq>, <var>              - Equation root finding",
                            "  • simplify / expand / factor     - Algebraic transformations",
                            "  • eval <expr>                    - Numerical reduction",
                            "  • laplace / fourier / inv_laplace- Integral transforms",
                            "  • det / inv / trace / transpose  - Linear algebra & matrices",
                            "  • factorial / combinations / bell- Discrete combinatorics",
                            "  • poly_roots / discriminant      - Polynomial analysis",
                            "  • convert 100 [km/h] to [m/s]    - Dimensional physics units",
                            "  • knuth / tetration / pentation  - Knuth up-arrows & hyperoperations",
                            "  • maxplus_add / minplus_add      - Tropical semirings",
                            "  • cf / is_prime / gcd / legendre - Computational number theory",
                            "  • stability 2x2 / fermi_dirac    - Control systems & statistical mechanics",
                            "  • sat / truth_table              - Boolean SAT & logic",
                            "  • proof <expr>                   - Formal Lean 4 & Coq proof script",
                            "  • vars / params / context        - System state inspection",
                        ].join("\n");
                    OperationResult::success(help.clone(), help.clone(), help)
                }
                SystemCommandKind::Vars | SystemCommandKind::Params => {
                    if ctx.symbol_summary.is_empty() {
                        OperationResult::success(
                            "No active notebook variables or parameters.",
                            "",
                            "",
                        )
                    } else {
                        let mut out = String::from("Active Notebook Variables & Parameters:\n");
                        for (name, role, val, unit) in &ctx.symbol_summary {
                            let unit_suffix = if unit.is_empty() {
                                String::new()
                            } else {
                                format!(" [{}]", unit)
                            };
                            let unc_suffix = ctx
                                .uncertainties
                                .get(name)
                                .filter(|&&u| u > 0.0)
                                .map(|u| format!(" ± {:.4}", u))
                                .unwrap_or_default();
                            out.push_str(&format!(
                                "  • {} ({}): {:.4}{}{}\n",
                                name, role, val, unc_suffix, unit_suffix
                            ));
                        }
                        let text = out.trim_end().to_string();
                        OperationResult::success(text.clone(), text.clone(), text)
                    }
                }
                SystemCommandKind::Context => {
                    let math_ctx = algebra_core::MathContext::default();
                    let out = format!("Active Mathematical Context:\n  • {}", math_ctx.summary());
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                SystemCommandKind::Log { .. } => OperationResult::error(
                    "`log` is handled by the CLI front end; the executor has no logging filter",
                ),
                SystemCommandKind::Preset { .. } => OperationResult::error(
                    "`preset` is handled by the CLI front end; the executor has no preset store",
                ),
                SystemCommandKind::Export { format, expr } => {
                    // Render the expression in the requested notation.
                    let id = match parser.parse(expr.trim()) {
                        Ok(id) => id,
                        Err(e) => return OperationResult::error(format!("export: {e:?}")),
                    };
                    let rendered = match format.to_ascii_lowercase().as_str() {
                        "latex" | "tex" => LatexFormatter.format(graph, id),
                        "unicode" | "text" | "txt" => UnicodeFormatter.format(graph, id),
                        "mathml" => algebra_core::format::MathMLFormatter.format(graph, id),
                        other => {
                            return OperationResult::error(format!(
                                "export: unknown format `{other}` (use latex, unicode or mathml)"
                            ));
                        }
                    };
                    match rendered {
                        Ok(text) => OperationResult::success(text.clone(), text.clone(), text),
                        Err(e) => OperationResult::error(format!("export: {e:?}")),
                    }
                }
                // Clearing the workspace is session state; `Session` intercepts it before dispatch.
                SystemCommandKind::Clear => OperationResult::success("Workspace cleared.", "", ""),
            },

            MathOperation::AiQuery { .. } => OperationResult::error(
                "AI queries need the agent layer (`algebra-advanced` with the `agent` feature)",
            ),
        }
    }
}

fn execute_symbolic_sum(
    graph: &ExprGraph,
    parser: &ExprParser,
    expression: &str,
    variable: &str,
    lower: &str,
    upper: &str,
) -> Result<ExprId, String> {
    let expr_id = parser.parse(expression).map_err(|e| e.to_string())?;
    let low_id = parser.parse(lower).map_err(|e| e.to_string())?;
    let up_id = parser.parse(upper).map_err(|e| e.to_string())?;
    let var_sym = graph.symbols.get_or_intern(variable);

    let as_int = |id: ExprId| -> Option<i64> {
        let simp = graph.simplify(id);
        match &graph.get(simp).kind {
            ExprKind::Number(algebra_core::Number::Integer(i)) => Some(*i),
            ExprKind::Neg(inner) => match &graph.get(*inner).kind {
                ExprKind::Number(algebra_core::Number::Integer(i)) => i.checked_neg(),
                _ => None,
            },
            _ => None,
        }
    };

    let is_inf_expr = |id: ExprId| -> bool {
        matches!(
            graph.get(id).kind,
            ExprKind::Number(algebra_core::Number::Constant(
                algebra_core::Constant::Infinity
            ))
        ) || matches!(graph.get(id).kind, ExprKind::Symbol(s) if matches!(graph.symbols.resolve(s).as_deref(), Some("oo" | "inf" | "infinity")))
    };

    // 1. Closed-form polynomial sums via exact Bernoulli/Faulhaber formula for degree <= 12
    // Run BEFORE the 10k direct loop so large finite sums like sum(k^2, k, 1, 100000) finish in O(1).
    if let Ok(poly) = UniPoly::from_expr(graph, expr_id, var_sym)
        && poly.degree() <= 12
    {
        if poly.is_zero() {
            return Ok(graph.integer(0));
        }
        let max_d = poly.degree();
        let bern = bernoulli_plus_table(max_d);

        let eval_poly_sum_at = |m_rat: &num_rational::BigRational| -> num_rational::BigRational {
            use num_bigint::BigInt;
            use num_rational::BigRational;
            use num_traits::Zero as _;
            let mut acc = BigRational::zero();
            for (p, coeff) in poly.coeffs.iter().enumerate() {
                if coeff.is_zero() {
                    continue;
                }
                let mut sp = BigRational::zero();
                let mut powers = vec![BigRational::from_integer(BigInt::from(1))];
                let mut power = BigRational::from_integer(BigInt::from(1));
                for _ in 0..=(p + 1) {
                    power = &power * m_rat;
                    powers.push(power.clone());
                }
                for j in 0..=p {
                    let bin = BigRational::from_integer(binomial_bigint(p + 1, j));
                    sp += &bin * &bern[j] * &powers[p + 1 - j];
                }
                sp /= BigRational::from_integer(BigInt::from(p + 1));
                acc += coeff * sp;
            }
            acc
        };

        if let (Some(lo), Some(hi)) = (as_int(low_id), as_int(up_id)) {
            if hi < lo {
                return Ok(graph.integer(0));
            }
            use num_bigint::BigInt;
            use num_rational::BigRational;
            let hi_r = BigRational::from_integer(BigInt::from(hi));
            let lo_m1_r = BigRational::from_integer(BigInt::from(lo - 1));
            let ans = eval_poly_sum_at(&hi_r) - eval_poly_sum_at(&lo_m1_r);
            return Ok(expr_from_rational(graph, &ans));
        }

        if !is_inf_expr(up_id) && !is_inf_expr(low_id) {
            let sum_to = |m: ExprId| -> ExprId {
                use num_bigint::BigInt;
                use num_rational::BigRational;
                use num_traits::Zero as _;
                let mut total_terms = Vec::new();
                for (p, coeff) in poly.coeffs.iter().enumerate() {
                    if coeff.is_zero() {
                        continue;
                    }
                    let denom = BigRational::from_integer(BigInt::from(p + 1));
                    for j in 0..=p {
                        let bin = BigRational::from_integer(binomial_bigint(p + 1, j));
                        let term_c = coeff * &bin * &bern[j] / &denom;
                        if term_c.is_zero() {
                            continue;
                        }
                        let exp_n = (p + 1 - j) as i64;
                        let m_pow = if exp_n == 1 {
                            m
                        } else {
                            graph.pow(m, graph.integer(exp_n))
                        };
                        let c_expr = expr_from_rational(graph, &term_c);
                        total_terms.push(if c_expr == graph.integer(1) {
                            m_pow
                        } else {
                            graph.mul([c_expr, m_pow])
                        });
                    }
                }
                if total_terms.is_empty() {
                    graph.integer(0)
                } else {
                    graph.add(total_terms)
                }
            };

            let s_hi = sum_to(up_id);
            let res = if as_int(low_id) == Some(1) {
                s_hi
            } else {
                let lo_minus_1 = graph.sub(low_id, graph.integer(1));
                let s_lo = sum_to(lo_minus_1);
                graph.sub(s_hi, s_lo)
            };
            return Ok(graph.simplify(res));
        }
    }

    // 2. Geometric series sum c * r^k (finite or infinite)
    if let Some((coeff_c, ratio_r)) = extract_geometric_series(graph, expr_id, var_sym) {
        let simp_r = graph.simplify(ratio_r);
        let one = graph.integer(1);
        if is_inf_expr(up_id) {
            let ctx = crate::numeric::EvalContext::default();
            if let Ok(r_val) = graph.evalf(simp_r, &ctx).map(|v| v.to_f64()) {
                if r_val.is_finite() && r_val.abs() < 1.0 {
                    let r_lo = graph.pow(simp_r, low_id);
                    let first = if coeff_c == one {
                        r_lo
                    } else {
                        graph.mul([coeff_c, r_lo])
                    };
                    let denom = graph.sub(one, simp_r);
                    return Ok(graph.simplify(graph.div(first, denom)));
                } else {
                    return Err(format!(
                        "Infinite geometric sum of '{}' does not converge (|r| >= 1)",
                        expression
                    ));
                }
            }
        } else if !is_inf_expr(low_id) {
            if simp_r == one {
                let count = graph.add([graph.sub(up_id, low_id), one]);
                return Ok(graph.simplify(graph.mul([coeff_c, count])));
            }
            let r_lo = graph.pow(simp_r, low_id);
            let up_p1 = graph.add([up_id, one]);
            let r_hi1 = graph.pow(simp_r, up_p1);
            let num = graph.mul([coeff_c, graph.sub(r_lo, r_hi1)]);
            let denom = graph.sub(one, simp_r);
            return Ok(graph.simplify(graph.div(num, denom)));
        }
    }

    // 3. Infinite p-series c / k^p
    if is_inf_expr(up_id)
        && let Some(lo) = as_int(low_id)
        && lo >= 1
        && let Some((coeff_c, p_exp)) = extract_p_series(graph, expr_id, var_sym)
    {
        if p_exp <= 1 {
            return Err(format!(
                "Infinite p-series '{}' diverges (p = {} <= 1)",
                expression, p_exp
            ));
        }
        let pi = graph.constant(algebra_core::Constant::Pi);
        let base_zeta = match p_exp {
            2 => Some(graph.div(graph.pow(pi, graph.integer(2)), graph.integer(6))),
            4 => Some(graph.div(graph.pow(pi, graph.integer(4)), graph.integer(90))),
            _ => None,
        };
        if let Some(zeta) = base_zeta {
            let mut prefix_sum = graph.integer(0);
            for k in 1..lo {
                let term = graph.div(graph.integer(1), graph.integer(k.pow(p_exp as u32)));
                prefix_sum = graph.add([prefix_sum, term]);
            }
            let tail = if lo == 1 {
                zeta
            } else {
                graph.sub(zeta, prefix_sum)
            };
            let res = if coeff_c == graph.integer(1) {
                tail
            } else {
                graph.mul([coeff_c, tail])
            };
            return Ok(graph.simplify(res));
        }
    }

    // 4. Direct finite evaluation fallback for non-polynomial/non-geometric expressions
    if let (Some(lo), Some(hi)) = (as_int(low_id), as_int(up_id)) {
        if hi < lo {
            return Ok(graph.integer(0));
        }
        let span = hi.saturating_sub(lo);
        if span <= 10_000 {
            let mut acc = Vec::with_capacity((span + 1) as usize);
            for k in lo..=hi {
                let k_id = graph.integer(k);
                let term = graph.substitute(expr_id, var_sym, k_id);
                acc.push(graph.simplify(term));
            }
            return Ok(graph.add(acc));
        }
        return Err(format!(
            "Finite sum span ({} terms) exceeds direct evaluation cap (10000)",
            span + 1
        ));
    }

    Err(format!(
        "Symbolic sum closed form unavailable for '{}' from {} to {}",
        expression, lower, upper
    ))
}

fn solution_satisfies_assumptions(
    graph: &ExprGraph,
    sol: ExprId,
    var_sym: algebra_core::SymbolId,
    assumptions: &crate::logic::AssumptionsContext,
    eval_ctx: &crate::numeric::EvalContext,
) -> bool {
    use crate::logic::Predicate;
    use crate::numeric::{BigValue, NumericalEval as _};

    let Ok(val) = graph.evalf(sol, eval_ctx) else {
        return true;
    };
    let (re, im) = match val {
        BigValue::Real(r) => (r, 0.0),
        BigValue::Complex(r, i) => (r, i),
    };
    if !re.is_finite() || !im.is_finite() {
        return false;
    }
    if assumptions.is(var_sym, Predicate::Real) && im.abs() > 1e-9 {
        return false;
    }
    if assumptions.is(var_sym, Predicate::Positive) && (im.abs() > 1e-9 || re <= 1e-12) {
        return false;
    }
    if assumptions.is(var_sym, Predicate::NonNegative) && (im.abs() > 1e-9 || re < -1e-12) {
        return false;
    }
    if assumptions.is(var_sym, Predicate::Negative) && (im.abs() > 1e-9 || re >= -1e-12) {
        return false;
    }
    if assumptions.is(var_sym, Predicate::NonZero) && re.abs() < 1e-12 && im.abs() < 1e-12 {
        return false;
    }
    if assumptions.is(var_sym, Predicate::Integer)
        && (im.abs() > 1e-9 || (re - re.round()).abs() > 1e-9)
    {
        return false;
    }
    if let Some(bounds) = assumptions.get_bounds(var_sym) {
        if im.abs() > 1e-9 || !bounds.contains(re) {
            return false;
        }
    }
    true
}

fn binomial_bigint(n: usize, k: usize) -> num_bigint::BigInt {
    use num_bigint::BigInt;
    let k = k.min(n - k);
    let mut res = BigInt::from(1u8);
    for i in 0..k {
        res = (res * BigInt::from(n - i)) / BigInt::from(i + 1);
    }
    res
}

fn bernoulli_plus_table(max_d: usize) -> Vec<num_rational::BigRational> {
    use num_bigint::BigInt;
    use num_rational::BigRational;
    let one = BigRational::from_integer(BigInt::from(1u8));
    let mut b = vec![one.clone()];
    for m in 1..=max_d {
        let mut sum = BigRational::from_integer(BigInt::from(0u8));
        for k in 0..m {
            let c = BigRational::from_integer(binomial_bigint(m, k));
            let denom = BigRational::from_integer(BigInt::from((m - k + 1) as i64));
            sum += c * &b[k] / denom;
        }
        b.push(&one - sum);
    }
    b
}

fn extract_geometric_series(
    graph: &ExprGraph,
    expr_id: ExprId,
    var_sym: algebra_core::SymbolId,
) -> Option<(ExprId, ExprId)> {
    let one = graph.integer(1);
    if !graph.has_symbol(expr_id, var_sym) {
        return None;
    }
    match &graph.get(expr_id).kind {
        ExprKind::Pow(base, exp)
            if !graph.has_symbol(*base, var_sym) && graph.has_symbol(*exp, var_sym) =>
        {
            let lin = UniPoly::from_expr(graph, *exp, var_sym).ok()?;
            if lin.degree() != 1 {
                return None;
            }
            let c0 = expr_from_rational(graph, &lin.coeffs[0]);
            let c1 = expr_from_rational(graph, &lin.coeffs[1]);
            let coeff_c = if c0 == graph.integer(0) {
                one
            } else {
                graph.pow(*base, c0)
            };
            let ratio_r = if c1 == one {
                *base
            } else if c1 == graph.integer(-1) {
                graph.div(one, *base)
            } else {
                graph.pow(*base, c1)
            };
            Some((coeff_c, ratio_r))
        }
        ExprKind::Div(num, den) => {
            if !graph.has_symbol(*num, var_sym) {
                let (c_den, r_den) = extract_geometric_series(graph, *den, var_sym)?;
                return Some((graph.div(*num, c_den), graph.div(one, r_den)));
            }
            None
        }
        ExprKind::Mul(factors) => {
            let mut coeff_c = one;
            let mut ratio_r = one;
            let mut saw_geo = false;
            for &f in factors {
                if !graph.has_symbol(f, var_sym) {
                    coeff_c = if coeff_c == one {
                        f
                    } else {
                        graph.mul([coeff_c, f])
                    };
                } else {
                    let (fc, fr) = extract_geometric_series(graph, f, var_sym)?;
                    coeff_c = if coeff_c == one {
                        fc
                    } else {
                        graph.mul([coeff_c, fc])
                    };
                    ratio_r = if ratio_r == one {
                        fr
                    } else {
                        graph.mul([ratio_r, fr])
                    };
                    saw_geo = true;
                }
            }
            if saw_geo {
                Some((coeff_c, ratio_r))
            } else {
                None
            }
        }
        ExprKind::Neg(inner) => {
            let (c, r) = extract_geometric_series(graph, *inner, var_sym)?;
            Some((graph.neg(c), r))
        }
        _ => None,
    }
}

fn extract_p_series(
    graph: &ExprGraph,
    expr_id: ExprId,
    var_sym: algebra_core::SymbolId,
) -> Option<(ExprId, i64)> {
    let as_sym = matches!(graph.get(expr_id).kind, ExprKind::Symbol(s) if s == var_sym);
    if as_sym {
        return Some((graph.integer(1), -1));
    }
    match &graph.get(expr_id).kind {
        ExprKind::Div(num, den) if !graph.has_symbol(*num, var_sym) => {
            let p_den = match &graph.get(*den).kind {
                ExprKind::Symbol(s) if *s == var_sym => Some(1i64),
                ExprKind::Pow(b, e) if matches!(graph.get(*b).kind, ExprKind::Symbol(s) if s == var_sym) => {
                    match &graph.get(*e).kind {
                        ExprKind::Number(algebra_core::Number::Integer(p)) => Some(*p),
                        _ => None,
                    }
                }
                _ => None,
            }?;
            Some((*num, p_den))
        }
        ExprKind::Pow(b, e) if matches!(graph.get(*b).kind, ExprKind::Symbol(s) if s == var_sym) => {
            let p = match &graph.get(*e).kind {
                ExprKind::Number(algebra_core::Number::Integer(k)) if *k < 0 => Some(-*k),
                ExprKind::Neg(inner) => match &graph.get(*inner).kind {
                    ExprKind::Number(algebra_core::Number::Integer(k)) if *k > 0 => Some(*k),
                    _ => None,
                },
                _ => None,
            }?;
            Some((graph.integer(1), p))
        }
        ExprKind::Mul(factors) => {
            let mut coeff = graph.integer(1);
            let mut total_p = 0i64;
            for &f in factors {
                if !graph.has_symbol(f, var_sym) {
                    coeff = if coeff == graph.integer(1) {
                        f
                    } else {
                        graph.mul([coeff, f])
                    };
                } else {
                    let (fc, fp) = extract_p_series(graph, f, var_sym)?;
                    coeff = if coeff == graph.integer(1) {
                        fc
                    } else {
                        graph.mul([coeff, fc])
                    };
                    total_p += fp;
                }
            }
            if total_p > 0 {
                Some((coeff, total_p))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn classify_galois_group_solvability(raw: &str) -> Result<(bool, String), String> {
    let norm: String = raw
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '_')
        .collect::<String>()
        .to_ascii_lowercase();
    if norm.is_empty() {
        return Err("Empty Galois group name".to_string());
    }

    // Check Symmetric S_n or Sym(n)
    if let Some(rest) = norm
        .strip_prefix("sym(")
        .and_then(|r| r.strip_suffix(')'))
        .or_else(|| norm.strip_prefix('s'))
        && let Ok(n) = rest.parse::<u64>()
        && n >= 1
    {
        return Ok((n <= 4, format!("Symmetric S_{}", n)));
    }

    // Check Alternating A_n or Alt(n)
    if let Some(rest) = norm
        .strip_prefix("alt(")
        .and_then(|r| r.strip_suffix(')'))
        .or_else(|| norm.strip_prefix('a'))
        && let Ok(n) = rest.parse::<u64>()
        && n >= 1
    {
        return Ok((n <= 4, format!("Alternating A_{}", n)));
    }

    // Cyclic C_n / Z_n, Dihedral D_n
    if let Some(rest) = norm.strip_prefix('c').or_else(|| norm.strip_prefix('z'))
        && let Ok(n) = rest.parse::<u64>()
        && n >= 1
    {
        return Ok((true, format!("Cyclic C_{}", n)));
    }
    if let Some(rest) = norm.strip_prefix('d')
        && let Ok(n) = rest.parse::<u64>()
        && n >= 1
    {
        return Ok((true, format!("Dihedral D_{}", n)));
    }

    // Known named finite groups
    match norm.as_str() {
        "trivial" | "1" | "c1" | "v4" | "klein" | "klein4" | "q8" | "f5" | "f20" | "agl(1,5)"
        | "gl(2,2)" | "sl(2,2)" | "psl(2,2)" | "psl(2,3)" | "sl(2,3)" | "gl(2,3)" => {
            return Ok((true, raw.trim().to_string()));
        }
        "m11" | "m12" | "m22" | "m23" | "m24" | "j1" | "j2" | "j3" | "j4" | "suz" | "monster"
        | "baby" => {
            return Ok((false, format!("Sporadic simple group {}", raw.trim())));
        }
        _ => {}
    }

    // PSL(2, q) / SL(2, q) / PGL(2, q)
    if let Some(args) = norm
        .strip_prefix("psl(2,")
        .or_else(|| norm.strip_prefix("sl(2,"))
        .or_else(|| norm.strip_prefix("pgl(2,"))
        .and_then(|r| r.strip_suffix(')'))
        && let Ok(q) = args.parse::<u64>()
    {
        return Ok((q <= 3, format!("{} (q={})", raw.trim(), q)));
    }

    Err(format!(
        "Unrecognized Galois group '{}' (supported: S_n, A_n, C_n, Z_n, D_n, V4, Q8, F20, PSL(2,q), sporadic simple groups)",
        raw.trim()
    ))
}

fn format_complex_value(re: f64, im: f64) -> String {
    let clean = |x: f64| {
        if x.abs() < 1e-13 {
            0.0
        } else {
            (x * 1e12).round() / 1e12
        }
    };
    let r = clean(re);
    let i = clean(im);
    let fmt_f = |x: f64| {
        if (x - x.round()).abs() < 1e-12 && x.abs() < 1e12 {
            format!("{}", x.round() as i64)
        } else {
            let s = format!("{:.6}", x);
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        }
    };
    if i == 0.0 {
        fmt_f(r)
    } else if r == 0.0 {
        if (i - 1.0).abs() < 1e-12 {
            "i".to_string()
        } else if (i + 1.0).abs() < 1e-12 {
            "-i".to_string()
        } else {
            format!("{}*i", fmt_f(i))
        }
    } else if i < 0.0 {
        format!("{} - {}*i", fmt_f(r), fmt_f(-i))
    } else {
        format!("{} + {}*i", fmt_f(r), fmt_f(i))
    }
}

fn parse_branch_cut_geometry(cut_str: &str) -> crate::branch::BranchCutGeometry {
    let lower = cut_str.trim().to_ascii_lowercase();
    if lower.contains("[0") && (lower.contains("inf") || lower.contains('∞')) {
        crate::branch::BranchCutGeometry::PositiveRealAxis
    } else if lower.contains("[1") && (lower.contains("inf") || lower.contains('∞')) {
        crate::branch::BranchCutGeometry::RealComplementIntervals { threshold: 1.0 }
    } else if lower.contains("[-1, 1]") || lower.contains("[-1,1]") {
        crate::branch::BranchCutGeometry::Segment {
            start: (-1.0, 0.0),
            end: (1.0, 0.0),
        }
    } else if let Ok(angle) = lower.parse::<f64>() {
        crate::branch::BranchCutGeometry::Ray {
            origin: (0.0, 0.0),
            angle_rad: angle,
        }
    } else {
        crate::branch::BranchCutGeometry::NegativeRealAxis
    }
}

fn fn_name_to_branch_kind(fn_name: &str) -> Option<crate::branch::BranchCutFunctionKind> {
    use crate::branch::BranchCutFunctionKind::*;
    match fn_name.to_ascii_lowercase().as_str() {
        "ln" | "log" => Some(Log),
        "sqrt" => Some(Sqrt),
        "asin" | "arcsin" => Some(ArcSin),
        "acos" | "arccos" => Some(ArcCos),
        "atan" | "arctan" => Some(ArcTan),
        "asinh" | "arcsinh" => Some(ArcSinh),
        "acosh" | "arccosh" => Some(ArcCosh),
        "atanh" | "arctanh" => Some(ArcTanh),
        "lambert_w" | "lambertw" | "w" => Some(LambertW),
        _ => None,
    }
}

fn execute_branch_op(
    graph: &ExprGraph,
    parser: &ExprParser<'_>,
    kind: &BranchOpKind,
    ctx: &ExecutionContext,
) -> OperationResult {
    use crate::branch::{BranchCutFunctionKind, BranchCutManager, BranchSelection};

    let eval_ctx = EvalContext::with_bindings(ctx.precision_digits, ctx.bindings.clone());
    match kind {
        BranchOpKind::EvaluateSheet {
            function,
            expression,
            sheet,
            cut,
        } => {
            let cut_geom = cut.as_deref().map(parse_branch_cut_geometry);
            let sel = match &cut_geom {
                Some(geom) => BranchSelection::CustomCut {
                    geometry: geom.clone(),
                    sheet: *sheet,
                },
                None => BranchSelection::Sheet(*sheet),
            };

            if let Some(expr_str) = expression {
                let expr_id = match parser.parse(expr_str) {
                    Ok(id) => id,
                    Err(e) => return OperationResult::error(e.to_string()),
                };
                if cut_geom.is_some()
                    && let Some(b_kind) = fn_name_to_branch_kind(function)
                {
                    let arg_id = match &graph.get(expr_id).kind {
                        ExprKind::Function { args, .. } if !args.is_empty() => args[0],
                        _ => expr_id,
                    };
                    if let Ok(val) = graph.evalf(arg_id, &eval_ctx) {
                        let (zr, zi) = val.as_complex();
                        if let Ok(mv) = BranchCutManager::eval(b_kind, zr, zi, &sel) {
                            let (re, im) = mv.primary_value();
                            let formatted = format_complex_value(re, im);
                            let mut res = OperationResult::success(
                                formatted.clone(),
                                formatted.clone(),
                                formatted,
                            );
                            res.metadata.insert("sheet".to_string(), sheet.to_string());
                            return res;
                        }
                    }
                }
                let sheet_ctx = eval_ctx.with_branch_sheet(Some(*sheet));
                return match graph.evalf(expr_id, &sheet_ctx) {
                    Ok(val) => {
                        let (re, im) = val.as_complex();
                        let formatted = format_complex_value(re, im);
                        let mut res = OperationResult::success(
                            formatted.clone(),
                            formatted.clone(),
                            formatted,
                        );
                        res.metadata.insert("sheet".to_string(), sheet.to_string());
                        res
                    }
                    Err(e) => OperationResult::error(e.to_string()),
                };
            }

            let Some(b_kind) = fn_name_to_branch_kind(function) else {
                let out = format!("Branch configuration: {} on sheet k={}", function, sheet);
                return OperationResult::success(out.clone(), out.clone(), out);
            };
            let cut_desc = cut.as_deref().unwrap_or("default");
            let (sample_z, sample_label) = match b_kind {
                BranchCutFunctionKind::LambertW => ((-0.2, 0.0), "-0.2"),
                BranchCutFunctionKind::ArcSin
                | BranchCutFunctionKind::ArcCos
                | BranchCutFunctionKind::ArcTanh => ((2.0, 0.0), "2"),
                BranchCutFunctionKind::ArcTan | BranchCutFunctionKind::ArcSinh => {
                    ((0.0, 2.0), "2*i")
                }
                _ => ((-1.0, 0.0), "-1"),
            };
            match BranchCutManager::eval(b_kind, sample_z.0, sample_z.1, &sel) {
                Ok(mv) => {
                    let (re, im) = mv.primary_value();
                    let out = format!(
                        "Branch {function}({sample_label}) [sheet k={sheet}, cut={cut_desc}] = {}",
                        format_complex_value(re, im)
                    );
                    let mut res = OperationResult::success(out.clone(), out.clone(), out);
                    res.metadata.insert("sheet".to_string(), sheet.to_string());
                    res
                }
                Err(e) => OperationResult::error(e.to_string()),
            }
        }
        BranchOpKind::AllBranches {
            expression,
            max_branches,
        } => {
            let expr_id = match parser.parse(expression) {
                Ok(id) => id,
                Err(e) => return OperationResult::error(e.to_string()),
            };
            let mv_res = match &graph.get(expr_id).kind {
                ExprKind::Pow(base, exp) => {
                    let b = match graph.evalf(*base, &eval_ctx) {
                        Ok(v) => v.as_complex(),
                        Err(e) => return OperationResult::error(e.to_string()),
                    };
                    let e = match graph.evalf(*exp, &eval_ctx) {
                        Ok(v) => v.as_complex(),
                        Err(e) => return OperationResult::error(e.to_string()),
                    };
                    BranchCutManager::eval_power(
                        b.0,
                        b.1,
                        e.0,
                        e.1,
                        &BranchSelection::AllSheets {
                            max_branches: *max_branches,
                        },
                    )
                }
                ExprKind::Function { name, args } if !args.is_empty() => {
                    let fn_name = graph.symbols.resolve(*name).unwrap_or_default();
                    let (zr, zi) = match graph.evalf(args[0], &eval_ctx) {
                        Ok(v) => v.as_complex(),
                        Err(e) => return OperationResult::error(e.to_string()),
                    };
                    if fn_name == "root" && args.len() >= 2 {
                        let n = match graph.evalf(args[1], &eval_ctx) {
                            Ok(v) => v.to_f64().round().max(1.0) as u32,
                            Err(e) => return OperationResult::error(e.to_string()),
                        };
                        BranchCutManager::eval(
                            BranchCutFunctionKind::Root(n),
                            zr,
                            zi,
                            &BranchSelection::AllSheets {
                                max_branches: *max_branches,
                            },
                        )
                    } else if let Some(b_kind) = fn_name_to_branch_kind(&fn_name) {
                        if matches!(b_kind, BranchCutFunctionKind::LambertW) {
                            let mut sheets = vec![0i64];
                            for m in 1..=(*max_branches as i64) {
                                if sheets.len() >= *max_branches {
                                    break;
                                }
                                sheets.push(-m);
                                if sheets.len() < *max_branches {
                                    sheets.push(m);
                                }
                            }
                            BranchCutManager::eval(
                                b_kind,
                                zr,
                                zi,
                                &BranchSelection::MultiValued { sheets },
                            )
                        } else {
                            BranchCutManager::eval(
                                b_kind,
                                zr,
                                zi,
                                &BranchSelection::AllSheets {
                                    max_branches: *max_branches,
                                },
                            )
                        }
                    } else {
                        return match graph.evalf(expr_id, &eval_ctx) {
                            Ok(val) => {
                                let out = format!("[{}]", val);
                                OperationResult::success(out.clone(), out.clone(), out)
                            }
                            Err(e) => OperationResult::error(e.to_string()),
                        };
                    }
                }
                _ => {
                    return match graph.evalf(expr_id, &eval_ctx) {
                        Ok(val) => {
                            let out = format!("[{}]", val);
                            OperationResult::success(out.clone(), out.clone(), out)
                        }
                        Err(e) => OperationResult::error(e.to_string()),
                    };
                }
            };

            match mv_res {
                Ok(mv) => {
                    let vals = mv.all_values();
                    let rendered: Vec<String> = vals
                        .iter()
                        .map(|(re, im, k)| format!("k={}: {}", k, format_complex_value(*re, *im)))
                        .collect();
                    let out = format!("Branches: [{}]", rendered.join(", "));
                    let mut res = OperationResult::success(out.clone(), out.clone(), out);
                    res.metadata
                        .insert("branch_count".to_string(), vals.len().to_string());
                    res
                }
                Err(e) => OperationResult::error(e.to_string()),
            }
        }
    }
}

fn verify_equivalence_pair(
    graph: &ExprGraph,
    lhs: ExprId,
    rhs: ExprId,
    assumptions: &crate::logic::AssumptionsContext,
) -> Result<(bool, String), String> {
    if lhs == rhs {
        return Ok((true, "structurally identical".to_string()));
    }
    let zero = graph.integer(0);
    let diff = graph.sub(lhs, rhs);
    let simp = Simplifier::simplify_with_assumptions(graph, diff, assumptions).unwrap_or(diff);
    if simp == zero {
        return Ok((true, "LHS - RHS = 0".to_string()));
    }
    let exp = AlgebraicTransformations::expand(graph, simp);
    let simp_exp = Simplifier::simplify_with_assumptions(graph, exp, assumptions).unwrap_or(exp);
    if simp_exp == zero {
        return Ok((true, "expand(LHS - RHS) = 0".to_string()));
    }

    let mut syms = std::collections::BTreeSet::new();
    collect_expr_symbols(graph, diff, &mut syms);
    let free_vars: Vec<String> = syms
        .into_iter()
        .filter_map(|s| graph.symbols.resolve(s))
        .filter(|name| !matches!(name.as_str(), "pi" | "π" | "e" | "i" | "I"))
        .collect();

    let seeds = [0.37_f64, 0.91, 1.43, 2.17];
    let mut matched = 0usize;
    for (p_idx, &base_val) in seeds.iter().enumerate() {
        let mut bindings = HashMap::new();
        for (v_idx, name) in free_vars.iter().enumerate() {
            bindings.insert(
                name.clone(),
                base_val + 0.19 * (v_idx as f64) + 0.07 * (p_idx as f64),
            );
        }
        let ctx = EvalContext::with_bindings(15, bindings.clone());
        let (Ok(lv), Ok(rv)) = (graph.evalf(lhs, &ctx), graph.evalf(rhs, &ctx)) else {
            continue;
        };
        let (lr, li) = lv.as_complex();
        let (rr, ri) = rv.as_complex();
        let err = (lr - rr).hypot(li - ri);
        if err.is_nan() {
            continue;
        }
        if err > 1e-6 {
            return Err(format!(
                "Counterexample at {:?}: |LHS - RHS| = {:.6e}",
                bindings, err
            ));
        }
        matched += 1;
        if free_vars.is_empty() {
            break;
        }
    }
    if matched > 0 {
        Ok((
            false,
            format!("{} numerical probes agree within 1e-6", matched),
        ))
    } else {
        Err("Unable to prove or evaluate equivalence".to_string())
    }
}

fn format_system_complex_root(r: &crate::poly::ComplexRoot) -> String {
    let fmt_real = |v: f64| -> String {
        let clean = if v.abs() < 1e-11 { 0.0 } else { v };
        let rounded = clean.round();
        if (clean - rounded).abs() < 1e-10 {
            format!("{}", rounded as i64)
        } else {
            let s = format!("{:.8}", clean);
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        }
    };
    let im_clean = if r.im.abs() < 1e-10 { 0.0 } else { r.im };
    if im_clean == 0.0 {
        return fmt_real(r.re);
    }
    let re_clean = if r.re.abs() < 1e-11 { 0.0 } else { r.re };
    let sign = if im_clean >= 0.0 { "+" } else { "-" };
    let im_abs = fmt_real(im_clean.abs());
    let im_term = if im_abs == "1" {
        "i".to_string()
    } else {
        format!("{}i", im_abs)
    };
    if re_clean == 0.0 {
        if im_clean < 0.0 {
            format!("-{}", im_term)
        } else {
            im_term
        }
    } else {
        format!("{} {} {}", fmt_real(re_clean), sign, im_term)
    }
}

fn execute_solve_system(
    graph: &ExprGraph,
    equations: &[String],
    variables: &[String],
    ctx: &ExecutionContext,
) -> OperationResult {
    if equations.is_empty() {
        return OperationResult::error("System solver requires at least one equation");
    }

    let parser = ExprParser::new(graph);
    let unicode_fmt = UnicodeFormatter;
    let latex_fmt = LatexFormatter;

    let mut raw_residuals = Vec::with_capacity(equations.len());
    for eq_str in equations {
        let parsed = match parser.parse(eq_str) {
            Ok(id) => id,
            Err(e) => return OperationResult::error(e.to_string()),
        };
        let residual = match &graph.get(parsed).kind {
            ExprKind::Relational { op, lhs, rhs } => {
                if *op == algebra_core::RelOp::Equal {
                    graph.sub(*lhs, *rhs)
                } else {
                    return OperationResult::error(
                        "System solver expects equations (=), got inequality",
                    );
                }
            }
            _ => parsed,
        };
        raw_residuals.push(residual);
    }

    let var_names: Vec<String> = if !variables.is_empty() {
        variables.to_vec()
    } else {
        let mut syms = std::collections::BTreeSet::new();
        for &res_id in &raw_residuals {
            collect_expr_symbols(graph, res_id, &mut syms);
        }
        let mut inferred: Vec<String> = syms
            .into_iter()
            .filter_map(|s| graph.symbols.resolve(s))
            .filter(|name| {
                !matches!(
                    name.as_str(),
                    "pi" | "π"
                        | "e"
                        | "i"
                        | "I"
                        | "tau"
                        | "τ"
                        | "oo"
                        | "inf"
                        | "Inf"
                        | "infinity"
                        | "∞"
                        | "nan"
                        | "NaN"
                ) && !ctx.bindings.contains_key(name)
            })
            .collect();
        for preferred in ["w", "z", "y", "x"] {
            if let Some(pos) = inferred.iter().position(|v| v == preferred) {
                let v = inferred.remove(pos);
                inferred.insert(0, v);
            }
        }
        inferred
    };

    if var_names.is_empty() {
        return OperationResult::error("No free variables found to solve for in system");
    }

    // Substitute active session parameter bindings (excluding solve target variables)
    let mut residuals = Vec::with_capacity(raw_residuals.len());
    for &raw_id in &raw_residuals {
        let mut sub_id = raw_id;
        for (param, &val) in &ctx.bindings {
            if !var_names.iter().any(|v| v == param) {
                let param_sym = graph.symbols.get_or_intern(param);
                let val_node = if val.fract() == 0.0 && val.abs() < 9e15 {
                    graph.integer(val as i64)
                } else {
                    graph.float(val)
                };
                sub_id = graph.substitute(sub_id, param_sym, val_node);
            }
        }
        let simp = Simplifier::simplify_with_assumptions(graph, sub_id, &ctx.assumptions)
            .unwrap_or(sub_id);
        residuals.push(simp);
    }

    let var_syms: Vec<_> = var_names
        .iter()
        .map(|v| graph.symbols.get_or_intern(v))
        .collect();
    let var_set: std::collections::HashSet<_> = var_syms.iter().copied().collect();
    let zero = graph.integer(0);

    // Check whether all equations are linear in `var_syms` and build A*x = b
    let m = residuals.len();
    let n = var_syms.len();
    let mut is_linear = true;
    let mut a_elems = Vec::with_capacity(m * n);
    let mut b_elems = Vec::with_capacity(m);

    'linear_check: for &f_i in &residuals {
        for &v_sym in &var_syms {
            let d = graph.diff(f_i, v_sym);
            let deriv = Simplifier::simplify(graph, d).unwrap_or(d);
            let mut d_syms = std::collections::BTreeSet::new();
            collect_expr_symbols(graph, deriv, &mut d_syms);
            if d_syms.iter().any(|s| var_set.contains(s)) {
                is_linear = false;
                break 'linear_check;
            }
            a_elems.push(deriv);
        }

        let mut const_term = f_i;
        for &v_sym in &var_syms {
            const_term = graph.substitute(const_term, v_sym, zero);
        }
        let const_simp = Simplifier::simplify(graph, const_term).unwrap_or(const_term);
        let mut c_syms = std::collections::BTreeSet::new();
        collect_expr_symbols(graph, const_simp, &mut c_syms);
        if c_syms.iter().any(|s| var_set.contains(s)) {
            is_linear = false;
            break 'linear_check;
        }
        let neg_c = graph.neg(const_simp);
        let b_i = Simplifier::simplify(graph, neg_c).unwrap_or(neg_c);
        b_elems.push(b_i);
    }

    let vars_header = format!("[{}]", var_names.join(", "));
    let vars_latex = var_names.join(", ");

    if is_linear {
        let a_mat = match SymbolicMatrix::new(m, n, a_elems) {
            Ok(mat) => mat,
            Err(e) => return OperationResult::error(e.to_string()),
        };
        match LinearSystemSolver::solve(graph, &a_mat, &b_elems) {
            Ok(space) => {
                let mut expr_sols = Vec::with_capacity(n);
                if !space.has_infinite_solutions {
                    for &p_i in &space.particular {
                        let s = Simplifier::simplify_with_assumptions(graph, p_i, &ctx.assumptions)
                            .unwrap_or(p_i);
                        expr_sols.push(s);
                    }
                } else {
                    let param_syms: Vec<_> = if space.nullspace_basis.len() == 1 {
                        vec![graph.symbol("t")]
                    } else {
                        (1..=space.nullspace_basis.len())
                            .map(|k| graph.symbol(&format!("t_{}", k)))
                            .collect()
                    };
                    for i in 0..n {
                        let mut acc = space.particular[i];
                        for (k, basis_vec) in space.nullspace_basis.iter().enumerate() {
                            let coeff = basis_vec[i];
                            if !SymbolicMatrix::is_entry_zero(graph, coeff) {
                                let term = graph.mul([coeff, param_syms[k]]);
                                acc = graph.add([acc, term]);
                            }
                        }
                        let s = Simplifier::simplify_with_assumptions(graph, acc, &ctx.assumptions)
                            .unwrap_or(acc);
                        expr_sols.push(s);
                    }
                }

                let mut u_parts = Vec::with_capacity(n);
                let mut l_parts = Vec::with_capacity(n);
                for (name, &sol_id) in var_names.iter().zip(expr_sols.iter()) {
                    let u_val = unicode_fmt.format(graph, sol_id).unwrap_or_default();
                    let l_val = latex_fmt.format(graph, sol_id).unwrap_or_default();
                    u_parts.push(format!("{} = {}", name, u_val));
                    l_parts.push(format!("{} = {}", name, l_val));
                }
                let text = format!(
                    "Solutions for {}: [{{{}}}]",
                    vars_header,
                    u_parts.join(", ")
                );
                let latex = format!("\\left\\{{ {} \\right\\}}", l_parts.join(",\\; "));
                let mut res = OperationResult::success(text.clone(), latex, text);
                res.metadata.insert(
                    "solution_count".to_string(),
                    if space.has_infinite_solutions {
                        "infinite".to_string()
                    } else {
                        "1".to_string()
                    },
                );
                res.metadata
                    .insert("rank".to_string(), space.rank.to_string());
                res
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("Inconsistent") {
                    let out = format!("Solutions for {}: []", vars_header);
                    let latex = format!("\\left[ {} \\right] \\in \\emptyset", vars_latex);
                    let mut res = OperationResult::success(out.clone(), latex, out);
                    res.metadata
                        .insert("solution_count".to_string(), "0".to_string());
                    res
                } else {
                    OperationResult::error(msg)
                }
            }
        }
    } else {
        match PolynomialSystemSolver::solve(graph, &residuals, &var_names) {
            Ok(tuples) => {
                if tuples.is_empty() {
                    let out = format!("Solutions for {}: []", vars_header);
                    let latex = format!("\\left[ {} \\right] \\in \\emptyset", vars_latex);
                    let mut res = OperationResult::success(out.clone(), latex, out);
                    res.metadata
                        .insert("solution_count".to_string(), "0".to_string());
                    return res;
                }
                let mut u_tuples = Vec::with_capacity(tuples.len());
                let mut l_tuples = Vec::with_capacity(tuples.len());
                for tup in &tuples {
                    let assigns: Vec<String> = var_names
                        .iter()
                        .zip(tup.iter())
                        .map(|(v, r)| format!("{} = {}", v, format_system_complex_root(r)))
                        .collect();
                    let joined = assigns.join(", ");
                    u_tuples.push(format!("{{{}}}", joined));
                    l_tuples.push(format!("\\left( {} \\right)", assigns.join(",\\; ")));
                }
                let out = format!("Solutions for {}: [{}]", vars_header, u_tuples.join(", "));
                let latex = format!("\\left\\{{ {} \\right\\}}", l_tuples.join(",\\; "));
                let mut res = OperationResult::success(out.clone(), latex, out);
                res.metadata
                    .insert("solution_count".to_string(), tuples.len().to_string());
                res
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("Inconsistent") {
                    let out = format!("Solutions for {}: []", vars_header);
                    let latex = format!("\\left[ {} \\right] \\in \\emptyset", vars_latex);
                    let mut res = OperationResult::success(out.clone(), latex, out);
                    res.metadata
                        .insert("solution_count".to_string(), "0".to_string());
                    res
                } else {
                    OperationResult::error(msg)
                }
            }
        }
    }
}
