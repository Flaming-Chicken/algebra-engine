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
        ("lim (1 + 2/x)^x as x->oo", "exp(2)"),
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

// --- Integration by parts for higher-degree polynomial factors ---------------

use algebra_core::parser::ExprParser;
use algebra_engine::calculus::SymbolicCalculus;
use algebra_engine::numeric::{EvalContext, NumericalEval};
use algebra_engine::solver::SymbolicSolver;
use std::collections::HashMap;

fn eval_at(graph: &ExprGraph, expr: algebra_core::ExprId, x: f64) -> f64 {
    let mut bindings = HashMap::new();
    bindings.insert("x".to_string(), x);
    graph
        .evalf(expr, &EvalContext::with_bindings(15, bindings))
        .unwrap()
        .to_f64()
}

/// The derivative of the antiderivative must equal the integrand.
#[test]
fn power_times_elementary_integrals_differentiate_back() {
    let integrands = [
        "x^3 * exp(x)",
        "x^4 * exp(x)",
        "x^6 * exp(x)",
        "x^3 * sin(x)",
        "x^4 * sin(x)",
        "x^5 * sin(x)",
        "x^3 * cos(x)",
        "x^4 * cos(x)",
        "x^2 * ln(x)",
        "x^3 * ln(x)",
        "x^7 * ln(x)",
    ];
    for src in integrands {
        let graph = ExprGraph::new();
        let x_sym = graph.symbols.get_or_intern("x");
        let integrand = ExprParser::new(&graph).parse(src).unwrap();
        let antiderivative = graph
            .integrate(integrand, x_sym)
            .unwrap_or_else(|e| panic!("`{src}` did not integrate: {e}"));
        let back = graph.diff(antiderivative, x_sym);
        for point in [0.7, 1.3, 2.1] {
            let want = eval_at(&graph, integrand, point);
            let got = eval_at(&graph, back, point);
            assert!(
                (want - got).abs() <= 1e-8 * want.abs().max(1.0),
                "d/dx of the antiderivative of `{src}` at {point}: {got}, expected {want}"
            );
        }
    }
}

// --- Equations in e^x ---------------------------------------------------------

fn complex_roots(src: &str) -> Vec<(f64, f64)> {
    let graph = ExprGraph::new();
    let x_sym = graph.symbols.get_or_intern("x");
    let eq = ExprParser::new(&graph).parse(src).unwrap();
    let mut roots: Vec<(f64, f64)> = graph
        .solveset(eq, x_sym)
        .unwrap()
        .iter()
        .map(|&r| {
            graph
                .evalf(r, &EvalContext::default())
                .unwrap()
                .as_complex()
        })
        .collect();
    roots.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    roots
}

#[test]
fn exp_quadratic_gives_clean_principal_roots() {
    // u = e^x and u^2 = 7: u = ±sqrt(7), so x = ln(7)/2 and ln(7)/2 + i*pi.
    // The second used to print as the unevaluated ln(-sqrt(7)).
    assert_eq!(
        run("solve exp(2*x) = 7, x")
            .output_text
            .trim()
            .chars()
            .take(31)
            .collect::<String>(),
        "Solutions for x: [1/2 · ln(7), "
    );
    let half_ln7 = 7f64.ln() / 2.0;
    let roots = complex_roots("exp(2*x) - 7");
    assert_eq!(roots.len(), 2);
    assert!((roots[0].0 - half_ln7).abs() < 1e-12 && roots[0].1.abs() < 1e-12);
    assert!(
        (roots[1].0 - half_ln7).abs() < 1e-12 && (roots[1].1 - std::f64::consts::PI).abs() < 1e-12
    );

    // Both roots positive: (u - 1)(u - 2) = 0 gives 0 and ln 2, both real.
    let roots = complex_roots("exp(2*x) - 3*exp(x) + 2");
    assert_eq!(roots.len(), 2);
    assert!(roots[0].0.abs() < 1e-12 && (roots[1].0 - 2f64.ln()).abs() < 1e-12);
    assert!(roots.iter().all(|r| r.1.abs() < 1e-12));
}

// --- Products, radicals and repeated factors -------------------------------

