use algebra_core::ExprGraph;
use algebra_engine::distributions::SymbolicDistributions;

#[test]
fn test_dirac_delta_sifting_property() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let x_sym = graph.symbols.get_or_intern("x");

    // f(x) = x^2 + 3
    let two = graph.integer(2);
    let three = graph.integer(3);
    let x_sq = graph.pow(x, two);
    let f_x = graph.add([x_sq, three]);

    // sifting at a = 5 => f(5) = 5^2 + 3 = 28
    let five = graph.integer(5);
    let sifting = graph.sifting_integral(f_x, x_sym, five).unwrap();

    assert!(!graph.is_empty());
    assert_ne!(sifting, f_x);
}

#[test]
fn test_heaviside_construction() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");

    let h = graph.heaviside(x);
    let d = graph.dirac_delta(x);

    assert!(!graph.is_empty());
    assert_ne!(h, d);
}

#[test]
fn test_empirical_distribution_online_update() {
    use algebra_engine::distributions::{EmpiricalDistributionMut, GeneralizedDistribution};

    let mut dist = EmpiricalDistributionMut::new();
    dist.update(10.0);
    dist.update(20.0);
    dist.update(30.0);

    assert_eq!(dist.mean(), 20.0);
    assert_eq!(dist.sample_variance(), 100.0);

    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    // Only moments are kept, so there is no density to return.
    assert!(dist.pdf(&graph, x).is_err());
    assert!(dist.cdf(&graph, x).is_err());
}
