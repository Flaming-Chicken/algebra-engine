//! Strangler-Fig migration traits and verification status enums.

use crate::context::LegacyBridgeContext;
use cat_core::{CatRuntimeContext, CatTerm, UniversalBall};

/// Implemented on the existing library's AST types to enable
/// incremental, module-by-module strangler-fig migration.
pub trait RetrofittableAst: Sized + Clone {
    type LegacyError: std::fmt::Debug;

    /// Lifts a legacy expression tree into the 3-tier `CatTerm` representation:
    /// 1. Infers Layer 0 `ObjId` and `DimVector`.
    /// 2. Erases dummy index names into anonymous Layer 2 `WireId` links.
    /// 3. Factors commutative scalars onto Layer 1 `CatTerm::spine`.
    fn lift_to_cat(
        &self,
        ctx: &mut CatRuntimeContext,
        bridge: &mut LegacyBridgeContext,
    ) -> Result<CatTerm, Self::LegacyError>;

    /// Lowers a canonicalized `CatTerm` back into the legacy AST so unmigrated
    /// consumers in the existing codebase continue functioning without modification.
    fn lower_from_cat(
        term: &CatTerm,
        ctx: &CatRuntimeContext,
        bridge: &LegacyBridgeContext,
    ) -> Result<Self, Self::LegacyError>;
}

/// Differential verification outcome used by Antigravity test gates during migration.
#[derive(Debug, Clone, PartialEq)]
pub enum MigrationVerifyStatus {
    /// Legacy output and lowered `CatTerm` output are syntactically identical after canonicalization.
    ExactMatch,
    /// Legacy output and `CatTerm` differ syntactically, but their difference vanishes
    /// inside the Two-Tiered Equivalence Lattice or lies within `UniversalBall` tolerance.
    EquivalentWithinEnclosure {
        max_deviation: f64,
        ball: UniversalBall,
    },
    /// The new engine caught a bug in the legacy library (e.g., invalid branch cut,
    /// missing Itô 1/2 dt drift, or illegal affine sum of Christoffel symbols / Celsius).
    LegacyBugDetected { explanation: &'static str },
    /// Genuine regression in the new pass — fails the Antigravity agent task gate!
    RegressionMismatch { detail: String },
}
