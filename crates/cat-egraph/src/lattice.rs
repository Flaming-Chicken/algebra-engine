//! Two-Tiered Equivalence Lattice: Strong Exact vs Weak Metric Approximation.

use cat_core::ball::UniversalBall;
use serde::{Deserialize, Serialize};

/// Classification of equivalence relations in the CAS engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EquivalenceKind {
    /// Strong Exact Equality (=_exact): mathematically sound, strictly guarded.
    /// E.g. x/x = 1 only if x is proven non-zero and invertible!
    StrongExact { is_invertible: bool },
    /// Weak Association (\approx_st): metric-enriched approximation edge with error tolerance \tau.
    WeakAssociation {
        tau_tolerance: f64,
        enclosure: UniversalBall,
    },
}

impl EquivalenceKind {
    /// Create strong exact equality.
    pub fn strong_exact(is_invertible: bool) -> Self {
        Self::StrongExact { is_invertible }
    }

    /// Create weak metric approximation edge.
    pub fn weak_association(tau: f64, enclosure: UniversalBall) -> Self {
        Self::WeakAssociation {
            tau_tolerance: tau,
            enclosure,
        }
    }

    /// Check if this edge qualifies as strong exact.
    pub fn is_exact(&self) -> bool {
        matches!(
            self,
            Self::StrongExact {
                is_invertible: true
            }
        )
    }
}

/// Directed or undirected equivalence edge in the e-graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquivalenceEdge {
    pub from_class: u32,
    pub to_class: u32,
    pub kind: EquivalenceKind,
    pub description: String,
}
