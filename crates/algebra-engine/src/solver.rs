//! # `algebra-solver`
//!
//! Algebraic equation root finding (`solveset`), Ordinary Differential Equation (ODE) solvers,
//! Partial Differential Equation (PDE) solvers, and integer Diophantine equation solvers.

use crate::calculus::SymbolicCalculus;
use crate::numeric::NumericalEval;
use crate::simplify::Simplifier;
use algebra_core::{
    AlgebraResult, ExprGraph, ExprId, ExprKind, MathStep, StepByStepResult, SymbolId,
};

/// Diophantine Equation Solver for integer solutions over $\mathbb{Z}$.
pub struct DiophantineSolver;

impl DiophantineSolver {
    /// Solve Linear Diophantine Equation $a x + b y = c$ for $x, y \in \mathbb{Z}$.
    /// Returns particular solution `(x0, y0)` if an integer solution exists.
    pub fn solve_linear_2var(a: i64, b: i64, c: i64) -> Option<(i64, i64)> {
        fn ext_gcd(a: i64, b: i64) -> (i64, i64, i64) {
            if b == 0 {
                (a, 1, 0)
            } else {
                let (g, x1, y1) = ext_gcd(b, a % b);
                (g, y1, x1 - (a / b) * y1)
            }
        }

        let (g, x0, y0) = ext_gcd(a.abs(), b.abs());
        if g == 0 || c % g != 0 {
            return None;
        }

        let k = c / g;
        let x_sol = x0 * k * (if a < 0 { -1 } else { 1 });
        let y_sol = y0 * k * (if b < 0 { -1 } else { 1 });

        Some((x_sol, y_sol))
    }

    /// Solve Pell's Equation $x^2 - D y^2 = 1$ for fundamental positive integer solution $(x_1, y_1)$.
    pub fn solve_pell(d: i64) -> Option<(i64, i64)> {
        let sqrt_d = (d as f64).sqrt();
        if sqrt_d.fract() == 0.0 || d <= 0 {
            return None;
        }

        let m0 = 0i64;
        let d0 = 1i64;
        let a0 = sqrt_d as i64;

        let mut m = m0;
        let mut denom = d0;
        let mut a = a0;

        let mut p_prev2;
        let mut p_prev1 = 1i64;
        let mut q_prev2;
        let mut q_prev1 = 0i64;

        let mut p = a0;
        let mut q = 1i64;

        for _ in 0..100 {
            if p * p - d * q * q == 1 {
                return Some((p, q));
            }

            m = denom * a - m;
            denom = (d - m * m) / denom;
            if denom == 0 {
                break;
            }
            a = (a0 + m) / denom;

            p_prev2 = p_prev1;
            p_prev1 = p;
            q_prev2 = q_prev1;
            q_prev1 = q;

            p = a * p_prev1 + p_prev2;
            q = a * q_prev1 + q_prev2;
        }

        None
    }
}

/// Structured outcome of a symbolic solve attempt (Symbolica-style explicit coverage tracking).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolveOutcome<T> {
    /// Complete closed-form solution set obtained.
    Solved { solution: T, method: &'static str },
    /// Partial solution set obtained (e.g., `RootOf` implicit placeholders or principal branch only).
    IncompleteCoverage { partial: T, reason: String },
    /// Equation class is unsupported or no closed-form isolation exists.
    Unsupported {
        reason: String,
        failing_subexpr: Option<ExprId>,
    },
}

impl<T> SolveOutcome<T> {
    /// Returns `true` if a complete closed-form solution was found.
    pub fn is_solved(&self) -> bool {
        matches!(self, Self::Solved { .. })
    }

    /// Consumes the outcome, returning `Some(solution)` for `Solved` and `None` otherwise.
    pub fn into_solution(self) -> Option<T> {
        match self {
            Self::Solved { solution, .. } => Some(solution),
            Self::IncompleteCoverage { .. } | Self::Unsupported { .. } => None,
        }
    }

    /// Returns any human-readable diagnostic explaining incomplete coverage or unsupported forms.
    pub fn diagnostic(&self) -> Option<&str> {
        match self {
            Self::Solved { .. } => None,
            Self::IncompleteCoverage { reason, .. } | Self::Unsupported { reason, .. } => {
                Some(reason.as_str())
            }
        }
    }
}

/// Equation solver trait for algebraic equations, ODEs, and PDEs.
pub trait SymbolicSolver {
    /// Solve algebraic equation $f(x) = 0$ for target variable `target` (Exact symbolic pathway).
    fn solveset(&self, eq: ExprId, target: SymbolId) -> AlgebraResult<Vec<ExprId>>;

    /// Solve algebraic equation $f(x) = 0$ with typed completeness/diagnostic classification.
    fn solveset_outcome(&self, eq: ExprId, target: SymbolId) -> SolveOutcome<Vec<ExprId>>;

    /// Solve algebraic equation $f(x) = 0$ for target variable `target` with verification certificate.
    fn solveset_certified(
        &self,
        eq: ExprId,
        target: SymbolId,
        target_epsilon: f64,
    ) -> AlgebraResult<algebra_core::probabilistic::Solution<Vec<ExprId>>>;

    /// Solve algebraic equation $f(x) = 0$ for target variable `target`,
    /// returning solutions alongside explicit step-by-step educational derivation steps.
    fn solveset_with_steps(
        &self,
        eq: ExprId,
        target: SymbolId,
    ) -> AlgebraResult<StepByStepResult<Vec<ExprId>>>;

    /// Solve first-order linear ODE $y'(x) + P(x) y(x) = Q(x)$ for $y(x)$.
    fn solve_ode_linear(&self, p: ExprId, q: ExprId, x: SymbolId) -> AlgebraResult<ExprId>;

    /// Solve 1D Heat PDE $u_t = \alpha^2 u_{xx}$ via separation of variables $u(x, t) = X(x) T(t)$.
    fn solve_pde_heat_1d(
        &self,
        alpha: ExprId,
        x: SymbolId,
        t: SymbolId,
    ) -> AlgebraResult<(ExprId, ExprId)>;

    /// Solve 1D Wave PDE $u_{tt} = c^2 u_{xx}$ via separation of variables $u(x, t) = X(x) T(t)$.
    fn solve_pde_wave_1d(
        &self,
        c: ExprId,
        x: SymbolId,
        t: SymbolId,
    ) -> AlgebraResult<(ExprId, ExprId)>;

