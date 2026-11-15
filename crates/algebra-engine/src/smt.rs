//! # `algebra_engine::smt`
//!
//! SMT-LIB2 Automated Theorem Prover (ATP) Bridge & Constructive Proof Certificates.
//!
//! Features:
//! - **SMT-LIB2 Standard Serializer**: Compatible with Z3, CVC5, Yices2, MathSAT across logics `QF_NRA`, `QF_LRA`, `QF_BV`, `QF_NIA`.
//! - **Typed SMT AST**: Relational, arithmetic, boolean, bitvector, and conditional expressions.
//! - **Formal Verification Proof Certificates**: Constructive proof script export for Lean 4 and Coq with automated tactics.

use serde::{Deserialize, Serialize};

/// SMT Standard Logics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SmtLogic {
    /// Quantifier-Free Non-linear Real Arithmetic.
    QfNra,
    /// Quantifier-Free Linear Real Arithmetic.
    QfLra,
    /// Quantifier-Free Fixed-Size Bit-Vectors.
    QfBv,
    /// Quantifier-Free Non-linear Integer Arithmetic.
    QfNia,
    /// Quantifier-Free Uninterpreted Functions.
    QfUf,
    /// All logics enabled.
    All,
}

impl SmtLogic {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::QfNra => "QF_NRA",
            Self::QfLra => "QF_LRA",
            Self::QfBv => "QF_BV",
            Self::QfNia => "QF_NIA",
            Self::QfUf => "QF_UF",
            Self::All => "ALL",
        }
    }
}

/// SMT Data Sorts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SmtSort {
    Real,
    Int,
    Bool,
    BitVec(usize),
}

impl SmtSort {
    pub fn to_smtlib2(&self) -> String {
        match self {
            Self::Real => "Real".to_string(),
            Self::Int => "Int".to_string(),
            Self::Bool => "Bool".to_string(),
            Self::BitVec(w) => format!("(_ BitVec {})", w),
        }
    }
}

/// SMT Expression Tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SmtExpr {
    Real(f64),
    Int(i64),
    Bool(bool),
    BitVec(u64, usize),
    Var(String),
    Add(Vec<SmtExpr>),
    Sub(Box<SmtExpr>, Box<SmtExpr>),
    Mul(Vec<SmtExpr>),
    Div(Box<SmtExpr>, Box<SmtExpr>),
    Neg(Box<SmtExpr>),
    Pow(Box<SmtExpr>, Box<SmtExpr>),
    Eq(Box<SmtExpr>, Box<SmtExpr>),
    Lt(Box<SmtExpr>, Box<SmtExpr>),
    Le(Box<SmtExpr>, Box<SmtExpr>),
    Gt(Box<SmtExpr>, Box<SmtExpr>),
    Ge(Box<SmtExpr>, Box<SmtExpr>),
    And(Vec<SmtExpr>),
    Or(Vec<SmtExpr>),
    Not(Box<SmtExpr>),
    Implies(Box<SmtExpr>, Box<SmtExpr>),
    Ite(Box<SmtExpr>, Box<SmtExpr>, Box<SmtExpr>),
    BvAnd(Box<SmtExpr>, Box<SmtExpr>),
    BvOr(Box<SmtExpr>, Box<SmtExpr>),
    BvXor(Box<SmtExpr>, Box<SmtExpr>),
    BvNot(Box<SmtExpr>),
    BvAdd(Box<SmtExpr>, Box<SmtExpr>),
    BvSub(Box<SmtExpr>, Box<SmtExpr>),
    BvMul(Box<SmtExpr>, Box<SmtExpr>),
}

