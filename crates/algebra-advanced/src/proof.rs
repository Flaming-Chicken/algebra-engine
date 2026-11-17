//! # `algebra-proof`
//!
//! Formal proof verification, Martelli-Montanari term unification, Higher-Order Logic (HOL)
//! term rewriting, and proof trace export to **Lean 4** and **Coq**.

use algebra_core::{AlgebraError, AlgebraResult, ExprGraph, ExprId, SymbolId};
use std::collections::HashMap;

/// Martelli-Montanari unification substitution mapping variable symbols to expressions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SubstitutionMap {
    pub mappings: HashMap<SymbolId, ExprId>,
}

impl SubstitutionMap {
    pub fn new() -> Self {
        Self {
            mappings: HashMap::new(),
        }
    }

    pub fn insert(&mut self, var: SymbolId, expr: ExprId) {
        self.mappings.insert(var, expr);
    }

    /// Apply the substitution to `expr`. For a substitution produced by
    /// [`UnificationEngine::unify`] this is idempotent.
    pub fn apply(&self, graph: &ExprGraph, expr: ExprId) -> ExprId {
        self.mappings.iter().fold(expr, |acc, (&var, &value)| {
            graph.substitute(acc, var, value)
        })
    }
}

/// Martelli-Montanari unification algorithm engine.
#[derive(Debug, Clone, Default)]
pub struct UnificationEngine;

impl UnificationEngine {
    pub fn new() -> Self {
        Self
    }

    /// Unify two expressions, treating every symbol as a variable (first-order, syntactic).
    ///
    /// This is the Martelli-Montanari algorithm: equations are decomposed structurally (same
    /// head and arity, arguments in order), a variable is bound to the other side unless it
    /// occurs there (occurs check), and each binding is propagated into the bindings already
    /// made. The result is the most general unifier: applying it to both inputs gives the same
    /// expression. Addition and multiplication are matched argument by argument, in order; no
    /// commutativity or associativity is assumed.
    pub fn unify(
        &self,
        graph: &ExprGraph,
        expr1: ExprId,
        expr2: ExprId,
    ) -> AlgebraResult<SubstitutionMap> {
        use algebra_core::ExprKind;
        let failure = |reason: String| AlgebraError::DomainViolation {
            domain: "Unification".to_string(),
            reason,
        };
        let mut subst = SubstitutionMap::new();
        let mut pending = vec![(expr1, expr2)];

        while let Some((left, right)) = pending.pop() {
            // Work on the current instances of both sides.
            let (left, right) = (subst.apply(graph, left), subst.apply(graph, right));
            if left == right {
                continue;
            }
            let (kind_left, kind_right) =
                (graph.get(left).kind.clone(), graph.get(right).kind.clone());
            let bind = |var: SymbolId, value: ExprId, subst: &mut SubstitutionMap| {
                if graph.has_symbol(value, var) {
                    return Err(failure(format!(
                        "occurs check: `{}` appears inside the term it would be bound to",
                        graph.symbols.resolve(var).unwrap_or_default()
                    )));
                }
                // Propagate the new binding into the existing ones, then record it.
                for existing in subst.mappings.values_mut() {
                    *existing = graph.substitute(*existing, var, value);
                }
                subst.insert(var, value);
                Ok(())
            };
            match (&kind_left, &kind_right) {
                (ExprKind::Symbol(v), _) => bind(*v, right, &mut subst)?,
                (_, ExprKind::Symbol(v)) => bind(*v, left, &mut subst)?,
                (ExprKind::Number(a), ExprKind::Number(b)) => {
                    return Err(failure(format!("numbers {a:?} and {b:?} do not match")));
                }
                (ExprKind::Add(a), ExprKind::Add(b)) | (ExprKind::Mul(a), ExprKind::Mul(b)) => {
                    if a.len() != b.len() {
                        return Err(failure("operand counts differ".to_string()));
                    }
                    pending.extend(a.iter().copied().zip(b.iter().copied()));
                }
                (ExprKind::Sub(a1, a2), ExprKind::Sub(b1, b2))
                | (ExprKind::Div(a1, a2), ExprKind::Div(b1, b2))
                | (ExprKind::Pow(a1, a2), ExprKind::Pow(b1, b2)) => {
                    pending.push((*a1, *b1));
                    pending.push((*a2, *b2));
                }
                (ExprKind::Neg(a), ExprKind::Neg(b)) => pending.push((*a, *b)),
                (
                    ExprKind::Function { name: n1, args: a },
                    ExprKind::Function { name: n2, args: b },
                ) => {
                    if n1 != n2 || a.len() != b.len() {
                        return Err(failure(format!(
                            "function heads differ: `{}`/{} vs `{}`/{}",
                            graph.symbols.resolve(*n1).unwrap_or_default(),
                            a.len(),
                            graph.symbols.resolve(*n2).unwrap_or_default(),
                            b.len()
                        )));
                    }
                    pending.extend(a.iter().copied().zip(b.iter().copied()));
                }
                _ => {
                    return Err(failure(
                        "structurally incompatible expressions (different operators)".to_string(),
                    ));
                }
            }
        }
        Ok(subst)
    }
}

