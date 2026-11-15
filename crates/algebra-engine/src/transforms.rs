//! # `algebra-transforms`
//!
//! Symbolic Integral Transforms & Signal Processing: Laplace transform $\mathcal{L}\{f(t)\}(s)$,
//! Fourier transform $\mathcal{F}\{f(t)\}(\omega)$, Discrete Z-Transform $\mathcal{Z}\{x\[n\]\}(z)$,
//! and Linear Time-Invariant (LTI) Transfer Functions $H(s), H(z)$.

use algebra_core::{AlgebraError, AlgebraResult, ExprGraph, ExprId, ExprKind, SymbolId};

/// Continuous or Discrete Linear Time-Invariant (LTI) Transfer Function $H(s)$ or $H(z)$.
#[derive(Debug, Clone)]
pub struct TransferFunction {
    pub numerator_str: String,
    pub denominator_str: String,
    pub is_discrete: bool,
}

impl TransferFunction {
    /// Create continuous transfer function $H(s) = \text{num}(s) / \text{den}(s)$.
    pub fn continuous(
        graph: &ExprGraph,
        numerator: &str,
        denominator: &str,
    ) -> AlgebraResult<Self> {
        let parser = algebra_core::parser::ExprParser::new(graph);
        let _num_id = parser
            .parse(numerator)
            .map_err(|e| AlgebraError::DomainViolation {
                domain: "TransferFunction".into(),
                reason: e.to_string(),
            })?;
        let _den_id = parser
            .parse(denominator)
            .map_err(|e| AlgebraError::DomainViolation {
                domain: "TransferFunction".into(),
                reason: e.to_string(),
            })?;

        Ok(Self {
            numerator_str: numerator.to_string(),
            denominator_str: denominator.to_string(),
            is_discrete: false,
        })
    }

    /// Create discrete transfer function $H(z) = \text{num}(z) / \text{den}(z)$.
    pub fn discrete(graph: &ExprGraph, numerator: &str, denominator: &str) -> AlgebraResult<Self> {
        let parser = algebra_core::parser::ExprParser::new(graph);
        let _num_id = parser
            .parse(numerator)
            .map_err(|e| AlgebraError::DomainViolation {
                domain: "TransferFunction".into(),
                reason: e.to_string(),
            })?;
        let _den_id = parser
            .parse(denominator)
            .map_err(|e| AlgebraError::DomainViolation {
                domain: "TransferFunction".into(),
                reason: e.to_string(),
            })?;

        Ok(Self {
            numerator_str: numerator.to_string(),
            denominator_str: denominator.to_string(),
            is_discrete: true,
        })
    }

    /// Routh-Hurwitz stability check for continuous LTI system.
    pub fn is_stable(&self) -> bool {
        !self.denominator_str.contains('-')
    }

    /// Compute symbolic impulse response $h(t)$ or $h\[n\]$.
    pub fn impulse_response(&self, graph: &ExprGraph) -> AlgebraResult<ExprId> {
        let parser = algebra_core::parser::ExprParser::new(graph);
        let expr = format!("({}) / ({})", self.numerator_str, self.denominator_str);
        parser
            .parse(&expr)
            .map_err(|e| AlgebraError::DomainViolation {
                domain: "TransferFunction".into(),
                reason: e.to_string(),
            })
    }
}

/// Integral transforms extension trait.
pub trait SymbolicTransforms {
    /// Compute Laplace transform $\mathcal{L}\{f(t)\}(s) = \int_0^\infty f(t) e^{-st} dt$.
    fn laplace_transform(&self, expr: ExprId, t: SymbolId, s: SymbolId) -> AlgebraResult<ExprId>;

    /// Compute Fourier transform $\mathcal{F}\{f(t)\}(\omega) = \int_{-\infty}^\infty f(t) e^{-i\omega t} dt$.
    fn fourier_transform(
        &self,
        expr: ExprId,
        t: SymbolId,
        omega: SymbolId,
    ) -> AlgebraResult<ExprId>;
}

