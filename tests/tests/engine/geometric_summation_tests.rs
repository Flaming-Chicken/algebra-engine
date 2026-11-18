//! `SymbolicCalculus::summation` for polynomial times geometric summands (via Gosper's algorithm),
//! compared with brute-force exact sums.

use algebra_core::parser::ExprParser;
use algebra_core::{ExprGraph, ExprKind, Number};
use algebra_engine::calculus::SymbolicCalculus;
use algebra_engine::simplify::Simplifier;
use num_bigint::BigInt;
use num_rational::BigRational;

fn rational(c: i64, d: i64) -> BigRational {
    BigRational::new(BigInt::from(c), BigInt::from(d))
}

/// The exact value `sum_{k=lo}^{hi} term(k)` of the summand text, by direct evaluation.
fn brute(text: &str, lo: i64, hi: i64) -> BigRational {
    let g = ExprGraph::new();
    let mut total = rational(0, 1);
    for k in lo..=hi {
        let e = ExprParser::new(&g)
            .parse(&text.replace('k', &format!("({k})")))
            .unwrap();
        total += value(&g, Simplifier::simplify(&g, e).unwrap());
    }
    total
}

fn value(g: &ExprGraph, e: algebra_core::ExprId) -> BigRational {
    match &g.get(e).kind {
        ExprKind::Number(Number::Integer(i)) => rational(*i, 1),
        ExprKind::Number(Number::Rational(n, d)) => rational(*n, *d),
        other => panic!("not a rational constant: {other:?}"),
    }
}

fn closed_form(text: &str, lo: i64, hi: i64) -> BigRational {
    let g = ExprGraph::new();
    let expr = ExprParser::new(&g).parse(text).unwrap();
    let ExprKind::Symbol(k) = g.get(g.symbol("k")).kind else {
        unreachable!()
    };
    let sum = g.summation(expr, k, g.integer(lo), g.integer(hi)).unwrap();
    value(&g, Simplifier::simplify(&g, sum).unwrap())
}

#[test]
fn polynomial_times_geometric_sums_match_brute_force() {
    for text in [
        "2^k",
        "k*2^k",
        "k^2*3^k",
        "(k+1)*2^k",
        "5^k",
        "k^3*(-2)^k",
        "(1/2)^k",
        "k*(1/3)^k",
        "2^k*k^2 + 2^k",
    ] {
        for (lo, hi) in [(0, 6), (1, 7), (3, 9), (2, 2)] {
            assert_eq!(
                closed_form(text, lo, hi),
                brute(text, lo, hi),
                "sum of {text} for k = {lo}..{hi}"
            );
        }
    }
}

#[test]
fn sums_without_a_closed_form_report_it() {
    let g = ExprGraph::new();
    let ExprKind::Symbol(k) = g.get(g.symbol("k")).kind else {
        unreachable!()
    };
    // A power of k with a non-numeric base, or a function of k, is simply unsupported...
    for text in ["x^k", "sin(k)", "1/k"] {
        let e = ExprParser::new(&g).parse(text).unwrap();
        assert!(
            g.summation(e, k, g.integer(1), g.integer(5)).is_err(),
            "{text}"
        );
    }
    // ...and the existing polynomial and constant cases still work.
    for text in ["k", "k^2", "7"] {
        let e = ExprParser::new(&g).parse(text).unwrap();
        let s = g.summation(e, k, g.integer(1), g.integer(10)).unwrap();
        assert_eq!(
            value(&g, Simplifier::simplify(&g, s).unwrap()),
            brute(text, 1, 10),
            "{text}"
        );
    }
}