/// Proof Step trace entry recording algebraic transformations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofStep {
    pub rule_name: String,
    pub before: ExprId,
    pub after: ExprId,
}

/// Lean 4 and Coq Proof Trace Exporter.
#[derive(Debug, Clone, Default)]
pub struct ProofTrace {
    pub steps: Vec<ProofStep>,
}

/// Target proof assistant for statement rendering.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Prover {
    Lean,
    Coq,
}

/// Render `id` as a real-valued term for `prover`, or `None` when it uses something the exporter
/// does not translate (an unknown function, a non-rational constant, an integral, ...).
fn render_term(
    graph: &ExprGraph,
    id: ExprId,
    prover: Prover,
    vars: &mut Vec<String>,
) -> Option<String> {
    use algebra_core::{ExprKind, Number};
    let node = graph.get(id);
    match &node.kind {
        ExprKind::Number(Number::Integer(i)) => Some(if *i < 0 {
            format!("({i})")
        } else {
            i.to_string()
        }),
        ExprKind::Number(Number::Rational(n, d)) => Some(format!("(({n}:R) / ({d}:R))")).map(|s| {
            if prover == Prover::Lean {
                format!("(({n}:ℝ) / ({d}:ℝ))")
            } else {
                s
            }
        }),
        ExprKind::Symbol(sym) => {
            let name = graph.symbols.resolve(*sym)?;
            if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return None;
            }
            if !vars.contains(&name) {
                vars.push(name.clone());
            }
            Some(name)
        }
        ExprKind::Add(terms) => {
            let parts: Option<Vec<String>> = terms
                .iter()
                .map(|&t| render_term(graph, t, prover, vars))
                .collect();
            Some(format!("({})", parts?.join(" + ")))
        }
        ExprKind::Mul(factors) => {
            let parts: Option<Vec<String>> = factors
                .iter()
                .map(|&f| render_term(graph, f, prover, vars))
                .collect();
            Some(format!("({})", parts?.join(" * ")))
        }
        ExprKind::Sub(l, r) => Some(format!(
            "({} - {})",
            render_term(graph, *l, prover, vars)?,
            render_term(graph, *r, prover, vars)?
        )),
        ExprKind::Div(l, r) => Some(format!(
            "({} / {})",
            render_term(graph, *l, prover, vars)?,
            render_term(graph, *r, prover, vars)?
        )),
        ExprKind::Neg(inner) => Some(format!("(-{})", render_term(graph, *inner, prover, vars)?)),
        ExprKind::Pow(base, exponent) => {
            // Only natural-number exponents are translated; they are valid for `ring`.
            let ExprKind::Number(Number::Integer(e)) = graph.get(*exponent).kind else {
                return None;
            };
            if e < 0 {
                return None;
            }
            Some(format!(
                "({} ^ {e})",
                render_term(graph, *base, prover, vars)?
            ))
        }
        _ => None,
    }
}

impl ProofTrace {
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    pub fn add_step(&mut self, rule_name: impl Into<String>, before: ExprId, after: ExprId) {
        self.steps.push(ProofStep {
            rule_name: rule_name.into(),
            before,
            after,
        });
    }

