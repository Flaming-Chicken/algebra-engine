//! Integrals of products of exponentials and trigonometric functions with linear arguments, and of polynomials
//! times such a function, in closed form.
//!
//! Three families, all with arguments linear in the integration variable (`a x + b`, rational `a != 0`):
//!
//! * `P(x) g(a x + b)` with `P` a polynomial and `g` one of `exp`, `sin`, `cos`: tabular integration by parts,
//!   $\int P g = \sum_{k \ge 0} (-1)^k P^{(k)} G_{k+1}$ with $G_{j+1} = \int G_j$;
//! * `exp(u) sin(v)` and `exp(u) cos(v)`: $\int e^u \sin v = e^u (\alpha \sin v - \gamma \cos v) / (\alpha^2 +
//!   \gamma^2)$ and the analogue for cosine, for $u' = \alpha$, $v' = \gamma$;
//! * `sin(u) cos(v)`, `sin(u) sin(v)`, `cos(u) cos(v)`: product-to-sum, then each term is a single trigonometric
//!   function of a linear argument.
//!
//! It also integrates `abs(a x + b)` as $u |u| / (2a)$.

use crate::poly::{UniPoly, expr_from_rational};
use algebra_core::{AlgebraResult, ExprGraph, ExprId, ExprKind, SymbolId};
use num_rational::BigRational;
use num_traits::{One, Zero};

/// `u = a x + b` with rational `a != 0`, as `(a, b)`.
fn linear_form(graph: &ExprGraph, id: ExprId, wrt: SymbolId) -> Option<(BigRational, BigRational)> {
    let p = UniPoly::from_expr(graph, id, wrt).ok()?;
    if p.degree() != 1 {
        return None;
    }
    let a = p.coeffs.get(1)?.clone();
    if a.is_zero() {
        return None;
    }
    Some((
        a,
        p.coeffs.first().cloned().unwrap_or_else(BigRational::zero),
    ))
}

/// The expression `a x + b`.
fn linear_expr(graph: &ExprGraph, x: ExprId, a: &BigRational, b: &BigRational) -> ExprId {
    let ax = if a.is_one() {
        x
    } else {
        graph.mul([expr_from_rational(graph, a), x])
    };
    if b.is_zero() {
        ax
    } else {
        graph.add([ax, expr_from_rational(graph, b)])
    }
}

/// The polynomial with coefficients `coeffs` (lowest degree first) as an expression.
fn poly_expr(graph: &ExprGraph, x: ExprId, coeffs: &[BigRational]) -> ExprId {
    let mut terms = Vec::new();
    for (k, c) in coeffs.iter().enumerate() {
        if c.is_zero() {
            continue;
        }
        let power = match k {
            0 => None,
            1 => Some(x),
            _ => Some(graph.pow(x, graph.integer(k as i64))),
        };
        terms.push(match power {
            None => expr_from_rational(graph, c),
            Some(p) if c.is_one() => p,
            Some(p) => graph.mul([expr_from_rational(graph, c), p]),
        });
    }
    match terms.len() {
        0 => graph.integer(0),
        1 => terms[0],
        _ => graph.add(terms),
    }
}

fn derivative_coeffs(coeffs: &[BigRational]) -> Vec<BigRational> {
    coeffs
        .iter()
        .enumerate()
        .skip(1)
        .map(|(k, c)| c * BigRational::from_integer((k as i64).into()))
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Exp,
    Sin,
    Cos,
}

fn kind_of(graph: &ExprGraph, id: ExprId) -> Option<(Kind, ExprId)> {
    let ExprKind::Function { name, args } = &graph.get(id).kind else {
        return None;
    };
    if args.len() != 1 {
        return None;
    }
    let kind = match graph.symbols.resolve(*name)?.as_str() {
        "exp" => Kind::Exp,
        "sin" => Kind::Sin,
        "cos" => Kind::Cos,
        _ => return None,
    };
    Some((kind, args[0]))
}