impl SmtExpr {
    /// Format expression into compliant SMT-LIB2 s-expression syntax.
    pub fn to_smtlib2(&self) -> String {
        match self {
            Self::Real(v) => {
                if v.fract() == 0.0 {
                    format!("{:.1}", v)
                } else {
                    format!("{}", v)
                }
            }
            Self::Int(v) => format!("{}", v),
            Self::Bool(b) => {
                if *b {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            Self::BitVec(v, w) => format!("(_ bv{} {})", v, w),
            Self::Var(name) => name.clone(),
            Self::Add(args) => {
                let inner = args
                    .iter()
                    .map(|a| a.to_smtlib2())
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("(+ {})", inner)
            }
            Self::Sub(a, b) => format!("(- {} {})", a.to_smtlib2(), b.to_smtlib2()),
            Self::Mul(args) => {
                let inner = args
                    .iter()
                    .map(|a| a.to_smtlib2())
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("(* {})", inner)
            }
            Self::Div(a, b) => format!("(/ {} {})", a.to_smtlib2(), b.to_smtlib2()),
            Self::Neg(a) => format!("(- {})", a.to_smtlib2()),
            Self::Pow(a, b) => format!("(^ {} {})", a.to_smtlib2(), b.to_smtlib2()),
            Self::Eq(a, b) => format!("(= {} {})", a.to_smtlib2(), b.to_smtlib2()),
            Self::Lt(a, b) => format!("(< {} {})", a.to_smtlib2(), b.to_smtlib2()),
            Self::Le(a, b) => format!("(<= {} {})", a.to_smtlib2(), b.to_smtlib2()),
            Self::Gt(a, b) => format!("(> {} {})", a.to_smtlib2(), b.to_smtlib2()),
            Self::Ge(a, b) => format!("(>= {} {})", a.to_smtlib2(), b.to_smtlib2()),
            Self::And(args) => {
                let inner = args
                    .iter()
                    .map(|a| a.to_smtlib2())
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("(and {})", inner)
            }
            Self::Or(args) => {
                let inner = args
                    .iter()
                    .map(|a| a.to_smtlib2())
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("(or {})", inner)
            }
            Self::Not(a) => format!("(not {})", a.to_smtlib2()),
            Self::Implies(a, b) => format!("(=> {} {})", a.to_smtlib2(), b.to_smtlib2()),
            Self::Ite(c, t, e) => format!(
                "(ite {} {} {})",
                c.to_smtlib2(),
                t.to_smtlib2(),
                e.to_smtlib2()
            ),
            Self::BvAnd(a, b) => format!("(bvand {} {})", a.to_smtlib2(), b.to_smtlib2()),
            Self::BvOr(a, b) => format!("(bvor {} {})", a.to_smtlib2(), b.to_smtlib2()),
            Self::BvXor(a, b) => format!("(bvxor {} {})", a.to_smtlib2(), b.to_smtlib2()),
            Self::BvNot(a) => format!("(bvnot {})", a.to_smtlib2()),
            Self::BvAdd(a, b) => format!("(bvadd {} {})", a.to_smtlib2(), b.to_smtlib2()),
            Self::BvSub(a, b) => format!("(bvsub {} {})", a.to_smtlib2(), b.to_smtlib2()),
            Self::BvMul(a, b) => format!("(bvmul {} {})", a.to_smtlib2(), b.to_smtlib2()),
        }
    }

    /// Converts an expression node from `ExprGraph` directly into a typed `SmtExpr`.
    pub fn from_graph(
        graph: &algebra_core::ExprGraph,
        expr: algebra_core::ExprId,
    ) -> Result<Self, algebra_core::error::AlgebraError> {
        let node = graph.get(expr);
        match &node.kind {
            algebra_core::ExprKind::Number(num) => match num {
                algebra_core::Number::Integer(i) => Ok(Self::Int(*i)),
                algebra_core::Number::Rational(n, d) => Ok(Self::Div(
                    Box::new(Self::Real(*n as f64)),
                    Box::new(Self::Real(*d as f64)),
                )),
                algebra_core::Number::Float(f) => Ok(Self::Real(f64::from_bits(*f))),
                _ => Ok(Self::Real(0.0)),
            },
            algebra_core::ExprKind::Symbol(sym) => {
                let name = graph
                    .symbols
                    .resolve(*sym)
                    .unwrap_or_else(|| "x".to_string());
                Ok(Self::Var(name))
            }
            algebra_core::ExprKind::Add(terms) => {
                let mut smt_terms = Vec::with_capacity(terms.len());
                for &t in terms {
                    smt_terms.push(Self::from_graph(graph, t)?);
                }
                Ok(Self::Add(smt_terms))
            }
            algebra_core::ExprKind::Sub(l, r) => {
                let left = Self::from_graph(graph, *l)?;
                let right = Self::from_graph(graph, *r)?;
                Ok(Self::Sub(Box::new(left), Box::new(right)))
            }
            algebra_core::ExprKind::Mul(factors) => {
                let mut smt_factors = Vec::with_capacity(factors.len());
                for &f in factors {
                    smt_factors.push(Self::from_graph(graph, f)?);
                }
                Ok(Self::Mul(smt_factors))
            }
            algebra_core::ExprKind::Div(num, den) => {
                let left = Self::from_graph(graph, *num)?;
                let right = Self::from_graph(graph, *den)?;
                Ok(Self::Div(Box::new(left), Box::new(right)))
            }
            algebra_core::ExprKind::Neg(inner) => {
                let inside = Self::from_graph(graph, *inner)?;
                Ok(Self::Neg(Box::new(inside)))
            }
            algebra_core::ExprKind::Pow(base, exp) => {
                let b = Self::from_graph(graph, *base)?;
                let e = Self::from_graph(graph, *exp)?;
                Ok(Self::Pow(Box::new(b), Box::new(e)))
            }
            algebra_core::ExprKind::Relational { op, lhs, rhs } => {
                let left = Box::new(Self::from_graph(graph, *lhs)?);
                let right = Box::new(Self::from_graph(graph, *rhs)?);
                match op {
                    algebra_core::RelOp::Equal => Ok(Self::Eq(left, right)),
                    algebra_core::RelOp::LessThan => Ok(Self::Lt(left, right)),
                    algebra_core::RelOp::LessEqual => Ok(Self::Le(left, right)),
                    algebra_core::RelOp::GreaterThan => Ok(Self::Gt(left, right)),
                    algebra_core::RelOp::GreaterEqual => Ok(Self::Ge(left, right)),
                    _ => Err(algebra_core::error::AlgebraError::EvaluationError(format!(
                        "Unsupported SMT relational operator: {:?}",
                        op
                    ))),
                }
            }
            _ => Err(algebra_core::error::AlgebraError::EvaluationError(format!(
                "Unsupported ExprKind for SMT translation: {:?}",
                node.kind
            ))),
        }
    }
}

/// SMT-LIB2 Problem Container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtProblem {
    pub logic: Option<SmtLogic>,
    pub declarations: Vec<(String, SmtSort)>,
    pub assertions: Vec<SmtExpr>,
}

impl Default for SmtProblem {
    fn default() -> Self {
        Self::new()
    }
}

impl SmtProblem {
    pub fn new() -> Self {
        Self {
            logic: None,
            declarations: Vec::new(),
            assertions: Vec::new(),
        }
    }

