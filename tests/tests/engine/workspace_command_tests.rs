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

#[test]
fn trigonometric_expansion_and_simplification_preserve_the_value() {
    // The oracle is numerical: the transformed expression must agree with the original at several points.
    let cases = [
        ("expand", "sin(2*x)"),
        ("expand", "cos(x+y)"),
        ("expand", "sin(x - y)"),
        ("expand", "cos(3*x)"),
        ("expand", "sin(4*x + y)"),
        ("expand_trig", "tan(x + y)"),
        ("trigsimp", "sin(x)^2 + cos(x)^2"),
        ("trigsimp", "sin(x)^4 - cos(x)^4"),
        ("trigsimp", "cos(2*x) + 2*sin(x)^2"),
        ("trigsimp", "sin(2*x)/(2*sin(x))"),
        ("trigsimp", "sin(x)^2*cos(x)^2 + cos(x)^4"),
    ];
    for (command, expression) in cases {
        let transformed = run(&format!("{command}({expression})"));
        for (x, y) in [("0.3", "1.1"), ("-0.7", "0.4"), ("2.2", "-1.3")] {
            let (a, b) = (
                value(expression, &[("x", x), ("y", y)]),
                value(&transformed, &[("x", x), ("y", y)]),
            );
            assert!(
                (a - b).abs() < 1e-9 * a.abs().max(1.0),
                "{command}({expression}) = {transformed} at x = {x}, y = {y}: {a} vs {b}"
            );
        }
    }
    assert_eq!(run("trigsimp(sin(x)^2 + cos(x)^2)"), "1");
    assert_eq!(run("trigsimp(cos(2*x) + 2*sin(x)^2)"), "1");
    assert_eq!(run("expand(sin(2*x))"), "2 · cos(x) · sin(x)");
}

#[test]
fn initial_value_problems_satisfy_the_equation_and_the_conditions() {
    // (equation as a residual of y, y', y'', conditions as (order, point, value)).
    type Residual = fn(f64, f64, f64, f64) -> f64;
    type Case<'a> = (&'a str, &'a [(usize, f64, f64)], Residual);
    let cases: [Case; 6] = [
        ("y' = 2*y, y(0)=1", &[(0, 0.0, 1.0)], |_, y, d1, _| {
            d1 - 2.0 * y
        }),
        (
            "y'' + y = 0, y(0)=1, y'(0)=0",
            &[(0, 0.0, 1.0), (1, 0.0, 0.0)],
            |_, y, _, d2| d2 + y,
        ),
        (
            "y'' - 3*y' + 2*y = 0, y(0)=0, y'(0)=1",
            &[(0, 0.0, 0.0), (1, 0.0, 1.0)],
            |_, y, d1, d2| d2 - 3.0 * d1 + 2.0 * y,
        ),
        ("y' + y = x, y(0)=2", &[(0, 0.0, 2.0)], |x, y, d1, _| {
            d1 + y - x
        }),
        ("y' = y^2, y(0)=1", &[(0, 0.0, 1.0)], |_, y, d1, _| {
            d1 - y * y
        }),
        (
            "y'' = 0, y(0)=1, y'(0)=2",
            &[(0, 0.0, 1.0), (1, 0.0, 2.0)],
            |_, _, _, d2| d2,
        ),
    ];
    for (problem, conditions, residual) in cases {
        let out = run(&format!("dsolve({problem})"));
        let solution = out.split_once('=').unwrap().1.trim().to_string();
        let first = run(&format!("diff({solution}, x)"));
        let second = run(&format!("diff({solution}, x, 2)"));
        for &(order, point, expected) in conditions {
            let text = [&solution, &first, &second][order];
            let got = value(text, &[("x", &point.to_string())]);
            assert!(
                (got - expected).abs() < 1e-9,
                "{problem}: y^({order})({point}) = {got}, expected {expected} ({out})"
            );
        }
        for x in [0.2f64, 0.45, 0.3] {
            let at = |text: &str| value(text, &[("x", &x.to_string())]);
            let r = residual(x, at(&solution), at(&first), at(&second));
            assert!(r.abs() < 1e-8, "{problem}: residual {r} at x = {x} ({out})");
        }
    }
    // Too few conditions is an error, not a partly determined answer.
    assert!(
        UraeSession::new()
            .execute_line("dsolve(y'' + y = 0, y(0)=1)")
            .is_error
    );
}

/// A vector printed as `[ a, b, c ]` becomes input text again.
fn as_input(printed: &str) -> String {
    printed
        .replace('·', "*")
        .replace("[ ", "[")
        .replace(" ]", "]")
}

