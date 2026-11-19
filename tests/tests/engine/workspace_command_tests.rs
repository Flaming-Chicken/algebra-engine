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

/// The number after the last `=` of `5 [km] = 3.1 [miles]`.
fn converted(out: &str) -> f64 {
    let after = out.rsplit_once('=').unwrap().1.trim();
    after.split_whitespace().next().unwrap().parse().unwrap()
}

#[test]
fn unit_conversion_is_dimensionally_checked_and_correct() {
    // `convert 5 km to miles` used to answer 5 (and `convert 100 F to C` 100): every unknown pair was the identity.
    for (query, expected) in [
        ("convert 5 km to miles", 5000.0 / 1609.344),
        ("convert 100 F to C", 37.777_777_78),
        ("convert 0 C to K", 273.15),
        ("convert 100 C to F", 212.0),
        ("convert 32 F to K", 273.15),
        ("convert 60 mph to km/h", 96.560_64),
        ("convert 9.81 m/s^2 to ft/s^2", 9.81 / 0.3048),
        ("convert 1 kWh to J", 3.6e6),
        ("convert 1 atm to psi", 14.695_948_78),
        ("convert 72 km/h to m/s", 20.0),
        ("convert 1 N to kg*m/s^2", 1.0),
        ("convert 2 hours to minutes", 120.0),
        ("convert 1 GHz to Hz", 1e9),
        ("convert 1 L to m^3", 1e-3),
        ("convert 180 deg to rad", std::f64::consts::PI),
    ] {
        let got = converted(&run(query));
        assert!(
            (got - expected).abs() <= 1e-8 * expected.abs().max(1.0),
            "{query}: {got} vs {expected}"
        );
    }
    // Round trips through another unit of the same dimension return the value.
    for (a, b) in [
        ("km", "mi"),
        ("lb", "kg"),
        ("psi", "bar"),
        ("F", "C"),
        ("mph", "m/s"),
        ("eV", "J"),
    ] {
        let there = converted(&run(&format!("convert 7.25 {a} to {b}")));
        let back = converted(&run(&format!("convert {there} {b} to {a}")));
        assert!((back - 7.25).abs() < 1e-6, "{a} -> {b} -> {a}: {back}");
    }
    // Incompatible dimensions and unknown units are errors.
    for bad in [
        "convert 5 km to kg",
        "convert 1 s to m",
        "convert 1 foo to m",
        "convert 5 J to W",
    ] {
        assert!(UraeSession::new().execute_line(bad).is_error, "{bad}");
    }
}

#[test]
fn undefined_arithmetic_and_more_statistics() {
    // `0/0` was `1`, `oo - oo` was `0`.
    for q in ["0/0", "1/0", "oo - oo", "5/(3-3)", "0^(-1)"] {
        assert!(run(q).starts_with("Undefined"), "{q}");
    }
    assert_eq!(run("mode [1,2,2,3]"), "mode = 2");
    assert_eq!(run("mode [5,5,6,6,7]"), "mode = 5, 6");
    assert_eq!(run("range [3,9,1]"), "range = 8");
    // Covariance and correlation against the definitions computed here.
    let (xs, ys) = ([1.0f64, 2.0, 3.0, 4.0], [2.0f64, 4.0, 6.0, 9.0]);
    let (mx, my) = (xs.iter().sum::<f64>() / 4.0, ys.iter().sum::<f64>() / 4.0);
    let sxy: f64 = xs.iter().zip(&ys).map(|(x, y)| (x - mx) * (y - my)).sum();
    let sxx: f64 = xs.iter().map(|x| (x - mx).powi(2)).sum();
    let syy: f64 = ys.iter().map(|y| (y - my).powi(2)).sum();
    let number_after_approx =
        |out: String| -> f64 { out.split('≈').nth(1).unwrap().trim().parse().unwrap() };
    assert!(
        (number_after_approx(run("covariance [1,2,3,4], [2,4,6,9]")) - sxy / 3.0).abs() < 1e-12
    );
    assert!(
        (number_after_approx(run("correlation [1,2,3,4], [2,4,6,9]")) - sxy / (sxx * syy).sqrt())
            .abs()
            < 1e-12
    );
    assert_eq!(run("correlation [[1,2],[2,4],[3,6]]"), "correlation = 1");
    assert_eq!(run("corr [1,2,3], [3,2,1]"), "correlation = -1");
}

