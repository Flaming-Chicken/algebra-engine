//! Workspace Integration Test Suite for `algebra-advanced`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "advanced/adele_tests.rs"]
mod adele_tests;

#[path = "advanced/agent_agent_tests.rs"]
mod agent_agent_tests;

#[path = "advanced/codegen_codegen_tests.rs"]
mod codegen_codegen_tests;

#[path = "advanced/crypto_crypto_tests.rs"]
mod crypto_crypto_tests;

#[path = "advanced/finite_finite_tests.rs"]
mod finite_finite_tests;

#[path = "advanced/galois_and_statmech_tests.rs"]
mod galois_and_statmech_tests;

#[path = "advanced/gpu_gpu_tests.rs"]
mod gpu_gpu_tests;

#[path = "advanced/knot_tests.rs"]
mod knot_tests;

#[path = "advanced/morphism_morphism_tests.rs"]
mod morphism_morphism_tests;

#[path = "advanced/padic_tests.rs"]
mod padic_tests;

#[path = "advanced/phase6_clifford_tests.rs"]
mod phase6_clifford_tests;

#[path = "advanced/proof_proof_tests.rs"]
mod proof_proof_tests;

#[path = "advanced/quantum_quantum_tests.rs"]
mod quantum_quantum_tests;

#[path = "advanced/stochastic_stochastic_tests.rs"]
mod stochastic_stochastic_tests;

#[path = "advanced/structure_tensor_tests.rs"]
mod structure_tensor_tests;

#[path = "advanced/surreal_tests.rs"]
mod surreal_tests;

#[path = "advanced/transcendents_tests.rs"]
mod transcendents_tests;
