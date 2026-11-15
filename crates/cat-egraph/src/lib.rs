//! `cat-egraph`: Two-Tiered Equivalence Lattice, speculative e-graph rewriting,
//! and Buckingham-Pi dimensional pre-reduction.

pub mod buckingham_pi;
pub mod egraph;
pub mod lattice;

pub use buckingham_pi::{BuckinghamPi, PiGroup};
pub use egraph::{EClass, EGraph, ENode, RewriteRule, RuleTier};
pub use lattice::{EquivalenceEdge, EquivalenceKind};
