//! Call spellings, ordinary variable names, products and Z-transforms.

use algebra_engine::session::UraeSession;

fn run(q: &str) -> String {
    let r = UraeSession::new().execute_line(q);
    assert!(!r.is_error, "{q}: {}", r.output_text);
    r.output_text
}

/// Evaluate `text` with the given variable values, in a fresh session.
fn value(text: &str, vars: &[(&str, &str)]) -> f64 {
    let mut s = UraeSession::new();
    for (name, v) in vars {
        s.execute_line(&format!("{name} = {v}"));
    }
    let out = s.execute_line(&format!("N({text})")).output_text;
    out.split('≈')
        .next_back()
        .unwrap()
        .trim()
        .parse()
        .unwrap_or_else(|_| panic!("cannot read a number from {out} (for {text})"))
}

#[test]
fn i_j_k_are_ordinary_variables() {
    // They were quaternion units in the simplifier: k^2 became -1, so exp(-k^2/4) became exp(1/4).
    assert_eq!(run("simplify(k^2/4)"), "1/4 · k^2");
    assert_eq!(run("simplify(exp(-k^2/4))"), "exp(-1/4 · k^2)");
    assert_eq!(run("simplify(2*j*j)"), "2 · j^2");
    assert_eq!(run("simplify(j*j*j)"), "j^3");
    // The Fourier transform of the Gaussian is sqrt(pi) exp(-k^2/4) whatever the frequency variable is called.
    for w in ["w", "k", "s"] {
        let out = run(&format!("fourier(exp(-x^2), x, {w})"));
        assert!(out.contains(&format!("{w}^2")), "{out}");
        assert!(!out.contains("exp(1/4)"), "{out}");
    }
    // qi, qj, qk are still the quaternion units.
    assert_eq!(run("simplify(qi*qj)"), "qk");
}

#[test]
fn call_spellings_match_command_spellings() {
    assert_eq!(run("expand((x+1)^2)"), run("expand (x+1)^2"));
    assert_eq!(run("factor(x^4 - 1)"), "(x - 1) · (x + 1) · (x^2 + 1)");
    assert_eq!(run("gcd(x^2 - 1, x^3 - 1)"), "x - 1");
    assert_eq!(run("gcd(12, 18)"), "6");
    assert!(run("laplace(exp(-t)*sin(t), t, s)").contains("s^2 + 2 · s + 2"));
}

#[test]
fn finite_and_closed_form_products() {
    assert_eq!(run("product(k, k, 1, 5)"), "120");
    assert_eq!(run("prod(k, k, 1, 5)"), "120");
    // Telescoping: prod_{k=2}^{n} (1 - 1/k^2) = (n + 1) / (2 n); at n = 10 this is 11/20.
    assert_eq!(run("product(1 - 1/k^2, k, 2, 10)"), "11/20");
    assert_eq!(run("product(c, k, 1, n)"), "c^n");
    assert_eq!(run("product(k, k, 1, n)"), "factorial(n)");
    // A closed form must agree with the finite product at several n.
    for n in 1..=6 {
        let closed = value(&run("product(2^k, k, 1, n)"), &[("n", &n.to_string())]);
        let direct: f64 = (1..=n).map(|k| 2f64.powi(k)).product();
        assert!((closed - direct).abs() < 1e-9 * direct, "n = {n}");
    }
}

#[test]
fn z_transforms_match_the_defining_series() {
    // |z| above the radius of convergence: compare with the truncated sum of x[n] z^-n.
    let z = 5.0f64;
    type Sequence = Box<dyn Fn(f64) -> f64>;
    let cases: [(&str, Sequence); 5] = [
        ("1", Box::new(|_| 1.0)),
        ("n", Box::new(|n| n)),
        ("n^2", Box::new(|n| n * n)),
        ("2^n", Box::new(|n| 2f64.powf(n))),
        ("n*3^n + 4", Box::new(|n| n * 3f64.powf(n) + 4.0)),
    ];
    for (sequence, x) in cases {
        let image = run(&format!("ztransform({sequence}, n, z)"));
        let series: f64 = (0..400).map(|n| x(n as f64) / z.powi(n)).sum();
        let got = value(&image, &[("z", "5")]);
        assert!(
            (got - series).abs() < 1e-9 * series.abs().max(1.0),
            "Z{{{sequence}}} = {image}: {got} vs {series}"
        );
    }
    // A sequence outside the family is an error rather than a wrong image.
    assert!(
        UraeSession::new()
            .execute_line("ztransform(sin(n), n, z)")
            .is_error
    );
}

#[test]
fn series_refuses_functions_that_are_not_analytic_at_the_point() {
    // x^(x^x) at 0 once returned `0`, reading 0^0 as 1 in the derivative method.
    for q in [
        "series(x^x^x, x, 0, 6)",
        "series(x^x, x, 0, 4)",
        "series(sin(x)^x, x, 0, 4)",
        "series((x - 1)^x, x, 1, 3)",
    ] {
        assert!(UraeSession::new().execute_line(q).is_error, "{q}");
    }
    // A base that is not zero at the point is fine: (1 + x)^x = 1 + x^2 - x^3/2 + ...
    let out = run("series((1 + x)^x, x, 0, 3)");
    assert_eq!(out, "x^2 - 1/2 · x^3 + 1");
}
