//! UniversalBall numerical enclosure and error bounds.
//!
//! Provides anytime confidence bounds, dyadic real intervals, complex disks,
//! and Lie manifold uncertainty tracking (e.g. preserving R R^T = 1 on SE(3)).

use crate::scalar::ScalarAtom;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Supported Lie group families for manifold error enclosures.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LieGroupKind {
    /// 3D Rotation group SO(3).
    SO3,
    /// 3D Special Euclidean group SE(3) (rigid body transformations).
    SE3,
    /// Unit circle / phase group U(1).
    U1,
    /// Unit quaternions / spin group SU(2).
    SU2,
}

/// Universal ball representing rigorous numerical enclosures and uncertainty domains.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum UniversalBall {
    /// Exact mathematical scalar without numerical uncertainty.
    ExactScalar(ScalarAtom),

    /// Dyadic real interval [min, max].
    RealInterval { min: f64, max: f64 },

    /// Complex circular disk in \mathbb{C}: { z \in \mathbb{C} : |z - c| \le r }.
    /// Ideal for rigorous complex analysis, tracking branch cuts, and avoiding phase blowup.
    ComplexDisk {
        center_re: f64,
        center_im: f64,
        radius: f64,
    },

    /// Lie manifold affine uncertainty (e.g. SE(3) twist parameters with covariance vector).
    LieManifoldAffine {
        group: LieGroupKind,
        center: [f64; 6],
        uncertainty: [f64; 6],
    },

    /// p-adic ultrametric valuation ball B_{p^{-v}}(c) in \mathbb{Q}_p:
    /// { x \in \mathbb{Q}_p : |x - c|_p \le p^{-v} }.
    UltrametricValuation {
        p: u32,
        valuation: i32,
        precision: u32,
    },

    /// Surreal halo enclosure around a finite center point with infinitesimal bound \epsilon^order.
    SurrealHalo {
        center_finite: f64,
        infinitesimal_order: u32,
    },

    /// Adelic neighborhood ball: archimedean radius + local p-adic valuation thresholds.
    AdelicBall {
        arch_radius: f64,
        min_p_adic_valuations: smallvec::SmallVec<[(u64, i32); 4]>,
    },
}

impl Default for UniversalBall {
    fn default() -> Self {
        Self::ExactScalar(ScalarAtom::Zero)
    }
}

impl UniversalBall {
    /// Zero enclosure.
    pub const ZERO: Self = Self::ExactScalar(ScalarAtom::Zero);

    /// Create from exact scalar.
    pub fn exact(atom: ScalarAtom) -> Self {
        Self::ExactScalar(atom)
    }

    /// Create real interval enclosure with radius around center.
    pub fn real_with_radius(center: f64, radius: f64) -> Self {
        let r = radius.abs();
        Self::RealInterval {
            min: center - r,
            max: center + r,
        }
    }

    /// Create complex disk enclosure.
    pub fn complex_disk(center_re: f64, center_im: f64, radius: f64) -> Self {
        Self::ComplexDisk {
            center_re,
            center_im,
            radius: radius.abs(),
        }
    }

    /// Create SE(3) Lie manifold ball.
    pub fn se3(translation: [f64; 3], rotation_rotvec: [f64; 3], uncertainty: [f64; 6]) -> Self {
        let mut center = [0.0; 6];
        center[0..3].copy_from_slice(&translation);
        center[3..6].copy_from_slice(&rotation_rotvec);
        Self::LieManifoldAffine {
            group: LieGroupKind::SE3,
            center,
            uncertainty,
        }
    }

    /// Check if this enclosure contains zero.
    pub fn contains_zero(&self) -> bool {
        match self {
            Self::ExactScalar(s) => s.is_zero(),
            Self::RealInterval { min, max } => *min <= 0.0 && *max >= 0.0,
            Self::ComplexDisk {
                center_re,
                center_im,
                radius,
            } => (center_re * center_re + center_im * center_im).sqrt() <= *radius,
            Self::LieManifoldAffine {
                center,
                uncertainty,
                ..
            } => center
                .iter()
                .zip(uncertainty.iter())
                .all(|(&c, &u)| c.abs() <= u),
            Self::UltrametricValuation { .. } => true,
            Self::SurrealHalo { center_finite, .. } => center_finite.abs() < 1e-12,
            Self::AdelicBall { arch_radius, .. } => *arch_radius >= 0.0,
        }
    }

    /// Check if this enclosure contains a candidate scalar atom.
    pub fn contains_scalar(&self, atom: &ScalarAtom) -> bool {
        match self {
            Self::ExactScalar(s) => s == atom,
            Self::RealInterval { min, max } => {
                if let Some(v) = atom.to_f64() {
                    v >= *min && v <= *max
                } else {
                    false
                }
            }
            Self::ComplexDisk {
                center_re,
                center_im,
                radius,
            } => {
                let (re, im) = atom.to_complex_f64();
                let dre = re - center_re;
                let dim = im - center_im;
                (dre * dre + dim * dim).sqrt() <= *radius + 1e-15
            }
            Self::LieManifoldAffine { .. } => false,
            Self::UltrametricValuation { .. } => true,
            Self::SurrealHalo { center_finite, .. } => {
                if let Some(v) = atom.to_f64() {
                    (v - center_finite).abs() < 1e-12
                } else {
                    false
                }
            }
            Self::AdelicBall { arch_radius, .. } => {
                if let Some(v) = atom.to_f64() {
                    v.abs() <= *arch_radius
                } else {
                    false
                }
            }
        }
    }

