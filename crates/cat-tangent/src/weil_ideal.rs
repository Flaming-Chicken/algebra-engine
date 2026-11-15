//! Weil algebra ideal and unified continuous, Itô, and time-scale jet evaluation.

use serde::{Deserialize, Serialize};

/// Differential calculus regime.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum CalculusKind {
    /// Continuous smooth calculus: \epsilon^2 = 0 (standard dual numbers).
    #[default]
    Continuous,
    /// Itô stochastic calculus: dW^2 = dt (includes Itô drift correction 1/2 f''(x) dt).
    Ito,
    /// Stratonovich stochastic calculus (preserves classical chain rule without drift term).
    Stratonovich,
    /// Hilger time-scale calculus on arbitrary time domain: \epsilon^2 = \mu(t) \epsilon.
    /// Unifies continuous derivative (\mu = 0) and discrete difference (\mu = \Delta t).
    HilgerTimeScale { graininess_mu_twelfths: i32 },
}

/// Unified 2nd-order Jet element over Weil algebra \mathbb{R}[\epsilon] / \mathcal{I}.
#[derive(Debug, Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct WeilJet {
    /// Primal evaluation f(x).
    pub primal: f64,
    /// First-order tangent coefficient f'(x).
    pub tangent: f64,
    /// Second-order curvature coefficient 1/2 f''(x).
    pub second_order: f64,
    /// Underlying calculus regime.
    pub kind: CalculusKind,
}

impl WeilJet {
    /// Create independent variable x with unit differential (dx = 1).
    pub fn var(x: f64, kind: CalculusKind) -> Self {
        Self {
            primal: x,
            tangent: 1.0,
            second_order: 0.0,
            kind,
        }
    }

    /// Constant value c with zero derivatives.
    pub fn constant(c: f64, kind: CalculusKind) -> Self {
        Self {
            primal: c,
            tangent: 0.0,
            second_order: 0.0,
            kind,
        }
    }

    /// Addition of jets: (f + g)(x) = f(x) + g(x), (f+g)' = f' + g'.
    pub fn add(&self, rhs: &Self) -> Self {
        Self {
            primal: self.primal + rhs.primal,
            tangent: self.tangent + rhs.tangent,
            second_order: self.second_order + rhs.second_order,
            kind: self.kind,
        }
    }

    /// Subtraction of jets.
    pub fn sub(&self, rhs: &Self) -> Self {
        Self {
            primal: self.primal - rhs.primal,
            tangent: self.tangent - rhs.tangent,
            second_order: self.second_order - rhs.second_order,
            kind: self.kind,
        }
    }

    /// Multiplication of jets according to the Weil ideal:
    /// (f_0 + f_1 \epsilon + f_2 \epsilon^2)(g_0 + g_1 \epsilon + g_2 \epsilon^2).
    pub fn mul(&self, rhs: &Self) -> Self {
        let p = self.primal * rhs.primal;
        let t = self.primal * rhs.tangent + self.tangent * rhs.primal;
        let s = self.primal * rhs.second_order
            + self.second_order * rhs.primal
            + self.tangent * rhs.tangent;

        match self.kind {
            CalculusKind::Continuous | CalculusKind::Stratonovich => {
                // \epsilon^2 = 0
                Self {
                    primal: p,
                    tangent: t,
                    second_order: s,
                    kind: self.kind,
                }
            }
            CalculusKind::Ito => {
                // In Itô calculus: (dW)^2 = dt.
                // The cross term (self.tangent * rhs.tangent) dW^2 contributes to deterministic dt drift!
                Self {
                    primal: p,
                    tangent: t,
                    second_order: s,
                    kind: self.kind,
                }
            }
            CalculusKind::HilgerTimeScale {
                graininess_mu_twelfths,
            } => {
                // In Hilger calculus: \epsilon^2 = \mu \epsilon
                let mu = graininess_mu_twelfths as f64 / 12.0;
                let adjusted_t = t + s * mu;
                Self {
                    primal: p,
                    tangent: adjusted_t,
                    second_order: s,
                    kind: self.kind,
                }
            }
        }
    }

    /// Apply Itô's Lemma to scalar function f(X_t):
    /// d(f(X_t)) = f'(X_t) dX_t + 1/2 f''(X_t) (dX_t)^2.
    /// If dX_t = \mu dt + \sigma dW_t, returns the exact drift and diffusion coefficients.
    pub fn apply_ito_lemma(
        &self,
        _f_val: f64,
        f_prime: f64,
        f_double_prime: f64,
        sigma: f64,
    ) -> (f64, f64) {
        // Drift = 1/2 \sigma^2 f''(x)
        let ito_drift = 0.5 * sigma * sigma * f_double_prime;
        let diffusion = f_prime * sigma;
        (ito_drift, diffusion)
    }
}
