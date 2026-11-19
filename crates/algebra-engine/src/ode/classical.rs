//! Classical first- and second-order ODE methods that the `dsolve` command needs beyond the constant-coefficient and
//! first-order linear cases:
//!
//! * a **linearity check**: the command read the coefficients of `y''`, `y'` and `y` by substituting `0` and `1`,
//!   which silently turned `y' = y^2` into `y' = y` (and `y' = y (1 - y)` into `y' = 0`); the model is now checked
//!   against the equation at random points before it is trusted;
//! * **separable** equations `y' = f(x) g(y)`: `int dy / g(y) = int f(x) dx + C`, solved for `y` when possible;
//! * constant-coefficient **non-homogeneous** equations `a y'' + b y' + c y = g(x)` by variation of parameters from
//!   the homogeneous basis: $y_p = -y_1 \int y_2 g / (a W) + y_2 \int y_1 g / (a W)$.

use crate::calculus::SymbolicCalculus;
use crate::numeric::{EvalContext, NumericalEval};
use crate::simplify::Simplifier;
use algebra_core::{ExprGraph, ExprId, ExprKind, SymbolId};
use std::collections::HashMap;

fn simplified(graph: &ExprGraph, id: ExprId) -> ExprId {
    // Distribute, merge the exponentials (so `x exp(x) exp(x) - x exp(2 x)` cancels), simplify, merge again.
    let expanded = crate::simplify::AlgebraicTransformations::expand(graph, id);
    let merged = merge_exponentials(graph, expanded);
    let simplified = Simplifier::simplify(graph, merged).unwrap_or(merged);
    merge_exponentials(graph, simplified)
}

/// Rewrite products of exponentials as one exponential, `exp(u) exp(v) = exp(u + v)` and `exp(u) / exp(v) =
/// exp(u - v)` (an exponential of zero is dropped): the general simplifier leaves `exp(x) exp(-x)` as it is, which
/// hides a Wronskian of `-2` behind exponentials.
fn merge_exponentials(graph: &ExprGraph, id: ExprId) -> ExprId {
    /// Split a product-like expression into non-exponential factors and the arguments of its exponential factors
    /// (`sign = -1` inverts them, for a denominator).
    fn split(
        graph: &ExprGraph,
        e: ExprId,
        negate: bool,
        others: &mut Vec<ExprId>,
        args: &mut Vec<ExprId>,
    ) {
        let signed = |u: ExprId| if negate { graph.neg(u) } else { u };
        match graph.get(e).kind.clone() {
            ExprKind::Mul(fs) => {
                for f in fs {
                    split(graph, f, negate, others, args);
                }
            }
            ExprKind::Neg(a) => {
                others.push(graph.integer(-1));
                split(graph, a, negate, others, args);
            }
            ExprKind::Div(a, b) => {
                split(graph, a, negate, others, args);
                split(graph, b, !negate, others, args);
            }
            ExprKind::Function { name, args: fa }
                if fa.len() == 1 && graph.symbols.resolve(name).as_deref() == Some("exp") =>
            {
                args.push(signed(fa[0]));
            }
            ExprKind::Pow(base, exp) => {
                if let ExprKind::Function { name, args: fa } = graph.get(base).kind.clone()
                    && fa.len() == 1
                    && graph.symbols.resolve(name).as_deref() == Some("exp")
                {
                    args.push(signed(graph.mul([exp, fa[0]])));
                } else {
                    let merged = merge_exponentials(graph, base);
                    let p = graph.pow(merged, exp);
                    others.push(if negate {
                        graph.pow(p, graph.integer(-1))
                    } else {
                        p
                    });
                }
            }
            ExprKind::Add(_) | ExprKind::Sub(_, _) => {
                let merged = merge_exponentials(graph, e);
                others.push(if negate {
                    graph.pow(merged, graph.integer(-1))
                } else {
                    merged
                });
            }
            _ => others.push(if negate {
                graph.pow(e, graph.integer(-1))
            } else {
                e
            }),
        }
    }
    match graph.get(id).kind.clone() {
        ExprKind::Add(terms) => graph.add(
            terms
                .iter()
                .map(|&t| merge_exponentials(graph, t))
                .collect::<Vec<_>>(),
        ),
        ExprKind::Sub(a, b) => {
            graph.sub(merge_exponentials(graph, a), merge_exponentials(graph, b))
        }
        ExprKind::Mul(_) | ExprKind::Div(_, _) | ExprKind::Neg(_) | ExprKind::Pow(_, _) => {
            let (mut others, mut args) = (Vec::new(), Vec::new());
            split(graph, id, false, &mut others, &mut args);
            if !args.is_empty() {
                let sum = Simplifier::simplify(graph, graph.add(args.clone()))
                    .unwrap_or_else(|_| graph.add(args));
                if sum != graph.integer(0) {
                    others.push(graph.function("exp", [sum]));
                }
            }
            match others.len() {
                0 => graph.integer(1),
                1 => others[0],
                _ => graph.mul(others),
            }
        }
        _ => id,
    }
}

