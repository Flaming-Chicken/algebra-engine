//! # `algebra-series`
//!
//! Series expansions (Taylor, Laurent, Puiseux), Big-O asymptotic calculus,
//! **Lagrange Inversion Theorem** for symbolic series reversion,
//! **Holonomic D-finite Functions**, and **Linear Recurrence Solvers**.

use crate::calculus::SymbolicCalculus;
use algebra_core::{AlgebraError, AlgebraResult, ExprGraph, ExprId, ExprKind, SymbolId};

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
        use num_rational::BigRational;
        use num_traits::{One, Zero};
        let fail = |why: &str| AlgebraError::EvaluationError(format!("lagrange inversion: {why}"));
        let order = order as usize;
        let out_name = self.symbols.resolve(out_var).unwrap_or_default();
        let y = self.symbol(&out_name);
        if order == 0 {
            return Ok(self.integer(0));
        }

        // Taylor coefficients of f(w) about 0, which must be exact rationals with f(0) = 0.
        let coeffs: Vec<BigRational> =
            crate::calculus::taylor_coeffs_vec(self, expr, wrt, self.integer(0), order)
                .iter()
                .map(|&c| crate::calculus::exact_rational(self, c))
                .collect::<Option<_>>()
                .ok_or_else(|| fail("the Taylor coefficients of f are not all exact rationals"))?;
        if !coeffs[0].is_zero() {
            return Err(fail("f(0) must be 0"));
        }
        if coeffs.get(1).is_none_or(|c| c.is_zero()) {
            return Err(fail("f'(0) must be non-zero for f to be invertible near 0"));
        }

        // f(w) = w g(w), so g_j = f_{j+1}; w = sum_n y^n/n [w^(n-1)] g(w)^(-n).
        let len = order; // g is needed through degree order - 1
        let g: Vec<BigRational> = (0..len).map(|j| coeffs[j + 1].clone()).collect();
        let mul = |a: &[BigRational], b: &[BigRational]| -> Vec<BigRational> {
            let mut out = vec![BigRational::zero(); len];
            for (i, x) in a.iter().enumerate() {
                for (j, z) in b.iter().enumerate() {
                    if i + j < len {
                        out[i + j] += x * z;
                    }
                }
            }
            out
        };
        // Power-series inverse of g (g_0 != 0).
        let mut inverse = vec![BigRational::zero(); len];
        inverse[0] = BigRational::one() / g[0].clone();
        for k in 1..len {
            let mut acc = BigRational::zero();
            for j in 1..=k {
                acc += &g[j] * &inverse[k - j];
            }
            inverse[k] = -acc / g[0].clone();
        }

        let mut terms = Vec::new();
        let mut power = inverse.clone(); // g^(-1)
        for n in 1..=order {
            if n > 1 {
                power = mul(&power, &inverse);
            }
            let coefficient = power[n - 1].clone() / BigRational::from_integer((n as i64).into());
            if coefficient.is_zero() {
                continue;
            }
            let c = crate::calculus::rational_expr(self, &coefficient)?;
            terms.push(self.mul([c, self.pow(y, self.integer(n as i64))]));
        }
        Ok(if terms.is_empty() {
            self.integer(0)
        } else {
            let sum = self.add(terms);
            crate::simplify::Simplifier::simplify(self, sum).unwrap_or(sum)
        })
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
            crate::calculus::SERIES_BUDGET_EXCEEDED.with(|f| f.set(false));
            let series = self.taylor_series(expr, wrt, point, regular_order);
            if crate::calculus::SERIES_BUDGET_EXCEEDED.with(|f| f.replace(false)) {
                return Err(AlgebraError::EvaluationError(format!(
                    "series: the derivatives of this expression grow too fast for order {regular_order} (the differentiation method is exponential here); try a lower order"
                )));
            }
            if has_singular_term(self, series) {
                return Err(AlgebraError::EvaluationError(
                    "series: the function is not analytic at this point (terms such as ln(0) or 1/0 appear), so it has no Taylor series there"
                        .into(),
                ));
            }
            return Ok(series);
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
        expr: ExprId,
        wrt: SymbolId,
        point: ExprId,
        denom_q: u32,
        order: u32,
    ) -> AlgebraResult<ExprId> {
        match denom_q {
            // Integer exponents only: a Puiseux series is then the Taylor series.
            1 => Ok(self.taylor_series(expr, wrt, point, order)),
            0 => Err(AlgebraError::EvaluationError(
                "puiseux: the exponent denominator must be at least 1".into(),
            )),
            q => Err(AlgebraError::EvaluationError(format!(
                "puiseux: fractional exponents (denominator {q}) are not implemented; only denominator 1 (the Taylor series) is"
            ))),
        }
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

        let constant = |v: f64| {
            if v.fract() == 0.0 && v.abs() < 1e15 {
                graph.integer(v as i64)
            } else {
                graph.float(v)
            }
        };
        let simplify = |e: ExprId| crate::simplify::Simplifier::simplify(graph, e).unwrap_or(e);
        let unsupported = |why: &str| AlgebraError::EvaluationError(format!("recurrence: {why}"));

        match coeffs.len() {
            0 => Err(unsupported("no coefficients given")),
            1 => {
                // a_n = c1 a_{n-1}  =>  a_n = a_0 c1^n
                let a0 = constant(*initial_conditions.first().unwrap_or(&1.0));
                Ok(graph.mul([a0, graph.pow(constant(coeffs[0]), n)]))
            }
            2 => {
                // a_n = c1 a_{n-1} + c2 a_{n-2}: characteristic polynomial r^2 = c1 r + c2.
                let (c1, c2) = (coeffs[0], coeffs[1]);
                if initial_conditions.len() < 2 {
                    return Err(unsupported(
                        "a second-order recurrence needs two initial values",
                    ));
                }
                if c2 == 0.0 {
                    return Err(unsupported("degenerate recurrence (c2 = 0)"));
                }
                let (a0, a1) = (initial_conditions[0], initial_conditions[1]);
                let discriminant = c1 * c1 + 4.0 * c2;
                if discriminant == 0.0 {
                    // Repeated root r = c1/2: a_n = (a_0 + (a_1/r - a_0) n) r^n.
                    let r = c1 / 2.0;
                    let slope = constant(a1 / r - a0);
                    let linear = graph.add([constant(a0), graph.mul([slope, n])]);
                    return Ok(simplify(graph.mul([linear, graph.pow(constant(r), n)])));
                }
                if discriminant < 0.0 {
                    return Err(unsupported(
                        "complex characteristic roots are not supported",
                    ));
                }
                // Distinct real roots r1, r2 = (c1 +- sqrt(D)) / 2.
                let root = discriminant.sqrt();
                let sqrt_d = if (root.round() * root.round() - discriminant).abs() < 1e-9 {
                    constant(root.round())
                } else {
                    graph.function("sqrt", [constant(discriminant)])
                };
                let two = graph.integer(2);
                let r1 = graph.div(graph.add([constant(c1), sqrt_d]), two);
                let r2 = graph.div(graph.sub(constant(c1), sqrt_d), two);
                let gap = graph.sub(r1, r2);
                // a_n = A r1^n + B r2^n with A + B = a_0 and A r1 + B r2 = a_1.
                let big_a = graph.div(graph.sub(constant(a1), graph.mul([constant(a0), r2])), gap);
                let big_b = graph.div(graph.sub(graph.mul([constant(a0), r1]), constant(a1)), gap);
                let sum = graph.add([
                    graph.mul([big_a, graph.pow(r1, n)]),
                    graph.mul([big_b, graph.pow(r2, n)]),
                ]);
                Ok(simplify(sum))
            }
            order => Err(unsupported(&format!(
                "order {order} is not supported (orders 1 and 2 are)"
            ))),
        }
    }
}

/// Whether the expression contains a division by the literal zero or `ln(0)`: a sign that a Taylor coefficient was
/// computed at a singular point.
fn has_singular_term(graph: &ExprGraph, root: ExprId) -> bool {
    let zero = graph.integer(0);
    let mut seen = std::collections::HashSet::new();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        let node = graph.get(id);
        match &node.kind {
            ExprKind::Div(_, d) if *d == zero => return true,
            ExprKind::Function { name, args }
                if args.len() == 1
                    && args[0] == zero
                    && matches!(graph.symbols.resolve(*name).as_deref(), Some("ln" | "log")) =>
            {
                return true;
            }
            _ => {}
        }
        stack.extend(node.children());
    }
    false
}
