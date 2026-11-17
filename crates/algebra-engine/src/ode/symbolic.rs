//! # `algebra_engine::ode::symbolic`
//!
//! Exact Symbolic Ordinary Differential Equation (ODE) Solvers.
//!
//! Implements classification and exact analytical solvers for:
//! - **1st-Order Linear ODEs**: $y'(x) + P(x)y(x) = Q(x)$ via Integrating Factor $\mu(x) = e^{\int P(x)dx}$.
//! - **Separable ODEs**: $\frac{dy}{dx} = g(x)h(y) \implies \int \frac{1}{h(y)}dy = \int g(x)dx + C$.
//! - **Exact Differential Equations**: $M(x, y)dx + N(x, y)dy = 0$ where $\frac{\partial M}{\partial y} = \frac{\partial N}{\partial x}$.
//! - **Bernoulli Equations**: $y' + P(x)y = Q(x)y^n \implies v = y^{1-n}$ linearization.
//! - **2nd-Order Linear Constant Coefficient ODEs**: $a y'' + b y' + c y = f(x)$ with characteristic polynomial roots and Wronskian Variation of Parameters.
//! - **Cauchy-Euler Equations**: $a x^2 y'' + b x y' + c y = 0$ via power ansatz $y = x^r$.
//! - **Systems of Linear ODEs**: $\mathbf{y}' = \mathbf{A}\mathbf{y}$ via Matrix Exponential $e^{\mathbf{A}t}$.
//! - **Frobenius Series Method**: [`SymbolicOdeSolver::frobenius_series_at_zero`] solves
//!   $x^2 y'' + x P(x) y' + Q(x) y = 0$ at the regular singular point $x = 0$ when $P$ and $Q$ have
//!   rational Taylor coefficients and the indicial roots are rational (see its documentation for
//!   the cases it declines).

use crate::calculus::SymbolicCalculus;
use crate::matrix::SymbolicMatrix;
use crate::simplify::Simplifier;
use algebra_core::{AlgebraError, AlgebraResult, ExprGraph, ExprId, SymbolId};

/// Classification category of an Ordinary Differential Equation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OdeType {
    /// First order separable: dy/dx = f(x)*g(y)
    Separable,
    /// First order linear: y' + P(x)y = Q(x)
    FirstOrderLinear,
    /// Exact equation: M(x,y)dx + N(x,y)dy = 0
    Exact,
    /// Bernoulli non-linear equation: y' + P(x)y = Q(x)y^n
    Bernoulli { n: i64 },
    /// Second order linear with constant coefficients: a*y'' + b*y' + c*y = f(x)
    SecondOrderLinearConstCoeff,
    /// Cauchy-Euler equation: a*x^2*y'' + b*x*y' + c*y = 0
    CauchyEuler,
    /// System of first-order linear equations: y' = A*y
    LinearSystem,
}

/// Symbolic ODE Solver Suite.
pub struct SymbolicOdeSolver;

