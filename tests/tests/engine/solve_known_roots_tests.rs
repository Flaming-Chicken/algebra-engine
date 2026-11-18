//! `solveset` against equations with known real roots (polynomials with repeated, rational and
//! irrational roots, extraneous-root radicals, absolute values, removable singularities, and
//! equations with no solution). Complex roots are covered elsewhere.

use algebra_core::ExprGraph;
use algebra_core::parser::ExprParser;
use algebra_engine::numeric::{EvalContext, NumericalEval};
use algebra_engine::solver::SymbolicSolver;
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
fn solveset_finds_exactly_the_real_roots_of_known_equations() {
    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let x_sym = graph.symbols.get_or_intern("x");
    // (equation, expected sorted real roots)
    let cases: Vec<(&str, Vec<f64>)> = vec![
        ("x^2-5*x+6", vec![2.0, 3.0]),
        ("(x-1)^2*(x+2)", vec![-2.0, 1.0]),
        ("x^3-2", vec![2.0_f64.cbrt()]),
        ("x^3-6*x^2+11*x-6", vec![1.0, 2.0, 3.0]),
        ("x^4-5*x^2+4", vec![-2.0, -1.0, 1.0, 2.0]),
        ("x^5-x", vec![-1.0, 0.0, 1.0]),
        ("2*x^2-3*x-2", vec![-0.5, 2.0]),
        ("x^2-2", vec![-(2.0_f64.sqrt()), 2.0_f64.sqrt()]),
        ("sqrt(x+2)-x", vec![2.0]),
        ("sqrt(2*x+3)-x", vec![3.0]),
        ("abs(x-1)-2", vec![-1.0, 3.0]),
        ("exp(x)-3", vec![3.0_f64.ln()]),
        ("ln(x)-1", vec![1.0_f64.exp()]),
        ("(x^2-1)/(x-1)", vec![-1.0]),
        ("x/(x-2)-1/(x-2)-1", vec![]),
        ("1/x-2", vec![0.5]),
        ("x^2*(x-3)", vec![0.0, 3.0]),
        ("x^3-x^2-x+1", vec![-1.0, 1.0]),
        ("(x+1)*(x-2)*(x-5)*(x+7)", vec![-7.0, -1.0, 2.0, 5.0]),
        (
            "x^2-x-1",
            vec![(1.0 - 5.0_f64.sqrt()) / 2.0, (1.0 + 5.0_f64.sqrt()) / 2.0],
        ),
        ("0*x+3", vec![]),
        ("0*x", vec![]),
    ];
    for (eq, want) in cases {
        let e = parser.parse(eq).unwrap();
        match graph.solveset(e, x_sym) {
            Ok(roots) => {
                let mut got: Vec<Option<f64>> =
                    roots.iter().map(|&r| val(&graph, r, 0.0)).collect();
                got.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let residual_ok = got
                    .iter()
                    .all(|g| g.is_some_and(|g| val(&graph, e, g).is_some_and(|r| r.abs() < 1e-7)));
                let mut gv: Vec<f64> = got.iter().flatten().copied().collect();
                gv.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
                let same = gv.len() == want.len()
                    && gv.iter().zip(&want).all(|(a, b)| (a - b).abs() < 1e-7);
                assert!(
                    same && residual_ok,
                    "solve {eq}: got {got:?}, want {want:?}"
                );
            }
            Err(err) => panic!("solve {eq} failed: {err:?}"),
        }
    }
}
