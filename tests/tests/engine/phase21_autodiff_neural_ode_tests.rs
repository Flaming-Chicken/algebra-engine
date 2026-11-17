//! Integration Tests for Phase 21: Automatic Differentiation, Neural ODEs & PINNs.

use algebra_engine::autodiff::{Dual, ForwardGradient, HyperDual, TapeFreeVjp};
use algebra_engine::neural_ode::{AdjointSensitivitySolver, NeuralOde};
use algebra_engine::pinn::PinnResidual;
use algebra_engine::{autodiff_forward, autodiff_hessian, neural_ode_step, pinn_burgers_residual};

#[test]
fn test_dual_numbers_transcendental_derivatives() {
    let x_val = 2.0;
    let x = Dual::var(x_val);

    // f(x) = exp(sin(x)) + ln(x^2 + 1)
    let f = x.sin().exp() + (x * x + 1.0).ln();

    let analytical_val = (x_val.sin()).exp() + (x_val * x_val + 1.0).ln();
    let analytical_deriv =
        x_val.cos() * (x_val.sin()).exp() + (2.0 * x_val) / (x_val * x_val + 1.0);

    assert!((f.val - analytical_val).abs() < 1e-12);
    assert!((f.eps - analytical_deriv).abs() < 1e-12);
}

#[test]
fn test_hyper_dual_exact_rosenbrock_hessian() {
    // Rosenbrock: f(x, y) = (1 - x)^2 + 100 * (y - x^2)^2
    let rosenbrock = |vars: &[HyperDual<f64>]| -> HyperDual<f64> {
        let x = vars[0];
        let y = vars[1];
        let term1 = (HyperDual::constant(1.0) - x) * (HyperDual::constant(1.0) - x);
        let diff = y - x * x;
        let term2 = HyperDual::constant(100.0) * diff * diff;
        term1 + term2
    };

    let pt = [1.0, 1.0];
    let (val, grad, hess) = ForwardGradient::hessian(rosenbrock, &pt);

    // At (1, 1), f(x, y) = 0.0, grad = (0.0, 0.0)
    assert!(val.abs() < 1e-10);
    assert!(grad[0].abs() < 1e-10);
    assert!(grad[1].abs() < 1e-10);

    // Exact Hessian at (1, 1): [[802, -400], [-400, 200]]
    assert!((hess[0][0] - 802.0).abs() < 1e-10);
    assert!((hess[0][1] - (-400.0)).abs() < 1e-10);
    assert!((hess[1][0] - (-400.0)).abs() < 1e-10);
    assert!((hess[1][1] - 200.0).abs() < 1e-10);
}

#[test]
fn test_tape_free_vjp() {
    // F(x, y) = [x^2 + y, sin(x) * y]
    let f = |vars: &[Dual<f64>]| -> Vec<Dual<f64>> {
        let x = vars[0];
        let y = vars[1];
        vec![x * x + y, x.sin() * y]
    };

    let pt = [0.5, 3.0];
    let v = [1.0, 2.0];
    let vjp_res = TapeFreeVjp::vjp(f, &pt, &v);

    // J = [[2*x, 1], [cos(x)*y, sin(x)]]
    // at (0.5, 3.0): J = [[1.0, 1.0], [cos(0.5)*3.0, sin(0.5)]]
    // v^T J = [1.0 * 1.0 + 2.0 * (cos(0.5)*3.0), 1.0 * 1.0 + 2.0 * sin(0.5)]
    let expected_0 = 1.0 + 2.0 * (0.5_f64.cos() * 3.0);
    let expected_1 = 1.0 + 2.0 * 0.5_f64.sin();

    assert!((vjp_res[0] - expected_0).abs() < 1e-10);
    assert!((vjp_res[1] - expected_1).abs() < 1e-10);
}

