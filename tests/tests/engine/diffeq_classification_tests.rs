//! Integration tests for Differential Equation classification and automatic solving.

use algebra_core::ExprGraph;
use algebra_engine::diffeq::{DiffEqClassifier, DiffEqKind, DiffEqLinearity};
use algebra_engine::pde::PdeTravelingWaveReduction;

#[test]
fn test_ode_first_order_linear() {
    let graph = ExprGraph::new();
    let y = graph.symbol("y");

    // dy/dt + 2y = 0 -> derivative of y wrt t
    let dy_dt = graph.derivative(y, "t", 1);
    let two_y = graph.mul([graph.integer(2), y]);
    let eq = graph.add([dy_dt, two_y]);

    let desc = DiffEqClassifier::classify(&graph, eq, None);
    assert_eq!(desc.kind, DiffEqKind::Ode);
    assert_eq!(desc.order, 1);
    assert_eq!(desc.linearity, DiffEqLinearity::Linear);
    assert_eq!(desc.independent_vars, vec!["t".to_string()]);
}

#[test]
fn test_ode_second_order_damped_oscillator() {
    let graph = ExprGraph::new();
    let y = graph.symbol("y");

    // y'' + 2*zeta*omega*y' + omega^2*y = 0
    let y_ddot = graph.derivative(y, "t", 2);
    let y_dot = graph.derivative(y, "t", 1);
    let damping = graph.mul([graph.float(0.5), y_dot]);
    let restoring = graph.mul([graph.float(4.0), y]);
    let eq = graph.add([y_ddot, damping, restoring]);

    let desc = DiffEqClassifier::classify(&graph, eq, None);
    assert_eq!(desc.kind, DiffEqKind::Ode);
    assert_eq!(desc.order, 2);
    assert_eq!(desc.linearity, DiffEqLinearity::Linear);
    assert_eq!(desc.independent_vars, vec!["t".to_string()]);
}

#[test]
fn test_ode_nonlinear_pendulum() {
    let graph = ExprGraph::new();
    let theta = graph.symbol("theta");

    // theta'' + sin(theta) = 0
    let theta_ddot = graph.derivative(theta, "t", 2);
    let sin_theta = graph.function("sin", [theta]);
    let eq = graph.add([theta_ddot, sin_theta]);

    let desc = DiffEqClassifier::classify(&graph, eq, None);
    assert_eq!(desc.kind, DiffEqKind::Ode);
    assert_eq!(desc.order, 2);
    assert_eq!(desc.linearity, DiffEqLinearity::Nonlinear);
}

#[test]
fn test_pde_heat_equation_2_vars() {
    let graph = ExprGraph::new();
    let u = graph.symbol("u");

    // u_t - alpha * u_xx = 0
    let u_t = graph.derivative(u, "t", 1);
    let u_xx = graph.derivative(u, "x", 2);
    let alpha_u_xx = graph.mul([graph.float(0.1), u_xx]);
    let eq = graph.sub(u_t, alpha_u_xx);

    let desc = DiffEqClassifier::classify(&graph, eq, None);
    assert_eq!(desc.kind, DiffEqKind::Pde);
    assert_eq!(desc.order, 2);
    assert_eq!(desc.independent_vars.len(), 2);
    assert!(desc.independent_vars.contains(&"t".to_string()));
    assert!(desc.independent_vars.contains(&"x".to_string()));
}

#[test]
fn test_pde_wave_equation_dalembert() {
    let graph = ExprGraph::new();
    let u = graph.symbol("u");

    // u_tt - c^2 * u_xx = 0
    let u_tt = graph.derivative(u, "t", 2);
    let u_xx = graph.derivative(u, "x", 2);
    let c2_u_xx = graph.mul([graph.float(4.0), u_xx]);
    let eq = graph.sub(u_tt, c2_u_xx);

    let desc = DiffEqClassifier::classify(&graph, eq, None);
    assert_eq!(desc.kind, DiffEqKind::Pde);
    assert_eq!(desc.order, 2);
    assert_eq!(desc.linearity, DiffEqLinearity::Linear);
}

#[test]
fn test_pde_nonlinear_kdv() {
    let graph = ExprGraph::new();
    let u = graph.symbol("u");

    // u_t + 6*u*u_x + u_xxx = 0
    let u_t = graph.derivative(u, "t", 1);
    let u_x = graph.derivative(u, "x", 1);
    let six_u_ux = graph.mul([graph.integer(6), u, u_x]);
    let u_xxx = graph.derivative(u, "x", 3);
    let eq = graph.add([u_t, six_u_ux, u_xxx]);

    let desc = DiffEqClassifier::classify(&graph, eq, None);
    assert_eq!(desc.kind, DiffEqKind::Pde);
    assert_eq!(desc.order, 3);
    // Highest derivative u_xxx has a constant coefficient; only a lower-order term is
    // nonlinear, so KdV is semilinear.
    assert_eq!(desc.linearity, DiffEqLinearity::Semilinear);
}

#[test]
fn test_manual_override_respected() {
    let graph = ExprGraph::new();
    let y = graph.symbol("y");

    // Naturally an ODE: y' + y = 0
    let dy_dt = graph.derivative(y, "t", 1);
    let eq = graph.add([dy_dt, y]);

    // Force PDE override
    let desc_pde = DiffEqClassifier::classify(&graph, eq, Some(DiffEqKind::Pde));
    assert_eq!(desc_pde.kind, DiffEqKind::Pde);

    // Naturally a PDE with u_x and u_t
    let u = graph.symbol("u");
    let u_t = graph.derivative(u, "t", 1);
    let u_x = graph.derivative(u, "x", 1);
    let pde_eq = graph.add([u_t, u_x]);

    // Force ODE override
    let desc_ode = DiffEqClassifier::classify(&graph, pde_eq, Some(DiffEqKind::Ode));
    assert_eq!(desc_ode.kind, DiffEqKind::Ode);
}

