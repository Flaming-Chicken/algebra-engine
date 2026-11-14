//! Lifetime-free arena handles for the `cat-core` categorical symbolic engine.
//!
//! Rule 1 Invariant: Every handle is strictly `Copy + 'static + Send + Sync`.
//! No struct or trait in the engine carries lifetime parameters, preventing
//! agentic borrow-checker fighting and enabling zero-overhead concurrency.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Handle referencing a categorical sort / object descriptor in [`crate::context::ObjArena`].
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct ObjId(pub u32);

impl fmt::Debug for ObjId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ObjId({})", self.0)
    }
}

impl fmt::Display for ObjId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Obj#{}", self.0)
    }
}

/// Handle referencing a scalar coefficient or atom in [`crate::context::ScalarArena`].
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct ScalarId(pub u32);

impl fmt::Debug for ScalarId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ScalarId({})", self.0)
    }
}

impl fmt::Display for ScalarId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Scalar#{}", self.0)
    }
}

/// Handle referencing an open hypergraph diagram in [`crate::context::HyperArena`].
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct DiagramId(pub u32);

impl fmt::Debug for DiagramId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DiagramId({})", self.0)
    }
}

impl fmt::Display for DiagramId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Diagram#{}", self.0)
    }
}

/// 16-bit handle for an internal anonymous wire connecting hyperedges in an [`crate::hypergraph::OpenHypergraph`].
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct WireId(pub u16);

impl fmt::Debug for WireId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WireId({})", self.0)
    }
}

impl fmt::Display for WireId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "w{}", self.0)
    }
}

/// 16-bit handle for a boundary port on an open hypergraph diagram.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct PortId(pub u16);

impl fmt::Debug for PortId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PortId({})", self.0)
    }
}

impl fmt::Display for PortId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "p{}", self.0)
    }
}

/// Handle referencing a symbolic dimension parameter for parametric Buckingham-Pi reduction.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct SymDimId(pub u32);

impl fmt::Debug for SymDimId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SymDimId({})", self.0)
    }
}

impl fmt::Display for SymDimId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SymDim#{}", self.0)
    }
}
