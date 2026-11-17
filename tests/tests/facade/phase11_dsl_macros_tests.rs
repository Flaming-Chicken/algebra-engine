//! Integration Tests for Phase 11: Comprehensive Ergonomic CAS DSL Macro Suite.

use algebra_core::ExprGraph;
use algebra_engine::*;

#[test]
fn test_macro_calculus_diff_integrate_risch() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let x_sym = graph.symbols.get_or_intern("x");

    // 1. diff!
    let x2 = graph.pow(x, graph.integer(2));
    let dx = diff!(&graph, x2, x_sym);
    assert_ne!(dx, x2);

    // Multi-order diff!
    let d2x = diff!(&graph, x2, x_sym, 2);
    assert_ne!(d2x, dx);

    // 2. integrate!
    let int_x = integrate!(&graph, x, x_sym).unwrap();
    assert_ne!(int_x, x);

    // 3. risch!
    let risch_res = risch!(&graph, x, x_sym);
    assert!(risch_res.is_ok());
}

#[test]
fn test_macro_limits_series_pade() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let x_sym = graph.symbols.get_or_intern("x");
    let zero = graph.integer(0);

    // 1. limit!
    let lim = limit!(&graph, x, x_sym, zero).unwrap();
    assert_eq!(lim, zero);

    // 2. taylor!
    let taylor_res = taylor!(&graph, x, x_sym, zero, 3);
    assert_ne!(taylor_res, zero);

    // 3. laurent!
    let laurent_res = laurent!(&graph, x, x_sym, zero, 1, 3).unwrap();
    assert_ne!(laurent_res, zero);

    // 4. puiseux!
    let puiseux_res = puiseux!(&graph, x, x_sym, zero, 2, 3).unwrap();
    assert_ne!(puiseux_res, zero);

    // 5. pade!
    let pade_res = pade!(&graph, x, x_sym, zero, 2, 2).unwrap();
    assert_ne!(pade_res, zero);
}

#[test]
fn test_macro_vector_calculus_grad_div_curl_laplacian_jacobian_hessian() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let y = graph.symbol("y");
    let z = graph.symbol("z");
    let x_sym = graph.symbols.get_or_intern("x");
    let y_sym = graph.symbols.get_or_intern("y");
    let z_sym = graph.symbols.get_or_intern("z");

    let f = graph.add([x, y, z]);

    // 1. grad!
    let g = grad!(&graph, f, [x_sym, y_sym, z_sym]);
    assert_eq!(g.len(), 3);

    // 2. div!
    let d = div!(&graph, [x, y, z], [x_sym, y_sym, z_sym]);
    assert_ne!(d, f);

    // 3. curl!
    let c = curl!(&graph, [x, y, z], [x_sym, y_sym, z_sym]);
    assert_eq!(c.len(), 3);

    // 4. laplacian!
    let lap = laplacian!(&graph, f, [x_sym, y_sym, z_sym]);
    assert_ne!(lap, f);

    // 5. jacobian!
    let jac = jacobian!(&graph, [x, y], [x_sym, y_sym]);
    assert_eq!(jac.len(), 2);
    assert_eq!(jac[0].len(), 2);

    // 6. hessian!
    let hess = hessian!(&graph, f, [x_sym, y_sym]);
    assert_eq!(hess.len(), 2);
    assert_eq!(hess[0].len(), 2);
}

#[test]
fn test_macro_solvers_and_simplification() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let x_sym = graph.symbols.get_or_intern("x");

    // 1. simplify! & simplify_certified!
    let zero = graph.integer(0);
    let x_plus_0 = graph.add([x, zero]);
    let simp = simplify!(&graph, x_plus_0).unwrap();
    assert_eq!(simp, x);

    let cert = simplify_certified!(&graph, x_plus_0, 1e-15).unwrap();
    assert_eq!(*cert.as_ref(), x);

    // 2. solve! & solve_certified!
    let x2 = graph.pow(x, graph.integer(2));
    let eq = graph.sub(x2, graph.integer(4));
    let roots = solve!(&graph, eq, x_sym).unwrap();
    assert_eq!(roots.len(), 2);

    let cert_roots = solve_certified!(&graph, eq, x_sym, 1e-12).unwrap();
    assert_eq!(cert_roots.into_inner().len(), 2);

    // 3. diophantine!
    let dio = diophantine!(3, 5, 1);
    assert!(dio.is_some());
}

#[test]
fn test_macro_differential_equations_and_numerical_steppers() {
    let graph = ExprGraph::new();
    let p = graph.integer(1);
    let q = graph.integer(0);
    let x_sym = graph.symbols.get_or_intern("x");
    let t_sym = graph.symbols.get_or_intern("t");

    // 1. ode_linear!
    let ode_res = ode_linear!(&graph, p, q, x_sym);
    assert!(ode_res.is_ok());

    // 2. pde_heat_1d!
    let alpha = graph.integer(1);
    let pde_res = pde_heat_1d!(&graph, alpha, x_sym, t_sym);
    assert!(pde_res.is_ok());

    // 3. verlet!
    let y0 = [1.0, 0.0];
    let traj = verlet!(|_t, y| vec![y[1], -y[0]], (0.0, 1.0), &y0);
    assert!(traj.is_ok());

    // 4. dopri5!
    let dopri_res = dopri5!(|_t, y| vec![-y[0]], (0.0, 1.0), &[1.0], 1e-6);
    assert!(dopri_res.is_ok());

    // 5. radau5!
    let radau_res = radau5!(|_t, y| vec![-50.0 * y[0]], (0.0, 0.1), &[1.0], 1e-4);
    assert!(radau_res.is_ok());
}

#[test]
fn test_macro_clifford_forms_quantum() {
    let graph = ExprGraph::new();
    let a = graph.symbol("A");
    let b = graph.symbol("B");

    // 1. rotor! & clifford_sandwich!
    let r = rotor!(std::f64::consts::PI, 1.0);
    let rotated = clifford_sandwich!(r, (1.0, 0.0));
    assert!((rotated.0 - (-1.0)).abs() < 1e-10);

    // 2. commutator! & anticommutator!
    let comm = commutator!(&graph, a, b);
    let anticomm = anticommutator!(&graph, a, b);
    assert_ne!(comm, anticomm);
}

#[test]
fn test_macro_transforms_block_diagrams_and_certification() {
    let graph = ExprGraph::new();
    let t_sym = graph.symbols.get_or_intern("t");
    let s_sym = graph.symbols.get_or_intern("s");

    // 1. laplace!
    let one = graph.integer(1);
    let l_res = laplace!(&graph, one, t_sym, s_sym);
    assert!(l_res.is_ok());

    // 2. block diagram algebra
    let g = graph.symbol("G");
    let h = graph.symbol("H");
    let feedback = block_feedback!(&graph, g, h);
    let series = block_series!(&graph, g, h);
    let parallel = block_parallel!(&graph, g, h);
    assert_ne!(feedback, series);
    assert_ne!(series, parallel);

    // 3. prob_test! & certify_zero!
    let verif = prob_test!(&graph, feedback, feedback, 1e-15);
    assert!(*verif.as_ref());

    let zero = graph.integer(0);
    let cert_z = certify_zero!(&graph, zero, 1e-15);
    assert!(*cert_z.as_ref());
}