#[test]
fn vector_calculus_identities_and_finite_differences() {
    let f = "x^2*y*z + sin(x*y) + z^3";
    let g = "[x*y*z, x^2 - z, y*sin(x) + z^2]";
    // curl(grad f) = 0 and div(curl F) = 0 for smooth fields.
    assert_eq!(
        run(&format!(
            "curl({}, [x,y,z])",
            as_input(&run(&format!("gradient({f}, [x,y,z])")))
        )),
        "[ 0, 0, 0 ]"
    );
    let curl = as_input(&run(&format!("curl({g}, [x,y,z])")));
    assert_eq!(run(&format!("divergence({curl}, [x,y,z])")), "0");
    // laplacian = divergence of the gradient.
    let gradient = as_input(&run(&format!("gradient({f}, [x,y,z])")));
    let (laplacian, divergence) = (
        run(&format!("laplacian({f}, [x,y,z])")),
        run(&format!("divergence({gradient}, [x,y,z])")),
    );
    for point in [("0.3", "-0.8", "1.2"), ("1.5", "0.4", "-0.6")] {
        let vars = [("x", point.0), ("y", point.1), ("z", point.2)];
        assert!((value(&laplacian, &vars) - value(&divergence, &vars)).abs() < 1e-9);
    }
    // The gradient against a central finite difference at a point.
    let components: Vec<String> = as_input(&run(&format!("gradient({f}, [x,y,z])")))
        .trim_matches(|c| c == '[' || c == ']')
        .split(", ")
        .map(String::from)
        .collect();
    let at = [0.3f64, -0.8, 1.2];
    let evaluate = |p: [f64; 3]| {
        value(
            f,
            &[
                ("x", &p[0].to_string()),
                ("y", &p[1].to_string()),
                ("z", &p[2].to_string()),
            ],
        )
    };
    for (i, component) in components.iter().enumerate() {
        let h = 1e-5;
        let (mut hi, mut lo) = (at, at);
        hi[i] += h;
        lo[i] -= h;
        let numeric = (evaluate(hi) - evaluate(lo)) / (2.0 * h);
        let exact = value(component, &[("x", "0.3"), ("y", "-0.8"), ("z", "1.2")]);
        assert!(
            (numeric - exact).abs() < 1e-6,
            "d/d{i}: {numeric} vs {exact}"
        );
    }
    // Jacobian and Hessian shapes, and the vector products.
    assert_eq!(run("jacobian([x*y, x+y], [x,y])"), "[ y, x ; 1, 1 ]");
    assert_eq!(run("hessian(x^2*y, [x,y])"), "[ 2 · y, 2 · x ; 2 · x, 0 ]");
    assert_eq!(run("cross([1,2,3],[4,5,6])"), "[ -3, 6, -3 ]");
    assert_eq!(run("dot([1,2,3],[4,5,6])"), "32");
    assert_eq!(run("norm([3,4])"), "5");
    // Lagrange's identity |a x b|^2 = |a|^2 |b|^2 - (a . b)^2 on integer vectors.
    for (a, b) in [([1i64, 2, 3], [4i64, 5, 6]), ([2, -1, 0], [3, 3, 7])] {
        let cross = run(&format!("cross({:?}, {:?})", a, b));
        let c: Vec<i64> = cross
            .trim_matches(|ch| ch == '[' || ch == ']' || ch == ' ')
            .split(", ")
            .map(|s| s.parse().unwrap())
            .collect();
        let dot: i64 = a.iter().zip(&b).map(|(x, y)| x * y).sum();
        let (na, nb): (i64, i64) = (a.iter().map(|x| x * x).sum(), b.iter().map(|x| x * x).sum());
        assert_eq!(c.iter().map(|x| x * x).sum::<i64>(), na * nb - dot * dot);
        assert_eq!(c.iter().zip(&a).map(|(x, y)| x * y).sum::<i64>(), 0);
    }
}

#[test]
fn laurent_series_agree_with_the_function_near_the_pole() {
    // `series(1/x, x, 0, 3)` once returned `x^2/2 + x^3/6`, and sums with a pole were reported as not analytic or
    // contained `oo`. The oracle: the truncated series equals the function a short distance from the point (the error
    // is of the order of the first omitted term).
    let cases: [(&str, &str, &str); 14] = [
        ("1/x", "0", "0.01"),
        ("1/x + 1/(1-x)", "0", "0.01"),
        ("1/x - 1/(x+1)", "0", "0.01"),
        ("1/x^2 + 1", "0", "0.01"),
        ("1/(x*(1-x))", "0", "0.01"),
        ("cot(x)", "0", "0.05"),
        ("1/sin(x)", "0", "0.05"),
        ("exp(x)/x^3", "0", "0.02"),
        ("1/(exp(x) - 1)", "0", "0.05"),
        ("x*cot(x)", "0", "0.05"),
        ("x/(exp(x) - 1)", "0", "0.05"),
        ("1/x^2 - 1/sin(x)^2", "0", "0.05"),
        ("1/(x - 1)", "1", "1.01"),
        ("1/(x - 1)^2 + 1/(x - 1)", "1", "1.01"),
    ];
    for (expression, point, near) in cases {
        let series = run(&format!("series({expression}, x, {point}, 8)"));
        assert!(
            !series.contains("oo") && !series.contains('∞'),
            "{expression}: {series}"
        );
        let (f, s) = (
            value(expression, &[("x", near)]),
            value(&series, &[("x", near)]),
        );
        assert!(
            (f - s).abs() < 1e-9 * f.abs().max(1.0),
            "series({expression}) = {series}: {f} vs {s} at x = {near}"
        );
    }
    assert_eq!(run("series(1/x, x, 0, 3)"), "x^(-1)");
    assert_eq!(
        run("series(cot(x), x, 0, 3)"),
        "-1/3 · x + x^(-1) - 1/45 · x^3"
    );
}