    /// Solve 1D Wave PDE via d'Alembert's formula: $u(x, t) = \frac{1}{2}[f(x - ct) + f(x + ct)] + \frac{1}{2c}\int_{x-ct}^{x+ct} g(s) ds$.
    fn solve_pde_wave_dalembert(
        &self,
        f: ExprId,
        g: Option<ExprId>,
        c: ExprId,
        x: SymbolId,
        t: SymbolId,
    ) -> AlgebraResult<ExprId>;

    /// Solve 2D Laplace PDE $u_{xx} + u_{yy} = 0$ via separation of variables.
    fn solve_pde_laplace_2d(
        &self,
        a: ExprId,
        x: SymbolId,
        y: SymbolId,
    ) -> AlgebraResult<(ExprId, ExprId)>;

    /// Solve 1D Transport / Advection PDE $u_t + c u_x = 0 \implies u(x, t) = f(x - c t)$.
    fn solve_pde_transport_1d(
        &self,
        f: ExprId,
        c: ExprId,
        x: SymbolId,
        t: SymbolId,
    ) -> AlgebraResult<ExprId>;

    /// Solve 2D Radial Laplacian Bessel eigenmode: $R_n(r) = J_n(k r)$.
    fn solve_pde_radial_bessel(
        &self,
        k: ExprId,
        r: SymbolId,
        order: usize,
    ) -> AlgebraResult<ExprId>;

    /// Solve polynomial equations using exact radicals (binomial, biquadratic, reciprocal, Cardano cubic, Bring radicals, or RootOf).
    fn solve_polynomial_radicals(&self, eq: ExprId, target: SymbolId)
    -> AlgebraResult<Vec<ExprId>>;
}

impl SymbolicSolver for ExprGraph {
    fn solveset_outcome(&self, eq: ExprId, target: SymbolId) -> SolveOutcome<Vec<ExprId>> {
        let node = self.get(eq);
        match &node.kind {
            ExprKind::Relational { lhs, rhs, .. } => {
                let diff = self.sub(*lhs, *rhs);
                return self.solveset_outcome(diff, target);
            }
            ExprKind::Mul(factors) => {
                let mut combined = Vec::new();
                for &f in factors {
                    if !self.has_symbol(f, target) {
                        continue;
                    }
                    match self.solveset_outcome(f, target) {
                        SolveOutcome::Solved { solution, .. } => {
                            for s in solution {
                                if !combined.contains(&s) {
                                    combined.push(s);
                                }
                            }
                        }
                        other => return other,
                    }
                }
                return SolveOutcome::Solved {
                    solution: combined,
                    method: "Factored Symbolic Solver",
                };
            }
            ExprKind::Pow(base, exp) if !self.has_symbol(*exp, target) => {
                return self.solveset_outcome(*base, target);
            }
            _ => {}
        }

        let eq_has_target = self.has_symbol(eq, target);
        if let Some(sub_expr) = find_non_polynomial_subexpr(self, eq, target) {
            if let Some(trans_roots) = solve_transcendental(self, eq, target)
                && !trans_roots.is_empty()
            {
                return SolveOutcome::Solved {
                    solution: trans_roots,
                    method: "Transcendental Inversion",
                };
            }
            return SolveOutcome::Unsupported {
                reason:
                    "Closed-form isolation unsupported for mixed or non-algebraic subexpression"
                        .to_string(),
                failing_subexpr: Some(sub_expr),
            };
        }

        match self.solveset(eq, target) {
            Ok(sols) => {
                let has_root_of = sols.iter().any(|&id| {
                    if let ExprKind::Function { name, .. } = &self.get(id).kind {
                        self.symbols.resolve(*name).as_deref() == Some("RootOf")
                    } else {
                        false
                    }
                });
                if has_root_of {
                    SolveOutcome::IncompleteCoverage {
                        partial: sols,
                        reason: "Polynomial exceeds closed-form radical formulas; implicit RootOf placeholders returned".to_string(),
                    }
                } else if sols.is_empty() && eq_has_target {
                    SolveOutcome::Unsupported {
                        reason: "No closed-form isolation found for equation".to_string(),
                        failing_subexpr: Some(eq),
                    }
                } else {
                    SolveOutcome::Solved {
                        solution: sols,
                        method: "Exact Symbolic Solver",
                    }
                }
            }
            Err(err) => SolveOutcome::Unsupported {
                reason: err.to_string(),
                failing_subexpr: Some(eq),
            },
        }
    }

    fn solveset_certified(
        &self,
        eq: ExprId,
        target: SymbolId,
        target_epsilon: f64,
    ) -> AlgebraResult<algebra_core::probabilistic::Solution<Vec<ExprId>>> {
        let roots = self.solveset(eq, target)?;
        let mut all_exact = true;
        let mut max_err: f64 = 0.0;
        let mut min_samples = usize::MAX;

        for &r in &roots {
            let substituted = self.substitute(eq, target, r);
            let verif =
                algebra_core::probabilistic::ProbabilisticVerifier::verify_zero_schwartz_zippel(
                    self,
                    substituted,
                    target_epsilon,
                );
            if !verif.is_deterministic() {
                all_exact = false;
                max_err = max_err.max(verif.error_probability());
                min_samples = min_samples.min(verif.sample_count());
            }
        }

        if all_exact || roots.is_empty() {
            Ok(algebra_core::probabilistic::Solution::Deterministic(roots))
        } else {
            Ok(algebra_core::probabilistic::Solution::Probabilistic {
                result: roots,
                error_probability_upper_bound: max_err.max(target_epsilon),
                sample_count: if min_samples == usize::MAX {
                    3
                } else {
                    min_samples
                },
                method: "Symbolic Solver & Schwartz-Zippel CRT Verification",
            })
        }
    }

    fn solveset(&self, eq: ExprId, target: SymbolId) -> AlgebraResult<Vec<ExprId>> {
        tracing::debug!(eq = ?eq, target = ?target, "Performing solveset exact symbolic equation root finding");
        let node = self.get(eq);

        match &node.kind {
            ExprKind::Relational { lhs, rhs, .. } => {
                let diff = self.sub(*lhs, *rhs);
                self.solveset(diff, target)
            }
            ExprKind::Sub(a, b) => {
                let neg_b = self.neg(*b);
                let sum = self.add([*a, neg_b]);
                self.solveset(sum, target)
            }
            ExprKind::Symbol(s) => {
                if *s == target {
                    Ok(vec![self.integer(0)])
                } else {
                    Ok(vec![])
                }
            }
            ExprKind::Mul(factors) => {
                let mut all_roots = Vec::new();
                for &f in factors {
                    if let Ok(roots) = self.solveset(f, target) {
                        for r in roots {
                            if !all_roots.contains(&r) {
                                all_roots.push(r);
                            }
                        }
                    }
                }
                Ok(all_roots)
            }
            ExprKind::Pow(base, _exp) => self.solveset(*base, target),
            _ => {
                if let Some(trans_roots) = solve_transcendental(self, eq, target) {
                    if !trans_roots.is_empty() {
                        return Ok(trans_roots);
                    }
                }
                self.solve_polynomial_radicals(eq, target)
            }
        }
    }

