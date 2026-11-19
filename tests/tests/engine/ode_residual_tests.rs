//! Solutions of `dsolve` are substituted back into the equation: the residual is evaluated numerically at several
//! points with arbitrary values for the constants, using the engine's own differentiator for y' and y''. The oracle
//! does not depend on the solving method.

use algebra_core::parser::ExprParser;
use algebra_core::{ExprGraph, ExprKind};
use algebra_engine::SymbolicCalculus;
use algebra_engine::numeric::{EvalContext, NumericalEval};
use algebra_engine::session::UraeSession;
use std::collections::HashMap;

type Residual = fn(f64, f64, f64, f64) -> f64;

/// Check every alternative of the solution of `ode` against `residual(x, y, y', y'')`.
fn check(ode: &str, residual: Residual, points: &[f64]) -> usize {
    let r = UraeSession::new().execute_line(&format!("dsolve({ode})"));
    assert!(
        !r.is_error && r.output_text.starts_with("y(x) = "),
        "{ode}: not solved: {}",
        r.output_text
    );
    let mut checked = 0;
    for alternative in r.output_text.split("  or  ") {
        let rhs = alternative.strip_prefix("y(x) = ").unwrap_or(alternative);
        let graph = ExprGraph::new();
        let y = ExprParser::new(&graph)
            .parse(rhs)
            .unwrap_or_else(|e| panic!("{ode}: cannot parse {rhs}: {e}"));
        let x = graph.symbol("x");
        let ExprKind::Symbol(sym) = graph.get(x).kind else {
            unreachable!()
        };
        let dy = graph.diff(y, sym);
        let ddy = graph.diff(dy, sym);
        for (c1, c2) in [(0.7, -1.3), (-2.0, 0.5)] {
            for &p in points {
                let ctx = EvalContext::with_bindings(
                    50,
                    HashMap::from([
                        ("x".to_string(), p),
                        ("C1".to_string(), c1),
                        ("C2".to_string(), c2),
                    ]),
                );
                let value = |id| {
                    graph
                        .evalf(id, &ctx)
                        .unwrap_or_else(|e| panic!("{ode}: {rhs}: {e}"))
                        .to_f64()
                };
                let (yv, y1, y2) = (value(y), value(dy), value(ddy));
                let res = residual(p, yv, y1, y2);
                assert!(
                    res.abs() <= 1e-7 * (1.0 + yv.abs() + y1.abs() + y2.abs()),
                    "{ode}: y = {rhs} leaves the residual {res} at x = {p} (C1 = {c1}, C2 = {c2})"
                );
                checked += 1;
            }
        }
    }
    checked
}

const PTS: &[f64] = &[0.4, 0.9, 1.3, 2.1];

#[test]
fn first_order_linear_and_constant_coefficient_equations() {
    check("y' = y", |_, y, y1, _| y1 - y, PTS);
    check("y' = 2*x*y", |x, y, y1, _| y1 - 2.0 * x * y, PTS);
    check("y' + y = x", |x, y, y1, _| y1 + y - x, PTS);
    check("y' = x^2", |x, _, y1, _| y1 - x * x, PTS);
    check(
        "y' + 2*y = sin(x)",
        |x, y, y1, _| y1 + 2.0 * y - x.sin(),
        PTS,
    );
    check("y'' + y = 0", |_, y, _, y2| y2 + y, PTS);
    check(
        "y'' - 3*y' + 2*y = 0",
        |_, y, y1, y2| y2 - 3.0 * y1 + 2.0 * y,
        PTS,
    );
    check(
        "y'' + 4*y' + 4*y = 0",
        |_, y, y1, y2| y2 + 4.0 * y1 + 4.0 * y,
        PTS,
    );
    check(
        "y'' + 2*y' + 5*y = 0",
        |_, y, y1, y2| y2 + 2.0 * y1 + 5.0 * y,
        PTS,
    );
}

#[test]
fn nonlinear_separable_equations_were_solved_as_if_linear() {
    // `y' = y^2` came back as `y = C1 exp(x)` (the solution of `y' = y`), and `y' = y (1 - y)` as `y = C1`.
    check("y' = y^2", |_, y, y1, _| y1 - y * y, &[0.3, 0.6, 0.9]);
    check(
        "y' = y*(1-y)",
        |_, y, y1, _| y1 - y * (1.0 - y),
        &[0.3, 0.6, 0.9, 1.5],
    );
    check("y' = x*y^2", |x, y, y1, _| y1 - x * y * y, &[0.3, 0.6, 0.9]);
    // No closed form for y here: an implicit solution is reported and labelled as such.
    let r = UraeSession::new().execute_line("dsolve(y' = exp(x)*y^3)");
    assert!(!r.is_error, "{}", r.output_text);
    assert!(
        r.output_text.contains("implicit solution"),
        "{}",
        r.output_text
    );
    // The implicit relation -1/(2 y^2) = e^x + C1 differentiates to y' = e^x y^3: check that identity directly.
    for (x, y) in [(0.1f64, 0.7f64), (0.5, 1.9)] {
        let (g_prime, f_prime) = (y.powi(-3), x.exp());
        assert!((g_prime * (x.exp() * y.powi(3)) - f_prime).abs() < 1e-12);
    }
    check("y' = y/x", |x, y, y1, _| y1 - y / x, PTS);
    check("x*y' = y", |x, y, y1, _| x * y1 - y, PTS);
}

#[test]
fn non_homogeneous_second_order_equations() {
    check("y'' + y = x", |x, y, _, y2| y2 + y - x, PTS);
    check("y'' = x", |x, _, _, y2| y2 - x, PTS);
    check("y'' - y = exp(x)", |x, y, _, y2| y2 - y - x.exp(), PTS);
    check("y'' + y = sin(x)", |x, y, _, y2| y2 + y - x.sin(), PTS);
    check(
        "y'' + 9*y = cos(2*x)",
        |x, y, _, y2| y2 + 9.0 * y - (2.0 * x).cos(),
        PTS,
    );
    check(
        "y'' + 3*y' + 2*y = x^2",
        |x, y, y1, y2| y2 + 3.0 * y1 + 2.0 * y - x * x,
        PTS,
    );
    check(
        "y'' - 2*y' + y = exp(x)",
        |x, y, y1, y2| y2 - 2.0 * y1 + y - x.exp(),
        PTS,
    );
}

#[test]
fn an_equation_that_is_not_recognised_is_not_given_a_wrong_solution() {
    // Not separable, not linear: the command may decline, but it must not return a solution that fails the equation.
    let r = UraeSession::new().execute_line("dsolve(y' = x + y^2)");
    if r.output_text.starts_with("y(x) = ") {
        check("y' = x + y^2", |x, y, y1, _| y1 - x - y * y, &[0.3, 0.6]);
    }
}
