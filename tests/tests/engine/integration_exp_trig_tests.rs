//! Integrals of polynomials times exp/sin/cos, of exp times sin/cos, of trigonometric products and of `abs`, each
//! checked by differentiating the result and comparing with the integrand at several points. The check does not use
//! the integrator's own rules (the derivative comes from the differentiator, the values from numerical
//! evaluation).

use algebra_core::parser::ExprParser;
use algebra_core::{ExprGraph, ExprKind};
use algebra_engine::SymbolicCalculus;
use algebra_engine::numeric::EvalContext;
use algebra_engine::numeric::NumericalEval;
use std::collections::HashMap;

fn value(graph: &ExprGraph, id: algebra_core::ExprId, x: f64) -> f64 {
    let ctx = EvalContext::with_bindings(50, HashMap::from([("x".to_string(), x)]));
    graph.evalf(id, &ctx).unwrap().to_f64()
}

fn check(integrand: &str, points: &[f64]) {
    let graph = ExprGraph::new();
    let f = ExprParser::new(&graph).parse(integrand).unwrap();
    let x = graph.symbol("x");
    let ExprKind::Symbol(sym) = graph.get(x).kind else {
        unreachable!()
    };
    let antiderivative = graph
        .integrate(f, sym)
        .unwrap_or_else(|e| panic!("integrate {integrand}: {e}"));
    let derivative = graph.diff(antiderivative, sym);
    for &p in points {
        let (got, want) = (value(&graph, derivative, p), value(&graph, f, p));
        assert!(
            (got - want).abs() <= 1e-9 * (1.0 + want.abs()),
            "d/dx of the integral of {integrand} at {p}: {got} vs {want}"
        );
    }
}

const POINTS: &[f64] = &[0.3, 0.7, 1.1, 1.9, -0.6];

#[test]
fn polynomials_times_exponentials() {
    for f in [
        "x*exp(2*x)",
        "x*exp(-x)",
        "x^2*exp(3*x)",
        "x^3*exp(x)",
        "(x^2+1)*exp(-2*x)",
        "x^4*exp(x/2)",
        "(3*x-1)*exp(x+1)",
    ] {
        check(f, POINTS);
    }
}

#[test]
fn polynomials_times_trigonometric_functions() {
    for f in [
        "x*sin(2*x)",
        "x*cos(3*x)",
        "x^2*sin(x/2)",
        "(x^2+x)*cos(2*x+1)",
        "x^3*sin(x)",
        "x^5*cos(x)",
        "(x-4)*sin(-x)",
    ] {
        check(f, POINTS);
    }
}

#[test]
fn exponential_times_trigonometric_functions() {
    for f in [
        "exp(x)*sin(x)",
        "exp(-x)*cos(x)",
        "exp(2*x)*sin(3*x)",
        "exp(x/2)*cos(2*x+1)",
        "sin(x)*exp(-3*x)",
        "cos(4*x)*exp(x+2)",
    ] {
        check(f, POINTS);
    }
}

#[test]
fn products_of_trigonometric_functions() {
    for f in [
        "sin(x)*cos(2*x)",
        "cos(3*x)*sin(x)",
        "sin(2*x)*sin(5*x)",
        "cos(x)*cos(4*x)",
        "sin(x+1)*cos(3*x-2)",
        "sin(2*x)*sin(2*x+1)",
    ] {
        check(f, POINTS);
    }
}

#[test]
fn absolute_value_of_a_linear_form() {
    // abs is not differentiable at 0, so stay away from the kink.
    for f in ["abs(x)", "abs(2*x-1)", "abs(3-x)"] {
        check(f, &[0.3, 0.7, 1.9, -0.6, 2.4]);
    }
}

// ---- quadratic radicals, by parts, trigonometric powers (risch_more) ----

