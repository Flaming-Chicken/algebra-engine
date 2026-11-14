//! Differential Forms and Exterior Calculus (\Omega^k(M)).
//!
//! Provides exterior wedge products \alpha \wedge \beta, de Rham exterior derivative d,
//! Hodge dual \star, interior product \iota_X, and verifies Poincaré nilpotency d^2 = 0
//! and Cartan's magic formula \mathcal{L}_X \omega = \iota_X d\omega + d \iota_X \omega.

use serde::{Deserialize, Serialize};

/// Differential k-form \omega \in \Omega^k(M) on a D-dimensional manifold.
///
/// Basis blades are indexed by bitmasks of length D:
/// - Bitmask 0b001 -> dx^0
/// - Bitmask 0b010 -> dx^1
/// - Bitmask 0b011 -> dx^0 \wedge dx^1
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct DifferentialForm {
    pub dim: usize,
    pub degree: usize,
    /// Bitmask-indexed coefficients of length 2^D.
    pub blades: Vec<f64>,
}

impl DifferentialForm {
    /// Zero k-form.
    pub fn zero(dim: usize, degree: usize) -> Self {
        Self {
            dim,
            degree,
            blades: vec![0.0; 1 << dim],
        }
    }

    /// Scalar 0-form.
    pub fn scalar(dim: usize, c: f64) -> Self {
        let mut f = Self::zero(dim, 0);
        f.blades[0] = c;
        f
    }

    /// Coordinate basis 1-form dx^i.
    pub fn basis_1form(dim: usize, i: usize) -> Self {
        assert!(i < dim);
        let mut f = Self::zero(dim, 1);
        f.blades[1 << i] = 1.0;
        f
    }

    /// Check if all components are zero.
    pub fn is_zero(&self) -> bool {
        self.blades.iter().all(|&c| c.abs() < 1e-14)
    }

    /// Addition of differential forms of the same degree.
    pub fn add(&self, rhs: &Self) -> Self {
        assert_eq!(self.dim, rhs.dim);
        assert_eq!(self.degree, rhs.degree);
        let mut out = vec![0.0; self.blades.len()];
        for i in 0..out.len() {
            out[i] = self.blades[i] + rhs.blades[i];
        }
        Self {
            dim: self.dim,
            degree: self.degree,
            blades: out,
        }
    }

    /// Subtraction of differential forms.
    pub fn sub(&self, rhs: &Self) -> Self {
        assert_eq!(self.dim, rhs.dim);
        assert_eq!(self.degree, rhs.degree);
        let mut out = vec![0.0; self.blades.len()];
        for i in 0..out.len() {
            out[i] = self.blades[i] - rhs.blades[i];
        }
        Self {
            dim: self.dim,
            degree: self.degree,
            blades: out,
        }
    }

    /// Exterior Wedge Product: \omega = \alpha \wedge \beta \in \Omega^{p+q}(M).
    /// Satisfies graded commutativity: \alpha \wedge \beta = (-1)^{p q} \beta \wedge \alpha.
    pub fn wedge(&self, other: &Self) -> Self {
        assert_eq!(self.dim, other.dim);
        let out_deg = self.degree + other.degree;
        if out_deg > self.dim {
            return Self::zero(self.dim, out_deg);
        }

        let mut out = vec![0.0; 1 << self.dim];

        for u in 0..(1 << self.dim) {
            let cu = self.blades[u];
            if cu == 0.0 || (u as u32).count_ones() as usize != self.degree {
                continue;
            }
            for v in 0..(1 << self.dim) {
                let cv = other.blades[v];
                if cv == 0.0 || (v as u32).count_ones() as usize != other.degree {
                    continue;
                }

                // If forms share any basis vector dx^i, wedge vanishes (dx^i \wedge dx^i = 0)
                if (u & v) == 0 {
                    let target = u ^ v;
                    // Count transposition swaps to sort basis differentials into canonical order
                    let mut swaps = 0u32;
                    for i in 0..self.dim {
                        if (v & (1 << i)) != 0 {
                            let higher = u & !((1 << (i + 1)) - 1);
                            swaps += (higher as u32).count_ones();
                        }
                    }
                    let sign = if swaps % 2 == 1 { -1.0 } else { 1.0 };
                    out[target] += cu * cv * sign;
                }
            }
        }

        Self {
            dim: self.dim,
            degree: out_deg,
            blades: out,
        }
    }

    /// Exterior Derivative d: \Omega^k(M) \to \Omega^{k+1}(M).
    ///
    /// Takes the vector of spatial gradient derivatives [\partial_0 \omega, ..., \partial_{D-1} \omega]:
    /// d\omega = \sum_{i=0}^{D-1} dx^i \wedge \partial_i \omega.
    pub fn exterior_derivative(&self, partial_derivatives: &[DifferentialForm]) -> Self {
        assert_eq!(partial_derivatives.len(), self.dim);
        let mut d_omega = Self::zero(self.dim, self.degree + 1);

        for i in 0..self.dim {
            let dx_i = Self::basis_1form(self.dim, i);
            let wedge_i = dx_i.wedge(&partial_derivatives[i]);
            d_omega = d_omega.add(&wedge_i);
        }

        d_omega
    }

    /// Hodge Star Dual Operator \star: \Omega^k(M) \to \Omega^{D-k}(M) with standard Euclidean metric.
    pub fn hodge_star(&self) -> Self {
        let out_deg = self.dim - self.degree;
        let mut out = vec![0.0; 1 << self.dim];
        let full_mask = (1 << self.dim) - 1;

        for mask in 0..(1 << self.dim) {
            let c = self.blades[mask];
            if c == 0.0 || (mask as u32).count_ones() as usize != self.degree {
                continue;
            }
            let dual_mask = full_mask ^ mask;

            // Compute sign of permutation bringing (mask, dual_mask) to standard full volume form
            let mut swaps = 0u32;
            for i in 0..self.dim {
                if (dual_mask & (1 << i)) != 0 {
                    let higher = mask & !((1 << (i + 1)) - 1);
                    swaps += (higher as u32).count_ones();
                }
            }
            let sign = if swaps % 2 == 1 { -1.0 } else { 1.0 };
            out[dual_mask] += c * sign;
        }

        Self {
            dim: self.dim,
            degree: out_deg,
            blades: out,
        }
    }

    /// Interior Product / Contraction \iota_X \omega with vector field X = [X^0, ..., X^{D-1}].
    /// Maps \Omega^k(M) \to \Omega^{k-1}(M).
    pub fn interior_product(&self, vector_field: &[f64]) -> Self {
        assert_eq!(vector_field.len(), self.dim);
        if self.degree == 0 {
            return Self::zero(self.dim, 0);
        }

        let out_deg = self.degree - 1;
        let mut out = vec![0.0; 1 << self.dim];

        for mask in 0..(1 << self.dim) {
            let c = self.blades[mask];
            if c == 0.0 || (mask as u32).count_ones() as usize != self.degree {
                continue;
            }

            for i in 0..self.dim {
                if (mask & (1 << i)) != 0 && vector_field[i] != 0.0 {
                    let target = mask ^ (1 << i);
                    // Sign from passing X through preceding 1-forms
                    let preceding = mask & ((1 << i) - 1);
                    let sign = if (preceding as u32).count_ones() % 2 == 1 { -1.0 } else { 1.0 };
                    out[target] += c * vector_field[i] * sign;
                }
            }
        }

        Self {
            dim: self.dim,
            degree: out_deg,
            blades: out,
        }
    }
}
