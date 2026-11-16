//! `cat-anytime`: Sub-frame anytime progressive evaluation engine, wavefront parallelism,
//! and algebraic zero-testing.

pub mod amitsur_levitzki;
pub mod scheduler;
pub mod se3_rotor;
pub mod wavefront;

pub use amitsur_levitzki::{AmitsurLevitzki, Matrix2Fp};
pub use scheduler::{
    AnytimeBudget, AnytimeHandle, FidelityStream, MultiFidelityOp, tier0_ball_result,
};
pub use se3_rotor::Se3Rotor;
pub use wavefront::{WavefrontLevel, WavefrontSchedule};
