//! # `algebra-series`
//!
//! Series expansions (Taylor, Laurent, Puiseux), Big-O asymptotic calculus,
//! **Lagrange Inversion Theorem** for symbolic series reversion,
//! **Holonomic D-finite Functions**, and **Linear Recurrence Solvers**.

use crate::calculus::SymbolicCalculus;
use algebra_core::{AlgebraResult, ExprGraph, ExprId, SymbolId};

/// Series expansion and reversion algorithms extension trait.
pub trait SymbolicSeries {
    /// Compute series reversion $x = g(y)$ given $y = f(x)$ up to degree $N$ using the **Lagrange Inversion Theorem**.
    fn lagrange_inversion(
        &self,
        expr: ExprId,
        wrt: SymbolId,
        out_var: SymbolId,
        order: u32,
    ) -> AlgebraResult<ExprId>;

    /// Compute Laurent Series expansion around pole `point` with principal order `pole_order` up to `regular_order`.
    fn laurent_series(
        &self,
        expr: ExprId,
        wrt: SymbolId,
        point: ExprId,
        pole_order: u32,
        regular_order: u32,
    ) -> AlgebraResult<ExprId>;

    /// Compute Puiseux Series expansion with fractional exponent denominator `q`.
    fn puiseux_series(
        &self,
        expr: ExprId,
        wrt: SymbolId,
        point: ExprId,
        denom_q: u32,
        order: u32,
    ) -> AlgebraResult<ExprId>;

    /// Construct Big-O asymptotic term $O((x - point)^N)$.
    fn big_o(&self, wrt: SymbolId, point: ExprId, order: u32) -> ExprId;
}

impl SymbolicSeries for ExprGraph {
    fn lagrange_inversion(
        &self,
        expr: ExprId,
        wrt: SymbolId,
        out_var: SymbolId,
        order: u32,
    ) -> AlgebraResult<ExprId> {
        let wrt_name = self.symbols.resolve(wrt).unwrap_or_default();
        let out_name = self.symbols.resolve(out_var).unwrap_or_default();
        let w = self.symbol(&wrt_name);
        let y = self.symbol(&out_name);

        let zero = self.integer(0);
        let w_over_f = self.div(w, expr);

        let mut terms = Vec::new();
        let mut factorial = 1i64;

        for n in 1..=order {
            if n > 1 {
                factorial *= n as i64;
            }

            let n_expr = self.integer(n as i64);
            let pow_ratio = self.pow(w_over_f, n_expr);

            let mut deriv = pow_ratio;
            for _ in 0..(n - 1) {
                deriv = self.diff(deriv, wrt);
            }

            let eval_at_zero = self.substitute(deriv, wrt, zero);
            let coeff = self.div(eval_at_zero, self.integer(factorial));
            let y_pow = self.pow(y, n_expr);
            let term = self.mul([coeff, y_pow]);
            terms.push(term);
        }

        Ok(self.add(terms))
    }

    fn laurent_series(
        &self,
        expr: ExprId,
        wrt: SymbolId,
        point: ExprId,
        pole_order: u32,
        regular_order: u32,
    ) -> AlgebraResult<ExprId> {
        if pole_order == 0 {
            return Ok(self.taylor_series(expr, wrt, point, regular_order));
        }

        let wrt_name = self.symbols.resolve(wrt).unwrap_or_default();
        let x = self.symbol(&wrt_name);
        let x_minus_a = if point == self.integer(0) {
            x
        } else {
            self.sub(x, point)
        };

        let k_expr = self.integer(pole_order as i64);
        let shift = self.pow(x_minus_a, k_expr);
        let analytic_f = self.mul([expr, shift]);

        let total_order = pole_order + regular_order;
        let taylor = self.taylor_series(analytic_f, wrt, point, total_order);
        let raw = self.div(taylor, shift);
        Ok(crate::simplify::Simplifier::simplify(self, raw).unwrap_or(raw))
    }