    fn solveset_with_steps(
        &self,
        eq: ExprId,
        target: SymbolId,
    ) -> AlgebraResult<StepByStepResult<Vec<ExprId>>> {
        let target_name = self
            .symbols
            .resolve(target)
            .unwrap_or_else(|| "x".to_string());
        let solutions = self.solveset(eq, target)?;
        let mut steps = Vec::new();

        if let Some(&first_sol) = solutions.first() {
            steps.push(MathStep::new(
                1,
                "Algebraic Variable Isolation",
                format!("Isolated target variable {} = f(...)", target_name),
                eq,
                first_sol,
            ));
        }

        Ok(StepByStepResult::new(solutions, steps))
    }

    fn solve_ode_linear(&self, p: ExprId, q: ExprId, x: SymbolId) -> AlgebraResult<ExprId> {
        let int_p = self.integrate(p, x)?;
        let mu = self.function("exp", [int_p]);
        let neg_int_p = self.neg(int_p);
        let inv_mu = self.function("exp", [neg_int_p]);
        let mu_q = self.mul([mu, q]);
        let int_mu_q = self.integrate(mu_q, x)?;
        let sol = self.mul([inv_mu, int_mu_q]);
        Ok(Simplifier::simplify(self, sol).unwrap_or(sol))
    }

    fn solve_pde_heat_1d(
        &self,
        alpha: ExprId,
        _x: SymbolId,
        _t: SymbolId,
    ) -> AlgebraResult<(ExprId, ExprId)> {
        let alpha_sq = self.pow(alpha, self.integer(2));
        let neg_alpha_sq = self.neg(alpha_sq);
        let spatial_sol = self.function("cos", [self.symbol("k_x")]);
        let temporal_sol = self.function(
            "exp",
            [self.mul([neg_alpha_sq, self.symbol("k_x"), self.symbol("t")])],
        );
        Ok((spatial_sol, temporal_sol))
    }

    fn solve_pde_wave_1d(
        &self,
        c: ExprId,
        x: SymbolId,
        t: SymbolId,
    ) -> AlgebraResult<(ExprId, ExprId)> {
        let l_node = self.symbol("L");
        crate::pde::SymbolicPdeSolver::solve_wave_1d_separated(self, c, l_node, x, t)
    }

    fn solve_pde_wave_dalembert(
        &self,
        f: ExprId,
        g: Option<ExprId>,
        c: ExprId,
        x: SymbolId,
        t: SymbolId,
    ) -> AlgebraResult<ExprId> {
        crate::pde::SymbolicPdeSolver::solve_wave_dalembert_exact(self, f, g, c, x, t)
    }

    fn solve_pde_laplace_2d(
        &self,
        a: ExprId,
        x: SymbolId,
        y: SymbolId,
    ) -> AlgebraResult<(ExprId, ExprId)> {
        crate::pde::SymbolicPdeSolver::solve_laplace_2d_separated(self, a, x, y)
    }

    fn solve_pde_transport_1d(
        &self,
        f: ExprId,
        c: ExprId,
        x: SymbolId,
        t: SymbolId,
    ) -> AlgebraResult<ExprId> {
        crate::pde::SymbolicPdeSolver::solve_transport_1d(self, f, c, x, t)
    }

    fn solve_pde_radial_bessel(
        &self,
        k: ExprId,
        r: SymbolId,
        order: usize,
    ) -> AlgebraResult<ExprId> {
        crate::pde::SymbolicPdeSolver::solve_bessel_radial(self, k, r, order)
    }