    fn export_with(&self, graph: &ExprGraph, prover: Prover) -> String {
        let mut out = match prover {
            Prover::Lean => String::from(
                "-- URAE proof trace for Lean 4 (Mathlib). Not machine-checked by URAE.\n-- A step is closed with `ring` when both sides are polynomial; other steps are left\n-- as `sorry` and must be proved by hand.\nimport Mathlib\n\n",
            ),
            Prover::Coq => String::from(
                "(* URAE proof trace for Coq. Not machine-checked by URAE.\n   A step is closed with `ring` when both sides are polynomial; other steps are left\n   as `Admitted` and must be proved by hand. *)\nRequire Import Reals.\nRequire Import Lra.\nRequire Import Ring.\nOpen Scope R_scope.\n\n",
            ),
        };
        for (index, step) in self.steps.iter().enumerate() {
            let number = index + 1;
            let mut vars = Vec::new();
            let before = render_term(graph, step.before, prover, &mut vars);
            let after = render_term(graph, step.after, prover, &mut vars);
            let (Some(before), Some(after)) = (before, after) else {
                out.push_str(&match prover {
                    Prover::Lean => format!(
                        "-- step {number} ({}): not translated (unsupported expression)\n\n",
                        step.rule_name
                    ),
                    Prover::Coq => format!(
                        "(* step {number} ({}): not translated (unsupported expression) *)\n\n",
                        step.rule_name
                    ),
                });
                continue;
            };
            match prover {
                Prover::Lean => {
                    let binders = if vars.is_empty() {
                        String::new()
                    } else {
                        format!(" ({} : ℝ)", vars.join(" "))
                    };
                    out.push_str(&format!(
                        "-- rule: {}\ntheorem step_{number}{binders} : ({before} : ℝ) = {after} := by\n  ring\n\n",
                        step.rule_name
                    ));
                }
                Prover::Coq => {
                    let binders = vars
                        .iter()
                        .map(|v| format!("forall {v} : R, "))
                        .collect::<String>();
                    out.push_str(&format!(
                        "(* rule: {} *)\nLemma step_{number} : {binders}{before} = {after}.\nProof. intros; ring. Qed.\n\n",
                        step.rule_name
                    ));
                }
            }
        }
        out
    }

    /// Export the trace to Lean 4 with each step as a theorem about the real expressions it
    /// rewrote. Both sides are translated only for polynomial arithmetic (`+ - * /`, natural
    /// powers, rational literals); a step outside that fragment is listed as a comment. The
    /// proofs are `ring` and are not checked by URAE; a step that is not a ring identity
    /// (for example one that divides by an expression) will fail in Lean.
    pub fn export_lean4_with(&self, graph: &ExprGraph) -> String {
        self.export_with(graph, Prover::Lean)
    }

    /// Export the trace to Coq; see [`ProofTrace::export_lean4_with`].
    pub fn export_coq_with(&self, graph: &ExprGraph) -> String {
        self.export_with(graph, Prover::Coq)
    }

    /// Skeleton export: rule names with `sorry`, no expressions. Prefer [`ProofTrace::export_lean4_with`].
    pub fn export_lean4(&self) -> String {
        let mut out = String::from("-- Generated Lean 4 Proof Trace\nimport Mathlib\n\n");
        out.push_str("theorem algebraic_simplification : True := by\n");
        for step in &self.steps {
            out.push_str(&format!("  -- Apply rule: {}\n  sorry\n", step.rule_name));
        }
        out.push_str("  trivial\n");
        out
    }

    /// Skeleton export: rule names only, no expressions. Prefer [`ProofTrace::export_coq_with`].
    pub fn export_coq(&self) -> String {
        let mut out = String::from(
            "(* Generated Coq Proof Trace *)\nRequire Import Utf8.\nRequire Import Arith.\n\n",
        );
        out.push_str("Theorem algebraic_simplification : True.\nProof.\n");
        for step in &self.steps {
            out.push_str(&format!("  (* Rule: {} *)\n", step.rule_name));
        }
        out.push_str("  trivial.\nQed.\n");
        out
    }

    /// Internally verify every step `(before, after)` in the trace for algebraic equivalence
    /// using deterministic/Schwartz-Zippel verification before exporting Lean 4 / Coq certificates.
    pub fn verify_steps(&self, graph: &ExprGraph) -> Result<usize, String> {
        for (idx, step) in self.steps.iter().enumerate() {
            if step.before == step.after {
                continue;
            }
            let verif = algebra_core::probabilistic::ProbabilisticVerifier::verify_equivalence(
                graph,
                step.before,
                step.after,
                1e-12,
            );
            if !*verif.as_ref() {
                return Err(format!(
                    "Proof step {} ('{}') failed internal equivalence verification",
                    idx, step.rule_name
                ));
            }
        }
        Ok(self.steps.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use algebra_core::ExprParser;

    #[test]
    fn test_proof_trace_internal_step_verification() {
        let graph = ExprGraph::new();
        let parser = ExprParser::new(&graph);
        let a = parser.parse("(x + 1)^2").expect("parse a");
        let b = parser.parse("x^2 + 2*x + 1").expect("parse b");
        let bad = parser.parse("x^2 + 3*x + 1").expect("parse bad");

        let mut ok_trace = ProofTrace::new();
        ok_trace.add_step("expand_square", a, b);
        assert_eq!(ok_trace.verify_steps(&graph), Ok(1));

        let mut bad_trace = ProofTrace::new();
        bad_trace.add_step("bogus_rule", a, bad);
        assert!(bad_trace.verify_steps(&graph).is_err());
    }
}