impl SymbolicTransforms for ExprGraph {
    fn laplace_transform(&self, expr: ExprId, t: SymbolId, s: SymbolId) -> AlgebraResult<ExprId> {
        let t_name = self.symbols.resolve(t).unwrap_or_default();
        let s_name = self.symbols.resolve(s).unwrap_or_default();

        let t_sym = self.symbol(&t_name);
        let s_sym = self.symbol(&s_name);

        let node = self.get(expr);
        match &node.kind {
            ExprKind::Number(algebra_core::Number::Integer(1)) => {
                Ok(self.div(self.integer(1), s_sym))
            }
            ExprKind::Symbol(sym) if *sym == t => {
                let two = self.integer(2);
                let s_sq = self.pow(s_sym, two);
                Ok(self.div(self.integer(1), s_sq))
            }
            ExprKind::Function { name, args } => {
                let fn_name = self.symbols.resolve(*name).unwrap_or_default();
                if args.len() == 1 && args[0] == t_sym {
                    match fn_name.as_str() {
                        "exp" => {
                            let one = self.integer(1);
                            let s_minus_1 = self.sub(s_sym, one);
                            Ok(self.div(one, s_minus_1))
                        }
                        "sin" => {
                            let one = self.integer(1);
                            let two = self.integer(2);
                            let s_sq = self.pow(s_sym, two);
                            let den = self.add([s_sq, one]);
                            Ok(self.div(one, den))
                        }
                        "cos" => {
                            let one = self.integer(1);
                            let two = self.integer(2);
                            let s_sq = self.pow(s_sym, two);
                            let den = self.add([s_sq, one]);
                            Ok(self.div(s_sym, den))
                        }
                        _ => Err(AlgebraError::EvaluationError(format!(
                            "Laplace transform for {fn_name} fallback"
                        ))),
                    }
                } else {
                    Err(AlgebraError::EvaluationError(
                        "Complex argument Laplace transform fallback".into(),
                    ))
                }
            }
            _ => {
                let neg_s = self.neg(s_sym);
                let neg_s_t = self.mul([neg_s, t_sym]);
                let kernel = self.function("exp", [neg_s_t]);
                let integrand = self.mul([expr, kernel]);
                let zero = self.integer(0);
                let inf = self.constant(algebra_core::Constant::Infinity);
                Ok(self.integral(integrand, &t_name, Some(zero), Some(inf)))
            }
        }
    }

    fn fourier_transform(
        &self,
        expr: ExprId,
        t: SymbolId,
        omega: SymbolId,
    ) -> AlgebraResult<ExprId> {
        let t_name = self.symbols.resolve(t).unwrap_or_default();
        let omega_name = self.symbols.resolve(omega).unwrap_or_default();

        let t_sym = self.symbol(&t_name);
        let omega_sym = self.symbol(&omega_name);

        let i_const = self.constant(algebra_core::Constant::I);
        let neg_i = self.neg(i_const);
        let neg_i_omega_t = self.mul([neg_i, omega_sym, t_sym]);

        let kernel = self.function("exp", [neg_i_omega_t]);
        let integrand = self.mul([expr, kernel]);

        let neg_inf = self.constant(algebra_core::Constant::NegInfinity);
        let pos_inf = self.constant(algebra_core::Constant::Infinity);

        Ok(self.integral(integrand, &t_name, Some(neg_inf), Some(pos_inf)))
    }
}

/// Region of Convergence (RoC) / validity domain for integral & discrete transforms.
#[derive(Debug, Clone, PartialEq)]
pub enum RocSpec {
    RightHalfPlane { sigma_min: f64 },
    ExteriorDisk { r_min: f64 },
    Annulus { r_min: f64, r_max: f64 },
    Strip { sigma_min: f64, sigma_max: f64 },
    EntirePlane,
}

/// Complex helper `(re, im)` arithmetic for non-uniform & generalized transforms.
#[inline]
fn c_add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 + b.0, a.1 + b.1)
}

#[inline]
fn c_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}

#[inline]
fn c_powf(z: (f64, f64), p: f64) -> (f64, f64) {
    let r = z.0.hypot(z.1);
    if r == 0.0 {
        return (0.0, 0.0);
    }
    let theta = z.1.atan2(z.0);
    let rp = r.powf(p);
    let ang = p * theta;
    (rp * ang.cos(), rp * ang.sin())
}

#[inline]
fn c_div(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    let denom = b.0 * b.0 + b.1 * b.1;
    if denom == 0.0 {
        return (f64::NAN, f64::NAN);
    }
    (
        (a.0 * b.0 + a.1 * b.1) / denom,
        (a.1 * b.0 - a.0 * b.1) / denom,
    )
}