/// `G_j`: the `j`-fold antiderivative of `g(a x + b)` (with the constants of integration taken as zero), as a
/// rational multiple of `exp(u)`, `sin(u)` or `cos(u)`.
fn antiderivative_power(
    graph: &ExprGraph,
    kind: Kind,
    u: ExprId,
    a: &BigRational,
    j: u32,
) -> ExprId {
    let mut scale = BigRational::one();
    for _ in 0..j {
        scale /= a;
    }
    let (sign, fun) = match (kind, j % 4) {
        (Kind::Exp, _) => (1, "exp"),
        (Kind::Sin, 1) => (-1, "cos"),
        (Kind::Sin, 2) => (-1, "sin"),
        (Kind::Sin, 3) => (1, "cos"),
        (Kind::Sin, _) => (1, "sin"),
        (Kind::Cos, 1) => (1, "sin"),
        (Kind::Cos, 2) => (-1, "cos"),
        (Kind::Cos, 3) => (-1, "sin"),
        (Kind::Cos, _) => (1, "cos"),
    };
    let coeff = if sign < 0 { -scale } else { scale };
    let f = graph.function(fun, [u]);
    if coeff.is_one() {
        f
    } else {
        graph.mul([expr_from_rational(graph, &coeff), f])
    }
}

/// `int sin(w) dx` or `int cos(w) dx` for `w = c x + d`; a constant `w` gives `f(d) x`.
fn trig_of_linear(
    graph: &ExprGraph,
    x: ExprId,
    sine: bool,
    c: &BigRational,
    d: &BigRational,
) -> ExprId {
    if c.is_zero() {
        let value = graph.function(
            if sine { "sin" } else { "cos" },
            [expr_from_rational(graph, d)],
        );
        return graph.mul([value, x]);
    }
    let w = linear_expr(graph, x, c, d);
    let inv = BigRational::one() / c;
    if sine {
        graph.mul([expr_from_rational(graph, &-inv), graph.function("cos", [w])])
    } else {
        graph.mul([expr_from_rational(graph, &inv), graph.function("sin", [w])])
    }
}