    fn solve_polynomial_radicals(
        &self,
        eq: ExprId,
        target: SymbolId,
    ) -> AlgebraResult<Vec<ExprId>> {
        let zero = self.integer(0);
        let ctx = crate::numeric::EvalContext::default();

        let get_coeff_val = |node_id: ExprId| -> f64 {
            match &self.get(node_id).kind {
                ExprKind::Number(n) => n.as_f64().unwrap_or(0.0),
                _ => self.evalf(node_id, &ctx).map(|v| v.to_f64()).unwrap_or(0.0),
            }
        };

        // Compute derivatives up to order 6 at 0
        let mut derivs = Vec::with_capacity(7);
        derivs.push(eq);
        for i in 0..6 {
            let d = self.diff(derivs[i], target);
            let simp = Simplifier::simplify(self, d).unwrap_or(d);
            derivs.push(simp);
        }

        let factorials = [1.0, 1.0, 2.0, 6.0, 24.0, 120.0, 720.0];
        let mut coeffs = Vec::with_capacity(7);
        let mut coeff_nodes = Vec::with_capacity(7);
        for (k, &d) in derivs.iter().enumerate() {
            let val_at_0 = self.substitute(d, target, zero);
            let simp_0 = Simplifier::simplify(self, val_at_0).unwrap_or(val_at_0);
            let val = get_coeff_val(simp_0) / factorials[k];
            coeffs.push(val);
            coeff_nodes.push(simp_0);
        }

        let mut deg = 0;
        for k in (1..=6).rev() {
            if coeffs[k].abs() > 1e-12 {
                deg = k;
                break;
            }
        }

        if deg == 0 {
            if coeffs[0].abs() < 1e-12 {
                return Ok(vec![self.symbol("x")]);
            } else {
                return Ok(vec![]);
            }
        }

        // Degree 1: c1 * x + c0 = 0 -> x = -c0 / c1
        if deg == 1 {
            let r_val = -coeffs[0] / coeffs[1];
            let r_lossless = algebra_core::Number::from_f64_lossless(r_val);
            if r_lossless.is_lossless() {
                let r_id = self.intern(algebra_core::ExprNode::new(
                    algebra_core::Domain::Rationals,
                    ExprKind::Number(r_lossless),
                ));
                return Ok(vec![r_id]);
            }
            let neg_c = self.neg(coeff_nodes[0]);
            let sol = self.div(neg_c, coeff_nodes[1]);
            let simp = Simplifier::simplify(self, sol).unwrap_or(sol);
            return Ok(vec![simp]);
        }

        // Pure Binomial: c_n * x^n + c_0 = 0
        let is_pure_binomial =
            deg >= 2 && (1..deg).all(|k| coeffs[k].abs() < 1e-12) && coeffs[0].abs() > 1e-12;

        if is_pure_binomial {
            let r_val = -coeffs[0] / coeffs[deg];
            let half = self.div(self.integer(1), self.integer(2));
            if deg == 2 {
                if r_val >= 0.0 {
                    let sq = r_val.sqrt();
                    if (sq - sq.round()).abs() < 1e-10 {
                        let s_int = sq.round() as i64;
                        return Ok(vec![self.integer(s_int), self.integer(-s_int)]);
                    }
                    let r_node = self.float(r_val);
                    let sqrt_r = self.pow(r_node, half);
                    return Ok(vec![sqrt_r, self.neg(sqrt_r)]);
                } else {
                    let pos_r = -r_val;
                    let sq = pos_r.sqrt();
                    let i_sym = self.constant(algebra_core::Constant::I);
                    if (sq - sq.round()).abs() < 1e-10 {
                        let s_int = sq.round() as i64;
                        let sol1 = self.mul([self.integer(s_int), i_sym]);
                        let sol2 = self.mul([self.integer(-s_int), i_sym]);
                        return Ok(vec![sol1, sol2]);
                    }
                    let pos_node = self.float(pos_r);
                    let sqrt_r = self.pow(pos_node, half);
                    let sol1 = self.mul([sqrt_r, i_sym]);
                    let sol2 = self.neg(sol1);
                    return Ok(vec![sol1, sol2]);
                }
            } else if deg == 3 {
                let one_third = self.div(self.integer(1), self.integer(3));
                let root_val = if r_val >= 0.0 {
                    r_val.cbrt()
                } else {
                    -(-r_val).cbrt()
                };
                if (root_val - root_val.round()).abs() < 1e-10 {
                    let r_int = root_val.round() as i64;
                    let r0 = self.integer(r_int);
                    let neg_half = self.div(self.integer(-1), self.integer(2));
                    let sqrt3_over_2 = self.div(self.pow(self.integer(3), half), self.integer(2));
                    let i_sym = self.constant(algebra_core::Constant::I);
                    let im_part = self.mul([sqrt3_over_2, i_sym]);
                    let omega1 = self.add([neg_half, im_part]);
                    let omega2 = self.sub(neg_half, im_part);
                    let r1 = self.mul([r0, omega1]);
                    let r2 = self.mul([r0, omega2]);
                    return Ok(vec![
                        r0,
                        Simplifier::simplify(self, r1).unwrap_or(r1),
                        Simplifier::simplify(self, r2).unwrap_or(r2),
                    ]);
                } else {
                    let abs_val = r_val.abs();
                    let base = self.float(abs_val);
                    let r0 = if r_val >= 0.0 {
                        self.pow(base, one_third)
                    } else {
                        self.neg(self.pow(base, one_third))
                    };
                    return Ok(vec![r0]);
                }
            } else if deg == 4 {
                let one_fourth = self.div(self.integer(1), self.integer(4));
                if r_val >= 0.0 {
                    let q_val = r_val.powf(0.25);
                    if (q_val - q_val.round()).abs() < 1e-10 {
                        let q_int = q_val.round() as i64;
                        let i_sym = self.constant(algebra_core::Constant::I);
                        let r1 = self.integer(q_int);
                        let r2 = self.integer(-q_int);
                        let r3 = self.mul([r1, i_sym]);
                        let r4 = self.mul([r2, i_sym]);
                        return Ok(vec![r1, r2, r3, r4]);
                    } else {
                        let base = self.float(r_val);
                        let r0 = self.pow(base, one_fourth);
                        let neg_r0 = self.neg(r0);
                        let i_sym = self.constant(algebra_core::Constant::I);
                        let r_im1 = self.mul([r0, i_sym]);
                        let r_im2 = self.neg(r_im1);
                        return Ok(vec![r0, neg_r0, r_im1, r_im2]);
                    }
                }
            } else if deg == 5 {
                let one_fifth = self.div(self.integer(1), self.integer(5));
                let root_val = if r_val >= 0.0 {
                    r_val.powf(0.2)
                } else {
                    -(-r_val).powf(0.2)
                };
                if (root_val - root_val.round()).abs() < 1e-10 {
                    let r_int = root_val.round() as i64;
                    return Ok(vec![self.integer(r_int)]);
                } else {
                    let abs_val = r_val.abs();
                    let base = self.float(abs_val);
                    let r0 = if r_val >= 0.0 {
                        self.pow(base, one_fifth)
                    } else {
                        self.neg(self.pow(base, one_fifth))
                    };
                    return Ok(vec![r0]);
                }
            }
        }

        // Degree 2: Quadratic formula
        if deg == 2 {
            let a = coeffs[2];
            let b = coeffs[1];
            let c = coeffs[0];
            let disc = b * b - 4.0 * a * c;
            let two_a = 2.0 * a;

            if disc >= 0.0 {
                let sqrt_d = disc.sqrt();
                if (sqrt_d - sqrt_d.round()).abs() < 1e-10 {
                    let sd = sqrt_d.round();
                    let r1_val = (-b + sd) / two_a;
                    let r2_val = (-b - sd) / two_a;
                    let r1 = algebra_core::Number::from_f64_lossless(r1_val);
                    let r2 = algebra_core::Number::from_f64_lossless(r2_val);
                    let r1_id = self.intern(algebra_core::ExprNode::new(
                        algebra_core::Domain::Rationals,
                        ExprKind::Number(r1),
                    ));
                    let r2_id = self.intern(algebra_core::ExprNode::new(
                        algebra_core::Domain::Rationals,
                        ExprKind::Number(r2),
                    ));
                    if (r1_val - r2_val).abs() < 1e-12 {
                        return Ok(vec![r1_id]);
                    } else {
                        return Ok(vec![r1_id, r2_id]);
                    }
                }

                let half = self.div(self.integer(1), self.integer(2));
                let disc_node = self.float(disc);
                let sqrt_disc = self.pow(disc_node, half);
                let neg_b = self.float(-b);
                let two_a_node = self.float(two_a);

                let num1 = self.add([neg_b, sqrt_disc]);
                let num2 = self.sub(neg_b, sqrt_disc);
                let sol1 = self.div(num1, two_a_node);
                let sol2 = self.div(num2, two_a_node);
                let s1 = Simplifier::simplify(self, sol1).unwrap_or(sol1);
                let s2 = Simplifier::simplify(self, sol2).unwrap_or(sol2);
                return Ok(vec![s1, s2]);
            } else {
                let pos_disc = -disc;
                let half = self.div(self.integer(1), self.integer(2));
                let disc_node = self.float(pos_disc);
                let sqrt_disc = self.pow(disc_node, half);
                let i_sym = self.constant(algebra_core::Constant::I);
                let im_term = self.mul([sqrt_disc, i_sym]);
                let neg_b = self.float(-b);
                let two_a_node = self.float(two_a);

                let num1 = self.add([neg_b, im_term]);
                let num2 = self.sub(neg_b, im_term);
                let sol1 = self.div(num1, two_a_node);
                let sol2 = self.div(num2, two_a_node);
                let s1 = Simplifier::simplify(self, sol1).unwrap_or(sol1);
                let s2 = Simplifier::simplify(self, sol2).unwrap_or(sol2);
                return Ok(vec![s1, s2]);
            }
        }

        // Degree 4: Biquadratic check: c3 == 0 and c1 == 0
        if deg == 4 && coeffs[3].abs() < 1e-12 && coeffs[1].abs() < 1e-12 {
            let a = coeffs[4];
            let b = coeffs[2];
            let c = coeffs[0];
            let disc_u = b * b - 4.0 * a * c;

            if disc_u >= 0.0 {
                let sqrt_du = disc_u.sqrt();
                let u1 = (-b + sqrt_du) / (2.0 * a);
                let u2 = (-b - sqrt_du) / (2.0 * a);

                let mut all_x = Vec::new();
                for &u in &[u1, u2] {
                    if u >= 0.0 {
                        let sq = u.sqrt();
                        if (sq - sq.round()).abs() < 1e-10 {
                            let val = sq.round() as i64;
                            all_x.push(self.integer(val));
                            if val != 0 {
                                all_x.push(self.integer(-val));
                            }
                        } else {
                            let half = self.div(self.integer(1), self.integer(2));
                            let u_node = self.float(u);
                            let sqrt_node = self.pow(u_node, half);
                            all_x.push(sqrt_node);
                            all_x.push(self.neg(sqrt_node));
                        }
                    } else {
                        let pos_u = -u;
                        let sq = pos_u.sqrt();
                        let i_sym = self.constant(algebra_core::Constant::I);
                        if (sq - sq.round()).abs() < 1e-10 {
                            let val = sq.round() as i64;
                            let sol1 = self.mul([self.integer(val), i_sym]);
                            let sol2 = self.mul([self.integer(-val), i_sym]);
                            all_x.push(sol1);
                            all_x.push(sol2);
                        } else {
                            let half = self.div(self.integer(1), self.integer(2));
                            let pos_node = self.float(pos_u);
                            let sqrt_node = self.pow(pos_node, half);
                            let sol1 = self.mul([sqrt_node, i_sym]);
                            let sol2 = self.neg(sol1);
                            all_x.push(sol1);
                            all_x.push(sol2);
                        }
                    }
                }
                return Ok(all_x);
            }
        }

        // Degree 4: Reciprocal / Palindromic Quartic: a x^4 + b x^3 + c x^2 + b x + a = 0
        if deg == 4
            && (coeffs[4] - coeffs[0]).abs() < 1e-10
            && (coeffs[3] - coeffs[1]).abs() < 1e-10
            && coeffs[4].abs() > 1e-12
        {
            let a = coeffs[4];
            let b = coeffs[3];
            let c_const = coeffs[2] - 2.0 * a;
            let disc_y = b * b - 4.0 * a * c_const;
            if disc_y >= 0.0 {
                let sqrt_dy = disc_y.sqrt();
                let y1 = (-b + sqrt_dy) / (2.0 * a);
                let y2 = (-b - sqrt_dy) / (2.0 * a);
                let mut all_x = Vec::new();

                for &y in &[y1, y2] {
                    let disc_x = y * y - 4.0;
                    if disc_x >= 0.0 {
                        let sqx = disc_x.sqrt();
                        let x1 = (y + sqx) / 2.0;
                        let x2 = (y - sqx) / 2.0;
                        let num1 = algebra_core::Number::from_f64_lossless(x1);
                        let num2 = algebra_core::Number::from_f64_lossless(x2);
                        all_x.push(self.intern(algebra_core::ExprNode::new(
                            algebra_core::Domain::Rationals,
                            ExprKind::Number(num1),
                        )));
                        all_x.push(self.intern(algebra_core::ExprNode::new(
                            algebra_core::Domain::Rationals,
                            ExprKind::Number(num2),
                        )));
                    } else {
                        let half = self.div(self.integer(1), self.integer(2));
                        let disc_node = self.float(-disc_x);
                        let sqrt_node = self.pow(disc_node, half);
                        let i_sym = self.constant(algebra_core::Constant::I);
                        let im = self.mul([sqrt_node, i_sym]);
                        let y_node = self.float(y / 2.0);
                        let im_over_2 = self.div(im, self.integer(2));
                        all_x.push(self.add([y_node, im_over_2]));
                        all_x.push(self.sub(y_node, im_over_2));
                    }
                }
                return Ok(all_x);
            }
        }

        // Degree 3: Cardano's Cubic Formula (with rational root factoring first)
        if deg == 3 {
            let test_candidates = [
                0.0,
                1.0,
                -1.0,
                2.0,
                -2.0,
                3.0,
                -3.0,
                4.0,
                -4.0,
                5.0,
                -5.0,
                0.5,
                -0.5,
                1.0 / 3.0,
                -1.0 / 3.0,
                2.0 / 3.0,
                -2.0 / 3.0,
                0.25,
                -0.25,
                0.75,
                -0.75,
            ];
            let mut found_rational: Option<f64> = None;
            for &cand in &test_candidates {
                let cand_id = self.float(cand);
                let sub_val = self.substitute(eq, target, cand_id);
                if let Ok(eval_res) = self.evalf(sub_val, &ctx)
                    && eval_res.to_f64().abs() < 1e-11
                {
                    found_rational = Some(cand);
                    break;
                }
            }

            if let Some(r) = found_rational {
                let a = coeffs[3];
                let b = coeffs[2];
                let c = coeffs[1];
                let cap_a = a;
                let cap_b = b + a * r;
                let cap_c = c + b * r + a * r * r;

                let mut all_roots = Vec::new();
                let r_num = algebra_core::Number::from_f64_lossless(r);
                all_roots.push(self.intern(algebra_core::ExprNode::new(
                    algebra_core::Domain::Rationals,
                    ExprKind::Number(r_num),
                )));

                let disc = cap_b * cap_b - 4.0 * cap_a * cap_c;
                if disc >= 0.0 {
                    let sq = disc.sqrt();
                    let r1 = (-cap_b + sq) / (2.0 * cap_a);
                    let r2 = (-cap_b - sq) / (2.0 * cap_a);
                    let n1 = algebra_core::Number::from_f64_lossless(r1);
                    let n2 = algebra_core::Number::from_f64_lossless(r2);
                    let id1 = self.intern(algebra_core::ExprNode::new(
                        algebra_core::Domain::Rationals,
                        ExprKind::Number(n1),
                    ));
                    let id2 = self.intern(algebra_core::ExprNode::new(
                        algebra_core::Domain::Rationals,
                        ExprKind::Number(n2),
                    ));
                    if !all_roots.contains(&id1) {
                        all_roots.push(id1);
                    }
                    if !all_roots.contains(&id2) {
                        all_roots.push(id2);
                    }
                } else {
                    let pos_d = -disc;
                    let half = self.div(self.integer(1), self.integer(2));
                    let disc_node = self.float(pos_d);
                    let sqrt_node = self.pow(disc_node, half);
                    let i_sym = self.constant(algebra_core::Constant::I);
                    let im_term = self.mul([sqrt_node, i_sym]);
                    let real_term = self.float(-cap_b / (2.0 * cap_a));
                    let im_over_2a = self.div(im_term, self.float(2.0 * cap_a));
                    all_roots.push(self.add([real_term, im_over_2a]));
                    all_roots.push(self.sub(real_term, im_over_2a));
                }
                return Ok(all_roots);
            }

            let a = coeffs[3];
            let b = coeffs[2];
            let c = coeffs[1];
            let d = coeffs[0];

            let p = (3.0 * a * c - b * b) / (3.0 * a * a);
            let q = (2.0 * b * b * b - 9.0 * a * b * c + 27.0 * a * a * d) / (27.0 * a * a * a);
            let delta = (q * q) / 4.0 + (p * p * p) / 27.0;

            let shift = -b / (3.0 * a);
            let _shift_node = self.float(shift);

            if delta >= 0.0 {
                let sqrt_delta = delta.sqrt();
                let u_val = -q / 2.0 + sqrt_delta;
                let v_val = -q / 2.0 - sqrt_delta;

                let u_sign = if u_val >= 0.0 { 1.0 } else { -1.0 };
                let v_sign = if v_val >= 0.0 { 1.0 } else { -1.0 };

                let u = u_sign * u_val.abs().cbrt();
                let v = v_sign * v_val.abs().cbrt();

                let real_root = u + v + shift;
                let real_node = self.float(real_root);

                let half = self.div(self.integer(1), self.integer(2));
                let sqrt3 = self.pow(self.integer(3), half);
                let i_sym = self.constant(algebra_core::Constant::I);

                let re_complex = -0.5 * (u + v) + shift;
                let re_node = self.float(re_complex);
                let im_factor = self.float(0.5 * (u - v));
                let im_part = self.mul([im_factor, sqrt3, i_sym]);

                let c1 = self.add([re_node, im_part]);
                let c2 = self.sub(re_node, im_part);

                return Ok(vec![real_node, c1, c2]);
            } else {
                let r = (-p * p * p / 27.0).sqrt();
                let theta = (-q / (2.0 * r)).acos();
                let mult = 2.0 * (-p / 3.0).sqrt();

                let x1 = mult * (theta / 3.0).cos() + shift;
                let x2 = mult * ((theta + 2.0 * std::f64::consts::PI) / 3.0).cos() + shift;
                let x3 = mult * ((theta + 4.0 * std::f64::consts::PI) / 3.0).cos() + shift;

                let r1 = self.intern(algebra_core::ExprNode::new(
                    algebra_core::Domain::Reals,
                    ExprKind::Number(algebra_core::Number::from_f64_lossless(x1)),
                ));
                let r2 = self.intern(algebra_core::ExprNode::new(
                    algebra_core::Domain::Reals,
                    ExprKind::Number(algebra_core::Number::from_f64_lossless(x2)),
                ));
                let r3 = self.intern(algebra_core::ExprNode::new(
                    algebra_core::Domain::Reals,
                    ExprKind::Number(algebra_core::Number::from_f64_lossless(x3)),
                ));
                return Ok(vec![r1, r2, r3]);
            }
        }

        // Degree 5: Bring Radical check: c4 == 0, c3 == 0, c2 == 0
        if deg == 5 && coeffs[4].abs() < 1e-12 && coeffs[3].abs() < 1e-12 && coeffs[2].abs() < 1e-12
        {
            let p = coeffs[1] / coeffs[5];
            let q = coeffs[0] / coeffs[5];

            if (p - 1.0).abs() < 1e-10 {
                let q_num = algebra_core::Number::from_f64_lossless(q);
                let q_node = self.intern(algebra_core::ExprNode::new(
                    algebra_core::Domain::Rationals,
                    ExprKind::Number(q_num),
                ));
                let br_node = self.function("BR", [q_node]);
                return Ok(vec![br_node]);
            } else if p > 0.0 {
                let p_quart = p.powf(0.25);
                let p_five_quart = p.powf(1.25);
                let arg = q / p_five_quart;
                let arg_num = algebra_core::Number::from_f64_lossless(arg);
                let arg_node = self.intern(algebra_core::ExprNode::new(
                    algebra_core::Domain::Rationals,
                    ExprKind::Number(arg_num),
                ));
                let br_node = self.function("BR", [arg_node]);
                let p_quart_node = self.float(p_quart);
                let sol = self.mul([p_quart_node, br_node]);
                return Ok(vec![sol]);
            }
        }

        // General Rational Root Theorem for higher degree polynomials
        let test_candidates = [
            0.0,
            1.0,
            -1.0,
            2.0,
            -2.0,
            3.0,
            -3.0,
            4.0,
            -4.0,
            5.0,
            -5.0,
            0.5,
            -0.5,
            1.0 / 3.0,
            -1.0 / 3.0,
            2.0 / 3.0,
            -2.0 / 3.0,
            0.25,
            -0.25,
            0.75,
            -0.75,
            1.0 / 5.0,
            -1.0 / 5.0,
        ];
        let mut rational_roots = Vec::new();
        for &cand in &test_candidates {
            let cand_num = algebra_core::Number::from_f64_lossless(cand);
            let cand_id = self.intern(algebra_core::ExprNode::new(
                algebra_core::Domain::Rationals,
                ExprKind::Number(cand_num),
            ));
            let sub_val = self.substitute(eq, target, cand_id);
            if let Ok(eval_res) = self.evalf(sub_val, &ctx)
                && eval_res.to_f64().abs() < 1e-11
                && !rational_roots.contains(&cand_id)
            {
                rational_roots.push(cand_id);
            }
        }

        if !rational_roots.is_empty() {
            return Ok(rational_roots);
        }

        // Exact Symbolic RootOf representation before lossy fallback
        let target_name = self
            .symbols
            .resolve(target)
            .unwrap_or_else(|| "x".to_string());
        let target_node = self.symbol(&target_name);
        let mut root_of_nodes = Vec::new();
        for k in 1..=deg {
            let k_node = self.integer(k as i64);
            let root_of = self.function("RootOf", [eq, target_node, k_node]);
            root_of_nodes.push(root_of);
        }
        Ok(root_of_nodes)
    }
}

