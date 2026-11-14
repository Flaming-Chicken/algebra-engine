//! # `algebra_engine::pde::numerical`
//!
//! Configurable numerical Partial Differential Equation (PDE) solver engine.
//!
//! Implements:
//! - **Method of Lines (MOL)**: Discretizes spatial dimensions into a coupled stiff ODE system and integrates via the adaptive ODE engine.
//! - **Finite Difference Method (FDM)**: Explicit FTCS with CFL condition tracking, Implicit Crank-Nicolson 2nd-order scheme.
//! - **1D Finite Element Method (FEM)**: Galerkin weak formulation $\int_\Omega u' v' dx = \int_\Omega f v dx$ with piecewise linear basis elements.

use crate::ode::numerical::{NumericalOdeConfig, NumericalOdeSolver};
use algebra_core::error::{AlgebraError, AlgebraResult};

/// Numerical boundary condition type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BoundaryCondition {
    /// Fixed boundary value $u(x_b, t) = \text{val}$ (Dirichlet)
    Dirichlet(f64),
    /// Fixed flux $\frac{\partial u}{\partial n}(x_b, t) = \text{val}$ (Neumann)
    Neumann(f64),
}

/// Numerical PDE Solution Grid.
#[derive(Debug, Clone)]
pub struct PdeGridSolution {
    /// Spatial grid $x_0, x_1, \dots, x_N$.
    pub x: Vec<f64>,
    /// Temporal grid $t_0, t_1, \dots, t_M$.
    pub t: Vec<f64>,
    /// Solution field $u(x_i, t_j)$ of dimensions $M \times N$.
    pub u: Vec<Vec<f64>>,
    /// Name of numerical scheme executed.
    pub scheme_name: String,
}

/// Numerical PDE Solver Engine.
pub struct NumericalPdeSolver;

impl NumericalPdeSolver {
    /// Solve 1D Diffusion-Reaction PDE via Method of Lines (MOL):
    ///
    /// $$\frac{\partial u}{\partial t} = D \frac{\partial^2 u}{\partial x^2} + R(u, x, t)$$
    ///
    /// on domain $[x_0, x_{\text{end}}]$ with Dirichlet boundary conditions.
    #[allow(clippy::too_many_arguments)]
    pub fn solve_diffusion_reaction_mol<R>(
        d_coeff: f64,
        reaction: R,
        x_span: (f64, f64),
        t_span: (f64, f64),
        n_spatial_points: usize,
        u0_func: impl Fn(f64) -> f64,
        bc_left: BoundaryCondition,
        bc_right: BoundaryCondition,
        ode_config: &NumericalOdeConfig,
    ) -> AlgebraResult<PdeGridSolution>
    where
        R: Fn(f64, f64, f64) -> f64 + Send + Sync + 'static,
    {
        if n_spatial_points < 3 {
            return Err(AlgebraError::EvaluationError(
                "Spatial grid points must be at least 3".into(),
            ));
        }

        let (x0, xend) = x_span;
        let dx = (xend - x0) / ((n_spatial_points - 1) as f64);
        let dx_sq = dx * dx;

        let mut x_grid = Vec::with_capacity(n_spatial_points);
        let mut u0 = Vec::with_capacity(n_spatial_points);
        for i in 0..n_spatial_points {
            let xi = x0 + (i as f64) * dx;
            x_grid.push(xi);
            u0.push(u0_func(xi));
        }

        // Apply Dirichlet boundary conditions to initial vector
        if let BoundaryCondition::Dirichlet(val) = bc_left {
            u0[0] = val;
        }
        if let BoundaryCondition::Dirichlet(val) = bc_right {
            u0[n_spatial_points - 1] = val;
        }

        // Construct ODE right-hand side du_i/dt = D * (u_{i+1} - 2u_i + u_{i-1})/dx^2 + R(u_i, x_i, t)
        let x_coords = x_grid.clone();
        let ode_rhs = move |t: f64, u_state: &[f64]| -> Vec<f64> {
            let n = u_state.len();
            let mut du_dt = vec![0.0; n];

            for i in 1..(n - 1) {
                let d2u_dx2 = (u_state[i + 1] - 2.0 * u_state[i] + u_state[i - 1]) / dx_sq;
                let r_val = reaction(u_state[i], x_coords[i], t);
                du_dt[i] = d_coeff * d2u_dx2 + r_val;
            }

            // Boundary rate of change
            match bc_left {
                BoundaryCondition::Dirichlet(_) => du_dt[0] = 0.0,
                BoundaryCondition::Neumann(flux) => {
                    let d2u_dx2 = 2.0 * (u_state[1] - u_state[0] - dx * flux) / dx_sq;
                    du_dt[0] = d_coeff * d2u_dx2 + reaction(u_state[0], x_coords[0], t);
                }
            }

            match bc_right {
                BoundaryCondition::Dirichlet(_) => du_dt[n - 1] = 0.0,
                BoundaryCondition::Neumann(flux) => {
                    let d2u_dx2 = 2.0 * (u_state[n - 2] - u_state[n - 1] + dx * flux) / dx_sq;
                    du_dt[n - 1] = d_coeff * d2u_dx2 + reaction(u_state[n - 1], x_coords[n - 1], t);
                }
            }

            du_dt
        };

        let traj = NumericalOdeSolver::solve(ode_rhs, t_span, &u0, ode_config)?;

        Ok(PdeGridSolution {
            x: x_grid,
            t: traj.t,
            u: traj.y,
            scheme_name: format!("Method of Lines ({})", traj.method_used),
        })
    }