#[test]
fn simplify_merges_repeated_bases_and_square_roots() {
    let cases = [
        ("simplify x*x", "x^2"),
        ("simplify x*x*x", "x^3"),
        ("simplify x^2*x^3", "x^5"),
        ("simplify x*x^(-1)", "1"),
        ("simplify 2*x*3*x", "6 · x^2"),
        ("simplify sqrt(2)*sqrt(8)", "4"),
        ("simplify sqrt(3)*sqrt(12)", "6"),
        ("simplify sqrt(8)*sqrt(2)*x", "4 · x"),
        // Not a perfect square: combined, not expanded.
        ("simplify sqrt(2)*sqrt(3)", "√(6)"),
        // Distinct bases are left alone and keep their order.
        ("simplify x*y", "x · y"),
    ];
    for (cmd, expected) in cases {
        assert_eq!(value(cmd), expected, "{cmd}");
    }
}

#[test]
fn symbolic_determinant_collects_repeated_factors() {
    assert_eq!(value("det [[x,1],[1,x]]"), "x^2 - 1");
    assert_eq!(value("det [[a,b],[c,d]]"), "a · d - b · c");
}

// --- 1/(x^2 - b^2) integrates to a logarithm --------------------------------

#[test]
fn quadratic_with_real_roots_integrates_to_logs() {
    // Points are away from the poles, where the logarithm's argument is positive.
    let cases = [
        ("1/(x^2 - 4)", vec![3.0, 4.5, -3.5]),
        ("3/(x^2 - 9)", vec![4.0, 6.0, -5.0]),
        ("1/(x^2 - 2)", vec![2.0, 3.0, -2.5]),
        ("2/(x^2 - 25)", vec![6.0, 9.0, -7.0]),
    ];
    for (src, points) in cases {
        let graph = ExprGraph::new();
        let x_sym = graph.symbols.get_or_intern("x");
        let integrand = ExprParser::new(&graph).parse(src).unwrap();
        let antiderivative = graph
            .integrate(integrand, x_sym)
            .unwrap_or_else(|e| panic!("`{src}` did not integrate: {e}"));
        let text = value(&format!("integrate {src}, x"));
        assert!(
            text.contains("ln") && !text.contains("atan"),
            "{src}: {text}"
        );
        let back = graph.diff(antiderivative, x_sym);
        for point in points {
            let want = eval_at(&graph, integrand, point);
            let got = eval_at(&graph, back, point);
            assert!(
                (want - got).abs() <= 1e-9 * want.abs().max(1.0),
                "d/dx of the antiderivative of `{src}` at {point}: {got}, expected {want}"
            );
        }
    }
    // Positive constant is still an arctan (printed with the canonical name `atan`).
    assert!(value("integrate 1/(x^2 + 4), x").contains("atan"));
}

// --- Difference of squares with multi-term roots ----------------------------

#[test]
fn factor_difference_of_squares_with_multi_term_roots() {
    let cases = [
        (
            "factor a^2 + 2*a*b + b^2 - c^2",
            "(a + b - c) · (a + b + c)",
        ),
        ("factor x^2 + 2*x*y + y^2 - 9", "(x + y - 3) · (x + y + 3)"),
        (
            "factor 4*a^2 - (b + c)^2",
            "(2 · a - b - c) · (2 · a + b + c)",
        ),
        ("factor x^4 - y^4", "(x - y) · (x + y) · (x^2 + y^2)"),
        ("factor x^2 - y^2", "(x - y) · (x + y)"),
        // Not a difference of squares: unchanged.
        ("factor a^2 + b^2 - c^2", "a^2 + b^2 - c^2"),
    ];
    for (cmd, expected) in cases {
        assert_eq!(value(cmd), expected, "{cmd}");
    }
}

// --- Identities after clearing denominators ---------------------------------

#[test]
fn equation_that_is_an_identity_on_its_domain_is_not_reported_as_unsolvable() {
    // Holds for every x where it is defined; reported like `x = x`, as the variable.
    assert_eq!(
        value("solve (x - 1)/(x - 1) = 1, x"),
        "Solutions for x: [x]"
    );
    assert_eq!(value("solve x/x = 1, x"), "Solutions for x: [x]");
    // The only candidate, x = 1, is a removable singularity: no solution.
    assert_eq!(
        value("solve (x^2 - 1)/(x - 1) = 2, x"),
        "Solutions for x: []"
    );
    // Ordinary rational equations are unaffected.
    assert_eq!(value("solve 1/x = 0, x"), "Solutions for x: []");
}