/// Whether `ode = c2 d2 + c1 d1 + c0 y + base` holds identically (checked at several pseudo-random points).
pub(crate) fn is_linear_model(
    graph: &ExprGraph,
    ode: ExprId,
    symbols: [SymbolId; 4],    // d2, d1, y, x
    coefficients: [ExprId; 4], // c2, c1, c0, base
) -> bool {
    let names: Vec<String> = symbols
        .iter()
        .map(|&s| graph.symbols.resolve(s).unwrap_or_default())
        .collect();
    let points: [[f64; 4]; 4] = [
        [0.7, -1.3, 0.9, 1.7],
        [-1.1, 0.4, 2.3, 0.6],
        [2.2, 1.9, -0.8, 2.9],
        [0.3, -0.6, 1.4, 0.45],
    ];
    for p in points {
        let bindings: HashMap<String, f64> = names.iter().cloned().zip(p).collect();
        let ctx = EvalContext::with_bindings(50, bindings);
        let Ok(actual) = graph.evalf(ode, &ctx).map(|v| v.to_f64()) else {
            return false;
        };
        let value = |id: ExprId| graph.evalf(id, &ctx).map(|v| v.to_f64());
        let (Ok(c2), Ok(c1), Ok(c0), Ok(b)) = (
            value(coefficients[0]),
            value(coefficients[1]),
            value(coefficients[2]),
            value(coefficients[3]),
        ) else {
            return false;
        };
        let model = c2 * p[0] + c1 * p[1] + c0 * p[2] + b;
        if !actual.is_finite()
            || !model.is_finite()
            || (actual - model).abs() > 1e-8 * (1.0 + actual.abs())
        {
            return false;
        }
    }
    true
}

/// Multiplicative factors of an expression: products are flattened, a quotient contributes the numerator's factors
/// and the denominator's as inverse powers, a negation contributes `-1`.
fn factors_of(graph: &ExprGraph, id: ExprId, out: &mut Vec<ExprId>) {
    match &graph.get(id).kind {
        ExprKind::Mul(fs) => {
            for &f in fs {
                factors_of(graph, f, out);
            }
        }
        ExprKind::Div(n, d) => {
            factors_of(graph, *n, out);
            let mut den = Vec::new();
            factors_of(graph, *d, &mut den);
            for f in den {
                out.push(graph.pow(f, graph.integer(-1)));
            }
        }
        ExprKind::Neg(a) => {
            out.push(graph.integer(-1));
            factors_of(graph, *a, out);
        }
        _ => out.push(id),
    }
}

/// The solution of a separable equation.
pub(crate) enum Separable {
    /// Explicit solutions `y = ...` (with the constant `C1`).
    Explicit(Vec<ExprId>),
    /// The implicit solution `g(y) = f(x) + C1`, when it cannot be solved for `y` in closed form.
    Implicit { g: ExprId, f: ExprId },
}

/// `y' = f(x) g(y)`: the solution, if the right-hand side separates.
pub(crate) fn solve_separable(
    graph: &ExprGraph,
    ode: ExprId,
    d1: SymbolId,
    y: SymbolId,
    x: SymbolId,
) -> Option<Separable> {
    use crate::solver::SymbolicSolver;
    // ode = d1 - h(x, y): h = -ode at d1 = 0 (the equation must be linear in d1 with coefficient 1).
    let at_zero = simplified(graph, graph.substitute(ode, d1, graph.integer(0)));
    let h = simplified(graph, graph.neg(at_zero));
    let mut factors = Vec::new();
    factors_of(graph, h, &mut factors);
    let (mut fx, mut gy) = (Vec::new(), Vec::new());
    for f in factors {
        let (has_x, has_y) = (graph.has_symbol(f, x), graph.has_symbol(f, y));
        match (has_x, has_y) {
            (true, true) => return None,
            (false, true) => gy.push(f),
            _ => fx.push(f),
        }
    }
    if gy.is_empty() {
        return None; // y' = f(x): a plain integral, handled elsewhere
    }
    let f_expr = if fx.is_empty() {
        graph.integer(1)
    } else {
        graph.mul(fx)
    };
    let g_expr = graph.mul(gy);
    // G(y) = int dy / g(y), F(x) = int f dx.
    let big_g = graph
        .integrate(simplified(graph, graph.div(graph.integer(1), g_expr)), y)
        .ok()?;
    let big_f = graph.integrate(f_expr, x).ok()?;
    let c1 = graph.symbol("C1");
    let implicit = graph.sub(graph.sub(big_g, big_f), c1);
    match graph.solveset(implicit, y) {
        Ok(solutions)
            if !solutions.is_empty() && !solutions.iter().any(|&s| graph.has_symbol(s, y)) =>
        {
            Some(Separable::Explicit(
                solutions
                    .into_iter()
                    .map(|s| simplified(graph, s))
                    .collect(),
            ))
        }
        _ => Some(Separable::Implicit {
            g: simplified(graph, big_g),
            f: simplified(graph, big_f),
        }),
    }
}

/// A particular solution of `a y'' + b y' + c y = g(x)` from the homogeneous basis `y1`, `y2`, by variation of
/// parameters, as the full solution `C1 y1 + C2 y2 + y_p`.
pub(crate) fn variation_of_parameters(
    graph: &ExprGraph,
    a: ExprId,
    forcing: ExprId,
    y1: ExprId,
    y2: ExprId,
    x: SymbolId,
) -> Option<ExprId> {
    let dy1 = graph.diff(y1, x);
    let dy2 = graph.diff(y2, x);
    let wronskian = simplified(graph, graph.sub(graph.mul([y1, dy2]), graph.mul([dy1, y2])));
    let denominator = simplified(graph, graph.mul([a, wronskian]));
    // Simplified, then merged again: the general simplifier rewrites `exp(2 x)` as `exp(x)^2`, which the integrator
    // does not take.
    let integrand = |u: ExprId| {
        merge_exponentials(
            graph,
            simplified(graph, graph.div(graph.mul([u, forcing]), denominator)),
        )
    };
    let first = graph.integrate(integrand(y2), x).ok()?;
    let second = graph.integrate(integrand(y1), x).ok()?;
    let particular = graph.add([graph.neg(graph.mul([y1, first])), graph.mul([y2, second])]);
    let c1 = graph.symbol("C1");
    let c2 = graph.symbol("C2");
    let solution = graph.add([graph.mul([c1, y1]), graph.mul([c2, y2]), particular]);
    Some(simplified(graph, solution))
}