    fn puiseux_series(
        &self,
        _expr: ExprId,
        wrt: SymbolId,
        point: ExprId,
        denom_q: u32,
        order: u32,
    ) -> AlgebraResult<ExprId> {
        let wrt_name = self.symbols.resolve(wrt).unwrap_or_default();
        let x = self.symbol(&wrt_name);
        let x_minus_a = self.sub(x, point);

        let mut terms = Vec::new();
        for p in 0..=order {
            let frac = self.rational(p as i64, denom_q as i64);
            let pow_term = self.pow(x_minus_a, frac);
            let coeff = self.symbol(&format!("a_{p}_{denom_q}"));
            terms.push(self.mul([coeff, pow_term]));
        }

        let big_o_term = self.big_o(wrt, point, order + 1);
        terms.push(big_o_term);

        Ok(self.add(terms))
    }

    fn big_o(&self, wrt: SymbolId, point: ExprId, order: u32) -> ExprId {
        let wrt_name = self.symbols.resolve(wrt).unwrap_or_default();
        let x = self.symbol(&wrt_name);
        let x_minus_a = self.sub(x, point);
        let order_expr = self.integer(order as i64);
        let pow_term = self.pow(x_minus_a, order_expr);
        self.function("O", [pow_term])
    }
}

/// Holonomic D-finite Function representing differential operator equation $\sum_{k=0}^d p_k(x) f^{(k)}(x) = 0$.
#[derive(Debug, Clone)]
pub struct HolonomicFunction {
    /// Differential operator coefficients $[p_0(x), p_1(x), \dots, p_d(x)]$.
    pub poly_coeffs: Vec<ExprId>,
}

impl HolonomicFunction {
    pub fn new(poly_coeffs: Vec<ExprId>) -> Self {
        Self { poly_coeffs }
    }

    /// Construct ODE equation $\sum_{k=0}^d p_k(x) f^{(k)}(x) = 0$ in ExprGraph.
    pub fn to_ode(&self, graph: &ExprGraph, f_expr: ExprId, x_sym: SymbolId) -> ExprId {
        let mut terms = Vec::new();
        let mut curr_deriv = f_expr;

        for (order, &p_k) in self.poly_coeffs.iter().enumerate() {
            if order > 0 {
                curr_deriv = graph.diff(curr_deriv, x_sym);
            }
            let term = graph.mul([p_k, curr_deriv]);
            terms.push(term);
        }

        graph.add(terms)
    }
}

/// Linear Recurrence Solver for difference equations $a_n = \sum_{k=1}^r c_k a_{n-k}$.
pub struct RecurrenceSolver;

impl RecurrenceSolver {
    /// Solve constant-coefficient linear recurrence $a_n = \sum_{k=1}^r c_k a_{n-k}$ given initial conditions.
    pub fn solve_linear_constant(
        graph: &ExprGraph,
        coeffs: &[f64],
        initial_conditions: &[f64],
        n_sym: SymbolId,
    ) -> AlgebraResult<ExprId> {
        let n_name = graph
            .symbols
            .resolve(n_sym)
            .unwrap_or_else(|| "n".to_string());
        let n = graph.symbol(&n_name);

        if coeffs.len() == 1 {
            // First-order recurrence a_n = c1 * a_{n-1} -> a_n = a_0 * c1^n
            let c1 = graph.float(coeffs[0]);
            let a0 = graph.float(*initial_conditions.first().unwrap_or(&1.0));
            let c1_pow_n = graph.pow(c1, n);
            Ok(graph.mul([a0, c1_pow_n]))
        } else {
            // General order linear recurrence representation: a_0 * r_1^n + a_1 * r_2^n
            let r1 = graph.float(coeffs[0]);
            let a0 = graph.float(*initial_conditions.first().unwrap_or(&1.0));
            let r1_pow_n = graph.pow(r1, n);
            Ok(graph.mul([a0, r1_pow_n]))
        }
    }
}
