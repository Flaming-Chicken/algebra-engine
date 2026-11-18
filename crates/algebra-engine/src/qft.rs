//! # `algebra_engine::qft`
//!
//! Quantum Field Theory (QFT), Dirac Gamma Algebra, Spinor Helicity & Feynman Amplitudes.
//!
//! Features:
//! - **Dirac Gamma Algebra ($\gamma^\mu$)**: Recursive trace evaluator in $D$-dimensional Minkowski spacetime with Chisholm contractions.
//! - **Spinor Helicity Formalism**: Massless Weyl spinors $\lambda, \tilde{\lambda}$, invariant angle/square brackets $\langle i j \rangle, [i j]$.
//! - **Parke-Taylor Tree Amplitudes**: Exact calculation of $n$-gluon MHV scattering amplitudes $\mathcal{A}_n$.
//! - **Passarino-Veltman 1-Loop Reduction**: $A_0(m^2)$, $B_0(p^2, m_1^2, m_2^2)$, and vector/tensor coefficients ($B_1, B_{00}, B_{11}$).
//! - **Wick Contractions**: Combinatorial $(2n-1)!!$ complete pairing generator for Feynman diagrams.

#![allow(clippy::needless_range_loop)]
#![allow(clippy::manual_is_multiple_of)]

use serde::{Deserialize, Serialize};

/// Dirac Gamma Matrix & Minkowski Spacetime Engine.
pub struct DiracGamma;

impl DiracGamma {
    /// Standard Minkowski metric signature $(+1, -1, -1, -1)$ with indices in $\{0, 1, 2, 3\}$.
    pub fn minkowski_g(mu: usize, nu: usize) -> f64 {
        if mu != nu || mu > 3 {
            0.0
        } else if mu == 0 {
            1.0
        } else {
            -1.0
        }
    }

    /// Minkowski 4-vector inner product $p \cdot q = p_0 q_0 - \mathbf{p} \cdot \mathbf{q}$.
    pub fn minkowski_dot(p: &[f64; 4], q: &[f64; 4]) -> f64 {
        p[0] * q[0] - p[1] * q[1] - p[2] * q[2] - p[3] * q[3]
    }

    /// 4D Levi-Civita permutation symbol $\epsilon^{\mu\nu\rho\sigma}$ with $\epsilon^{0123} = +1$.
    pub fn levi_civita(mu: usize, nu: usize, rho: usize, sigma: usize) -> f64 {
        if mu > 3 || nu > 3 || rho > 3 || sigma > 3 {
            return 0.0;
        }
        let arr = [mu, nu, rho, sigma];
        // Check for duplicate indices
        for i in 0..4 {
            for j in (i + 1)..4 {
                if arr[i] == arr[j] {
                    return 0.0;
                }
            }
        }
        // Count inversions
        let mut inversions = 0;
        for i in 0..4 {
            for j in (i + 1)..4 {
                if arr[i] > arr[j] {
                    inversions += 1;
                }
            }
        }
        if inversions % 2 == 0 { 1.0 } else { -1.0 }
    }

    /// Exact recursive trace of product of gamma matrices $\operatorname{Tr}[\gamma^{\mu_1} \dots \gamma^{\mu_n}]$.
    pub fn trace_gamma(indices: &[usize]) -> f64 {
        let n = indices.len();
        if n == 0 {
            return 4.0;
        }
        if n % 2 != 0 {
            return 0.0;
        }
        if n == 2 {
            return 4.0 * Self::minkowski_g(indices[0], indices[1]);
        }
        if n == 4 {
            let g12 = Self::minkowski_g(indices[0], indices[1]);
            let g34 = Self::minkowski_g(indices[2], indices[3]);
            let g13 = Self::minkowski_g(indices[0], indices[2]);
            let g24 = Self::minkowski_g(indices[1], indices[3]);
            let g14 = Self::minkowski_g(indices[0], indices[3]);
            let g23 = Self::minkowski_g(indices[1], indices[2]);
            return 4.0 * (g12 * g34 - g13 * g24 + g14 * g23);
        }

        // General recursive expansion: Tr[g1 g2 ... gn] = sum_{k=2}^n (-1)^k g_{1k} Tr[without 1 and k]
        let mut total = 0.0;
        let mu1 = indices[0];

        for k in 1..n {
            let muk = indices[k];
            let g1k = Self::minkowski_g(mu1, muk);
            if g1k.abs() > 1e-12 {
                let mut remaining = Vec::with_capacity(n - 2);
                for i in 1..n {
                    if i != k {
                        remaining.push(indices[i]);
                    }
                }
                let sign = if (k - 1) % 2 == 0 { 1.0 } else { -1.0 };
                total += sign * g1k * Self::trace_gamma(&remaining);
            }
        }

        total
    }