#[test]
fn exact_trigonometric_values_at_multiples_of_pi_over_twelve() {
    for k in -30i64..=30 {
        let angle = k as f64 * std::f64::consts::PI / 12.0;
        for (name, f) in [("sin", f64::sin as fn(f64) -> f64), ("cos", f64::cos)] {
            let out = run(&format!("{name}({k}*pi/12)"));
            let got = value(&out.replace('√', "sqrt").replace('·', "*"), &[]);
            assert!(
                (got - f(angle)).abs() < 1e-12,
                "{name}({k} pi/12) = {out}: {got} vs {}",
                f(angle)
            );
        }
        // tan at the odd multiples of pi/2 is undefined.
        if k.rem_euclid(12) != 6 {
            let out = run(&format!("tan({k}*pi/12)"));
            let got = value(&out.replace('√', "sqrt").replace('·', "*"), &[]);
            assert!(
                (got - angle.tan()).abs() < 1e-9 * angle.tan().abs().max(1.0),
                "tan({k} pi/12) = {out}"
            );
        }
    }
    assert_eq!(run("tan(pi/12)"), "2 - √(3)");
    assert_eq!(run("sin(pi/6)"), "1/2");
    assert_eq!(run("cos(pi/4)"), "1/2 · √(2)");
    assert_eq!(run("sin(pi/12)"), "-1/4 · √(2) + 1/4 · √(6)");
}

#[test]
fn propositional_formulas_are_classified_like_their_truth_tables() {
    // Brute force over every assignment of (p, q, r) decides the expected class.
    type Formula = fn(bool, bool, bool) -> bool;
    let cases: [(&str, Formula); 10] = [
        ("p and not p", |p, _, _| p && !p),
        ("p or not p", |p, _, _| p || !p),
        ("p implies q", |p, q, _| !p || q),
        ("(p implies q) iff (not p or q)", |p, q, _| {
            (!p || q) == (!p || q)
        }),
        ("p xor q", |p, q, _| p ^ q),
        ("not (p and q) iff (not p or not q)", |p, q, _| {
            !(p && q) == (!p || !q)
        }),
        ("(p and q) or r", |p, q, r| (p && q) || r),
        ("p -> (q -> p)", |p, q, _| !p || (!q || p)),
        ("(p -> q) and (q -> r) and not (p -> r)", |p, q, r| {
            (!p || q) && (!q || r) && !(!p || r)
        }),
        ("p & ~p", |p, _, _| p && !p),
    ];
    for (formula, f) in cases {
        let mut values = Vec::new();
        for bits in 0..8u8 {
            values.push(f(bits & 1 == 1, bits & 2 != 0, bits & 4 != 0));
        }
        // The formula may not mention every variable; evaluating over all three still decides the class.
        let expected = if values.iter().all(|v| !v) {
            "contradiction"
        } else if values.iter().all(|v| *v) {
            "tautology"
        } else {
            "contingent"
        };
        let out = run(formula);
        assert!(
            out.contains(expected),
            "{formula}: expected {expected}, got {out}"
        );
    }
    // Arithmetic that happens to contain words is not logic.
    assert_eq!(run("x + 1"), "x + 1");
}