    /// Solve 1D Wave Equation $u_{tt} = c^2 u_{xx}$ via Finite Difference Method (FDM Leapfrog):
    ///
    /// $$u_i^{n+1} = 2u_i^n - u_i^{n-1} + \left(\frac{c \Delta t}{\Delta x}\right)^2 \left( u_{i+1}^n - 2u_i^n + u_{i-1}^n \right)$$
    pub fn solve_wave_fdm(
        c_speed: f64,
        x_span: (f64, f64),
        t_span: (f64, f64),
        n_spatial_points: usize,
        u0_func: impl Fn(f64) -> f64,
        v0_func: impl Fn(f64) -> f64,
        dt_step: Option<f64>,
    ) -> AlgebraResult<PdeGridSolution>
where {
        let (x0, xend) = x_span;
        let (t0, tend) = t_span;
        let dx = (xend - x0) / ((n_spatial_points - 1) as f64);

        // CFL condition: Courant number C = c * dt / dx <= 1.0 for stability
        let max_stable_dt = 0.9 * dx / c_speed;
        let dt = dt_step.unwrap_or(max_stable_dt).min(max_stable_dt);
        let courant_sq = (c_speed * dt / dx).powi(2);

        let n_time_steps = ((tend - t0) / dt).ceil() as usize;

        let mut x_grid = Vec::with_capacity(n_spatial_points);
        let mut u_prev = Vec::with_capacity(n_spatial_points);
        let mut v_init = Vec::with_capacity(n_spatial_points);

        for i in 0..n_spatial_points {
            let xi = x0 + (i as f64) * dx;
            x_grid.push(xi);
            u_prev.push(u0_func(xi));
            v_init.push(v0_func(xi));
        }

        // First time step using initial velocity: u^1 = u^0 + dt * v^0 + 0.5 * C^2 * (u_{i+1}^0 - 2u_i^0 + u_{i-1}^0)
        let mut u_curr = vec![0.0; n_spatial_points];
        for i in 1..(n_spatial_points - 1) {
            let lap = u_prev[i + 1] - 2.0 * u_prev[i] + u_prev[i - 1];
            u_curr[i] = u_prev[i] + dt * v_init[i] + 0.5 * courant_sq * lap;
        }

        let mut t_grid = vec![t0, t0 + dt];
        let mut u_history = vec![u_prev.clone(), u_curr.clone()];

        let mut u_n_minus_1 = u_prev;
        let mut u_n = u_curr;

        for step in 2..=n_time_steps {
            let t_val = t0 + (step as f64) * dt;
            let mut u_next = vec![0.0; n_spatial_points];

            for i in 1..(n_spatial_points - 1) {
                let lap = u_n[i + 1] - 2.0 * u_n[i] + u_n[i - 1];
                u_next[i] = 2.0 * u_n[i] - u_n_minus_1[i] + courant_sq * lap;
            }

            // Fixed Dirichlet boundaries at endpoints
            u_next[0] = 0.0;
            u_next[n_spatial_points - 1] = 0.0;

            t_grid.push(t_val);
            u_history.push(u_next.clone());

            u_n_minus_1 = u_n;
            u_n = u_next;
        }

        Ok(PdeGridSolution {
            x: x_grid,
            t: t_grid,
            u: u_history,
            scheme_name: "Finite Difference Wave (CFL Stable)".to_string(),
        })
    }