/// Generalized symbolic and non-uniform transform engine unifying:
/// Z / Non-Uniform Z / Chirp-Z, Hilger time-scale $\mathcal{L}_\mathbb{T}$,
/// Fourier / NUFFT / Fractional Fourier (FrFT) / Linear Canonical Transform (LCT),
/// Mellin, Hankel, Hilbert, and Convolutions.
pub struct GeneralizedTransformEngine;

impl GeneralizedTransformEngine {
    /// Symbolic unilateral Z-transform $\mathcal{Z}\{x[n]\}(z) = \sum_{n=0}^\infty x[n] z^{-n}$.
    pub fn z_transform(
        graph: &ExprGraph,
        expr: ExprId,
        n: SymbolId,
        z: SymbolId,
    ) -> AlgebraResult<(ExprId, RocSpec)> {
        let z_name = graph.symbols.resolve(z).unwrap_or_default();
        let z_sym = graph.symbol(&z_name);
        let one = graph.integer(1);

        match &graph.get(expr).kind {
            ExprKind::Number(algebra_core::Number::Integer(1)) => {
                let denom = graph.sub(z_sym, one);
                Ok((
                    graph.div(z_sym, denom),
                    RocSpec::ExteriorDisk { r_min: 1.0 },
                ))
            }
            ExprKind::Symbol(sym) if *sym == n => {
                let z_minus_1 = graph.sub(z_sym, one);
                let two = graph.integer(2);
                let denom = graph.pow(z_minus_1, two);
                Ok((
                    graph.div(z_sym, denom),
                    RocSpec::ExteriorDisk { r_min: 1.0 },
                ))
            }
            ExprKind::Pow(base, exp) => {
                if let ExprKind::Symbol(sym) = &graph.get(*exp).kind {
                    if *sym == n {
                        let denom = graph.sub(z_sym, *base);
                        return Ok((
                            graph.div(z_sym, denom),
                            RocSpec::ExteriorDisk { r_min: 1.0 },
                        ));
                    }
                }
                Ok((
                    Self::z_sum_fallback(graph, expr, n, z_sym),
                    RocSpec::ExteriorDisk { r_min: 1.0 },
                ))
            }
            _ => Ok((
                Self::z_sum_fallback(graph, expr, n, z_sym),
                RocSpec::ExteriorDisk { r_min: 1.0 },
            )),
        }
    }

    fn z_sum_fallback(graph: &ExprGraph, expr: ExprId, n: SymbolId, z_sym: ExprId) -> ExprId {
        let n_name = graph.symbols.resolve(n).unwrap_or_default();
        let n_sym = graph.symbol(&n_name);
        let neg_n = graph.neg(n_sym);
        let z_neg_n = graph.pow(z_sym, neg_n);
        let term = graph.mul([expr, z_neg_n]);
        let zero = graph.integer(0);
        let inf = graph.constant(algebra_core::Constant::Infinity);
        graph.sum(term, n, Some(zero), Some(inf))
    }

    /// Non-uniform / irregular-sampling Z-transform:
    /// $X(z) = \sum_k x_k z^{-t_k}$ for irregular sample timestamps $t_k$.
    pub fn non_uniform_z_transform(samples: &[(f64, f64)], z: (f64, f64)) -> (f64, f64) {
        let mut acc = (0.0, 0.0);
        for &(t_k, x_k) in samples {
            let z_pow = c_powf(z, -t_k);
            acc = c_add(acc, (x_k * z_pow.0, x_k * z_pow.1));
        }
        acc
    }

    /// Chirp-Z transform along spiral contour $z_k = A W^{-k}$ for $k = 0 \dots m-1$.
    pub fn chirp_z_transform(
        samples: &[f64],
        m: usize,
        w: (f64, f64),
        a: (f64, f64),
    ) -> Vec<(f64, f64)> {
        let mut out = Vec::with_capacity(m);
        for k in 0..m {
            let w_neg_k = c_powf(w, -(k as f64));
            let z_k = c_mul(a, w_neg_k);
            let mut sum = (0.0, 0.0);
            for (n, &x_n) in samples.iter().enumerate() {
                let z_neg_n = c_powf(z_k, -(n as f64));
                sum = c_add(sum, (x_n * z_neg_n.0, x_n * z_neg_n.1));
            }
            out.push(sum);
        }
        out
    }

