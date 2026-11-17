//! # `urae` — Universal Rust Algebra Engine
//!
//! Master umbrella facade crate for the **Universal Rust Algebra Engine (URAE)**: a high-performance,
//! arena-allocated, parallelized Computer Algebra System (CAS) in Rust.
//!
//! ---
//!
//! ## Architecture Overview
//!
//! URAE is organized into cohesive, layered crates under `crates/`:
//!
//! - **`algebra-core`** ([`core`]): Memory-safe arena DAG storage (`ExprGraph`), symbols, interval arithmetic, multi-notation parser, operation AST (`MathOperation`), and 2D Unicode / LaTeX formatters.
//! - **`algebra-engine`** ([`engine`]): Core CAS computation — E-Graph simplification, equation solving, symbolic calculus, polynomials, linear algebra, tensors, physics units, combinatorics, topology, control theory, number theory, esoteric operators (Knuth up-arrows, tropical semirings), and centralized operation executor (`OperationExecutor`).
//! - **`algebra-advanced`** ([`advanced`]): Advanced extensions — formal proofs (Lean 4 / Coq export), category morphisms, quantum mechanics, cryptography, Galois theory, statistical mechanics, and AI copilot.
//!
//! ## Quickstart
//!
//! ```rust
//! use urae::prelude::*;
//!
//! let graph = ExprGraph::new();
//! let x = graph.symbol("x");
//! let x_sym = graph.symbols.get_or_intern("x");
//! let parser = ExprParser::new(&graph);
//! let expr = parser.parse("x^2 + 0").unwrap();
//!
//! // Symbolic differentiation d/dx and E-Graph simplification
//! let diff = graph.diff(expr, x_sym);
//! let simplified = Simplifier::simplify(&graph, expr).unwrap();
//! let latex = LatexFormatter.format(&graph, simplified).unwrap();
//! assert_eq!(latex, "{x}^{2}");
//! ```

pub mod prelude;

pub use algebra_core as core;
pub use algebra_engine as engine;
pub use auto_promoters;
pub use cat_numbers::Magnitude;

// Re-export all CAS DSL macros from algebra_engine
pub use algebra_engine::{
    airfoil, albert_det, almkvist_zeilberger_gaussian, anticommutator, autodiff_forward,
    autodiff_hessian, block_feedback, block_parallel, block_series, certify_zero, char_set,
    clebsch_gordan, clifford_layer_forward, clifford_sandwich, commutator, cox_de_boor_basis, curl,
    diff, diophantine, dirac_slashed_trace, dirac_trace, discrete_laplacian_0, div, domain_color,
    dopri5, e8_weyl_reflect, ext_diff, fea_elasticity, fea_heat, fea_solve, fontaine_module,
    fourier, gear, grad, groebner, hessian, hodge, hyperbolic_dist, hyperbolic_triangle_area,
    iga_stiffness_entry_2d, integrate, jacobian, kac_conformal_weight, kac_moody_bracket, laplace,
    laplacian, laurent, limit, metric_curvature, neural_ode_step, octonion, octonion_associator,
    octonion_mul, ode_linear, pade, parke_taylor_mhv, passarino_veltman_b1, pde_heat_1d,
    persistence_diagram, pinn_burgers_residual, prob_test, proof_certificate, puiseux,
    quantifier_elim, quantum_bell_state, quantum_ghz_state, radau5, risch, rotor, screw,
    simplify_certified, simulate_cad, smt_assert, smt_problem, solve, solve_certified,
    spherical_dist, spherical_harmonic, spherical_triangle_area, spinor_bracket_angle,
    spinor_bracket_square, spring, streamline_2d, sugawara_central_charge, symplectic_integrate,
    tate_twist_frob, tate_twist_weights, taylor, transfer_fn, verlet, vietoris_rips,
    virasoro_bracket, weyl_commute, weyl_d, weyl_x, wgsl_grid, zeilberger_binomial_proof,
};

#[cfg(feature = "advanced")]
pub use algebra_advanced as advanced;

// Convenient direct module accessors
pub use algebra_core::format;
pub use algebra_core::interval;
pub use algebra_core::numbers;
pub use algebra_core::operation;
pub use algebra_core::parser;

pub use algebra_engine::api;
pub use algebra_engine::autodiff;
pub use algebra_engine::cad;
pub use algebra_engine::calculus;
pub use algebra_engine::cft;
pub use algebra_engine::combinatorics;
pub use algebra_engine::control;
pub use algebra_engine::curvature;
pub use algebra_engine::diffalg;
pub use algebra_engine::distributions;
pub use algebra_engine::exceptional_lie;
pub use algebra_engine::executor;
pub use algebra_engine::geometric_dl;
pub use algebra_engine::geometry;
pub use algebra_engine::hecs;
pub use algebra_engine::heuristic;
pub use algebra_engine::html_export;
pub use algebra_engine::hyperop;
pub use algebra_engine::iga;
pub use algebra_engine::interconnect;
pub use algebra_engine::logic;
pub use algebra_engine::matrix;
pub use algebra_engine::neural_ode;
pub use algebra_engine::noneuclidean;
pub use algebra_engine::numbertheory;
pub use algebra_engine::numeric;
pub use algebra_engine::padic_hodge;
pub use algebra_engine::pde;
pub use algebra_engine::physics;
pub use algebra_engine::pinn;
pub use algebra_engine::pipeline;
pub use algebra_engine::poly;
pub use algebra_engine::qft;
pub use algebra_engine::quantum_circuit;
pub use algebra_engine::series;
pub use algebra_engine::session;
pub use algebra_engine::sets;
pub use algebra_engine::simplify;
pub use algebra_engine::smt;
pub use algebra_engine::solver;
pub use algebra_engine::sparse;
pub use algebra_engine::special;
pub use algebra_engine::stream;
pub use algebra_engine::symplectic;
pub use algebra_engine::tda;
pub use algebra_engine::tensor;
pub use algebra_engine::topology;
pub use algebra_engine::transforms;
pub use algebra_engine::tropical;
pub use algebra_engine::urscript_pose;
pub use algebra_engine::visualizer;
pub use algebra_engine::weyl_dmodules;
pub use algebra_engine::workbench;

#[cfg(feature = "advanced")]
pub use algebra_advanced::{
    agent, codegen, crypto, finite, galois, gpu, morphism, proof, quantum, statmech, stochastic,
};

#[cfg(feature = "cli")]
pub use urae_cli as cli;

pub use prelude::*;
