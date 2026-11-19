//! Solving a single inequality in one real variable: `x^2 > 4`, `abs(x) < 3`, `(x - 1)/(x + 2) >= 0`.
//!
//! The critical points are the real zeros of `f = lhs - rhs` and of its denominator; between consecutive points the
//! sign of `f` is constant, so one test value per interval decides it. The answer is a union of intervals.

use crate::numeric::{EvalContext, NumericalEval};
use crate::simplify::{AlgebraicTransformations, SymbolicSimplifier};
use crate::solver::SymbolicSolver;
use algebra_core::format::{Formatter, UnicodeFormatter};
use algebra_core::{ExprGraph, ExprId, ExprKind, RelOp, SymbolId};
use std::collections::HashMap;

#[derive(Clone)]
struct Point {
    value: f64,
    text: String,
}

fn evaluate(graph: &ExprGraph, expr: ExprId, var: &str, at: f64) -> Option<f64> {
    let mut bindings = HashMap::new();
    bindings.insert(var.to_string(), at);
    let ctx = EvalContext::with_bindings(50, bindings);
    match graph.evalf(expr, &ctx).ok()? {
        // A complex value means the expression is not real there (outside the domain).
        crate::numeric::BigValue::Real(r) if r.is_finite() => Some(r),
        _ => None,
    }
}

fn has_periodic(graph: &ExprGraph, expr: ExprId) -> bool {
    match graph.get(expr).kind.clone() {
        ExprKind::Function { name, args } => {
            matches!(
                graph.symbols.resolve(name).as_deref(),
                Some("sin" | "cos" | "tan" | "cot" | "sec" | "csc")
            ) || args.iter().any(|&a| has_periodic(graph, a))
        }
        ExprKind::Add(ts) | ExprKind::Mul(ts) => ts.iter().any(|&t| has_periodic(graph, t)),
        ExprKind::Sub(a, b) | ExprKind::Div(a, b) | ExprKind::Pow(a, b) => {
            has_periodic(graph, a) || has_periodic(graph, b)
        }
        ExprKind::Neg(a) => has_periodic(graph, a),
        _ => false,
    }
}

fn real_roots(graph: &ExprGraph, expr: ExprId, var: SymbolId, name: &str, out: &mut Vec<Point>) {
    let Ok(roots) = graph.solveset(expr, var) else {
        return;
    };
    let fmt = UnicodeFormatter;
    for root in roots {
        let root = graph.simplify(root);
        if crate::complex_exact::mentions_i(graph, root) {
            continue;
        }
        if let Some(value) = evaluate(graph, root, name, 0.0) {
            // Real roots only: an imaginary root does not evaluate to a finite real number.
            let text = fmt.format(graph, root).unwrap_or_default();
            out.push(Point { value, text });
        }
    }
}

/// The arguments of the functions that restrict the domain (`sqrt`, `ln`, `log`), whose zeros bound it.
fn domain_arguments(graph: &ExprGraph, expr: ExprId, out: &mut Vec<ExprId>) {
    match graph.get(expr).kind.clone() {
        ExprKind::Function { name, args } => {
            if matches!(
                graph.symbols.resolve(name).as_deref(),
                Some("sqrt" | "ln" | "log")
            ) {
                out.push(args[0]);
            }
            args.iter().for_each(|&a| domain_arguments(graph, a, out));
        }
        ExprKind::Add(ts) | ExprKind::Mul(ts) => {
            ts.iter().for_each(|&t| domain_arguments(graph, t, out))
        }
        ExprKind::Sub(a, b) | ExprKind::Div(a, b) => {
            domain_arguments(graph, a, out);
            domain_arguments(graph, b, out);
        }
        ExprKind::Pow(a, b) => {
            // A rational power with an even denominator needs a non-negative base.
            if let ExprKind::Number(algebra_core::Number::Rational(_, d)) = graph.get(b).kind
                && d % 2 == 0
            {
                out.push(a);
            }
            domain_arguments(graph, a, out);
            domain_arguments(graph, b, out);
        }
        ExprKind::Neg(a) => domain_arguments(graph, a, out),
        _ => {}
    }
}