/// Integrate a product of two factors in the families above, or `None` if it is not one of them.
pub(crate) fn integrate_exp_trig_product(
    graph: &ExprGraph,
    factors: &[ExprId],
    wrt: SymbolId,
    x: ExprId,
) -> Option<ExprId> {
    if factors.len() != 2 {
        return None;
    }
    for (pi, gi) in [(0usize, 1usize), (1, 0)] {
        let Some((kind, arg)) = kind_of(graph, factors[gi]) else {
            continue;
        };
        let Some((a, _b)) = linear_form(graph, arg, wrt) else {
            continue;
        };
        // Polynomial times g(a x + b): tabular integration by parts.
        if let Ok(p) = UniPoly::from_expr(graph, factors[pi], wrt) {
            if p.is_zero() {
                continue;
            }
            let mut terms = Vec::new();
            let mut coeffs = p.coeffs.clone();
            let mut k = 0u32;
            while !coeffs.is_empty() {
                let pk = poly_expr(graph, x, &coeffs);
                let g = antiderivative_power(graph, kind, arg, &a, k + 1);
                let term = graph.mul([pk, g]);
                terms.push(if k.is_multiple_of(2) {
                    term
                } else {
                    graph.neg(term)
                });
                coeffs = derivative_coeffs(&coeffs);
                k += 1;
            }
            return Some(graph.add(terms));
        }
    }
    // exp(u) sin(v) / exp(u) cos(v), and the trigonometric products.
    let (k0, u0) = kind_of(graph, factors[0])?;
    let (k1, u1) = kind_of(graph, factors[1])?;
    let (la, lb) = (linear_form(graph, u0, wrt)?, linear_form(graph, u1, wrt)?);
    let half = BigRational::new(1.into(), 2.into());
    match (k0, k1) {
        (Kind::Exp, Kind::Sin | Kind::Cos) | (Kind::Sin | Kind::Cos, Kind::Exp) => {
            let (e_arg, e_form, t_kind, t_arg, t_form) = if k0 == Kind::Exp {
                (u0, &la, k1, u1, &lb)
            } else {
                (u1, &lb, k0, u0, &la)
            };
            let (alpha, gamma) = (e_form.0.clone(), t_form.0.clone());
            let denominator = &alpha * &alpha + &gamma * &gamma;
            let e = graph.function("exp", [e_arg]);
            let sin_v = graph.function("sin", [t_arg]);
            let cos_v = graph.function("cos", [t_arg]);
            let (first, second) = if t_kind == Kind::Sin {
                // e^u (alpha sin v - gamma cos v) / (alpha^2 + gamma^2)
                (
                    graph.mul([expr_from_rational(graph, &alpha), sin_v]),
                    graph.neg(graph.mul([expr_from_rational(graph, &gamma), cos_v])),
                )
            } else {
                // e^u (alpha cos v + gamma sin v) / (alpha^2 + gamma^2)
                (
                    graph.mul([expr_from_rational(graph, &alpha), cos_v]),
                    graph.mul([expr_from_rational(graph, &gamma), sin_v]),
                )
            };
            let inv = BigRational::one() / denominator;
            Some(graph.mul([
                expr_from_rational(graph, &inv),
                e,
                graph.add([first, second]),
            ]))
        }
        (Kind::Sin | Kind::Cos, Kind::Sin | Kind::Cos) => {
            let (ua, ub) = (la, lb);
            let sum = (&ua.0 + &ub.0, &ua.1 + &ub.1);
            let diff = (&ua.0 - &ub.0, &ua.1 - &ub.1);
            let (term_sum, term_diff) = match (k0, k1) {
                // sin u cos v = 1/2 [sin(u + v) + sin(u - v)]
                (Kind::Sin, Kind::Cos) => (
                    trig_of_linear(graph, x, true, &sum.0, &sum.1),
                    trig_of_linear(graph, x, true, &diff.0, &diff.1),
                ),
                // cos u sin v = 1/2 [sin(u + v) - sin(u - v)]
                (Kind::Cos, Kind::Sin) => (
                    trig_of_linear(graph, x, true, &sum.0, &sum.1),
                    graph.neg(trig_of_linear(graph, x, true, &diff.0, &diff.1)),
                ),
                // sin u sin v = 1/2 [cos(u - v) - cos(u + v)]
                (Kind::Sin, Kind::Sin) => (
                    graph.neg(trig_of_linear(graph, x, false, &sum.0, &sum.1)),
                    trig_of_linear(graph, x, false, &diff.0, &diff.1),
                ),
                // cos u cos v = 1/2 [cos(u - v) + cos(u + v)]
                _ => (
                    trig_of_linear(graph, x, false, &sum.0, &sum.1),
                    trig_of_linear(graph, x, false, &diff.0, &diff.1),
                ),
            };
            Some(graph.mul([
                expr_from_rational(graph, &half),
                graph.add([term_sum, term_diff]),
            ]))
        }
        _ => None,
    }
}

/// `int abs(a x + b) dx = u |u| / (2 a)` for `u = a x + b`.
pub(crate) fn integrate_abs(
    graph: &ExprGraph,
    id: ExprId,
    wrt: SymbolId,
) -> Option<AlgebraResult<ExprId>> {
    let ExprKind::Function { name, args } = &graph.get(id).kind else {
        return None;
    };
    if args.len() != 1 || graph.symbols.resolve(*name)?.as_str() != "abs" {
        return None;
    }
    let (a, _b) = linear_form(graph, args[0], wrt)?;
    let u = args[0];
    let scale = BigRational::one() / (BigRational::from_integer(2.into()) * a);
    Some(Ok(graph.mul([
        expr_from_rational(graph, &scale),
        u,
        graph.function("abs", [u]),
    ])))
}