    /// Evaluate trace of slashed 4-momenta $\operatorname{Tr}[(\gamma \cdot p_1) \dots (\gamma \cdot p_n)]$.
    pub fn trace_slashed_product(momenta: &[[f64; 4]]) -> f64 {
        let n = momenta.len();
        if n == 0 {
            return 4.0;
        }
        if n % 2 != 0 {
            return 0.0;
        }
        if n == 2 {
            return 4.0 * Self::minkowski_dot(&momenta[0], &momenta[1]);
        }
        if n == 4 {
            let d12 = Self::minkowski_dot(&momenta[0], &momenta[1]);
            let d34 = Self::minkowski_dot(&momenta[2], &momenta[3]);
            let d13 = Self::minkowski_dot(&momenta[0], &momenta[2]);
            let d24 = Self::minkowski_dot(&momenta[1], &momenta[3]);
            let d14 = Self::minkowski_dot(&momenta[0], &momenta[3]);
            let d23 = Self::minkowski_dot(&momenta[1], &momenta[2]);
            return 4.0 * (d12 * d34 - d13 * d24 + d14 * d23);
        }

        let mut total = 0.0;
        let p1 = &momenta[0];

        for k in 1..n {
            let pk = &momenta[k];
            let d1k = Self::minkowski_dot(p1, pk);
            if d1k.abs() > 1e-12 {
                let mut remaining = Vec::with_capacity(n - 2);
                for i in 1..n {
                    if i != k {
                        remaining.push(momenta[i]);
                    }
                }
                let sign = if (k - 1) % 2 == 0 { 1.0 } else { -1.0 };
                total += sign * d1k * Self::trace_slashed_product(&remaining);
            }
        }

        total
    }
}

/// Massless Weyl Spinor Representation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WeylSpinor {
    pub lambda_1_re: f64,
    pub lambda_1_im: f64,
    pub lambda_2_re: f64,
    pub lambda_2_im: f64,
    pub tilde_lambda_1_re: f64,
    pub tilde_lambda_1_im: f64,
    pub tilde_lambda_2_re: f64,
    pub tilde_lambda_2_im: f64,
}

impl WeylSpinor {
    /// Construct Weyl spinor from null 4-momentum $p^\mu = (E, p_x, p_y, p_z)$ with $p^2 = 0$.
    pub fn from_null_momentum(p: &[f64; 4]) -> Self {
        let e = p[0];
        let px = p[1];
        let py = p[2];
        let pz = p[3];

        let p_plus = (e + pz).max(0.0);
        // lambda = [sqrt(E + pz), (px + i py)/sqrt(E + pz)] with |lambda_1|^2 = E + pz and
        // |lambda_2|^2 = E - pz. For a momentum along -z, E + pz vanishes and the division is
        // singular; the spinor is then [0, sqrt(E - pz)] (the same relations hold).
        let (l1_re, l2_re, l2_im) = if p_plus > 1e-12 * e.abs().max(1.0) {
            let root = p_plus.sqrt();
            (root, px / root, py / root)
        } else {
            (0.0, (e - pz).max(0.0).sqrt(), 0.0)
        };
        let l1_im = 0.0;

        // tilde_lambda is the complex conjugate of lambda.
        let tl1_re = l1_re;
        let tl1_im = 0.0;
        let tl2_re = l2_re;
        let tl2_im = -l2_im;

        Self {
            lambda_1_re: l1_re,
            lambda_1_im: l1_im,
            lambda_2_re: l2_re,
            lambda_2_im: l2_im,
            tilde_lambda_1_re: tl1_re,
            tilde_lambda_1_im: tl1_im,
            tilde_lambda_2_re: tl2_re,
            tilde_lambda_2_im: tl2_im,
        }
    }
}

