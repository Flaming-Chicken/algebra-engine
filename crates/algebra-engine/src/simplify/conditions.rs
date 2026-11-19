//! Side conditions of a simplification: the points the original expression excludes that the result no longer does.
//!
//! `x / x` simplifies to `1`, but the original is undefined at `x = 0`; `exp(ln(x))` simplifies to `x`, but the
//! original needs `x > 0`. A result that drops such a restriction is correct only away from the excluded points, so
//! [`Simplifier::simplify_with_conditions`] reports them.
//!
//! A condition is an *atom*: `e != 0`, `e >= 0` or `e > 0`. The conditions of an expression are the denominators
//! that must not vanish, the arguments of `ln`, `log` and `sqrt`, the bases of negative or fractional powers and the
//! denominators hidden in `tan` and `cot`. They are split into atoms (`x*z != 0` becomes `x != 0` and `z != 0`) so
//! that a product that loses one factor still reports the factor it lost. The excluded points are the original's
//! atoms that no atom of the result implies.

use super::{Simplifier, SimplifyResult};
use algebra_core::format::{Formatter, UnicodeFormatter};
use algebra_core::{ExprGraph, ExprId, ExprKind};

/// What an atom requires of its expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ConditionKind {
    /// `expr != 0`
    NonZero,
    /// `expr >= 0`
    NonNegative,
    /// `expr > 0`
    Positive,
}

/// One side condition: `expr` is non-zero, non-negative or positive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Condition {
    pub kind: ConditionKind,
    pub expr: ExprId,
}

impl Condition {
    /// The condition as text, for example `x ≠ 0`.
    pub fn describe(&self, graph: &ExprGraph) -> String {
        let text = UnicodeFormatter
            .format(graph, self.expr)
            .unwrap_or_else(|_| "?".to_string());
        match self.kind {
            ConditionKind::NonZero => format!("{text} ≠ 0"),
            ConditionKind::NonNegative => format!("{text} ≥ 0"),
            ConditionKind::Positive => format!("{text} > 0"),
        }
    }

    /// Whether `self` guarantees `other` (positive implies non-negative and non-zero).
    fn implies(&self, other: &Condition) -> bool {
        self.expr == other.expr
            && (self.kind == other.kind
                || (self.kind == ConditionKind::Positive
                    && matches!(
                        other.kind,
                        ConditionKind::NonNegative | ConditionKind::NonZero
                    )))
    }
}

/// A simplified expression together with the points the simplification lost.
#[derive(Debug, Clone)]
pub struct ConditionedResult {
    pub expr: ExprId,
    /// Restrictions of the original that the result no longer carries: the identity holds where all of them hold.
    pub excluded: Vec<Condition>,
}

impl ConditionedResult {
    /// The excluded points as text.
    pub fn describe_excluded(&self, graph: &ExprGraph) -> Vec<String> {
        self.excluded.iter().map(|c| c.describe(graph)).collect()
    }
}

fn number_sign(graph: &ExprGraph, id: ExprId) -> Option<f64> {
    match &graph.get(id).kind {
        ExprKind::Number(n) => n.as_f64(),
        _ => None,
    }
}

/// Atoms for `expr != 0`.
fn push_nonzero(graph: &ExprGraph, id: ExprId, out: &mut Vec<Condition>) {
    match &graph.get(id).kind {
        ExprKind::Number(_) => {} // a non-zero constant is no condition (and zero is not our business here)
        ExprKind::Mul(factors) => {
            for &f in factors {
                push_nonzero(graph, f, out);
            }
        }
        ExprKind::Neg(inner) => push_nonzero(graph, *inner, out),
        ExprKind::Div(a, b) => {
            push_nonzero(graph, *a, out);
            push_nonzero(graph, *b, out);
        }
        ExprKind::Pow(base, exp) => {
            // b^k for a number k is non-zero exactly when b is (k > 0 or k < 0 alike, away from 0^0).
            if number_sign(graph, *exp).is_some() {
                push_nonzero(graph, *base, out);
            } else {
                out.push(Condition {
                    kind: ConditionKind::NonZero,
                    expr: id,
                });
            }
        }
        _ => out.push(Condition {
            kind: ConditionKind::NonZero,
            expr: id,
        }),
    }
}

