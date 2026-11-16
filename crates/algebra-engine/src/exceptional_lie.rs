//! # `algebra_engine::exceptional_lie`
//!
//! Exceptional Lie Groups, Octonionic Fano Plane Division Algebra & E8 Root System.
//!
//! Features:
//! - **Octonion Division Algebra ($\mathbb{O}$)**: 8D normed alternative non-associative algebra with Fano plane multiplication.
//! - **Albert Exceptional Jordan Algebra ($\mathcal{H}_3(\mathbb{O})$)**: 27D Jordan algebra with Jordan product $X \circ Y = \frac{1}{2}(XY + YX)$, trace, and Freudenthal cubic determinant.
//! - **E8 Root Lattice ($\Gamma_8$)**: All 240 root vectors of length $\sqrt{2}$, simple root basis, Cartan matrix, and Weyl reflections.

#![allow(clippy::needless_range_loop)]
#![allow(clippy::manual_is_multiple_of)]

use serde::{Deserialize, Serialize};

/// 8-Dimensional Octonion $\mathbb{O} = x_0 + \sum_{i=1}^7 x_i e_i$.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Octonion {
    pub coords: [f64; 8],
}

pub type LieOctonion = Octonion;

impl Octonion {
    /// Create new octonion from coordinates $[x_0, x_1, \dots, x_7]$.
    pub fn new(coords: [f64; 8]) -> Self {
        Self { coords }
    }

    /// Real scalar octonion $x_0 \cdot 1$.
    pub fn real(x0: f64) -> Self {
        Self {
            coords: [x0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        }
    }

    /// Basis unit $e_i$ ($i=0$ is $1$, $i=1..7$ are imaginary units).
    pub fn basis(i: usize) -> Self {
        assert!(i < 8);
        let mut coords = [0.0; 8];
        coords[i] = 1.0;
        Self { coords }
    }

    pub fn zero() -> Self {
        Self { coords: [0.0; 8] }
    }

    pub fn add(&self, other: &Self) -> Self {
        let mut res = [0.0; 8];
        for i in 0..8 {
            res[i] = self.coords[i] + other.coords[i];
        }
        Self { coords: res }
    }

    pub fn sub(&self, other: &Self) -> Self {
        let mut res = [0.0; 8];
        for i in 0..8 {
            res[i] = self.coords[i] - other.coords[i];
        }
        Self { coords: res }
    }

    pub fn scale(&self, s: f64) -> Self {
        let mut res = [0.0; 8];
        for i in 0..8 {
            res[i] = self.coords[i] * s;
        }
        Self { coords: res }
    }

    /// Octonionic conjugation $\bar{x} = x_0 - \sum_{i=1}^7 x_i e_i$.
    pub fn conjugate(&self) -> Self {
        let mut res = [0.0; 8];
        res[0] = self.coords[0];
        for i in 1..8 {
            res[i] = -self.coords[i];
        }
        Self { coords: res }
    }

    /// Euclidean norm squared $N(x) = \sum_{i=0}^7 x_i^2 = x \bar{x}$.
    pub fn norm_squared(&self) -> f64 {
        self.coords.iter().map(|&x| x * x).sum()
    }

    /// Euclidean norm $\|x\| = \sqrt{N(x)}$.
    pub fn norm(&self) -> f64 {
        self.norm_squared().sqrt()
    }

    /// Inverse $x^{-1} = \bar{x} / N(x)$.
    pub fn inverse(&self) -> Option<Self> {
        let n_sq = self.norm_squared();
        if n_sq < 1e-15 {
            None
        } else {
            Some(self.conjugate().scale(1.0 / n_sq))
        }
    }

    /// Fano plane oriented multiplication lookup for imaginary basis elements $e_a e_b$.
    /// Triples: (1, 2, 3), (1, 4, 5), (1, 7, 6), (2, 4, 6), (2, 5, 7), (3, 4, 7), (3, 6, 5).
    fn fano_mul(a: usize, b: usize) -> (usize, f64) {
        if a == 0 {
            return (b, 1.0);
        }
        if b == 0 {
            return (a, 1.0);
        }
        if a == b {
            return (0, -1.0);
        }

        const FANO_LINES: [(usize, usize, usize); 7] = [
            (1, 2, 3),
            (1, 4, 5),
            (1, 7, 6),
            (2, 4, 6),
            (2, 5, 7),
            (3, 4, 7),
            (3, 6, 5),
        ];

        for &(p, q, r) in &FANO_LINES {
            if a == p && b == q {
                return (r, 1.0);
            }
            if a == q && b == r {
                return (p, 1.0);
            }
            if a == r && b == p {
                return (q, 1.0);
            }

            if a == q && b == p {
                return (r, -1.0);
            }
            if a == r && b == q {
                return (p, -1.0);
            }
            if a == p && b == r {
                return (q, -1.0);
            }
        }
        (0, 0.0)
    }

    /// Non-associative octonion product $x \cdot y$.
    pub fn mul(&self, other: &Self) -> Self {
        let mut res = [0.0; 8];
        for a in 0..8 {
            if self.coords[a].abs() < 1e-15 {
                continue;
            }
            for b in 0..8 {
                if other.coords[b].abs() < 1e-15 {
                    continue;
                }
                let (target_idx, sign) = Self::fano_mul(a, b);
                res[target_idx] += sign * self.coords[a] * other.coords[b];
            }
        }
        Self { coords: res }
    }

    /// Associator $[x, y, z] = (xy)z - x(yz)$.
    pub fn associator(&self, y: &Self, z: &Self) -> Self {
        let xy_z = self.mul(y).mul(z);
        let x_yz = self.mul(&y.mul(z));
        xy_z.sub(&x_yz)
    }

    /// Commutator $[x, y] = xy - yx$.
    pub fn commutator(&self, y: &Self) -> Self {
        let xy = self.mul(y);
        let yx = y.mul(self);
        xy.sub(&yx)
    }
}

/// 27-Dimensional Exceptional Jordan Algebra (Albert Algebra) $\mathcal{H}_3(\mathbb{O})$.
/// Elements are $3 \times 3$ Hermitian matrices with octonionic entries:
/// Matrix $X = \begin{pmatrix} \xi_1 & x_3 & \bar{x}_2 \\ \bar{x}_3 & \xi_2 & x_1 \\ x_2 & \bar{x}_1 & \xi_3 \end{pmatrix}$.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlbertAlgebra {
    pub diag: [f64; 3],          // \xi_1, \xi_2, \xi_3
    pub off_diag: [Octonion; 3], // x_1, x_2, x_3
}

