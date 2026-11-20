//! Every solution the solver returns must satisfy the equation: each real root is substituted back and the residual
//! evaluated numerically (an oracle independent of the solving method).

use algebra_core::parser::ExprParser;
use algebra_core::{ExprGraph, ExprKind};
use algebra_engine::SymbolicSolver;
use algebra_engine::numeric::{BigValue, EvalContext, NumericalEval};
use std::collections::HashMap;

/// Solve `equation = 0` for `x` and return (number of real roots checked, number of roots returned).
fn check_residuals(equation: &str) -> (usize, usize) {
    let graph = ExprGraph::new();
    let eq = ExprParser::new(&graph).parse(equation).unwrap();
    let x = graph.symbol("x");
    let ExprKind::Symbol(sym) = graph.get(x).kind else {
        unreachable!()
    };
    let roots = graph
        .solveset(eq, sym)
        .unwrap_or_else(|e| panic!("{equation}: {e}"));
    let mut checked = 0;
    for root in &roots {
        // The root's own value (it may be an exact expression such as pi/3 or 2^(1/3)).
        let Ok(value) = graph.evalf(*root, &EvalContext::default()) else {
            continue;
        };
        let BigValue::Real(v) = value else { continue }; // complex roots are not checked here
        let ctx = EvalContext::with_bindings(50, HashMap::from([("x".to_string(), v)]));
        let residual = graph.evalf(eq, &ctx).unwrap().to_f64();
        assert!(
            residual.abs() < 1e-8 * (1.0 + v.abs().powi(3)),
            "{equation}: x = {v} leaves the residual {residual}"
        );
        checked += 1;
    }
    (checked, roots.len())
}

#[test]
fn polynomial_and_radical_equations() {
    for (eq, real_roots) in [
        ("x^2 - 5*x + 6", 2),
        ("x^3 - 6*x^2 + 11*x - 6", 3),
        ("x^4 - 5*x^2 + 4", 4),
        ("x^3 - 2", 1),
        ("x^2 - 2", 2),
        ("sqrt(x) - 3", 1),
        ("sqrt(x+1) - x + 1", 1),
        ("1/(x-1) + 1/(x+1) - 1", 2),
        ("x/(x-1) - 3", 1),
        ("abs(x) - 2", 2),
        // Roots with no closed form (`RootOf`) are evaluated numerically: a quintic with one real root, a quartic with four,
        // a cubic with three.
        ("x^5 - x - 1", 1),
        ("x^4 - 3*x^2 + x + 1", 4),
        ("x^3 - 3*x + 1", 3),
    ] {
        let (checked, _) = check_residuals(eq);
        assert_eq!(checked, real_roots, "{eq}");
    }
}

#[test]
fn transcendental_equations_including_polynomials_in_one_function() {
    for (eq, real_roots) in [
        ("exp(x) - 5", 1),
        ("ln(x) - 2", 1),
        ("2^x - 8", 1),
        ("ln(x) + ln(x-1) - ln(2)", 1),
        ("exp(2*x) - 3*exp(x) + 2", 2),
        // New: polynomials in a single function of the unknown.
        ("cos(x)^2 - 1/4", 4),
        ("tan(x)^2 - 3", 2),
        ("ln(x)^2 - 3*ln(x) + 2", 2),
        ("sin(x)^2 - sin(x)/2", 3),
        ("cos(2*x)^2 - 1/2", 4),
    ] {
        let (checked, _) = check_residuals(eq);
        assert!(
            checked >= real_roots,
            "{eq}: only {checked} real roots checked"
        );
    }
}
