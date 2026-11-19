//! Colombeau algebra of generalized functions and distribution regularization.
//!
//! Enables rigorous multiplication of singular distributions, evaluating
//! products such as \delta(x) \cdot \Theta(x) \approx 1/2 \delta(x)
//! via net of smooth mollifiers \rho_\epsilon(x).

/// Colombeau generalized distribution representation.
#[derive(Debug, Clone, PartialEq)]
pub enum GeneralizedDistribution {
    /// Dirac delta distribution \delta(x).
    DiracDelta,
    /// Heaviside step distribution \Theta(x).
    Heaviside,
    /// Regularized product of distributions with scalar factor.
    Product(f64, Box<GeneralizedDistribution>),
    /// Constant scalar.
    Scalar(f64),
    /// `coefficient * epsilon^power * base`: a generalized function whose size depends on the regularization
    /// scale `epsilon` (a moderate net in the Colombeau algebra). `delta^2` is `epsilon^-1 / (2 sqrt(pi)) delta`
    /// for the Gaussian mollifier: its integral against a test function grows like `1 / epsilon`, so it is not a
    /// multiple of `delta` with a fixed coefficient.
    Scaled {
        coefficient: f64,
        epsilon_power: i32,
        base: Box<GeneralizedDistribution>,
    },
}

impl GeneralizedDistribution {
    /// Regularize and evaluate product of two distributions using the Gaussian mollifier
    /// $\rho(x) = e^{-x^2/2}/\sqrt{2\pi}$ (so $\delta_\epsilon$ and $\Theta_\epsilon$ are those of
    /// [`GeneralizedDistribution::eval_mollified`]). The products are *associated* distributions, that is, equal
    /// after integration against test functions as $\epsilon \to 0$:
    ///
    /// - $\delta \cdot \Theta \approx \tfrac12 \delta$ (symmetry of the mollifier);
    /// - $\delta^2 = \frac{1}{2\sqrt\pi}\,\epsilon^{-1}\,\delta$, since $\int \delta_\epsilon^2 = \frac{1}{2\sqrt\pi\,\epsilon}$:
    ///   a product with no limit, kept as a scaled generalized function;
    /// - $\Theta^2 \approx \Theta$ (the difference $\Theta_\epsilon^2 - \Theta_\epsilon$ tends to $0$ weakly).
    ///
    /// Scalars and scaled factors multiply through. (This used to give $\delta^2 = 1 \cdot \delta$ and
    /// $\Theta^2 = 0$, and dropped every product with a `Product` factor.)
    pub fn multiply(&self, rhs: &Self) -> Self {
        match (self, rhs) {
            (Self::Scalar(a), Self::Scalar(b)) => Self::Scalar(a * b),
            (Self::Scalar(c), other) | (other, Self::Scalar(c)) => other.scaled_by(*c, 0),
            (Self::Product(c, inner), other) | (other, Self::Product(c, inner)) => {
                inner.multiply(other).scaled_by(*c, 0)
            }
            (
                Self::Scaled {
                    coefficient,
                    epsilon_power,
                    base,
                },
                other,
            )
            | (
                other,
                Self::Scaled {
                    coefficient,
                    epsilon_power,
                    base,
                },
            ) => base.multiply(other).scaled_by(*coefficient, *epsilon_power),
            (Self::DiracDelta, Self::Heaviside) | (Self::Heaviside, Self::DiracDelta) => {
                Self::Product(0.5, Box::new(Self::DiracDelta))
            }
            (Self::DiracDelta, Self::DiracDelta) => Self::Scaled {
                coefficient: 1.0 / (2.0 * std::f64::consts::PI.sqrt()),
                epsilon_power: -1,
                base: Box::new(Self::DiracDelta),
            },
            (Self::Heaviside, Self::Heaviside) => Self::Heaviside,
        }
    }

    /// `factor * epsilon^power * self`, collapsing nested scalings.
    fn scaled_by(&self, factor: f64, power: i32) -> Self {
        match self {
            Self::Scalar(c) if power == 0 => Self::Scalar(factor * c),
            Self::Product(c, inner) => inner.scaled_by(factor * c, power),
            Self::Scaled {
                coefficient,
                epsilon_power,
                base,
            } => base.scaled_by(factor * coefficient, power + epsilon_power),
            other if power == 0 => Self::Product(factor, Box::new(other.clone())),
            other => Self::Scaled {
                coefficient: factor,
                epsilon_power: power,
                base: Box::new(other.clone()),
            },
        }
    }

    /// Numerical evaluation with Gaussian mollifier of width epsilon.
    pub fn eval_mollified(&self, x: f64, epsilon: f64) -> f64 {
        match self {
            Self::DiracDelta => {
                // \delta_\epsilon(x) = 1 / (\sqrt{2\pi} \epsilon) \exp(-x^2 / 2\epsilon^2)
                let inv = 1.0 / ((2.0 * std::f64::consts::PI).sqrt() * epsilon);
                inv * (-0.5 * x * x / (epsilon * epsilon)).exp()
            }
            Self::Heaviside => {
                // \Theta_\epsilon(x) = 1/2 (1 + \text{erf}(x / (\sqrt{2} \epsilon)))
                0.5 * (1.0 + erf_approx(x / (std::f64::consts::SQRT_2 * epsilon)))
            }
            Self::Product(factor, dist) => *factor * dist.eval_mollified(x, epsilon),
            Self::Scalar(c) => *c,
            Self::Scaled {
                coefficient,
                epsilon_power,
                base,
            } => coefficient * epsilon.powi(*epsilon_power) * base.eval_mollified(x, epsilon),
        }
    }
}

/// Numerical error function approximation.
fn erf_approx(x: f64) -> f64 {
    // Abramowitz and Stegun formula 7.1.26
    let sign = if x >= 0.0 { 1.0 } else { -1.0 };
    let x_abs = x.abs();
    let p = 0.3275911;
    let t = 1.0 / (1.0 + p * x_abs);
    let poly = t
        * (0.254829592
            + t * (-0.284496736 + t * (1.421413741 + t * (-1.453152027 + t * 1.061405429))));
    sign * (1.0 - poly * (-x_abs * x_abs).exp())
}