/// Exact transcendental equation solver ($e^x = c$, $\ln(x) = c$, $\sin(x) = c$, $x e^x = c$ via Lambert W).
fn solve_transcendental(graph: &ExprGraph, eq: ExprId, target: SymbolId) -> Option<Vec<ExprId>> {
    // Check for quadratic in e^x first: A*e^(2x) + B*e^x + C = 0
    if let Some(roots) = solve_transcendental_quadratic(graph, eq, target) {
        return Some(roots);
    }

    let node = graph.get(eq);
    let wrt_name = graph.symbols.resolve(target)?;
    let x_sym = graph.symbol(&wrt_name);

    // Decompose eq = 0 into T(x) = C (where C is independent of target)
    let (t_x, c_val) = match &node.kind {
        ExprKind::Sub(a, b) => {
            let a_has = graph.has_symbol(*a, target);
            let b_has = graph.has_symbol(*b, target);
            if a_has && !b_has {
                (*a, *b)
            } else if !a_has && b_has {
                (*b, *a)
            } else {
                return solve_transcendental_quadratic(graph, eq, target);
            }
        }
        ExprKind::Add(terms) => {
            let mut with_x = Vec::new();
            let mut without_x = Vec::new();
            for &t in terms {
                if graph.has_symbol(t, target) {
                    with_x.push(t);
                } else {
                    without_x.push(t);
                }
            }
            if with_x.len() == 1 {
                let c_neg = if without_x.is_empty() {
                    graph.integer(0)
                } else if without_x.len() == 1 {
                    graph.neg(without_x[0])
                } else {
                    graph.neg(graph.add(without_x))
                };
                (with_x[0], c_neg)
            } else {
                return solve_transcendental_quadratic(graph, eq, target);
            }
        }
        ExprKind::Function { .. } => {
            if graph.has_symbol(eq, target) {
                (eq, graph.integer(0))
            } else {
                return None;
            }
        }
        ExprKind::Mul(factors) => {
            return solve_transcendental_product(graph, factors, target);
        }
        _ => return None,
    };

    // Check if t_x is a single transcendental function: F(arg) = C
    let t_node = graph.get(t_x);
    if let ExprKind::Function { name, args } = &t_node.kind {
        let fn_name = graph.symbols.resolve(*name)?;
        if args.len() == 1 {
            let arg = args[0];
            match fn_name.as_str() {
                "exp" => {
                    let ln_c = graph.function("ln", [c_val]);
                    if arg == x_sym {
                        return Some(vec![ln_c]);
                    }
                    let new_eq = graph.sub(arg, ln_c);
                    return graph.solveset(new_eq, target).ok();
                }
                "ln" | "log" => {
                    let exp_c = graph.function("exp", [c_val]);
                    if arg == x_sym {
                        return Some(vec![exp_c]);
                    }
                    let new_eq = graph.sub(arg, exp_c);
                    return graph.solveset(new_eq, target).ok();
                }
                "sin" => {
                    let asin_c = graph.function("asin", [c_val]);
                    if arg == x_sym {
                        return Some(vec![asin_c]);
                    }
                    let new_eq = graph.sub(arg, asin_c);
                    return graph.solveset(new_eq, target).ok();
                }
                "cos" => {
                    let acos_c = graph.function("acos", [c_val]);
                    if arg == x_sym {
                        return Some(vec![acos_c]);
                    }
                    let new_eq = graph.sub(arg, acos_c);
                    return graph.solveset(new_eq, target).ok();
                }
                "tan" => {
                    let atan_c = graph.function("atan", [c_val]);
                    if arg == x_sym {
                        return Some(vec![atan_c]);
                    }
                    let new_eq = graph.sub(arg, atan_c);
                    return graph.solveset(new_eq, target).ok();
                }
                "sqrt" => {
                    let c_sq = match graph.get(c_val).kind {
                        ExprKind::Number(algebra_core::Number::Integer(n)) => graph.integer(n * n),
                        _ => {
                            let pow_node = graph.pow(c_val, graph.integer(2));
                            Simplifier::simplify(graph, pow_node).unwrap_or(pow_node)
                        }
                    };
                    if arg == x_sym {
                        return Some(vec![c_sq]);
                    }
                    let new_eq = graph.sub(arg, c_sq);
                    return graph.solveset(new_eq, target).ok();
                }
                "lambert_w" | "LambertW" | "W" => {
                    let exp_c = graph.function("exp", [c_val]);
                    let c_exp_c = graph.mul([c_val, exp_c]);
                    if arg == x_sym {
                        return Some(vec![c_exp_c]);
                    }
                    let new_eq = graph.sub(arg, c_exp_c);
                    return graph.solveset(new_eq, target).ok();
                }
                _ => {}
            }
        }
    }

    // Check for Lambert W forms: x * exp(x) = C or x * ln(x) = C
    if let ExprKind::Mul(factors) = &t_node.kind {
        if factors.len() == 2 {
            let (f1, f2) = (factors[0], factors[1]);
            let is_x1 = f1 == x_sym;
            let is_x2 = f2 == x_sym;
            if is_x1 || is_x2 {
                let other = if is_x1 { f2 } else { f1 };
                let other_node = graph.get(other);
                if let ExprKind::Function { name, args } = &other_node.kind {
                    let fn_name = graph.symbols.resolve(*name).unwrap_or_default();
                    if fn_name == "exp" && args.len() == 1 && args[0] == x_sym {
                        let w_c = graph.function("lambert_w", [c_val]);
                        return Some(vec![w_c]);
                    } else if (fn_name == "ln" || fn_name == "log")
                        && args.len() == 1
                        && args[0] == x_sym
                    {
                        let w_c = graph.function("lambert_w", [c_val]);
                        let exp_w_c = graph.function("exp", [w_c]);
                        return Some(vec![exp_w_c]);
                    }
                }
            }
        }
    }

    None
}

