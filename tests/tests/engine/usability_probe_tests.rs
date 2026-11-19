//! Regressions found by running the commands people type first: a definite integral written `integrate f from a to
//! b` used to return the antiderivative (the bounds were dropped), and `factor 360` returned `360`.

use algebra_engine::session::UraeSession;

fn run(q: &str) -> String {
    let r = UraeSession::new().execute_line(q);
    assert!(!r.is_error, "{q}: {}", r.output_text);
    r.output_text
}

/// Composite Simpson's rule: the oracle for the definite integrals.
fn simpson(f: impl Fn(f64) -> f64, a: f64, b: f64) -> f64 {
    let n = 2000;
    let h = (b - a) / n as f64;
    let mut sum = f(a) + f(b);
    for i in 1..n {
        sum += f(a + i as f64 * h) * if i % 2 == 1 { 4.0 } else { 2.0 };
    }
    sum * h / 3.0
}

#[test]
fn definite_integrals_written_with_from_and_to_return_numbers() {
    // Each answer must be the value of the integral, not an antiderivative in x.
    assert_eq!(
        run("integrate x^2 from 0 to 1"),
        "1/3 (w.r.t x along [0, 1])"
    );
    assert_eq!(
        run("integrate sin(x) from 0 to pi"),
        "2 (w.r.t x along [0, pi])"
    );
    assert_eq!(run("integrate 1/x from 1 to e"), "1 (w.r.t x along [1, e])");
    assert_eq!(
        run("integrate x^2 dx from 0 to 3"),
        "9 (w.r.t x along [0, 3])"
    );
    assert_eq!(
        run("integral of cos(t) from 0 to pi/2"),
        "1 (w.r.t t along [0, pi/2])"
    );
    // The Gaussian has no elementary antiderivative; the answer is erf at the upper bound (erf(0) = 0).
    let gauss = run("integrate exp(-x^2) from 0 to 1");
    assert!(gauss.contains("erf(1)"), "{gauss}");
    // The value agrees with numerical quadrature: sqrt(pi)/2 erf(1) = 0.746824...
    assert!((simpson(|x| (-x * x).exp(), 0.0, 1.0) - 0.746_824_132_812_427).abs() < 1e-9);
}

#[test]
fn bounds_are_never_silently_dropped() {
    // Several variables and bounds: ask which one, instead of integrating over every variable or none.
    let r = UraeSession::new().execute_line("integrate x*y from 0 to 1");
    assert!(r.is_error, "{r:?}");
    assert!(
        r.output_text.contains("several variables"),
        "{}",
        r.output_text
    );
    // With the variable named, the other one stays symbolic.
    assert_eq!(
        run("integrate x*y dx from 0 to 1"),
        "1/2 · y (w.r.t x along [0, 1])"
    );
    // An indefinite integral is still an antiderivative.
    assert_eq!(run("integrate x^2"), "1/3 · x^3 (w.r.t x)");
}

#[test]
fn factor_splits_integers_into_primes() {
    // The product is printed in the engine's canonical factor order, so compare the set of factors.
    let mut parts: Vec<String> = run("factor 360").split(" · ").map(str::to_string).collect();
    parts.sort();
    assert_eq!(parts, vec!["2^3", "3^2", "5"]);
    assert_eq!(run("factor 97"), "97");
    assert_eq!(run("factor 1024"), "2^10");
    // Polynomials are unaffected.
    assert_eq!(run("factor x^4 - 1"), "(x - 1) · (x + 1) · (x^2 + 1)");
}
