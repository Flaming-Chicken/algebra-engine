//! Named formulas as expression rewrites: geometry (`circle_area`, `sphere_volume`, `hypot`, `deg2rad`) and
//! probability (`normal_pdf`, `normal_cdf`, `poisson_pmf`, `binomial_pmf`, `exponential_cdf`, ...).
//!
//! Each name is replaced by its defining expression in terms of `pi`, `exp`, `erf` and `sqrt`, so the result is exact
//! and the numerical evaluator needs nothing new.

use algebra_core::{Constant, ExprGraph, ExprId};

/// The expression a call to `name` stands for, or `None` if `name` is not in the formulary or the argument count is
/// wrong.
pub fn rewrite(graph: &ExprGraph, name: &str, args: &[ExprId]) -> Option<ExprId> {
    let pi = graph.constant(Constant::Pi);
    let int = |n: i64| graph.integer(n);
    let square = |x: ExprId| graph.pow(x, int(2));
    let half = graph.rational(1, 2);
    let sqrt = |x: ExprId| graph.function("sqrt", [x]);
    let exp = |x: ExprId| graph.function("exp", [x]);
    // x standardised: (x - mu) / sigma, with the default N(0, 1).
    let standardised = |x: ExprId, mu: Option<ExprId>, sigma: Option<ExprId>| -> ExprId {
        let centred = match mu {
            Some(m) => graph.sub(x, m),
            None => x,
        };
        match sigma {
            Some(s) => graph.div(centred, s),
            None => centred,
        }
    };
    Some(match (name, args) {
        ("circle_area", [r]) => graph.mul([pi, square(*r)]),
        ("circle_circumference" | "circle_perimeter", [r]) => graph.mul([int(2), pi, *r]),
        ("sphere_volume", [r]) => graph.mul([graph.rational(4, 3), pi, graph.pow(*r, int(3))]),
        ("sphere_area" | "sphere_surface", [r]) => graph.mul([int(4), pi, square(*r)]),
        ("cylinder_volume", [r, h]) => graph.mul([pi, square(*r), *h]),
        ("cone_volume", [r, h]) => graph.mul([graph.rational(1, 3), pi, square(*r), *h]),
        ("hypot", [a, b]) => sqrt(graph.add([square(*a), square(*b)])),
        ("deg2rad", [d]) => graph.div(graph.mul([*d, pi]), int(180)),
        ("rad2deg", [r]) => graph.div(graph.mul([int(180), *r]), pi),
        ("normal_pdf", [x]) | ("normal_pdf", [x, _, _]) => {
            let (mu, sigma) = if args.len() == 3 {
                (Some(args[1]), Some(args[2]))
            } else {
                (None, None)
            };
            let z = standardised(*x, mu, sigma);
            let density = graph.div(
                exp(graph.neg(graph.mul([half, square(z)]))),
                sqrt(graph.mul([int(2), pi])),
            );
            match sigma {
                Some(s) => graph.div(density, s),
                None => density,
            }
        }
        ("normal_cdf", [x]) | ("normal_cdf", [x, _, _]) => {
            let (mu, sigma) = if args.len() == 3 {
                (Some(args[1]), Some(args[2]))
            } else {
                (None, None)
            };
            let z = standardised(*x, mu, sigma);
            graph.mul([
                half,
                graph.add([int(1), graph.function("erf", [graph.div(z, sqrt(int(2)))])]),
            ])
        }
        ("exponential_pdf", [x, rate]) => {
            graph.mul([*rate, exp(graph.neg(graph.mul([*rate, *x])))])
        }
        ("exponential_cdf", [x, rate]) => graph.sub(int(1), exp(graph.neg(graph.mul([*rate, *x])))),
        ("poisson_pmf", [k, lambda]) => graph.div(
            graph.mul([exp(graph.neg(*lambda)), graph.pow(*lambda, *k)]),
            graph.function("factorial", [*k]),
        ),
        ("binomial_pmf", [n, p, k]) => graph.mul([
            graph.div(
                graph.function("factorial", [*n]),
                graph.mul([
                    graph.function("factorial", [*k]),
                    graph.function("factorial", [graph.sub(*n, *k)]),
                ]),
            ),
            graph.pow(*p, *k),
            graph.pow(graph.sub(int(1), *p), graph.sub(*n, *k)),
        ]),
        ("geometric_pmf", [k, p]) => {
            graph.mul([graph.pow(graph.sub(int(1), *p), graph.sub(*k, int(1))), *p])
        }
        ("uniform_cdf", [x, a, b]) => graph.div(graph.sub(*x, *a), graph.sub(*b, *a)),
        _ => return None,
    })
}
