//! Trigonometric expansion (`sin(a + b)`, `cos(3 x)`) and a Pythagorean reduction (`trigsimp`).
//!
//! The angle-addition formulas are applied recursively: `sin(a + b) = sin a cos b + cos a sin b`,
//! `cos(a + b) = cos a cos b - sin a sin b`, and an integer multiple `n f` is `(n - 1) f + f`.

use algebra_core::{ExprGraph, ExprId, ExprKind, Number};

fn sin_cos_of(graph: &ExprGraph, angle: ExprId) -> (ExprId, ExprId) {
    // Both functions of an angle, expanded.
    (
        expand_function(graph, "sin", angle),
        expand_function(graph, "cos", angle),
    )
}

fn integer_multiple(graph: &ExprGraph, angle: ExprId) -> Option<(i64, ExprId)> {
    if let ExprKind::Mul(factors) = graph.get(angle).kind.clone() {
        for (i, f) in factors.iter().enumerate() {
            if let ExprKind::Number(Number::Integer(n)) = graph.get(*f).kind
                && n.abs() >= 2
                && n.abs() <= 16
            {
                let rest: Vec<ExprId> = factors
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| *j != i)
                    .map(|(_, &g)| g)
                    .collect();
                let rest = if rest.len() == 1 {
                    rest[0]
                } else {
                    graph.mul(rest)
                };
                return Some((n, rest));
            }
        }
    }
    None
}

/// `sin(angle)` or `cos(angle)` with sums and integer multiples expanded.
fn expand_function(graph: &ExprGraph, name: &str, angle: ExprId) -> ExprId {
    let is_sin = name == "sin";
    match graph.get(angle).kind.clone() {
        ExprKind::Neg(inner) => {
            let value = expand_function(graph, name, inner);
            return if is_sin { graph.neg(value) } else { value };
        }
        ExprKind::Add(terms) if terms.len() >= 2 => {
            let first = terms[0];
            let rest = if terms.len() == 2 {
                terms[1]
            } else {
                graph.add(terms[1..].to_vec())
            };
            let (sa, ca) = sin_cos_of(graph, first);
            let (sb, cb) = sin_cos_of(graph, rest);
            return if is_sin {
                graph.add([graph.mul([sa, cb]), graph.mul([ca, sb])])
            } else {
                graph.sub(graph.mul([ca, cb]), graph.mul([sa, sb]))
            };
        }
        ExprKind::Sub(a, b) => {
            return expand_function(graph, name, graph.add([a, graph.neg(b)]));
        }
        _ => {}
    }
    if let Some((n, factor)) = integer_multiple(graph, angle) {
        if n < 0 {
            let positive = expand_function(graph, name, graph.mul([graph.integer(-n), factor]));
            return if is_sin {
                graph.neg(positive)
            } else {
                positive
            };
        }
        let (s1, c1) = (
            graph.function("sin", [factor]),
            graph.function("cos", [factor]),
        );
        let (mut s, mut c) = (s1, c1);
        for _ in 1..n {
            let next_s = graph.add([graph.mul([s, c1]), graph.mul([c, s1])]);
            let next_c = graph.sub(graph.mul([c, c1]), graph.mul([s, s1]));
            (s, c) = (next_s, next_c);
        }
        return if is_sin { s } else { c };
    }
    graph.function(name, [angle])
}

/// Expand every `sin`, `cos` and `tan` of a sum or an integer multiple inside `expr`.
pub fn expand_trig(graph: &ExprGraph, expr: ExprId) -> ExprId {
    match graph.get(expr).kind.clone() {
        ExprKind::Add(ts) => graph.add(
            ts.iter()
                .map(|&t| expand_trig(graph, t))
                .collect::<Vec<_>>(),
        ),
        ExprKind::Mul(fs) => graph.mul(
            fs.iter()
                .map(|&t| expand_trig(graph, t))
                .collect::<Vec<_>>(),
        ),
        ExprKind::Sub(a, b) => graph.sub(expand_trig(graph, a), expand_trig(graph, b)),
        ExprKind::Div(a, b) => graph.div(expand_trig(graph, a), expand_trig(graph, b)),
        ExprKind::Pow(a, b) => graph.pow(expand_trig(graph, a), expand_trig(graph, b)),
        ExprKind::Neg(a) => graph.neg(expand_trig(graph, a)),
        ExprKind::Function { name, args } if args.len() == 1 => {
            let fname = graph.symbols.resolve(name).unwrap_or_default();
            let angle = expand_trig(graph, args[0]);
            match fname.as_str() {
                "sin" | "cos" => expand_function(graph, &fname, angle),
                "tan" => graph.div(
                    expand_function(graph, "sin", angle),
                    expand_function(graph, "cos", angle),
                ),
                _ => graph.function(&fname, [angle]),
            }
        }
        _ => expr,
    }
}

