//! # `algebra_engine::interconnect`
//!
//! Universal Cross-Domain Mathematical Bridges & Algebraic Morphisms.
//!
//! Interconnects disparate mathematical domains:
//! - **Non-Euclidean & Conformal**: Cayley isomorphism between Poincaré Disk $\mathbb{D}$ and Upper Half-Plane $\mathbb{H}^2$.
//! - **CAD & FEA/IGA Simulation**: Isogeometric NURBS surface to Finite Element `FeaMesh` discretization bridge.
//! - **Quantum & Clifford Geometry**: Massless Weyl spinors & Dirac gamma matrices into Space-Time Algebra $Cl(1, 3)$.
//! - **Differential Algebra & D-Modules**: Weyl $A_n$ operators into Ritt-Wu differential polynomials.
//! - **Octonions & Jordan Algebras**: Octonionic embeddings into Albert exceptional Jordan algebras $\mathcal{H}_3(\mathbb{O})$.
//! - **Representation Theory**: Clebsch-Gordan irrep coupling to spherical harmonics tensor products.

#![allow(clippy::needless_range_loop)]
#![allow(clippy::manual_is_multiple_of)]

use crate::diffalg::{DiffIndeterminate, DiffPolynomial, DiffTerm};
use crate::exceptional_lie::{AlbertAlgebra, Octonion};
use crate::geometric_dl::ClebschGordan;
use crate::iga::NurbsPatchND;
use crate::noneuclidean::{PoincareDiskPoint, UpperHalfPlanePoint};
use crate::pde::{ElementType, FeaMesh};
use crate::qft::WeylSpinor;
use crate::weyl_dmodules::{WeylDOperator, WeylMonomial, WeylTerm};

/// Universal Cross-Domain Mathematical Bridge.
pub struct UniversalBridge;

impl UniversalBridge {
    /// Cayley Isomorphism: Poincaré Disk point $z \in \mathbb{D}$ to Upper Half-Plane point $w \in \mathbb{H}^2$.
    /// Conformal transformation: $w = i \frac{1 + z}{1 - z} = \frac{2 \operatorname{Im}(z) + i(1 - |z|^2)}{|1 - z|^2}$.
    #[inline]
    pub fn poincare_to_upper_half_plane(p: &PoincareDiskPoint) -> UpperHalfPlanePoint {
        let x = p.x;
        let y = p.y;
        let den = (1.0 - x).powi(2) + y.powi(2);
        let u_out = 2.0 * y / den;
        let v_out = (1.0 - (x.powi(2) + y.powi(2))) / den;
        UpperHalfPlanePoint::new(u_out, v_out.max(1e-12))
            .unwrap_or(UpperHalfPlanePoint { x: u_out, y: 1.0 })
    }

    /// Inverse Cayley Isomorphism: Upper Half-Plane point $w \in \mathbb{H}^2$ to Poincaré Disk point $z \in \mathbb{D}$.
    /// Conformal transformation: $z = \frac{w - i}{w + i} = \frac{u^2 + v^2 - 1 + 2i u}{u^2 + (v + 1)^2}$.
    #[inline]
    pub fn upper_half_plane_to_poincare(p: &UpperHalfPlanePoint) -> PoincareDiskPoint {
        let u = p.x;
        let v = p.y;
        let den = u.powi(2) + (v + 1.0).powi(2);
        let z_re = (u.powi(2) + v.powi(2) - 1.0) / den;
        let z_im = (2.0 * u) / den;
        PoincareDiskPoint::new(z_re, z_im).unwrap_or(PoincareDiskPoint::origin())
    }

    /// Discretize an exact N-Dimensional NURBS CAD patch into an FEA/FEM simulation mesh (`FeaMesh`).
    pub fn nurbs_to_fea_mesh(patch: &NurbsPatchND, res_u: usize, res_v: usize) -> FeaMesh {
        assert!(res_u >= 2 && res_v >= 2);
        let mut nodes = Vec::with_capacity(res_u * res_v);
        let mut elements = Vec::with_capacity((res_u - 1) * (res_v - 1) * 2);

        // Generate grid nodes on surface
        for i in 0..res_u {
            let u = i as f64 / (res_u - 1) as f64;
            for j in 0..res_v {
                let v = j as f64 / (res_v - 1) as f64;
                let pt = patch.evaluate_surface(u, v);
                nodes.push(vec![pt[0], pt[1], pt[2]]);
            }
        }

        // Generate triangular surface elements
        for i in 0..(res_u - 1) {
            for j in 0..(res_v - 1) {
                let n00 = i * res_v + j;
                let n01 = i * res_v + (j + 1);
                let n10 = (i + 1) * res_v + j;
                let n11 = (i + 1) * res_v + (j + 1);

                // Triangle 1: (n00, n10, n01)
                elements.push(vec![n00, n10, n01]);
                // Triangle 2: (n10, n11, n01)
                elements.push(vec![n10, n11, n01]);
            }
        }

        let mut mesh = FeaMesh::new(3, ElementType::Tri3);
        mesh.nodes = nodes;
        mesh.elements = elements;
        mesh
    }

    /// Embed an Octonion $x \in \mathbb{O}$ into a 27-dimensional Albert exceptional Jordan algebra $\mathcal{H}_3(\mathbb{O})$.
    #[inline]
    pub fn octonion_to_albert_element(x: &Octonion, diag_val: f64) -> AlbertAlgebra {
        AlbertAlgebra::new(
            [diag_val, diag_val, diag_val],
            [*x, Octonion::zero(), Octonion::zero()],
        )
    }