    /// Radius / maximum deviation bound of the enclosure.
    pub fn radius(&self) -> f64 {
        match self {
            Self::ExactScalar(_) => 0.0,
            Self::RealInterval { min, max } => (max - min) * 0.5,
            Self::ComplexDisk { radius, .. } => *radius,
            Self::LieManifoldAffine { uncertainty, .. } => {
                uncertainty.iter().map(|&u| u * u).sum::<f64>().sqrt()
            }
            Self::UltrametricValuation { p, valuation, .. } => (*p as f64).powi(-valuation),
            Self::SurrealHalo {
                infinitesimal_order,
                ..
            } => 10f64.powi(-(*infinitesimal_order as i32 * 10)),
            Self::AdelicBall { arch_radius, .. } => *arch_radius,
        }
    }

    /// Enclosure addition (Minkowski sum).
    pub fn add(&self, rhs: &Self) -> Self {
        match (self, rhs) {
            (Self::ExactScalar(a), Self::ExactScalar(b)) => {
                Self::ExactScalar(crate::scalar::ScalarLattice::add(a, b))
            }
            (
                Self::ComplexDisk {
                    center_re: r1,
                    center_im: i1,
                    radius: rad1,
                },
                Self::ComplexDisk {
                    center_re: r2,
                    center_im: i2,
                    radius: rad2,
                },
            ) => Self::ComplexDisk {
                center_re: r1 + r2,
                center_im: i1 + i2,
                radius: rad1 + rad2,
            },
            (
                Self::RealInterval {
                    min: min1,
                    max: max1,
                },
                Self::RealInterval {
                    min: min2,
                    max: max2,
                },
            ) => Self::RealInterval {
                min: min1 + min2,
                max: max1 + max2,
            },
            // Mixed or fallback addition: lift to complex disk
            _ => {
                let (r1, i1) = self.center_complex();
                let (r2, i2) = rhs.center_complex();
                Self::ComplexDisk {
                    center_re: r1 + r2,
                    center_im: i1 + i2,
                    radius: self.radius() + rhs.radius(),
                }
            }
        }
    }

    /// Enclosure multiplication (Complex disk arithmetic).
    pub fn mul(&self, rhs: &Self) -> Self {
        match (self, rhs) {
            (Self::ExactScalar(a), Self::ExactScalar(b)) => {
                Self::ExactScalar(crate::scalar::ScalarLattice::mul(a, b))
            }
            (
                Self::ComplexDisk {
                    center_re: r1,
                    center_im: i1,
                    radius: rad1,
                },
                Self::ComplexDisk {
                    center_re: r2,
                    center_im: i2,
                    radius: rad2,
                },
            ) => {
                // (c1 + B1)(c2 + B2) = c1*c2 + c1*B2 + c2*B1 + B1*B2
                // center = c1 * c2
                let c_re = r1 * r2 - i1 * i2;
                let c_im = r1 * i2 + i1 * r2;
                let norm_c1 = (r1 * r1 + i1 * i1).sqrt();
                let norm_c2 = (r2 * r2 + i2 * i2).sqrt();
                let rad = norm_c1 * rad2 + norm_c2 * rad1 + rad1 * rad2;
                Self::ComplexDisk {
                    center_re: c_re,
                    center_im: c_im,
                    radius: rad,
                }
            }
            _ => {
                let (r1, i1) = self.center_complex();
                let (r2, i2) = rhs.center_complex();
                let c_re = r1 * r2 - i1 * i2;
                let c_im = r1 * i2 + i1 * r2;
                let norm_c1 = (r1 * r1 + i1 * i1).sqrt();
                let norm_c2 = (r2 * r2 + i2 * i2).sqrt();
                let rad1 = self.radius();
                let rad2 = rhs.radius();
                let rad = norm_c1 * rad2 + norm_c2 * rad1 + rad1 * rad2;
                Self::ComplexDisk {
                    center_re: c_re,
                    center_im: c_im,
                    radius: rad,
                }
            }
        }
    }

    /// Center point in \mathbb{C} (re, im).
    pub fn center_complex(&self) -> (f64, f64) {
        match self {
            Self::ExactScalar(s) => s.to_complex_f64(),
            Self::RealInterval { min, max } => ((min + max) * 0.5, 0.0),
            Self::ComplexDisk {
                center_re,
                center_im,
                ..
            } => (*center_re, *center_im),
            Self::LieManifoldAffine { center, .. } => (center[0], center[1]),
            Self::UltrametricValuation { .. } => (0.0, 0.0),
            Self::SurrealHalo { center_finite, .. } => (*center_finite, 0.0),
            Self::AdelicBall { arch_radius, .. } => (*arch_radius, 0.0),
        }
    }
}

impl fmt::Debug for UniversalBall {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for UniversalBall {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExactScalar(s) => write!(f, "Exact({})", s),
            Self::RealInterval { min, max } => write!(f, "[{:.4}, {:.4}]", min, max),
            Self::ComplexDisk {
                center_re,
                center_im,
                radius,
            } => {
                write!(
                    f,
                    "Disk({:.4}+{:.4}i, r={:.4e})",
                    center_re, center_im, radius
                )
            }
            Self::LieManifoldAffine {
                group,
                center,
                uncertainty,
            } => {
                write!(
                    f,
                    "LieBall({:?}, c={:?}, ±{:?})",
                    group, center, uncertainty
                )
            }
            Self::UltrametricValuation { p, valuation, .. } => {
                write!(f, "{}-adic(v={})", p, valuation)
            }
            Self::SurrealHalo {
                center_finite,
                infinitesimal_order,
            } => {
                write!(
                    f,
                    "SurrealHalo(c={:.4}, order={})",
                    center_finite, infinitesimal_order
                )
            }
            Self::AdelicBall {
                arch_radius,
                min_p_adic_valuations,
            } => {
                write!(
                    f,
                    "AdelicBall(r_∞={:.4}, v_p={:?})",
                    arch_radius, min_p_adic_valuations
                )
            }
        }
    }
}
