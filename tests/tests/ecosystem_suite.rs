//! Workspace Integration Test Suite for URAE Ecosystem: Python Bindings, Jupyter Kernel, LSP Tooling, CLI, FFI, Facade & System Consolidation.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "ecosystem/phase18_python_kernel_tests.rs"]
mod phase18_python_kernel_tests;

#[path = "ecosystem/phase19_lsp_tooling_tests.rs"]
mod phase19_lsp_tooling_tests;

#[path = "cli/cli_tests.rs"]
mod cli_tests;

#[path = "ffi/ffi_tests.rs"]
mod ffi_tests;

#[path = "facade/facade_tests.rs"]
mod facade_tests;

#[path = "facade/phase11_dsl_macros_tests.rs"]
mod phase11_dsl_macros_tests;

#[path = "consolidation/consolidation_tests.rs"]
mod consolidation_tests;