    pub fn with_logic(logic: SmtLogic) -> Self {
        Self {
            logic: Some(logic),
            declarations: Vec::new(),
            assertions: Vec::new(),
        }
    }

    pub fn add_var(&mut self, name: &str, sort: SmtSort) -> &mut Self {
        self.declarations.push((name.to_string(), sort));
        self
    }

    pub fn assert(&mut self, expr: SmtExpr) -> &mut Self {
        self.assertions.push(expr);
        self
    }

    /// Serialize into standard compliant SMT-LIB2 script format for Z3 / CVC5.
    pub fn to_smtlib2_string(&self) -> String {
        let mut lines = Vec::new();
        lines.push("; Generated by URAE Automated Theorem Prover Bridge".to_string());
        lines.push("(set-option :produce-models true)".to_string());

        if let Some(logic) = self.logic {
            lines.push(format!("(set-logic {})", logic.as_str()));
        }

        for (name, sort) in &self.declarations {
            lines.push(format!("(declare-const {} {})", name, sort.to_smtlib2()));
        }

        for expr in &self.assertions {
            lines.push(format!("(assert {})", expr.to_smtlib2()));
        }

        lines.push("(check-sat)".to_string());
        lines.push("(get-model)".to_string());

        lines.join("\n")
    }
}

/// Constructive Proof Certificate for Interactive Theorem Provers (Lean 4 & Coq).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofCertificate {
    pub theorem_name: String,
    pub hypotheses: Vec<(String, String)>,
    pub conclusion: String,
    pub tactic: String,
}

impl ProofCertificate {
    pub fn new(
        theorem_name: impl Into<String>,
        hypotheses: Vec<(String, String)>,
        conclusion: impl Into<String>,
        tactic: impl Into<String>,
    ) -> Self {
        Self {
            theorem_name: theorem_name.into(),
            hypotheses,
            conclusion: conclusion.into(),
            tactic: tactic.into(),
        }
    }