impl SymbolicOdeSolver {
    /// Frobenius series solutions of $x^2 y'' + x P(x) y' + Q(x) y = 0$ about the regular singular
    /// point $x = 0$, where $P$ and $Q$ are analytic there.
    ///
    /// The indicial equation is $r(r-1) + P(0) r + Q(0) = 0$. For each root $r$ the series
    /// $y = x^r \sum_{n \ge 0} a_n x^n$ has $a_0 = 1$ and
    /// $$a_n = -\frac{\sum_{k<n} a_k \left((r+k) P_{n-k} + Q_{n-k}\right)}{F(r+n)},\quad
    /// F(s) = s(s-1) + P_0 s + Q_0,$$
    /// with $P_j, Q_j$ the Taylor coefficients of $P, Q$. Returns the truncated series through
    /// order `order`.
    ///
    /// The larger root always gives a solution. The smaller root gives a second one unless the
    /// roots differ by an integer and the recurrence is resonant with a non-zero right-hand side,
    /// in which case the second solution contains a logarithm and only the first is returned. Errors when the coefficients of $P$ or $Q$ are not exact
    /// rationals or when the indicial roots are complex or irrational.
    pub fn frobenius_series_at_zero(
        graph: &ExprGraph,
        p: ExprId,
        q: ExprId,
        x: SymbolId,
        order: usize,
    ) -> AlgebraResult<Vec<ExprId>> {
        use num_bigint::BigInt;
        use num_rational::BigRational;
        use num_traits::{One, Signed, ToPrimitive, Zero};
        let fail = |why: &str| AlgebraError::EvaluationError(format!("frobenius: {why}"));
        let zero = graph.integer(0);
        let coefficients = |e: ExprId| -> AlgebraResult<Vec<BigRational>> {
            crate::calculus::taylor_coeffs_vec(graph, e, x, zero, order)
                .iter()
                .map(|&c| crate::calculus::exact_rational(graph, c))
                .collect::<Option<_>>()
                .ok_or_else(|| fail("P and Q need exact rational Taylor coefficients at 0"))
        };
        let (p_coeffs, q_coeffs) = (coefficients(p)?, coefficients(q)?);
        let (p0, q0) = (p_coeffs[0].clone(), q_coeffs[0].clone());

        // Indicial roots: r^2 + (P0 - 1) r + Q0 = 0, which must have rational roots.
        let b = &p0 - BigRational::one();
        let discriminant = &b * &b - BigRational::from_integer(BigInt::from(4)) * &q0;
        if discriminant.is_negative() {
            return Err(fail("the indicial roots are complex"));
        }
        let rational_sqrt = |v: &BigRational| -> Option<BigRational> {
            let (n, d) = (v.numer().sqrt(), v.denom().sqrt());
            (&n * &n == *v.numer() && &d * &d == *v.denom()).then(|| BigRational::new(n, d))
        };
        let root = rational_sqrt(&discriminant)
            .ok_or_else(|| fail("the indicial roots are irrational"))?;
        let two = BigRational::from_integer(BigInt::from(2));
        let larger = (-&b + &root) / &two;
        let smaller = (-&b - &root) / &two;

        let indicial = |s: &BigRational| s * (s - BigRational::one()) + &p0 * s + &q0;
        let series = |r: &BigRational| -> AlgebraResult<Option<ExprId>> {
            let mut a = vec![BigRational::one()];
            for n in 1..=order {
                let mut sum = BigRational::zero();
                for (k, ak) in a.iter().enumerate().take(n) {
                    let shift = r + BigRational::from_integer(BigInt::from(k));
                    sum += ak * (&shift * &p_coeffs[n - k] + &q_coeffs[n - k]);
                }
                let denominator = indicial(&(r + BigRational::from_integer(BigInt::from(n))));
                if denominator.is_zero() {
                    // Resonance: the recurrence is 0 * a_n = -sum. A zero right-hand side leaves
                    // a_n free (take 0); otherwise the solution needs a logarithm.
                    if sum.is_zero() {
                        a.push(BigRational::zero());
                        continue;
                    }
                    return Ok(None);
                }
                a.push(-sum / denominator);
            }
            let mut terms = Vec::new();
            for (n, c) in a.iter().enumerate() {
                if c.is_zero() {
                    continue;
                }
                let value = crate::calculus::rational_expr(graph, c)?;
                let power = graph.pow(
                    graph.symbol(&graph.symbols.resolve(x).unwrap_or_default()),
                    graph.integer(n as i64),
                );
                terms.push(if n == 0 {
                    value
                } else {
                    graph.mul([value, power])
                });
            }
            let polynomial = graph.add(terms);
            let exponent = match (r.numer().to_i64(), r.denom().to_i64()) {
                (Some(n), Some(d)) => graph.rational(n, d),
                _ => return Err(fail("exponent too large")),
            };
            let x_expr = graph.symbol(&graph.symbols.resolve(x).unwrap_or_default());
            let leading = graph.pow(x_expr, exponent);
            Ok(Some(graph.mul([leading, polynomial])))
        };

        let mut solutions = Vec::new();
        solutions.extend(series(&larger)?);
        if larger != smaller {
            solutions.extend(series(&smaller)?);
        }
        Ok(solutions)
    }

