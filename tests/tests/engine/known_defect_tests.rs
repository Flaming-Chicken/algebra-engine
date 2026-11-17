//! Known defects from `docs/PLANNING.md`, written as the behaviour that should hold.
//!
//! Each test is `#[ignore]`d because it currently fails; the ignore reason names the ledger id.
//! Fixing a defect means removing its `#[ignore]` in the same change. Run them with
//! `cargo test -p urae-tests --test engine_suite known_defect -- --ignored`.

use algebra_core::ExprGraph;
use algebra_core::format::{Formatter, UnicodeFormatter};
use algebra_engine::calculus::SymbolicCalculus;
use algebra_engine::series::RecurrenceSolver;
use algebra_engine::solver::SymbolicSolver;

fn show(g: &ExprGraph, e: algebra_core::ExprId) -> String {
    UnicodeFormatter
        .format(g, e)
        .unwrap_or_else(|err| format!("format error: {err:?}"))
}

#[test]
#[ignore = "B4: y' = 1 returns an expression containing 1 / 0"]
fn known_defect_linear_ode_with_zero_coefficient() {
    let g = ExprGraph::new();
    let x = g.symbols.get_or_intern("x");
    let out = g
        .solve_ode_linear(g.integer(0), g.integer(1), x)
        .map(|e| show(&g, e));
    if let Ok(text) = out {
        assert!(!text.contains("/ 0"), "division by zero in result: {text}");
    }
}

#[test]
#[ignore = "B1: sum of k^2 is treated as a constant and returns k^2 * n"]
fn known_defect_summation_of_a_non_constant_summand() {
    let g = ExprGraph::new();
    let k = g.symbols.get_or_intern("k");
    let square = g.pow(g.symbol("k"), g.integer(2));
    let out = g
        .summation(square, k, g.integer(1), g.symbol("n"))
        .map(|e| show(&g, e));
    // Either an honest error or a closed form; never `k^2` pulled out as a constant.
    if let Ok(text) = out {
        assert!(
            !text.starts_with("k^2"),
            "summand treated as constant: {text}"
        );
    }
}

#[test]
#[ignore = "B6: the Fibonacci recurrence a(n) = a(n-1) + a(n-2) returns 0"]
fn known_defect_second_order_recurrence() {
    let g = ExprGraph::new();
    let n = g.symbols.get_or_intern("n");
    let out = RecurrenceSolver::solve_linear_constant(&g, &[1.0, 1.0], &[0.0, 1.0], n)
        .map(|e| show(&g, e));
    if let Ok(text) = out {
        assert_ne!(text, "0", "Fibonacci closed form collapsed to zero");
    }
}