    /// Export theorem to Lean 4 code.
    pub fn to_lean4_theorem(&self) -> String {
        let mut hyps = Vec::new();
        for (var, typ) in &self.hypotheses {
            hyps.push(format!("({} : {})", var, typ));
        }
        let hyps_str = if hyps.is_empty() {
            "".to_string()
        } else {
            format!(" {}", hyps.join(" "))
        };

        format!(
            "theorem {}{} : {} := by\n  {}",
            self.theorem_name, hyps_str, self.conclusion, self.tactic
        )
    }

    /// Export lemma to Coq code.
    pub fn to_coq_lemma(&self) -> String {
        let mut hyps = Vec::new();
        for (var, typ) in &self.hypotheses {
            hyps.push(format!("({} : {})", var, typ));
        }
        let hyps_str = if hyps.is_empty() {
            "".to_string()
        } else {
            format!(" {}", hyps.join(" "))
        };

        format!(
            "Lemma {}{} : {}.\nProof.\n  {}.\nQed.",
            self.theorem_name, hyps_str, self.conclusion, self.tactic
        )
    }
}

/// Internal SMT/interval satisfiability check result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmtCheckStatus {
    Sat,
    Unsat,
    Unknown,
}

impl SmtExpr {
    fn eval_interval(
        &self,
        env: &std::collections::HashMap<String, (f64, f64)>,
    ) -> Option<(f64, f64)> {
        match self {
            SmtExpr::Int(i) => Some((*i as f64, *i as f64)),
            SmtExpr::Real(r) => Some((*r, *r)),
            SmtExpr::Var(name) => env.get(name).copied(),
            SmtExpr::Add(args) => {
                let mut acc = (0.0, 0.0);
                for a in args {
                    let iv = a.eval_interval(env)?;
                    acc = (acc.0 + iv.0, acc.1 + iv.1);
                }
                Some(acc)
            }
            SmtExpr::Sub(a, b) => {
                let ia = a.eval_interval(env)?;
                let ib = b.eval_interval(env)?;
                Some((ia.0 - ib.1, ia.1 - ib.0))
            }
            SmtExpr::Neg(a) => {
                let ia = a.eval_interval(env)?;
                Some((-ia.1, -ia.0))
            }
            SmtExpr::Mul(args) => {
                if args.len() == 2 && args[0].to_smtlib2() == args[1].to_smtlib2() {
                    let iv = args[0].eval_interval(env)?;
                    let sq = Self::interval_even_pow(iv, 2);
                    return Some(sq);
                }
                let mut acc = (1.0, 1.0);
                for a in args {
                    let iv = a.eval_interval(env)?;
                    let p = [acc.0 * iv.0, acc.0 * iv.1, acc.1 * iv.0, acc.1 * iv.1];
                    acc = (
                        p.iter().copied().fold(f64::INFINITY, f64::min),
                        p.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                    );
                }
                Some(acc)
            }
            SmtExpr::Pow(base, exp) => {
                let ib = base.eval_interval(env)?;
                if let SmtExpr::Int(k) = exp.as_ref() {
                    if *k >= 0 && *k <= 16 {
                        if *k % 2 == 0 {
                            return Some(Self::interval_even_pow(ib, *k as i32));
                        } else {
                            return Some((ib.0.powi(*k as i32), ib.1.powi(*k as i32)));
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn interval_even_pow(iv: (f64, f64), k: i32) -> (f64, f64) {
        if k == 0 {
            return (1.0, 1.0);
        }
        let a = iv.0.powi(k);
        let b = iv.1.powi(k);
        if iv.0 <= 0.0 && iv.1 >= 0.0 {
            (0.0, a.max(b))
        } else {
            (a.min(b), a.max(b))
        }
    }

    fn check_assertion_interval(
        &self,
        env: &std::collections::HashMap<String, (f64, f64)>,
    ) -> SmtCheckStatus {
        match self {
            SmtExpr::Bool(true) => SmtCheckStatus::Sat,
            SmtExpr::Bool(false) => SmtCheckStatus::Unsat,
            SmtExpr::And(parts) => {
                let mut all_sat = true;
                for p in parts {
                    match p.check_assertion_interval(env) {
                        SmtCheckStatus::Unsat => return SmtCheckStatus::Unsat,
                        SmtCheckStatus::Unknown => all_sat = false,
                        SmtCheckStatus::Sat => {}
                    }
                }
                if all_sat {
                    SmtCheckStatus::Sat
                } else {
                    SmtCheckStatus::Unknown
                }
            }
            SmtExpr::Or(parts) => {
                let mut any_unknown = false;
                for p in parts {
                    match p.check_assertion_interval(env) {
                        SmtCheckStatus::Sat => return SmtCheckStatus::Sat,
                        SmtCheckStatus::Unknown => any_unknown = true,
                        SmtCheckStatus::Unsat => {}
                    }
                }
                if any_unknown {
                    SmtCheckStatus::Unknown
                } else {
                    SmtCheckStatus::Unsat
                }
            }
            SmtExpr::Lt(a, b) => match (a.eval_interval(env), b.eval_interval(env)) {
                (Some(ia), Some(ib)) => {
                    if ia.1 < ib.0 {
                        SmtCheckStatus::Sat
                    } else if ia.0 >= ib.1 {
                        SmtCheckStatus::Unsat
                    } else {
                        SmtCheckStatus::Unknown
                    }
                }
                _ => SmtCheckStatus::Unknown,
            },
            SmtExpr::Le(a, b) => match (a.eval_interval(env), b.eval_interval(env)) {
                (Some(ia), Some(ib)) => {
                    if ia.1 <= ib.0 {
                        SmtCheckStatus::Sat
                    } else if ia.0 > ib.1 {
                        SmtCheckStatus::Unsat
                    } else {
                        SmtCheckStatus::Unknown
                    }
                }
                _ => SmtCheckStatus::Unknown,
            },
            SmtExpr::Gt(a, b) => SmtExpr::Lt(b.clone(), a.clone()).check_assertion_interval(env),
            SmtExpr::Ge(a, b) => SmtExpr::Le(b.clone(), a.clone()).check_assertion_interval(env),
            SmtExpr::Eq(a, b) => match (a.eval_interval(env), b.eval_interval(env)) {
                (Some(ia), Some(ib)) => {
                    if ia.1 < ib.0 || ib.1 < ia.0 {
                        SmtCheckStatus::Unsat
                    } else if ia.0 == ia.1 && ib.0 == ib.1 && (ia.0 - ib.0).abs() < 1e-12 {
                        SmtCheckStatus::Sat
                    } else {
                        SmtCheckStatus::Unknown
                    }
                }
                _ => SmtCheckStatus::Unknown,
            },
            _ => SmtCheckStatus::Unknown,
        }
    }
}

impl SmtProblem {
    /// Run internal interval-arithmetic satisfiability & contradiction guard
    /// over bounded variable intervals `(name, min, max)` without external solvers.
    pub fn check_interval_satisfiability(&self, bounds: &[(&str, f64, f64)]) -> SmtCheckStatus {
        let mut env = std::collections::HashMap::new();
        for &(name, lo, hi) in bounds {
            env.insert(name.to_string(), (lo, hi));
        }
        let mut all_sat = true;
        for assertion in &self.assertions {
            match assertion.check_assertion_interval(&env) {
                SmtCheckStatus::Unsat => return SmtCheckStatus::Unsat,
                SmtCheckStatus::Unknown => all_sat = false,
                SmtCheckStatus::Sat => {}
            }
        }
        if all_sat {
            SmtCheckStatus::Sat
        } else {
            SmtCheckStatus::Unknown
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smt_internal_interval_guard() {
        let mut prob = SmtProblem::with_logic(SmtLogic::QfNra);
        prob.add_var("x", SmtSort::Real);
        // Assert x^2 + 1 < 0, which is impossible for x in [-10, 10]
        let x = SmtExpr::Var("x".to_string());
        let x_sq = SmtExpr::Mul(vec![x.clone(), x]);
        let lhs = SmtExpr::Add(vec![x_sq, SmtExpr::Int(1)]);
        prob.assert(SmtExpr::Lt(Box::new(lhs), Box::new(SmtExpr::Int(0))));

        assert_eq!(
            prob.check_interval_satisfiability(&[("x", -10.0, 10.0)]),
            SmtCheckStatus::Unsat
        );
    }
}
