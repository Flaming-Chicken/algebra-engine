//! # `algebra_engine::pde`
//!
//! Partial Differential Equation (PDE) computational subsystem for URAE.
//!
//! Encompasses:
//! - **Symbolic Solvers**: Linear 2nd-order classification, d'Alembert wave solutions, separation of variables, Heat Green's kernel.
//! - **Numerical Solvers**: Method of Lines (MOL), Finite Difference Method (FDM) wave propagation, 1D Finite Element Method (FEM) Galerkin.

pub mod fea;
pub mod numerical;
pub mod reduction;
pub mod symbolic;

pub use fea::{
    CsrMatrix, ElementType, FeaBoundaryCondition, FeaEngine, FeaMesh, FeaSolution,
    PhysicsDiscipline,
};
pub use numerical::{BoundaryCondition, NumericalPdeSolver, PdeGridSolution};
pub use reduction::PdeTravelingWaveReduction;
pub use symbolic::{PdeClassification, SymbolicPdeSolver};