    /// Solve 1st-Order Linear ODE $y'(x) + P(x) y(x) = Q(x)$ for $y(x)$.
    ///
    /// $$y(x) = e^{-\int P(x)dx} \left( \int e^{\int P(x)dx} Q(x) dx + C_1 \right)$$
    pub fn solve_first_order_linear(
        graph: &ExprGraph,
        p: ExprId,
        q: ExprId,
        x: SymbolId,
    ) -> AlgebraResult<ExprId> {
        let int_p = graph.integrate(p, x)?;
        let mu = graph.function("exp", [int_p]);
        let neg_int_p = graph.neg(int_p);
        let inv_mu = graph.function("exp", [neg_int_p]);

        let mu_q = graph.mul([mu, q]);
        let int_mu_q = graph.integrate(mu_q, x)?;

        let c1 = graph.symbol("C1");
        let sum_with_c1 = graph.add([int_mu_q, c1]);
        let sol = graph.mul([inv_mu, sum_with_c1]);

        Ok(Simplifier::simplify(graph, sol).unwrap_or(sol))
    }

    /// Solve Bernoulli Equation $y'(x) + P(x) y(x) = Q(x) y^n(x)$ with substitution $v(x) = y^{1-n}(x)$.
    ///
    /// Linearized equation: $v'(x) + (1-n)P(x)v(x) = (1-n)Q(x)$.
    pub fn solve_bernoulli(
        graph: &ExprGraph,
        p: ExprId,
        q: ExprId,
        n: i64,
        x: SymbolId,
    ) -> AlgebraResult<ExprId> {
        if n == 0 {
            return Self::solve_first_order_linear(graph, p, q, x);
        }
        if n == 1 {
            let zero = graph.integer(0);
            let p_minus_q = graph.sub(p, q);
            return Self::solve_first_order_linear(graph, p_minus_q, zero, x);
        }

        let one_minus_n = graph.integer(1 - n);
        let p_v = graph.mul([one_minus_n, p]);
        let q_v = graph.mul([one_minus_n, q]);

        let v_sol = Self::solve_first_order_linear(graph, p_v, q_v, x)?;

        // y(x) = v(x)^(1 / (1 - n))
        let one = graph.integer(1);
        let exponent = graph.div(one, one_minus_n);
        let y_sol = graph.pow(v_sol, exponent);

        Ok(Simplifier::simplify(graph, y_sol).unwrap_or(y_sol))
    }