    /// Convert a 1-variable Weyl differential operator $P \in A_1$ into a Differential Polynomial for Ritt-Wu reduction.
    ///
    /// A differential polynomial has no explicit independent variable, so only
    /// constant-coefficient operators $\sum c_k \partial^k$ correspond to it. An operator with
    /// an $x^j$ factor (`j > 0`) or more than one variable is rejected rather than having that
    /// factor dropped.
    pub fn weyl_to_diff_polynomial(
        op: &WeylDOperator,
        var_idx: usize,
    ) -> Result<DiffPolynomial, String> {
        if op.num_vars != 1 {
            return Err(format!(
                "only one-variable operators convert; this one has {} variables",
                op.num_vars
            ));
        }
        let mut terms = Vec::with_capacity(op.terms.len());
        for t in &op.terms {
            if t.monomial.x_powers.iter().any(|&p| p > 0) {
                return Err(
                    "the operator has a non-constant coefficient (an x power); a differential \
                     polynomial cannot represent it"
                        .to_string(),
                );
            }
            let order = t.monomial.d_powers.first().copied().unwrap_or(0);
            let indet = DiffIndeterminate::new(var_idx, order);
            terms.push(DiffTerm::new(t.coeff, vec![(indet, 1)]));
        }
        Ok(DiffPolynomial::new(terms))
    }

    /// Decompose product of two angular momentum states into spherical tensor harmonics irreps via Clebsch-Gordan series.
    pub fn clebsch_gordan_decomposition(
        l1: usize,
        m1: i64,
        l2: usize,
        m2: i64,
    ) -> Vec<(usize, i64, f64)> {
        let mut irreps = Vec::new();
        let l_min = (l1 as isize - l2 as isize).unsigned_abs();
        let l_max = l1 + l2;
        let m_tot = m1 + m2;

        for l in l_min..=l_max {
            if m_tot.abs() <= l as i64 {
                let cg =
                    ClebschGordan::coefficient(l1, m1 as isize, l2, m2 as isize, l, m_tot as isize);
                if cg.abs() > 1e-12 {
                    irreps.push((l, m_tot, cg));
                }
            }
        }
        irreps
    }

    /// Convert a 2-component Weyl spinor into a 4-component real polarization vector.
    #[inline]
    pub fn weyl_spinor_polarization(spinor: &WeylSpinor) -> [f64; 4] {
        // Pauli matrix projections: u^\dagger \sigma^\mu u
        let u0_re = spinor.lambda_1_re;
        let u0_im = spinor.lambda_1_im;
        let u1_re = spinor.lambda_2_re;
        let u1_im = spinor.lambda_2_im;

        let n0 = u0_re * u0_re + u0_im * u0_im;
        let n1 = u1_re * u1_re + u1_im * u1_im;

        let t = n0 + n1;
        let x = 2.0 * (u0_re * u1_re + u0_im * u1_im);
        let y = 2.0 * (u0_im * u1_re - u0_re * u1_im);
        let z = n0 - n1;

        [t, x, y, z]
    }
}

// ============================================================================
// Standard Conversion Implementations
// ============================================================================

impl From<PoincareDiskPoint> for UpperHalfPlanePoint {
    #[inline]
    fn from(p: PoincareDiskPoint) -> Self {
        UniversalBridge::poincare_to_upper_half_plane(&p)
    }
}

impl From<UpperHalfPlanePoint> for PoincareDiskPoint {
    #[inline]
    fn from(p: UpperHalfPlanePoint) -> Self {
        UniversalBridge::upper_half_plane_to_poincare(&p)
    }
}

impl From<[f64; 8]> for Octonion {
    #[inline]
    fn from(coords: [f64; 8]) -> Self {
        Octonion::new(coords)
    }
}

impl From<Octonion> for [f64; 8] {
    #[inline]
    fn from(oct: Octonion) -> Self {
        oct.coords
    }
}

/// A differential polynomial is a Weyl operator only when it is linear in a single differential
/// indeterminate: each term must be `c * y^(k)` (or a constant), which maps to `c * d^k`.
/// Products or powers of derivatives (`y * y'`, `(y')^2`) and several indeterminates have no
/// image in the Weyl algebra and are rejected.
impl TryFrom<DiffPolynomial> for WeylDOperator {
    type Error = String;

    fn try_from(poly: DiffPolynomial) -> Result<Self, Self::Error> {
        let mut variable: Option<usize> = None;
        let mut terms = Vec::with_capacity(poly.terms.len());
        for t in poly.terms {
            let order = match t.factors.as_slice() {
                [] => 0,
                [(indeterminate, 1)] => {
                    if *variable.get_or_insert(indeterminate.var_index) != indeterminate.var_index {
                        return Err("the polynomial involves more than one indeterminate".into());
                    }
                    indeterminate.order
                }
                _ => {
                    return Err(
                        "a nonlinear term (a product or power of derivatives) has no \
                         Weyl-algebra representation"
                            .into(),
                    );
                }
            };
            terms.push(WeylTerm::new(
                t.coeff,
                WeylMonomial::new(vec![0], vec![order]),
            ));
        }
        Ok(WeylDOperator { num_vars: 1, terms })
    }
}
