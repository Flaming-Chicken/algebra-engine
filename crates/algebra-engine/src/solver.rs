//! # `algebra-solver`
//!
//! Algebraic equation root finding (`solveset`), Ordinary Differential Equation (ODE) solvers,
//! Partial Differential Equation (PDE) solvers, and integer Diophantine equation solvers.

use crate::calculus::SymbolicCalculus;
use crate::numeric::NumericalEval;
use crate::simplify::Simplifier;
use algebra_core::{
    AlgebraResult, ExprGraph, ExprId, ExprKind, MathStep, Number, StepByStepResult, SymbolId,
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
            if let Some(trans_roots) = solve_transcendental(self, eq, target) {
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
            ExprKind::Neg(inner) => self.solveset(*inner, target),
            ExprKind::Sub(a, b) => {
                let neg_b = self.neg(*b);
                let sum = self.add([*a, neg_b]);
                self.solveset(sum, target)
            }
            ExprKind::Number(_) => Ok(vec![]),
            ExprKind::Symbol(s) => {
                if *s == target {
                    Ok(vec![self.integer(0)])
                } else {
                    Ok(vec![])
                }
            }
            ExprKind::Div(num, den) => {
                let num_roots = self.solveset(*num, target)?;
                Ok(filter_nonzero_den(self, num_roots, *den, target))
            }
            ExprKind::Mul(factors) => {
                // If any factor has a target-dependent denominator, clear denominators first
                if expr_has_target_denominator(self, eq, target) {
                    let (num, den) = to_rational_fraction(self, eq, target);
                    let num_roots = self.solveset(num, target)?;
                    return Ok(filter_nonzero_den(self, num_roots, den, target));
                }
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
            ExprKind::Pow(base, exp) => {
                if !self.has_symbol(*exp, target) {
                    if let Some(q) = eval_constant_rational(self, *exp) {
                        use num_traits::Signed as _;
                        if q.is_negative() || q == num_rational::BigRational::from_integer(0.into())
                        {
                            return Ok(vec![]);
                        }
                    }
                    self.solveset(*base, target)
                } else {
                    Ok(vec![])
                }
            }
            _ => {
                if expr_has_target_denominator(self, eq, target) {
                    let (num, den) = to_rational_fraction(self, eq, target);
                    let num_roots = self.solveset(num, target)?;
                    return Ok(filter_nonzero_den(self, num_roots, den, target));
                }
                if let Some(trans_roots) = solve_transcendental(self, eq, target) {
                    return Ok(trans_roots);
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
        use num_rational::BigRational;
        use num_traits::{ToPrimitive as _, Zero as _};

        let zero = self.integer(0);
        let two = self.integer(2);
        let ctx = crate::numeric::EvalContext::default();

        // Extract exact symbolic polynomial coefficients [c_0, c_1, ..., c_d] if `eq` is polynomial in `target`
        let Some(mut coeff_nodes) = extract_poly_coeffs(self, eq, target) else {
            // Fall back to RootOf placeholder for non-polynomial expressions (so Outcome classifier flags it)
            let target_name = self
                .symbols
                .resolve(target)
                .unwrap_or_else(|| "x".to_string());
            let target_node = self.symbol(&target_name);
            return Ok(vec![self.function("RootOf", [eq, target_node, self.integer(1)])]);
        };

        // Simplify coefficients and trim trailing symbolic zeros
        for c in &mut coeff_nodes {
            let simp = Simplifier::simplify(self, *c).unwrap_or(*c);
            *c = simp;
        }
        while coeff_nodes.len() > 1
            && coeff_nodes
                .last()
                .is_some_and(|&c| is_symbolic_zero(self, c))
        {
            coeff_nodes.pop();
        }

        let deg = coeff_nodes.len().saturating_sub(1);

        if deg == 0 {
            return Ok(vec![]);
        }

        // Check if every coefficient is an exact rational number
        let rational_coeffs: Option<Vec<BigRational>> = coeff_nodes
            .iter()
            .map(|&c| eval_constant_rational(self, c))
            .collect();

        if let Some(rcoeffs) = rational_coeffs {
            return solve_rational_polynomial(self, &rcoeffs, eq, target);
        }

        // Parametric / symbolic polynomial coefficients (contain free parameter symbols like a, b, c, y)
        if deg == 1 {
            let neg_c0 = self.neg(coeff_nodes[0]);
            let sol = self.div(neg_c0, coeff_nodes[1]);
            let simp = Simplifier::simplify(self, sol).unwrap_or(sol);
            return Ok(vec![simp]);
        }

        if deg == 2 {
            let a = coeff_nodes[2];
            let b = coeff_nodes[1];
            let c = coeff_nodes[0];
            return Ok(solve_symbolic_quadratic(self, a, b, c));
        }

        // Pure symbolic binomial: a * x^n + c = 0
        if deg >= 2 && (1..deg).all(|k| is_symbolic_zero(self, coeff_nodes[k])) {
            let neg_c = self.neg(coeff_nodes[0]);
            let ratio = self.div(neg_c, coeff_nodes[deg]);
            let simp_ratio = Simplifier::simplify(self, ratio).unwrap_or(ratio);
            let exp_frac = self.div(self.integer(1), self.integer(deg as i64));
            let root = self.pow(simp_ratio, exp_frac);
            if deg == 2 {
                let neg_root = self.neg(root);
                return Ok(vec![root, neg_root]);
            }
            return Ok(vec![root]);
        }

        // Fallback to numeric/Cardano for non-rational constant expressions (e.g. involving pi/e/sqrt)
        let get_coeff_val = |node_id: ExprId| -> Option<f64> {
            match &self.get(node_id).kind {
                ExprKind::Number(n) => n.as_f64(),
                _ => self.evalf(node_id, &ctx).ok().map(|v| v.to_f64()),
            }
        };

        let mut coeffs = Vec::with_capacity(coeff_nodes.len());
        for &cn in &coeff_nodes {
            if let Some(v) = get_coeff_val(cn)
                && v.is_finite()
            {
                coeffs.push(v);
            } else {
                let target_name = self
                    .symbols
                    .resolve(target)
                    .unwrap_or_else(|| "x".to_string());
                let target_node = self.symbol(&target_name);
                let mut root_of_nodes = Vec::with_capacity(deg);
                for k in 1..=deg {
                    root_of_nodes.push(self.function("RootOf", [eq, target_node, self.integer(k as i64)]));
                }
                return Ok(root_of_nodes);
            }
        }

        if deg == 3 {
            let a = coeffs[3];
            let b = coeffs[2];
            let c = coeffs[1];
            let d = coeffs[0];

            let p = (3.0 * a * c - b * b) / (3.0 * a * a);
            let q = (2.0 * b * b * b - 9.0 * a * b * c + 27.0 * a * a * d) / (27.0 * a * a * a);
            let delta = (q * q) / 4.0 + (p * p * p) / 27.0;
            let shift = -b / (3.0 * a);

            if delta >= 0.0 {
                let sqrt_delta = delta.sqrt();
                let u_val = -q / 2.0 + sqrt_delta;
                let v_val = -q / 2.0 - sqrt_delta;
                let u = u_val.signum() * u_val.abs().cbrt();
                let v = v_val.signum() * v_val.abs().cbrt();

                let real_node = self.float(u + v + shift);
                let half = self.div(self.integer(1), two);
                let sqrt3 = self.pow(self.integer(3), half);
                let i_sym = self.constant(algebra_core::Constant::I);

                let re_node = self.float(-0.5 * (u + v) + shift);
                let im_factor = self.float(0.5 * (u - v));
                let im_part = self.mul([im_factor, sqrt3, i_sym]);

                let c1 = self.add([re_node, im_part]);
                let c2 = self.sub(re_node, im_part);
                return Ok(vec![real_node, c1, c2]);
            } else {
                let r = (-p * p * p / 27.0).sqrt();
                let theta = (-q / (2.0 * r)).clamp(-1.0, 1.0).acos();
                let mult = 2.0 * (-p / 3.0).sqrt();
                let x1 = mult * (theta / 3.0).cos() + shift;
                let x2 = mult * ((theta + 2.0 * std::f64::consts::PI) / 3.0).cos() + shift;
                let x3 = mult * ((theta + 4.0 * std::f64::consts::PI) / 3.0).cos() + shift;
                let mk = |val: f64| {
                    let n = algebra_core::Number::from_f64_lossless(val);
                    self.intern(algebra_core::ExprNode::new(
                        algebra_core::Domain::Rationals,
                        ExprKind::Number(n),
                    ))
                };
                return Ok(vec![mk(x1), mk(x2), mk(x3)]);
            }
        }

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
        let _ = (zero, BigRational::zero().to_i64(), coeffs[deg].is_sign_positive());
        Ok(root_of_nodes)
    }
}

/// Exact transcendental equation solver ($e^x = c$, $\ln(x) = c$, $\sin(x) = c$, $x e^x = c$ via Lambert W).
fn solve_transcendental(graph: &ExprGraph, eq: ExprId, target: SymbolId) -> Option<Vec<ExprId>> {
    // Check for general radical equations sqrt(A(x)) + R(x) = 0 (where R may contain target)
    if let Some(roots) = solve_radical_equation(graph, eq, target) {
        return Some(roots);
    }

    // Check for multi-log combinations c1*ln(A(x)) + c2*ln(B(x)) + ... = C
    if let Some(roots) = solve_log_combination(graph, eq, target) {
        return Some(roots);
    }

    // Check for quadratic in e^x first: A*e^(2x) + B*e^x + C = 0
    if let Some(roots) = solve_transcendental_quadratic(graph, eq, target) {
        return Some(roots);
    }

    let node = graph.get(eq);
    let wrt_name = graph.symbols.resolve(target)?;
    let x_sym = graph.symbol(&wrt_name);

    // Decompose eq = 0 into T(x) = C (where C is independent of target)
    let (mut t_x, mut c_val) = match &node.kind {
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
        ExprKind::Function { .. } | ExprKind::Pow(_, _) => {
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

    c_val = Simplifier::simplify(graph, c_val).unwrap_or(c_val);

    // Peel off Neg(...) and constant multiplicative factors from t_x: k * f(x) = C => f(x) = C / k
    loop {
        match &graph.get(t_x).kind {
            ExprKind::Neg(inner) => {
                t_x = *inner;
                c_val = graph.neg(c_val);
                c_val = Simplifier::simplify(graph, c_val).unwrap_or(c_val);
            }
            ExprKind::Mul(factors) if factors.iter().any(|&f| !graph.has_symbol(f, target)) => {
                let mut x_factors = Vec::new();
                let mut const_factors = Vec::new();
                for &f in factors {
                    if graph.has_symbol(f, target) {
                        x_factors.push(f);
                    } else {
                        const_factors.push(f);
                    }
                }
                if x_factors.len() == 1 && !const_factors.is_empty() {
                    let k = if const_factors.len() == 1 {
                        const_factors[0]
                    } else {
                        graph.mul(const_factors)
                    };
                    if is_symbolic_zero(graph, k) {
                        return None;
                    }
                    t_x = x_factors[0];
                    let divided = graph.div(c_val, k);
                    c_val = Simplifier::simplify(graph, divided).unwrap_or(divided);
                } else {
                    break;
                }
            }
            _ => break,
        }
    }

    // Check if t_x is base^exp where base does not depend on target and exp depends on target
    let t_node = graph.get(t_x);
    if let ExprKind::Pow(base, exp) = &t_node.kind
        && !graph.has_symbol(*base, target)
        && graph.has_symbol(*exp, target)
    {
        if let Some(c_num) = eval_constant_real(graph, c_val) {
            if let Some(b_num) = eval_constant_real(graph, *base)
                && b_num > 0.0
                && c_num <= 0.0
            {
                return Some(vec![]);
            }
        }
        let rhs_expr = if let Some(q_log) = exact_rational_log(graph, *base, c_val) {
            crate::poly::expr_from_rational(graph, &q_log)
        } else {
            let ln_c = graph.function("ln", [c_val]);
            let ln_b = graph.function("ln", [*base]);
            graph.div(ln_c, ln_b)
        };
        if *exp == x_sym {
            return Some(vec![rhs_expr]);
        }
        let new_eq = graph.sub(*exp, rhs_expr);
        return graph.solveset(new_eq, target).ok();
    }

    // Check if t_x is a single transcendental/special function: F(arg) = C
    if let ExprKind::Function { name, args } = &t_node.kind {
        let fn_name = graph.symbols.resolve(*name)?;
        if args.len() == 1 {
            let arg = args[0];
            match fn_name.as_str() {
                "abs" => {
                    if let Some(v) = eval_constant_real(graph, c_val) {
                        if v < -1e-12 {
                            return Some(vec![]);
                        }
                        if v.abs() < 1e-12 {
                            return graph.solveset(arg, target).ok();
                        }
                    }
                    let mut roots = Vec::new();
                    let eq_pos = graph.sub(arg, c_val);
                    if let Ok(r_pos) = graph.solveset(eq_pos, target) {
                        for r in r_pos {
                            if !roots.contains(&r) {
                                roots.push(r);
                            }
                        }
                    }
                    let neg_c = graph.neg(c_val);
                    let eq_neg = graph.sub(arg, neg_c);
                    if let Ok(r_neg) = graph.solveset(eq_neg, target) {
                        for r in r_neg {
                            if !roots.contains(&r) {
                                roots.push(r);
                            }
                        }
                    }
                    return Some(roots);
                }
                "exp" => {
                    if let Some(v) = eval_constant_real(graph, c_val) {
                        if v.abs() < 1e-15 {
                            return Some(vec![]);
                        }
                        if (v + 1.0).abs() < 1e-12 {
                            let i_sym = graph.constant(algebra_core::Constant::I);
                            let pi_sym = graph.constant(algebra_core::Constant::Pi);
                            let i_pi = graph.mul([i_sym, pi_sym]);
                            if arg == x_sym {
                                return Some(vec![i_pi]);
                            }
                            let new_eq = graph.sub(arg, i_pi);
                            return graph.solveset(new_eq, target).ok();
                        }
                    }
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
                    let pi_sym = graph.constant(algebra_core::Constant::Pi);
                    let sup_c = graph.sub(pi_sym, asin_c);
                    let distinct_second = eval_constant_real(graph, c_val)
                        .map(|v| (v.abs() - 1.0).abs() > 1e-12 && v.abs() <= 1.0)
                        .unwrap_or(true);
                    if arg == x_sym {
                        return Some(if distinct_second {
                            vec![asin_c, sup_c]
                        } else {
                            vec![asin_c]
                        });
                    }
                    let mut all = graph.solveset(graph.sub(arg, asin_c), target).unwrap_or_default();
                    if distinct_second {
                        for r in graph.solveset(graph.sub(arg, sup_c), target).unwrap_or_default() {
                            if !all.contains(&r) {
                                all.push(r);
                            }
                        }
                    }
                    return Some(all);
                }
                "cos" => {
                    let acos_c = graph.function("acos", [c_val]);
                    let neg_acos_c = graph.neg(acos_c);
                    let distinct_second = eval_constant_real(graph, c_val)
                        .map(|v| (v.abs() - 1.0).abs() > 1e-12 && v.abs() <= 1.0)
                        .unwrap_or(true);
                    if arg == x_sym {
                        return Some(if distinct_second {
                            vec![acos_c, neg_acos_c]
                        } else {
                            vec![acos_c]
                        });
                    }
                    let mut all = graph.solveset(graph.sub(arg, acos_c), target).unwrap_or_default();
                    if distinct_second {
                        for r in graph.solveset(graph.sub(arg, neg_acos_c), target).unwrap_or_default() {
                            if !all.contains(&r) {
                                all.push(r);
                            }
                        }
                    }
                    return Some(all);
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
                    if let Some(v) = eval_constant_real(graph, c_val)
                        && v < -1e-12
                    {
                        return Some(vec![]);
                    }
                    let c_sq = if let Some(q) = eval_constant_rational(graph, c_val) {
                        crate::poly::expr_from_rational(graph, &(&q * &q))
                    } else {
                        let pow_node = graph.pow(c_val, graph.integer(2));
                        Simplifier::simplify(graph, pow_node).unwrap_or(pow_node)
                    };
                    let candidates = if arg == x_sym {
                        vec![c_sq]
                    } else {
                        let new_eq = graph.sub(arg, c_sq);
                        graph.solveset(new_eq, target).ok()?
                    };
                    // Verify candidates against principal square root
                    if let Some(target_c) = eval_constant_real(graph, c_val) {
                        let filtered: Vec<ExprId> = candidates
                            .into_iter()
                            .filter(|&r| {
                                let sub_arg = graph.substitute(arg, target, r);
                                if let Some(a_val) = eval_constant_real(graph, sub_arg) {
                                    a_val >= -1e-12 && (a_val.max(0.0).sqrt() - target_c).abs() < 1e-8
                                } else {
                                    true
                                }
                            })
                            .collect();
                        return Some(filtered);
                    }
                    return Some(candidates);
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
                        let w0 = graph.function("lambert_w", [c_val]);
                        let inv_e = -(-1.0f64).exp();
                        if let Some(cv) = eval_constant_real(graph, c_val)
                            && cv > inv_e + 1e-12
                            && cv < 0.0
                        {
                            let w_neg1 = graph.function("lambert_w", [graph.integer(-1), c_val]);
                            return Some(vec![w0, w_neg1]);
                        }
                        return Some(vec![w0]);
                    } else if (fn_name == "ln" || fn_name == "log")
                        && args.len() == 1
                        && args[0] == x_sym
                    {
                        let w0 = graph.function("lambert_w", [c_val]);
                        let exp_w0 = graph.function("exp", [w0]);
                        let inv_e = -(-1.0f64).exp();
                        if let Some(cv) = eval_constant_real(graph, c_val)
                            && cv > inv_e + 1e-12
                            && cv < 0.0
                        {
                            let w_neg1 = graph.function("lambert_w", [graph.integer(-1), c_val]);
                            let exp_w_neg1 = graph.function("exp", [w_neg1]);
                            return Some(vec![exp_w0, exp_w_neg1]);
                        }
                        return Some(vec![exp_w0]);
                    }
                }
            }
        }
    }

    None
}

fn eval_constant_real(graph: &ExprGraph, id: ExprId) -> Option<f64> {
    let mut syms = std::collections::BTreeSet::new();
    crate::poly::collect_expr_symbols(graph, id, &mut syms);
    if !syms.is_empty() {
        return None;
    }
    let ctx = crate::numeric::EvalContext::default();
    let v = graph.evalf(id, &ctx).ok()?.to_f64();
    if v.is_finite() { Some(v) } else { None }
}

fn eval_constant_rational(graph: &ExprGraph, id: ExprId) -> Option<num_rational::BigRational> {
    use num_traits::Zero as _;
    match &graph.get(id).kind {
        ExprKind::Number(n) => crate::poly::rational_from_number(n).ok(),
        ExprKind::Neg(inner) => Some(-eval_constant_rational(graph, *inner)?),
        ExprKind::Add(terms) => {
            let mut acc = num_rational::BigRational::zero();
            for &t in terms {
                acc += eval_constant_rational(graph, t)?;
            }
            Some(acc)
        }
        ExprKind::Sub(a, b) => {
            Some(eval_constant_rational(graph, *a)? - eval_constant_rational(graph, *b)?)
        }
        ExprKind::Mul(factors) => {
            let mut acc = num_rational::BigRational::from_integer(1.into());
            for &f in factors {
                acc *= eval_constant_rational(graph, f)?;
            }
            Some(acc)
        }
        ExprKind::Div(a, b) => {
            let da = eval_constant_rational(graph, *a)?;
            let db = eval_constant_rational(graph, *b)?;
            if db.is_zero() { None } else { Some(da / db) }
        }
        ExprKind::Pow(b, e) => {
            let qb = eval_constant_rational(graph, *b)?;
            let qe = eval_constant_rational(graph, *e)?;
            if !qe.is_integer() {
                return None;
            }
            let exp_i32: i32 = qe.numer().try_into().ok()?;
            if exp_i32.abs() > 64 {
                return None;
            }
            if qb.is_zero() && exp_i32 < 0 {
                return None;
            }
            Some(qb.pow(exp_i32))
        }
        _ => None,
    }
}

fn rational_pow_i32(q: &num_rational::BigRational, exp: i32) -> num_rational::BigRational {
    if exp >= 0 {
        q.pow(exp)
    } else {
        q.recip().pow(-exp)
    }
}

fn exact_rational_log(
    graph: &ExprGraph,
    base: ExprId,
    val: ExprId,
) -> Option<num_rational::BigRational> {
    use num_bigint::BigInt;
    use num_rational::BigRational;
    use num_traits::{One as _, Signed as _};

    let b = eval_constant_rational(graph, base)?;
    let v = eval_constant_rational(graph, val)?;
    if !b.is_positive() || b.is_one() || !v.is_positive() {
        return None;
    }

    // Check b^k == v directly for small integer k
    for k in -64..=64i32 {
        if rational_pow_i32(&b, k) == v {
            return Some(BigRational::from_integer(BigInt::from(k)));
        }
    }

    // Check if b = u^g (for g in 2..=6) and v = u^m => log_b(v) = m / g
    let bn = b.numer();
    let bd = b.denom();
    for g in 2..=6u32 {
        let rn = bn.nth_root(g);
        let rd = bd.nth_root(g);
        if &rn.pow(g) == bn && &rd.pow(g) == bd {
            let u = BigRational::new(rn, rd);
            for m in -64..=64i32 {
                if rational_pow_i32(&u, m) == v {
                    return Some(BigRational::new(BigInt::from(m), BigInt::from(g)));
                }
            }
        }
    }
    None
}

fn expr_has_target_denominator(graph: &ExprGraph, expr: ExprId, target: SymbolId) -> bool {
    if !graph.has_symbol(expr, target) {
        return false;
    }
    match &graph.get(expr).kind {
        ExprKind::Div(_, den) => {
            graph.has_symbol(*den, target) || expr_has_target_denominator(graph, *den, target)
        }
        ExprKind::Pow(_, _) => false,
        ExprKind::Neg(inner) => expr_has_target_denominator(graph, *inner, target),
        ExprKind::Add(items) | ExprKind::Mul(items) => items
            .iter()
            .any(|&it| expr_has_target_denominator(graph, it, target)),
        ExprKind::Sub(a, b) => {
            expr_has_target_denominator(graph, *a, target)
                || expr_has_target_denominator(graph, *b, target)
        }
        _ => false,
    }
}

fn to_rational_fraction(graph: &ExprGraph, expr: ExprId, target: SymbolId) -> (ExprId, ExprId) {
    let one = graph.integer(1);
    if !graph.has_symbol(expr, target) {
        return (expr, one);
    }
    match &graph.get(expr).kind {
        ExprKind::Div(num, den) => {
            let (na, da) = to_rational_fraction(graph, *num, target);
            let (nb, db) = to_rational_fraction(graph, *den, target);
            let new_num = if db == one { na } else { graph.mul([na, db]) };
            let new_den = if da == one { nb } else { graph.mul([da, nb]) };
            (new_num, new_den)
        }
        ExprKind::Neg(inner) => {
            let (n, d) = to_rational_fraction(graph, *inner, target);
            (graph.neg(n), d)
        }
        ExprKind::Sub(a, b) => {
            let (na, da) = to_rational_fraction(graph, *a, target);
            let (nb, db) = to_rational_fraction(graph, *b, target);
            if da == db {
                (graph.sub(na, nb), da)
            } else {
                let lhs = if db == one { na } else { graph.mul([na, db]) };
                let rhs = if da == one { nb } else { graph.mul([nb, da]) };
                let den = if da == one {
                    db
                } else if db == one {
                    da
                } else {
                    graph.mul([da, db])
                };
                (graph.sub(lhs, rhs), den)
            }
        }
        ExprKind::Add(terms) => {
            let mut acc = to_rational_fraction(graph, terms[0], target);
            for &t in &terms[1..] {
                let (nb, db) = to_rational_fraction(graph, t, target);
                let (na, da) = acc;
                if da == db {
                    acc = (graph.add([na, nb]), da);
                } else {
                    let lhs = if db == one { na } else { graph.mul([na, db]) };
                    let rhs = if da == one { nb } else { graph.mul([nb, da]) };
                    let den = if da == one {
                        db
                    } else if db == one {
                        da
                    } else {
                        graph.mul([da, db])
                    };
                    acc = (graph.add([lhs, rhs]), den);
                }
            }
            acc
        }
        ExprKind::Mul(factors) => {
            let mut nums = Vec::new();
            let mut dens = Vec::new();
            for &f in factors {
                let (nf, df) = to_rational_fraction(graph, f, target);
                nums.push(nf);
                if df != one {
                    dens.push(df);
                }
            }
            let num = if nums.len() == 1 {
                nums[0]
            } else {
                graph.mul(nums)
            };
            let den = if dens.is_empty() {
                one
            } else if dens.len() == 1 {
                dens[0]
            } else {
                graph.mul(dens)
            };
            (num, den)
        }
        _ => (expr, one),
    }
}

fn filter_nonzero_den(
    graph: &ExprGraph,
    roots: Vec<ExprId>,
    den: ExprId,
    target: SymbolId,
) -> Vec<ExprId> {
    roots
        .into_iter()
        .filter(|&r| {
            let sub = graph.substitute(den, target, r);
            !is_symbolic_zero(graph, sub)
        })
        .collect()
}

fn is_symbolic_zero(graph: &ExprGraph, expr: ExprId) -> bool {
    use num_traits::Zero as _;
    let s = Simplifier::simplify(graph, expr).unwrap_or(expr);
    if s == graph.integer(0) {
        return true;
    }
    if let Some(q) = eval_constant_rational(graph, s) {
        return q.is_zero();
    }
    // Expand polynomially if univariate/multivariate expressions cancel out (e.g. (-2*y)^2 - 4*y^2)
    let expanded = expand_poly_expr_zero_check(graph, s);
    if expanded {
        return true;
    }
    if let Some(v) = eval_constant_real(graph, s) {
        return v.abs() < 1e-12;
    }
    false
}

fn expand_poly_expr_zero_check(graph: &ExprGraph, expr: ExprId) -> bool {
    let mut sym_set = std::collections::BTreeSet::new();
    crate::poly::collect_expr_symbols(graph, expr, &mut sym_set);
    if sym_set.is_empty() {
        return false;
    }
    let syms: Vec<SymbolId> = sym_set.into_iter().collect();
    // Expand iteratively across all symbols until no symbols remain; check if leaf is zero
    fn check_rec(graph: &ExprGraph, e: ExprId, syms: &[SymbolId]) -> bool {
        if syms.is_empty() {
            return eval_constant_rational(graph, e).is_some_and(|q| {
                use num_traits::Zero as _;
                q.is_zero()
            });
        }
        let Some(coeffs) = extract_poly_coeffs(graph, e, syms[0]) else {
            return false;
        };
        coeffs.iter().all(|&c| check_rec(graph, c, &syms[1..]))
    }
    check_rec(graph, expr, &syms)
}

fn poly_add_exprs(graph: &ExprGraph, a: &[ExprId], b: &[ExprId]) -> Vec<ExprId> {
    let len = a.len().max(b.len());
    let zero = graph.integer(0);
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let ca = *a.get(i).unwrap_or(&zero);
        let cb = *b.get(i).unwrap_or(&zero);
        if ca == zero {
            out.push(cb);
        } else if cb == zero {
            out.push(ca);
        } else {
            out.push(graph.add([ca, cb]));
        }
    }
    out
}

fn poly_sub_exprs(graph: &ExprGraph, a: &[ExprId], b: &[ExprId]) -> Vec<ExprId> {
    let len = a.len().max(b.len());
    let zero = graph.integer(0);
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let ca = *a.get(i).unwrap_or(&zero);
        let cb = *b.get(i).unwrap_or(&zero);
        if cb == zero {
            out.push(ca);
        } else if ca == zero {
            out.push(graph.neg(cb));
        } else {
            out.push(graph.sub(ca, cb));
        }
    }
    out
}

fn poly_mul_exprs(graph: &ExprGraph, a: &[ExprId], b: &[ExprId]) -> Vec<ExprId> {
    let zero = graph.integer(0);
    if a.is_empty() || b.is_empty() {
        return vec![zero];
    }
    let mut out = vec![zero; a.len() + b.len() - 1];
    for (i, &ca) in a.iter().enumerate() {
        if ca == zero {
            continue;
        }
        for (j, &cb) in b.iter().enumerate() {
            if cb == zero {
                continue;
            }
            let prod = graph.mul([ca, cb]);
            if out[i + j] == zero {
                out[i + j] = prod;
            } else {
                out[i + j] = graph.add([out[i + j], prod]);
            }
        }
    }
    out
}

fn extract_poly_coeffs(graph: &ExprGraph, expr: ExprId, target: SymbolId) -> Option<Vec<ExprId>> {
    use num_traits::{Signed as _, ToPrimitive as _};
    if !graph.has_symbol(expr, target) {
        return Some(vec![expr]);
    }
    match &graph.get(expr).kind {
        ExprKind::Symbol(s) if *s == target => Some(vec![graph.integer(0), graph.integer(1)]),
        ExprKind::Neg(inner) => {
            let p = extract_poly_coeffs(graph, *inner, target)?;
            Some(p.into_iter().map(|c| graph.neg(c)).collect())
        }
        ExprKind::Add(terms) => {
            let mut acc = vec![graph.integer(0)];
            for &t in terms {
                let pt = extract_poly_coeffs(graph, t, target)?;
                acc = poly_add_exprs(graph, &acc, &pt);
            }
            Some(acc)
        }
        ExprKind::Sub(a, b) => {
            let pa = extract_poly_coeffs(graph, *a, target)?;
            let pb = extract_poly_coeffs(graph, *b, target)?;
            Some(poly_sub_exprs(graph, &pa, &pb))
        }
        ExprKind::Mul(factors) => {
            let mut acc = vec![graph.integer(1)];
            for &f in factors {
                let pf = extract_poly_coeffs(graph, f, target)?;
                acc = poly_mul_exprs(graph, &acc, &pf);
            }
            Some(acc)
        }
        ExprKind::Div(num, den) => {
            if graph.has_symbol(*den, target) {
                return None;
            }
            let pn = extract_poly_coeffs(graph, *num, target)?;
            Some(pn.into_iter().map(|c| graph.div(c, *den)).collect())
        }
        ExprKind::Pow(base, exp) => {
            if graph.has_symbol(*exp, target) {
                return None;
            }
            let q = eval_constant_rational(graph, *exp)?;
            if !q.is_integer() || q.is_negative() {
                return None;
            }
            let n = q.numer().to_u32()?;
            if n > 24 {
                return None;
            }
            let pb = extract_poly_coeffs(graph, *base, target)?;
            let mut acc = vec![graph.integer(1)];
            for _ in 0..n {
                acc = poly_mul_exprs(graph, &acc, &pb);
            }
            Some(acc)
        }
        _ => None,
    }
}

fn extract_square_factor_bigint(n: &num_bigint::BigInt) -> (num_bigint::BigInt, num_bigint::BigInt) {
    use num_bigint::BigInt;
    use num_traits::{One as _, Zero as _};
    let mut k = BigInt::one();
    let mut rem = n.clone();
    if rem.is_zero() || rem.is_one() {
        return (BigInt::one(), rem);
    }
    let four = BigInt::from(4);
    while (&rem % &four).is_zero() {
        k *= 2;
        rem /= 4;
    }
    let mut p = 3u64;
    while p <= 10_000 {
        let p2 = BigInt::from(p * p);
        if p2 > rem {
            break;
        }
        while (&rem % &p2).is_zero() {
            k *= p;
            rem /= &p2;
        }
        p += 2;
    }
    (k, rem)
}

fn build_rational_radical_term(
    graph: &ExprGraph,
    coeff: &num_rational::BigRational,
    radicand: &num_bigint::BigInt,
    imag: bool,
) -> ExprId {
    use num_traits::One as _;
    let half = graph.div(graph.integer(1), graph.integer(2));
    let i_sym = graph.constant(algebra_core::Constant::I);
    let mut factors = Vec::new();
    let c_abs = if coeff < &num_rational::BigRational::from_integer(0.into()) {
        -coeff
    } else {
        coeff.clone()
    };
    let is_neg = coeff < &num_rational::BigRational::from_integer(0.into());

    if !c_abs.is_one() || (radicand.is_one() && !imag) {
        factors.push(crate::poly::expr_from_rational(graph, &c_abs));
    }
    if !radicand.is_one() {
        let r_expr = crate::poly::expr_from_rational(
            graph,
            &num_rational::BigRational::from_integer(radicand.clone()),
        );
        factors.push(graph.pow(r_expr, half));
    }
    if imag {
        factors.push(i_sym);
    }
    let body = if factors.is_empty() {
        graph.integer(1)
    } else if factors.len() == 1 {
        factors[0]
    } else {
        graph.mul(factors)
    };
    if is_neg { graph.neg(body) } else { body }
}

fn exact_quadratic_roots(
    graph: &ExprGraph,
    a: &num_rational::BigRational,
    b: &num_rational::BigRational,
    c: &num_rational::BigRational,
) -> Vec<ExprId> {
    use num_bigint::BigInt;
    use num_rational::BigRational;
    use num_traits::{One as _, Signed as _, Zero as _};

    let two = BigInt::from(2);
    let four = BigInt::from(4);
    let disc = b * b - BigRational::from_integer(four) * a * c;
    let two_a = BigRational::from_integer(two) * a;
    let p0 = -b / &two_a;

    if disc.is_zero() {
        return vec![crate::poly::expr_from_rational(graph, &p0)];
    }

    let abs_disc = disc.abs();
    let combined = abs_disc.numer() * abs_disc.denom();
    let (k, sqfree) = extract_square_factor_bigint(&combined);
    let q0 = BigRational::new(k, abs_disc.denom().clone()) / &two_a;

    if disc.is_positive() && sqfree.is_one() {
        let r1 = &p0 + &q0;
        let r2 = &p0 - &q0;
        return vec![
            crate::poly::expr_from_rational(graph, &r1),
            crate::poly::expr_from_rational(graph, &r2),
        ];
    }

    let imag = disc.is_negative();
    let q0_pos = q0.abs();
    let rad_term = build_rational_radical_term(graph, &q0_pos, &sqfree, imag);
    if p0.is_zero() {
        vec![rad_term, graph.neg(rad_term)]
    } else {
        let p0_expr = crate::poly::expr_from_rational(graph, &p0);
        vec![graph.add([p0_expr, rad_term]), graph.sub(p0_expr, rad_term)]
    }
}

fn solve_symbolic_quadratic(graph: &ExprGraph, a: ExprId, b: ExprId, c: ExprId) -> Vec<ExprId> {
    let b_sq = graph.pow(b, graph.integer(2));
    let four_ac = graph.mul([graph.integer(4), a, c]);
    let raw_disc = graph.sub(b_sq, four_ac);
    let disc = Simplifier::simplify(graph, raw_disc).unwrap_or(raw_disc);
    let two_a = Simplifier::simplify(graph, graph.mul([graph.integer(2), a]))
        .unwrap_or_else(|_| graph.mul([graph.integer(2), a]));
    let neg_b = Simplifier::simplify(graph, graph.neg(b)).unwrap_or_else(|_| graph.neg(b));

    if is_symbolic_zero(graph, disc) {
        let r = graph.div(neg_b, two_a);
        return vec![Simplifier::simplify(graph, r).unwrap_or(r)];
    }

    // Check exact symbolic perfect square discriminant (e.g. 4*y^2 -> 2*y, giving roots [y, -y])
    if let Some(sqrt_exact) = extract_exact_symbolic_sqrt(graph, disc) {
        let num1 = graph.add([neg_b, sqrt_exact]);
        let num2 = graph.sub(neg_b, sqrt_exact);
        let sol1 = Simplifier::simplify(graph, graph.div(num1, two_a))
            .unwrap_or_else(|_| graph.div(num1, two_a));
        let sol2 = Simplifier::simplify(graph, graph.div(num2, two_a))
            .unwrap_or_else(|_| graph.div(num2, two_a));
        return if sol1 == sol2 {
            vec![sol1]
        } else {
            vec![sol1, sol2]
        };
    }

    let half = graph.div(graph.integer(1), graph.integer(2));
    // Check sign when discriminant is constant real (e.g. y_k^2 - 4 in x^4+x^3+x^2+x+1=0)
    let radical_part = if let Some(d_val) = eval_constant_real(graph, disc)
        && d_val < -1e-12
    {
        let pos_disc = Simplifier::simplify(graph, graph.neg(disc)).unwrap_or_else(|_| graph.neg(disc));
        let sqrt_pos = graph.pow(pos_disc, half);
        let i_sym = graph.constant(algebra_core::Constant::I);
        graph.mul([sqrt_pos, i_sym])
    } else {
        graph.pow(disc, half)
    };

    let num1 = graph.add([neg_b, radical_part]);
    let num2 = graph.sub(neg_b, radical_part);
    let sol1 = graph.div(num1, two_a);
    let sol2 = graph.div(num2, two_a);
    vec![
        Simplifier::simplify(graph, sol1).unwrap_or(sol1),
        Simplifier::simplify(graph, sol2).unwrap_or(sol2),
    ]
}

fn bigint_divisors(n: &num_bigint::BigInt, cap: usize) -> Vec<num_bigint::BigInt> {
    use num_bigint::BigInt;
    use num_traits::{Signed as _, ToPrimitive as _};
    let n_abs = n.abs();
    let Some(v) = n_abs.to_u64() else {
        return vec![BigInt::from(1)];
    };
    if v == 0 {
        return vec![BigInt::from(1)];
    }
    let mut small = Vec::new();
    let mut large = Vec::new();
    let limit = (v as f64).sqrt() as u64 + 1;
    for d in 1..=limit {
        if d * d > v {
            break;
        }
        if v % d == 0 {
            small.push(BigInt::from(d));
            if d * d != v {
                large.push(BigInt::from(v / d));
            }
            if small.len() + large.len() >= cap {
                break;
            }
        }
    }
    small.extend(large.into_iter().rev());
    small
}

fn eval_rational_poly(coeffs: &[num_rational::BigRational], x: &num_rational::BigRational) -> num_rational::BigRational {
    use num_traits::Zero as _;
    let mut acc = num_rational::BigRational::zero();
    for c in coeffs.iter().rev() {
        acc = &acc * x + c;
    }
    acc
}

fn deflate_rational_poly(
    coeffs: &[num_rational::BigRational],
    root: &num_rational::BigRational,
) -> Vec<num_rational::BigRational> {
    use num_traits::Zero as _;
    let deg = coeffs.len() - 1;
    let mut out = vec![num_rational::BigRational::zero(); deg];
    let mut rem = coeffs[deg].clone();
    out[deg - 1] = rem.clone();
    for k in (1..deg).rev() {
        rem = &rem * root + &coeffs[k];
        out[k - 1] = rem.clone();
    }
    out
}

fn rational_roots_deflated(
    coeffs: &[num_rational::BigRational],
) -> (Vec<num_rational::BigRational>, Vec<num_rational::BigRational>) {
    use num::Integer as _;
    use num_bigint::BigInt;
    use num_rational::BigRational;
    use num_traits::{One as _, Zero as _};

    let mut cur = coeffs.to_vec();
    while cur.len() > 1 && cur.last().is_some_and(|c| c.is_zero()) {
        cur.pop();
    }

    let mut found = Vec::new();
    while cur.len() > 1 && cur[0].is_zero() {
        let z = BigRational::zero();
        if !found.contains(&z) {
            found.push(z);
        }
        cur.remove(0);
    }

    loop {
        let deg = cur.len().saturating_sub(1);
        if deg == 0 {
            break;
        }
        let mut den_lcm = BigInt::one();
        for c in &cur {
            den_lcm = den_lcm.lcm(c.denom());
        }
        let int_coeffs: Vec<BigInt> = cur
            .iter()
            .map(|c| (c * BigRational::from_integer(den_lcm.clone())).to_integer())
            .collect();
        let p_divs = bigint_divisors(&int_coeffs[0], 64);
        let q_divs = bigint_divisors(&int_coeffs[deg], 64);

        let mut hit: Option<BigRational> = None;
        'outer: for p in &p_divs {
            for q in &q_divs {
                for sign in [1i8, -1i8] {
                    let cand = if sign > 0 {
                        BigRational::new(p.clone(), q.clone())
                    } else {
                        BigRational::new(-p.clone(), q.clone())
                    };
                    if eval_rational_poly(&cur, &cand).is_zero() {
                        hit = Some(cand);
                        break 'outer;
                    }
                }
            }
        }
        if let Some(r) = hit {
            if !found.contains(&r) {
                found.push(r.clone());
            }
            cur = deflate_rational_poly(&cur, &r);
            while cur.len() > 1 && cur.last().is_some_and(|c| c.is_zero()) {
                cur.pop();
            }
        } else {
            break;
        }
    }

    (found, cur)
}

fn solve_rational_polynomial(
    graph: &ExprGraph,
    rcoeffs: &[num_rational::BigRational],
    eq: ExprId,
    target: SymbolId,
) -> AlgebraResult<Vec<ExprId>> {
    use num_bigint::BigInt;
    use num_rational::BigRational;
    use num_traits::{Signed as _, ToPrimitive as _, Zero as _};

    let (rat_roots, rem) = rational_roots_deflated(rcoeffs);
    let mut results: Vec<ExprId> = rat_roots
        .iter()
        .map(|q| crate::poly::expr_from_rational(graph, q))
        .collect();

    let deg = rem.len().saturating_sub(1);
    if deg == 0 {
        return Ok(results);
    }

    if deg == 1 {
        let r = -&rem[0] / &rem[1];
        let r_id = crate::poly::expr_from_rational(graph, &r);
        if !results.contains(&r_id) {
            results.push(r_id);
        }
        return Ok(results);
    }

    if deg == 2 {
        for r in exact_quadratic_roots(graph, &rem[2], &rem[1], &rem[0]) {
            if !results.contains(&r) {
                results.push(r);
            }
        }
        return Ok(results);
    }

    // Pure binomial a * x^n + c = 0
    if deg >= 3 && (1..deg).all(|k| rem[k].is_zero()) && !rem[0].is_zero() {
        let r_val = -&rem[0] / &rem[deg];
        if deg == 3 {
            let abs_r = r_val.abs();
            let base = crate::poly::expr_from_rational(graph, &abs_r);
            let one_third = graph.div(graph.integer(1), graph.integer(3));
            let real_root = if r_val.is_negative() {
                graph.neg(graph.pow(base, one_third))
            } else {
                graph.pow(base, one_third)
            };
            results.push(real_root);
            return Ok(results);
        } else if deg == 4 && r_val.is_positive() {
            let one_fourth = graph.div(graph.integer(1), graph.integer(4));
            let base = crate::poly::expr_from_rational(graph, &r_val);
            let r0 = graph.pow(base, one_fourth);
            let i_sym = graph.constant(algebra_core::Constant::I);
            let ri = graph.mul([r0, i_sym]);
            results.extend([r0, graph.neg(r0), ri, graph.neg(ri)]);
            return Ok(results);
        }
    }

    // Biquadratic a*x^4 + b*x^2 + c = 0
    if deg == 4 && rem[3].is_zero() && rem[1].is_zero() {
        let u_roots = exact_quadratic_roots(graph, &rem[4], &rem[2], &rem[0]);
        for u in u_roots {
            for r in solve_symbolic_quadratic(graph, graph.integer(1), graph.integer(0), graph.neg(u)) {
                if !results.contains(&r) {
                    results.push(r);
                }
            }
        }
        return Ok(results);
    }

    // Palindromic / Reciprocal Quartic: a*x^4 + b*x^3 + c*x^2 + b*x + a = 0
    if deg == 4 && rem[4] == rem[0] && rem[3] == rem[1] {
        let a = &rem[4];
        let b = &rem[3];
        let c_const = &rem[2] - BigRational::from_integer(BigInt::from(2)) * a;
        let y_roots = exact_quadratic_roots(graph, a, b, &c_const);
        for y in y_roots {
            let neg_y = graph.neg(y);
            for r in solve_symbolic_quadratic(graph, graph.integer(1), neg_y, graph.integer(1)) {
                if !results.contains(&r) {
                    results.push(r);
                }
            }
        }
        return Ok(results);
    }

    // Numeric Cardano fallback for irreducible deg == 3 rational cubics
    if deg == 3
        && let (Some(a), Some(b), Some(c), Some(d)) = (
            rem[3].to_f64(),
            rem[2].to_f64(),
            rem[1].to_f64(),
            rem[0].to_f64(),
        )
    {
        let p = (3.0 * a * c - b * b) / (3.0 * a * a);
        let q = (2.0 * b * b * b - 9.0 * a * b * c + 27.0 * a * a * d) / (27.0 * a * a * a);
        let delta = (q * q) / 4.0 + (p * p * p) / 27.0;
        let shift = -b / (3.0 * a);
        if delta >= 0.0 {
            let sd = delta.sqrt();
            let u_val = -q / 2.0 + sd;
            let v_val = -q / 2.0 - sd;
            let u = u_val.signum() * u_val.abs().cbrt();
            let v = v_val.signum() * v_val.abs().cbrt();
            let half = graph.div(graph.integer(1), graph.integer(2));
            let sqrt3 = graph.pow(graph.integer(3), half);
            let i_sym = graph.constant(algebra_core::Constant::I);
            let re = graph.float(-0.5 * (u + v) + shift);
            let im = graph.mul([graph.float(0.5 * (u - v)), sqrt3, i_sym]);
            results.push(graph.float(u + v + shift));
            results.push(graph.add([re, im]));
            results.push(graph.sub(re, im));
            return Ok(results);
        } else {
            let r = (-p * p * p / 27.0).sqrt();
            let theta = (-q / (2.0 * r)).clamp(-1.0, 1.0).acos();
            let mult = 2.0 * (-p / 3.0).sqrt();
            for m in 0..3 {
                let val = mult * ((theta + 2.0 * (m as f64) * std::f64::consts::PI) / 3.0).cos() + shift;
                results.push(graph.float(val));
            }
            return Ok(results);
        }
    }

    // Exact Bring radical for Bring-Jerrard quintic c5*x^5 + c1*x + c0 = 0
    if deg == 5 && rem[4].is_zero() && rem[3].is_zero() && rem[2].is_zero() && !rem[1].is_zero() {
        let a_lead = &rem[5];
        let b_lin = &rem[1];
        let c_const = &rem[0];
        if a_lead == b_lin {
            let a_param = c_const / a_lead;
            let a_expr = crate::poly::expr_from_rational(graph, &a_param);
            results.push(graph.function("BringRadical", [a_expr]));
            return Ok(results);
        }
        let ratio = b_lin / a_lead;
        if !ratio.is_negative() {
            // t = (b/a)^(1/4), a_param = c / (a * t^5) -> root = t * BR(a_param)
            let half = graph.div(graph.integer(1), graph.integer(4));
            let t_expr = graph.pow(crate::poly::expr_from_rational(graph, &ratio), half);
            let denom = graph.mul([
                crate::poly::expr_from_rational(graph, a_lead),
                graph.pow(t_expr, graph.integer(5)),
            ]);
            let arg = graph.div(crate::poly::expr_from_rational(graph, c_const), denom);
            let br = graph.function("BringRadical", [arg]);
            results.push(graph.mul([t_expr, br]));
            return Ok(results);
        }
    }

    // Irreducible higher-degree polynomials -> exact RootOf placeholders
    let target_name = graph
        .symbols
        .resolve(target)
        .unwrap_or_else(|| "x".to_string());
    let target_node = graph.symbol(&target_name);
    for k in 1..=deg {
        results.push(graph.function("RootOf", [eq, target_node, graph.integer(k as i64)]));
    }
    Ok(results)
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

fn flatten_signed_add_terms(graph: &ExprGraph, id: ExprId, sign: i64, out: &mut Vec<(i64, ExprId)>) {
    match &graph.get(id).kind {
        ExprKind::Add(ts) => {
            for &t in ts {
                flatten_signed_add_terms(graph, t, sign, out);
            }
        }
        ExprKind::Sub(a, b) => {
            flatten_signed_add_terms(graph, *a, sign, out);
            flatten_signed_add_terms(graph, *b, -sign, out);
        }
        ExprKind::Neg(inner) => flatten_signed_add_terms(graph, *inner, -sign, out),
        _ => out.push((sign, id)),
    }
}

fn match_sqrt_term(graph: &ExprGraph, term: ExprId, target: SymbolId) -> Option<(ExprId, ExprId)> {
    let one = graph.integer(1);
    match &graph.get(term).kind {
        ExprKind::Function { name, args }
            if graph.symbols.resolve(*name).as_deref() == Some("sqrt")
                && args.len() == 1
                && graph.has_symbol(args[0], target) =>
        {
            Some((one, args[0]))
        }
        ExprKind::Pow(base, exp)
            if graph.has_symbol(*base, target)
                && eval_constant_rational(graph, *exp)
                    == Some(num_rational::BigRational::new(1.into(), 2.into())) =>
        {
            Some((one, *base))
        }
        ExprKind::Mul(fs) => {
            let mut coeff = Vec::new();
            let mut rad = None;
            for &f in fs {
                if rad.is_none()
                    && let Some((c1, r)) = match_sqrt_term(graph, f, target)
                {
                    if c1 != one {
                        coeff.push(c1);
                    }
                    rad = Some(r);
                } else if graph.has_symbol(f, target) {
                    return None;
                } else {
                    coeff.push(f);
                }
            }
            let r = rad?;
            let c = if coeff.is_empty() {
                one
            } else if coeff.len() == 1 {
                coeff[0]
            } else {
                graph.mul(coeff)
            };
            Some((c, r))
        }
        _ => None,
    }
}

fn collect_log_args(graph: &ExprGraph, id: ExprId, target: SymbolId, out: &mut Vec<ExprId>) {
    match &graph.get(id).kind {
        ExprKind::Function { name, args }
            if matches!(graph.symbols.resolve(*name).as_deref(), Some("ln" | "log"))
                && args.len() == 1
                && graph.has_symbol(args[0], target) =>
        {
            out.push(args[0]);
        }
        ExprKind::Add(ts) | ExprKind::Mul(ts) => {
            for &t in ts {
                collect_log_args(graph, t, target, out);
            }
        }
        ExprKind::Sub(a, b) | ExprKind::Div(a, b) | ExprKind::Pow(a, b) => {
            collect_log_args(graph, *a, target, out);
            collect_log_args(graph, *b, target, out);
        }
        ExprKind::Neg(inner) => collect_log_args(graph, *inner, target, out),
        _ => {}
    }
}

fn verify_candidate_root_numeric(graph: &ExprGraph, eq: ExprId, target: SymbolId, root: ExprId) -> bool {
    use crate::numeric::NumericalEval as _;
    let ctx = crate::numeric::EvalContext::default();
    let mut log_args = Vec::new();
    collect_log_args(graph, eq, target, &mut log_args);
    for arg in log_args {
        let sub_arg = graph.substitute(arg, target, root);
        if let Ok(v) = graph.evalf(sub_arg, &ctx) {
            let (re, im) = match v {
                crate::numeric::BigValue::Real(r) => (r, 0.0),
                crate::numeric::BigValue::Complex(r, i) => (r, i),
            };
            if im.abs() > 1e-7 || re <= 1e-12 {
                return false;
            }
        }
    }
    let sub_eq = graph.substitute(eq, target, root);
    let simp_eq = Simplifier::simplify(graph, sub_eq).unwrap_or(sub_eq);
    if is_symbolic_zero(graph, simp_eq) {
        return true;
    }
    if let Ok(v) = graph.evalf(simp_eq, &ctx) {
        let (re, im) = match v {
            crate::numeric::BigValue::Real(r) => (r, 0.0),
            crate::numeric::BigValue::Complex(r, i) => (r, i),
        };
        return re.is_finite() && im.is_finite() && re.abs() < 1e-6 && im.abs() < 1e-6;
    }
    true
}

fn solve_radical_equation(graph: &ExprGraph, eq: ExprId, target: SymbolId) -> Option<Vec<ExprId>> {
    let mut terms = Vec::new();
    flatten_signed_add_terms(graph, eq, 1, &mut terms);
    if terms.len() < 2 {
        return None;
    }
    let sqrt_idx = terms
        .iter()
        .position(|&(_, t)| match_sqrt_term(graph, t, target).is_some())?;
    let (s_sign, s_term) = terms[sqrt_idx];
    let (coeff, radicand) = match_sqrt_term(graph, s_term, target)?;

    // Isolate s_sign * coeff * sqrt(rad) = -sum(other)
    let mut rhs_terms = Vec::new();
    for (idx, &(sgn, t)) in terms.iter().enumerate() {
        if idx == sqrt_idx {
            continue;
        }
        let eff_sign = -s_sign * sgn;
        rhs_terms.push(if eff_sign < 0 { graph.neg(t) } else { t });
    }
    let rhs = if rhs_terms.len() == 1 {
        rhs_terms[0]
    } else {
        graph.add(rhs_terms)
    };
    let lhs_sq = if coeff == graph.integer(1) {
        radicand
    } else {
        graph.mul([graph.pow(coeff, graph.integer(2)), radicand])
    };
    let squared_eq = graph.sub(lhs_sq, graph.pow(rhs, graph.integer(2)));
    let simplified_sq = crate::simplify::AlgebraicTransformations::expand(graph, squared_eq);
    let cand_roots = graph.solveset(simplified_sq, target).ok()?;
    let verified: Vec<ExprId> = cand_roots
        .into_iter()
        .filter(|&r| verify_candidate_root_numeric(graph, eq, target, r))
        .collect();
    Some(verified)
}

fn match_log_term(graph: &ExprGraph, term: ExprId) -> Option<(i64, ExprId)> {
    match &graph.get(term).kind {
        ExprKind::Function { name, args }
            if matches!(graph.symbols.resolve(*name).as_deref(), Some("ln" | "log"))
                && args.len() == 1 =>
        {
            Some((1, args[0]))
        }
        ExprKind::Mul(fs) if fs.len() == 2 => {
            if let ExprKind::Number(Number::Integer(k)) = graph.get(fs[0]).kind
                && let Some((c, arg)) = match_log_term(graph, fs[1])
            {
                return Some((k * c, arg));
            }
            if let ExprKind::Number(Number::Integer(k)) = graph.get(fs[1]).kind
                && let Some((c, arg)) = match_log_term(graph, fs[0])
            {
                return Some((k * c, arg));
            }
            None
        }
        _ => None,
    }
}

fn solve_log_combination(graph: &ExprGraph, eq: ExprId, target: SymbolId) -> Option<Vec<ExprId>> {
    let mut terms = Vec::new();
    flatten_signed_add_terms(graph, eq, 1, &mut terms);
    let mut log_count = 0usize;
    let mut target_log_count = 0usize;
    let mut num_factors = Vec::new();
    let mut den_factors = Vec::new();
    let mut non_log_terms = Vec::new();

    for &(sgn, t) in &terms {
        if let Some((coeff, arg)) = match_log_term(graph, t) {
            log_count += 1;
            if graph.has_symbol(arg, target) {
                target_log_count += 1;
            }
            let power = sgn * coeff;
            let mag = power.unsigned_abs();
            if mag > 8 {
                return None;
            }
            let factor = if mag == 1 {
                arg
            } else {
                graph.pow(arg, graph.integer(mag as i64))
            };
            if power > 0 {
                num_factors.push(factor);
            } else {
                den_factors.push(factor);
            }
        } else if graph.has_symbol(t, target) {
            return None;
        } else {
            non_log_terms.push(if sgn < 0 { graph.neg(t) } else { t });
        }
    }

    if log_count < 2 || target_log_count == 0 {
        return None;
    }

    let num_prod = if num_factors.is_empty() {
        graph.integer(1)
    } else if num_factors.len() == 1 {
        num_factors[0]
    } else {
        graph.mul(num_factors)
    };
    let den_prod = if den_factors.is_empty() {
        graph.integer(1)
    } else if den_factors.len() == 1 {
        den_factors[0]
    } else {
        graph.mul(den_factors)
    };

    let rhs_expr = if non_log_terms.is_empty() {
        graph.integer(1)
    } else {
        let rest = if non_log_terms.len() == 1 {
            non_log_terms[0]
        } else {
            graph.add(non_log_terms)
        };
        graph.function("exp", [graph.neg(rest)])
    };

    // num_prod / den_prod = exp(-rest)  =>  num_prod - rhs_expr * den_prod = 0
    let combined_eq = graph.sub(num_prod, graph.mul([rhs_expr, den_prod]));
    let simp_eq = crate::simplify::AlgebraicTransformations::expand(graph, combined_eq);
    let cands = graph.solveset(simp_eq, target).ok()?;
    let filtered: Vec<ExprId> = cands
        .into_iter()
        .filter(|&r| verify_candidate_root_numeric(graph, eq, target, r))
        .collect();
    Some(filtered)
}

fn extract_exact_symbolic_sqrt(graph: &ExprGraph, id: ExprId) -> Option<ExprId> {
    fn sqrt_monomial(graph: &ExprGraph, node_id: ExprId) -> Option<ExprId> {
        match &graph.get(node_id).kind {
            ExprKind::Number(Number::Integer(n)) if *n >= 0 => {
                let s = (*n as f64).sqrt().round() as i64;
                if s * s == *n {
                    Some(graph.integer(s))
                } else {
                    None
                }
            }
            ExprKind::Number(n) => {
                let q = crate::poly::rational_from_number(n).ok()?;
                use num_traits::Signed as _;
                if q.is_negative() {
                    return None;
                }
                let sn = q.numer().sqrt();
                let sd = q.denom().sqrt();
                if &(&sn * &sn) == q.numer() && &(&sd * &sd) == q.denom() {
                    Some(crate::poly::expr_from_rational(
                        graph,
                        &num_rational::BigRational::new(sn, sd),
                    ))
                } else {
                    None
                }
            }
            ExprKind::Div(a, b) => Some(graph.div(sqrt_monomial(graph, *a)?, sqrt_monomial(graph, *b)?)),
            ExprKind::Pow(base, exp) => {
                if let ExprKind::Number(Number::Integer(k)) = graph.get(*exp).kind
                    && k > 0
                    && k % 2 == 0
                {
                    let half = k / 2;
                    return Some(if half == 1 {
                        *base
                    } else {
                        graph.pow(*base, graph.integer(half))
                    });
                }
                None
            }
            ExprKind::Mul(fs) => {
                let mut parts = Vec::with_capacity(fs.len());
                for &f in fs {
                    parts.push(sqrt_monomial(graph, f)?);
                }
                Some(graph.mul(parts))
            }
            _ => None,
        }
    }

    if let Some(direct) = sqrt_monomial(graph, id) {
        return Some(direct);
    }
    let factored = crate::simplify::AlgebraicTransformations::factor(graph, id);
    sqrt_monomial(graph, factored)
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
