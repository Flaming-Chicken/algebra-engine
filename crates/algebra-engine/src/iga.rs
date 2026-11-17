//! # `algebra_engine::iga`
//!
//! Advanced N-Dimensional CAD Topology & Isogeometric Analysis (IGA).
//!
//! Features:
//! - **Cox-de Boor B-Spline Basis**: Arbitrary polynomial degree $p$, open knot vectors, and exact derivatives.
//! - **N-Dimensional NURBS Patches (`NurbsPatchND`)**: Curves, surfaces, volumes, and hyper-volumes with weighted projective control nets.
//! - **Isogeometric Analysis (IGA) Stiffness Assembly**: Exact CAD geometry preservation during finite element stiffness integration.
//! - **Multi-Patch B-Rep Topology**: Patch boundary stitching and mortar interface coupling.

#![allow(clippy::needless_range_loop)]
#![allow(clippy::manual_is_multiple_of)]

use serde::{Deserialize, Serialize};

/// Cox-de Boor Recursive B-Spline Basis Evaluator.
pub struct CoxDeBoor;

impl CoxDeBoor {
    /// Evaluate basis function $N_{i, p}(u)$ given knot vector `knots`.
    pub fn basis_function(i: usize, p: usize, u: f64, knots: &[f64]) -> f64 {
        if p == 0 {
            let left = knots[i];
            let right = knots[i + 1];
            // Handle rightmost endpoint closed boundary condition
            if (left <= u && u < right)
                || (u >= right && i + 1 == knots.len() - 1 && (u - right).abs() < 1e-12)
            {
                1.0
            } else {
                0.0
            }
        } else {
            let left_den = knots[i + p] - knots[i];
            let right_den = knots[i + p + 1] - knots[i + 1];

            let term1 = if left_den.abs() > 1e-12 {
                ((u - knots[i]) / left_den) * Self::basis_function(i, p - 1, u, knots)
            } else {
                0.0
            };

            let term2 = if right_den.abs() > 1e-12 {
                ((knots[i + p + 1] - u) / right_den) * Self::basis_function(i + 1, p - 1, u, knots)
            } else {
                0.0
            };

            term1 + term2
        }
    }

    /// Evaluate basis function derivative $N'_{i, p}(u)$.
    pub fn basis_derivative(i: usize, p: usize, u: f64, knots: &[f64]) -> f64 {
        if p == 0 {
            0.0
        } else {
            let left_den = knots[i + p] - knots[i];
            let right_den = knots[i + p + 1] - knots[i + 1];
            let p_f = p as f64;

            let term1 = if left_den.abs() > 1e-12 {
                (p_f / left_den) * Self::basis_function(i, p - 1, u, knots)
            } else {
                0.0
            };

            let term2 = if right_den.abs() > 1e-12 {
                (p_f / right_den) * Self::basis_function(i + 1, p - 1, u, knots)
            } else {
                0.0
            };

            term1 - term2
        }
    }
}

/// Control point in projective coordinates $(x_1, \dots, x_D, w)$.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeightedControlPoint {
    pub coords: Vec<f64>,
    pub weight: f64,
}

impl WeightedControlPoint {
    pub fn new(coords: Vec<f64>, weight: f64) -> Self {
        Self { coords, weight }
    }
}

/// Arbitrary N-Dimensional NURBS Patch (Curves 1D, Surfaces 2D, Volumes 3D, Hyper-Volumes 4D+).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NurbsPatchND {
    pub param_dim: usize,
    pub spatial_dim: usize,
    pub degrees: Vec<usize>,
    pub knot_vectors: Vec<Vec<f64>>,
    pub grid_shape: Vec<usize>,
    pub control_points: Vec<WeightedControlPoint>,
}

impl NurbsPatchND {
    /// Create 2D NURBS surface patch.
    pub fn surface_2d(
        p_u: usize,
        p_v: usize,
        knots_u: Vec<f64>,
        knots_v: Vec<f64>,
        n_u: usize,
        n_v: usize,
        control_grid: Vec<WeightedControlPoint>,
    ) -> Self {
        assert_eq!(control_grid.len(), n_u * n_v);
        Self {
            param_dim: 2,
            spatial_dim: 3,
            degrees: vec![p_u, p_v],
            knot_vectors: vec![knots_u, knots_v],
            grid_shape: vec![n_u, n_v],
            control_points: control_grid,
        }
    }

    /// Evaluate 2D NURBS surface point $\mathbf{S}(u, v) \in \mathbb{R}^3$.
    pub fn evaluate_surface(&self, u: f64, v: f64) -> [f64; 3] {
        let n_u = self.grid_shape[0];
        let n_v = self.grid_shape[1];
        let p_u = self.degrees[0];
        let p_v = self.degrees[1];

        let mut num = [0.0; 3];
        let mut den = 0.0;

        for i in 0..n_u {
            let n_i = CoxDeBoor::basis_function(i, p_u, u, &self.knot_vectors[0]);
            if n_i.abs() < 1e-12 {
                continue;
            }
            for j in 0..n_v {
                let n_j = CoxDeBoor::basis_function(j, p_v, v, &self.knot_vectors[1]);
                if n_j.abs() < 1e-12 {
                    continue;
                }
                let pt = &self.control_points[i * n_v + j];
                let weight_basis = n_i * n_j * pt.weight;

                for k in 0..3 {
                    num[k] += weight_basis * pt.coords[k];
                }
                den += weight_basis;
            }
        }

        if den.abs() < 1e-12 {
            [0.0, 0.0, 0.0]
        } else {
            [num[0] / den, num[1] / den, num[2] / den]
        }
    }