/// The number in an interval end point: an integer or a fraction (the cases below have no other end points).
fn endpoint(text: &str) -> f64 {
    match text.trim() {
        "-∞" => f64::NEG_INFINITY,
        "∞" => f64::INFINITY,
        t => match t.split_once('/') {
            Some((a, b)) => a.parse::<f64>().unwrap() / b.parse::<f64>().unwrap(),
            None => t.parse().unwrap_or_else(|_| panic!("end point {t}")),
        },
    }
}

/// Whether `x` lies in a printed union such as `(-∞, -2) ∪ [2, ∞)` or `{1}`.
fn in_union(set: &str, x: f64) -> bool {
    set.split(" ∪ ").any(|part| {
        let part = part.trim();
        if let Some(point) = part.strip_prefix('{') {
            return (x - endpoint(point.trim_end_matches('}'))).abs() < 1e-9;
        }
        let (open, close) = (part.starts_with('['), part.ends_with(']'));
        let (lo, hi) = part[1..part.len() - 1]
            .split_once(", ")
            .map(|(a, b)| (endpoint(a), endpoint(b)))
            .unwrap();
        (x > lo || (open && x == lo)) && (x < hi || (close && x == hi))
    })
}

#[test]
fn inequalities_agree_with_direct_evaluation() {
    type Predicate = fn(f64) -> bool;
    let cases: [(&str, Predicate); 12] = [
        ("x^2 > 4", |x| x * x > 4.0),
        ("x^2 >= 4", |x| x * x >= 4.0),
        ("abs(x) < 3", |x| x.abs() < 3.0),
        ("(x-1)/(x+2) >= 0", |x| {
            x != -2.0 && (x - 1.0) / (x + 2.0) >= 0.0
        }),
        ("x^3 - x > 0", |x| x * x * x - x > 0.0),
        ("x^2 - 4*x + 3 <= 0", |x| x * x - 4.0 * x + 3.0 <= 0.0),
        ("1/x > 1", |x| x != 0.0 && 1.0 / x > 1.0),
        ("sqrt(x) < 2", |x| x >= 0.0 && x.sqrt() < 2.0),
        ("x^4 - 5*x^2 + 4 < 0", |x| {
            x.powi(4) - 5.0 * x * x + 4.0 < 0.0
        }),
        ("x*(x-2)*(x+3) < 0", |x| x * (x - 2.0) * (x + 3.0) < 0.0),
        ("(x-1)^2 <= 0", |x| (x - 1.0).powi(2) <= 0.0),
        ("x^2 + 1 > 0", |x| x * x + 1.0 > 0.0),
    ];
    for (inequality, predicate) in cases {
        let out = run(&format!("solve({inequality}, x)"));
        let set = out.strip_prefix("Solutions for x: ").unwrap();
        // A grid that includes the critical points themselves (integers and halves) and points between them.
        for k in -80..=80 {
            let x = k as f64 / 8.0;
            let expected = predicate(x);
            let got = !set.starts_with("no real") && in_union(set, x);
            assert_eq!(got, expected, "{inequality} -> {set} at x = {x}");
        }
    }
    // Periodic inequalities have infinitely many intervals: an error, not a wrong finite answer.
    assert!(
        UraeSession::new()
            .execute_line("solve(sin(x) > 0, x)")
            .is_error
    );
}