    /// Solve 1D Poisson Equation $-u''(x) = f(x)$ on $[x_0, x_{\text{end}}]$ with $u(x_0) = u_L, u(x_{\text{end}}) = u_R$
    /// via the 1D Galerkin Finite Element Method (FEM) with piecewise linear Lagrange basis elements.
    pub fn solve_poisson_fem_1d(
        x_span: (f64, f64),
        n_elements: usize,
        f_source: impl Fn(f64) -> f64,
        u_left: f64,
        u_right: f64,
    ) -> AlgebraResult<(Vec<f64>, Vec<f64>)> {
        let (x0, xend) = x_span;
        let n_nodes = n_elements + 1;
        let h = (xend - x0) / (n_elements as f64);

        let mut x_nodes = Vec::with_capacity(n_nodes);
        for i in 0..n_nodes {
            x_nodes.push(x0 + (i as f64) * h);
        }

        // Global stiffness matrix K (tridiagonal) and load vector F
        // K_ii = 2/h, K_{i, i+1} = -1/h
        let mut diag = vec![2.0 / h; n_nodes];
        let mut off_diag = vec![-1.0 / h; n_nodes - 1];
        let mut load = vec![0.0; n_nodes];

        // Assemble load vector F_i = \int f(x) \phi_i(x) dx \approx h * f(x_i)
        for i in 0..n_nodes {
            load[i] = h * f_source(x_nodes[i]);
        }

        // Enforce Dirichlet Boundary Conditions: u[0] = u_left, u[n_nodes-1] = u_right
        diag[0] = 1.0;
        off_diag[0] = 0.0;
        load[0] = u_left;

        diag[n_nodes - 1] = 1.0;
        off_diag[n_nodes - 2] = 0.0;
        load[n_nodes - 1] = u_right;

        // Thomas Algorithm for tridiagonal system K * u = F
        let mut c_prime = vec![0.0; n_nodes - 1];
        let mut d_prime = vec![0.0; n_nodes];

        c_prime[0] = off_diag[0] / diag[0];
        d_prime[0] = load[0] / diag[0];

        for i in 1..(n_nodes - 1) {
            let denom = diag[i] - off_diag[i - 1] * c_prime[i - 1];
            c_prime[i] = off_diag[i] / denom;
            d_prime[i] = (load[i] - off_diag[i - 1] * d_prime[i - 1]) / denom;
        }

        let denom_last = diag[n_nodes - 1] - off_diag[n_nodes - 2] * c_prime[n_nodes - 2];
        d_prime[n_nodes - 1] =
            (load[n_nodes - 1] - off_diag[n_nodes - 2] * d_prime[n_nodes - 2]) / denom_last;

        let mut u_sol = vec![0.0; n_nodes];
        u_sol[n_nodes - 1] = d_prime[n_nodes - 1];

        for i in (0..(n_nodes - 1)).rev() {
            u_sol[i] = d_prime[i] - c_prime[i] * u_sol[i + 1];
        }

        Ok((x_nodes, u_sol))
    }