fn solve_transcendental_product(
    graph: &ExprGraph,
    factors: &[ExprId],
    target: SymbolId,
) -> Option<Vec<ExprId>> {
    let mut roots = Vec::new();
    for &f in factors {
        if graph.has_symbol(f, target) {
            if let Ok(r) = graph.solveset(f, target) {
                for root in r {
                    if !roots.contains(&root) {
                        roots.push(root);
                    }
                }
            }
        }
    }
    if !roots.is_empty() { Some(roots) } else { None }
}

fn solve_transcendental_quadratic(
    graph: &ExprGraph,
    eq: ExprId,
    target: SymbolId,
) -> Option<Vec<ExprId>> {
    let wrt_name = graph.symbols.resolve(target)?;
    let x_sym = graph.symbol(&wrt_name);
    let exp_x = graph.function("exp", [x_sym]);
    let two_x = graph.mul([graph.integer(2), x_sym]);
    let exp_2x = graph.function("exp", [two_x]);

    let u_sym = graph.symbol("__u_sub");
    let u_target = match graph.get(u_sym).kind {
        ExprKind::Symbol(s) => s,
        _ => return None,
    };
    let u_sq = graph.pow(u_sym, graph.integer(2));

    let sub1 = replace_exact(graph, eq, exp_2x, u_sq);
    let sub2 = replace_exact(graph, sub1, exp_x, u_sym);

    if graph.has_symbol(sub2, target) {
        return None;
    }

    let u_roots = graph.solve_polynomial_radicals(sub2, u_target).ok()?;
    if u_roots.is_empty() {
        return None;
    }

    let mut x_roots = Vec::new();
    for u_r in u_roots {
        let simp = if u_r == graph.integer(1) {
            graph.integer(0)
        } else {
            let ln_u = graph.function("ln", [u_r]);
            Simplifier::simplify(graph, ln_u).unwrap_or(ln_u)
        };
        x_roots.push(simp);
    }

    Some(x_roots)
}

