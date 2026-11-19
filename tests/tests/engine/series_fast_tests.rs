//! The fast Taylor path (exact series arithmetic on rational coefficients): known coefficients, a numerical oracle
//! (the truncated polynomial against the function at a small argument) and a speed bound.

use algebra_engine::session::UraeSession;
use std::time::Instant;

fn series(q: &str) -> String {
    let r = UraeSession::new().execute_line(q);
    assert!(!r.is_error, "{q}: {}", r.output_text);
    r.output_text
}

/// Evaluate a series printed as a polynomial in `x` by substituting a number through the engine itself.
fn eval_at(poly_text: &str, x: f64) -> f64 {
    let mut s = UraeSession::new();
    s.execute_line(&format!("x = {x}"));
    s.execute_line(poly_text)
        .output_text
        .parse::<f64>()
        .unwrap_or_else(|_| panic!("not a number: {poly_text}"))
}

#[test]
fn known_coefficients() {
    assert_eq!(series("series(exp(x), x, 0, 4)").matches('/').count(), 3); // 1/2, 1/6, 1/24 (after 1 + x)
    assert_eq!(
        series("taylor(sin(x), x, 0, 7)"),
        "x - 1/6 · x^3 + 1/120 · x^5 - 1/5040 · x^7"
    );
    let tan = series("series(tan(x), x, 0, 7)");
    for piece in ["1/3 · x^3", "2/15 · x^5", "17/315 · x^7"] {
        assert!(tan.contains(piece), "{tan}");
    }
    let ln = series("series(ln(1+x), x, 0, 4)");
    for piece in ["x", "1/2 · x^2", "1/3 · x^3", "1/4 · x^4"] {
        assert!(ln.contains(piece), "{ln}");
    }
    let sqrt = series("series(sqrt(1+x), x, 0, 4)");
    for piece in ["1/2 · x", "1/8 · x^2", "1/16 · x^3", "5/128 · x^4"] {
        assert!(sqrt.contains(piece), "{sqrt}");
    }
    // exp(sin x) = 1 + x + x^2/2 - x^4/8 - x^5/15 - x^6/240 + ...
    let e = series("series(exp(sin(x)), x, 0, 6)");
    for piece in ["1/2 · x^2", "1/8 · x^4", "1/15 · x^5", "1/240 · x^6"] {
        assert!(e.contains(piece), "{e}");
    }
    // Around a non-zero rational point: ln x around 1 is (x-1) - (x-1)^2/2 + ...
    assert!(series("series(ln(x), x, 1, 3)").contains("1/3"));
}

#[test]
fn the_truncated_series_matches_the_function_at_a_small_argument() {
    // Truncation error is O(x^(n+1)) = 1e-13 at x = 0.1 and n = 12; the session prints about ten digits.
    let h = 0.1f64;
    let cases: Vec<(&str, f64)> = vec![
        ("exp(sin(x))", h.sin().exp()),
        ("sin(exp(x) - 1)", (h.exp() - 1.0).sin()),
        ("1/(1 - x - x^2)", 1.0 / (1.0 - h - h * h)),
        ("sqrt(1 + sin(x))", (1.0 + h.sin()).sqrt()),
        ("exp(x)*cos(x)", h.exp() * h.cos()),
        ("ln(1 + x + x^2)", (1.0 + h + h * h).ln()),
        ("atan(sinh(x))", h.sinh().atan()),
        ("tan(x)/(1+x)", h.tan() / (1.0 + h)),
        ("(1+x)^(3/2)", (1.0 + h).powf(1.5)),
        ("cosh(x)^3", h.cosh().powi(3)),
    ];
    for (f, exact) in cases {
        let poly = series(&format!("series({f}, x, 0, 12)"));
        let value = eval_at(&poly, h);
        assert!((value - exact).abs() < 1e-9, "{f}: {value} vs {exact}");
    }
}

#[test]
fn deep_compositions_are_fast() {
    // The differentiation method took 22 seconds (release) for the first of these.
    let start = Instant::now();
    series("series(exp(sin(x)), x, 0, 12)");
    series("series(sin(sin(exp(x) - 1)), x, 0, 15)");
    series("series(ln(1 + tan(x)), x, 0, 15)");
    series("series(exp(exp(x) - 1), x, 0, 15)");
    assert!(
        start.elapsed().as_secs_f64() < 5.0,
        "took {:?}",
        start.elapsed()
    );
}
