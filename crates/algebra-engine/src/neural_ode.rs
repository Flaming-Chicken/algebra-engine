//! # `algebra_engine::neural_ode`
//!
//! Continuous-Depth Neural Ordinary Differential Equations (Neural ODEs).
//!
//! Formulates parameterized continuous vector fields $\frac{d\mathbf{x}}{dt} = f_\theta(\mathbf{x}, t)$
//! with forward trajectory solving and an adjoint sensitivity method that back-propagates exactly through the RK4 steps.

use serde::{Deserialize, Serialize};

/// Trajectory output from a Neural ODE simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralOdeTrajectory {
    pub times: Vec<f64>,
    pub states: Vec<Vec<f64>>,
}

/// Continuous-Depth Neural ODE System.
pub struct NeuralOde;

impl NeuralOde {
    /// Solve forward trajectory $\mathbf{x}(t)$ from $t_0$ to $t_1$ with step size $h$ using 4th-order Runge-Kutta (RK4).
    pub fn forward<F>(
        vector_field: F,
        x0: &[f64],
        theta: &[f64],
        t0: f64,
        t1: f64,
        steps: usize,
    ) -> NeuralOdeTrajectory
    where
        F: Fn(&[f64], f64, &[f64]) -> Vec<f64>,
    {
        let h = (t1 - t0) / (steps as f64);
        let dim = x0.len();
        let mut times = Vec::with_capacity(steps + 1);
        let mut states = Vec::with_capacity(steps + 1);

        let mut current_t = t0;
        let mut current_x = x0.to_vec();

        times.push(current_t);
        states.push(current_x.clone());

        for _ in 0..steps {
            // k1 = f(x, t)
            let k1 = vector_field(&current_x, current_t, theta);

            // k2 = f(x + 0.5*h*k1, t + 0.5*h)
            let mut x_k2 = vec![0.0; dim];
            for i in 0..dim {
                x_k2[i] = current_x[i] + 0.5 * h * k1[i];
            }
            let k2 = vector_field(&x_k2, current_t + 0.5 * h, theta);

            // k3 = f(x + 0.5*h*k2, t + 0.5*h)
            let mut x_k3 = vec![0.0; dim];
            for i in 0..dim {
                x_k3[i] = current_x[i] + 0.5 * h * k2[i];
            }
            let k3 = vector_field(&x_k3, current_t + 0.5 * h, theta);

            // k4 = f(x + h*k3, t + h)
            let mut x_k4 = vec![0.0; dim];
            for i in 0..dim {
                x_k4[i] = current_x[i] + h * k3[i];
            }
            let k4 = vector_field(&x_k4, current_t + h, theta);

            // Update state
            for i in 0..dim {
                current_x[i] += (h / 6.0) * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
            }
            current_t += h;

            times.push(current_t);
            states.push(current_x.clone());
        }

        NeuralOdeTrajectory { times, states }
    }
}

/// Adjoint Sensitivity Method for Neural ODE Backpropagation.
///
/// This is the *discrete* adjoint of the RK4 forward pass in [`NeuralOde::forward`]: it
/// back-propagates through the four stages of every step, so it returns the exact gradient of the
/// loss computed from the numerical trajectory (and matches the continuous adjoint equation as the
/// step size goes to zero, at RK4's fourth order). A scheme that integrates the continuous
/// adjoint ODE with a lower-order rule instead has an O(h) gradient error against a fourth-order
/// forward pass.
pub struct AdjointSensitivitySolver;

