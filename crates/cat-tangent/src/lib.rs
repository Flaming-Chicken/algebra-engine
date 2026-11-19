//! `cat-tangent`: Unified jet, stochastic (Itô), distributional (Colombeau),
//! and discrete time-scale (Hilger) calculi over categorical open hypergraphs.

pub mod colombeau;
pub mod functor;
pub mod weil_ideal;

pub use colombeau::GeneralizedDistribution;
pub use functor::{TangentError, TangentFunctor};
pub use weil_ideal::{CalculusKind, WeilJet};
