//! Detection of undefined constant arithmetic: division by zero (`1/0`, `0/0`), `oo - oo`, `0 * oo`, `0^(-1)`.
//!
//! The rewrite rules `x / x = 1` and `0 * x = 0` hold only where `x` is a finite non-zero number, so these inputs
//! must be recognised before the simplifier sees them.

use algebra_core::{Constant, ExprGraph, ExprId, ExprKind, Number};

fn is_zero(graph: &ExprGraph, id: ExprId) -> bool {
    match &graph.get(id).kind {
        ExprKind::Number(Number::Integer(0)) => true,
        ExprKind::Number(Number::Rational(0, _)) => true,
        ExprKind::Neg(a) => is_zero(graph, *a),
        _ => false,
    }
}

/// A denominator that is zero, also after simplification (`x - x`, `3 - 3`).
fn vanishes(graph: &ExprGraph, id: ExprId) -> bool {
    is_zero(graph, id)
        || (matches!(graph.get(id).kind, ExprKind::Add(_) | ExprKind::Sub(..))
            && crate::simplify::Simplifier::simplify(graph, id).is_ok_and(|s| is_zero(graph, s)))
}

/// `Some(true)` for `+oo`, `Some(false)` for `-oo`, `None` if not an infinity.
fn infinity_sign(graph: &ExprGraph, id: ExprId) -> Option<bool> {
    match &graph.get(id).kind {
        ExprKind::Number(Number::Constant(Constant::Infinity)) => Some(true),
        ExprKind::Number(Number::Constant(Constant::NegInfinity)) => Some(false),
        ExprKind::Symbol(s)
            if matches!(
                graph.symbols.resolve(*s).as_deref(),
                Some("oo" | "inf" | "infinity" | "∞")
            ) =>
        {
            Some(true)
        }
        ExprKind::Neg(a) => infinity_sign(graph, *a).map(|s| !s),
        _ => None,
    }
}

/// Whether `expr` contains a term whose value is undefined (an indeterminate form or a division by zero).
pub fn is_undefined(graph: &ExprGraph, expr: ExprId) -> bool {
    match graph.get(expr).kind.clone() {
        ExprKind::Div(a, b) => {
            vanishes(graph, b) || is_undefined(graph, a) || is_undefined(graph, b)
        }
        ExprKind::Pow(a, b) => {
            let negative_exponent = match &graph.get(b).kind {
                ExprKind::Number(Number::Integer(k)) => *k < 0,
                ExprKind::Number(Number::Rational(n, _)) => *n < 0,
                _ => false,
            };
            (is_zero(graph, a) && negative_exponent)
                || is_undefined(graph, a)
                || is_undefined(graph, b)
        }
        ExprKind::Mul(fs) => {
            let zero = fs.iter().any(|&f| is_zero(graph, f));
            let infinite = fs.iter().any(|&f| infinity_sign(graph, f).is_some());
            (zero && infinite) || fs.iter().any(|&f| is_undefined(graph, f))
        }
        ExprKind::Add(ts) => {
            let signs: Vec<bool> = ts.iter().filter_map(|&t| infinity_sign(graph, t)).collect();
            (signs.contains(&true) && signs.contains(&false))
                || ts.iter().any(|&t| is_undefined(graph, t))
        }
        ExprKind::Sub(a, b) => {
            matches!((infinity_sign(graph, a), infinity_sign(graph, b)), (Some(x), Some(y)) if x == y)
                || is_undefined(graph, a)
                || is_undefined(graph, b)
        }
        ExprKind::Neg(a) => is_undefined(graph, a),
        ExprKind::Function { args, .. } => args.iter().any(|&a| is_undefined(graph, a)),
        _ => false,
    }
}
