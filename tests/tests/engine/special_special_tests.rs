use algebra_core::ExprGraph;
use algebra_core::format::{Formatter, UnicodeFormatter};
use algebra_engine::special::SpecialFunctions;

#[test]
fn test_special_function_construction() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let fmt = UnicodeFormatter;

    let g = graph.gamma(x);
    let z = graph.zeta(x);
    let d = graph.dirac_delta(x);
    let e = graph.erf(x);

    assert_ne!(g, z);
    assert_ne!(d, e);
    assert_eq!(fmt.format(&graph, g).unwrap(), "gamma(x)");
    assert_eq!(fmt.format(&graph, z).unwrap(), "zeta(x)");
    assert_eq!(fmt.format(&graph, e).unwrap(), "erf(x)");
}

#[test]
fn test_gamma_exact_evaluation() {
    let graph = ExprGraph::new();
    let five = graph.integer(5);

    let g5 = graph.gamma(five);
    let eval = graph.evaluate_special(g5).unwrap();

    // Gamma(5) = 4! = 24
    assert_eq!(eval, graph.integer(24));
}

#[test]
fn test_zeta_2_evaluation() {
    let graph = ExprGraph::new();
    let two = graph.integer(2);

    let z2 = graph.zeta(two);
    let eval = graph.evaluate_special(z2).unwrap();

    // zeta(2) = pi^2 / 6
    let fmt = UnicodeFormatter;
    assert_eq!(fmt.format(&graph, eval).unwrap(), "π^2 / 6");
}