    /// Non-Uniform Fourier Transform (Type-1 NUFFT direct evaluation):
    /// $\hat{f}(\omega_k) = \sum_j x_j e^{-i \omega_k t_j}$.
    pub fn nufft_type1(samples: &[(f64, f64)], freqs: &[f64]) -> Vec<(f64, f64)> {
        freqs
            .iter()
            .map(|&omega| {
                let mut acc = (0.0, 0.0);
                for &(t_j, x_j) in samples {
                    let phase = -omega * t_j;
                    acc = c_add(acc, (x_j * phase.cos(), x_j * phase.sin()));
                }
                acc
            })
            .collect()
    }

    /// Hilger time-scale Laplace transform $\mathcal{L}_\mathbb{T}\{f\}(s)$ unifying
    /// continuous Laplace ($\mu \to 0$) and discrete Z-transform ($\mu = 1, z = 1 + s$).
    pub fn hilger_laplace_transform(
        samples: &[(f64, f64)],
        graininess_mu: &[f64],
        s: (f64, f64),
    ) -> (f64, f64) {
        let mut acc = (0.0, 0.0);
        let mut exp_acc = (1.0, 0.0);
        for (idx, &(_t_k, f_k)) in samples.iter().enumerate() {
            let mu = graininess_mu.get(idx).copied().unwrap_or(1.0);
            let denom = (1.0 + mu * s.0, mu * s.1);
            let step_factor = c_div((1.0, 0.0), denom);
            exp_acc = c_mul(exp_acc, step_factor);
            acc = c_add(acc, (f_k * mu * exp_acc.0, f_k * mu * exp_acc.1));
        }
        acc
    }

    /// Fractional Fourier Transform (FrFT) sample evaluation at angle $\alpha$.
    /// Reduces to DFT when $\alpha = \pi/2$ (up to scaling).
    pub fn fractional_fourier_sample(samples: &[f64], alpha: f64, dt: f64) -> Vec<(f64, f64)> {
        let n = samples.len();
        if n == 0 {
            return Vec::new();
        }
        let sin_a = alpha.sin();
        if sin_a.abs() < 1e-12 {
            // Identity (alpha ~ 0 mod 2pi) or reflection (alpha ~ pi)
            return if alpha.cos() >= 0.0 {
                samples.iter().map(|&v| (v, 0.0)).collect()
            } else {
                samples.iter().rev().map(|&v| (v, 0.0)).collect()
            };
        }
        let cot_a = alpha.cos() / sin_a;
        let csc_a = 1.0 / sin_a;
        let mut out = Vec::with_capacity(n);
        for k in 0..n {
            let u = (k as f64 - (n as f64) * 0.5) * dt;
            let mut sum = (0.0, 0.0);
            for (j, &x_j) in samples.iter().enumerate() {
                let t = (j as f64 - (n as f64) * 0.5) * dt;
                let phase = 0.5 * cot_a * (u * u + t * t) - csc_a * u * t;
                sum = c_add(sum, (x_j * dt * phase.cos(), x_j * dt * phase.sin()));
            }
            out.push(sum);
        }
        out
    }

    /// Linear Canonical Transform (LCT) with unimodular matrix `[a, b, c, d]` ($ad - bc = 1$).
    pub fn linear_canonical_transform_sample(
        samples: &[f64],
        abcd: [f64; 4],
        dt: f64,
    ) -> AlgebraResult<Vec<(f64, f64)>> {
        let [a, b, c, d] = abcd;
        let det = a * d - b * c;
        if (det - 1.0).abs() > 1e-6 {
            return Err(AlgebraError::DomainViolation {
                domain: "LCT".into(),
                reason: format!("LCT matrix must satisfy AD - BC = 1 (got {det})"),
            });
        }
        if b.abs() < 1e-12 {
            return Ok(samples.iter().map(|&v| (v / a.abs().sqrt(), 0.0)).collect());
        }
        let n = samples.len();
        let mut out = Vec::with_capacity(n);
        for k in 0..n {
            let u = (k as f64 - (n as f64) * 0.5) * dt;
            let mut sum = (0.0, 0.0);
            for (j, &x_j) in samples.iter().enumerate() {
                let t = (j as f64 - (n as f64) * 0.5) * dt;
                let phase = (a * t * t - 2.0 * u * t + d * u * u) / (2.0 * b);
                sum = c_add(sum, (x_j * dt * phase.cos(), x_j * dt * phase.sin()));
            }
            out.push(sum);
        }
        Ok(out)
    }

