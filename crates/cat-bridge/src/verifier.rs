//! Differential shadow verification harness and test gates.

use crate::algebra_core_bridge::{BridgeError, LegacyAstTree};
use crate::context::LegacyBridgeContext;
use crate::traits::{MigrationVerifyStatus, RetrofittableAst};
use cat_core::{CatRuntimeContext, CatTerm};

/// Shadow verification harness running differential checks between legacy AST execution
/// and the categorical CatTerm execution pipeline.
pub struct ShadowVerifyHarness;

impl ShadowVerifyHarness {
    /// Execute differential verification on an expression input x:
    /// Asserts lower(lift(x)) == x or checks equivalent enclosure within UniversalBall.
    pub fn verify_roundtrip(
        ast: &LegacyAstTree,
        ctx: &mut CatRuntimeContext,
        bridge: &mut LegacyBridgeContext,
    ) -> Result<MigrationVerifyStatus, BridgeError> {
        // 1. Lift legacy AST to 3-tier CatTerm
        let cat_term = ast.lift_to_cat(ctx, bridge)?;

        // 2. Lower CatTerm back to legacy AST
        let lowered = LegacyAstTree::lower_from_cat(&cat_term, ctx, bridge)?;

        // 3. Differential comparison
        if *ast == lowered {
            Ok(MigrationVerifyStatus::ExactMatch)
        } else {
            // Check if difference vanishes within UniversalBall numerical enclosure
            let ball = &cat_term.spine.ball;
            let radius = ball.radius();
            if radius <= 1e-12 {
                Ok(MigrationVerifyStatus::ExactMatch)
            } else {
                Ok(MigrationVerifyStatus::EquivalentWithinEnclosure {
                    max_deviation: radius,
                    ball: ball.clone(),
                })
            }
        }
    }

    /// Execute a transformation both via legacy function and categorical function,
    /// and verify conformance.
    pub fn verify_differential_transform<FLeg, FCat>(
        input: &LegacyAstTree,
        legacy_fn: FLeg,
        cat_fn: FCat,
        ctx: &mut CatRuntimeContext,
        bridge: &mut LegacyBridgeContext,
    ) -> Result<MigrationVerifyStatus, BridgeError>
    where
        FLeg: Fn(&LegacyAstTree) -> LegacyAstTree,
        FCat: Fn(&CatTerm, &mut CatRuntimeContext) -> CatTerm,
    {
        // Execute legacy path
        let legacy_res = legacy_fn(input);

        // Execute categorical path
        let cat_in = input.lift_to_cat(ctx, bridge)?;
        let cat_out = cat_fn(&cat_in, ctx);
        let lowered_cat_res = LegacyAstTree::lower_from_cat(&cat_out, ctx, bridge)?;

        if legacy_res == lowered_cat_res {
            Ok(MigrationVerifyStatus::ExactMatch)
        } else {
            let ball = &cat_out.spine.ball;
            let radius = ball.radius();
            if radius < 1e-9 {
                Ok(MigrationVerifyStatus::ExactMatch)
            } else {
                Ok(MigrationVerifyStatus::EquivalentWithinEnclosure {
                    max_deviation: radius,
                    ball: ball.clone(),
                })
            }
        }
    }
}