/// The conditions an expression needs to be defined (over the reals).
fn collect(graph: &ExprGraph, id: ExprId, out: &mut Vec<Condition>) {
    let node = graph.get(id);
    match &node.kind {
        ExprKind::Div(num, den) => {
            push_nonzero(graph, *den, out);
            collect(graph, *num, out);
            collect(graph, *den, out);
        }
        ExprKind::Pow(base, exp) => {
            match number_sign(graph, *exp) {
                Some(e) if e < 0.0 => push_nonzero(graph, *base, out),
                Some(e) if e.fract() != 0.0 => out.push(Condition {
                    kind: ConditionKind::NonNegative,
                    expr: *base,
                }),
                _ => {}
            }
            collect(graph, *base, out);
            collect(graph, *exp, out);
        }
        ExprKind::Function { name, args } => {
            let name = graph.symbols.resolve(*name).unwrap_or_default();
            if let Some(&arg) = args.first() {
                match name.as_str() {
                    "ln" | "log" | "log2" | "log10" => out.push(Condition {
                        kind: ConditionKind::Positive,
                        expr: arg,
                    }),
                    "sqrt" => out.push(Condition {
                        kind: ConditionKind::NonNegative,
                        expr: arg,
                    }),
                    "tan" | "sec" => {
                        let cos = graph.function("cos", [arg]);
                        out.push(Condition {
                            kind: ConditionKind::NonZero,
                            expr: cos,
                        });
                    }
                    "cot" | "csc" => {
                        let sin = graph.function("sin", [arg]);
                        out.push(Condition {
                            kind: ConditionKind::NonZero,
                            expr: sin,
                        });
                    }
                    _ => {}
                }
            }
            for &a in args {
                collect(graph, a, out);
            }
        }
        _ => {
            for child in node.children() {
                collect(graph, child, out);
            }
        }
    }
}

/// The distinct atoms of an expression's conditions, each expression simplified so equal conditions compare equal.
fn atoms(graph: &ExprGraph, id: ExprId) -> Vec<Condition> {
    let mut raw = Vec::new();
    collect(graph, id, &mut raw);
    let mut out: Vec<Condition> = Vec::new();
    for c in raw {
        let expr = Simplifier::simplify(graph, c.expr).unwrap_or(c.expr);
        // Positive and non-negative conditions on a positive constant (`ln(2)`) are vacuous.
        if let Some(v) = number_sign(graph, expr) {
            let holds = match c.kind {
                ConditionKind::NonZero => v != 0.0,
                ConditionKind::NonNegative => v >= 0.0,
                ConditionKind::Positive => v > 0.0,
            };
            if holds {
                continue;
            }
        }
        let c = Condition { expr, kind: c.kind };
        if !out.contains(&c) {
            out.push(c);
        }
    }
    out
}

impl Simplifier {
    /// Simplify, and report which restrictions of the original expression the result has lost.
    ///
    /// `x / x` gives `1` with the excluded point `x ≠ 0`; `(x² − 1) / (x − 1)` gives `x + 1` with `x − 1 ≠ 0`;
    /// `exp(ln x)` gives `x` with `x > 0`. A restriction the result still carries is not reported.
    pub fn simplify_with_conditions(
        graph: &ExprGraph,
        root: ExprId,
    ) -> SimplifyResult<ConditionedResult> {
        let expr = Self::simplify(graph, root)?;
        let before = atoms(graph, root);
        let after = atoms(graph, expr);
        let excluded = before
            .into_iter()
            .filter(|b| !after.iter().any(|a| a.implies(b)))
            .collect();
        Ok(ConditionedResult { expr, excluded })
    }
}