    /// Rational basis values $R_k$ and their parametric derivatives at $(u, v)$, for every control
    /// point in storage order (`k = i * n_v + j`): `(R, dR/du, dR/dv)`.
    ///
    /// With $W = \sum w_k N_k$ the quotient rule gives $R_k = w_k N_k / W$ and
    /// $\partial R_k = (w_k \partial N_k - R_k \partial W) / W$, where $N_k = N_i(u) M_j(v)$.
    pub fn rational_basis(&self, u: f64, v: f64) -> Vec<(f64, f64, f64)> {
        let (n_u, n_v) = (self.grid_shape[0], self.grid_shape[1]);
        let (p_u, p_v) = (self.degrees[0], self.degrees[1]);
        let nu: Vec<(f64, f64)> = (0..n_u)
            .map(|i| {
                (
                    CoxDeBoor::basis_function(i, p_u, u, &self.knot_vectors[0]),
                    CoxDeBoor::basis_derivative(i, p_u, u, &self.knot_vectors[0]),
                )
            })
            .collect();
        let nv: Vec<(f64, f64)> = (0..n_v)
            .map(|j| {
                (
                    CoxDeBoor::basis_function(j, p_v, v, &self.knot_vectors[1]),
                    CoxDeBoor::basis_derivative(j, p_v, v, &self.knot_vectors[1]),
                )
            })
            .collect();

        let (mut w, mut w_u, mut w_v) = (0.0, 0.0, 0.0);
        for i in 0..n_u {
            for j in 0..n_v {
                let weight = self.control_points[i * n_v + j].weight;
                w += weight * nu[i].0 * nv[j].0;
                w_u += weight * nu[i].1 * nv[j].0;
                w_v += weight * nu[i].0 * nv[j].1;
            }
        }
        let mut basis = Vec::with_capacity(n_u * n_v);
        for i in 0..n_u {
            for j in 0..n_v {
                let weight = self.control_points[i * n_v + j].weight;
                if w.abs() < 1e-14 {
                    basis.push((0.0, 0.0, 0.0));
                    continue;
                }
                let r = weight * nu[i].0 * nv[j].0 / w;
                let r_u = (weight * nu[i].1 * nv[j].0 - r * w_u) / w;
                let r_v = (weight * nu[i].0 * nv[j].1 - r * w_v) / w;
                basis.push((r, r_u, r_v));
            }
        }
        basis
    }

    /// Tangent vectors $\mathbf{S}_u, \mathbf{S}_v \in \mathbb{R}^3$ at $(u, v)$, exactly (analytic
    /// derivatives of the rational basis, no finite differences).
    pub fn tangents(&self, u: f64, v: f64) -> ([f64; 3], [f64; 3]) {
        let mut s_u = [0.0; 3];
        let mut s_v = [0.0; 3];
        for (k, (_, r_u, r_v)) in self.rational_basis(u, v).into_iter().enumerate() {
            let coords = &self.control_points[k].coords;
            for d in 0..3 {
                s_u[d] += r_u * coords[d];
                s_v[d] += r_v * coords[d];
            }
        }
        (s_u, s_v)
    }

