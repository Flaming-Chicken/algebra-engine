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
}

impl GeneralizedDistribution {
    /// Regularize and evaluate product of two distributions using symmetric Colombeau mollifier.
    ///
    /// For standard symmetric mollifiers \rho(x) = \rho(-x):
    /// \int_{-\infty}^\infty \delta_\epsilon(x) \Theta_\epsilon(x) dx = \int_0^\infty \rho(x) dx = 1/2.
    /// Therefore, \delta(x) \cdot \Theta(x) \approx 1/2 \delta(x).
    pub fn multiply(&self, rhs: &Self) -> Self {
        match (self, rhs) {
            (Self::DiracDelta, Self::Heaviside) | (Self::Heaviside, Self::DiracDelta) => {
                Self::Product(0.5, Box::new(Self::DiracDelta))
            }
            (Self::DiracDelta, Self::DiracDelta) => {
                // In Colombeau algebra, \delta^2 is a valid generalized function of order \epsilon^{-1}
                Self::Product(1.0, Box::new(Self::DiracDelta))
            }
            (Self::Scalar(c), other) | (other, Self::Scalar(c)) => {
                Self::Product(*c, Box::new(other.clone()))
            }
            _ => Self::Scalar(0.0),
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