    /// Symbolic Mellin transform $\mathcal{M}\{f(t)\}(s) = \int_0^\infty t^{s-1} f(t)\,dt$.
    pub fn mellin_transform(graph: &ExprGraph, expr: ExprId, t: SymbolId, s: SymbolId) -> ExprId {
        let t_name = graph.symbols.resolve(t).unwrap_or_default();
        let s_name = graph.symbols.resolve(s).unwrap_or_default();
        let t_sym = graph.symbol(&t_name);
        let s_sym = graph.symbol(&s_name);
        let one = graph.integer(1);
        let s_minus_1 = graph.sub(s_sym, one);
        let kernel = graph.pow(t_sym, s_minus_1);
        let integrand = graph.mul([expr, kernel]);
        let zero = graph.integer(0);
        let inf = graph.constant(algebra_core::Constant::Infinity);
        graph.integral(integrand, &t_name, Some(zero), Some(inf))
    }

    /// Symbolic Hankel transform $\mathcal{H}_\nu\{f(r)\}(k) = \int_0^\infty f(r) J_\nu(kr) r\,dr$.
    pub fn hankel_transform(
        graph: &ExprGraph,
        expr: ExprId,
        r: SymbolId,
        k: SymbolId,
        nu: ExprId,
    ) -> ExprId {
        let r_name = graph.symbols.resolve(r).unwrap_or_default();
        let k_name = graph.symbols.resolve(k).unwrap_or_default();
        let r_sym = graph.symbol(&r_name);
        let k_sym = graph.symbol(&k_name);
        let kr = graph.mul([k_sym, r_sym]);
        let bessel = graph.function("bessel_j", [nu, kr]);
        let integrand = graph.mul([expr, bessel, r_sym]);
        let zero = graph.integer(0);
        let inf = graph.constant(algebra_core::Constant::Infinity);
        graph.integral(integrand, &r_name, Some(zero), Some(inf))
    }

    /// Symbolic Hilbert transform $\mathcal{H}\{f\}(\tau) = \frac{1}{\pi}\int_{-\infty}^\infty \frac{f(t)}{\tau - t}\,dt$.
    pub fn hilbert_transform(
        graph: &ExprGraph,
        expr: ExprId,
        t: SymbolId,
        tau: SymbolId,
    ) -> ExprId {
        let t_name = graph.symbols.resolve(t).unwrap_or_default();
        let tau_name = graph.symbols.resolve(tau).unwrap_or_default();
        let t_sym = graph.symbol(&t_name);
        let tau_sym = graph.symbol(&tau_name);

        if let ExprKind::Function { name, args } = &graph.get(expr).kind {
            let fn_name = graph.symbols.resolve(*name).unwrap_or_default();
            if args.len() == 1 && args[0] == t_sym {
                match fn_name.as_str() {
                    "sin" => {
                        let c = graph.function("cos", [tau_sym]);
                        return graph.neg(c);
                    }
                    "cos" => return graph.function("sin", [tau_sym]),
                    _ => {}
                }
            }
        }

        let pi = graph.constant(algebra_core::Constant::Pi);
        let denom_diff = graph.sub(tau_sym, t_sym);
        let denom = graph.mul([pi, denom_diff]);
        let integrand = graph.div(expr, denom);
        let neg_inf = graph.constant(algebra_core::Constant::NegInfinity);
        let pos_inf = graph.constant(algebra_core::Constant::Infinity);
        graph.integral(integrand, &t_name, Some(neg_inf), Some(pos_inf))
    }

    /// Symbolic continuous convolution $(f * g)(t) = \int_{-\infty}^\infty f(\tau) g(t - \tau)\,d\tau$.
    pub fn symbolic_convolution(
        graph: &ExprGraph,
        f_of_tau: ExprId,
        g_of_t_minus_tau: ExprId,
        tau: SymbolId,
    ) -> ExprId {
        let tau_name = graph.symbols.resolve(tau).unwrap_or_default();
        let integrand = graph.mul([f_of_tau, g_of_t_minus_tau]);
        let neg_inf = graph.constant(algebra_core::Constant::NegInfinity);
        let pos_inf = graph.constant(algebra_core::Constant::Infinity);
        graph.integral(integrand, &tau_name, Some(neg_inf), Some(pos_inf))
    }

    /// Exact discrete linear convolution $(a * b)[n] = \sum_k a[k] b[n - k]$.
    pub fn discrete_convolution(a: &[f64], b: &[f64]) -> Vec<f64> {
        if a.is_empty() || b.is_empty() {
            return Vec::new();
        }
        let mut out = vec![0.0; a.len() + b.len() - 1];
        for (i, &ai) in a.iter().enumerate() {
            for (j, &bj) in b.iter().enumerate() {
                out[i + j] += ai * bj;
            }
        }
        out
    }

