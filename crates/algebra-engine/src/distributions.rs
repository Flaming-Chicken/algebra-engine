//! # `algebra-distributions`
//!
//! Generalized Functions / Distribution Theory: Dirac Delta $\delta(x)$, Heaviside Step $H(x)$,
//! sifting property evaluation, Sobolev weak derivatives, and Generalized Probability Distributions with online updates.

use algebra_core::{AlgebraResult, ExprGraph, ExprId, SymbolId};

/// Generalized Probability Distribution trait supporting PDF, CDF, moments, and Bayesian updating.
pub trait GeneralizedDistribution {
    /// Probability Density Function (PDF) / Probability Mass Function (PMF) $f(x)$.
    fn pdf(&self, graph: &ExprGraph, x: ExprId) -> AlgebraResult<ExprId>;

    /// Cumulative Distribution Function (CDF) $F(x) = P(X \le x)$.
    fn cdf(&self, graph: &ExprGraph, x: ExprId) -> AlgebraResult<ExprId>;

    /// Expected value / Mean $E\[X\]$.
    fn mean(&self) -> f64;

    /// Variance $\text{Var}(X)$.
    fn variance(&self) -> f64;
}

/// Mutable Empirical Distribution for online stateful moment updating (running mean & variance via Welford's algorithm).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EmpiricalDistributionMut {
    pub count: usize,
    pub mean_val: f64,
    pub m2: f64,
}

impl EmpiricalDistributionMut {
    pub fn new() -> Self {
        Self::default()
    }

    /// Online stateful update with new observation $x$ (Welford's algorithm).
    pub fn update(&mut self, x: f64) {
        self.count += 1;
        let delta = x - self.mean_val;
        self.mean_val += delta / (self.count as f64);
        let delta2 = x - self.mean_val;
        self.m2 += delta * delta2;
    }

    /// Return current sample variance $s^2$.
    pub fn sample_variance(&self) -> f64 {
        if self.count < 2 {
            0.0
        } else {
            self.m2 / ((self.count - 1) as f64)
        }
    }
}

/// The empirical distribution keeps only a running count, mean and variance (Welford), not the
/// observations, so it has no density or distribution function to return.
fn no_density() -> algebra_core::AlgebraError {
    algebra_core::AlgebraError::EvaluationError(
        "EmpiricalDistributionMut keeps only running moments; it has no PDF or CDF".into(),
    )
}

impl GeneralizedDistribution for EmpiricalDistributionMut {
    fn pdf(&self, _graph: &ExprGraph, _x: ExprId) -> AlgebraResult<ExprId> {
        Err(no_density())
    }

    fn cdf(&self, _graph: &ExprGraph, _x: ExprId) -> AlgebraResult<ExprId> {
        Err(no_density())
    }

    fn mean(&self) -> f64 {
        self.mean_val
    }

    fn variance(&self) -> f64 {
        self.sample_variance()
    }
}

/// Generalized functions & distributions extension trait.
pub trait SymbolicDistributions {
    /// Heaviside Step function $H(x)$ constructor.
    fn heaviside(&self, arg: ExprId) -> ExprId;

    /// Dirac Delta distribution $\delta(x)$ constructor.
    fn dirac_delta(&self, arg: ExprId) -> ExprId;

    /// Apply the sifting property $\int_{-\infty}^\infty f(x) \delta(x - a) dx = f(a)$.
    fn sifting_integral(
        &self,
        f_expr: ExprId,
        wrt: SymbolId,
        point: ExprId,
    ) -> AlgebraResult<ExprId>;
}

impl SymbolicDistributions for ExprGraph {
    fn heaviside(&self, arg: ExprId) -> ExprId {
        self.function("heaviside", [arg])
    }

    fn dirac_delta(&self, arg: ExprId) -> ExprId {
        self.function("dirac_delta", [arg])
    }

    fn sifting_integral(
        &self,
        f_expr: ExprId,
        wrt: SymbolId,
        point: ExprId,
    ) -> AlgebraResult<ExprId> {
        Ok(self.substitute(f_expr, wrt, point))
    }
}