fn replace_exact(graph: &ExprGraph, expr: ExprId, pattern: ExprId, replacement: ExprId) -> ExprId {
    if expr == pattern {
        return replacement;
    }
    let node = graph.get(expr);
    match &node.kind {
        ExprKind::Add(terms) => {
            let new_terms: Vec<ExprId> = terms
                .iter()
                .map(|&t| replace_exact(graph, t, pattern, replacement))
                .collect();
            graph.add(new_terms)
        }
        ExprKind::Mul(factors) => {
            let new_factors: Vec<ExprId> = factors
                .iter()
                .map(|&f| replace_exact(graph, f, pattern, replacement))
                .collect();
            graph.mul(new_factors)
        }
        ExprKind::Sub(a, b) => {
            let na = replace_exact(graph, *a, pattern, replacement);
            let nb = replace_exact(graph, *b, pattern, replacement);
            graph.sub(na, nb)
        }
        ExprKind::Div(n, d) => {
            let nn = replace_exact(graph, *n, pattern, replacement);
            let nd = replace_exact(graph, *d, pattern, replacement);
            graph.div(nn, nd)
        }
        ExprKind::Neg(inner) => {
            let ni = replace_exact(graph, *inner, pattern, replacement);
            graph.neg(ni)
        }
        ExprKind::Pow(b, e) => {
            let nb = replace_exact(graph, *b, pattern, replacement);
            let ne = replace_exact(graph, *e, pattern, replacement);
            graph.pow(nb, ne)
        }
        ExprKind::Function { name, args } => {
            let new_args: Vec<ExprId> = args
                .iter()
                .map(|&a| replace_exact(graph, a, pattern, replacement))
                .collect();
            let fn_name = graph.symbols.resolve(*name).unwrap_or_default();
            graph.function(&fn_name, new_args)
        }
        _ => expr,
    }
}