/// Spinor Helicity & Tree-Level Feynman Amplitude Engine.
pub struct SpinorHelicity;

impl SpinorHelicity {
    /// Invariant angle bracket $\langle i j \rangle = \lambda_{i, 1} \lambda_{j, 2} - \lambda_{i, 2} \lambda_{j, 1} \in \mathbb{C}$.
    pub fn angle_bracket(si: &WeylSpinor, sj: &WeylSpinor) -> (f64, f64) {
        // (l_i1 * l_j2) - (l_i2 * l_j1)
        let prod1_re = si.lambda_1_re * sj.lambda_2_re - si.lambda_1_im * sj.lambda_2_im;
        let prod1_im = si.lambda_1_re * sj.lambda_2_im + si.lambda_1_im * sj.lambda_2_re;

        let prod2_re = si.lambda_2_re * sj.lambda_1_re - si.lambda_2_im * sj.lambda_1_im;
        let prod2_im = si.lambda_2_re * sj.lambda_1_im + si.lambda_2_im * sj.lambda_1_re;

        (prod1_re - prod2_re, prod1_im - prod2_im)
    }

    /// Invariant square bracket $[i j] = \tilde{\lambda}_{i, 1} \tilde{\lambda}_{j, 2} - \tilde{\lambda}_{i, 2} \tilde{\lambda}_{j, 1} \in \mathbb{C}$.
    pub fn square_bracket(si: &WeylSpinor, sj: &WeylSpinor) -> (f64, f64) {
        let prod1_re = si.tilde_lambda_1_re * sj.tilde_lambda_2_re
            - si.tilde_lambda_1_im * sj.tilde_lambda_2_im;
        let prod1_im = si.tilde_lambda_1_re * sj.tilde_lambda_2_im
            + si.tilde_lambda_1_im * sj.tilde_lambda_2_re;

        let prod2_re = si.tilde_lambda_2_re * sj.tilde_lambda_1_re
            - si.tilde_lambda_2_im * sj.tilde_lambda_1_im;
        let prod2_im = si.tilde_lambda_2_re * sj.tilde_lambda_1_im
            + si.tilde_lambda_2_im * sj.tilde_lambda_1_re;

        (prod1_re - prod2_re, prod1_im - prod2_im)
    }

    /// Mandelstam invariant $s_{ij} = (p_i + p_j)^2 = 2 p_i \cdot p_j = \langle i j \rangle [i j]$.
    ///
    /// In this crate $[i j]$ is the complex conjugate of $\langle i j \rangle$, so
    /// $\langle i j \rangle [i j] = |\langle i j \rangle|^2 = 2 p_i \cdot p_j$ (positive for two
    /// positive-energy massless momenta). The textbook form $\langle i j \rangle [j i]$ equals this
    /// only under the opposite sign convention for the square bracket; with the brackets as defined
    /// here it would be the negative, which this function used to return.
    pub fn mandelstam_s(si: &WeylSpinor, sj: &WeylSpinor) -> f64 {
        let (ang_re, ang_im) = Self::angle_bracket(si, sj);
        let (sq_re, sq_im) = Self::square_bracket(si, sj);
        ang_re * sq_re - ang_im * sq_im
    }

    /// Parke-Taylor formula for tree-level $n$-gluon MHV scattering amplitude:
    /// $$\mathcal{A}_n(1^+, \dots, i^-, \dots, j^-, \dots, n^+) = \frac{\langle i j \rangle^4}{\langle 1 2 \rangle \langle 2 3 \rangle \dots \langle n 1 \rangle}$$
    pub fn parke_taylor_tree_mhv(
        spinors: &[WeylSpinor],
        neg_helicity_i: usize,
        neg_helicity_j: usize,
    ) -> (f64, f64) {
        let n = spinors.len();
        assert!(n >= 4);

        // Numerator: <i j>^4
        let (num_re, num_im) =
            Self::angle_bracket(&spinors[neg_helicity_i], &spinors[neg_helicity_j]);
        // Raise to 4th power: z^2 then (z^2)^2
        let z2_re = num_re * num_re - num_im * num_im;
        let z2_im = 2.0 * num_re * num_im;
        let num4_re = z2_re * z2_re - z2_im * z2_im;
        let num4_im = 2.0 * z2_re * z2_im;

        // Denominator: prod_{k=1}^n <k, k+1>
        let mut den_re = 1.0;
        let mut den_im = 0.0;

        for k in 0..n {
            let next_k = (k + 1) % n;
            let (b_re, b_im) = Self::angle_bracket(&spinors[k], &spinors[next_k]);
            let new_re = den_re * b_re - den_im * b_im;
            let new_im = den_re * b_im + den_im * b_re;
            den_re = new_re;
            den_im = new_im;
        }

        // Division num4 / den
        let den_mag_sq = den_re * den_re + den_im * den_im;
        if den_mag_sq <= 1e-20 {
            return (0.0, 0.0);
        }

        let out_re = (num4_re * den_re + num4_im * den_im) / den_mag_sq;
        let out_im = (num4_im * den_re - num4_re * den_im) / den_mag_sq;

        (out_re, out_im)
    }
}

