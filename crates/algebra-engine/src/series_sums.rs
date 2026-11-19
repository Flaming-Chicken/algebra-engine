//! Closed forms of some infinite sums `sum_{k = a}^{oo} f(k)`:
//!
//! - rational functions with simple integer poles (telescoping through partial fractions and harmonic numbers):
//!   `1/(k (k + 1)) = 1`, `1/(k (k + 2)) = 3/4`, `1/(k^2 - 1)` from 2 is `3/4`;
//! - `x^k / k!` (`e^x`), `x^k / k` (`-ln(1 - x)`);
//! - a few classical series recognised by their exact first terms: the alternating harmonic series, the Leibniz series,
//!   `sum 1/(2k - 1)^2 = pi^2/8`, `sum (-1)^(k + 1)/k^2 = pi^2/12`.

use crate::numeric::try_eval_exact;
use crate::poly::{UniPoly, expr_from_rational};
use algebra_core::{Constant, ExprGraph, ExprId, ExprKind, SymbolId};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};
use std::collections::HashMap;

type Q = BigRational;

fn harmonic(n: i64) -> Q {
    (1..=n).fold(Q::zero(), |s, k| s + Q::new(BigInt::one(), BigInt::from(k)))
}

fn eval_poly(poly: &UniPoly, x: &Q) -> Q {
    poly.coeffs
        .iter()
        .rev()
        .fold(Q::zero(), |acc, c| acc * x + c)
}

fn derivative_at(poly: &UniPoly, x: &Q) -> Q {
    poly.coeffs
        .iter()
        .enumerate()
        .skip(1)
        .rev()
        .fold(Q::zero(), |acc, (i, c)| {
            acc * x + c * Q::from_integer(BigInt::from(i))
        })
}

/// Telescoping rational sums.
fn rational_telescoping(
    graph: &ExprGraph,
    expr: ExprId,
    var: SymbolId,
    lower: i64,
) -> Option<ExprId> {
    let together = crate::simplify::AlgebraicTransformations::together(graph, expr);
    let (numerator, denominator) = match graph.get(together).kind.clone() {
        ExprKind::Div(n, d) => (n, d),
        _ => return None,
    };
    let (num, den) = (
        UniPoly::from_expr(graph, numerator, var).ok()?,
        UniPoly::from_expr(graph, denominator, var).ok()?,
    );
    let (dn, dd) = (num.degree(), den.degree());
    if dd < 2 || dn + 2 > dd {
        return None;
    }
    // Integer roots of the denominator, each simple, accounting for the whole degree.
    let mut roots = Vec::new();
    for candidate in -60..=60i64 {
        if eval_poly(&den, &Q::from_integer(BigInt::from(candidate))).is_zero() {
            roots.push(candidate);
        }
    }
    if roots.len() != dd {
        return None;
    }
    let mut total = Q::zero();
    let mut residues = Q::zero();
    for &r in &roots {
        let point = Q::from_integer(BigInt::from(r));
        let slope = derivative_at(&den, &point);
        if slope.is_zero() {
            return None;
        }
        let c = eval_poly(&num, &point) / slope;
        // 1/(k - r) summed from `lower`: needs lower - r >= 1.
        let n = lower - r - 1;
        if n < 0 {
            return None;
        }
        residues += &c;
        total -= c * harmonic(n);
    }
    if !residues.is_zero() {
        return None;
    }
    Some(expr_from_rational(graph, &total))
}

fn exact_at(graph: &ExprGraph, expr: ExprId, var: SymbolId, k: i64) -> Option<Q> {
    let value = graph.substitute(expr, var, graph.integer(k));
    try_eval_exact(graph, value, &HashMap::new()).ok().flatten()
}

/// Classical series recognised by their exact leading terms.
fn classical(graph: &ExprGraph, expr: ExprId, var: SymbolId, lower: i64) -> Option<ExprId> {
    let q = |n: i64, d: i64| Q::new(BigInt::from(n), BigInt::from(d));
    let sign = |k: i64| if k.rem_euclid(2) == 0 { 1 } else { -1 };
    type Term = Box<dyn Fn(i64) -> Q>;
    let candidates: Vec<(i64, Term, ExprId)> = vec![
        // (first index, k-th term, value)
        (
            1,
            Box::new(move |k| q(sign(k + 1), k)),
            graph.function("ln", [graph.integer(2)]),
        ),
        (
            1,
            Box::new(move |k| q(sign(k), k)),
            graph.neg(graph.function("ln", [graph.integer(2)])),
        ),
        (
            0,
            Box::new(move |k| q(sign(k), 2 * k + 1)),
            graph.div(graph.constant(Constant::Pi), graph.integer(4)),
        ),
        (
            1,
            Box::new(move |k| q(1, (2 * k - 1) * (2 * k - 1))),
            graph.div(
                graph.pow(graph.constant(Constant::Pi), graph.integer(2)),
                graph.integer(8),
            ),
        ),
        (
            1,
            Box::new(move |k| q(sign(k + 1), k * k)),
            graph.div(
                graph.pow(graph.constant(Constant::Pi), graph.integer(2)),
                graph.integer(12),
            ),
        ),
        (
            0,
            Box::new(move |k| Q::new(BigInt::one(), (1..=k).product::<i64>().into())),
            graph.function("exp", [graph.integer(1)]),
        ),
    ];
    for (first, term, value) in candidates {
        if lower != first {
            continue;
        }
        let matches = (first..first + 12)
            .all(|k| exact_at(graph, expr, var, k).is_some_and(|v| v == term(k)));
        if matches {
            return Some(value);
        }
    }
    None
}

/// `x^k / k!` from 0 and `x^k / k` from 1 with `x` free of `k`.
fn power_series(graph: &ExprGraph, expr: ExprId, var: SymbolId, lower: i64) -> Option<ExprId> {
    let k = graph.symbol(&graph.symbols.resolve(var)?);
    let (numerator, denominator) = match graph.get(expr).kind.clone() {
        ExprKind::Div(n, d) => (n, d),
        _ => return None,
    };
    let base = match graph.get(numerator).kind.clone() {
        ExprKind::Pow(b, e) if e == k && !graph.has_symbol(b, var) => b,
        _ => return None,
    };
    let is_factorial_of_k = matches!(
        graph.get(denominator).kind.clone(),
        ExprKind::Function { name, args } if graph.symbols.resolve(name).as_deref() == Some("factorial") && args.len() == 1 && args[0] == k
    );
    if lower == 0 && is_factorial_of_k {
        return Some(graph.function("exp", [base]));
    }
    if lower == 1 && denominator == k {
        // -ln(1 - x), for |x| < 1 (and x = -1).
        return Some(graph.neg(graph.function("ln", [graph.sub(graph.integer(1), base)])));
    }
    None
}

/// A closed form for `sum_{var = lower}^{oo} expr`, if one of the families above applies.
pub fn infinite_sum(graph: &ExprGraph, expr: ExprId, var: SymbolId, lower: i64) -> Option<ExprId> {
    rational_telescoping(graph, expr, var, lower)
        .or_else(|| classical(graph, expr, var, lower))
        .or_else(|| power_series(graph, expr, var, lower))
}
