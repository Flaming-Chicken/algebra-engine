//! `cat-core` v2.1: Categorical symbolic computation engine.
//!
//! Provides the foundational 3-tier term architecture:
//! - Layer 0: Categorical sorts and physical dimensions ([`ObjectDescriptor`], [`DimVector`])
//! - Layer 1: Enriched linear spine and error enclosures ([`EnrichedSpine`], [`UniversalBall`], [`ScalarAtom`])
//! - Layer 2: Open hypergraphs with anonymous internal wires ([`OpenHypergraph`], [`HyperEdge`], [`HyperOp`])
//!
//! All handles are strictly `Copy + 'static + Send + Sync` integer identifiers without lifetime annotations.

pub mod ball;
pub mod context;
pub mod descriptor;
pub mod dimensions;
pub mod handles;
pub mod hypergraph;
pub mod scalar;
pub mod term;

pub use ball::{LieGroupKind, UniversalBall};
pub use context::{
    CatRuntimeContext, HyperArena, ObjArena, ScalarArena, SymDimArena, set_ambient_context,
    with_ambient_context,
};
pub use descriptor::ObjectDescriptor;
pub use dimensions::{DimVector, UnitGaugeSystem, UnitSpaceKind};
pub use handles::{DiagramId, ObjId, PortId, ScalarId, SymDimId, WireId};
pub use hypergraph::{BoundaryPort, HyperEdge, HyperOp, OpenHypergraph, PortPolarity};
pub use scalar::{AlgebraKind, CocycleMultivector, ScalarAtom, ScalarLattice};
pub use term::{CatTerm, EnrichedSpine, EnrichmentKind};
