//! Workspace Integration Test Suite for `algebra-core`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "core/domain_tests.rs"]
mod domain_tests;

#[path = "core/format_format_tests.rs"]
mod format_format_tests;

#[path = "core/graph_tests.rs"]
mod graph_tests;

#[path = "core/interval_tests.rs"]
mod interval_tests;

#[path = "core/number_tests.rs"]
mod number_tests;

#[path = "core/numbers_number_tests.rs"]
mod numbers_number_tests;

#[path = "core/operation_tests.rs"]
mod operation_tests;

#[path = "core/parallel_concurrency.rs"]
mod parallel_concurrency;

#[path = "core/parallel_tests.rs"]
mod parallel_tests;

#[path = "core/parser_parser_tests.rs"]
mod parser_parser_tests;

#[path = "core/symbol_tests.rs"]
mod symbol_tests;

#[path = "core/const_environment_tests.rs"]
mod const_environment_tests;
