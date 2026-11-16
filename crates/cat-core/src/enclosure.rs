//! Standardized 4-Fidelity Evaluation Modes, Moderated Error Enclosures,
//! and Temporary Form Preservation (`Hold`) for `cat-core` v2.1.

use crate::ball::UniversalBall;
use crate::term::CatTerm;
use serde::{Deserialize, Serialize};

/// Standardized 4-tier fidelity selection for symbolic/numeric operations.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum FidelityMode {
    /// Tier 0: Fast numerical approximation with rigorous error enclosure (target <= 50 us).
    NumericalApprox,
    /// Tier 1: Domain-specific symbolic approximation (Taylor/Padé/linearisation + O(...) remainder).
    SymbolicApprox,
    /// Tier 2: Full-accuracy budgeted symbolic computation.
    SymbolicExact,
    /// Tier 3: Full-accuracy symbolic computation seeded/pruned by an early automatic approximation.
    #[default]
    SymbolicHybrid,
}

/// Controls whether lossless smart-constructor canonicalization is active.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum EvalForm {
    /// Apply generality-preserving simplifications continuously on construction.
    #[default]
    Canonical,
    /// Preserve the original unsimplified structure temporarily (e.g. step-by-step proofs,
    /// pattern-matching rules, or unverified branch-cut/zero-divisor guards).
    Hold,
}

/// Parametric family for probabilistic error spreads.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum SpreadKind {
    Gaussian,
    StudentT { dof: f64 },
    MomentBound,
    PolynomialChaos,
}

/// Moderated 5-kind uncertainty lattice preventing silent conflation of
/// hard intervals, probabilistic bell curves, and disjoint solution sets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum UncertaintyKind {
    /// Zero error (`delta = 0`).
    #[default]
    Exact,
    /// Hard worst-case enclosure (`|x - x_hat| <= eps`).
    WorstCaseBall(UniversalBall),
    /// Probabilistic spread with optional Chebyshev/Hoeffding PAC tail delta.
    ProbabilisticSpread {
        mean: f64,
        std_dev: f64,
        dist: SpreadKind,
        pac_delta: Option<f64>,
    },
    /// Disjoint interval union (e.g. `1 / [-1, 1]`, multi-root isolations) or weighted sample cloud.
    SetOrMultiset {
        intervals: Vec<UniversalBall>,
        weighted_points: Option<Vec<(f64, f64)>>,
    },
    /// Truncated asymptotic remainder (`|R_N(x)| <= C * |x|^N` on validity domain).
    Asymptotic {
        order: u32,
        constant_bound: Option<f64>,
    },
}

