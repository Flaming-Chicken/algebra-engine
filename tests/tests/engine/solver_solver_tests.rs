use algebra_core::ExprGraph;
use algebra_engine::solver::SymbolicSolver;

#[test]
fn test_solveset_linear() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let x_sym = graph.symbols.get_or_intern("x");
    let five = graph.integer(5);

    // x + 5 = 0 => x = -5
    let eq = graph.add([x, five]);
    let roots = graph.solveset(eq, x_sym).unwrap();

    assert_eq!(roots.len(), 1);
    assert_eq!(graph.get(roots[0]).kind, graph.get(graph.integer(-5)).kind);
}

#[test]
fn test_solve_ode_linear() {
    use algebra_engine::numeric::{EvalContext, NumericalEval};
    use std::collections::HashMap;

    let graph = ExprGraph::new();
    let x_sym = graph.symbols.get_or_intern("x");

    // Non-zero constant coefficient: y' + 1 * y = 1 => particular solution y_p(x) = 1
    let zero = graph.integer(0);
    let one = graph.integer(1);
    let sol_nonzero = graph.solve_ode_linear(one, one, x_sym).unwrap();
    let mut bindings = HashMap::new();
    bindings.insert("x".to_string(), 4.5);
    let ctx = EvalContext::with_bindings(53, bindings);
    let val = graph.evalf(sol_nonzero, &ctx).unwrap().to_f64();
    assert!(
        (val - 1.0).abs() < 1e-12,
        "expected y_p(4.5)=1.0, got {val}"
    );

    // p = 0 (y' = 1): the integrating factor exp(0) used to survive unreduced and the result failed to
    // evaluate with a division by zero (audit B4, fixed). The particular solution is y = x.
    let sol_zero_p = graph.solve_ode_linear(zero, one, x_sym).unwrap();
    let value = graph
        .evalf(sol_zero_p, &ctx)
        .expect("y' = 1 has an evaluable solution")
        .to_f64();
    assert!(
        (value - 4.5).abs() < 1e-12,
        "expected y_p(4.5) = 4.5, got {value}"
    );
}

#[test]
fn test_solve_pde_heat_1d() {
    use algebra_core::format::{Formatter, UnicodeFormatter};

    let graph = ExprGraph::new();
    let alpha = graph.symbol("alpha");
    let x_sym = graph.symbols.get_or_intern("x");
    let t_sym = graph.symbols.get_or_intern("t");

    let (x_sol, t_sol) = graph.solve_pde_heat_1d(alpha, x_sym, t_sym).unwrap();
    let fmt = UnicodeFormatter;
    let xs = fmt.format(&graph, x_sol).unwrap();
    let ts = fmt.format(&graph, t_sol).unwrap();
    assert!(xs.contains("cos") && xs.contains("k_x"), "got spatial={xs}");
    assert!(
        ts.contains("exp") && ts.contains("alpha") && ts.contains('t'),
        "got temporal={ts}"
    );
}

#[test]
fn test_diophantine_solvers() {
    use algebra_engine::solver::DiophantineSolver;

    // 3x + 5y = 1 => x = 2, y = -1 (3(2) + 5(-1) = 6 - 5 = 1)
    let (x, y) = DiophantineSolver::solve_linear_2var(3, 5, 1).unwrap();
    assert_eq!(3 * x + 5 * y, 1);

    // Pell's equation x^2 - 2 y^2 = 1 => fundamental solution x = 3, y = 2
    let (px, py) = DiophantineSolver::solve_pell(2).unwrap();
    assert_eq!(px * px - 2 * py * py, 1);
}
