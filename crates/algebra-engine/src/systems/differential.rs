//! # `algebra_engine::systems::differential`
//!
//! Coupled Differential Equations Systems Solvers.
//!
//! Provides:
//! - **Linear Autonomous Systems**: $\mathbf{y}' = \mathbf{A}\mathbf{y} + \mathbf{g}(t)$ via Matrix Exponential $\mathbf{\Phi}(t) = e^{\mathbf{A}t}$.
//! - **Reduction of Order**: Converts $n$-th order ODEs $y^{(n)} = F(t, y, y', \dots, y^{(n-1)})$
//!   into an $n$-dimensional first-order state vector system $\mathbf{x}' = \mathbf{f}(t, \mathbf{x})$.

use crate::calculus::SymbolicCalculus;
use crate::matrix::SymbolicMatrix;
use crate::ode::symbolic::SymbolicOdeSolver;
use crate::simplify::Simplifier;
use algebra_core::error::{AlgebraError, AlgebraResult};
use algebra_core::{ExprGraph, ExprId, SymbolId};

/// Solver for coupled systems of ordinary differential equations.
pub struct DifferentialSystemSolver;

impl DifferentialSystemSolver {
    /// Solves linear autonomous $2 \times 2$ differential system $\mathbf{y}' = \mathbf{A}\mathbf{y} + \mathbf{g}(t)$
    /// with initial condition $\mathbf{y}(0) = \mathbf{y}_0$.
    pub fn solve_linear_system_2x2(
        graph: &ExprGraph,
        a_matrix: &SymbolicMatrix,
        g_forcing: Option<&[ExprId]>,
        y0_initial: Option<&[ExprId]>,
        t: SymbolId,
    ) -> AlgebraResult<Vec<ExprId>> {
        if a_matrix.rows != 2 || a_matrix.cols != 2 {
            return Err(AlgebraError::DimensionMismatch(format!(
                "Expected 2x2 matrix, found {}x{}",
                a_matrix.rows, a_matrix.cols
            )));
        }

        // 1. Compute fundamental matrix Phi(t) = exp(A * t)
        let phi = SymbolicOdeSolver::solve_linear_system_2x2(graph, a_matrix, t)?;

        let c1 = graph.symbol("C1");
        let c2 = graph.symbol("C2");

        let init_y1 = y0_initial.map(|y| y[0]).unwrap_or(c1);
        let init_y2 = y0_initial.map(|y| y[1]).unwrap_or(c2);

        // Homogeneous solution y_h(t) = Phi(t) * y(0)
        let p00 = phi.get(0, 0);
        let p01 = phi.get(0, 1);
        let p10 = phi.get(1, 0);
        let p11 = phi.get(1, 1);

        let yh1 = graph.add([graph.mul([p00, init_y1]), graph.mul([p01, init_y2])]);
        let yh2 = graph.add([graph.mul([p10, init_y1]), graph.mul([p11, init_y2])]);

        let yh1_simp = Simplifier::simplify(graph, yh1).unwrap_or(yh1);
        let yh2_simp = Simplifier::simplify(graph, yh2).unwrap_or(yh2);

        // If forcing vector g(t) is present, add particular solution (variation of parameters)
        if let Some(g) = g_forcing {
            if g.len() != 2 {
                return Err(AlgebraError::DimensionMismatch(
                    "Forcing vector must have length 2".into(),
                ));
            }
            // Particular solution by variation of parameters (Duhamel):
            //   y_p(t) = integral_0^t Phi(t - tau) g(tau) d tau.
            let t_name = graph.symbols.resolve(t).unwrap_or_else(|| "t".to_string());
            let t_expr = graph.symbol(&t_name);
            let tau_id = graph.symbols.get_or_intern("tau");
            let tau = graph.symbol("tau");
            let shifted = graph.sub(t_expr, tau);
            // Phi evaluated at (t - tau), forcing evaluated at tau.
            let phi_shift = |entry: ExprId| graph.substitute(entry, t, shifted);
            let g_tau = [
                graph.substitute(g[0], t, tau),
                graph.substitute(g[1], t, tau),
            ];
            let integrand = [
                graph.add([
                    graph.mul([phi_shift(p00), g_tau[0]]),
                    graph.mul([phi_shift(p01), g_tau[1]]),
                ]),
                graph.add([
                    graph.mul([phi_shift(p10), g_tau[0]]),
                    graph.mul([phi_shift(p11), g_tau[1]]),
                ]),
            ];
            let definite = |f: ExprId| -> ExprId {
                let f = Simplifier::simplify(graph, f).unwrap_or(f);
                match graph.integrate(f, tau_id) {
                    Ok(anti) => graph.sub(
                        graph.substitute(anti, tau_id, t_expr),
                        graph.substitute(anti, tau_id, graph.integer(0)),
                    ),
                    // No closed form: keep the convolution integral rather than drop it.
                    Err(_) => graph.integral(f, "tau", Some(graph.integer(0)), Some(t_expr)),
                }
            };

            let total1 = graph.add([yh1_simp, definite(integrand[0])]);
            let total2 = graph.add([yh2_simp, definite(integrand[1])]);

            Ok(vec![
                Simplifier::simplify(graph, total1).unwrap_or(total1),
                Simplifier::simplify(graph, total2).unwrap_or(total2),
            ])
        } else {
            Ok(vec![yh1_simp, yh2_simp])
        }
    }

    /// Reduces an $n$-th order ODE $y^{(n)} = F(t, y, y', \dots, y^{(n-1)})$ to a system of $n$ 1st-order ODEs.
    ///
    /// Returns the state derivative vector $\left[ x_1' = x_2, \, x_2' = x_3, \dots, \, x_n' = F(t, x_1, \dots, x_n) \right]$.
    pub fn reduce_nth_order_to_system(
        graph: &ExprGraph,
        order: usize,
        f_highest_derivative: ExprId,
    ) -> AlgebraResult<Vec<ExprId>> {
        if order == 0 {
            return Err(AlgebraError::EvaluationError(
                "ODE order must be at least 1".into(),
            ));
        }

        let mut derivatives = Vec::with_capacity(order);

        // x_i' = x_{i+1} for i in 1..(order-1)
        for i in 1..order {
            let next_state = graph.symbol(&format!("x{}", i + 1));
            derivatives.push(next_state);
        }

        // x_n' = F(t, x_1, ..., x_n)
        derivatives.push(f_highest_derivative);

        Ok(derivatives)
    }
}