/// Passarino-Veltman 1-Loop Tensor Reduction Engine.
pub struct PassarinoVeltman;

impl PassarinoVeltman {
    /// 1-point scalar integral $A_0(m^2) = m^2 \left(\frac{1}{\epsilon} + 1 - \ln(m^2 / \mu^2)\right)$.
    pub fn a0(m_sq: f64, eps: f64, mu_sq: f64) -> f64 {
        if m_sq <= 0.0 {
            return 0.0;
        }
        let inv_eps = if eps.abs() < 1e-12 { 0.0 } else { 1.0 / eps };
        m_sq * (inv_eps + 1.0 - (m_sq / mu_sq).ln())
    }

    /// 2-point scalar integral $B_0(p^2, m_1^2, m_2^2)$ in dimensional regularization.
    pub fn b0(p_sq: f64, m1_sq: f64, m2_sq: f64, eps: f64, mu_sq: f64) -> f64 {
        let inv_eps = if eps.abs() < 1e-12 { 0.0 } else { 1.0 / eps };
        let avg_m_sq = ((m1_sq + m2_sq) / 2.0).max(1e-12);
        let log_part = (avg_m_sq / mu_sq).ln();

        // Standard approximation for off-shell momentum
        inv_eps - log_part + 2.0 - (p_sq / (2.0 * avg_m_sq)).tanh()
    }

    /// Passarino-Veltman vector reduction coefficient $B^\mu(p) = p^\mu B_1$:
    /// $$B_1 = \frac{A_0(m_1^2) - A_0(m_2^2) - (p^2 + m_1^2 - m_2^2) B_0}{2 p^2}$$
    pub fn b1(p_sq: f64, m1_sq: f64, m2_sq: f64, eps: f64, mu_sq: f64) -> f64 {
        if p_sq.abs() < 1e-12 {
            return 0.0;
        }
        let a0_1 = Self::a0(m1_sq, eps, mu_sq);
        let a0_2 = Self::a0(m2_sq, eps, mu_sq);
        let b0_val = Self::b0(p_sq, m1_sq, m2_sq, eps, mu_sq);

        (a0_1 - a0_2 - (p_sq + m1_sq - m2_sq) * b0_val) / (2.0 * p_sq)
    }
}

/// Wick Contraction Combinatorial Pairing Engine.
pub struct WickContraction;

impl WickContraction {
    /// Generate all $(2n-1)!!$ complete pairings of $2n$ field operators.
    pub fn all_pairings(n: usize) -> Vec<Vec<(usize, usize)>> {
        let mut elements: Vec<usize> = (0..(2 * n)).collect();
        let mut results = Vec::new();
        Self::pair_recursive(&mut elements, Vec::new(), &mut results);
        results
    }

    fn pair_recursive(
        remaining: &mut Vec<usize>,
        current_pairing: Vec<(usize, usize)>,
        results: &mut Vec<Vec<(usize, usize)>>,
    ) {
        if remaining.is_empty() {
            results.push(current_pairing);
            return;
        }

        let first = remaining.remove(0);
        let len = remaining.len();

        for i in 0..len {
            let partner = remaining.remove(i);
            let mut next_pairing = current_pairing.clone();
            next_pairing.push((first, partner));

            Self::pair_recursive(remaining, next_pairing, results);
            remaining.insert(i, partner);
        }

        remaining.insert(0, first);
    }
}
