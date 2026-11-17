//! The adjoint gradient is the exact gradient of the discrete RK4 trajectory.

use algebra_engine::neural_ode::{AdjointSensitivitySolver, NeuralOde};

type Field = fn(&[f64], f64, &[f64]) -> Vec<f64>;

/// Loss L = 0.5 |x(1)|^2 through the forward solver, for finite differencing.
fn loss(field: Field, x0: &[f64], theta: &[f64], steps: usize) -> f64 {
    let traj = NeuralOde::forward(field, x0, theta, 0.0, 1.0, steps);
    traj.states
        .last()
        .unwrap()
        .iter()
        .map(|v| 0.5 * v * v)
        .sum()
}

#[test]
fn linear_decay_gradient_matches_the_analytic_value_to_fourth_order() {
    // dx/dt = -theta x, x(1) = x0 e^{-theta}, L = x(1)^2 / 2, dL/dtheta = -x0^2 e^{-2 theta}.
    let field: Field = |x, _t, th| vec![-th[0] * x[0]];
    let df_dx = |_x: &[f64], _t: f64, th: &[f64]| vec![vec![-th[0]]];
    let df_dth = |x: &[f64], _t: f64, _th: &[f64]| vec![vec![-x[0]]];
    let (x0, theta) = ([1.0], [0.5]);
    let exact = -(-1.0_f64).exp();

    let mut previous = f64::INFINITY;
    for steps in [10, 20, 40] {
        let traj = NeuralOde::forward(field, &x0, &theta, 0.0, 1.0, steps);
        let seed = [traj.states.last().unwrap()[0]];
        let grad =
            AdjointSensitivitySolver::compute_gradient(field, df_dx, df_dth, &traj, &theta, &seed);
        let error = (grad[0] - exact).abs();
        // Fourth order: halving the step cuts the error by about 16.
        assert!(
            error < previous / 10.0,
            "steps {steps}: error {error:e} vs {previous:e}"
        );
        previous = error;
    }
    assert!(previous < 1e-8, "{previous:e}");
}

#[test]
fn gradient_equals_the_finite_difference_of_the_discrete_solver() {
    // A coupled nonlinear system with two parameters:
    //   x0' = theta0 * sin(x1),  x1' = -theta1 * x0 + x0 * x1
    let field: Field = |x, _t, th| vec![th[0] * x[1].sin(), -th[1] * x[0] + x[0] * x[1]];
    let df_dx = |x: &[f64], _t: f64, th: &[f64]| {
        vec![vec![0.0, th[0] * x[1].cos()], vec![-th[1] + x[1], x[0]]]
    };
    let df_dth = |x: &[f64], _t: f64, _th: &[f64]| vec![vec![x[1].sin(), 0.0], vec![0.0, -x[0]]];
    let (x0, theta, steps) = ([0.7, -0.4], [0.9, 1.3], 25);

    let traj = NeuralOde::forward(field, &x0, &theta, 0.0, 1.0, steps);
    let seed: Vec<f64> = traj.states.last().unwrap().clone(); // dL/dx(1) for L = |x|^2 / 2
    let grad =
        AdjointSensitivitySolver::compute_gradient(field, df_dx, df_dth, &traj, &theta, &seed);

    // The adjoint differentiates the *discrete* map, so a central difference of the same
    // discrete solver agrees to finite-difference accuracy, with no truncation-error floor.
    let eps = 1e-6;
    for p in 0..2 {
        let (mut up, mut down) = (theta, theta);
        up[p] += eps;
        down[p] -= eps;
        let fd = (loss(field, &x0, &up, steps) - loss(field, &x0, &down, steps)) / (2.0 * eps);
        assert!(
            (grad[p] - fd).abs() < 1e-8 * fd.abs().max(1.0),
            "theta[{p}]: adjoint {} vs finite difference {fd}",
            grad[p]
        );
    }
}

#[test]
fn a_single_point_trajectory_has_no_gradient() {
    let field: Field = |x, _t, th| vec![-th[0] * x[0]];
    let traj = NeuralOde::forward(field, &[1.0], &[0.5], 0.0, 1.0, 0);
    let grad = AdjointSensitivitySolver::compute_gradient(
        field,
        |_x: &[f64], _t: f64, th: &[f64]| vec![vec![-th[0]]],
        |x: &[f64], _t: f64, _th: &[f64]| vec![vec![-x[0]]],
        &traj,
        &[0.5],
        &[1.0],
    );
    assert_eq!(grad, vec![0.0]);
}