/// Schwartz Distribution / Generalized Function on test space $\mathcal{D}(\mathbb{R}) = C_c^\infty(\mathbb{R})$.
#[derive(Debug, Clone, PartialEq)]
pub enum SchwartzDistribution {
    /// Dirac delta distribution $\delta^{(n)}(x - a)$ of derivative order $n \ge 0$.
    DiracDelta { shift: f64, order: usize },
    /// Heaviside step function $\theta(x - a)$.
    Heaviside { shift: f64 },
    /// Cauchy Principal Value $\operatorname{p.v.}\left(\frac{1}{x - a}\right)$.
    PrincipalValue { shift: f64 },
    /// Signum function $\operatorname{sgn}(x - a) = 2\theta(x - a) - 1$.
    Signum { shift: f64 },
    /// Polynomial-weighted delta distribution $P(x) \delta^{(n)}(x - a)$.
    PolynomialDelta {
        shift: f64,
        power: usize,
        order: usize,
    },
}

impl SchwartzDistribution {
    /// Create standard Dirac delta $\delta(x)$.
    pub fn delta() -> Self {
        Self::DiracDelta {
            shift: 0.0,
            order: 0,
        }
    }

    /// Create standard Heaviside step $\theta(x)$.
    pub fn step() -> Self {
        Self::Heaviside { shift: 0.0 }
    }

    /// Create standard Cauchy Principal Value $\operatorname{p.v.}(1/x)$.
    pub fn pv() -> Self {
        Self::PrincipalValue { shift: 0.0 }
    }

    /// Distributional derivative $T' = \frac{\mathrm{d}}{\mathrm{d}x} T$ defined by $\langle T', \phi \rangle = -\langle T, \phi' \rangle$.
    pub fn derivative(&self) -> Self {
        match self {
            Self::Heaviside { shift } => Self::DiracDelta {
                shift: *shift,
                order: 0,
            },
            Self::DiracDelta { shift, order } => Self::DiracDelta {
                shift: *shift,
                order: order + 1,
            },
            Self::Signum { shift } => Self::DiracDelta {
                shift: *shift,
                order: 0,
            },
            Self::PrincipalValue { shift } => Self::DiracDelta {
                shift: *shift,
                order: 1,
            },
            Self::PolynomialDelta {
                shift,
                power,
                order,
            } => {
                if *power > 0 {
                    Self::PolynomialDelta {
                        shift: *shift,
                        power: power - 1,
                        order: *order,
                    }
                } else {
                    Self::DiracDelta {
                        shift: *shift,
                        order: order + 1,
                    }
                }
            }
        }
    }

    /// Action of distribution on test function $\phi$: pairing $\langle T, \phi \rangle$.
    /// `phi` is a closure mapping $x$ and derivative order $k$ to $\phi^{(k)}(x)$.
    pub fn pair<F>(&self, phi: &F) -> f64
    where
        F: Fn(f64, usize) -> f64,
    {
        match self {
            Self::DiracDelta { shift, order } => {
                // <delta^(n)(x - a), phi> = (-1)^n phi^(n)(a)
                let sign = if order % 2 == 0 { 1.0 } else { -1.0 };
                sign * phi(*shift, *order)
            }
            Self::Heaviside { shift } => {
                // Approximate integral from shift to R with midpoint rule
                let mut sum = 0.0;
                let step = 0.01;
                let r = 100.0;
                let mut x = *shift + step / 2.0;
                while x < r {
                    sum += phi(x, 0) * step;
                    x += step;
                }
                sum
            }
            Self::Signum { shift } => {
                // <sgn(x - a), phi> = \int_a^\infty phi(x)dx - \int_{-\infty}^a phi(x)dx
                let step = 0.01;
                let r = 100.0;
                let mut pos = 0.0;
                let mut neg = 0.0;

                let mut x = *shift + step / 2.0;
                while x < r {
                    pos += phi(x, 0) * step;
                    x += step;
                }
                x = *shift - step / 2.0;
                while x > -r {
                    neg += phi(x, 0) * step;
                    x -= step;
                }
                pos - neg
            }
            Self::PrincipalValue { shift } => {
                // Cauchy Principal Value: \lim_{\epsilon \to 0} \int_{|x-a| > \epsilon} \frac{\phi(x)}{x-a} dx
                let eps = 1e-4;
                let r = 50.0;
                let step = 0.005;
                let mut sum = 0.0;

                let mut x = *shift + eps;
                while x < r {
                    let term = (phi(x, 0) - phi(2.0 * shift - x, 0)) / (x - shift);
                    sum += term * step;
                    x += step;
                }
                sum
            }
            Self::PolynomialDelta {
                shift,
                power,
                order,
            } => {
                let sign = if order % 2 == 0 { 1.0 } else { -1.0 };
                let poly_val = shift.powi(*power as i32);
                sign * poly_val * phi(*shift, *order)
            }
        }
    }
}