impl AdjointSensitivitySolver {
    /// Gradient $\frac{\partial L}{\partial \theta}$ of a loss that depends on the final state,
    /// given `final_loss_grad` $= \partial L / \partial x(t_1)$.
    ///
    /// `vector_field` is recomputed at the stage states of each step (the trajectory stores only
    /// step endpoints, so memory stays $O(\text{steps})$ with no stored stage values). `df_dx`
    /// returns $\partial f_i / \partial x_j$ as an `n x n` matrix and `df_dtheta` returns
    /// $\partial f_i / \partial \theta_p$ as an `n x p` matrix, both at `(x, t, theta)`.
    pub fn compute_gradient<F, Dfx, Dftheta>(
        vector_field: F,
        df_dx: Dfx,
        df_dtheta: Dftheta,
        traj: &NeuralOdeTrajectory,
        theta: &[f64],
        final_loss_grad: &[f64],
    ) -> Vec<f64>
    where
        F: Fn(&[f64], f64, &[f64]) -> Vec<f64>,
        Dfx: Fn(&[f64], f64, &[f64]) -> Vec<Vec<f64>>,
        Dftheta: Fn(&[f64], f64, &[f64]) -> Vec<Vec<f64>>,
    {
        let p_dim = theta.len();
        let mut dl_dtheta = vec![0.0; p_dim];
        let mut lambda = final_loss_grad.to_vec();
        let n = lambda.len();
        if traj.times.len() < 2 {
            return dl_dtheta;
        }

        // y = J^T g for an `rows x cols` Jacobian `j` (j[i][c]) and a vector `g` over the rows.
        let transpose_apply = |j: &[Vec<f64>], g: &[f64], cols: usize| -> Vec<f64> {
            (0..cols)
                .map(|c| g.iter().enumerate().map(|(i, gi)| gi * j[i][c]).sum())
                .collect()
        };

        for step in (0..traj.times.len() - 1).rev() {
            let (t, h) = (traj.times[step], traj.times[step + 1] - traj.times[step]);
            let x = &traj.states[step];

            // Recompute the stage states of this RK4 step.
            let stage_input = |base: &[f64], k: &[f64], scale: f64| -> Vec<f64> {
                (0..n).map(|i| base[i] + scale * k[i]).collect()
            };
            let x1 = x.clone();
            let k1 = vector_field(&x1, t, theta);
            let x2 = stage_input(x, &k1, 0.5 * h);
            let k2 = vector_field(&x2, t + 0.5 * h, theta);
            let x3 = stage_input(x, &k2, 0.5 * h);
            let k3 = vector_field(&x3, t + 0.5 * h, theta);
            let x4 = stage_input(x, &k3, h);
            let stages = [(x1, t), (x2, t + 0.5 * h), (x3, t + 0.5 * h), (x4, t + h)];

            // Cotangents of the stage slopes: x_{n+1} = x + h/6 (k1 + 2 k2 + 2 k3 + k4).
            let mut g = [
                lambda.iter().map(|l| h / 6.0 * l).collect::<Vec<f64>>(),
                lambda.iter().map(|l| h / 3.0 * l).collect(),
                lambda.iter().map(|l| h / 3.0 * l).collect(),
                lambda.iter().map(|l| h / 6.0 * l).collect(),
            ];
            let mut gx = vec![0.0; n];

            for stage in (0..4).rev() {
                let (state, time) = &stages[stage];
                let j_x = df_dx(state, *time, theta);
                let j_theta = df_dtheta(state, *time, theta);
                // k_stage = f(X_stage): X_stage-cotangent and parameter gradient.
                let g_state = transpose_apply(&j_x, &g[stage], n);
                let g_param = transpose_apply(&j_theta, &g[stage], p_dim);
                for (acc, v) in dl_dtheta.iter_mut().zip(&g_param) {
                    *acc += v;
                }
                // X_stage depends on x directly and on the previous stage slope.
                for (acc, v) in gx.iter_mut().zip(&g_state) {
                    *acc += v;
                }
                if stage > 0 {
                    let carry = if stage == 3 { h } else { 0.5 * h };
                    for (slot, v) in g[stage - 1].iter_mut().zip(&g_state) {
                        *slot += carry * v;
                    }
                }
            }
            for (l, v) in lambda.iter_mut().zip(&gx) {
                *l += v;
            }
        }

        dl_dtheta
    }
}