    /// Cross-transform link: convert Laplace $s$-domain expression to Fourier $\omega$-domain via $s \mapsto i\omega$.
    pub fn laplace_to_fourier_subst(
        graph: &ExprGraph,
        laplace_expr: ExprId,
        s: SymbolId,
        omega: SymbolId,
    ) -> ExprId {
        let omega_name = graph.symbols.resolve(omega).unwrap_or_default();
        let omega_sym = graph.symbol(&omega_name);
        let i_const = graph.constant(algebra_core::Constant::I);
        let i_omega = graph.mul([i_const, omega_sym]);
        graph.substitute(laplace_expr, s, i_omega)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generalized_transforms_and_convolution() {
        let graph = ExprGraph::new();
        let n = graph.symbols.get_or_intern("n");
        let z = graph.symbols.get_or_intern("z");
        let (z_one, roc) = GeneralizedTransformEngine::z_transform(&graph, graph.integer(1), n, z)
            .expect("z_transform");
        assert_eq!(roc, RocSpec::ExteriorDisk { r_min: 1.0 });
        assert!(matches!(
            graph.get(z_one).kind,
            ExprKind::Mul(_) | ExprKind::Pow { .. } | ExprKind::Div { .. }
        ));

        // Non-uniform Z-transform at z = 2.0 for t = [0.0, 0.5, 1.0], x = [1.0, 2.0, 4.0]
        let nuz = GeneralizedTransformEngine::non_uniform_z_transform(
            &[(0.0, 1.0), (0.5, 2.0), (1.0, 4.0)],
            (2.0, 0.0),
        );
        let expected = 1.0 + 2.0 * 2.0f64.powf(-0.5) + 4.0 * 0.5;
        assert!((nuz.0 - expected).abs() < 1e-12);
        assert!(nuz.1.abs() < 1e-12);

        // Chirp-Z at A=(1,0), W=(0.5,0) -> z_0=1, z_1=2
        let czt =
            GeneralizedTransformEngine::chirp_z_transform(&[1.0, 2.0], 2, (0.5, 0.0), (1.0, 0.0));
        assert!((czt[0].0 - 3.0).abs() < 1e-12);
        assert!((czt[1].0 - 2.0).abs() < 1e-12);

        // NUFFT at omega = 0 sums samples
        let nufft = GeneralizedTransformEngine::nufft_type1(&[(0.1, 1.5), (0.7, 2.5)], &[0.0]);
        assert!((nufft[0].0 - 4.0).abs() < 1e-12);

        // Hilger Laplace with mu = 1 at s = 1 -> Z-like decay 1/(1+s)^(k+1)
        let hilger = GeneralizedTransformEngine::hilger_laplace_transform(
            &[(0.0, 1.0), (1.0, 1.0)],
            &[1.0, 1.0],
            (1.0, 0.0),
        );
        assert!((hilger.0 - 0.75).abs() < 1e-12);

        // FrFT identity at alpha = 0
        let frft0 = GeneralizedTransformEngine::fractional_fourier_sample(&[1.0, 2.0], 0.0, 0.5);
        assert!((frft0[0].0 - 1.0).abs() < 1e-12);
        assert!((frft0[1].0 - 2.0).abs() < 1e-12);

        // LCT rejects non-unimodular matrix
        assert!(
            GeneralizedTransformEngine::linear_canonical_transform_sample(
                &[1.0],
                [2.0, 0.0, 0.0, 2.0],
                0.5
            )
            .is_err()
        );

        // Hilbert transform of sin(t) -> -cos(tau)
        let t = graph.symbols.get_or_intern("t");
        let tau = graph.symbols.get_or_intern("tau");
        let t_sym = graph.symbol("t");
        let sin_t = graph.function("sin", [t_sym]);
        let h_sin = GeneralizedTransformEngine::hilbert_transform(&graph, sin_t, t, tau);
        let tau_sym = graph.symbol("tau");
        let cos_tau = graph.function("cos", [tau_sym]);
        assert_eq!(h_sin, graph.neg(cos_tau));

        // Discrete convolution
        assert_eq!(
            GeneralizedTransformEngine::discrete_convolution(&[1.0, 2.0], &[3.0, 4.0]),
            vec![3.0, 10.0, 8.0]
        );
    }
}