fn find_non_polynomial_subexpr(
    graph: &ExprGraph,
    expr: ExprId,
    target: SymbolId,
) -> Option<ExprId> {
    if !graph.has_symbol(expr, target) {
        return None;
    }
    match &graph.get(expr).kind {
        ExprKind::Number(_) | ExprKind::Symbol(_) => None,
        ExprKind::Neg(inner) => find_non_polynomial_subexpr(graph, *inner, target),
        ExprKind::Add(items) | ExprKind::Mul(items) => items
            .iter()
            .find_map(|&item| find_non_polynomial_subexpr(graph, item, target)),
        ExprKind::Sub(a, b) => find_non_polynomial_subexpr(graph, *a, target)
            .or_else(|| find_non_polynomial_subexpr(graph, *b, target)),
        ExprKind::Div(num, den) => {
            if graph.has_symbol(*den, target) {
                Some(expr)
            } else {
                find_non_polynomial_subexpr(graph, *num, target)
            }
        }
        ExprKind::Pow(base, exp) => {
            if graph.has_symbol(*exp, target) {
                return Some(expr);
            }
            let is_nonneg_int = match &graph.get(*exp).kind {
                ExprKind::Number(n) => n
                    .as_f64()
                    .is_some_and(|v| v >= 0.0 && (v - v.round()).abs() < 1e-12),
                _ => false,
            };
            if !is_nonneg_int {
                Some(expr)
            } else {
                find_non_polynomial_subexpr(graph, *base, target)
            }
        }
        _ => Some(expr),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use algebra_core::ExprParser;

    #[test]
    fn test_solveset_outcome_classification() {
        let graph = ExprGraph::new();
        let x = graph.symbols.get_or_intern("x");
        let parser = ExprParser::new(&graph);

        // Quadratic with closed-form integer/radical roots -> Solved
        let quad = parser.parse("x^2 - 5*x + 6").expect("parse quad");
        let out_quad = graph.solveset_outcome(quad, x);
        assert!(out_quad.is_solved(), "expected Solved, got {out_quad:?}");
        assert!(out_quad.diagnostic().is_none());
        let sols = out_quad.into_solution().expect("solutions");
        assert_eq!(sols.len(), 2);

        // High-degree polynomial with no rational roots -> IncompleteCoverage (RootOf)
        let poly6 = parser.parse("x^6 + 2*x + 3").expect("parse poly6");
        let out_poly6 = graph.solveset_outcome(poly6, x);
        assert!(
            matches!(out_poly6, SolveOutcome::IncompleteCoverage { .. }),
            "expected IncompleteCoverage, got {out_poly6:?}"
        );
        assert!(out_poly6.diagnostic().is_some());

        // Non-algebraic mixed equation -> Unsupported with failing_subexpr
        let mixed = parser.parse("sin(x) + x").expect("parse mixed");
        let out_mixed = graph.solveset_outcome(mixed, x);
        assert!(
            matches!(
                out_mixed,
                SolveOutcome::Unsupported {
                    failing_subexpr: Some(_),
                    ..
                }
            ),
            "expected Unsupported, got {out_mixed:?}"
        );
    }
}