impl UncertaintyKind {
    /// Soundly combine two uncertainty descriptions under additive composition,
    /// promoting mixed worst-case + probabilistic spreads to a Chebyshev PAC bound.
    pub fn join_additive(&self, rhs: &Self) -> Self {
        match (self, rhs) {
            (Self::Exact, other) | (other, Self::Exact) => other.clone(),
            (Self::WorstCaseBall(a), Self::WorstCaseBall(b)) => Self::WorstCaseBall(a.add(b)),
            (
                Self::ProbabilisticSpread {
                    mean: m1,
                    std_dev: s1,
                    dist,
                    pac_delta: d1,
                },
                Self::ProbabilisticSpread {
                    mean: m2,
                    std_dev: s2,
                    pac_delta: d2,
                    ..
                },
            ) => Self::ProbabilisticSpread {
                mean: m1 + m2,
                std_dev: (s1 * s1 + s2 * s2).sqrt(),
                dist: *dist,
                pac_delta: match (d1, d2) {
                    (Some(a), Some(b)) => Some((a + b).min(1.0)),
                    (Some(a), None) | (None, Some(a)) => Some(*a),
                    (None, None) => None,
                },
            },
            (
                Self::WorstCaseBall(ball),
                Self::ProbabilisticSpread {
                    mean,
                    std_dev,
                    dist,
                    pac_delta,
                },
            )
            | (
                Self::ProbabilisticSpread {
                    mean,
                    std_dev,
                    dist,
                    pac_delta,
                },
                Self::WorstCaseBall(ball),
            ) => {
                let (re, _) = ball.center_complex();
                let r = ball.radius();
                let k = if *std_dev > 0.0 {
                    (r / std_dev).max(1.0)
                } else {
                    1.0
                };
                let chebyshev_delta = (1.0 / (k * k)).min(1.0);
                let combined_delta = pac_delta.map_or(chebyshev_delta, |d| d.max(chebyshev_delta));
                Self::ProbabilisticSpread {
                    mean: mean + re,
                    std_dev: *std_dev + r,
                    dist: *dist,
                    pac_delta: Some(combined_delta),
                }
            }
            (
                Self::SetOrMultiset {
                    intervals: i1,
                    weighted_points: w1,
                },
                Self::SetOrMultiset {
                    intervals: i2,
                    weighted_points: w2,
                },
            ) => {
                let mut intervals = i1.clone();
                intervals.extend(i2.iter().cloned());
                let weighted_points = match (w1, w2) {
                    (Some(a), Some(b)) => {
                        let mut v = a.clone();
                        v.extend(b.iter().copied());
                        Some(v)
                    }
                    (Some(a), None) | (None, Some(a)) => Some(a.clone()),
                    (None, None) => None,
                };
                Self::SetOrMultiset {
                    intervals,
                    weighted_points,
                }
            }
            (
                Self::SetOrMultiset {
                    intervals,
                    weighted_points,
                },
                Self::WorstCaseBall(b),
            )
            | (
                Self::WorstCaseBall(b),
                Self::SetOrMultiset {
                    intervals,
                    weighted_points,
                },
            ) => {
                let shifted = intervals.iter().map(|iv| iv.add(b)).collect();
                Self::SetOrMultiset {
                    intervals: shifted,
                    weighted_points: weighted_points.clone(),
                }
            }
            (
                Self::Asymptotic {
                    order: o1,
                    constant_bound: c1,
                },
                Self::Asymptotic {
                    order: o2,
                    constant_bound: c2,
                },
            ) => Self::Asymptotic {
                order: (*o1).min(*o2),
                constant_bound: match (c1, c2) {
                    (Some(a), Some(b)) => Some(a + b),
                    _ => None,
                },
            },
            (
                Self::Asymptotic {
                    order,
                    constant_bound,
                },
                _,
            )
            | (
                _,
                Self::Asymptotic {
                    order,
                    constant_bound,
                },
            ) => Self::Asymptotic {
                order: *order,
                constant_bound: *constant_bound,
            },
            (lhs, rhs) => Self::SetOrMultiset {
                intervals: vec![
                    UniversalBall::real_with_radius(0.0, lhs.worst_case_radius().unwrap_or(0.0)),
                    UniversalBall::real_with_radius(0.0, rhs.worst_case_radius().unwrap_or(0.0)),
                ],
                weighted_points: None,
            },
        }
    }

    /// Return a conservative scalar radius when bounded, or `None` for open/asymptotic sets.
    pub fn worst_case_radius(&self) -> Option<f64> {
        match self {
            Self::Exact => Some(0.0),
            Self::WorstCaseBall(b) => Some(b.radius()),
            Self::ProbabilisticSpread { std_dev, .. } => Some(3.0 * std_dev),
            Self::SetOrMultiset { intervals, .. } => {
                intervals.iter().map(UniversalBall::radius).reduce(f64::max)
            }
            Self::Asymptotic { .. } => None,
        }
    }
}

/// Complete error metadata attached to an evaluated result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorEnclosure {
    pub kind: UncertaintyKind,
    pub abs_err: Option<f64>,
    pub rel_err: Option<f64>,
    pub method: String,
}

impl Default for ErrorEnclosure {
    fn default() -> Self {
        Self {
            kind: UncertaintyKind::Exact,
            abs_err: Some(0.0),
            rel_err: Some(0.0),
            method: "exact".to_string(),
        }
    }
}

impl ErrorEnclosure {
    pub fn exact(method: impl Into<String>) -> Self {
        Self {
            kind: UncertaintyKind::Exact,
            abs_err: Some(0.0),
            rel_err: Some(0.0),
            method: method.into(),
        }
    }

    pub fn from_ball(ball: UniversalBall, method: impl Into<String>) -> Self {
        let r = ball.radius();
        Self {
            kind: UncertaintyKind::WorstCaseBall(ball),
            abs_err: Some(r),
            rel_err: None,
            method: method.into(),
        }
    }
}

/// Validity domain, operating point, and region-of-convergence metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DomainCondition {
    pub operating_point: Option<String>,
    pub assumptions: Vec<String>,
    pub roc_summary: Option<String>,
}

/// Standardized output envelope for multi-fidelity symbolic/numeric evaluation.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ApproxResult<T> {
    pub value: T,
    pub mode_used: FidelityMode,
    pub form: EvalForm,
    pub enclosure: ErrorEnclosure,
    pub remainder_term: Option<CatTerm>,
    pub validity: DomainCondition,
    pub budget_exceeded: bool,
}