    /// Solve the 1D Viscous Burgers PDE via Method of Lines (MOL):
    ///
    /// $$\frac{\partial u}{\partial t} + u \frac{\partial u}{\partial x} = \nu \frac{\partial^2 u}{\partial x^2}$$
    ///
    /// Uses an entropy-satisfying Engquist-Osher conservative convective flux paired with
    /// central 2nd-order diffusion and delegates temporal integration to `NumericalOdeSolver`
    /// (`Rk4`, `DormandPrince54`, `Tsitouras54`, `RadauIIA5`, or `SelfOptimizingMeta`).
    #[allow(clippy::too_many_arguments)]
    pub fn solve_burgers_1d_mol(
        nu: f64,
        x_span: (f64, f64),
        t_span: (f64, f64),
        n_spatial_points: usize,
        u0_func: impl Fn(f64) -> f64,
        bc_left: BoundaryCondition,
        bc_right: BoundaryCondition,
        ode_config: &NumericalOdeConfig,
    ) -> AlgebraResult<PdeGridSolution> {
        if n_spatial_points < 3 {
            return Err(AlgebraError::EvaluationError(
                "Spatial grid points must be at least 3".into(),
            ));
        }
        if nu < 0.0 || !nu.is_finite() {
            return Err(AlgebraError::EvaluationError(
                "Kinematic viscosity nu must be finite and non-negative".into(),
            ));
        }

        let (x0, xend) = x_span;
        let dx = (xend - x0) / ((n_spatial_points - 1) as f64);
        let dx_sq = dx * dx;

        let mut x_grid = Vec::with_capacity(n_spatial_points);
        let mut u0 = Vec::with_capacity(n_spatial_points);
        for i in 0..n_spatial_points {
            let xi = x0 + (i as f64) * dx;
            x_grid.push(xi);
            u0.push(u0_func(xi));
        }

        if let BoundaryCondition::Dirichlet(val) = bc_left {
            u0[0] = val;
        }
        if let BoundaryCondition::Dirichlet(val) = bc_right {
            u0[n_spatial_points - 1] = val;
        }

        let f_plus = |v: f64| if v > 0.0 { 0.5 * v * v } else { 0.0 };
        let f_minus = |v: f64| if v < 0.0 { 0.5 * v * v } else { 0.0 };

        let ode_rhs = move |_t: f64, u_state: &[f64]| -> Vec<f64> {
            let n = u_state.len();
            let mut du_dt = vec![0.0; n];

            for i in 1..(n - 1) {
                let flux_r = f_plus(u_state[i]) + f_minus(u_state[i + 1]);
                let flux_l = f_plus(u_state[i - 1]) + f_minus(u_state[i]);
                let conv = (flux_r - flux_l) / dx;
                let diff = nu * (u_state[i + 1] - 2.0 * u_state[i] + u_state[i - 1]) / dx_sq;
                du_dt[i] = diff - conv;
            }

            match bc_left {
                BoundaryCondition::Dirichlet(_) => du_dt[0] = 0.0,
                BoundaryCondition::Neumann(flux) => {
                    let diff = nu * 2.0 * (u_state[1] - u_state[0] - dx * flux) / dx_sq;
                    let conv = u_state[0] * flux;
                    du_dt[0] = diff - conv;
                }
            }

            match bc_right {
                BoundaryCondition::Dirichlet(_) => du_dt[n - 1] = 0.0,
                BoundaryCondition::Neumann(flux) => {
                    let diff = nu * 2.0 * (u_state[n - 2] - u_state[n - 1] + dx * flux) / dx_sq;
                    let conv = u_state[n - 1] * flux;
                    du_dt[n - 1] = diff - conv;
                }
            }

            du_dt
        };

        let traj = NumericalOdeSolver::solve(ode_rhs, t_span, &u0, ode_config)?;

        Ok(PdeGridSolution {
            x: x_grid,
            t: traj.t,
            u: traj.y,
            scheme_name: format!("Burgers 1D Method of Lines ({})", traj.method_used),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ode::numerical::NumericalOdeMethod;

    #[test]
    fn test_diffusion_mol_with_self_optimizing_meta_and_tsit5() {
        let alpha = 0.05;
        let t_end = 0.25;
        let n_pts = 15;
        let pi = std::f64::consts::PI;

        for method in [
            NumericalOdeMethod::Tsitouras54,
            NumericalOdeMethod::SelfOptimizingMeta,
        ] {
            let cfg = NumericalOdeConfig {
                method,
                rtol: 1e-4,
                atol: 1e-6,
                max_step: 0.002,
                ..Default::default()
            };
            let sol = NumericalPdeSolver::solve_diffusion_reaction_mol(
                alpha,
                |_u, _x, _t| 0.0,
                (0.0, 1.0),
                (0.0, t_end),
                n_pts,
                |x| (pi * x).sin(),
                BoundaryCondition::Dirichlet(0.0),
                BoundaryCondition::Dirichlet(0.0),
                &cfg,
            )
            .expect("MOL heat solve");

            let final_u = sol.u.last().expect("final slice");
            let mid_idx = n_pts / 2;
            let expected_mid = (-alpha * pi * pi * t_end).exp() * (pi * sol.x[mid_idx]).sin();
            assert!(
                (final_u[mid_idx] - expected_mid).abs() < 1e-2,
                "{}: mid u={} expected={}",
                sol.scheme_name,
                final_u[mid_idx],
                expected_mid
            );
            assert!(final_u[0].abs() < 1e-12);
            assert!(final_u[n_pts - 1].abs() < 1e-12);
        }
    }

    #[test]
    fn test_burgers_1d_mol_viscous_smoothing() {
        let pi = std::f64::consts::PI;
        let cfg = NumericalOdeConfig {
            method: NumericalOdeMethod::SelfOptimizingMeta,
            rtol: 1e-4,
            atol: 1e-6,
            max_step: 0.002,
            ..Default::default()
        };
        let sol = NumericalPdeSolver::solve_burgers_1d_mol(
            0.05,
            (0.0, 1.0),
            (0.0, 0.2),
            21,
            |x| (pi * x).sin(),
            BoundaryCondition::Dirichlet(0.0),
            BoundaryCondition::Dirichlet(0.0),
            &cfg,
        )
        .expect("Burgers MOL solve");

        assert!(sol.scheme_name.starts_with("Burgers 1D Method of Lines"));
        let u0 = sol.u.first().expect("initial slice");
        let u_end = sol.u.last().expect("final slice");
        let energy0: f64 = u0.iter().map(|v| v * v).sum();
        let energy_end: f64 = u_end.iter().map(|v| v * v).sum();
        assert!(energy_end < energy0, "expected viscous energy decay");
        assert!(u_end.iter().all(|v| v.is_finite() && *v >= -1e-9));
        assert!(u_end[0].abs() < 1e-12 && u_end[20].abs() < 1e-12);
    }
}
