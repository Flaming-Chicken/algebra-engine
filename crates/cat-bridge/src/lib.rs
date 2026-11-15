//! `cat-bridge`: Categorical Strangler-Fig Bridge ($L \dashv R$) for retrofitting legacy CAS libraries.
//!
//! Provides bidirectional lifting and lowering between legacy recursive tree ASTs
//! and the 3-tier `CatTerm` architecture, along with differential verification gates.

pub mod algebra_core_bridge;
pub mod context;
pub mod traits;
pub mod verifier;

pub use algebra_core_bridge::{BridgeError, LegacyAstTree};
pub use context::{LegacyBridgeContext, LegacyCompatibilityDomain};
pub use traits::{MigrationVerifyStatus, RetrofittableAst};
pub use verifier::ShadowVerifyHarness;
