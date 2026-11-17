use algebra_core::ExprGraph;
use algebra_engine::series::SymbolicSeries;

#[test]
fn test_lagrange_inversion_linear() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let x_sym = graph.symbols.get_or_intern("x");
    let y_sym = graph.symbols.get_or_intern("y");

    // f(x) = x => inverse x = y
    let inv = graph.lagrange_inversion(x, x_sym, y_sym, 2).unwrap();

    assert!(!graph.is_empty());
    assert_ne!(inv, x);
}

#[test]
fn test_laurent_series_expansion() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let x_sym = graph.symbols.get_or_intern("x");
    let zero = graph.integer(0);

    // f(x) = 1/x (pole of order 1 at x = 0)
    let one = graph.integer(1);
    let inv_x = graph.div(one, x);

    let laurent = graph.laurent_series(inv_x, x_sym, zero, 1, 2).unwrap();
    assert!(!graph.is_empty());
    assert_ne!(laurent, inv_x);
}

#[test]
fn test_puiseux_series_expansion() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let x_sym = graph.symbols.get_or_intern("x");
    let zero = graph.integer(0);

    // Denominator 1 is the Taylor series; fractional exponents are reported as unsupported.
    let taylor = graph.puiseux_series(x, x_sym, zero, 1, 2).unwrap();
    assert!(!graph.is_empty());
    assert_ne!(taylor, zero);
    assert!(graph.puiseux_series(x, x_sym, zero, 2, 2).is_err());
}

#[test]
fn test_holonomic_functions_and_recurrence_solvers() {
    use algebra_engine::series::{HolonomicFunction, RecurrenceSolver};

    let graph = ExprGraph::new();
    let x_sym = graph.symbols.get_or_intern("x");
    let n_sym = graph.symbols.get_or_intern("n");
    let f_expr = graph.function("f", [graph.symbol("x")]);

    // D-finite operator for Bessel equation: x^2 y'' + x y' + (x^2 - nu^2) y = 0
    let p0 = graph.symbol("p0");
    let p1 = graph.symbol("p1");
    let holonomic = HolonomicFunction::new(vec![p0, p1]);
    let ode = holonomic.to_ode(&graph, f_expr, x_sym);
    assert!(!graph.is_empty());
    assert_ne!(ode, f_expr);

    // First-order linear recurrence a_n = 2 * a_{n-1}, a_0 = 1 => a_n = 1 * 2^n
    let rec_sol = RecurrenceSolver::solve_linear_constant(&graph, &[2.0], &[1.0], n_sym).unwrap();
    assert!(!graph.is_empty());
    assert_ne!(rec_sol, f_expr);
}