    /// Solve 2nd-Order Linear ODE with Constant Coefficients: $a y''(x) + b y'(x) + c y(x) = 0$.
    ///
    /// Uses characteristic roots $r = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}$:
    /// - Distinct real roots: $y(x) = C_1 e^{r_1 x} + C_2 e^{r_2 x}$
    /// - Repeated real roots: $y(x) = (C_1 + C_2 x) e^{r x}$
    /// - Complex conjugate roots $\alpha \pm i \beta$: $y(x) = e^{\alpha x} (C_1 \cos(\beta x) + C_2 \sin(\beta x))$
    pub fn solve_second_order_const_coeff_homogeneous(
        graph: &ExprGraph,
        a: f64,
        b: f64,
        c: f64,
        x: SymbolId,
    ) -> AlgebraResult<ExprId> {
        if a.abs() < 1e-12 {
            return Err(AlgebraError::EvaluationError(
                "Coefficient 'a' cannot be zero for 2nd order ODE".into(),
            ));
        }

        let disc = b * b - 4.0 * a * c;
        let c1 = graph.symbol("C1");
        let c2 = graph.symbol("C2");
        let var_x = if let Some(s_str) = graph.symbols.resolve(x) {
            graph.symbol(&s_str)
        } else {
            graph.symbol("x")
        };

        let num_node = |v: f64| -> ExprId {
            if (v - v.round()).abs() < 1e-12 && v.abs() < 1e15 {
                graph.integer(v.round() as i64)
            } else {
                graph.float(v)
            }
        };
        let scale_var = |c: f64| -> ExprId {
            if (c - 1.0).abs() < 1e-12 {
                var_x
            } else if (c + 1.0).abs() < 1e-12 {
                graph.neg(var_x)
            } else {
                graph.mul([num_node(c), var_x])
            }
        };

        if disc.abs() < 1e-12 {
            // Repeated root r = -b / (2a)
            let r = -b / (2.0 * a);
            let exp_rx = if r.abs() < 1e-12 {
                graph.integer(1)
            } else {
                graph.function("exp", [scale_var(r)])
            };
            let c2_x = graph.mul([c2, var_x]);
            let inner = graph.add([c1, c2_x]);
            let sol = if r.abs() < 1e-12 {
                inner
            } else {
                graph.mul([inner, exp_rx])
            };
            Ok(Simplifier::simplify(graph, sol).unwrap_or(sol))
        } else if disc > 0.0 {
            // Distinct real roots
            let sqrt_d = disc.sqrt();
            let r1 = (-b + sqrt_d) / (2.0 * a);
            let r2 = (-b - sqrt_d) / (2.0 * a);

            let exp_r1 = if r1.abs() < 1e-12 {
                graph.integer(1)
            } else {
                graph.function("exp", [scale_var(r1)])
            };
            let exp_r2 = if r2.abs() < 1e-12 {
                graph.integer(1)
            } else {
                graph.function("exp", [scale_var(r2)])
            };

            let term1 = if r1.abs() < 1e-12 {
                c1
            } else {
                graph.mul([c1, exp_r1])
            };
            let term2 = if r2.abs() < 1e-12 {
                c2
            } else {
                graph.mul([c2, exp_r2])
            };
            let sol = graph.add([term1, term2]);
            Ok(Simplifier::simplify(graph, sol).unwrap_or(sol))
        } else {
            // Complex conjugate roots alpha +- i*beta
            let alpha = -b / (2.0 * a);
            let beta = (-disc).sqrt() / (2.0 * a);

            let beta_x = scale_var(beta);
            let cos_beta = graph.function("cos", [beta_x]);
            let sin_beta = graph.function("sin", [beta_x]);

            let c1_cos = graph.mul([c1, cos_beta]);
            let c2_sin = graph.mul([c2, sin_beta]);
            let inner = graph.add([c1_cos, c2_sin]);
            let sol = if alpha.abs() < 1e-12 {
                inner
            } else {
                let exp_alpha = graph.function("exp", [scale_var(alpha)]);
                graph.mul([exp_alpha, inner])
            };
            Ok(Simplifier::simplify(graph, sol).unwrap_or(sol))
        }
    }

    /// Solve Cauchy-Euler Equidimensional Equation: $a x^2 y''(x) + b x y'(x) + c y(x) = 0$.
    ///
    /// Characteristic equation: $a r(r - 1) + b r + c = a r^2 + (b - a) r + c = 0$.
    pub fn solve_cauchy_euler(
        graph: &ExprGraph,
        a: f64,
        b: f64,
        c: f64,
        x: SymbolId,
    ) -> AlgebraResult<ExprId> {
        let eff_b = b - a;
        let disc = eff_b * eff_b - 4.0 * a * c;
        let c1 = graph.symbol("C1");
        let c2 = graph.symbol("C2");
        let var_x = if let Some(s_str) = graph.symbols.resolve(x) {
            graph.symbol(&s_str)
        } else {
            graph.symbol("x")
        };

        if disc.abs() < 1e-12 {
            let r = -eff_b / (2.0 * a);
            let r_node = graph.float(r);
            let x_pow_r = graph.pow(var_x, r_node);
            let ln_x = graph.function("ln", [var_x]);
            let c2_ln_x = graph.mul([c2, ln_x]);
            let inner = graph.add([c1, c2_ln_x]);
            let sol = graph.mul([inner, x_pow_r]);
            Ok(Simplifier::simplify(graph, sol).unwrap_or(sol))
        } else if disc > 0.0 {
            let sqrt_d = disc.sqrt();
            let r1 = (-eff_b + sqrt_d) / (2.0 * a);
            let r2 = (-eff_b - sqrt_d) / (2.0 * a);

            let x_pow_r1 = graph.pow(var_x, graph.float(r1));
            let x_pow_r2 = graph.pow(var_x, graph.float(r2));

            let term1 = graph.mul([c1, x_pow_r1]);
            let term2 = graph.mul([c2, x_pow_r2]);
            let sol = graph.add([term1, term2]);
            Ok(Simplifier::simplify(graph, sol).unwrap_or(sol))
        } else {
            let alpha = -eff_b / (2.0 * a);
            let beta = (-disc).sqrt() / (2.0 * a);

            let x_pow_alpha = graph.pow(var_x, graph.float(alpha));
            let ln_x = graph.function("ln", [var_x]);
            let beta_ln_x = graph.mul([graph.float(beta), ln_x]);

            let cos_term = graph.function("cos", [beta_ln_x]);
            let sin_term = graph.function("sin", [beta_ln_x]);

            let c1_cos = graph.mul([c1, cos_term]);
            let c2_sin = graph.mul([c2, sin_term]);
            let inner = graph.add([c1_cos, c2_sin]);
            let sol = graph.mul([x_pow_alpha, inner]);
            Ok(Simplifier::simplify(graph, sol).unwrap_or(sol))
        }
    }