#[test]
fn finite_set_expressions_agree_with_hash_sets() {
    use std::collections::BTreeSet;
    let show = |s: &BTreeSet<i32>| {
        format!(
            "{{{}}}",
            s.iter().map(i32::to_string).collect::<Vec<_>>().join(", ")
        )
    };
    let samples: [(Vec<i32>, Vec<i32>); 4] = [
        (vec![1, 2, 3], vec![3, 4]),
        (vec![5, -1, 2, 2], vec![2, 5, 9]),
        (vec![], vec![1]),
        (vec![7], vec![7]),
    ];
    for (a, b) in samples {
        let (sa, sb): (BTreeSet<i32>, BTreeSet<i32>) =
            (a.iter().copied().collect(), b.iter().copied().collect());
        let text = |v: &Vec<i32>| {
            format!(
                "{{{}}}",
                v.iter().map(i32::to_string).collect::<Vec<_>>().join(",")
            )
        };
        let (ta, tb) = (text(&a), text(&b));
        assert_eq!(
            run(&format!("{ta} union {tb}")),
            show(&sa.union(&sb).copied().collect())
        );
        assert_eq!(
            run(&format!("{ta} intersect {tb}")),
            show(&sa.intersection(&sb).copied().collect())
        );
        assert_eq!(
            run(&format!("{ta} minus {tb}")),
            show(&sa.difference(&sb).copied().collect())
        );
        assert_eq!(
            run(&format!("{ta} xor {tb}")),
            show(&sa.symmetric_difference(&sb).copied().collect())
        );
        assert_eq!(
            run(&format!("{ta} subset {tb}")),
            sa.is_subset(&sb).to_string()
        );
        assert_eq!(run(&format!("size {ta}")), sa.len().to_string());
        for x in [-1, 2, 3, 7, 100] {
            assert_eq!(
                run(&format!("{x} in {ta}")),
                sa.contains(&x).to_string(),
                "{x} in {ta}"
            );
        }
    }
    // Equal rationals are one element; `in` is not read as a domain declaration.
    assert_eq!(run("{1/2, 0.5, 2/4} union {1}"), "{1/2, 1}");
    assert_eq!(run("{1,2} cross {a,b}"), "{(1, a), (1, b), (2, a), (2, b)}");
}

#[test]
fn commands_compose_and_large_powers_expand() {
    // `factor(expand(P))` was echoed back with the inner call unevaluated.
    let p = "(x^2+x+1)*(x^3+2*x+5)*(x-7)*(x^4+1)";
    let factored = run(&format!("factor(expand({p}))"));
    assert!(!factored.contains("expand"), "{factored}");
    for x in ["0.5", "-1.3", "2.2"] {
        let (a, b) = (
            value(p, &[("x", x)]),
            value(&factored.replace('·', "*"), &[("x", x)]),
        );
        assert!((a - b).abs() < 1e-9 * a.abs().max(1.0), "{factored} at {x}");
    }
    // The derivative of the antiderivative of an expanded cube is the cube.
    let anti = run("integrate(expand((x+1)^3), x)");
    let anti = anti.split(" (w.r.t").next().unwrap().replace('·', "*");
    let derivative = run(&format!("diff({anti}, x)")).replace('·', "*");
    for x in ["0.5", "-1.3", "2.2"] {
        assert!((value(&derivative, &[("x", x)]) - value("(x+1)^3", &[("x", x)])).abs() < 1e-9);
    }
    assert_eq!(run("diff(simplify((x^2-1)/(x-1)), x)"), "1");
    assert_eq!(
        run("solve(expand((x-1)*(x-2)) = 0, x)"),
        "Solutions for x: [1, 2]"
    );
    // (x + 1)^60: 61 binomial coefficients, the central one C(60, 30).
    let big = run("expand((x+1)^60)");
    assert_eq!(big.matches(" + ").count(), 60, "{big}");
    assert!(big.contains("118264581564861424"));
}

/// `[ a, b ; c, d ]` as numbers (fractions allowed).
fn grid(text: &str) -> Vec<Vec<f64>> {
    let number = |s: &str| -> f64 {
        match s.trim().split_once('/') {
            Some((a, b)) => a.trim().parse::<f64>().unwrap() / b.trim().parse::<f64>().unwrap(),
            None => s
                .trim()
                .parse()
                .unwrap_or_else(|_| panic!("number {s} in {text}")),
        }
    };
    text.trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(';')
        .map(|row| row.split(',').map(number).collect())
        .collect()
}