#[test]
fn gaussian_integrals_and_one_sided_limits() {
    for (integral, expected) in [
        ("exp(-x^2), x, -oo, oo", "√(π)"),
        ("exp(-x^2), x, 0, oo", "1/2 · √(π)"),
        ("exp(-2*x^2), x, -oo, oo", "√(π / 2)"),
        ("exp(-x^2 + 2*x), x, -oo, oo", "exp(1) · √(π)"),
    ] {
        let out = run(&format!("integrate({integral})"));
        assert!(out.starts_with(expected), "{integral}: {out}");
    }
    // The exact value agrees with the numerical integral.
    let numeric = run("nintegrate(exp(-x^2), x, -oo, oo)");
    let digits: String = numeric
        .split('≈')
        .nth(1)
        .unwrap()
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_digit() || matches!(c, '.' | '-'))
        .collect();
    let approx: f64 = digits.parse().unwrap();
    assert!((approx - std::f64::consts::PI.sqrt()).abs() < 1e-9);
    assert_eq!(
        run("integrate(sqrt(x), x, 0, 4)"),
        "16/3 (w.r.t x along [0, 4])"
    );
    // One-sided limits, and values that the symbolic engine used to get wrong.
    for (query, expected) in [
        ("limit(abs(x)/x, x, 0, +)", "= 1"),
        ("limit(abs(x)/x, x, 0, -)", "= -1"),
        ("limit(abs(x)/x, x, 0)", "= Undefined"),
        ("limit(1/x, x, 0, +)", "= ∞"),
        ("limit(1/x, x, 0, -)", "= -∞"),
        ("limit(exp(1/x), x, 0, +)", "= ∞"),
        ("limit(exp(1/x), x, 0, -)", "= 0"),
        ("limit(sin(1/x), x, 0)", "= Undefined"),
        ("limit(tan(x), x, pi/2, -)", "= ∞"),
        ("limit(sin(x)/x, x, 0)", "= 1"),
        ("limit((1 - cos(x))/x^2, x, 0)", "= 1/2"),
    ] {
        let out = run(query);
        assert!(out.ends_with(expected), "{query}: {out}");
    }
}

#[test]
fn radicals_and_logarithms_of_rationals_are_exact_and_correct() {
    // The oracle is f64 arithmetic: the exact form must agree with the direct evaluation.
    let cases: [(&str, f64); 22] = [
        ("2^(3/2)", 2f64.powf(1.5)),
        ("8^(1/3)", 2.0),
        ("27^(2/3)", 9.0),
        ("sqrt(50) + sqrt(18)", 50f64.sqrt() + 18f64.sqrt()),
        ("(sqrt(2) + 1)^2", (2f64.sqrt() + 1.0).powi(2)),
        ("1/sqrt(2)", 1.0 / 2f64.sqrt()),
        ("1/(1 + sqrt(2))", 1.0 / (1.0 + 2f64.sqrt())),
        ("sqrt(3/4)", 0.75f64.sqrt()),
        ("sqrt(2)^3", 2f64.sqrt().powi(3)),
        ("sqrt(2)*sqrt(3)*sqrt(5)", 30f64.sqrt()),
        ("cbrt(16)", 16f64.cbrt()),
        ("root(32, 5)", 2.0),
        ("2^(-1/2)", 2f64.powf(-0.5)),
        ("(3 + 2*sqrt(2))*(3 - 2*sqrt(2))", 1.0),
        ("exp(2*ln(3))", 9.0),
        ("e^(ln(2) + ln(3))", 6.0),
        ("exp(ln(2)/2)", 2f64.sqrt()),
        ("ln(8)/ln(2)", 3.0),
        ("log2(1/8)", -3.0),
        ("log10(1000)", 3.0),
        ("log(81, 3)", 4.0),
        ("ln(27)/ln(9)", 1.5),
    ];
    for (input, expected) in cases {
        let out = run(input);
        let text = out.replace('√', "sqrt").replace('·', "*");
        let got = value(&text, &[]);
        assert!(
            (got - expected).abs() < 1e-12 * expected.abs().max(1.0),
            "{input} = {out}: {got} vs {expected}"
        );
    }
    // The exact forms, not just the values.
    assert_eq!(run("2^(3/2)"), "2 · √(2)");
    assert_eq!(run("(sqrt(2) + 1)^2"), "3 + 2 · √(2)");
    assert_eq!(run("1/(1 + sqrt(2))"), "-1 + √(2)");
    assert_eq!(run("sqrt(18)/sqrt(2)"), "3");
    // Definite integrals of roots come out in radical form.
    assert_eq!(
        run("integrate(sqrt(x), x, 1, 2)"),
        "4/3 · √(2) - 2/3 (w.r.t x along [1, 2])"
    );
    assert_eq!(
        run("integrate(sqrt(x), x, 0, 2)"),
        "4/3 · √(2) (w.r.t x along [0, 2])"
    );
    // A symbolic expression is shown simplified when that is shorter.
    assert_eq!(run("x^2*x^3"), "x^5");
    assert_eq!(run("x^5/x^2"), "x^3");
    assert_eq!(run("(x^3)^2"), "x^6");
    // ...and an irrational logarithm stays symbolic.
    assert_eq!(run("ln(2)/ln(3)"), "ln(2) / ln(3)");
}