/// Whether `expr` contains a trigonometric function.
pub fn has_trig(graph: &ExprGraph, expr: ExprId) -> bool {
    match graph.get(expr).kind.clone() {
        ExprKind::Add(ts) | ExprKind::Mul(ts) => ts.iter().any(|&t| has_trig(graph, t)),
        ExprKind::Sub(a, b) | ExprKind::Div(a, b) | ExprKind::Pow(a, b) => {
            has_trig(graph, a) || has_trig(graph, b)
        }
        ExprKind::Neg(a) => has_trig(graph, a),
        ExprKind::Function { name, args } => {
            matches!(
                graph.symbols.resolve(name).as_deref(),
                Some("sin" | "cos" | "tan")
            ) || args.iter().any(|&a| has_trig(graph, a))
        }
        _ => false,
    }
}

/// Replace `sin(a)^k` by powers of `cos(a)` through `sin^2 = 1 - cos^2`.
fn sin_squared_to_cos(graph: &ExprGraph, expr: ExprId) -> ExprId {
    match graph.get(expr).kind.clone() {
        ExprKind::Add(ts) => graph.add(
            ts.iter()
                .map(|&t| sin_squared_to_cos(graph, t))
                .collect::<Vec<_>>(),
        ),
        ExprKind::Mul(fs) => graph.mul(
            fs.iter()
                .map(|&t| sin_squared_to_cos(graph, t))
                .collect::<Vec<_>>(),
        ),
        ExprKind::Sub(a, b) => {
            graph.sub(sin_squared_to_cos(graph, a), sin_squared_to_cos(graph, b))
        }
        ExprKind::Neg(a) => graph.neg(sin_squared_to_cos(graph, a)),
        ExprKind::Div(a, b) => {
            graph.div(sin_squared_to_cos(graph, a), sin_squared_to_cos(graph, b))
        }
        ExprKind::Pow(base, exponent) => {
            if let ExprKind::Function { name, args } = graph.get(base).kind.clone()
                && args.len() == 1
                && graph.symbols.resolve(name).as_deref() == Some("sin")
                && let ExprKind::Number(Number::Integer(k)) = graph.get(exponent).kind
                && k >= 2
            {
                let cos = graph.function("cos", [args[0]]);
                let one_minus = graph.sub(graph.integer(1), graph.pow(cos, graph.integer(2)));
                let squares = graph.pow(one_minus, graph.integer(k / 2));
                return if k % 2 == 0 {
                    squares
                } else {
                    graph.mul([base, squares])
                };
            }
            graph.pow(
                sin_squared_to_cos(graph, base),
                sin_squared_to_cos(graph, exponent),
            )
        }
        _ => expr,
    }
}

/// Simplify a trigonometric expression: the plain simplifier, the fully expanded form, and the Pythagorean
/// reduction of the expanded form; the shortest wins.
pub fn trigsimp(graph: &ExprGraph, expr: ExprId) -> ExprId {
    use crate::simplify::{AlgebraicTransformations, Simplifier};
    use algebra_core::format::{Formatter, UnicodeFormatter};
    let simplify = |e: ExprId| Simplifier::simplify(graph, e).unwrap_or(e);
    let expanded = AlgebraicTransformations::expand(graph, expand_trig(graph, expr));
    let reduced = AlgebraicTransformations::expand(graph, sin_squared_to_cos(graph, expanded));
    let candidates = [
        simplify(expr),
        simplify(expanded),
        simplify(reduced),
        simplify(simplify(reduced)),
    ];
    let size = |e: ExprId| {
        UnicodeFormatter
            .format(graph, e)
            .map(|s| s.chars().count())
            .unwrap_or(usize::MAX)
    };
    candidates
        .into_iter()
        .min_by_key(|&e| size(e))
        .unwrap_or(expr)
}