fn literal(m: &[Vec<f64>]) -> String {
    format!(
        "[{}]",
        m.iter()
            .map(|r| format!(
                "[{}]",
                r.iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn mat_mul(a: &[Vec<f64>], b: &[Vec<f64>]) -> Vec<Vec<f64>> {
    (0..a.len())
        .map(|i| {
            (0..b[0].len())
                .map(|j| (0..b.len()).map(|k| a[i][k] * b[k][j]).sum())
                .collect()
        })
        .collect()
}

#[test]
fn matrix_arithmetic_agrees_with_direct_computation() {
    // `[[1,2],[3,4]] + [[1,1],[1,1]]` and `2*A` were echoed back as `matrix(row(...))` text.
    let (a, b) = (
        vec![
            vec![2.0, -1.0, 3.0],
            vec![0.0, 4.0, 1.0],
            vec![5.0, 2.0, -2.0],
        ],
        vec![
            vec![1.0, 1.0, 0.0],
            vec![-2.0, 0.0, 3.0],
            vec![4.0, 1.0, 1.0],
        ],
    );
    let (ta, tb) = (literal(&a), literal(&b));
    let sum = grid(&run(&format!("{ta} + {tb}")));
    let difference = grid(&run(&format!("{ta} - {tb}")));
    let product = grid(&run(&format!("{ta} * {tb}")));
    let scaled = grid(&run(&format!("3*{ta} - {tb}/2")));
    for i in 0..3 {
        for j in 0..3 {
            assert_eq!(sum[i][j], a[i][j] + b[i][j]);
            assert_eq!(difference[i][j], a[i][j] - b[i][j]);
            assert_eq!(scaled[i][j], 3.0 * a[i][j] - b[i][j] / 2.0);
        }
    }
    assert_eq!(product, mat_mul(&a, &b));
    // (AB)^T = B^T A^T, A A^-1 = I, A^3 = A A A, and the Cayley-Hamilton theorem for the characteristic polynomial.
    let left = grid(&run(&format!("transpose({ta} * {tb})")));
    let right = grid(&run(&format!("transpose({tb}) * transpose({ta})")));
    assert_eq!(left, right);
    let inverse = grid(&run(&format!("{ta}^-1")));
    let identity = mat_mul(&a, &inverse);
    for i in 0..3 {
        for j in 0..3 {
            assert!((identity[i][j] - if i == j { 1.0 } else { 0.0 }).abs() < 1e-12);
        }
    }
    assert_eq!(
        grid(&run(&format!("{ta}^3"))),
        mat_mul(&mat_mul(&a, &a), &a)
    );
    assert_eq!(
        run("[[1,2],[3,4]]^2 - 5*[[1,2],[3,4]] - 2*identity(2)"),
        "[ 0, 0 ; 0, 0 ]"
    );
    // A matrix times a vector, a vector times a matrix, and a solved linear system.
    assert_eq!(run("[[1,2],[3,4]] * [1,1]"), "[ 3 ; 7 ]");
    assert_eq!(run("[1,1] * [[1,2],[3,4]]"), "[ 4, 6 ]");
    let x = grid(&run("solve_linear([[2,1],[1,3]], [5,10])"));
    assert!(
        (2.0 * x[0][0] + x[1][0] - 5.0).abs() < 1e-12
            && (x[0][0] + 3.0 * x[1][0] - 10.0).abs() < 1e-12
    );
    // Eigenvectors satisfy A v = lambda v.
    let out = run("eigenvectors([[2,1],[1,2]])");
    for part in out.split("; ") {
        let (lambda, vector) = part.trim_start_matches("λ = ").split_once(": ").unwrap();
        let lambda: f64 = lambda.parse().unwrap();
        let v: Vec<f64> = vector
            .trim_matches(|c| c == '[' || c == ']')
            .split(", ")
            .map(|s| s.parse().unwrap())
            .collect();
        let av = [2.0 * v[0] + v[1], v[0] + 2.0 * v[1]];
        assert!(
            (av[0] - lambda * v[0]).abs() < 1e-12 && (av[1] - lambda * v[1]).abs() < 1e-12,
            "{part}"
        );
    }
    assert_eq!(run("eigenvalues([[0,1],[-1,0]])"), "[i, -i]");
    assert_eq!(run("det(inverse([[1,2],[3,4]]))"), "-1/2");
    assert_eq!(run("identity(3)"), "[ 1, 0, 0 ; 0, 1, 0 ; 0, 0, 1 ]");
    assert_eq!(run("x*[[1,2],[3,4]]"), "[ x, 2 · x ; 3 · x, 4 · x ]");
    assert!(
        UraeSession::new()
            .execute_line("[[1,2],[3,4]] * [[1,2,3]]")
            .is_error
    );
}

#[test]
fn infinite_sums_agree_with_partial_sums() {
    // The oracle: the partial sum to N (with the average of two consecutive ones for alternating series).
    type Term = fn(f64) -> f64;
    let cases: [(&str, i64, Term, bool); 8] = [
        ("1/(k*(k+1))", 1, |k| 1.0 / (k * (k + 1.0)), false),
        ("1/(k*(k+2))", 1, |k| 1.0 / (k * (k + 2.0)), false),
        ("1/(k^2-1)", 2, |k| 1.0 / (k * k - 1.0), false),
        (
            "1/((k)*(k+1)*(k+2))",
            1,
            |k| 1.0 / (k * (k + 1.0) * (k + 2.0)),
            false,
        ),
        (
            "(-1)^(k+1)/k",
            1,
            |k| (if k as i64 % 2 == 0 { -1.0 } else { 1.0 }) / k,
            true,
        ),
        (
            "(-1)^k/(2*k+1)",
            0,
            |k| (if k as i64 % 2 == 0 { 1.0 } else { -1.0 }) / (2.0 * k + 1.0),
            true,
        ),
        ("1/(2*k-1)^2", 1, |k| 1.0 / ((2.0 * k - 1.0).powi(2)), false),
        (
            "1/factorial(k)",
            0,
            |k| {
                if k > 170.0 {
                    0.0
                } else {
                    1.0 / (1..=(k as u64)).map(|i| i as f64).product::<f64>()
                }
            },
            false,
        ),
    ];
    for (summand, from, term, alternating) in cases {
        let out = run(&format!("sum({summand}, k, {from}, oo)"));
        let closed = value(&out.replace('π', "pi"), &[]);
        let n = 200_000;
        let mut partial = 0.0;
        let mut previous = 0.0;
        for k in from..from + n {
            previous = partial;
            partial += term(k as f64);
        }
        let estimate = if alternating {
            (partial + previous) / 2.0
        } else {
            partial
        };
        let tolerance = if alternating { 1e-9 } else { 2e-5 };
        assert!(
            (closed - estimate).abs() < tolerance,
            "sum({summand}) = {out}: {closed} vs {estimate}"
        );
    }
    assert_eq!(run("sum(x^k/factorial(k), k, 0, oo)"), "exp(x)");
    assert_eq!(run("sum(1/(k*(k+1)), k, 1, oo)"), "1");
    // Divergent series are errors, not values.
    assert!(
        UraeSession::new()
            .execute_line("sum(1/k, k, 1, oo)")
            .is_error
    );
}

#[test]
fn probability_and_geometry_formulas() {
    // Published values of the standard normal distribution, to 1e-12.
    for (x, expected) in [
        (0.0, 0.5),
        (1.0, 0.841_344_746_068_542_9),
        (1.96, 0.975_002_104_851_779_5),
        (-1.0, 0.158_655_253_931_457_05),
    ] {
        let out = run(&format!("N(normal_cdf({x}))"));
        assert!(
            (value(&format!("normal_cdf({x})"), &[]) - expected).abs() < 1e-12,
            "{out}"
        );
    }
    // The pdf integrates to the cdf (an independent numerical integral).
    let integral = run("nintegrate(normal_pdf(t), t, -9, 1.3)");
    let digits: String = integral
        .split('≈')
        .nth(1)
        .unwrap()
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    assert!((digits.parse::<f64>().unwrap() - value("normal_cdf(1.3)", &[])).abs() < 1e-9);
    // Binomial and Poisson masses sum to one; the binomial mean is n p.
    let n = 12;
    let masses: Vec<f64> = (0..=n)
        .map(|k| value(&format!("binomial_pmf({n}, 3/10, {k})"), &[]))
        .collect();
    assert!((masses.iter().sum::<f64>() - 1.0).abs() < 1e-12);
    assert!(
        (masses
            .iter()
            .enumerate()
            .map(|(k, m)| k as f64 * m)
            .sum::<f64>()
            - 3.6)
            .abs()
            < 1e-12
    );
    let poisson: f64 = (0..80)
        .map(|k| value(&format!("poisson_pmf({k}, 4)"), &[]))
        .sum();
    assert!((poisson - 1.0).abs() < 1e-12);
    assert_eq!(run("binomial_pmf(5, 1/2, 2)"), "5/16");
    assert_eq!(run("poisson_pmf(3, 2)"), "4/3 · exp(-2)");
    // Geometry.
    assert_eq!(run("circle_area(3)"), "9 · π");
    assert_eq!(run("sphere_volume(3)"), "36 · π");
    assert_eq!(run("hypot(3,4)"), "5");
    assert_eq!(run("deg2rad(180)"), "π");
    assert_eq!(run("rad2deg(pi)"), "180");
    assert_eq!(run("circle_area(r)"), "π · r^2");
}

#[test]
fn order_statistics_moments_and_point_geometry() {
    // Type-7 quantiles, central moments, against direct f64 computation on several data sets.
    let sets: [&[f64]; 4] = [
        &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        &[7.0, 1.0, 9.0, 3.0, 3.0, 12.0, 20.0],
        &[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0],
        &[10.0, 20.0, 30.0],
    ];
    let number = |out: &str| -> f64 {
        let tail = out
            .rsplit_once("≈")
            .map(|(_, t)| t)
            .unwrap_or_else(|| out.rsplit_once('=').unwrap().1);
        tail.trim().parse::<f64>().unwrap_or_else(|_| {
            let (a, b) = tail.trim().split_once('/').unwrap();
            a.parse::<f64>().unwrap() / b.parse::<f64>().unwrap()
        })
    };
    for data in sets {
        let text = format!(
            "[{}]",
            data.iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        let mut sorted = data.to_vec();
        sorted.sort_by(f64::total_cmp);
        let quantile = |q: f64| {
            let pos = q * (sorted.len() - 1) as f64;
            let (lo, hi) = (pos.floor() as usize, pos.ceil() as usize);
            sorted[lo] + (sorted[hi] - sorted[lo]) * (pos - lo as f64)
        };
        for p in [10.0, 25.0, 50.0, 75.0, 90.0] {
            let got = number(&run(&format!("percentile {text}, {p}")));
            assert!(
                (got - quantile(p / 100.0)).abs() < 1e-9,
                "percentile {p} of {text}: {got}"
            );
        }
        let n = data.len() as f64;
        let mean = data.iter().sum::<f64>() / n;
        let moment = |k: i32| data.iter().map(|v| (v - mean).powi(k)).sum::<f64>() / n;
        let (m2, m3, m4) = (moment(2), moment(3), moment(4));
        assert!(
            (number(&run(&format!("skewness {text}"))) - m3 / m2.powf(1.5)).abs() < 1e-9,
            "skewness {text}"
        );
        assert!(
            (number(&run(&format!("kurtosis {text}"))) - (m4 / (m2 * m2) - 3.0)).abs() < 1e-9,
            "kurtosis {text}"
        );
    }
    assert_eq!(
        run("quartiles [1,2,3,4,5,6]"),
        "quartiles = Q1 = 9/4, Q2 (median) = 7/2, Q3 = 19/4, IQR = 5/2"
    );
    // Points written as tuples or lists.
    assert_eq!(run("distance((0,0),(3,4))"), "5");
    assert_eq!(run("distance([1,2,3],[4,6,3])"), "5");
    assert_eq!(run("distance((1,1,1),(2,3,3))"), "3");
    assert_eq!(run("midpoint((0,0),(2,4))"), "[ 1, 2 ]");
    assert_eq!(run("slope((1,2),(3,8))"), "3");
    assert_eq!(run("angle([1,0],[0,1])"), "π / 2");
    // The triangle's area by the shoelace formula, for lattice triangles.
    for (a, b, c) in [
        ((0i64, 0i64), (4i64, 0i64), (0i64, 3i64)),
        ((1, 1), (5, 2), (3, 7)),
        ((-2, 3), (4, -1), (0, 0)),
    ] {
        let shoelace =
            ((a.0 * (b.1 - c.1) + b.0 * (c.1 - a.1) + c.0 * (a.1 - b.1)) as f64).abs() / 2.0;
        let out = run(&format!(
            "triangle_area(({},{}),({},{}),({},{}))",
            a.0, a.1, b.0, b.1, c.0, c.1
        ));
        assert!(
            (value(&out, &[]) - shoelace).abs() < 1e-12,
            "{out} vs {shoelace}"
        );
    }
}

#[test]
fn elementary_identities_are_applied_to_plain_expressions() {
    // Shown simplified, and the simplified form has the same value as the input at several points.
    let cases = [
        ("x + x", "2 · x"),
        ("1*x", "x"),
        ("sin(-x)", "-sin(x)"),
        ("tan(-x)", "-tan(x)"),
        ("abs(-x)", "abs(x)"),
        ("exp(x)*exp(-x)", "1"),
        ("exp(x)*exp(y)", "exp(x + y)"),
        ("(-b)^2", "b^2"),
        ("x^2*x^3", "x^5"),
        ("(x+y)^2 - x^2 - 2*x*y - y^2", "0"),
        ("a/b*b/a", "1"),
    ];
    for (input, expected) in cases {
        let out = run(input);
        assert_eq!(out, expected, "{input}");
        let shown = out.replace('·', "*");
        for (x, y, a, b) in [(0.7, -1.3, 2.1, -0.4), (-2.2, 0.5, 1.7, 3.3)] {
            let vars = [
                ("x", x.to_string()),
                ("y", y.to_string()),
                ("a", a.to_string()),
                ("b", b.to_string()),
            ];
            let vars: Vec<(&str, &str)> = vars.iter().map(|(n, v)| (*n, v.as_str())).collect();
            let (lhs, rhs) = (value(input, &vars), value(&shown, &vars));
            assert!(
                (lhs - rhs).abs() < 1e-9 * lhs.abs().max(1.0),
                "{input} = {out}: {lhs} vs {rhs}"
            );
        }
    }
}

#[test]
fn kron_diagonalize_and_singular_values() {
    let (a, b, c, d) = (
        vec![vec![1.0, 2.0], vec![3.0, 4.0]],
        vec![vec![0.0, 1.0], vec![1.0, 2.0]],
        vec![vec![2.0, 0.0], vec![1.0, 1.0]],
        vec![vec![1.0, -1.0], vec![0.0, 3.0]],
    );
    // kron against its definition, and the mixed-product property (A x B)(C x D) = AC x BD.
    let kron = |x: &[Vec<f64>], y: &[Vec<f64>]| -> Vec<Vec<f64>> {
        (0..x.len() * y.len())
            .map(|i| {
                (0..x[0].len() * y[0].len())
                    .map(|j| x[i / y.len()][j / y[0].len()] * y[i % y.len()][j % y[0].len()])
                    .collect()
            })
            .collect()
    };
    let shown = |m: &[Vec<f64>]| literal(m);
    assert_eq!(
        grid(&run(&format!("kron({}, {})", shown(&a), shown(&b)))),
        kron(&a, &b)
    );
    let left = mat_mul(&kron(&a, &b), &kron(&c, &d));
    let right = kron(&mat_mul(&a, &c), &mat_mul(&b, &d));
    assert_eq!(left, right);
    // diagonalize: A = P D P^-1 for matrices with rational eigenvalues.
    for m in [
        vec![vec![2.0, 1.0], vec![1.0, 2.0]],
        vec![vec![4.0, 1.0], vec![2.0, 3.0]],
        vec![vec![2.0, 0.0], vec![0.0, 3.0]],
    ] {
        let out = run(&format!("diagonalize({})", literal(&m)));
        let body = out.split("   [").next().unwrap();
        let (p_text, d_text) = body
            .trim_start_matches("P = ")
            .split_once(", D = ")
            .unwrap();
        let (p, dd) = (grid(p_text), grid(d_text));
        let inverse = grid(&run(&format!("{}^-1", literal(&p))));
        let rebuilt = mat_mul(&mat_mul(&p, &dd), &inverse);
        for i in 0..2 {
            for j in 0..2 {
                assert!((rebuilt[i][j] - m[i][j]).abs() < 1e-12, "{out}");
            }
        }
    }
    // A defective matrix is an error, not a wrong factorisation.
    assert!(
        UraeSession::new()
            .execute_line("diagonalize([[1,1],[0,1]])")
            .is_error
    );
    // Singular values: their product is |det A| and the sum of squares is the squared Frobenius norm.
    for m in [
        vec![vec![3.0, 0.0], vec![0.0, 4.0]],
        vec![vec![1.0, 2.0], vec![3.0, 4.0]],
        vec![vec![2.0, -1.0], vec![5.0, 7.0]],
    ] {
        let out = run(&format!("svd({})", literal(&m)));
        let list = out
            .trim_start_matches("singular values = ")
            .trim_matches(|ch| ch == '[' || ch == ']');
        let values: Vec<f64> = list
            .split(", ")
            .map(|s| value(&s.replace('√', "sqrt").replace('·', "*"), &[]))
            .collect();
        let det = m[0][0] * m[1][1] - m[0][1] * m[1][0];
        assert!(
            (values.iter().product::<f64>() - det.abs()).abs() < 1e-9,
            "{out}"
        );
        let frobenius: f64 = m.iter().flatten().map(|v| v * v).sum();
        assert!(
            (values.iter().map(|v| v * v).sum::<f64>() - frobenius).abs() < 1e-9,
            "{out}"
        );
        assert!(values[0] >= values[1]);
    }
}

#[test]
fn continuous_distributions_match_closed_forms_and_tables() {
    let v = |q: &str| value(q, &[]);
    // Closed forms of special cases.
    for x in [0.3f64, 1.0, 2.5, 7.0] {
        assert!((v(&format!("chi2_cdf({x}, 2)")) - (1.0 - (-x / 2.0).exp())).abs() < 1e-12);
        assert!((v(&format!("chi2_cdf({x}, 1)")) - v(&format!("erf(sqrt({x}/2))"))).abs() < 1e-12);
        assert!((v(&format!("gamma_cdf({x}, 1, 2)")) - (1.0 - (-x / 2.0).exp())).abs() < 1e-12);
        assert!((v(&format!("gamma_inc_p(1, {x})")) - (1.0 - (-x).exp())).abs() < 1e-12);
    }
    for x in [0.1f64, 0.5, 0.8] {
        // I_x(2, 3) = 6 x^2 - 8 x^3 + 3 x^4
        assert!(
            (v(&format!("beta_cdf({x}, 2, 3)"))
                - (6.0 * x * x - 8.0 * x.powi(3) + 3.0 * x.powi(4)))
            .abs()
                < 1e-12
        );
    }
    for t in [-2.0f64, -0.4, 0.0, 1.3, 3.0] {
        // Student t with 1 degree of freedom is the Cauchy distribution; with 2 it has a closed form.
        assert!(
            (v(&format!("t_cdf({t}, 1)")) - (0.5 + t.atan() / std::f64::consts::PI)).abs() < 1e-12
        );
        assert!(
            (v(&format!("t_cdf({t}, 2)")) - (0.5 + t / (2.0 * (2.0 + t * t).sqrt()))).abs() < 1e-12
        );
    }
    // Published critical values (to the digits of the tables).
    assert!((v("chi2_quantile(0.95, 1)") - 3.841_458_820_694_124).abs() < 1e-9);
    assert!((v("chi2_quantile(0.95, 10)") - 18.307_038_053_275_146).abs() < 1e-9);
    assert!((v("t_quantile(0.975, 10)") - 2.228_138_851_986_274).abs() < 1e-9);
    assert!((v("t_quantile(0.975, 30)") - 2.042_272_456_301_238).abs() < 1e-9);
    assert!((v("normal_quantile(0.975)") - 1.959_963_984_540_054).abs() < 1e-12);
    assert!(
        (v("normal_quantile(0.95, 100, 15)") - (100.0 + 15.0 * 1.644_853_626_951_472_7)).abs()
            < 1e-9
    );
    // Quantiles invert the cdf.
    for p in [0.01, 0.2, 0.5, 0.9, 0.999] {
        for k in [1, 3, 12] {
            assert!(
                (v(&format!("chi2_cdf(chi2_quantile({p}, {k}), {k})")) - p).abs() < 1e-10,
                "chi2 p = {p}, k = {k}"
            );
            assert!(
                (v(&format!("t_cdf(t_quantile({p}, {k}), {k})")) - p).abs() < 1e-10,
                "t p = {p}, k = {k}"
            );
        }
        assert!((v(&format!("normal_cdf(normal_quantile({p}))")) - p).abs() < 1e-12);
    }
    // A pdf integrates to the cdf (an independent numerical integral).
    let integral = run("nintegrate(t_pdf(s, 5), s, -40, 1.7)");
    let digits: String = integral
        .split('≈')
        .nth(1)
        .unwrap()
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    assert!(
        (digits.parse::<f64>().unwrap() - v("t_cdf(1.7, 5)")).abs() < 1e-6,
        "{integral}"
    );
}