    /// Compute fundamental solution matrix $\mathbf{\Phi}(t) = e^{\mathbf{A}t}$ for linear ODE system $\mathbf{y}' = \mathbf{A}\mathbf{y}$.
    pub fn solve_linear_system_2x2(
        graph: &ExprGraph,
        a_matrix: &SymbolicMatrix,
        t: SymbolId,
    ) -> AlgebraResult<SymbolicMatrix> {
        if a_matrix.rows != 2 || a_matrix.cols != 2 {
            return Err(AlgebraError::DimensionMismatch(format!(
                "Expected 2x2 matrix, found {}x{}",
                a_matrix.rows, a_matrix.cols
            )));
        }

        let var_t = if let Some(t_str) = graph.symbols.resolve(t) {
            graph.symbol(&t_str)
        } else {
            graph.symbol("t")
        };
        let a11 = a_matrix.get(0, 0);
        let a12 = a_matrix.get(0, 1);
        let a21 = a_matrix.get(1, 0);
        let a22 = a_matrix.get(1, 1);

        // Putzer's Algorithm / Cayley-Hamilton for 2x2: e^{At} = r1(t)*I + r2(t)*(A - lambda_1*I)
        let trace = graph.add([a11, a22]);
        let prod_diag = graph.mul([a11, a22]);
        let prod_off = graph.mul([a12, a21]);
        let det = graph.sub(prod_diag, prod_off);

        // Compute characteristic eigenvalues analytically
        let half = graph.div(graph.integer(1), graph.integer(2));
        let half_tr = graph.mul([half, trace]);
        let tr_sq = graph.pow(trace, graph.integer(2));
        let four_det = graph.mul([graph.integer(4), det]);
        let disc = graph.sub(tr_sq, four_det);
        let sqrt_disc = graph.pow(disc, half);
        let half_sqrt_disc = graph.mul([half, sqrt_disc]);

        let lam1 = graph.add([half_tr, half_sqrt_disc]);
        let lam2 = graph.sub(half_tr, half_sqrt_disc);

        let exp_lam1_t = graph.function("exp", [graph.mul([lam1, var_t])]);
        let exp_lam2_t = graph.function("exp", [graph.mul([lam2, var_t])]);
        // Putzer: e^{At} = exp(lam1 * t) * I + r2(t) * (A - lam1 * I)
        // where r2(t) = (exp(lam1 * t) - exp(lam2 * t)) / (lam1 - lam2)
        let lam_diff = graph.sub(lam1, lam2);
        let exp_diff = graph.sub(exp_lam1_t, exp_lam2_t);
        let r2_t = graph.div(exp_diff, lam_diff);

        let a11_minus_lam1 = graph.sub(a11, lam1);
        let a22_minus_lam1 = graph.sub(a22, lam1);

        let m00 = graph.add([exp_lam1_t, graph.mul([r2_t, a11_minus_lam1])]);
        let m01 = graph.mul([r2_t, a12]);
        let m10 = graph.mul([r2_t, a21]);
        let m11 = graph.add([exp_lam1_t, graph.mul([r2_t, a22_minus_lam1])]);

        let m00_simp = Simplifier::simplify(graph, m00).unwrap_or(m00);
        let m01_simp = Simplifier::simplify(graph, m01).unwrap_or(m01);
        let m10_simp = Simplifier::simplify(graph, m10).unwrap_or(m10);
        let m11_simp = Simplifier::simplify(graph, m11).unwrap_or(m11);

        SymbolicMatrix::new(2, 2, vec![m00_simp, m01_simp, m10_simp, m11_simp])
    }
}
