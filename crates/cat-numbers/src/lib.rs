//! `cat-numbers`: High-performance complex, hypercomplex, Clifford cocycle algebra,
//! Conway Surreal Numbers (\mathbf{No}), Adelic Ring (\mathbb{A}_\mathbb{Q}),
//! Butler-Portugal tensor canonicalization, Metric Geometry, and Exterior Differential Forms.

pub mod adele;
pub mod butler_portugal;
pub mod cocycle;
pub mod differential_forms;
pub mod lyndon;
pub mod magnitude;
pub mod metric;
pub mod pd_algebra;
pub mod surreal;
pub mod tensor;

pub use adele::{AdeleNumber, PadicNumber};
pub use butler_portugal::{ButlerPortugal, CanonicalTensorResult, TensorSymmetry};
pub use cocycle::CocycleEngine;
pub use differential_forms::DifferentialForm;
pub use lyndon::{LieBracketTree, LyndonWord};
pub use magnitude::Magnitude;
pub use metric::MetricTensor;
pub use pd_algebra::{lucas_binomial_mod_p, DividedPowerAtom};
pub use surreal::SurrealNumber;
pub use tensor::{IndexVariance, Tensor};
