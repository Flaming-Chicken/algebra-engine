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

#[test]
fn constants_and_complex_numbers_are_evaluated_exactly() {
    // These were echoed back: constants built from pi, and any arithmetic with i.
    for (q, expected) in [
        ("sin(pi/6)", "1/2"),
        ("cos(pi/3)", "1/2"),
        ("tan(pi/4)", "1"),
        ("sin(pi)", "0"),
        ("cos(pi)", "-1"),
        ("(1+2i)*(3-i)", "5 + 5i"),
        ("(2+3i)/(1-i)", "-1/2 + 5i/2"),
        ("(1+i)^2", "2i"),
        ("(1+i)^8", "16"),
        ("i^2", "-1"),
        ("sqrt(-4)", "2i"),
        ("abs(3+4i)", "5"),
        ("conj(1+i)", "1 - i"),
        ("re(2+3i)", "2"),
        ("im(2+3i)", "3"),
    ] {
        assert_eq!(run(q), expected, "{q}");
    }
    // Oracle: the exact complex product agrees with component arithmetic for many small Gaussian integers.
    for (a, b, c, d) in [
        (1i64, 2i64, 3i64, -1i64),
        (-4, 5, 7, 2),
        (0, 3, 3, 0),
        (2, -2, -2, 2),
    ] {
        let out = run(&format!("(({a})+({b})*i)*(({c})+({d})*i)"));
        let (re, im) = (a * c - b * d, a * d + b * c);
        let expected = match (re, im) {
            (r, 0) => r.to_string(),
            (0, 1) => "i".into(),
            (0, -1) => "-i".into(),
            (0, m) => format!("{}i", m),
            (r, m) => format!(
                "{r} {} {}",
                if m < 0 { "-" } else { "+" },
                if m.abs() == 1 {
                    "i".into()
                } else {
                    format!("{}i", m.abs())
                }
            ),
        };
        assert_eq!(out, expected, "({a}+{b}i)({c}+{d}i)");
    }
    // A symbol keeps the expression symbolic.
    assert_eq!(run("x + i"), "x + i");
}

fn matrix_entries(text: &str) -> Vec<Vec<i128>> {
    text.trim_start_matches('[')
        .trim_end_matches(']')
        .split(';')
        .map(|row| row.split(',').map(|v| v.trim().parse().unwrap()).collect())
        .collect()
}

#[test]
fn matrix_powers_lu_and_integer_factorisation() {
    // Powers against repeated multiplication done here on integers.
    let multiply = |a: &Vec<Vec<i128>>, b: &Vec<Vec<i128>>| -> Vec<Vec<i128>> {
        (0..2)
            .map(|i| {
                (0..2)
                    .map(|j| (0..2).map(|k| a[i][k] * b[k][j]).sum())
                    .collect()
            })
            .collect()
    };
    for (m, text) in [
        ([[1i128, 2], [3, 4]], "[[1,2],[3,4]]"),
        ([[1, 1], [1, 0]], "[[1,1],[1,0]]"),
        ([[0, -1], [1, 0]], "[[0,-1],[1,0]]"),
    ] {
        let base: Vec<Vec<i128>> = m.iter().map(|r| r.to_vec()).collect();
        let mut expected = vec![vec![1, 0], vec![0, 1]];
        for n in 0..=12 {
            assert_eq!(
                matrix_entries(&run(&format!("{text}^{n}"))),
                expected,
                "{text}^{n}"
            );
            expected = multiply(&expected, &base);
        }
    }
    // A negative power is the inverse of the positive power: M^-3 M^3 = I.
    let inverse = matrix_entries(&run("[[2,1],[1,1]]^-3"));
    let forward = matrix_entries(&run("[[2,1],[1,1]]^3"));
    assert_eq!(multiply(&inverse, &forward), vec![vec![1, 0], vec![0, 1]]);
    assert_eq!(
        run("lu([[4,3],[6,3]])"),
        "L = [ 1, 0 ; 3/2, 1 ], U = [ 4, 3 ; 0, -3/2 ]"
    );
    // Integer factorisation of constant expressions: the factors multiply back (Fermat numbers 2^32+1, 2^64+1).
    for (q, n) in [
        ("factor(2^32+1)", 4_294_967_297u128),
        ("factor(10!)", 3_628_800),
        ("factor(2^64+1)", 18_446_744_073_709_551_617),
    ] {
        let out = run(q);
        let head = out.split('[').next().unwrap();
        let product: u128 = head
            .split('=')
            .nth(1)
            .unwrap()
            .split('*')
            .map(|f| {
                let f = f.trim();
                match f.split_once('^') {
                    Some((b, e)) => b.parse::<u128>().unwrap().pow(e.parse().unwrap()),
                    None => f.parse().unwrap(),
                }
            })
            .product();
        assert_eq!(product, n, "{out}");
    }
}