#[test]
fn test_neural_ode_trajectory_and_adjoint_sensitivity() {
    // System: dx/dt = -theta * x
    let vector_field = |x: &[f64], _t: f64, theta: &[f64]| -> Vec<f64> { vec![-theta[0] * x[0]] };
    let df_dx = |_x: &[f64], _t: f64, theta: &[f64]| -> Vec<Vec<f64>> { vec![vec![-theta[0]]] };
    let df_dtheta = |x: &[f64], _t: f64, _theta: &[f64]| -> Vec<Vec<f64>> { vec![vec![-x[0]]] };

    let x0 = [1.0];
    let theta = [0.5];
    let traj = NeuralOde::forward(vector_field, &x0, &theta, 0.0, 1.0, 100);

    // Exact solution: x(1) = exp(-0.5)
    let exact_final = (-0.5_f64).exp();
    let computed_final = traj.states.last().unwrap()[0];
    assert!((computed_final - exact_final).abs() < 1e-4);

    // Loss L = 0.5 * (x(1) - 0.0)^2 => dL/dx(1) = x(1)
    let final_loss_grad = [computed_final];
    let dl_dtheta = AdjointSensitivitySolver::compute_gradient(
        vector_field,
        df_dx,
        df_dtheta,
        &traj,
        &theta,
        &final_loss_grad,
    );

    // Analytical dL/dtheta: x(1) * dx(1)/dtheta = exp(-0.5) * (-1.0 * exp(-0.5)) = -exp(-1.0)
    let analytical_grad = -(-1.0_f64).exp();
    // The adjoint now differentiates the RK4 steps themselves, so with 100 steps it is accurate
    // to about 1e-9 (the earlier first-order scheme needed a 1e-2 tolerance).
    assert!((dl_dtheta[0] - analytical_grad).abs() < 1e-8);
}

#[test]
fn test_pinn_burgers_equation_residual() {
    // Exact steady state or ansatz function
    let u_ansatz = |x: HyperDual<f64>, _t: HyperDual<f64>| -> HyperDual<f64> {
        // Simple linear velocity profile u(x, t) = x
        x
    };

    // Burgers: u_t + u * u_x - nu * u_xx
    // For u = x: u_t = 0, u_x = 1, u_xx = 0 => residual = 0 + x * 1 - 0 = x
    let res = PinnResidual::burgers_residual(u_ansatz, 2.5, 0.0, 0.01);
    assert!((res - 2.5).abs() < 1e-10);

    let residuals = vec![res, 1.0, -1.0];
    let mse = PinnResidual::mean_squared_loss(&residuals);
    // (2.5^2 + 1 + 1) / 3 = (6.25 + 2) / 3 = 8.25 / 3 = 2.75
    assert!((mse - 2.75).abs() < 1e-10);
}

#[test]
fn test_phase21_dsl_macros() {
    let f_dual = |vars: &[Dual<f64>]| -> Dual<f64> { vars[0] * vars[0] + vars[1] * vars[1] };
    let pt = [3.0, 4.0];
    let (val, grad) = autodiff_forward!(f_dual, &pt);
    assert!((val - 25.0).abs() < 1e-10);
    assert!((grad[0] - 6.0).abs() < 1e-10);
    assert!((grad[1] - 8.0).abs() < 1e-10);

    let f_hd = |vars: &[HyperDual<f64>]| -> HyperDual<f64> {
        vars[0] * vars[0] * vars[0] + vars[1] * vars[1]
    };
    let (_v, _g, hess) = autodiff_hessian!(f_hd, &pt);
    // H_11 = 6*x = 18.0, H_22 = 2.0
    assert!((hess[0][0] - 18.0).abs() < 1e-10);
    assert!((hess[1][1] - 2.0).abs() < 1e-10);

    let vf = |x: &[f64], _t: f64, th: &[f64]| -> Vec<f64> { vec![th[0] * x[0]] };
    let traj = neural_ode_step!(
        vf,
        x0 = &[1.0],
        theta = &[1.0],
        t0 = 0.0,
        t1 = 1.0,
        steps = 50
    );
    assert_eq!(traj.states.len(), 51);

    let u_linear = |x: HyperDual<f64>, _t: HyperDual<f64>| -> HyperDual<f64> { x };
    let res = pinn_burgers_residual!(u_linear, x = 3.0, t = 0.0, nu = 0.1);
    assert!((res - 3.0).abs() < 1e-10);
}
