//! Regression tests for the cases the 16 Nov portfolio audit found still wrong
//! after the first round of URAE fixes: limits at infinity through radicals and
//! variable exponents, two-sided limits through `abs`, and the primality label on
//! numbers Pollard rho could not split.

use algebra_core::ExprGraph;
use algebra_core::parser::parse_operation;
use algebra_engine::executor::{ExecutionContext, OperationExecutor, OperationResult};

fn run(cmd: &str) -> OperationResult {
    let graph = ExprGraph::new();
    let ctx = ExecutionContext::new();
    OperationExecutor::execute(&graph, &parse_operation(cmd), &ctx)
}

/// The value after the last `= ` of a result line such as `lim_{x -> 0} (f) = 1`.
fn value(cmd: &str) -> String {
    let res = run(cmd);
    assert!(!res.is_error, "`{cmd}` failed: {:?}", res.error_msg);
    res.output_text
        .rsplit("= ")
        .next()
        .unwrap_or_default()
        .trim()
        .to_string()
}

#[test]
fn limits_at_infinity_through_radicals() {
    let cases = [
        // ∞ - ∞ resolved by rationalising.
        ("lim sqrt(x^2 + x) - x as x->oo", "1/2"),
        ("lim sqrt(x^2 + 3*x) - sqrt(x^2 + x) as x->oo", "1"),
        ("lim sqrt(x + 1) - sqrt(x) as x->oo", "0"),
        // Dominant term of a quotient with a radical.
        ("lim sqrt(4*x^2 + x)/x as x->oo", "2"),
        // Genuinely divergent differences stay divergent.
        ("lim x - sqrt(x) as x->oo", "∞"),
        ("lim sqrt(x) as x->oo", "∞"),
        ("lim 1/sqrt(x) as x->oo", "0"),
    ];
    for (cmd, expected) in cases {
        assert_eq!(value(cmd), expected, "{cmd}");
    }
}

#[test]
fn limits_at_infinity_with_a_variable_exponent() {
    let cases = [
        ("lim x^(1/x) as x->oo", "1"),
        ("lim (2*x)^(1/x) as x->oo", "1"),
        ("lim x^x as x->oo", "∞"),
        ("lim (1/x)^x as x->oo", "0"),
        // The existing 1^∞ path is untouched.
        ("lim (1 + 1/x)^x as x->oo", "e"),
        ("lim (1 + 2/x)^x as x->oo", "e^2"),
    ];
    for (cmd, expected) in cases {
        assert_eq!(value(cmd), expected, "{cmd}");
    }
}

#[test]
fn two_sided_limit_through_abs_requires_agreeing_sides() {
    // x / |x| is -1 from the left and +1 from the right at 0.
    assert_eq!(value("lim x/abs(x) as x->0"), "Undefined");
    assert_eq!(value("lim abs(x - 1)/(x - 1) as x->1"), "Undefined");
    // Away from the kink the two sides agree, and |x| is continuous at 0.
    assert_eq!(value("lim abs(x)/x as x->2"), "1");
    assert_eq!(value("lim abs(x) as x->0"), "0");
}

#[test]
fn unchanged_limits_still_hold() {
    let cases = [
        ("lim sin(x)/x as x->0", "1"),
        ("lim 1/x as x->0", "Undefined"),
        ("lim exp(-x) as x->oo", "0"),
        ("lim (3*x^3 - x)/(x^3 + 7) as x->oo", "3"),
        ("lim (x^2 + 1)/(2*x^2 - 3) as x->oo", "1/2"),
    ];
    for (cmd, expected) in cases {
        assert_eq!(value(cmd), expected, "{cmd}");
    }
}

#[test]
fn factorisation_labels_nothing_when_every_factor_is_prime() {
    let res = run("factor_primes 1000000016000000063");
    assert!(!res.is_error);
    assert!(res.output_text.contains("1000000007 * 1000000009"));
    assert!(!res.output_text.contains("not factored"));
}

/// 2^128 + 1 is composite (59649589127497217 · 5704689200685129054721) but
/// beyond the Pollard rho budget, so it must not be presented as a prime.
/// Slow: the budget is exhausted before giving up.
#[test]
#[ignore = "takes about a minute while Pollard rho exhausts its budget"]
fn unfactored_composite_is_not_reported_as_prime() {
    let res = run("factor_primes 340282366920938463463374607431768211457");
    assert!(!res.is_error);
    assert!(
        res.output_text.contains("composite, not factored"),
        "{}",
        res.output_text
    );
}
