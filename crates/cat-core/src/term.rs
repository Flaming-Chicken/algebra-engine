//! Three-tier `CatTerm` categorical expression structure.
//!
//! - Layer 0: ObjectDescriptor (`ObjId`)
//! - Layer 1: EnrichedSpine (`ScalarId` + `UniversalBall` enclosure)
//! - Layer 2: OpenHypergraph (`DiagramId`)

use crate::ball::UniversalBall;
use crate::handles::{DiagramId, ObjId, ScalarId};
use serde::{Deserialize, Serialize};

/// Categorical enrichment category for the linear spine.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum EnrichmentKind {
    /// Standard vector space / R-module enrichment.
    #[default]
    LinearModule,
    /// Affine space (translations allowed, point sums disallowed).
    AffineSpace,
    /// Convex space (convex combinations \sum \lambda_i x_i = 1 allowed).
    ConvexSpace,
}

/// Layer 1: Enriched spine separating scalar factor, enrichment kind, and error ball enclosure.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct EnrichedSpine {
    /// Central commutative or base scalar coefficient.
    pub scalar: ScalarId,
    /// Mathematical enrichment category.
    pub enrichment: EnrichmentKind,
    /// Rigorous numerical enclosure (complex disk, real interval, or Lie manifold ball).
    pub ball: UniversalBall,
}

impl Default for EnrichedSpine {
    fn default() -> Self {
        Self {
            scalar: ScalarId(0),
            enrichment: EnrichmentKind::LinearModule,
            ball: UniversalBall::ZERO,
        }
    }
}

impl EnrichedSpine {
    /// Create a new spine with a scalar handle and ball.
    pub fn new(scalar: ScalarId, ball: UniversalBall) -> Self {
        Self {
            scalar,
            enrichment: EnrichmentKind::LinearModule,
            ball,
        }
    }

    /// Create an exact scalar spine.
    pub fn exact(scalar: ScalarId, atom: crate::scalar::ScalarAtom) -> Self {
        Self {
            scalar,
            enrichment: EnrichmentKind::LinearModule,
            ball: UniversalBall::ExactScalar(atom),
        }
    }
}

/// Unified 3-Tier Categorical Term (`CatTerm`).
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct CatTerm {
    /// Layer 0: Sort and dimensional typing descriptor in ObjArena.
    pub sort: ObjId,
    /// Layer 1: Enriched spine with scalar coefficient and error enclosure.
    pub spine: EnrichedSpine,
    /// Layer 2: Open hypergraph diagram with anonymous internal wire connections.
    pub diagram: DiagramId,
}

impl CatTerm {
    /// Construct a new CatTerm.
    pub fn new(sort: ObjId, spine: EnrichedSpine, diagram: DiagramId) -> Self {
        Self {
            sort,
            spine,
            diagram,
        }
    }

    /// Check if this term's enclosure contains zero.
    pub fn is_zero(&self) -> bool {
        self.spine.ball.contains_zero()
    }
}