const ANY: &[f64] = &[0.3, 0.7, 1.1, 1.9, -0.6];
const POSITIVE: &[f64] = &[0.4, 0.9, 1.6, 2.5, 3.7];
const OUTSIDE_UNIT: &[f64] = &[1.4, 2.2, 3.1, 4.5];
const INSIDE_UNIT: &[f64] = &[0.15, 0.4, -0.3, 0.7, -0.8];

#[test]
fn integrals_involving_the_square_root_of_a_quadratic() {
    for f in [
        "sqrt(x^2+1)",
        "sqrt(x^2+4*x+5)",
        "1/sqrt(x^2+2*x+5)",
        "x^2/sqrt(x^2+1)",
        "x^3*sqrt(x^2+3)",
        "(x+1)*sqrt(2*x^2+x+4)",
        "x^2/sqrt(x^2+x+1)",
        "1/sqrt(3*x^2+1)",
    ] {
        check(f, ANY);
    }
    for f in ["sqrt(x^2-1)", "x^2/sqrt(x^2-4)", "x*sqrt(x^2-1)"] {
        check(f, OUTSIDE_UNIT);
    }
    for f in [
        "sqrt(1-x^2)",
        "sqrt(4-x^2)",
        "x^2/sqrt(1-x^2)",
        "x^2*sqrt(1-x^2)",
        "1/sqrt(2-x^2-x)",
    ] {
        check(f, INSIDE_UNIT);
    }
}

#[test]
fn polynomials_times_inverse_trigonometric_functions_and_powers_of_ln() {
    for f in [
        "x*atan(x)",
        "x^2*atan(x)",
        "(x^2+1)*atan(x)",
        "3*x^3*atan(x)",
    ] {
        check(f, ANY);
    }
    for f in ["x*asin(x)", "x^2*asin(x)", "x*acos(x)", "x^3*acos(x)"] {
        check(f, INSIDE_UNIT);
    }
    for f in [
        "x*ln(x)^2",
        "x^2*ln(x)^3",
        "ln(x)^4",
        "(x+1)*ln(x)^2",
        "x^3*ln(x)^2",
    ] {
        check(f, POSITIVE);
    }
}

#[test]
fn powers_of_sine_and_cosine_and_their_reciprocals() {
    for f in [
        "sin(x)^2",
        "sin(x)^4",
        "cos(x)^3",
        "cos(x)^6",
        "sin(2*x)^3",
        "cos(3*x+1)^5",
        "sin(x)^7",
    ] {
        check(f, ANY);
    }
    for f in ["1/cos(x)", "1/sin(x)", "1/cos(2*x)", "1/sin(3*x+1)"] {
        check(f, &[0.3, 0.7, 1.0]);
    }
}

// ---- rational functions: partial fractions over Q (rational_pf) ----

#[test]
fn repeated_irreducible_quadratic_denominators_do_not_overflow_the_stack() {
    // These aborted the whole process with a stack overflow before.
    for f in [
        "1/(x^2+1)^2",
        "1/(x^2+1)^3",
        "x/(x^2+1)^2",
        "1/(x^2+2*x+2)^2",
        "(3*x+2)/(x^2+x+1)^2",
        "1/((x^2+1)^2*(x-1))",
        "x^3/(x^2+4)^3",
        "1/((x^2+1)*(x^2+4)^2)",
        "(x^4+1)/(x^2+x+1)^3",
    ] {
        check(f, ANY);
    }
}

#[test]
fn rational_functions_with_mixed_factors() {
    for f in [
        "1/(x^4+1)",
        "x^2/(x^4+1)",
        "(x^3+2*x+1)/(x^4+1)",
        "1/(x^2*(x+1))",
        "1/((x-1)^2*(x+2))",
        "x^5/(x^2+1)",
        "(x^2+1)/(x^3-x)",
        "1/(x^4+5*x^2+4)",
        "(2*x^3-x+7)/((x+3)^2*(x^2+x+1))",
    ] {
        // Stay clear of the poles at 0, 1, -2 and -3.
        check(f, &[0.3, 0.7, 1.9, 2.6, 4.2]);
    }
}
