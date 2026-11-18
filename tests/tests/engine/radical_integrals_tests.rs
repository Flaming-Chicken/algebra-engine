//! Integrals of a polynomial times a non-integer power of a linear form (`x * sqrt(x + 1)`).

use algebra_core::ExprGraph;
use algebra_core::parser::ExprParser;
use algebra_engine::calculus::SymbolicCalculus;
use algebra_engine::numeric::{EvalContext, NumericalEval};
use std::collections::HashMap;

fn val(graph: &ExprGraph, e: algebra_core::ExprId, x: f64) -> Option<f64> {
    let mut b = HashMap::new();
    b.insert("x".to_string(), x);
    graph
        .evalf(e, &EvalContext::with_bindings(15, b))
        .ok()
        .map(|v| v.to_f64())
}

#[test]
fn polynomial_times_a_power_of_a_linear_form_integrates_exactly() {
    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let x = graph.symbols.get_or_intern("x");
    // Every one of these must integrate, and the antiderivative must differentiate back.
    for f in [
        "x*sqrt(x+1)",
        "sqrt(2*x+3)",
        "x^2*(x+1)^(3/2)",
        "1/sqrt(x+1)",
        "x/sqrt(x+1)",
        "x^2*sqrt(3-x)",
        "(x+1)^(1/3)",
        "x*(2*x+1)^(1/3)",
        "x*sqrt(x)",
        "(x-1)/sqrt(x+2)",
        "1/(x+1)^(3/2)",
        "sqrt(x+1)*(x+2)",
        "(3*x^3-x+2)*sqrt(2*x+5)",
        "(x^2+1)/(x-4)^(1/3)",
    ] {
        let e = parser.parse(f).expect("parse");
        let i = graph
            .integrate(e, x)
            .unwrap_or_else(|err| panic!("{f} did not integrate: {err:?}"));
        let d = graph.diff(i, x);
        for p in [4.7, 5.3, 6.1, 9.9] {
            let (a, b) = (
                val(&graph, d, p).expect("derivative"),
                val(&graph, e, p).expect("integrand"),
            );
            assert!(
                (a - b).abs() < 1e-9 * (1.0 + b.abs()),
                "{f} at {p}: {a} vs {b}"
            );
        }
    }
    // Outside this family the answer is a refusal or a correct antiderivative, never a wrong one.
    for f in ["x*sqrt(x^2+1)", "sqrt(x+1)*sqrt(x+2)", "sqrt(x+1)/x"] {
        let e = parser.parse(f).expect("parse");
        if let Ok(i) = graph.integrate(e, x) {
            let d = graph.diff(i, x);
            for p in [4.7, 5.3, 6.1] {
                let (a, b) = (
                    val(&graph, d, p).expect("derivative"),
                    val(&graph, e, p).expect("integrand"),
                );
                assert!(
                    (a - b).abs() < 1e-9 * (1.0 + b.abs()),
                    "{f} at {p}: {a} vs {b}"
                );
            }
        }
    }
}