#[test]
fn test_kdv_traveling_wave_reduction() {
    use algebra_engine::numeric::{EvalContext, NumericalEval};
    use std::collections::HashMap;

    let graph = ExprGraph::new();
    let x_sym = graph.symbols.get_or_intern("x");
    let t_sym = graph.symbols.get_or_intern("t");

    // KdV single-soliton with speed c = 4.0: crest value = c/2 = 2.0 along x = 4*t
    let sol = PdeTravelingWaveReduction::kdv_soliton(&graph, 4.0, x_sym, t_sym).unwrap();
    let mut b = HashMap::new();
    b.insert("x".to_string(), 4.0);
    b.insert("t".to_string(), 1.0);
    let peak = graph
        .evalf(sol, &EvalContext::with_bindings(53, b))
        .unwrap()
        .to_f64();
    assert!(
        (peak - 2.0).abs() < 1e-10,
        "expected KdV crest 2.0, got {peak}"
    );

    let mut b_far = HashMap::new();
    b_far.insert("x".to_string(), 20.0);
    b_far.insert("t".to_string(), 0.0);
    let tail = graph
        .evalf(sol, &EvalContext::with_bindings(53, b_far))
        .unwrap()
        .to_f64();
    assert!(tail < 1e-4, "expected decaying soliton tail, got {tail}");
}

#[test]
fn test_burgers_traveling_wave_reduction() {
    use algebra_engine::numeric::{EvalContext, NumericalEval};
    use std::collections::HashMap;

    let graph = ExprGraph::new();
    let x_sym = graph.symbols.get_or_intern("x");
    let t_sym = graph.symbols.get_or_intern("t");

    // Burgers shock with u_L = 2.0, u_R = 0.0, nu = 0.1 => shock speed c = 1.0
    let sol =
        PdeTravelingWaveReduction::burgers_shock(&graph, 2.0, 0.0, 0.1, x_sym, t_sym).unwrap();

    // At shock center x = c*t = 1.0 (t=1.0), u = (u_L + u_R)/2 = 1.0
    let mut b_mid = HashMap::new();
    b_mid.insert("x".to_string(), 1.0);
    b_mid.insert("t".to_string(), 1.0);
    let mid = graph
        .evalf(sol, &EvalContext::with_bindings(53, b_mid))
        .unwrap()
        .to_f64();
    assert!(
        (mid - 1.0).abs() < 1e-10,
        "expected shock midpoint 1.0, got {mid}"
    );

    // Upstream x = -5 -> u_L = 2.0, Downstream x = 5 -> u_R = 0.0
    let mut b_left = HashMap::new();
    b_left.insert("x".to_string(), -5.0);
    b_left.insert("t".to_string(), 0.0);
    let left_val = graph
        .evalf(sol, &EvalContext::with_bindings(53, b_left))
        .unwrap()
        .to_f64();
    assert!((left_val - 2.0).abs() < 1e-6);
}

#[test]
fn test_pde_nonlinearity_is_located_not_just_detected() {
    let graph = ExprGraph::new();
    let u = graph.symbol("u");
    let d = |wrt: &str, order: u32| graph.derivative(u, wrt, order);
    let classify = |eq| DiffEqClassifier::classify(&graph, eq, None).linearity;

    // Heat equation u_t = u_xx: linear.
    assert_eq!(
        classify(graph.sub(d("t", 1), d("x", 2))),
        DiffEqLinearity::Linear
    );

    // Burgers u_t + u u_x = 0: the coefficient of u_x is u, so quasilinear.
    assert_eq!(
        classify(graph.add([d("t", 1), graph.mul([u, d("x", 1)])])),
        DiffEqLinearity::Quasilinear
    );

    // Sine-Gordon u_tt - u_xx + sin(u) = 0: nonlinearity only in the lower-order term.
    assert_eq!(
        classify(graph.add([d("t", 2), graph.neg(d("x", 2)), graph.function("sin", [u])])),
        DiffEqLinearity::Semilinear
    );

    // Monge-Ampere u_xx u_yy - u_xy^2 = f: the highest derivatives multiply each other.
    let u_xy = graph.derivative(graph.derivative(u, "x", 1), "y", 1);
    let monge = graph.sub(
        graph.mul([d("x", 2), d("y", 2)]),
        graph.pow(u_xy, graph.integer(2)),
    );
    assert_eq!(classify(monge), DiffEqLinearity::Nonlinear);

    // (u_xx)^2 + u_t = 0: highest derivative inside a power.
    assert_eq!(
        classify(graph.add([graph.pow(d("x", 2), graph.integer(2)), d("t", 1)])),
        DiffEqLinearity::Nonlinear
    );

    // ODE vocabulary is unchanged: the pendulum is just nonlinear.
    let theta = graph.symbol("theta");
    let pendulum = graph.add([
        graph.derivative(theta, "t", 2),
        graph.function("sin", [theta]),
    ]);
    assert_eq!(
        DiffEqClassifier::classify(&graph, pendulum, None).linearity,
        DiffEqLinearity::Nonlinear
    );
}