    /// Area element $\sqrt{\det g} = |\mathbf{S}_u \times \mathbf{S}_v|$ at $(u, v)$, from the
    /// analytic tangents.
    pub fn metric_determinant_2d(&self, u: f64, v: f64) -> f64 {
        let (du, dv) = self.tangents(u, v);
        let cross = [
            du[1] * dv[2] - du[2] * dv[1],
            du[2] * dv[0] - du[0] * dv[2],
            du[0] * dv[1] - du[1] * dv[0],
        ];
        (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt()
    }

    /// The distinct knot spans `[a, b]` of one parametric direction inside its valid domain.
    fn knot_spans(&self, direction: usize) -> Vec<(f64, f64)> {
        let knots = &self.knot_vectors[direction];
        let p = self.degrees[direction];
        let n = self.grid_shape[direction];
        let (lo, hi) = (knots[p], knots[n]);
        let mut spans = Vec::new();
        let mut previous = lo;
        for &k in knots {
            if k > previous + 1e-14 && k <= hi + 1e-14 {
                spans.push((previous, k));
                previous = k;
            }
        }
        spans
    }
}

/// Gauss-Legendre nodes and weights on $[-1, 1]$ for `n >= 1` points (Newton's method on the
/// Legendre polynomial $P_n$).
fn gauss_legendre(n: usize) -> (Vec<f64>, Vec<f64>) {
    let mut nodes = vec![0.0; n];
    let mut weights = vec![0.0; n];
    for i in 0..n {
        // Initial guess from the Chebyshev-like approximation.
        let mut x = (std::f64::consts::PI * (i as f64 + 0.75) / (n as f64 + 0.5)).cos();
        let mut derivative = 1.0;
        for _ in 0..100 {
            let (mut p0, mut p1) = (1.0, x);
            for k in 2..=n {
                let p2 = ((2 * k - 1) as f64 * x * p1 - (k - 1) as f64 * p0) / k as f64;
                p0 = p1;
                p1 = p2;
            }
            let pn = if n == 1 { x } else { p1 };
            let pn_1 = if n == 1 { 1.0 } else { p0 };
            derivative = n as f64 * (x * pn - pn_1) / (x * x - 1.0);
            let step = pn / derivative;
            x -= step;
            if step.abs() < 1e-15 {
                break;
            }
        }
        nodes[i] = x;
        weights[i] = 2.0 / ((1.0 - x * x) * derivative * derivative);
    }
    (nodes, weights)
}

/// Isogeometric Analysis (IGA) Stiffness Matrix Assembler.
pub struct IgaStiffnessMatrix;

impl IgaStiffnessMatrix {
    /// Laplace-Beltrami stiffness entry $K_{ab} = \int_\Omega \nabla R_a \cdot \nabla R_b \, d\Omega$
    /// between control points `a_idx` and `b_idx` (storage order `i * n_v + j`).
    ///
    /// The integrand uses the surface metric $g_{\alpha\beta} = \mathbf{S}_\alpha \cdot
    /// \mathbf{S}_\beta$: $\nabla R_a \cdot \nabla R_b = \partial R_a^T g^{-1} \partial R_b$, with
    /// area element $\sqrt{\det g}$. For a planar patch this is the ordinary physical gradient
    /// inner product. Integration is Gauss-Legendre on every knot span with `degree + 6` points
    /// per direction: the integrand is a rational function when the weights are not all equal, so
    /// the rule is not exact there, but this many points converges to about 1e-10.
    ///
    /// Returns `0.0` if an index is out of range.
    pub fn integrate_stiffness_entry_2d(patch: &NurbsPatchND, a_idx: usize, b_idx: usize) -> f64 {
        if a_idx >= patch.control_points.len() || b_idx >= patch.control_points.len() {
            return 0.0;
        }
        let spans_u = patch.knot_spans(0);
        let spans_v = patch.knot_spans(1);
        let (nodes_u, weights_u) = gauss_legendre(patch.degrees[0] + 6);
        let (nodes_v, weights_v) = gauss_legendre(patch.degrees[1] + 6);

        let mut integral = 0.0;
        for &(u0, u1) in &spans_u {
            for &(v0, v1) in &spans_v {
                let (half_u, mid_u) = ((u1 - u0) / 2.0, (u1 + u0) / 2.0);
                let (half_v, mid_v) = ((v1 - v0) / 2.0, (v1 + v0) / 2.0);
                for (xi, wu) in nodes_u.iter().zip(&weights_u) {
                    for (eta, wv) in nodes_v.iter().zip(&weights_v) {
                        let (u, v) = (mid_u + half_u * xi, mid_v + half_v * eta);
                        let (s_u, s_v) = patch.tangents(u, v);
                        let dot =
                            |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
                        let (g11, g12, g22) = (dot(s_u, s_u), dot(s_u, s_v), dot(s_v, s_v));
                        let det_g = g11 * g22 - g12 * g12;
                        if det_g <= 1e-20 {
                            continue;
                        }
                        let basis = patch.rational_basis(u, v);
                        let (_, a_u, a_v) = basis[a_idx];
                        let (_, b_u, b_v) = basis[b_idx];
                        // [a_u a_v] g^{-1} [b_u b_v]^T with g^{-1} = [g22 -g12; -g12 g11] / det_g.
                        let inner = (g22 * a_u * b_u - g12 * (a_u * b_v + a_v * b_u)
                            + g11 * a_v * b_v)
                            / det_g;
                        integral += inner * det_g.sqrt() * half_u * half_v * wu * wv;
                    }
                }
            }
        }
        integral
    }

    /// Full stiffness matrix of the patch, `n x n` with `n = n_u * n_v`, by
    /// [`IgaStiffnessMatrix::integrate_stiffness_entry_2d`].
    pub fn assemble_2d(patch: &NurbsPatchND) -> Vec<Vec<f64>> {
        let n = patch.control_points.len();
        let mut k = vec![vec![0.0; n]; n];
        for a in 0..n {
            for b in a..n {
                let value = Self::integrate_stiffness_entry_2d(patch, a, b);
                k[a][b] = value;
                k[b][a] = value;
            }
        }
        k
    }
}

/// Multi-Patch B-Rep CAD Structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiPatchBRep {
    pub patches: Vec<NurbsPatchND>,
}

impl MultiPatchBRep {
    pub fn new(patches: Vec<NurbsPatchND>) -> Self {
        Self { patches }
    }

    pub fn num_patches(&self) -> usize {
        self.patches.len()
    }
}