impl AlbertAlgebra {
    pub fn new(diag: [f64; 3], off_diag: [Octonion; 3]) -> Self {
        Self { diag, off_diag }
    }

    pub fn zero() -> Self {
        Self {
            diag: [0.0; 3],
            off_diag: [Octonion::zero(), Octonion::zero(), Octonion::zero()],
        }
    }

    /// Trace $\operatorname{Tr}(X) = \xi_1 + \xi_2 + \xi_3$.
    pub fn trace(&self) -> f64 {
        self.diag[0] + self.diag[1] + self.diag[2]
    }

    /// Jordan Product $X \circ Y = \frac{1}{2}(XY + YX)$.
    pub fn jordan_product(&self, other: &Self) -> Self {
        // Diagonal entries of X \circ Y:
        // (X \circ Y)_{11} = \xi_1 \eta_1 + \frac{1}{2}( x_3 \bar{y}_3 + y_3 \bar{x}_3 + \bar{x}_2 y_2 + \bar{y}_2 x_2 )
        // Note: For octonions, x \bar{y} + y \bar{x} = 2 Re(x \bar{y}) = 2 (x \cdot y).
        let d0 = self.diag[0] * other.diag[0]
            + 0.5
                * (self.off_diag[2].mul(&other.off_diag[2].conjugate()).coords[0]
                    + other.off_diag[2].mul(&self.off_diag[2].conjugate()).coords[0]
                    + self.off_diag[1].conjugate().mul(&other.off_diag[1]).coords[0]
                    + other.off_diag[1].conjugate().mul(&self.off_diag[1]).coords[0]);

        let d1 = self.diag[1] * other.diag[1]
            + 0.5
                * (self.off_diag[0].mul(&other.off_diag[0].conjugate()).coords[0]
                    + other.off_diag[0].mul(&self.off_diag[0].conjugate()).coords[0]
                    + self.off_diag[2].conjugate().mul(&other.off_diag[2]).coords[0]
                    + other.off_diag[2].conjugate().mul(&self.off_diag[2]).coords[0]);

        let d2 = self.diag[2] * other.diag[2]
            + 0.5
                * (self.off_diag[1].mul(&other.off_diag[1].conjugate()).coords[0]
                    + other.off_diag[1].mul(&self.off_diag[1].conjugate()).coords[0]
                    + self.off_diag[0].conjugate().mul(&other.off_diag[0]).coords[0]
                    + other.off_diag[0].conjugate().mul(&self.off_diag[0]).coords[0]);

        // Off-diagonal entries (x_1, x_2, x_3 components):
        // (X \circ Y)_{23} corresponds to entry x_1
        let off0 = self.off_diag[0]
            .scale(0.5 * (self.diag[1] + self.diag[2]))
            .add(&other.off_diag[0].scale(0.5 * (other.diag[1] + other.diag[2])))
            .add(
                &self.off_diag[2]
                    .conjugate()
                    .mul(&other.off_diag[1].conjugate())
                    .scale(0.5),
            )
            .add(
                &other.off_diag[2]
                    .conjugate()
                    .mul(&self.off_diag[1].conjugate())
                    .scale(0.5),
            );

        let off1 = self.off_diag[1]
            .scale(0.5 * (self.diag[2] + self.diag[0]))
            .add(&other.off_diag[1].scale(0.5 * (other.diag[2] + other.diag[0])))
            .add(
                &self.off_diag[0]
                    .conjugate()
                    .mul(&other.off_diag[2].conjugate())
                    .scale(0.5),
            )
            .add(
                &other.off_diag[0]
                    .conjugate()
                    .mul(&self.off_diag[2].conjugate())
                    .scale(0.5),
            );

        let off2 = self.off_diag[2]
            .scale(0.5 * (self.diag[0] + self.diag[1]))
            .add(&other.off_diag[2].scale(0.5 * (other.diag[0] + other.diag[1])))
            .add(
                &self.off_diag[1]
                    .conjugate()
                    .mul(&other.off_diag[0].conjugate())
                    .scale(0.5),
            )
            .add(
                &other.off_diag[1]
                    .conjugate()
                    .mul(&self.off_diag[0].conjugate())
                    .scale(0.5),
            );

        Self {
            diag: [d0, d1, d2],
            off_diag: [off0, off1, off2],
        }
    }