/// Solve `relation` (a `Relational` node with `<`, `<=`, `>` or `>=`) for `var`; the answer as a union of intervals.
pub fn solve(graph: &ExprGraph, relation: ExprId, var: &str) -> Result<String, String> {
    let ExprKind::Relational { op, lhs, rhs } = graph.get(relation).kind.clone() else {
        return Err("not a relation".into());
    };
    let var_sym = graph.symbols.get_or_intern(var);
    let f = graph.simplify(graph.sub(lhs, rhs));
    if has_periodic(graph, f) {
        return Err(
            "inequalities with periodic functions have infinitely many intervals; not supported"
                .into(),
        );
    }
    let together = AlgebraicTransformations::together(graph, f);
    let (numerator, denominator) = match graph.get(together).kind.clone() {
        ExprKind::Div(n, d) => (n, Some(d)),
        _ => (together, None),
    };
    let mut points: Vec<Point> = Vec::new();
    real_roots(graph, numerator, var_sym, var, &mut points);
    if let Some(d) = denominator {
        real_roots(graph, d, var_sym, var, &mut points);
    }
    let mut restricted = Vec::new();
    domain_arguments(graph, f, &mut restricted);
    for u in restricted {
        real_roots(graph, u, var_sym, var, &mut points);
    }
    points.sort_by(|a, b| a.value.total_cmp(&b.value));
    points.dedup_by(|a, b| (a.value - b.value).abs() < 1e-12);

    let strict = matches!(op, RelOp::LessThan | RelOp::GreaterThan);
    let wants_positive = matches!(op, RelOp::GreaterThan | RelOp::GreaterEqual);
    let tolerance = 1e-9;
    let holds = |value: f64| match (wants_positive, strict) {
        (true, true) => value > tolerance,
        (true, false) => value > -tolerance,
        (false, true) => value < -tolerance,
        (false, false) => value < tolerance,
    };

    // Intervals and points alternate: I0, P0, I1, P1, ..., In. Undefined values (outside the domain, poles) fail.
    let n = points.len();
    let mut state: Vec<bool> = Vec::with_capacity(2 * n + 1);
    for k in 0..=n {
        let test = match (k == 0, k == n) {
            (true, true) => 0.0,
            (true, false) => points[0].value - 1.0,
            (false, true) => points[n - 1].value + 1.0,
            _ => (points[k - 1].value + points[k].value) / 2.0,
        };
        state.push(evaluate(graph, f, var, test).is_some_and(holds));
        if k < n {
            state.push(evaluate(graph, f, var, points[k].value).is_some_and(holds));
        }
    }
    // Maximal runs of satisfied elements.
    let mut parts = Vec::new();
    let mut s = 0;
    while s < state.len() {
        if !state[s] {
            s += 1;
            continue;
        }
        let mut e = s;
        while e + 1 < state.len() && state[e + 1] {
            e += 1;
        }
        if s == e && s % 2 == 1 {
            parts.push(format!("{{{}}}", points[(s - 1) / 2].text));
        } else {
            let (lo, lo_closed) = if s % 2 == 1 {
                (points[(s - 1) / 2].text.clone(), true)
            } else if s == 0 {
                ("-∞".to_string(), false)
            } else {
                (points[s / 2 - 1].text.clone(), false)
            };
            let (hi, hi_closed) = if e % 2 == 1 {
                (points[(e - 1) / 2].text.clone(), true)
            } else if e / 2 == n {
                ("∞".to_string(), false)
            } else {
                (points[e / 2].text.clone(), false)
            };
            parts.push(format!(
                "{}{lo}, {hi}{}",
                if lo_closed { "[" } else { "(" },
                if hi_closed { "]" } else { ")" }
            ));
        }
        s = e + 1;
    }
    if parts.is_empty() {
        return Ok("no real solutions (the empty set)".into());
    }
    Ok(parts.join(" ∪ "))
}