    /// Freudenthal Cubic Determinant $\det(X) = \xi_1 \xi_2 \xi_3 - \xi_1 N(x_1) - \xi_2 N(x_2) - \xi_3 N(x_3) + 2 \operatorname{Re}(x_1 (x_2 x_3))$.
    pub fn determinant(&self) -> f64 {
        let term1 = self.diag[0] * self.diag[1] * self.diag[2];
        let term2 = self.diag[0] * self.off_diag[0].norm_squared();
        let term3 = self.diag[1] * self.off_diag[1].norm_squared();
        let term4 = self.diag[2] * self.off_diag[2].norm_squared();

        let x1_x2x3 = self.off_diag[0].mul(&self.off_diag[1].mul(&self.off_diag[2]));
        let re_cubic = 2.0 * x1_x2x3.coords[0];

        term1 - term2 - term3 - term4 + re_cubic
    }
}

/// Exceptional E8 Root Lattice $\Gamma_8 \subset \mathbb{R}^8$.
pub struct E8Lattice;

impl E8Lattice {
    /// Generate all 240 root vectors of length $\sqrt{2}$.
    pub fn all_roots() -> Vec<[f64; 8]> {
        let mut roots = Vec::with_capacity(240);

        // 1. 112 roots of form (\pm 1, \pm 1, 0, 0, 0, 0, 0, 0)
        for i in 0..8 {
            for j in (i + 1)..8 {
                for &s1 in &[-1.0, 1.0] {
                    for &s2 in &[-1.0, 1.0] {
                        let mut root = [0.0; 8];
                        root[i] = s1;
                        root[j] = s2;
                        roots.push(root);
                    }
                }
            }
        }

        // 2. 128 roots of form (\pm 1/2, ..., \pm 1/2) with even number of minus signs
        for mask in 0..256 {
            let num_minus = (0..8).filter(|&k| (mask & (1 << k)) != 0).count();
            if num_minus % 2 == 0 {
                let mut root = [0.5; 8];
                for k in 0..8 {
                    if (mask & (1 << k)) != 0 {
                        root[k] = -0.5;
                    }
                }
                roots.push(root);
            }
        }

        assert_eq!(roots.len(), 240);
        roots
    }

    /// Standard Simple Root basis $\alpha_1, \dots, \alpha_8$ for E8.
    pub fn simple_roots() -> [[f64; 8]; 8] {
        [
            [0.5, -0.5, -0.5, -0.5, -0.5, -0.5, -0.5, 0.5], // \alpha_1
            [1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],       // \alpha_2
            [-1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],      // \alpha_3
            [0.0, -1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],      // \alpha_4
            [0.0, 0.0, -1.0, 1.0, 0.0, 0.0, 0.0, 0.0],      // \alpha_5
            [0.0, 0.0, 0.0, -1.0, 1.0, 0.0, 0.0, 0.0],      // \alpha_6
            [0.0, 0.0, 0.0, 0.0, -1.0, 1.0, 0.0, 0.0],      // \alpha_7
            [0.0, 0.0, 0.0, 0.0, 0.0, -1.0, 1.0, 0.0],      // \alpha_8
        ]
    }

    /// E8 Cartan Matrix $A_{ij} = \alpha_i \cdot \alpha_j$.
    pub fn cartan_matrix() -> [[i32; 8]; 8] {
        let simples = Self::simple_roots();
        let mut cartan = [[0; 8]; 8];
        for i in 0..8 {
            for j in 0..8 {
                let dot: f64 = simples[i]
                    .iter()
                    .zip(simples[j].iter())
                    .map(|(&a, &b)| a * b)
                    .sum();
                cartan[i][j] = dot.round() as i32;
            }
        }
        cartan
    }

    /// Weyl Reflection of vector $v$ with respect to root $\alpha$: $s_\alpha(v) = v - 2\frac{v \cdot \alpha}{\alpha \cdot \alpha}\alpha$.
    pub fn weyl_reflect(v: &[f64; 8], alpha: &[f64; 8]) -> [f64; 8] {
        let v_dot_alpha: f64 = v.iter().zip(alpha.iter()).map(|(&a, &b)| a * b).sum();
        let alpha_sq: f64 = alpha.iter().map(|&a| a * a).sum();
        let factor = 2.0 * v_dot_alpha / alpha_sq;

        let mut res = [0.0; 8];
        for i in 0..8 {
            res[i] = v[i] - factor * alpha[i];
        }
        res
    }
}
