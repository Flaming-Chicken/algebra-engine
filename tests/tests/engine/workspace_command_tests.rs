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

#[test]
fn latex_and_english_input_mean_what_the_plain_input_means() {
    // Each LaTeX or English phrase gives exactly the result of the plain command it stands for.
    let pairs: [(&str, &str); 28] = [
        (r"\frac{1}{2} + \frac{1}{3}", "1/2 + 1/3"),
        (r"\int_0^1 x^2 dx", "integrate(x^2, x, 0, 1)"),
        (r"\int x^2 dx", "integrate(x^2, x)"),
        (r"\int_{0}^{\pi} \sin(x) dx", "integrate(sin(x), x, 0, pi)"),
        (r"\sum_{k=1}^{10} k", "sum(k, k, 1, 10)"),
        (r"\sum_{k=1}^{\infty} \frac{1}{k^2}", "sum(1/k^2, k, 1, oo)"),
        (r"\prod_{k=1}^{5} k", "product(k, k, 1, 5)"),
        (r"\lim_{x \to 0} \frac{\sin x}{x}", "limit(sin(x)/x, x, 0)"),
        (r"\lim_{x \to 0^+} \frac{1}{x}", "limit(1/x, x, 0, +)"),
        (r"\sqrt{16}", "sqrt(16)"),
        (r"\sqrt[3]{27}", "root(27, 3)"),
        (r"\frac{d}{dx} x^3", "diff(x^3, x)"),
        (r"\frac{d^2}{dx^2} x^4", "diff(x^4, x, 2)"),
        (r"\sin^2 x + \cos^2 x", "sin(x)^2 + cos(x)^2"),
        (r"\binom{5}{2}", "binomial(5, 2)"),
        (r"\log_2 8", "log(8, 2)"),
        (r"\ln e", "ln(e)"),
        (r"\pi r^2", "pi*r^2"),
        (
            r"\det \begin{pmatrix} 1 & 2 \\ 3 & 4 \end{pmatrix}",
            "det([[1,2],[3,4]])",
        ),
        ("what is 2+2", "2+2"),
        ("calculate 3*4", "3*4"),
        ("2 plus 3", "2 + 3"),
        ("10 divided by 4", "10 / 4"),
        ("square root of 16", "sqrt(16)"),
        ("factorial of 5", "(5)!"),
        ("3 squared", "3^2"),
        ("2 to the power 10", "2^10"),
        ("20 percent of 150", "(20)/100*(150)"),
    ];
    for (phrase, plain) in pairs {
        assert_eq!(run(phrase), run(plain), "{phrase} vs {plain}");
    }
    // The exact values, so that both sides are not equally wrong.
    assert_eq!(run(r"\frac{1}{2} + \frac{1}{3}"), "5/6");
    assert_eq!(run(r"\int_0^1 x^2 dx"), "1/3 (w.r.t x along [0, 1])");
    assert_eq!(run(r"\sum_{k=1}^{10} k"), "55");
    assert_eq!(run(r"\binom{5}{2}"), "10");
    assert_eq!(run(r"\log_2 8"), "3");
    assert_eq!(run("what is 2+2"), "4");
    assert_eq!(run("square root of 16"), "4");
    assert_eq!(run("20 percent of 150"), "30");
    // Case is kept: a symbol named X is not turned into x.
    assert_eq!(run("X plus X"), "2 · X");
    // `e^{i\pi}` was read through a backslash operator (`mldivide`).
    assert!(!run(r"e^{i\pi}").contains("mldivide"));
}

#[test]
fn euler_power_and_the_scope_of_a_sum() {
    // e^x is exp(x): exp(i pi) = -1 holds for both spellings (e^(i pi) was printed unevaluated).
    assert_eq!(run("e^(i*pi)"), "-1");
    assert_eq!(run("exp(i*pi)"), "-1");
    assert_eq!(run("e^(i*pi/2)"), "i");
    assert_eq!(run("e^(ln(2))"), "2");
    assert_eq!(run("e^(-x)*e^x"), "1");
    assert_eq!(run("diff(e^(x^2), x)"), run("diff(exp(x^2), x)"));
    // A sum applies to the next term only: sum k + 1 is 10 + 1, sum k^2 - 1 is 30 - 1.
    assert_eq!(run(r"\sum_{k=1}^{4} k + 1"), "11");
    assert_eq!(run(r"\sum_{k=1}^{4} k^2 - 1"), "29");
    assert_eq!(run(r"\sum_{k=1}^{4} 2k"), "20");
    assert_eq!(run("sum(k, k, 1, 4) + 1"), "11");
    assert_eq!(run("2*sum(k, k, 1, 4)"), "20");
    // The whole-input call is not treated as a nested one (this recursed without end).
    assert_eq!(run("simplify(x - x)"), "0");
}

#[test]
fn series_at_infinity_agree_with_the_function_for_large_x() {
    // The oracle: at a large x the truncated expansion equals the function up to the first omitted term (~ x^-(n+1)).
    let cases = [
        ("1/(x+1)", 6),
        ("1/(x^2+1)", 6),
        ("x/(x+1)", 6),
        ("(x+1)/(x-1)", 6),
        ("x^2/(x+1)", 5),
        ("exp(1/x)", 6),
        ("sin(1/x)", 7),
        ("ln(1+1/x)", 6),
        ("(x^2+3)/(x^2-1)", 6),
    ];
    for (expression, order) in cases {
        let series = run(&format!("series({expression}, x, oo, {order})"));
        assert!(!series.contains("(x^(-1))"), "unsimplified: {series}");
        for x in ["30", "55.5", "-40"] {
            let (f, s) = (value(expression, &[("x", x)]), value(&series, &[("x", x)]));
            assert!(
                (f - s).abs() < 1e-8 * f.abs().max(1.0),
                "series({expression}, oo) = {series}: {f} vs {s} at x = {x}"
            );
        }
    }
    assert_eq!(
        run("series(1/(x+1), x, oo, 4)"),
        "x^(-1) - x^(-2) + x^(-3) - x^(-4)"
    );
}

#[test]
fn linear_recurrences_have_closed_forms_that_reproduce_the_sequence() {
    // Each case: the recurrence text with its initial values, and the same recurrence iterated here in f64.
    type Step = fn(&[f64], usize) -> f64;
    type Case<'a> = (&'a str, &'a [(i64, f64)], Step);
    let cases: [Case; 10] = [
        ("a(n+2) = a(n+1) + a(n)", &[(0, 0.0), (1, 1.0)], |a, _| {
            a[a.len() - 1] + a[a.len() - 2]
        }),
        ("a(n+1) = 2*a(n) + 1", &[(0, 0.0)], |a, _| {
            2.0 * a[a.len() - 1] + 1.0
        }),
        (
            "a(n+2) = 5*a(n+1) - 6*a(n)",
            &[(0, 1.0), (1, 0.0)],
            |a, _| 5.0 * a[a.len() - 1] - 6.0 * a[a.len() - 2],
        ),
        (
            "a(n+2) = 4*a(n+1) - 4*a(n)",
            &[(0, 1.0), (1, 3.0)],
            |a, _| 4.0 * a[a.len() - 1] - 4.0 * a[a.len() - 2],
        ),
        ("a(n+1) = a(n) + n", &[(0, 0.0)], |a, i| {
            a[a.len() - 1] + (i as f64 - 1.0)
        }),
        ("a(n+1) = 2*a(n) + 3^n", &[(0, 1.0)], |a, i| {
            2.0 * a[a.len() - 1] + 3f64.powi(i as i32 - 1)
        }),
        ("a(n+1) = 2*a(n) + 2^n", &[(0, 1.0)], |a, i| {
            2.0 * a[a.len() - 1] + 2f64.powi(i as i32 - 1)
        }),
        ("a(n+2) = 2*a(n+1) - a(n)", &[(0, 1.0), (1, 3.0)], |a, _| {
            2.0 * a[a.len() - 1] - a[a.len() - 2]
        }),
        ("a(n+2) = -a(n)", &[(0, 1.0), (1, 0.0)], |a, _| {
            -a[a.len() - 2]
        }),
        (
            "a(n+3) = 6*a(n+2) - 11*a(n+1) + 6*a(n)",
            &[(0, 0.0), (1, 1.0), (2, 2.0)],
            |a, _| 6.0 * a[a.len() - 1] - 11.0 * a[a.len() - 2] + 6.0 * a[a.len() - 3],
        ),
    ];
    for (recurrence, initial, step) in cases {
        let conditions: Vec<String> = initial.iter().map(|(i, v)| format!("a({i})={v}")).collect();
        let out = run(&format!("rsolve {recurrence}, {}", conditions.join(", ")));
        let closed = out
            .strip_prefix("a(n) = ")
            .unwrap()
            .replace('·', "*")
            .replace('√', "sqrt");
        let mut terms: Vec<f64> = initial.iter().map(|(_, v)| *v).collect();
        while terms.len() < 16 {
            let next = step(&terms, terms.len());
            terms.push(next);
        }
        for (n, expected) in terms.iter().enumerate() {
            let got = value(&closed, &[("n", &n.to_string())]);
            assert!(
                (got - expected).abs() < 1e-6 * expected.abs().max(1.0),
                "{recurrence}: a({n}) = {got}, iterated {expected} ({out})"
            );
        }
    }
    // Exact forms.
    assert_eq!(run("rsolve a(n+1) = 3*a(n), a(0)=2"), "a(n) = 2 · 3^n");
    assert_eq!(run("rsolve(a(n) = 2*a(n-1) + 1, a(0)=0)"), "a(n) = 2^n - 1");
    assert_eq!(run("rsolve a(n+1) = a(n) + 2*n + 1, a(0)=0"), "a(n) = n^2");
    assert_eq!(
        run("rsolve a(n+2) = 2*a(n+1) - a(n), a(0)=1, a(1)=3"),
        "a(n) = 2 · n + 1"
    );
    // Too few initial values is an error.
    assert!(
        UraeSession::new()
            .execute_line("rsolve a(n+2) = a(n+1) + a(n), a(0)=0")
            .is_error
    );
}

#[test]
fn sequence_rules_predict_the_true_continuation() {
    // For each sequence, the next five terms printed by the finder equal the terms computed here from the true rule.
    type Rule = fn(usize) -> i64;
    let polynomial_cases: [(&str, Rule); 6] = [
        ("squares", |n| (n * n) as i64),
        ("cubes", |n| (n * n * n) as i64),
        ("triangular", |n| (n * (n + 1) / 2) as i64),
        ("odd", |n| 2 * n as i64 - 1),
        ("quartic", |n| {
            (n * n * n * n) as i64 - 3 * (n * n) as i64 + 7
        }),
        ("shifted", |n| 5 * n as i64 + 11),
    ];
    for (name, rule) in polynomial_cases {
        let given: Vec<String> = (1..=7).map(|n| rule(n).to_string()).collect();
        let out = run(&format!("next_term {}", given.join(", ")));
        let next: Vec<i64> = out
            .rsplit("next terms: ")
            .next()
            .unwrap()
            .split(", ")
            .map(|s| s.parse().unwrap())
            .collect();
        assert_eq!(
            next,
            (8..=12).map(rule).collect::<Vec<_>>(),
            "{name}: {out}"
        );
    }
    // Linear recurrences with constant coefficients, from 2r + 1 terms (one confirming term).
    type Recurrence<'a> = (&'a str, Vec<i64>, Vec<i64>);
    let recurrences: [Recurrence; 5] = [
        ("fibonacci", vec![1, 1], vec![1, 1]),
        ("lucas", vec![2, 1], vec![1, 1]),
        ("affine", vec![1], vec![3]),
        ("pell", vec![0, 1], vec![1, 2]),
        ("third order", vec![1, 2, 4], vec![1, 0, 1]),
    ];
    for (name, start, coefficients) in recurrences {
        let order = coefficients.len();
        // The affine rule a(n) = 3 a(n-1) + 1 satisfies a homogeneous recurrence of order 2.
        let minimal_order = order + usize::from(name == "affine");
        let mut terms = start.clone();
        while terms.len() < 2 * minimal_order + 6 {
            let n = terms.len();
            let value: i64 = (0..order)
                .map(|i| coefficients[i] * terms[n - 1 - i])
                .sum::<i64>()
                + if name == "affine" { 1 } else { 0 };
            terms.push(value);
        }
        let given = &terms[..terms.len() - 5];
        let text: Vec<String> = given.iter().map(i64::to_string).collect();
        let out = run(&format!("find_pattern {}", text.join(", ")));
        let next: Vec<i64> = out
            .rsplit("next terms: ")
            .next()
            .unwrap()
            .split(", ")
            .map(|s| s.parse().unwrap())
            .collect();
        assert_eq!(next, terms[terms.len() - 5..].to_vec(), "{name}: {out}");
    }
    // Nothing is invented for the primes.
    assert!(
        UraeSession::new()
            .execute_line("find_pattern 2, 3, 5, 7, 11, 13, 17, 19")
            .is_error
    );
    assert!(run("next_term 1, 4, 9, 16").starts_with("a(n) = n^2"));
}

#[test]
fn linear_programs_match_vertex_enumeration() {
    // The oracle is a different algorithm: enumerate every intersection of two constraint lines (the axes included),
    // keep the feasible points, and take the best objective. Deterministic pseudo-random small programs.
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = move |modulus: i64| -> i64 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % modulus as u64) as i64
    };
    for case in 0..25 {
        let constraint_count = 3 + next(2) as usize;
        let mut lines: Vec<(f64, f64, f64)> = Vec::new(); // a x + b y <= c
        let mut text = Vec::new();
        for _ in 0..constraint_count {
            let (a, b, c) = (1 + next(5), 1 + next(5), 5 + next(20));
            lines.push((a as f64, b as f64, c as f64));
            text.push(format!("{a}*x + {b}*y <= {c}"));
        }
        let (p, q) = (1 + next(6), 1 + next(6));
        let maximise = case % 2 == 0;
        let objective = format!("{p}*x + {q}*y");
        // Candidate boundaries: the constraint lines and the two axes.
        let mut boundaries = lines.clone();
        boundaries.push((1.0, 0.0, 0.0));
        boundaries.push((0.0, 1.0, 0.0));
        let feasible = |x: f64, y: f64| {
            x >= -1e-9 && y >= -1e-9 && lines.iter().all(|(a, b, c)| a * x + b * y <= c + 1e-9)
        };
        let mut best: Option<f64> = None;
        for i in 0..boundaries.len() {
            for j in i + 1..boundaries.len() {
                let (a1, b1, c1) = boundaries[i];
                let (a2, b2, c2) = boundaries[j];
                let det = a1 * b2 - a2 * b1;
                if det.abs() < 1e-12 {
                    continue;
                }
                let (x, y) = ((c1 * b2 - c2 * b1) / det, (a1 * c2 - a2 * c1) / det);
                if feasible(x, y) {
                    let value = p as f64 * x + q as f64 * y;
                    best = Some(match best {
                        None => value,
                        Some(b) => {
                            if maximise {
                                b.max(value)
                            } else {
                                b.min(value)
                            }
                        }
                    });
                }
            }
        }
        let command = format!(
            "{} {objective} subject to {}",
            if maximise { "maximize" } else { "minimize" },
            text.join(", ")
        );
        let out = run(&command);
        let got: f64 = out
            .split("value = ")
            .nth(1)
            .unwrap()
            .split(" at ")
            .next()
            .map(|s| match s.split_once('/') {
                Some((a, b)) => a.parse::<f64>().unwrap() / b.parse::<f64>().unwrap(),
                None => s.parse().unwrap(),
            })
            .unwrap();
        assert!(
            (got - best.unwrap()).abs() < 1e-9,
            "{command}: {out} vs {best:?}"
        );
    }
    assert_eq!(
        run("maximize 3*x + 2*y subject to x + y <= 4, x + 3*y <= 6"),
        "maximum value = 12 at x = 4, y = 0"
    );
    assert!(run("maximize x + y subject to x - y <= 1").starts_with("unbounded"));
    assert!(run("maximize x subject to x <= 3, x >= 5").starts_with("infeasible"));
    assert_eq!(
        run("minimize x + y subject to x + 2*y >= 4, 3*x + y >= 6"),
        "minimum value = 14/5 at x = 8/5, y = 6/5"
    );
}

#[test]
fn stationary_points_and_quadrature_rules_match_their_oracles() {
    // f' = k (x - r1)(x - r2)(x - r3) with distinct integer roots: the critical points are exactly the r_i and, for
    // k > 0, the smallest and largest are minima and the middle one a maximum (the signs flip for k < 0).
    for (k, roots) in [
        (1i64, [-2i64, 0, 3]),
        (2, [-1, 1, 4]),
        (-1, [-3, -1, 2]),
        (4, [0, 2, 5]),
    ] {
        let [r1, r2, r3] = roots;
        // f = k * integral of (x-r1)(x-r2)(x-r3) = k * (x^4/4 - s1 x^3/3 + s2 x^2/2 - s3 x)
        let s1 = r1 + r2 + r3;
        let s2 = r1 * r2 + r1 * r3 + r2 * r3;
        let s3 = r1 * r2 * r3;
        let f = format!("{k}*(x^4/4 - {s1}*x^3/3 + {s2}*x^2/2 - {s3}*x)");
        let out = run(&format!("extrema({f})"));
        for (i, r) in roots.iter().enumerate() {
            let outer = i != 1;
            let kind = if outer == (k > 0) {
                "local minimum"
            } else {
                "local maximum"
            };
            let part = format!("x = {r}: {kind}");
            assert!(out.contains(&part), "{f}: expected `{part}` in `{out}`");
        }
        assert_eq!(out.matches("x = ").count(), 3, "{out}");
    }
    assert_eq!(run("critical_points(x^3-3x)"), "critical points (x): 1, -1");
    // Quadrature rules against closed forms: Simpson is exact for cubics, the trapezoid error is -(b-a)h^2 f''/12,
    // and Simpson on sin over [0, pi] with n = 10 is the published 2.0001095173.
    let value = |text: String| -> f64 { text.rsplit(": ").next().unwrap().trim().parse().unwrap() };
    assert!((value(run("simpson(x^3 - 2*x, 0, 2, 2)")) - 0.0).abs() < 1e-12);
    assert!((value(run("simpson(sin(x), 0, pi, 10)")) - 2.0001095173150043).abs() < 1e-12);
    let n = 100.0;
    let trapezoid = value(run("trapezoid(x^2, 0, 1, 100)"));
    assert!(
        (trapezoid - (1.0 / 3.0 + 1.0 / (6.0 * n * n))).abs() < 1e-12,
        "{trapezoid}"
    );
    let midpoint = value(run("midpoint(x^2, 0, 1, 100)"));
    assert!(
        (midpoint - (1.0 / 3.0 - 1.0 / (12.0 * n * n))).abs() < 1e-12,
        "{midpoint}"
    );
    assert!(run("newton_method(x^2 - 2, 1)").contains("1.41421356237309"));
    assert!(run("bisection(x^3 - x - 2, 1, 2)").contains("1.52137970680"));
}

#[test]
fn complex_functions_and_transforms_match_their_definitions() {
    // Parse "a", "bi", "a + bi", "a - bi" back into (re, im).
    fn parse_complex(text: &str) -> (f64, f64) {
        let t = text.trim();
        if let Some(body) = t.strip_suffix('i') {
            // Split at the last sign that is not the first character.
            if let Some(at) = body
                .rfind([' '])
                .and_then(|_| body.rfind(" + ").or(body.rfind(" - ")))
            {
                let re: f64 = body[..at].trim().parse().unwrap();
                let sign = if &body[at..at + 3] == " - " {
                    -1.0
                } else {
                    1.0
                };
                let im: f64 = body[at + 3..].trim().parse().unwrap();
                return (re, sign * im);
            }
            let im: f64 = if body.is_empty() {
                1.0
            } else if body == "-" {
                -1.0
            } else {
                body.parse().unwrap()
            };
            return (0.0, im);
        }
        (t.parse().unwrap(), 0.0)
    }
    fn parse_list(text: &str) -> Vec<(f64, f64)> {
        text.trim_start_matches('[')
            .trim_end_matches(']')
            .split(", ")
            .map(parse_complex)
            .collect()
    }
    // fft / dft against the defining sum X_k = sum_j x_j e^{-2 pi i jk/n}, for lengths 1..=9 (powers of two and not).
    let mut state = 0xD1B5_4A32_D192_ED03u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % 19) as i64 - 9
    };
    for n in 1..=9usize {
        let x: Vec<i64> = (0..n).map(|_| next()).collect();
        let literal = format!(
            "[{}]",
            x.iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
        for name in ["fft", "dft"] {
            let got = parse_list(&run(&format!("{name}({literal})")));
            assert_eq!(got.len(), n);
            for (k, g) in got.iter().enumerate() {
                let (mut re, mut im) = (0.0f64, 0.0f64);
                for (j, v) in x.iter().enumerate() {
                    let a = -2.0 * std::f64::consts::PI * ((j * k) % n) as f64 / n as f64;
                    re += *v as f64 * a.cos();
                    im += *v as f64 * a.sin();
                }
                assert!(
                    (g.0 - re).abs() < 1e-9 && (g.1 - im).abs() < 1e-9,
                    "{name}{literal}[{k}]: {g:?} vs ({re}, {im})"
                );
            }
        }
        // Parseval: sum |X_k|^2 = n sum |x_j|^2; and the inverse recovers the input.
        let spectrum = parse_list(&run(&format!("fft({literal})")));
        let energy: f64 = spectrum.iter().map(|c| c.0 * c.0 + c.1 * c.1).sum();
        let direct: f64 = x.iter().map(|v| (*v * *v) as f64).sum::<f64>() * n as f64;
        assert!(
            (energy - direct).abs() < 1e-6 * (1.0 + direct),
            "Parseval {literal}"
        );
        let back = parse_list(&run(&format!("ifft(fft({literal}))")));
        for (b, v) in back.iter().zip(&x) {
            assert!(
                (b.0 - *v as f64).abs() < 1e-9 && b.1.abs() < 1e-9,
                "ifft(fft({literal}))"
            );
        }
    }
    // convolve is polynomial multiplication: (1 + 2x + 3x^2)(1 - x) has coefficients [1, 1, 1, -3].
    assert_eq!(run("convolve([1, 2, 3], [1, -1])"), "[1, 1, 1, -3]");
    assert_eq!(run("convolve([2, 0, 1], [3])"), "[6, 0, 3]");
    // arg, modulus and polar.
    for (z, expected) in [
        ("1", "0"),
        ("i", "π/2"),
        ("-1", "π"),
        ("-i", "-π/2"),
        ("1+i", "π/4"),
        ("-1+i", "3*π/4"),
        ("-1-i", "-3*π/4"),
        ("1-i", "-π/4"),
        ("sqrt(3)+i", "π/6"),
        ("1+sqrt(3)*i", "π/3"),
    ] {
        assert_eq!(run(&format!("arg({z})")), expected, "arg({z})");
    }
    let numeric: f64 = run("arg(3+4i)").parse().unwrap();
    assert!((numeric - 4.0f64.atan2(3.0)).abs() < 1e-12);
    for (z, r) in [
        ("3+4i", "5"),
        ("5+12i", "13"),
        ("-8+15i", "17"),
        ("1+i", "√(2)"),
    ] {
        assert_eq!(run(&format!("modulus({z})")), r, "modulus({z})");
    }
    assert_eq!(run("polar(1+i)"), "√(2)·e^(i·π/4)");
    assert_eq!(run("atan2(1, -1)"), "3*π/4");
}

#[test]
fn residues_and_poles_match_closed_forms() {
    // Known residues.
    assert_eq!(run("residue(1/(x^2-1), x, 1)"), "1/2");
    assert_eq!(run("residue(1/(x^2-1), x, -1)"), "-1/2");
    assert_eq!(run("residue(1/(x^2+1), x, i)"), "-1/2·i");
    assert_eq!(run("residue(exp(x)/x^3, x, 0)"), "1/2");
    assert_eq!(run("residue(cot(x), x, 0)"), "1");
    assert_eq!(run("residue(1/(x^2*(x-1)), x, 0)"), "-1");
    assert_eq!(run("residue(x^2+3, x, 0)"), "0");
    // Simple poles r_k of 1 / prod (x - r_k): the residue at r_k is 1 / prod_{j != k} (r_k - r_j), and for
    // three or more poles the residues sum to zero.
    for roots in [[1i64, 2, 4], [-3, 0, 5], [2, 3, 7]] {
        let f = format!(
            "1/((x-({}))*(x-({}))*(x-({})))",
            roots[0], roots[1], roots[2]
        );
        let mut total = 0.0f64;
        for (k, r) in roots.iter().enumerate() {
            let denominator: i64 = roots
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != k)
                .map(|(_, o)| r - o)
                .product();
            let out = run(&format!("residue({f}, x, {r})"));
            let got: f64 = match out.split_once('/') {
                Some((a, b)) => a.parse::<f64>().unwrap() / b.parse::<f64>().unwrap(),
                None => out.parse().unwrap(),
            };
            assert!(
                (got - 1.0 / denominator as f64).abs() < 1e-9,
                "{f} at {r}: {out}"
            );
            total += got;
        }
        assert!(total.abs() < 1e-9, "{f}: residues sum to {total}");
    }
    // Poles with their orders.
    assert_eq!(run("poles(1/(x^2-1))"), "x = 1 (order 1), x = -1 (order 1)");
    assert_eq!(
        run("poles((x+1)/(x^2*(x-3)))"),
        "x = 0 (order 2), x = 3 (order 1)"
    );
    // abs of a complex constant is its modulus.
    assert_eq!(run("abs(1+i)"), "√(2)");
    assert_eq!(run("abs(3+4i)"), "5");
    assert_eq!(run("abs(-3)"), "3");
}

#[test]
fn fixed_step_ode_methods_converge_at_their_orders() {
    let value = |text: String| -> f64 {
        text.split(" ≈ ")
            .nth(1)
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    // y' = y, y(0) = 1: Euler is exactly (1 + h)^n, and the error of each method falls like h^p.
    assert!((value(run("euler_method(y, 0, 1, 1, 10)")) - 1.1f64.powi(10)).abs() < 1e-12);
    let exact = std::f64::consts::E;
    for (method, order) in [
        ("euler_method", 1.0f64),
        ("heun", 2.0),
        ("midpoint_method", 2.0),
        ("rk4", 4.0),
    ] {
        let error = |n: u32| (value(run(&format!("{method}(y, 0, 1, 1, {n})"))) - exact).abs();
        let rate = (error(20) / error(40)).log2();
        assert!(
            (rate - order).abs() < 0.2,
            "{method}: observed order {rate}, expected {order}"
        );
    }
    // y' = x + y, y(0) = 1 has the solution 2 e^x - x - 1; the second spelling names the unknown function.
    let solution = 2.0 * std::f64::consts::E - 2.0;
    assert!((value(run("rk4(x + y, 0, 1, 1, 200)")) - solution).abs() < 1e-9);
    assert!((value(run("rk4(y' = x + y, y(0) = 1, 1, 200)")) - solution).abs() < 1e-9);
    // y' = -2 t y, y(0) = 1: the solution is exp(-t^2); the independent variable may be t.
    assert!((value(run("rk4(-2*t*y, 0, 1, 1, 200)")) - (-1.0f64).exp()).abs() < 1e-9);
    assert!(
        UraeSession::new()
            .execute_line("rk4(y, 0, 1, 1, 0)")
            .is_error
    );
}

#[test]
fn constrained_extrema_match_closed_forms() {
    let number = |text: &str| -> f64 {
        let v = text
            .split(" = ")
            .nth(1)
            .unwrap()
            .split(" at ")
            .next()
            .unwrap();
        match v.split_once('/') {
            Some((a, b)) => a.parse::<f64>().unwrap() / b.parse::<f64>().unwrap(),
            None => v.parse().unwrap(),
        }
    };
    // max x*y on x + y = s is s^2 / 4, at x = y = s / 2 (AM-GM).
    for s in [4i64, 10, 7] {
        let out = run(&format!("maximize x*y subject to x + y = {s}"));
        assert!((number(&out) - (s * s) as f64 / 4.0).abs() < 1e-9, "{out}");
        assert!(out.starts_with("local maximum"), "{out}");
        // x*y has no minimum on the line: the stationary point is a maximum, so a minimum request is refused.
        assert!(
            UraeSession::new()
                .execute_line(&format!("minimize x*y subject to x + y = {s}"))
                .is_error
        );
    }
    // min x^2 + y^2 on a x + b y = c is c^2 / (a^2 + b^2) (squared distance from the origin to the line).
    for (a, b, c) in [(1i64, 1i64, 1i64), (3, 4, 10), (2, -1, 5)] {
        let out = run(&format!(
            "minimize x^2 + y^2 subject to {a}*x + {b}*y = {c}"
        ));
        let expected = (c * c) as f64 / (a * a + b * b) as f64;
        assert!(
            (number(&out) - expected).abs() < 1e-6,
            "{out} vs {expected}"
        );
    }
    // The extremes of x + y on the unit circle are +-sqrt(2); of x*y they are +-1/2; AM-GM for three variables.
    let out = run("maximize x + y subject to x^2 + y^2 = 1");
    assert!((number(&out) - 2f64.sqrt()).abs() < 1e-5, "{out}");
    let out = run("minimize x + y subject to x^2 + y^2 = 1");
    assert!((number(&out) + 2f64.sqrt()).abs() < 1e-5, "{out}");
    assert!((number(&run("maximize x*y subject to x^2 + y^2 = 1")) - 0.5).abs() < 1e-5);
    assert!((number(&run("maximize x*y*z subject to x + y + z = 3")) - 1.0).abs() < 1e-6);
    // Inequality constraints with a nonlinear objective are not supported, and linear programs still work.
    assert!(
        UraeSession::new()
            .execute_line("maximize x*y subject to x + y <= 4")
            .is_error
    );
    assert_eq!(
        run("maximize 3*x + 2*y subject to x + y <= 4, x + 3*y <= 6"),
        "maximum value = 12 at x = 4, y = 0"
    );
    // extrema of a function the exact solver cannot factor: x^2 e^-x has a minimum at 0 and a maximum 4 e^-2 at 2.
    let out = run("extrema(x^2*exp(-x))");
    assert!(
        out.contains("x = 0: local minimum") && out.contains("x = 2: local maximum"),
        "{out}"
    );
    let top: f64 = out.rsplit("f = ").next().unwrap().trim().parse().unwrap();
    assert!((top - 4.0 * (-2.0f64).exp()).abs() < 1e-9, "{out}");
}

#[test]
fn combinatorial_spellings_match_enumeration() {
    let int = |q: &str| -> i64 { run(q).parse().unwrap() };
    // Compositions of n into k positive parts, counted by brute force over all parts lists.
    fn count(n: i64, k: i64) -> i64 {
        if k == 0 {
            return (n == 0) as i64;
        }
        (1..=n).map(|first| count(n - first, k - 1)).sum()
    }
    for n in 1..=9i64 {
        let total: i64 = (1..=n).map(|k| count(n, k)).sum();
        assert_eq!(
            int(&format!("compositions({n})")),
            total,
            "compositions({n})"
        );
        for k in 1..=n {
            assert_eq!(
                int(&format!("compositions({n}, {k})")),
                count(n, k),
                "compositions({n}, {k})"
            );
        }
    }
    // Tribonacci by its recurrence T(n) = T(n-1) + T(n-2) + T(n-3), T(0) = T(1) = 0, T(2) = 1.
    let mut t = vec![0i64, 0, 1];
    for n in 3..=30 {
        t.push(t[n - 1] + t[n - 2] + t[n - 3]);
    }
    for (n, v) in t.iter().enumerate() {
        assert_eq!(int(&format!("tribonacci({n})")), *v);
    }
    // nCr / nPr agree with choose / perm and with Pascal's rule; prime_pi with primepi.
    for n in 1..=12i64 {
        for k in 1..n {
            assert_eq!(
                int(&format!("nCr({n}, {k})")),
                int(&format!("choose({n}, {k})"))
            );
            assert_eq!(
                int(&format!("nCr({n}, {k})")),
                int(&format!("nCr({}, {})", n - 1, k - 1)) + int(&format!("nCr({}, {k})", n - 1))
            );
            assert_eq!(
                int(&format!("nPr({n}, {k})")),
                int(&format!("nCr({n}, {k})")) * (1..=k).product::<i64>()
            );
        }
    }
    assert_eq!(int("prime_pi(100)"), 25);
    assert_eq!(int("Fibonacci(10)"), 55);
}

#[test]
fn regression_with_two_lists_and_polynomial_lcm() {
    // Least squares by the closed form slope = Sxy / Sxx, intercept = ybar - slope xbar, on pseudo-random data.
    let mut state = 0xA076_1D64_78BD_642Fu64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % 41) as i64 - 20
    };
    for _ in 0..6 {
        let n = 5 + (next().rem_euclid(4)) as usize;
        let xs: Vec<i64> = (0..n as i64).collect();
        let ys: Vec<i64> = xs.iter().map(|x| 3 * x - 4 + next() / 4).collect();
        let (nf, xbar) = (n as f64, xs.iter().sum::<i64>() as f64 / n as f64);
        let ybar = ys.iter().sum::<i64>() as f64 / nf;
        let sxy: f64 = xs
            .iter()
            .zip(&ys)
            .map(|(x, y)| (*x as f64 - xbar) * (*y as f64 - ybar))
            .sum();
        let sxx: f64 = xs.iter().map(|x| (*x as f64 - xbar).powi(2)).sum();
        let (slope, intercept) = (sxy / sxx, ybar - sxy / sxx * xbar);
        let list = |v: &[i64]| {
            format!(
                "[{}]",
                v.iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        for name in ["fit", "polyfit", "linear_regression"] {
            let out = run(&format!("{name}({}, {})", list(&xs), list(&ys)));
            // The line is printed like `51/25 · x - 1/20`: evaluate it at 0 and at 1.
            let evaluated = run(&format!("subs({out}, x, 0)"));
            let parse = |t: &str| -> f64 {
                match t.trim().split_once('/') {
                    Some((a, b)) => a.parse::<f64>().unwrap() / b.parse::<f64>().unwrap(),
                    None => t.trim().parse().unwrap(),
                }
            };
            assert!(
                (parse(&evaluated) - intercept).abs() < 1e-9,
                "{name}: intercept {evaluated} vs {intercept}"
            );
            let at_one = run(&format!("subs({out}, x, 1)"));
            assert!(
                (parse(&at_one) - (slope + intercept)).abs() < 1e-9,
                "{name}: {at_one} vs {}",
                slope + intercept
            );
        }
    }
    // Degree 2 through three points is exact interpolation.
    assert_eq!(run("polyfit([0, 1, 2], [1, 2, 5], 2)"), "x^2 + 1");
    // lcm of polynomials with chosen roots: the lcm has the union of the roots (compared after expansion).
    let monic = |roots: &[i64]| {
        roots
            .iter()
            .map(|r| format!("(x - ({r}))"))
            .collect::<Vec<_>>()
            .join("*")
    };
    for (a, b) in [
        (vec![1, -1], vec![-1]),
        (vec![1, -1], vec![-1, -1, 2]),
        (vec![0, 2, 3], vec![2, 3, 5]),
    ] {
        let (pa, pb) = (monic(&a), monic(&b));
        let mut union = a.clone();
        for r in &b {
            // Multiplicity is the larger of the two counts.
            let (ca, cb) = (
                a.iter().filter(|v| *v == r).count(),
                b.iter().filter(|v| *v == r).count(),
            );
            if cb > ca && union.iter().filter(|v| *v == r).count() < cb {
                union.push(*r);
            }
        }
        let got = run(&format!("expand({})", run(&format!("lcm({pa}, {pb})"))));
        let expected = run(&format!("expand({})", monic(&union)));
        assert_eq!(got, expected, "lcm({pa}, {pb})");
    }
    assert_eq!(run("lcm(6, 4)"), "12");
}

#[test]
fn substitution_spellings_replace_whole_words_simultaneously() {
    assert_eq!(run("subs(2*x + 1, x, 3)"), "7");
    assert_eq!(run("subs(x^2 + y, x = 2, y = 3)"), "7");
    assert_eq!(run("substitute(sin(x), x, pi/6)"), "1/2");
    assert_eq!(run("replace(2*x + 1, x, 3)"), "7");
    // Simultaneous: x -> y and y -> 1 applied once each, not chained.
    assert_eq!(run("subs(x + y, x = y, y = 1)"), "y + 1");
    // Whole words only: `max` and `xy` do not contain the variable `x`.
    assert_eq!(run("subs(xy + x, x, 2)"), "xy + 2");
    // A polynomial substituted at integer points agrees with direct evaluation.
    for a in -3i64..=3 {
        let direct = (a * a * a - 2 * a + 5).to_string();
        assert_eq!(run(&format!("subs(x^3 - 2*x + 5, x, {a})")), direct);
    }
}

#[test]
fn decimal_input_gets_a_floating_point_value() {
    let tail = |q: &str| -> f64 {
        let out = run(q);
        out.rsplit(" ≈ ")
            .next()
            .unwrap()
            .trim()
            .parse()
            .unwrap_or_else(|_| panic!("{q}: {out}"))
    };
    // The appended value agrees with the standard library.
    for (q, v) in [
        ("sin(0.5)", 0.5f64.sin()),
        ("exp(1.5)", 1.5f64.exp()),
        ("sqrt(2.5)", 2.5f64.sqrt()),
        ("ln(2.0)", 2.0f64.ln()),
        ("cos(0.25) + sin(0.25)", 0.25f64.cos() + 0.25f64.sin()),
        ("atan(0.3)", 0.3f64.atan()),
    ] {
        assert!((tail(q) - v).abs() < 1e-12, "{q}");
    }
    // Normal cdf at 1.96 is the published 0.9750021048517795 (to 15 digits).
    assert!((tail("normal_cdf(1.96)") - 0.975_002_104_851_779_5).abs() < 1e-12);
    // Exact answers stay exact: a plain rational gets no tail, and integer inputs get none either.
    assert_eq!(run("1.5 + 2"), "7/2");
    assert_eq!(run("0.1 + 0.2"), "3/10");
    assert_eq!(run("sin(0)"), "0");
    assert!(!run("sin(1/2)").contains('≈'));
    // Symbols are left alone.
    assert!(!run("2.5*x").contains('≈'));
}

#[test]
fn calculus_shorthand_means_the_explicit_command() {
    // Each shorthand must give exactly what the explicit command gives.
    for (short, explicit) in [
        ("d/dx x^3", "diff(x^3, x)"),
        ("d/dx(x^3 + 2*x)", "diff(x^3 + 2*x, x)"),
        ("d/dx sin(x)*x", "diff(sin(x)*x, x)"),
        ("d^2/dx^2 x^4", "diff(x^4, x, 2)"),
        ("d²/dx² x^4", "diff(x^4, x, 2)"),
        ("∂/∂y (x^2*y^3)", "diff(x^2*y^3, y)"),
        ("derivative(x^3, x, 2)", "diff(x^3, x, 2)"),
        ("lim x->0 sin(x)/x", "limit(sin(x)/x, x, 0)"),
        ("limit x -> oo 1/x", "limit(1/x, x, oo)"),
        ("lim x→2 x^2", "limit(x^2, x, 2)"),
        ("sqrt 16", "sqrt(16)"),
        ("sin x", "sin(x)"),
        ("∫ x^2 dx", "integrate(x^2, x)"),
        ("∫_0^1 x^2 dx", "integrate(x^2, x, 0, 1)"),
        ("∑_{k=1}^{4} k", "sum(k, k, 1, 4)"),
        ("nth_root(27, 3)", "root(27, 3)"),
    ] {
        assert_eq!(run(short), run(explicit), "{short} vs {explicit}");
    }
    // And the values are right.
    assert_eq!(run("d^2/dx^2 x^4"), "12 · x^2");
    assert_eq!(run("∑_{k=1}^{4} k"), "10");
    assert_eq!(run("nth_root(27, 3)"), "3");
    // A division of two symbols is not a derivative.
    assert_eq!(run("d/dx"), "d / dx");
}

#[test]
fn quantities_in_sentences_sum_and_convert() {
    let result = |q: &str| -> f64 {
        let out = run(q);
        out.rsplit(" = ")
            .next()
            .unwrap()
            .split(" [")
            .next()
            .unwrap()
            .trim()
            .parse()
            .unwrap_or_else(|_| panic!("{q}: {out}"))
    };
    // SI factors: metres per unit, grams per unit, seconds per unit.
    let metres = [("km", 1000.0), ("m", 1.0), ("cm", 0.01), ("mm", 0.001)];
    let grams = [("kg", 1000.0), ("g", 1.0), ("mg", 0.001)];
    let seconds = [("h", 3600.0), ("min", 60.0), ("s", 1.0)];
    for table in [&metres[..], &grams[..], &seconds[..]] {
        for (a, fa) in table {
            for (b, fb) in table {
                for (c, fc) in table {
                    let (x, y) = (3.0, 250.0);
                    // x a + y b to c, and the subtraction.
                    let plus = result(&format!("{x} {a} + {y} {b} to {c}"));
                    let minus = result(&format!("{x} {a} - {y} {b} to {c}"));
                    assert!(
                        (plus - (x * fa + y * fb) / fc).abs() < 1e-9 * (1.0 + plus.abs()),
                        "{x} {a} + {y} {b} to {c}: {plus}"
                    );
                    assert!(
                        (minus - (x * fa - y * fb) / fc).abs() < 1e-9 * (1.0 + minus.abs()),
                        "{x} {a} - {y} {b} to {c}: {minus}"
                    );
                }
            }
        }
    }
    // The old behaviour silently dropped the second term of `convert 3 kg + 500 g to g`.
    assert_eq!(
        run("convert 3 kg + 500 g to g"),
        "3 [kg] + 500 [g] = 3500 [g]"
    );
    assert_eq!(run("2 km - 500 m to m"), "2 [km] - 500 [m] = 1500 [m]");
    // A single quantity without the word convert, and the other spellings.
    assert_eq!(run("5 km to m"), run("convert 5 km to m"));
    assert_eq!(run("100 C to F"), run("convert 100 C to F"));
    assert_eq!(run("10 choose 3"), "120");
    assert_eq!(run("|-3|"), "3");
    assert_eq!(run("|x - 3|"), run("abs(x - 3)"));
    // Bracket literals are not JSON requests.
    assert_eq!(run("[[1,2],[3,4]] * [[5,6],[7,8]]"), "[ 19, 22 ; 43, 50 ]");
}

#[test]
fn spaced_products_and_radical_solutions() {
    // A space between two factors is a product: each spelling gives exactly what the starred form gives.
    for (spaced, starred) in [
        ("2 x", "2*x"),
        ("a x + b", "a*x + b"),
        ("x y", "x*y"),
        ("a x^2 + b x + c", "a*x^2 + b*x + c"),
        ("3 x + 2 x", "3*x + 2*x"),
        ("(x + 1) (x - 1)", "(x + 1)*(x - 1)"),
        ("2 (3 + 4)", "2*(3 + 4)"),
        ("x sin(x)", "x*sin(x)"),
        (
            "solve(a x^2 + b x + c = 0, x)",
            "solve(a*x^2 + b*x + c = 0, x)",
        ),
    ] {
        assert_eq!(run(spaced), run(starred), "{spaced} vs {starred}");
    }
    // Not guessed: a bare function name or two plain numbers stay errors.
    assert!(UraeSession::new().execute_line("sin x cos x").is_error);
    assert!(UraeSession::new().execute_line("1 2").is_error);
    // Square roots in solutions print as radicals.
    for n in [2i64, 3, 5, 6, 7, 10] {
        assert_eq!(
            run(&format!("solve(x^2 = {n})")),
            format!("Solutions for x: [√({n}), -√({n})]")
        );
    }
    let general = run("solve(a*x^2 + b*x + c = 0, x)");
    assert!(
        general.contains('√') && !general.contains("^(1/2)"),
        "{general}"
    );
}

#[test]
fn double_factorial_primes_and_digit_functions_match_definitions() {
    let int = |q: &str| -> u128 { run(q).parse().unwrap() };
    // n!! = n (n-2)!! with 0!! = 1!! = 1, and n!! (n-1)!! = n!. The old reading of `10!!` was (10!)!.
    let mut double = vec![1u128, 1];
    for n in 2..=25usize {
        let v = n as u128 * double[n - 2];
        double.push(v);
    }
    let mut factorial = 1u128;
    for n in 0..=25usize {
        if n > 0 {
            factorial *= n as u128;
        }
        assert_eq!(int(&format!("{n}!!")), double[n], "{n}!!");
        if n >= 1 {
            assert_eq!(
                double[n] * double[n - 1],
                factorial,
                "n!! (n-1)!! = n! at {n}"
            );
        }
    }
    assert_eq!(int("10!!"), 3840);
    // primes(n) against trial division.
    for limit in [2u32, 10, 30, 100, 200] {
        let expected: Vec<String> = (2..=limit)
            .filter(|n| (2..*n).take_while(|d| d * d <= *n).all(|d| n % d != 0))
            .map(|n| n.to_string())
            .collect();
        assert_eq!(
            run(&format!("primes({limit})")),
            format!("primes({limit}) = [{}]", expected.join(", "))
        );
    }
    // digit_sum against the decimal string, is_square against integer square roots.
    for n in [0u64, 7, 99, 12345, 987_654_321, 1_000_000_007] {
        let expected: u32 = n.to_string().chars().map(|c| c.to_digit(10).unwrap()).sum();
        assert_eq!(run(&format!("digit_sum({n})")), expected.to_string());
        assert_eq!(run(&format!("sum_of_digits({n})")), expected.to_string());
    }
    for n in 0u64..=200 {
        let root = (n as f64).sqrt().round() as u64;
        assert_eq!(
            run(&format!("is_square({n})")),
            format!("is_square({n}) = {}", root * root == n)
        );
    }
}

#[test]
fn discrete_probability_functions_match_their_definitions() {
    let frac = |text: &str| -> f64 {
        match text.trim().split_once('/') {
            Some((a, b)) => a.parse::<f64>().unwrap() / b.parse::<f64>().unwrap(),
            None => text.trim().parse().unwrap(),
        }
    };
    let choose =
        |n: u64, k: u64| -> f64 { (1..=k).fold(1.0, |acc, i| acc * (n - k + i) as f64 / i as f64) };
    // binomial_cdf(n, p, k) is the sum of the pmf terms C(n, i) p^i (1-p)^(n-i), i = 0..k.
    for (n, num, den) in [(10u64, 1u64, 2u64), (12, 1, 3), (7, 3, 4), (20, 1, 10)] {
        let p = num as f64 / den as f64;
        let mut cumulative = 0.0;
        for k in 0..=n {
            cumulative += choose(n, k) * p.powi(k as i32) * (1.0 - p).powi((n - k) as i32);
            let got = frac(&run(&format!("binomial_cdf({n}, {num}/{den}, {k})")));
            assert!(
                (got - cumulative).abs() < 1e-12,
                "binomial_cdf({n}, {num}/{den}, {k}): {got} vs {cumulative}"
            );
        }
        assert_eq!(run(&format!("binomial_cdf({n}, {num}/{den}, {n})")), "1");
        assert_eq!(run(&format!("binomial_cdf({n}, {num}/{den}, -1)")), "0");
    }
    // The hypergeometric pmf sums to 1 over k, and matches the ratio of binomials.
    for (total, successes, draws) in [(10u64, 4u64, 5u64), (20, 7, 6), (52, 13, 5)] {
        let mut sum = 0.0;
        for k in 0..=draws.min(successes) {
            let got = frac(&run(&format!(
                "hypergeometric_pmf({k}, {total}, {successes}, {draws})"
            )));
            let expected =
                choose(successes, k) * choose(total - successes, draws - k) / choose(total, draws);
            assert!(
                (got - expected).abs() < 1e-12,
                "hypergeometric_pmf({k}, {total}, {successes}, {draws})"
            );
            sum += got;
        }
        assert!((sum - 1.0).abs() < 1e-12);
    }
    // Bayes' rule and z-scores.
    assert_eq!(run("bayes(1/100, 9/10, 1/20)"), "2/13");
    assert_eq!(run("z_score(75, 70, 5)"), "1");
    // Expected value and variance against the definitions sum p x and sum p (x - mu)^2.
    for (xs, ps) in [
        ([1.0, 2.0, 3.0], [(1u32, 2u32), (1, 4), (1, 4)]),
        ([0.0, 5.0, 10.0], [(1, 5), (3, 5), (1, 5)]),
    ] {
        let probabilities: Vec<f64> = ps.iter().map(|(a, b)| *a as f64 / *b as f64).collect();
        let mean: f64 = xs.iter().zip(&probabilities).map(|(x, p)| x * p).sum();
        let variance: f64 = xs
            .iter()
            .zip(&probabilities)
            .map(|(x, p)| p * (x - mean).powi(2))
            .sum();
        let list = |v: &[String]| format!("[{}]", v.join(", "));
        let values = list(&xs.iter().map(|x| x.to_string()).collect::<Vec<_>>());
        let probs = list(
            &ps.iter()
                .map(|(a, b)| format!("{a}/{b}"))
                .collect::<Vec<_>>(),
        );
        let e = run(&format!("expected_value({values}, {probs})"));
        let v = run(&format!("variance_discrete({values}, {probs})"));
        assert!(
            (frac(e.rsplit("= ").next().unwrap()) - mean).abs() < 1e-12,
            "{e}"
        );
        assert!(
            (frac(v.rsplit("= ").next().unwrap()) - variance).abs() < 1e-12,
            "{v}"
        );
    }
    // Probabilities that do not sum to 1 are refused.
    assert!(
        UraeSession::new()
            .execute_line("expected_value([1, 2], [1/2, 1/4])")
            .is_error
    );
}

#[test]
fn series_sums_and_triangle_formulas_match_their_definitions() {
    let frac = |text: &str| -> f64 {
        match text.trim().split_once('/') {
            Some((a, b)) => a.parse::<f64>().unwrap() / b.parse::<f64>().unwrap(),
            None => text.trim().parse().unwrap(),
        }
    };
    // arithmetic_sum(a, d, n) and geometric_sum(a, r, n) against term-by-term addition.
    for (a, d, n) in [(1i64, 2i64, 10i64), (5, -3, 7), (0, 1, 100), (3, 0, 4)] {
        let direct: i64 = (0..n).map(|k| a + k * d).sum();
        assert_eq!(
            run(&format!("arithmetic_sum({a}, {d}, {n})")),
            direct.to_string()
        );
    }
    for (a, r, n) in [(1i64, 2i64, 10u32), (3, 3, 5), (2, -2, 6), (5, 1, 4)] {
        let direct: i64 = (0..n).map(|k| a * r.pow(k)).sum();
        assert_eq!(
            run(&format!("geometric_sum({a}, {r}, {n})")),
            direct.to_string()
        );
    }
    assert_eq!(run("geometric_sum(1, 1/2, 3)"), "7/4");
    // Pythagorean triples: the hypotenuse is exact, and Heron's formula on a triple gives half the legs' product.
    for (a, b, c) in [(3i64, 4i64, 5i64), (5, 12, 13), (8, 15, 17), (20, 21, 29)] {
        assert_eq!(run(&format!("hypotenuse({a}, {b})")), c.to_string());
        assert_eq!(
            run(&format!("area_triangle({a}, {b}, {c})")),
            (a * b / 2).to_string()
        );
    }
    assert_eq!(run("hypotenuse(1, 1)"), "√(2)");
    // Heron against the coordinate (shoelace) area for a triangle placed at the origin.
    for (base, height) in [(4i64, 3i64), (6, 8), (10, 4)] {
        let side = ((base * base + height * height) as f64).sqrt();
        let heron = run(&format!(
            "area_triangle({base}, {height}, sqrt({}))",
            base * base + height * height
        ));
        let _ = side;
        assert_eq!(
            run(&format!("area_triangle((0,0),({base},0),(0,{height}))")),
            (base * height / 2).to_string()
        );
        assert!(!heron.is_empty());
    }
    // angle_between against atan2 of the cross and dot products.
    for (u, v) in [
        ([1.0f64, 0.0], [0.0, 1.0]),
        ([1.0, 1.0], [1.0, 0.0]),
        ([2.0, 1.0], [-1.0, 3.0]),
        ([1.0, 0.0], [-1.0, 0.0]),
    ] {
        let expected = (u[0] * v[1] - u[1] * v[0])
            .abs()
            .atan2(u[0] * v[0] + u[1] * v[1]);
        let out = run(&format!(
            "angle_between([{}, {}], [{}, {}])",
            u[0], u[1], v[0], v[1]
        ));
        let got = if out.contains('π') {
            let (num, den) = match out.split_once("π/") {
                Some((n, d)) => (
                    if n.is_empty() {
                        1.0
                    } else {
                        n.trim_end_matches('*').parse().unwrap()
                    },
                    d.parse::<f64>().unwrap(),
                ),
                None => (
                    if out.trim() == "π" {
                        1.0
                    } else {
                        out.trim_end_matches("*π").parse().unwrap()
                    },
                    1.0,
                ),
            };
            num * std::f64::consts::PI / den
        } else {
            frac(out.rsplit("≈ ").next().unwrap())
        };
        assert!(
            (got - expected).abs() < 1e-9,
            "angle_between({u:?}, {v:?}) = {out}, expected {expected}"
        );
    }
    assert_eq!(run("angle_between([1, 0], [0, 1])"), "π/2");
    assert_eq!(run("maclaurin(cos(x), 4)"), run("taylor(cos(x), x, 0, 4)"));
}

#[test]
fn bit_operations_and_number_bases_match_the_standard_library() {
    let int = |q: &str| -> u64 { run(q).parse().unwrap_or_else(|_| panic!("{q}: {}", run(q))) };
    // Bit operations on a grid of values, against Rust's own operators.
    for a in [0u64, 1, 5, 12, 255, 1000, 65_535, 123_456_789] {
        for b in [0u64, 1, 3, 10, 170, 4096] {
            assert_eq!(int(&format!("{a} & {b}")), a & b, "{a} & {b}");
            assert_eq!(int(&format!("{a} | {b}")), a | b, "{a} | {b}");
            assert_eq!(int(&format!("{a} xor {b}")), a ^ b, "{a} xor {b}");
            assert_eq!(int(&format!("bitand({a}, {b})")), a & b);
        }
        for shift in [0u32, 1, 3, 8] {
            assert_eq!(
                int(&format!("{a} << {shift}")),
                a << shift,
                "{a} << {shift}"
            );
            assert_eq!(
                int(&format!("{a} >> {shift}")),
                a >> shift,
                "{a} >> {shift}"
            );
        }
    }
    // Base conversion against the standard formatters, and the round trip through each base.
    for n in [0u64, 1, 7, 10, 255, 4096, 987_654_321] {
        assert_eq!(
            run(&format!("{n} to binary")),
            format!("dec2bin({n}) = {n:b}")
        );
        assert_eq!(run(&format!("{n} in hex")), format!("dec2hex({n}) = {n:x}"));
        assert_eq!(
            run(&format!("{n} as octal")),
            format!("dec2oct({n}) = {n:o}")
        );
        assert_eq!(
            run(&format!("from_binary({n:b})")),
            format!("bin2dec({n:b}) = {n}")
        );
        assert_eq!(
            run(&format!("from_hex({n:x})")),
            format!("hex2dec({n:x}) = {n}")
        );
        // 0x, 0b and 0o literals are numbers.
        assert_eq!(int(&format!("0x{n:x}")), n);
        assert_eq!(int(&format!("0b{n:b}")), n);
        assert_eq!(int(&format!("0o{n:o}")), n);
    }
    assert_eq!(int("0xFF + 0b1"), 256);
    assert_eq!(run("0b1010 + 1"), "11");
    // Words that are not variables are never read as a product: `5 xor 3` was `15 xor`.
    assert!(!run("5 xor 3").contains("xor"));
    // A set typed with braces is a set: the power set has 2^n members.
    for items in ["{1, 2}", "{1, 2, 3}", "[1, 2, 3, 4]"] {
        let out = run(&format!("powerset({items})"));
        let members = items.matches(',').count() + 1;
        let subsets = out.matches('[').count() - 1;
        assert_eq!(subsets, 1 << members, "{out}");
    }
}

#[test]
fn logic_commands_match_brute_force_and_set_notation_works() {
    // Each formula is paired with an independent Rust evaluation over (p, q, r).
    type Formula = (&'static str, fn(bool, bool, bool) -> bool);
    let formulas: [Formula; 8] = [
        ("p and q", |p, q, _| p && q),
        ("p or q", |p, q, _| p || q),
        ("p xor q", |p, q, _| p ^ q),
        ("p -> q", |p, q, _| !p || q),
        ("p <-> q", |p, q, _| p == q),
        ("not (p and q) or r", |p, q, r| !(p && q) || r),
        ("(p -> q) and (q -> r)", |p, q, r| (!p || q) && (!q || r)),
        ("p and (q or r)", |p, q, r| p && (q || r)),
    ];
    for (formula, truth) in formulas {
        let words: Vec<&str> = formula.split(|c: char| !c.is_alphabetic()).collect();
        let vars: Vec<char> = ['p', 'q', 'r']
            .into_iter()
            .filter(|v| words.contains(&v.to_string().as_str()))
            .collect();
        let n = vars.len();
        let table = run(&format!("truth_table({formula})"));
        let rows: Vec<&str> = table.lines().skip(1).collect();
        assert_eq!(rows.len(), 1 << n, "{formula}: {table}");
        let mut satisfying = 0;
        for (i, row) in rows.iter().enumerate() {
            let value = |v: char| match vars.iter().position(|x| *x == v) {
                Some(k) => (i >> (n - 1 - k)) & 1 == 0,
                None => false,
            };
            let expected = truth(value('p'), value('q'), value('r'));
            assert!(
                row.ends_with(if expected { "| T" } else { "| F" }),
                "{formula} row {i}: {row}"
            );
            satisfying += expected as usize;
        }
        assert_eq!(
            run(&format!("count_models({formula})")),
            format!("{satisfying} of {} assignments satisfy it", 1 << n)
        );
        assert_eq!(
            run(&format!("tautology({formula})")),
            (satisfying == 1 << n).to_string(),
            "{formula}"
        );
        assert_eq!(
            run(&format!("contradiction({formula})")),
            (satisfying == 0).to_string(),
            "{formula}"
        );
        assert_eq!(
            run(&format!("satisfiable({formula})")).starts_with("satisfiable"),
            satisfying > 0,
            "{formula}"
        );
    }
    // Classical tautologies and equivalences.
    for law in [
        "p or not p",
        "(p -> q) <-> (not q -> not p)",
        "not (p and q) <-> (not p or not q)",
        "((p -> q) and p) -> q",
    ] {
        assert_eq!(run(&format!("tautology({law})")), "true", "{law}");
    }
    assert_eq!(run("equivalent(not (p or q), not p and not q)"), "true");
    assert_eq!(run("equivalent(p -> q, q -> p)"), "false");
    assert_eq!(run("satisfiable(p and not p)"), "unsatisfiable");
    // Set cardinality and Cartesian products: |A x B| = |A| |B|.
    assert_eq!(run("|{1, 2, 3}|"), "3");
    assert_eq!(run("cardinality({1, 2, 2, 3})"), "3");
    for (a, b) in [
        ("{1, 2}", "{3, 4, 5}"),
        ("{a}", "{1, 2, 3, 4}"),
        ("{1, 2, 3}", "{x, y, z}"),
    ] {
        let size = |set: &str| -> usize { run(&format!("size {set}")).parse().unwrap() };
        let product = run(&format!("{a} x {b}"));
        assert_eq!(product, run(&format!("cartesian_product({a}, {b})")));
        assert_eq!(product.matches('(').count(), size(a) * size(b), "{product}");
    }
    assert_eq!(run("|x|"), run("abs(x)"));
}

#[test]
fn degrees_temperatures_and_signed_denominators() {
    let tail = |q: &str| -> f64 {
        let out = run(q);
        out.rsplit(" ≈ ")
            .next()
            .unwrap()
            .trim()
            .parse()
            .unwrap_or_else(|_| panic!("{q}: {out}"))
    };
    // Degree literals in three spellings against the standard library (an angle with a decimal gets a numeric tail).
    for angle in [10.5f64, 22.5, 37.25, 100.5, 200.5] {
        let r = angle.to_radians();
        for spelling in [
            format!("{angle}°"),
            format!("{angle} deg"),
            format!("{angle} degrees"),
        ] {
            assert!(
                (tail(&format!("sin({spelling})")) - r.sin()).abs() < 1e-12,
                "sin({spelling})"
            );
            assert!(
                (tail(&format!("cos({spelling})")) - r.cos()).abs() < 1e-12,
                "cos({spelling})"
            );
            assert!(
                (tail(&format!("tan({spelling})")) - r.tan()).abs() < 1e-9,
                "tan({spelling})"
            );
        }
    }
    // Exact special angles.
    assert_eq!(run("sin(30°)"), "1/2");
    assert_eq!(run("cos(60 degrees)"), "1/2");
    assert_eq!(run("tan(45 deg)"), "1");
    assert_eq!(run("sin(90°) + cos(0°)"), "2");
    assert_eq!(run("deg2rad(90)"), "1/2 · π");
    assert_eq!(run("radians(180)"), "π");
    assert_eq!(run("degrees(pi)"), "180");
    assert_eq!(run("rad2deg(pi/2)"), "90");
    // Temperatures: known points and the round trip.
    assert_eq!(run("celsius_to_fahrenheit(100)"), "212");
    assert_eq!(run("celsius_to_fahrenheit(-40)"), "-40");
    assert_eq!(run("fahrenheit_to_celsius(32)"), "0");
    for c in [-273i64, -40, 0, 37, 100, 1000] {
        let f = run(&format!("celsius_to_fahrenheit({c})"));
        assert_eq!(run(&format!("fahrenheit_to_celsius({f})")), c.to_string());
    }
    // A name that merely starts like a unit is not an angle.
    assert!(UraeSession::new().execute_line("x deg").is_error);
    // x / -y prints as -x / y: the parallel-resistance solution is no longer `-1 / (-(...))`.
    let parallel = run("solve(1/R = 1/a + 1/b, R)");
    assert!(!parallel.contains("-1 / (-"), "{parallel}");
    assert_eq!(parallel, "Solutions for R: [1 / (1 / a + 1 / b)]");
}

#[test]
fn graph_commands_match_closed_forms_and_brute_force() {
    // Evaluate "P(k) = k^4 - 4*k^3 + 6*k^2 - 3*k" at an integer.
    fn evaluate(text: &str, k: i64) -> i64 {
        let body = text.trim_start_matches("P(k) = ");
        let mut total = 0i64;
        let mut sign = 1i64;
        for token in body.split_whitespace() {
            match token {
                "+" => sign = 1,
                "-" => sign = -1,
                term => {
                    let (coefficient, power) = match term.split_once("*k^") {
                        Some((c, p)) => (c.parse::<i64>().unwrap(), p.parse::<u32>().unwrap()),
                        None => match term.split_once("*k") {
                            Some((c, _)) => (c.parse().unwrap(), 1),
                            None => match term.strip_prefix("k^") {
                                Some(p) => (1, p.parse().unwrap()),
                                None if term == "k" => (1, 1),
                                None => (term.parse().unwrap(), 0),
                            },
                        },
                    };
                    total += sign * coefficient * k.pow(power);
                    sign = 1;
                }
            }
        }
        total
    }
    let count: i64 = 0;
    let _ = count;
    // Cayley's formula n^(n-2), the cycle and the path.
    for n in 2..=8u32 {
        assert_eq!(
            run(&format!("spanning_trees(K{n})")),
            (n as u128).pow(n - 2).to_string(),
            "K{n}"
        );
    }
    for n in 3..=9 {
        assert_eq!(run(&format!("spanning_trees(C{n})")), n.to_string());
        assert_eq!(run(&format!("spanning_trees(P{n})")), "1");
        assert_eq!(run(&format!("is_tree(P{n})")), "true");
        assert_eq!(run(&format!("is_tree(C{n})")), "false");
    }
    // Chromatic polynomials: K_n is k (k-1) ... (k-n+1), C_n is (k-1)^n + (-1)^n (k-1), a tree on n vertices k (k-1)^(n-1).
    for n in 3..=7i64 {
        for k in 0..=6i64 {
            let complete: i64 = (0..n).map(|i| k - i).product();
            assert_eq!(
                evaluate(&run(&format!("chromatic_polynomial(K{n})")), k),
                complete,
                "K{n} at {k}"
            );
            let cycle = (k - 1).pow(n as u32) + (-1i64).pow(n as u32) * (k - 1);
            assert_eq!(
                evaluate(&run(&format!("chromatic_polynomial(C{n})")), k),
                cycle,
                "C{n} at {k}"
            );
            let path = k * (k - 1).pow(n as u32 - 1);
            assert_eq!(
                evaluate(&run(&format!("chromatic_polynomial(P{n})")), k),
                path,
                "P{n} at {k}"
            );
        }
    }
    assert_eq!(run("chromatic_number(K5)"), "5");
    assert_eq!(run("chromatic_number(C7)"), "3");
    assert_eq!(run("chromatic_number(C8)"), "2");
    assert_eq!(run("chromatic_polynomial(K3)"), "P(k) = k^3 - 3*k^2 + 2*k");
    // Random graphs on 6 vertices: proper colourings and spanning trees by brute force.
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = move |m: u64| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state % m
    };
    let n = 6usize;
    let pairs: Vec<(usize, usize)> = (0..n)
        .flat_map(|a| (a + 1..n).map(move |b| (a, b)))
        .collect();
    for _ in 0..12 {
        let edges: Vec<(usize, usize)> = pairs.iter().copied().filter(|_| next(100) < 45).collect();
        if edges.is_empty() {
            continue;
        }
        let literal = format!(
            "[{}]",
            edges
                .iter()
                .map(|(a, b)| format!("[{a}, {b}]"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        // Vertices that appear in no edge do not exist in an edge-list graph: count only used vertices.
        let mut used: Vec<usize> = edges.iter().flat_map(|(a, b)| [*a, *b]).collect();
        used.sort_unstable();
        used.dedup();
        let m = used.len();
        let index = |v: usize| used.iter().position(|x| *x == v).unwrap();
        let local: Vec<(usize, usize)> =
            edges.iter().map(|(a, b)| (index(*a), index(*b))).collect();
        // Proper colourings with k colours.
        let polynomial = run(&format!("chromatic_polynomial({literal})"));
        for k in 1..=4i64 {
            let mut proper = 0i64;
            for code in 0..(k as usize).pow(m as u32) {
                let colour = |v: usize| (code / (k as usize).pow(v as u32)) % k as usize;
                if local.iter().all(|(a, b)| colour(*a) != colour(*b)) {
                    proper += 1;
                }
            }
            assert_eq!(evaluate(&polynomial, k), proper, "{literal} at k = {k}");
        }
        // Spanning trees: subsets of m - 1 edges that connect all m vertices.
        let mut trees = 0u64;
        for mask in 0u32..(1 << local.len()) {
            if mask.count_ones() as usize != m - 1 {
                continue;
            }
            let mut parent: Vec<usize> = (0..m).collect();
            fn find(p: &mut Vec<usize>, x: usize) -> usize {
                if p[x] != x {
                    let r = find(p, p[x]);
                    p[x] = r;
                }
                p[x]
            }
            let mut acyclic = true;
            for (i, (a, b)) in local.iter().enumerate() {
                if mask >> i & 1 == 1 {
                    let (ra, rb) = (find(&mut parent, *a), find(&mut parent, *b));
                    if ra == rb {
                        acyclic = false;
                        break;
                    }
                    parent[ra] = rb;
                }
            }
            trees += acyclic as u64;
        }
        assert_eq!(
            run(&format!("spanning_trees({literal})")),
            trees.to_string(),
            "{literal}"
        );
    }
    // Basic structure.
    assert_eq!(
        run("degree_sequence([[1,2],[2,3],[3,1],[3,4]])"),
        "[3, 2, 2, 1]"
    );
    assert_eq!(run("components([[1,2],[3,4]])"), "2");
    assert_eq!(run("is_bipartite(C6)"), "true");
    assert_eq!(run("is_bipartite(C7)"), "false");
    assert_eq!(
        run("shortest_path([[1,2],[2,3],[3,4],[1,4]], 1, 3)")
            .split(':')
            .next()
            .unwrap(),
        "length 2"
    );
    assert_eq!(run("diameter(C8)"), "4");
    assert_eq!(run("diameter(P6)"), "5");
}

#[test]
fn percentages_and_rounding_are_exact() {
    let ratio = |text: &str| -> (i128, i128) {
        match text.trim().split_once('/') {
            Some((a, b)) => (a.parse().unwrap(), b.parse().unwrap()),
            None => (text.trim().parse().unwrap(), 1),
        }
    };
    let equal = |text: &str, n: i128, d: i128| {
        let (a, b) = ratio(text);
        assert_eq!(a * d, n * b, "{text} vs {n}/{d}");
    };
    // p% of m = p m / 100, m + p% = m (100 + p) / 100, m - p% = m (100 - p) / 100.
    for p in [5i128, 15, 20, 33, 100, 150] {
        for m in [80i128, 200, 7, 1250] {
            equal(&run(&format!("{p}% of {m}")), p * m, 100);
            equal(&run(&format!("what is {p}% of {m}")), p * m, 100);
            equal(&run(&format!("{m} + {p}%")), m * (100 + p), 100);
            equal(&run(&format!("{m} - {p}%")), m * (100 - p), 100);
        }
        equal(&run(&format!("{p}%")), p, 100);
    }
    assert_eq!(run("0.75 as fraction"), "3/4");
    assert_eq!(run("17 % 5"), "2");
    // round(x, n) against exact integer arithmetic: half away from zero at n decimals, x = a / 1000.
    for a in [
        -9999i128, -2675, -2500, -5, -1, 0, 1, 5, 14, 15, 2675, 3141, 9995,
    ] {
        for digits in 0..=3u32 {
            let scale = 10i128.pow(3 - digits.min(3));
            // a / 1000 rounded to `digits` decimals: round(a / scale) * scale / 1000.
            let q = a.abs() / scale + if (a.abs() % scale) * 2 >= scale { 1 } else { 0 };
            let rounded = q * a.signum();
            let value = format!(
                "{}{}.{:03}",
                if a < 0 { "-" } else { "" },
                a.abs() / 1000,
                a.abs() % 1000
            );
            let out = run(&format!("round({value}, {digits})"));
            let expected = rounded * scale; // in thousandths
            let shown: i128 = {
                let negative = out.starts_with('-');
                let body = out.trim_start_matches('-');
                let (whole, fraction) = body.split_once('.').unwrap_or((body, ""));
                let padded = format!("{fraction:0<3}");
                let thousandths =
                    whole.parse::<i128>().unwrap() * 1000 + padded.parse::<i128>().unwrap();
                if negative { -thousandths } else { thousandths }
            };
            assert_eq!(shown, expected, "round({value}, {digits}) = {out}");
        }
    }
    assert_eq!(run("round(3.14159, 2)"), "3.14");
    assert_eq!(run("round(2.675, 2)"), "2.68");
    assert_eq!(run("round(1234, -2)"), "1200");
    assert_eq!(run("round(7/3, 3)"), "2.333");
}

#[test]
fn binomial_equations_have_exact_roots_that_satisfy_them() {
    // "a + bi", "bi" or "a" back to (re, im).
    fn complex(text: &str) -> (f64, f64) {
        let t = text.trim();
        if let Some(body) = t.strip_suffix('i') {
            if let Some(at) = body.rfind(" + ").or_else(|| body.rfind(" - ")) {
                let re: f64 = body[..at].trim().parse().unwrap();
                let im: f64 = body[at + 3..].trim().parse().unwrap();
                return (re, if &body[at..at + 3] == " - " { -im } else { im });
            }
            return (0.0, body.trim().parse().unwrap());
        }
        (t.parse().unwrap(), 0.0)
    }
    // Top-level comma split of "Solutions for x: [a, b, c]".
    fn roots_of(text: &str) -> Vec<String> {
        let inner = &text[text.find('[').unwrap() + 1..text.rfind(']').unwrap()];
        let (mut depth, mut current, mut out) = (0i32, String::new(), Vec::new());
        for c in inner.chars() {
            match c {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
            if c == ',' && depth == 0 {
                out.push(std::mem::take(&mut current));
            } else {
                current.push(c);
            }
        }
        out.push(current);
        out.into_iter().map(|s| s.trim().to_string()).collect()
    }
    let cases: [(&str, u32, f64); 11] = [
        ("x^8 = 1", 8, 1.0),
        ("x^6 = 1", 6, 1.0),
        ("x^4 = -1", 4, -1.0),
        ("x^3 = 8", 3, 8.0),
        ("x^4 = 16", 4, 16.0),
        ("x^6 = 64", 6, 64.0),
        ("x^4 + 4 = 0", 4, -4.0),
        ("x^3 - 2 = 0", 3, 2.0),
        ("2*x^4 = 32", 4, 16.0),
        ("x^12 = 1", 12, 1.0),
        ("x^24 = 1", 24, 1.0),
    ];
    for (equation, n, c) in cases {
        let out = run(&format!("solve({equation})"));
        let roots = roots_of(&out);
        assert_eq!(roots.len(), n as usize, "{equation}: {out}");
        let mut values: Vec<(f64, f64)> = Vec::new();
        for root in &roots {
            // The root numerically, then its n-th power against c.
            let z = complex(&run(&format!("evalf({root})")));
            let (mut re, mut im) = (1.0f64, 0.0f64);
            for _ in 0..n {
                (re, im) = (re * z.0 - im * z.1, re * z.1 + im * z.0);
            }
            assert!(
                (re - c).abs() < 1e-9 && im.abs() < 1e-9,
                "{equation}: ({root})^{n} = {re} + {im}i, expected {c}"
            );
            values.push(z);
        }
        // The roots are distinct.
        for i in 0..values.len() {
            for j in i + 1..values.len() {
                let d = ((values[i].0 - values[j].0).powi(2) + (values[i].1 - values[j].1).powi(2))
                    .sqrt();
                assert!(d > 1e-6, "{equation}: repeated root {:?}", values[i]);
            }
        }
        assert!(!out.contains("RootOf") && !out.contains("^(1/2)"), "{out}");
    }
    // `roots` agrees, exact values keep their closed form, and other degrees keep the general solver.
    assert_eq!(run("roots(x^4 - 16)"), run("solve(x^4 = 16)"));
    assert_eq!(
        run("solve(x^4 = 16)"),
        "Solutions for x: [2, 2 · i, -2, -2 · i]"
    );
    assert!(run("solve(x^5 = 1)").starts_with("Solutions for x: [1,"));
}

#[test]
fn list_utilities_match_direct_computation() {
    let mut state = 0xC2B2_AE3D_27D4_EB4Fu64;
    let mut next = move |m: u64| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % m) as i64
    };
    let literal = |v: &[i64]| {
        format!(
            "[{}]",
            v.iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let ints = |text: &str| -> Vec<i64> {
        text.trim_matches(|c| c == '[' || c == ']')
            .split(", ")
            .filter(|s| !s.is_empty())
            .map(|s| s.parse().unwrap())
            .collect()
    };
    for _ in 0..25 {
        let n = 3 + next(7) as usize;
        let v: Vec<i64> = (0..n).map(|_| next(21) - 10).collect();
        let l = literal(&v);
        assert_eq!(run(&format!("sum({l})")), v.iter().sum::<i64>().to_string());
        assert_eq!(run(&format!("len({l})")), n.to_string());
        let prefix: Vec<i64> = v
            .iter()
            .scan(0, |s, x| {
                *s += x;
                Some(*s)
            })
            .collect();
        assert_eq!(ints(&run(&format!("cumsum({l})"))), prefix);
        let differences: Vec<i64> = v.windows(2).map(|w| w[1] - w[0]).collect();
        assert_eq!(ints(&run(&format!("diff({l})"))), differences);
        let mut sorted = v.clone();
        sorted.sort();
        assert_eq!(ints(&run(&format!("sort({l})"))), sorted);
        let mut unique = sorted.clone();
        unique.dedup();
        assert_eq!(ints(&run(&format!("unique({l})"))), unique);
        let mut reversed = v.clone();
        reversed.reverse();
        assert_eq!(ints(&run(&format!("reverse({l})"))), reversed);
        let target = v[0];
        assert_eq!(
            run(&format!("count({l}, {target})")),
            v.iter().filter(|x| **x == target).count().to_string()
        );
        let (max, min) = (*v.iter().max().unwrap(), *v.iter().min().unwrap());
        assert_eq!(
            v[run(&format!("argmax({l})")).parse::<usize>().unwrap()],
            max
        );
        assert_eq!(
            v[run(&format!("argmin({l})")).parse::<usize>().unwrap()],
            min
        );
        assert_eq!(run(&format!("min({l})")), min.to_string());
        assert_eq!(
            run(&format!("sum_squares({l})")),
            v.iter().map(|x| x * x).sum::<i64>().to_string()
        );
        // Moving average: exact rationals against direct sums.
        let w = 2 + next(2) as usize;
        let averages = run(&format!("moving_average({l}, {w})"));
        let expected: Vec<(i64, i64)> = v
            .windows(w)
            .map(|win| (win.iter().sum::<i64>(), w as i64))
            .collect();
        for (got, (num, den)) in averages
            .trim_matches(|c| c == '[' || c == ']')
            .split(", ")
            .zip(expected)
        {
            let (a, b) = match got.split_once('/') {
                Some((a, b)) => (a.parse::<i64>().unwrap(), b.parse::<i64>().unwrap()),
                None => (got.parse().unwrap(), 1),
            };
            assert_eq!(a * den, num * b, "moving_average({l}, {w}): {averages}");
        }
        // Average ranks: ties share the mean of their positions.
        let ranks = run(&format!("rank_data({l})"));
        for (i, got) in ranks
            .trim_matches(|c| c == '[' || c == ']')
            .split(", ")
            .enumerate()
        {
            let less = v.iter().filter(|x| **x < v[i]).count() as f64;
            let equal = v.iter().filter(|x| **x == v[i]).count() as f64;
            let expected = less + (equal + 1.0) / 2.0;
            let value = match got.split_once('/') {
                Some((a, b)) => a.parse::<f64>().unwrap() / b.parse::<f64>().unwrap(),
                None => got.parse().unwrap(),
            };
            assert!(
                (value - expected).abs() < 1e-12,
                "rank_data({l})[{i}]: {got} vs {expected}"
            );
        }
        // Histogram counts.
        let histogram = run(&format!("histogram({l})"));
        for value in &unique {
            let count = v.iter().filter(|x| *x == value).count();
            assert!(
                histogram.contains(&format!("{value}: {count}")),
                "{histogram}"
            );
        }
    }
    // Sequences.
    assert_eq!(run("range(5)"), "[0, 1, 2, 3, 4]");
    assert_eq!(run("range(1, 10, 3)"), "[1, 4, 7]");
    assert_eq!(run("range(10, 0, -3)"), "[10, 7, 4, 1]");
    assert_eq!(run("linspace(0, 1, 5)"), "[0, 1/4, 1/2, 3/4, 1]");
    assert_eq!(run("arange(0, 1, 1/4)"), "[0, 1/4, 1/2, 3/4]");
    // Means: exact where the answer is rational or a radical.
    assert_eq!(run("harmonic_mean([1, 2, 4])"), "12/7");
    assert_eq!(run("weighted_mean([1, 2, 3], [1, 1, 2])"), "9/4");
    assert_eq!(run("geometric_mean([1, 2, 4])"), "2");
    assert_eq!(run("rms([3, 4])"), "5/2 · √(2)");
    // Older meanings are untouched: a series sum, a derivative, the statistical range of a list, mean of scalars.
    assert_eq!(run("sum(k, k, 1, 10)"), "55");
    assert_eq!(run("diff(x^3, x)"), "3 · x^2");
    assert_eq!(run("range([1, 2, 10])"), "range = 9");
    assert_eq!(run("mean(2, 4, 6)"), "mean = 4");
}

#[test]
fn matrix_variables_keep_their_value_across_lines() {
    // "[ 7, 10 ; 15, 22 ]" back to rows of exact (numerator, denominator) pairs.
    fn rows(text: &str) -> Vec<Vec<(i128, i128)>> {
        let inner = text.trim().trim_start_matches('[').trim_end_matches(']');
        inner
            .split(';')
            .map(|row| {
                row.split(',')
                    .map(|cell| match cell.trim().split_once('/') {
                        Some((a, b)) => (a.trim().parse().unwrap(), b.trim().parse().unwrap()),
                        None => (cell.trim().parse().unwrap(), 1),
                    })
                    .collect()
            })
            .collect()
    }
    let mut state = 0x1234_5678_9ABC_DEF1u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % 9) as i128 - 4
    };
    for size in [2usize, 3] {
        for _ in 0..6 {
            let a: Vec<Vec<i128>> = (0..size)
                .map(|_| (0..size).map(|_| next()).collect())
                .collect();
            let literal = format!(
                "[{}]",
                a.iter()
                    .map(|r| format!(
                        "[{}]",
                        r.iter()
                            .map(|x| x.to_string())
                            .collect::<Vec<_>>()
                            .join(",")
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            );
            let mut session = UraeSession::new();
            let say = |s: &mut UraeSession, q: &str| -> String {
                let r = s.execute_line(q);
                assert!(!r.is_error, "{q}: {}", r.output_text);
                r.output_text
            };
            say(&mut session, &format!("A = {literal}"));
            // A * A against direct multiplication, stored as B and reused.
            let square: Vec<Vec<i128>> = (0..size)
                .map(|i| {
                    (0..size)
                        .map(|j| (0..size).map(|k| a[i][k] * a[k][j]).sum())
                        .collect()
                })
                .collect();
            let product = rows(&say(&mut session, "A*A"));
            for i in 0..size {
                for j in 0..size {
                    assert_eq!(product[i][j], (square[i][j], 1), "A*A for {literal}");
                }
            }
            say(&mut session, "B = A*A");
            let sum = rows(&say(&mut session, "B + A"));
            for i in 0..size {
                for j in 0..size {
                    assert_eq!(
                        sum[i][j],
                        (square[i][j] + a[i][j], 1),
                        "B + A for {literal}"
                    );
                }
            }
            // Determinant against the cofactor expansion, and the inverse (when it exists) against A * A^-1 = I.
            let det: i128 = if size == 2 {
                a[0][0] * a[1][1] - a[0][1] * a[1][0]
            } else {
                a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
                    - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
                    + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0])
            };
            assert_eq!(
                say(&mut session, "det(A)"),
                det.to_string(),
                "det of {literal}"
            );
            if det != 0 {
                say(&mut session, "C = inverse(A)");
                let identity = rows(&say(&mut session, "A*C"));
                for i in 0..size {
                    for j in 0..size {
                        let (n, d) = identity[i][j];
                        assert_eq!(
                            n,
                            if i == j { d } else { 0 },
                            "A * inverse(A) for {literal}"
                        );
                    }
                }
            }
            let transposed = rows(&say(&mut session, "transpose(A)"));
            for i in 0..size {
                for j in 0..size {
                    assert_eq!(transposed[i][j], (a[j][i], 1));
                }
            }
        }
    }
    // A matrix times a vector variable, and a name reused for a scalar.
    let mut session = UraeSession::new();
    session.execute_line("A = [[1,2],[3,4]]");
    session.execute_line("v = [1,2]");
    assert_eq!(session.execute_line("A*v").output_text, "[ 5 ; 11 ]");
    assert_eq!(session.execute_line("A = 5").output_text, "A = 5");
    assert_eq!(session.execute_line("A + 1").output_text, "6");
    // A scalar variable named like nothing matrix-related is unaffected.
    session.execute_line("x = 3");
    assert_eq!(session.execute_line("x + 1").output_text, "4");
}

#[test]
fn workspace_commands_list_show_clear_and_reset() {
    let mut s = UraeSession::new();
    let mut say = |q: &str| -> String {
        let r = s.execute_line(q);
        assert!(!r.is_error, "{q}: {}", r.output_text);
        r.output_text
    };
    assert_eq!(say("who"), "No variables defined.");
    say("a = 5");
    say("b = a^2 + 1");
    say("p = (x+1)^2");
    say("f(x) = x^2");
    say("A = [[1,2],[3,4]]");
    // who lists every kind of name, sorted: scalars, aliases, functions, matrices.
    let listing = say("who");
    let names: Vec<&str> = listing
        .lines()
        .map(|l| l.split(['=', '(']).next().unwrap().trim())
        .collect();
    assert_eq!(names, ["A", "a", "b", "f", "p"], "{listing}");
    assert!(
        listing.contains("A = [[1,2],[3,4]]")
            && listing.contains("b = 26")
            && listing.contains("f(x) = x^2"),
        "{listing}"
    );
    assert_eq!(say("vars"), say("vars")); // the older command still answers
    // show prints one name; an unknown name is not a command.
    assert_eq!(say("show a"), "a = 5");
    assert_eq!(say("show A"), "A = [[1,2],[3,4]]");
    assert_eq!(say("show f"), "f(x) = x^2");
    // clear / delete remove names of every kind and report the ones that were not defined.
    assert_eq!(say("clear a"), "cleared a");
    assert_eq!(say("delete p, f"), "cleared p, f");
    assert_eq!(say("clear nothing"), "nothing not defined");
    assert_eq!(say("clear A b"), "cleared A, b");
    assert_eq!(say("who"), "No variables defined.");
    // After clearing, the name is a plain symbol again.
    assert_eq!(say("a + 1"), "a + 1");
    // % is the previous result when it stands where an operand should; 17 % 5 is still a remainder.
    assert_eq!(say("7"), "7");
    assert_eq!(say("% + 1"), "8");
    assert_eq!(say("%"), "8");
    assert_eq!(say("% * 2"), "16");
    assert_eq!(say("17 % 5"), "2");
    // reset forgets everything.
    say("c = 3");
    assert_eq!(say("reset"), "Workspace reset.");
    assert_eq!(say("who"), "No variables defined.");
    assert_eq!(say("c + 1"), "c + 1");
}

#[test]
fn indexing_reads_and_writes_every_element() {
    let mut state = 0xFEED_FACE_CAFE_BEEFu64;
    let mut next = move |m: u64| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % m) as i64
    };
    for _ in 0..8 {
        let (rows, cols) = (2 + next(3) as usize, 2 + next(3) as usize);
        let mut grid: Vec<Vec<i64>> = (0..rows)
            .map(|_| (0..cols).map(|_| next(41) - 20).collect())
            .collect();
        let literal = |g: &Vec<Vec<i64>>| {
            format!(
                "[{}]",
                g.iter()
                    .map(|r| format!(
                        "[{}]",
                        r.iter()
                            .map(|x| x.to_string())
                            .collect::<Vec<_>>()
                            .join(",")
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        };
        let mut s = UraeSession::new();
        let say = |s: &mut UraeSession, q: &str| -> String {
            let r = s.execute_line(q);
            assert!(!r.is_error, "{q}: {}", r.output_text);
            r.output_text
        };
        say(&mut s, &format!("A = {}", literal(&grid)));
        // Every element by A[i,j], A[i][j] and negative indices.
        for i in 0..rows {
            for j in 0..cols {
                assert_eq!(
                    say(&mut s, &format!("A[{},{}]", i + 1, j + 1)),
                    grid[i][j].to_string()
                );
                assert_eq!(
                    say(&mut s, &format!("A[{}][{}]", i + 1, j + 1)),
                    grid[i][j].to_string()
                );
                assert_eq!(
                    say(
                        &mut s,
                        &format!("A[{},{}]", i as i64 - rows as i64, j as i64 - cols as i64)
                    ),
                    grid[i][j].to_string()
                );
            }
        }
        // Shape queries and the diagonal.
        assert_eq!(say(&mut s, "size(A)"), format!("{rows} x {cols}"));
        assert_eq!(say(&mut s, "rows(A)"), rows.to_string());
        assert_eq!(say(&mut s, "ncols(A)"), cols.to_string());
        let diagonal: Vec<String> = (0..rows.min(cols))
            .map(|k| grid[k][k].to_string())
            .collect();
        let expected = if diagonal.len() == 1 {
            diagonal[0].clone()
        } else {
            format!("[ {} ]", diagonal.join(", "))
        };
        assert_eq!(say(&mut s, "diag(A)"), expected);
        // A row and a column slice.
        let row0: Vec<String> = grid[0].iter().map(|x| x.to_string()).collect();
        assert_eq!(say(&mut s, "row(A, 1)"), format!("[ {} ]", row0.join(", ")));
        assert_eq!(say(&mut s, "A[1,:]"), format!("[ {} ]", row0.join(", ")));
        let col0: Vec<String> = grid.iter().map(|r| r[0].to_string()).collect();
        assert_eq!(say(&mut s, "A[:,1]"), format!("[ {} ]", col0.join(" ; ")));
        // Element assignment, then the whole matrix and the element read back.
        let (i, j, value) = (
            next(rows as u64) as usize,
            next(cols as u64) as usize,
            next(100) - 50,
        );
        say(&mut s, &format!("A[{},{}] = {value}", i + 1, j + 1));
        grid[i][j] = value;
        assert_eq!(
            say(&mut s, &format!("A[{},{}]", i + 1, j + 1)),
            value.to_string()
        );
        let shown = grid
            .iter()
            .map(|r| {
                r.iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .collect::<Vec<_>>()
            .join(" ; ");
        assert_eq!(say(&mut s, "A"), format!("[ {shown} ]"));
        // The edited matrix feeds arithmetic: its trace is the sum of the diagonal.
        let trace: i64 = (0..rows.min(cols)).map(|k| grid[k][k]).sum();
        if rows == cols {
            assert_eq!(say(&mut s, "trace(A)"), trace.to_string());
        }
        // Out of range and zero are errors, not wrapped values.
        assert!(s.execute_line(&format!("A[{},1]", rows + 1)).is_error);
        assert!(s.execute_line("A[0,1]").is_error);
    }
    // Lists: one index, negative from the end, assignment.
    let mut s = UraeSession::new();
    s.execute_line("v = [10,20,30]");
    assert_eq!(s.execute_line("v[1]").output_text, "10");
    assert_eq!(s.execute_line("v[-1]").output_text, "30");
    assert_eq!(s.execute_line("[10,20,30][2]").output_text, "20");
    assert!(s.execute_line("v[4]").is_error);
    s.execute_line("v[2] = 99");
    assert_eq!(s.execute_line("v").output_text, "[ 10, 99, 30 ]");
    // A flat list is still turned into a diagonal matrix by diag.
    assert_eq!(
        s.execute_line("diag([1,2,3])").output_text,
        "[ 1, 0, 0 ; 0, 2, 0 ; 0, 0, 3 ]"
    );
    // Symbolic cells.
    s.execute_line("B = [[a,b],[c,d]]");
    assert_eq!(s.execute_line("B[1,2]").output_text, "b");
    s.execute_line("B[2,2] = x+1");
    assert_eq!(s.execute_line("B").output_text, "[ a, b ; c, x + 1 ]");
}

#[test]
fn inverse_laplace_recovers_the_time_function_and_finance_matches_formulas() {
    let number = |text: String| -> f64 {
        let t = text.rsplit(" ≈ ").next().unwrap().trim().to_string();
        match t.split_once('/') {
            Some((a, b)) => a.parse::<f64>().unwrap() / b.parse::<f64>().unwrap(),
            None => t.parse().unwrap_or_else(|_| panic!("not a number: {text}")),
        }
    };
    // Build f(t) from standard pieces in Rust, write its transform F(s), invert with the engine, and compare the engine's
    // answer with f at several times.
    //   c e^(a t)                    <->  c / (s - a)
    //   c t^n e^(a t)                <->  c n! / (s - a)^(n+1)
    //   e^(a t)(c cos wt + d sin wt) <->  (c (s - a) + d w) / ((s - a)^2 + w^2)
    #[derive(Clone, Copy)]
    enum Piece {
        Exp { c: f64, a: i32 },
        Poly { c: f64, n: u32, a: i32 },
        Wave { c: f64, d: f64, a: i32, w: i32 },
    }
    let factorial = |n: u32| (1..=n).product::<u32>() as f64;
    let cases: [Vec<Piece>; 6] = [
        vec![Piece::Exp { c: 3.0, a: -2 }],
        vec![Piece::Poly { c: 2.0, n: 2, a: 1 }],
        vec![Piece::Wave {
            c: 1.0,
            d: 2.0,
            a: -1,
            w: 2,
        }],
        vec![Piece::Exp { c: 1.0, a: 1 }, Piece::Exp { c: -2.0, a: -3 }],
        vec![
            Piece::Poly {
                c: 1.0,
                n: 1,
                a: -1,
            },
            Piece::Wave {
                c: 0.0,
                d: 1.0,
                a: 0,
                w: 3,
            },
        ],
        vec![
            Piece::Exp { c: 4.0, a: 0 },
            Piece::Wave {
                c: 1.0,
                d: -1.0,
                a: 1,
                w: 1,
            },
            Piece::Poly {
                c: 3.0,
                n: 0,
                a: -2,
            },
        ],
    ];
    for pieces in cases {
        let transform: Vec<String> = pieces
            .iter()
            .map(|p| match *p {
                Piece::Exp { c, a } => format!("({c})/(s - ({a}))"),
                Piece::Poly { c, n, a } => format!("({})/(s - ({a}))^{}", c * factorial(n), n + 1),
                Piece::Wave { c, d, a, w } => format!(
                    "(({c})*(s - ({a})) + ({})) / ((s - ({a}))^2 + {})",
                    d * w as f64,
                    w * w
                ),
            })
            .collect();
        let f = transform.join(" + ");
        let out = run(&format!("ilaplace({f}, s, t)"));
        let expression = out.split_once(" = ").unwrap().1.to_string();
        for t in [0.3f64, 0.7, 1.0, 1.9] {
            let expected: f64 = pieces
                .iter()
                .map(|p| match *p {
                    Piece::Exp { c, a } => c * (a as f64 * t).exp(),
                    Piece::Poly { c, n, a } => c * t.powi(n as i32) * (a as f64 * t).exp(),
                    Piece::Wave { c, d, a, w } => {
                        (a as f64 * t).exp() * (c * (w as f64 * t).cos() + d * (w as f64 * t).sin())
                    }
                })
                .sum();
            let got = number(run(&format!("subs(({expression}), t, {t:?})")));
            assert!(
                (got - expected).abs() < 1e-9 * (1.0 + expected.abs()),
                "F = {f}: f({t}) = {got}, expected {expected}\n{out}"
            );
        }
    }
    // Exact forms.
    assert_eq!(
        run("ilaplace(1/(s^2+1), s, t)"),
        "L^-1{1/(s^2+1)}(t) = sin(t)"
    );
    assert_eq!(run("ilaplace(1/s^2)"), "L^-1{1/s^2}(t) = t");
    assert!(
        UraeSession::new()
            .execute_line("ilaplace(s^2/(s^2+1), s, t)")
            .is_error
    );
    // Finance against the closed forms.
    for (p, r, n) in [
        (1000.0f64, 0.05f64, 10i32),
        (2500.0, 0.035, 7),
        (10.0, 0.2, 3),
    ] {
        let growth = (1.0 + r).powi(n);
        assert!(
            (number(run(&format!("compound_interest({p}, {r}, {n})"))) - p * growth).abs()
                < 1e-9 * p * growth
        );
        assert!(
            (number(run(&format!("present_value({p}, {r}, {n})"))) - p / growth).abs() < 1e-9 * p
        );
        assert!(
            (number(run(&format!("annuity({p}, {r}, {n})"))) - p * (1.0 - 1.0 / growth) / r).abs()
                < 1e-9 * p * 10.0
        );
        // pmt is the payment that the annuity formula turns back into the loan.
        let payment = number(run(&format!("pmt({r}, {n}, {p})")));
        assert!(
            (payment * (1.0 - 1.0 / growth) / r - p).abs() < 1e-9 * p,
            "pmt({r}, {n}, {p}) = {payment}"
        );
    }
    assert_eq!(run("simple_interest(1000, 1/20, 3)"), "150");
    assert_eq!(run("percent_change(80, 100)"), "25");
    // A 30-year mortgage: the payment of 200000 at 6% a year is the published 1199.10.
    assert!((number(run("loan_payment(200000, 0.06, 30)")) - 1199.1010503).abs() < 1e-6);
    // An exact short rational gets no extra value, a long one does.
    assert_eq!(run("0.1 + 0.2"), "3/10");
    assert!(run("1000*(1+0.05)^10").contains(" ≈ 1628.89462677744"));
}

#[test]
fn npv_irr_cagr_and_doubling_time_are_self_consistent() {
    let number = |text: String| -> f64 {
        let t = text.rsplit(" ≈ ").next().unwrap().trim().to_string();
        match t.split_once('/') {
            Some((a, b)) => a.parse::<f64>().unwrap() / b.parse::<f64>().unwrap(),
            None => t.parse().unwrap_or_else(|_| panic!("not a number: {text}")),
        }
    };
    // npv is the discounted sum of the flows, the first at time 0 (exact rationals against f64).
    for (rate, flows) in [
        (0.1f64, vec![-100.0, 50.0, 60.0]),
        (0.05, vec![-1000.0, 300.0, 400.0, 500.0]),
        (0.2, vec![10.0, 10.0, 10.0]),
    ] {
        let list = format!(
            "[{}]",
            flows
                .iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
        let expected: f64 = flows
            .iter()
            .enumerate()
            .map(|(k, f)| f / (1.0 + rate).powi(k as i32))
            .sum();
        let got = number(run(&format!("npv({rate:?}, {list})")));
        assert!(
            (got - expected).abs() < 1e-9,
            "npv({rate}, {list}) = {got}, expected {expected}"
        );
    }
    assert_eq!(run("npv(0.1, [-100, 50, 60])"), "-600/121");
    assert_eq!(run("npv(0, [1, 2, 3])"), "6");
    // The internal rate of return is the rate at which the npv vanishes.
    for flows in [
        "[-100, 50, 60]",
        "[-1000, 300, 400, 500]",
        "[-50, 20, 20, 20, 20]",
        "[-10, 0, 0, 15]",
    ] {
        let rate = number(run(&format!("irr({flows})")));
        assert!(rate > -0.99 && rate < 100.0);
        let residual = number(run(&format!("npv({rate:?}, {flows})")));
        assert!(
            residual.abs() < 1e-8,
            "irr({flows}) = {rate}, npv there = {residual}"
        );
    }
    // Flows that never change sign have no internal rate of return.
    assert!(UraeSession::new().execute_line("irr([100, 50])").is_error);
    // cagr: (1 + cagr)^years = end / start; doubling time: (1 + r)^T = 2.
    for (start, end, years) in [
        (100.0f64, 200.0f64, 5.0f64),
        (1000.0, 1500.0, 3.0),
        (50.0, 40.0, 4.0),
    ] {
        let growth = number(run(&format!("cagr({start:?}, {end:?}, {years:?})")));
        assert!(
            ((1.0 + growth).powf(years) - end / start).abs() < 1e-12,
            "cagr({start}, {end}, {years})"
        );
    }
    for rate in [0.01f64, 0.07, 0.5, 2.0] {
        let time = number(run(&format!("doubling_time({rate:?})")));
        assert!(
            ((1.0 + rate).powf(time) - 2.0).abs() < 1e-9,
            "doubling_time({rate}) = {time}"
        );
    }
    assert_eq!(run("doubling_time(1)"), "1");
}

#[test]
fn mixed_partial_derivatives_and_one_sided_limit_shorthand() {
    // d^(a+b) / dx^a dy^b of x^m y^n is m!/(m-a)! n!/(n-b)! x^(m-a) y^(n-b), whatever the order of the variables.
    let falling = |m: i64, a: i64| -> i64 { (0..a).map(|k| m - k).product() };
    for (m, n) in [(2i64, 3i64), (4, 2), (3, 3), (5, 1)] {
        for a in 0..=m.min(3) {
            for b in 0..=n.min(3) {
                if a + b < 2 || a == 0 || b == 0 {
                    continue;
                }
                let coefficient = falling(m, a) * falling(n, b);
                let expected = {
                    let (px, py) = (m - a, n - b);
                    let x = match px {
                        0 => String::new(),
                        1 => "x".to_string(),
                        p => format!("x^{p}"),
                    };
                    let y = match py {
                        0 => String::new(),
                        1 => "y".to_string(),
                        p => format!("y^{p}"),
                    };
                    let factors: Vec<String> =
                        [x, y].into_iter().filter(|f| !f.is_empty()).collect();
                    let body = factors.join(" · ");
                    match (coefficient, body.is_empty()) {
                        (c, true) => c.to_string(),
                        (1, false) => body,
                        (c, false) => format!("{c} · {body}"),
                    }
                };
                let f = format!("x^{m}*y^{n}");
                let by_x_then_y = run(&format!("diff({f}, x, {a}, y, {b})"));
                let by_y_then_x = run(&format!("diff({f}, y, {b}, x, {a})"));
                assert_eq!(by_x_then_y, expected, "diff({f}, x, {a}, y, {b})");
                // Clairaut: the order of differentiation does not matter.
                assert_eq!(by_y_then_x, expected, "diff({f}, y, {b}, x, {a})");
            }
        }
    }
    // Repeating a variable is the same as giving its order, and the spellings agree.
    assert_eq!(run("diff(x^2*y^3, x, x, y)"), run("diff(x^2*y^3, x, 2, y)"));
    for spelling in [
        "partial(x^2*y^3, x, y)",
        "mixed_partial(x^2*y^3, x, y)",
        "∂^2/∂x∂y (x^2*y^3)",
        "d^2/dxdy (x^2*y^3)",
        "diff(x^2*y^3, y, x)",
    ] {
        assert_eq!(run(spelling), "6 · x · y^2", "{spelling}");
    }
    assert_eq!(run("d^3/dx^2dy x^2*y^3"), "6 · y^2");
    assert_eq!(
        run("diff(sin(x*y), x, y)"),
        "-x · y · sin(x · y) + cos(x · y)"
    );
    // The ordinary derivatives are unchanged.
    assert_eq!(run("diff(x^3, x)"), "3 · x^2");
    assert_eq!(run("diff(x^2*y^3, x, 2)"), "2 · y^3");
    assert_eq!(run("d^2/dx^2 x^4"), "12 · x^2");
    // One-sided limits written as a sentence equal the call form.
    for (short, explicit) in [
        ("lim x->0+ 1/x", "limit(1/x, x, 0+)"),
        ("lim x->0- 1/x", "limit(1/x, x, 0-)"),
        ("limit x -> 2- 1/(x-2)", "limit(1/(x-2), x, 2-)"),
        ("limit(1/x, x, 0^+)", "limit(1/x, x, 0+)"),
        ("limit(1/x, x, 0^-)", "limit(1/x, x, 0-)"),
    ] {
        assert_eq!(run(short), run(explicit), "{short}");
    }
    assert!(run("lim x->0+ 1/x").ends_with("= ∞"));
    assert!(run("lim x->0- 1/x").ends_with("= -∞"));
    assert!(run("lim x->0+ abs(x)/x").ends_with("= 1"));
}

#[test]
fn linear_ode_systems_satisfy_their_equations_and_initial_conditions() {
    let number = |text: String| -> f64 {
        let t = text.rsplit(" ≈ ").next().unwrap().trim().to_string();
        match t.split_once('/') {
            Some((a, b)) => a.parse::<f64>().unwrap() / b.parse::<f64>().unwrap(),
            None => t.parse().unwrap_or_else(|_| panic!("not a number: {text}")),
        }
    };
    let mut state = 0x0DDB_A11C_AFE5_EED5u64;
    let mut next = move |m: u64| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % m) as i64
    };
    let names = ["x", "y", "z"];
    let mut solved = 0;
    for size in [2usize, 2, 2, 2, 2, 2, 3, 3, 3, 3] {
        let a: Vec<Vec<i64>> = (0..size)
            .map(|_| (0..size).map(|_| next(5) - 2).collect())
            .collect();
        let initial: Vec<i64> = (0..size).map(|_| next(7) - 3).collect();
        let equations: Vec<String> = (0..size)
            .map(|i| {
                let rhs: Vec<String> = (0..size)
                    .map(|j| format!("({})*{}", a[i][j], names[j]))
                    .collect();
                format!("{}' = {}", names[i], rhs.join(" + "))
            })
            .collect();
        let conditions: Vec<String> = (0..size)
            .map(|i| format!("{}(0) = {}", names[i], initial[i]))
            .collect();
        let result = UraeSession::new().execute_line(&format!(
            "dsolve([{}], {})",
            equations.join(", "),
            conditions.join(", ")
        ));
        if result.is_error {
            // An irreducible factor of degree 3 or more is reported, not guessed.
            assert!(
                result.output_text.contains("not supported")
                    || result
                        .error_msg
                        .clone()
                        .unwrap_or_default()
                        .contains("not supported"),
                "{equations:?}: {}",
                result.output_text
            );
            continue;
        }
        solved += 1;
        let parts: Vec<String> = result.output_text.split(", ").map(str::to_string).collect();
        // Each part is "x(t) = expression"; commas inside an expression do not occur.
        assert_eq!(parts.len(), size, "{}", result.output_text);
        let functions: Vec<String> = parts
            .iter()
            .map(|p| p.split_once(" = ").unwrap().1.to_string())
            .collect();
        let at = |i: usize, t: f64| number(run(&format!("subs(({}), t, {t:?})", functions[i])));
        for (i, value) in initial.iter().enumerate() {
            assert!(
                (at(i, 0.0) - *value as f64).abs() < 1e-9,
                "{equations:?}: {}(0) = {} expected {value}",
                names[i],
                at(i, 0.0)
            );
        }
        for t in [0.3f64, 0.8] {
            let h = 1e-5;
            for i in 0..size {
                let derivative = (at(i, t + h) - at(i, t - h)) / (2.0 * h);
                let right: f64 = (0..size).map(|j| a[i][j] as f64 * at(j, t)).sum();
                assert!(
                    (derivative - right).abs() < 1e-5 * (1.0 + right.abs()),
                    "{equations:?} at t = {t}: {}' = {derivative}, A v = {right}",
                    names[i]
                );
            }
        }
    }
    assert!(solved >= 6, "only {solved} systems were solved");
    // Known closed forms and refusals.
    assert_eq!(
        run("dsolve([x' = y, y' = -x], x(0) = 1, y(0) = 0)"),
        "x(t) = cos(t), y(t) = -sin(t)"
    );
    assert_eq!(
        run("dsolve([x' = x, y' = 2*y])"),
        "x(t) = C1 · exp(t), y(t) = C2 · exp(2 · t)"
    );
    assert!(
        UraeSession::new()
            .execute_line("dsolve([x' = x*y, y' = y])")
            .is_error
    );
}

#[test]
fn amortization_schedules_balance_to_zero() {
    for (principal, rate, years) in [
        (10_000.0f64, 0.06f64, 1u32),
        (200_000.0, 0.06, 30),
        (5_000.0, 0.12, 2),
        (75_000.0, 0.045, 5),
    ] {
        let months = (years * 12) as usize;
        let out = run(&format!(
            "amortization({principal:?}, {rate:?}, {years}, {months})"
        ));
        let mut lines = out.lines();
        let header = lines.next().unwrap();
        let payment: f64 = header
            .split("payment = ")
            .nth(1)
            .unwrap()
            .split(' ')
            .next()
            .unwrap()
            .parse()
            .unwrap();
        // The payment against the closed form.
        let i = rate / 12.0;
        let exact = principal * i / (1.0 - (1.0 + i).powi(-(months as i32)));
        assert!(
            (payment - exact).abs() <= 0.0051,
            "payment {payment} vs {exact}"
        );
        let (mut balance, mut principal_paid, mut rows) = (principal, 0.0f64, 0);
        for (k, line) in lines.enumerate() {
            let cells: Vec<f64> = line
                .split(" | ")
                .skip(1)
                .map(|c| c.trim().parse().unwrap())
                .collect();
            let (interest, repaid, shown) = (cells[0], cells[1], cells[2]);
            // Each row against the recurrence: interest = balance * i, balance falls by the principal repaid.
            let expected_interest = balance * i;
            assert!(
                (interest - expected_interest).abs() <= 0.0101,
                "row {}: interest {interest} vs {expected_interest}",
                k + 1
            );
            balance = balance * (1.0 + i) - exact;
            assert!(
                (shown - balance).abs() <= 0.02 * (k as f64 + 1.0).min(5.0) + 0.0101,
                "row {}: balance {shown} vs {balance}",
                k + 1
            );
            // Interest plus principal is the payment (to the cent).
            assert!((interest + repaid - payment).abs() <= 0.0101);
            principal_paid += repaid;
            rows += 1;
        }
        assert_eq!(rows, months);
        // The loan is repaid in full: the last balance is zero and the principal repaid adds up to the loan.
        assert!(out.lines().last().unwrap().ends_with("| 0.00"), "{out}");
        assert!(
            (principal_paid - principal).abs() <= 0.01 * months as f64 * 0.5 + 0.02,
            "principal repaid {principal_paid}"
        );
    }
    // The first twelve rows by default, a zero rate repays in equal parts.
    assert_eq!(run("amortization(10000, 0.06, 2)").lines().count(), 13);
    let level = run("amortization(1200, 0, 1, 3)");
    assert!(
        level.starts_with("payment = 100.00 per month")
            && level.contains("3 | 0.00 | 100.00 | 900.00"),
        "{level}"
    );
}

#[test]
fn forced_linear_systems_satisfy_their_equations() {
    let number = |text: String| -> f64 {
        let t = text.rsplit(" ≈ ").next().unwrap().trim().to_string();
        match t.split_once('/') {
            Some((a, b)) => a.parse::<f64>().unwrap() / b.parse::<f64>().unwrap(),
            None => t.parse().unwrap_or_else(|_| panic!("not a number: {text}")),
        }
    };
    // Forcing terms with their values at t.
    type Forcing = (&'static str, fn(f64) -> f64);
    let forcings: [Forcing; 5] = [
        ("1", |_| 1.0),
        ("2*t", |t| 2.0 * t),
        ("exp(t)", f64::exp),
        ("sin(t)", f64::sin),
        ("cos(2*t)", |t| (2.0 * t).cos()),
    ];
    let mut state = 0xBADC_0FFE_E0DD_F00Du64;
    let mut next = move |m: u64| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % m) as i64
    };
    let mut solved = 0;
    for _ in 0..14 {
        let a: Vec<Vec<i64>> = (0..2)
            .map(|_| (0..2).map(|_| next(5) - 2).collect())
            .collect();
        let (force_row, force_index) = (next(2) as usize, next(5) as usize);
        let initial = [next(5) - 2, next(5) - 2];
        let names = ["x", "y"];
        let equations: Vec<String> = (0..2)
            .map(|i| {
                let mut rhs = format!("({})*x + ({})*y", a[i][0], a[i][1]);
                if i == force_row {
                    rhs.push_str(&format!(" + {}", forcings[force_index].0));
                }
                format!("{}' = {rhs}", names[i])
            })
            .collect();
        let result = UraeSession::new().execute_line(&format!(
            "dsolve([{}], x(0) = {}, y(0) = {})",
            equations.join(", "),
            initial[0],
            initial[1]
        ));
        assert!(!result.is_error, "{equations:?}: {}", result.output_text);
        solved += 1;
        let functions: Vec<String> = result
            .output_text
            .split(", ")
            .map(|p| p.split_once(" = ").unwrap().1.to_string())
            .collect();
        assert_eq!(functions.len(), 2, "{}", result.output_text);
        let at = |i: usize, t: f64| number(run(&format!("subs(({}), t, {t:?})", functions[i])));
        for (i, value) in initial.iter().enumerate() {
            assert!(
                (at(i, 0.0) - *value as f64).abs() < 1e-9,
                "{equations:?}: {}(0)",
                names[i]
            );
        }
        for t in [0.4f64, 0.9] {
            let h = 1e-5;
            for i in 0..2 {
                let derivative = (at(i, t + h) - at(i, t - h)) / (2.0 * h);
                let mut right: f64 = (0..2).map(|j| a[i][j] as f64 * at(j, t)).sum();
                if i == force_row {
                    right += (forcings[force_index].1)(t);
                }
                assert!(
                    (derivative - right).abs() < 1e-5 * (1.0 + right.abs()),
                    "{equations:?} at t = {t}: {}' = {derivative}, expected {right}",
                    names[i]
                );
            }
        }
    }
    assert_eq!(solved, 14);
    assert_eq!(
        run("dsolve([x' = y, y' = 2*t])"),
        "x(t) = C2 · t + C1 + 1/3 · t^3, y(t) = C2 + t^2"
    );
    assert_eq!(
        run("dsolve([x' = -x + 1, y' = x - y], x(0) = 0, y(0) = 0)"),
        "x(t) = -exp(-t) + 1, y(t) = -t · exp(-t) - exp(-t) + 1"
    );
    // Nonlinear terms and forcing that depends on the unknowns are still refused.
    assert!(
        UraeSession::new()
            .execute_line("dsolve([x' = x*y, y' = y])")
            .is_error
    );
    assert!(
        UraeSession::new()
            .execute_line("dsolve([x' = x + a, y' = y])")
            .is_error
    );
}

#[test]
fn homogeneous_and_exact_first_order_equations_satisfy_themselves() {
    let number = |text: String| -> f64 {
        let t = text.rsplit(" ≈ ").next().unwrap().trim().to_string();
        match t.split_once('/') {
            Some((a, b)) => a.parse::<f64>().unwrap() / b.parse::<f64>().unwrap(),
            None => t.parse().unwrap_or_else(|_| panic!("not a number: {text}")),
        }
    };
    // y' = F(x, y) as a closure, for the checks.
    type Rhs = fn(f64, f64) -> f64;
    // Homogeneous equations y' = F(y/x): the solution is explicit or an implicit relation psi(x, y) = ln x + C1.
    let homogeneous: [(&str, Rhs); 4] = [
        ("(x^2+y^2)/(x*y)", |x, y| (x * x + y * y) / (x * y)),
        ("(y-x)/(y+x)", |x, y| (y - x) / (y + x)),
        ("(x+y)/(x-y)", |x, y| (x + y) / (x - y)),
        ("(x+y)/x", |x, y| (x + y) / x),
    ];
    for (rhs, f) in homogeneous {
        let out = run(&format!("dsolve(y' = {rhs})"));
        assert!(
            !out.starts_with("Differential Equation Analysis"),
            "{rhs}: {out}"
        );
        if let Some((relation, _)) = out.split_once("  (implicit") {
            // psi(x, y) = ln x + C1: along a solution d(psi - ln x) = 0, so psi_x - 1/x + psi_y F = 0.
            let psi = relation.split_once(" = ln(x) + C1").unwrap().0.to_string();
            let at = |x: f64, y: f64| number(run(&format!("subs(({psi}), x = {x:?}, y = {y:?})")));
            for (x, y) in [(1.5f64, 0.7f64), (2.3, -0.4), (0.8, 1.9)] {
                let h = 1e-5;
                let psi_x = (at(x + h, y) - at(x - h, y)) / (2.0 * h);
                let psi_y = (at(x, y + h) - at(x, y - h)) / (2.0 * h);
                let residual = psi_x - 1.0 / x + psi_y * f(x, y);
                assert!(
                    residual.abs() < 1e-5 * (1.0 + psi_y.abs() * f(x, y).abs()),
                    "{rhs}: residual {residual} at ({x}, {y})\n{out}"
                );
            }
        } else {
            // y(x) = expression [or ...]: every branch differentiates to F(x, y(x)).
            for branch in out.split("  or  ") {
                let expression = branch.split_once(" = ").unwrap().1.to_string();
                let y_at =
                    |x: f64| number(run(&format!("subs(({expression}), x = {x:?}, C1 = 0.5)")));
                for x in [1.6f64, 2.4, 3.1] {
                    let h = 1e-5;
                    let derivative = (y_at(x + h) - y_at(x - h)) / (2.0 * h);
                    let expected = f(x, y_at(x));
                    assert!(
                        (derivative - expected).abs() < 1e-5 * (1.0 + expected.abs()),
                        "{rhs}: y' = {derivative}, F = {expected} at {x}\n{out}"
                    );
                }
            }
        }
    }
    // Exact equations M + N y' = 0 give psi(x, y) = C1 with psi_x = M and psi_y = N.
    type Pair = (&'static str, Rhs, Rhs);
    let exact: [Pair; 4] = [
        (
            "(2*x*y + 1) + (x^2 + 2*y)*y' = 0",
            |x, y| 2.0 * x * y + 1.0,
            |x, y| x * x + 2.0 * y,
        ),
        (
            "(3*x^2 + y) + (x + 2*y)*y' = 0",
            |x, y| 3.0 * x * x + y,
            |x, y| x + 2.0 * y,
        ),
        (
            "(2*x + y) + (x + 2*y)*y' = 0",
            |x, y| 2.0 * x + y,
            |x, y| x + 2.0 * y,
        ),
        (
            "(y*cos(x) + 2*x) + (sin(x) + 3*y^2)*y' = 0",
            |x, y| y * x.cos() + 2.0 * x,
            |x, y| x.sin() + 3.0 * y * y,
        ),
    ];
    for (equation, m, n) in exact {
        let out = run(&format!("dsolve({equation})"));
        assert!(
            out.ends_with("(implicit solution of the exact equation)"),
            "{equation}: {out}"
        );
        let psi = out.split_once(" = C1").unwrap().0.to_string();
        let at = |x: f64, y: f64| number(run(&format!("subs(({psi}), x = {x:?}, y = {y:?})")));
        for (x, y) in [(0.6f64, 0.9f64), (1.3, -0.5), (2.1, 1.4)] {
            let h = 1e-5;
            let psi_x = (at(x + h, y) - at(x - h, y)) / (2.0 * h);
            let psi_y = (at(x, y + h) - at(x, y - h)) / (2.0 * h);
            // psi is determined up to a constant and a common factor of 1 here: psi_x = M, psi_y = N.
            assert!(
                (psi_x - m(x, y)).abs() < 1e-5 * (1.0 + m(x, y).abs()),
                "{equation}: psi_x = {psi_x}, M = {}",
                m(x, y)
            );
            assert!(
                (psi_y - n(x, y)).abs() < 1e-5 * (1.0 + n(x, y).abs()),
                "{equation}: psi_y = {psi_y}, N = {}",
                n(x, y)
            );
        }
    }
    // Non-exact equations with an integrating factor: psi_x = mu M and psi_y = mu N, so psi_x N - psi_y M = 0.
    let factored: [Pair; 3] = [
        ("y + (x^2*y - x)*y' = 0", |_, y| y, |x, y| x * x * y - x),
        (
            "(x^2 + y^2 + x) + x*y*y' = 0",
            |x, y| x * x + y * y + x,
            |x, y| x * y,
        ),
        (
            "(3*x*y + y^2) + (x^2 + x*y)*y' = 0",
            |x, y| 3.0 * x * y + y * y,
            |x, y| x * x + x * y,
        ),
    ];
    for (equation, m, n) in factored {
        let out = run(&format!("dsolve({equation})"));
        if out.starts_with("y(x)") {
            continue; // an explicit solution is checked by the homogeneous test above
        }
        assert!(
            out.contains("integrating factor") || out.contains("exact"),
            "{equation}: {out}"
        );
        let psi = out.split_once(" = C1").unwrap().0.to_string();
        let at = |x: f64, y: f64| number(run(&format!("subs(({psi}), x = {x:?}, y = {y:?})")));
        for (x, y) in [(0.7f64, 0.9f64), (1.4, -0.6), (2.2, 1.3)] {
            let h = 1e-5;
            let psi_x = (at(x + h, y) - at(x - h, y)) / (2.0 * h);
            let psi_y = (at(x, y + h) - at(x, y - h)) / (2.0 * h);
            let residual = psi_x * n(x, y) - psi_y * m(x, y);
            assert!(
                residual.abs() < 1e-5 * (1.0 + (psi_x * n(x, y)).abs()),
                "{equation}: residual {residual} at ({x}, {y}): {out}"
            );
        }
    }
    // The coefficient of y' is not assumed to be 1: these were solved as if it were (2 y' = y gave exp(x)).
    type Known = (&'static str, fn(f64) -> f64);
    let known: [Known; 4] = [
        ("2*y' = y", |x: f64| (x / 2.0).exp()),
        ("(1+x^2)*y' = x*y", |x: f64| (1.0 + x * x).sqrt()),
        ("x*y' = y", |x: f64| x),
        ("3*y' + 6*y = 0", |x: f64| (-2.0 * x).exp()),
    ];
    for (equation, solution) in known {
        let out = run(&format!("dsolve({equation})"));
        let expression = out
            .split("  or  ")
            .next()
            .unwrap()
            .split_once(" = ")
            .unwrap()
            .1
            .to_string();
        let y_at = |x: f64| number(run(&format!("subs(({expression}), x = {x:?}, C1 = 1.0)")));
        for x in [0.5f64, 1.2, 2.0] {
            let ratio = y_at(x) / solution(x);
            let other = y_at(x + 0.7) / solution(x + 0.7);
            assert!(
                (ratio - other).abs() < 1e-9 * (1.0 + ratio.abs()),
                "{equation}: not a constant multiple of the true solution ({out})"
            );
        }
    }
    assert_eq!(run("dsolve(2*y' = y)"), "y(x) = C1 · exp(1/2 · x)");
    // The simplifier folds exp(c ln u) into a power, and a negated negative coefficient prints without a double sign.
    assert_eq!(run("exp(-2*ln(x))"), "x^(-2)");
    assert_eq!(run("exp(ln(x)/2)"), "x^(1/2)");
    assert_eq!(run("-(-1/2*x)"), "1/2 · x");
    // Equations the standard solver answers keep its answer.
    assert_eq!(run("dsolve(y' = y)"), "y(x) = C1 · exp(x)");
    assert_eq!(run("dsolve(y' = y/x + 1)"), "y(x) = x · (ln(x) + C1)");
}

#[test]
fn separable_equations_in_quotient_form_are_solved() {
    let number = |text: String| -> f64 {
        let t = text.rsplit(" ≈ ").next().unwrap().trim().to_string();
        match t.split_once('/') {
            Some((a, b)) => a.parse::<f64>().unwrap() / b.parse::<f64>().unwrap(),
            None => t.parse().unwrap_or_else(|_| panic!("not a number: {text}")),
        }
    };
    // y' = F(x, y): the solution is explicit (differentiate it) or g(y) = f(x) + C1 (g'(y) F = f'(x)).
    type Rhs = fn(f64, f64) -> f64;
    let cases: [(&str, Rhs); 8] = [
        ("1/y^2", |_, y| 1.0 / (y * y)),
        ("x/y^2", |x, y| x / (y * y)),
        ("1/(y^2+1)", |_, y| 1.0 / (y * y + 1.0)),
        ("(x^2+1)/(y^2+1)", |x, y| (x * x + 1.0) / (y * y + 1.0)),
        ("x*(1+y^2)", |x, y| x * (1.0 + y * y)),
        ("x + x*y^2", |x, y| x + x * y * y),
        ("exp(x - y)", |x, y| (x - y).exp()),
        ("y^2/x", |x, y| y * y / x),
    ];
    for (rhs, f) in cases {
        let out = run(&format!("dsolve(y' = {rhs})"));
        assert!(
            !out.starts_with("Differential Equation Analysis"),
            "{rhs}: {out}"
        );
        if let Some((relation, _)) = out.split_once("  (implicit") {
            let (g, rest) = relation.split_once(" = ").unwrap();
            let fx = rest.strip_suffix(" + C1").unwrap_or(rest).to_string();
            let g = g.to_string();
            let g_at = |y: f64| number(run(&format!("subs(({g}), y = {y:.4})")));
            let f_at = |x: f64| number(run(&format!("subs(({fx}), x = {x:.4})")));
            for (x, y) in [(0.8f64, 0.6f64), (1.4, 1.1), (2.0, 0.4)] {
                let h = 1e-3;
                let g_prime = (g_at(y + h) - g_at(y - h)) / (2.0 * h);
                let f_prime = (f_at(x + h) - f_at(x - h)) / (2.0 * h);
                assert!(
                    (g_prime * f(x, y) - f_prime).abs() < 1e-4 * (1.0 + f_prime.abs()),
                    "{rhs}: g' F = {}, f' = {f_prime}\n{out}",
                    g_prime * f(x, y)
                );
            }
        } else {
            for branch in out.split("  or  ") {
                let expression = branch.split_once(" = ").unwrap().1.to_string();
                let y_at =
                    |x: f64| number(run(&format!("subs(({expression}), x = {x:.4}, C1 = 1.5)")));
                for x in [1.1f64, 1.7, 2.3] {
                    let h = 1e-3;
                    let derivative = (y_at(x + h) - y_at(x - h)) / (2.0 * h);
                    let expected = f(x, y_at(x));
                    assert!(
                        (derivative - expected).abs() < 1e-4 * (1.0 + expected.abs()),
                        "{rhs}: y' = {derivative}, F = {expected} at {x}\n{out}"
                    );
                }
            }
        }
    }
    // Equations that were solved before keep their form.
    assert_eq!(run("dsolve(y' = y^2)"), "y(x) = -(x + C1)^(-1)");
    // The simplifier: quotients and powers of powers with integer exponents.
    assert_eq!(run("1/((y^2+1)^(-1))"), "y^2 + 1");
    assert_eq!(run("((y^2+1)^(-1))^(-1)"), "y^2 + 1");
    assert_eq!(run("(1/(y^2+1))^(-1)"), "y^2 + 1");
    assert_eq!(run("(a/b)^(-2)"), "(b / a)^2");
    assert_eq!(run("(x^2)^3"), "x^6");
    assert_eq!(run("x/(y^(-2))"), "x · y^2");
    // (x^2)^(1/2) is not x: a fractional outer exponent is left alone.
    assert_eq!(run("(x^2)^(1/2)"), "(x^2)^(1/2)");
}

#[test]
fn rationals_beyond_64_bits_stay_numbers_in_powers_and_quotients() {
    // The e-graph carried such numbers as symbols named by their text: (a/b)^x came back as the symbol `a/b` to the power x,
    // printed `a/b^x`, and the value tail computed from that text was wrong.
    let big = "1263006600030000066000600000000003/200000000000000000000000000000000";
    assert_eq!(run(&format!("({big})^x")), format!("({big})^x"));
    assert_eq!(
        run(&format!("({big})^(1/3)")).split(" ≈ ").next().unwrap(),
        format!("({big})^(1/3)")
    );
    let (a, b) = (
        1263006600030000066000600000000003.0f64,
        200000000000000000000000000000000.0f64,
    );
    let tail: f64 = run(&format!("evalf(({big})^(1/3))"))
        .rsplit(" ≈ ")
        .next()
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert!(
        (tail - (a / b).powf(1.0 / 3.0)).abs() < 1e-9 * tail,
        "{tail}"
    );
    // The same through substitution, which is how the ODE checks found it.
    let value: f64 = run("subs((3/2 · x^2 + 3 · C1)^(1/3), x = 1.1000100000000001, C1 = 1.5)")
        .rsplit(" ≈ ")
        .next()
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let x = 1.1000100000000001f64;
    assert!(
        (value - (1.5 * x * x + 4.5).powf(1.0 / 3.0)).abs() < 1e-9,
        "{value}"
    );
    // Big integers and quotients of them keep their meaning too.
    assert_eq!(
        run("(2^100+1)/3^50"),
        "1267650600228229401496703205377/717897987691852588770249"
    );
    for (n, d) in [
        (
            "1267650600228229401496703205377",
            "717897987691852588770249",
        ),
        (
            "340282366920938463463374607431768211457",
            "18446744073709551629",
        ),
    ] {
        let out = run(&format!("({n}/{d})^y"));
        assert_eq!(out, format!("({n}/{d})^y"));
        let denominator_out = run(&format!("x/({n}/{d})"));
        assert!(
            denominator_out.starts_with(&format!("{d}/{n}")),
            "{denominator_out}"
        );
    }
}

#[test]
fn inference_commands_agree_with_an_independent_integration() {
    // ln Gamma by the Lanczos approximation (g = 7), written here so that the oracle shares nothing with the engine.
    fn ln_gamma(x: f64) -> f64 {
        const G: [f64; 9] = [
            0.999_999_999_999_809_9,
            676.520_368_121_885_1,
            -1_259.139_216_722_402_8,
            771.323_428_777_653_1,
            -176.615_029_162_140_6,
            12.507_343_278_686_905,
            -0.138_571_095_265_720_12,
            9.984_369_578_019_572e-6,
            1.505_632_735_149_311_6e-7,
        ];
        let x = x - 1.0;
        let t = x + 7.5;
        let mut a = G[0];
        for (i, g) in G.iter().enumerate().skip(1) {
            a += g / (x + i as f64);
        }
        0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
    }
    // Two-sided p-value of t with df degrees of freedom: 1 - 2 * integral of the density from 0 to |t| (Simpson).
    let p_value = |t: f64, df: f64| -> f64 {
        let density = |u: f64| {
            (ln_gamma((df + 1.0) / 2.0)
                - ln_gamma(df / 2.0)
                - 0.5 * (df * std::f64::consts::PI).ln()
                - (df + 1.0) / 2.0 * (1.0 + u * u / df).ln())
            .exp()
        };
        let n = 4000;
        let h = t.abs() / n as f64;
        let mut sum = density(0.0) + density(t.abs());
        for k in 1..n {
            sum += density(k as f64 * h) * if k % 2 == 1 { 4.0 } else { 2.0 };
        }
        1.0 - 2.0 * sum * h / 3.0
    };
    let value = |text: &str, key: &str| -> f64 {
        let after = text
            .split(key)
            .nth(1)
            .unwrap_or_else(|| panic!("{key} not in {text}"));
        after
            .trim_start_matches([' ', '='])
            .split([',', ' ', ')', ';'])
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    let samples: [&str; 4] = [
        "[1,2,3,4,5]",
        "[5.1,4.9,5.6,5.8,6.0,5.5]",
        "[12,15,11,14,18,13,16]",
        "[2.5,3.1,2.8,3.6,3.3,2.9,3.4,3.0]",
    ];
    for data in samples {
        for mu in ["0", "3", "5.5"] {
            let out = run(&format!("t_test({data}, {mu})"));
            let (t, df, p) = (value(&out, "t ="), value(&out, "df ="), value(&out, "p ="));
            assert!(
                (p - p_value(t, df)).abs() < 2e-6,
                "t_test({data}, {mu}): p = {p}, integration gives {}\n{out}",
                p_value(t, df)
            );
        }
        // The endpoints of the 95% interval are the hypothesised means at which p = 0.05.
        let interval = run(&format!("confidence_interval({data})"));
        let inside = interval
            .split_once("mean: (")
            .unwrap()
            .1
            .split_once(')')
            .unwrap()
            .0
            .to_string();
        for endpoint in inside.split(", ") {
            let out = run(&format!("t_test({data}, {endpoint})"));
            assert!(
                (value(&out, "p =") - 0.05).abs() < 5e-5,
                "{data} at {endpoint}: {out}"
            );
        }
    }
    // Textbook values: t = 3 / sqrt(2.5 / 5) = 4.2426 with 4 df gives p = 0.0132; the 95% interval is (1.0368, 4.9632).
    let out = run("t_test([1,2,3,4,5], 0)");
    assert!(
        out.starts_with("t = 4.242641, df = 4, p = 0.013236"),
        "{out}"
    );
    assert!(run("confidence_interval([1,2,3,4,5])").contains("(1.036757, 4.963243)"));
    // Welch's degrees of freedom lie between the smaller sample's and the pooled count, and the test is symmetric in the samples.
    let welch = run("t_test([5.1,4.9,5.6,5.8,6.0,5.5], [4.8,5.0,5.1,4.7,5.2,4.9])");
    let swapped = run("t_test([4.8,5.0,5.1,4.7,5.2,4.9], [5.1,4.9,5.6,5.8,6.0,5.5])");
    let (df, p) = (value(&welch, "df ="), value(&welch, "p ="));
    assert!((5.0..=10.0).contains(&df), "{welch}");
    assert!((p - p_value(value(&welch, "t ="), df)).abs() < 2e-6);
    assert!(
        (value(&swapped, "p =") - p).abs() < 1e-9
            && (value(&swapped, "t =") + value(&welch, "t =")).abs() < 1e-9
    );
    // Paired: the one-sample test of the differences.
    let paired = run("paired_t_test([85,80,78,90,77],[82,81,75,86,74])");
    let one_sample = run("t_test([3,-1,3,4,3], 0)");
    assert!(
        (value(&paired, "t =") - value(&one_sample, "t =")).abs() < 1e-9
            && (value(&paired, "p =") - value(&one_sample, "p =")).abs() < 1e-9
    );
    // z: a known sigma, against the normal distribution.
    let z = run("z_test([102,98,101,99,100,103], 100, 2)");
    assert!(
        (value(&z, "z =") - 0.612372).abs() < 1e-6 && (value(&z, "p =") - 0.540291).abs() < 1e-5,
        "{z}"
    );
    // Too little data is refused.
    assert!(UraeSession::new().execute_line("t_test([1], 0)").is_error);
    assert!(
        UraeSession::new()
            .execute_line("confidence_interval([1,2,3], 1.5)")
            .is_error
    );
}

#[test]
fn chi_square_proportion_anova_and_one_sided_tests_satisfy_their_identities() {
    let value = |text: &str, key: &str| -> f64 {
        let after = text
            .split(key)
            .nth(1)
            .unwrap_or_else(|| panic!("{key} not in {text}"));
        after
            .trim_start_matches([' ', '='])
            .split([',', ' ', ')', ';'])
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    // Phi by the Maclaurin series of erf, written here independently of the engine.
    let phi = |x: f64| -> f64 {
        let z = x / std::f64::consts::SQRT_2;
        let (mut term, mut sum) = (z, z);
        for k in 1..200 {
            term *= -z * z / k as f64;
            sum += term / (2 * k + 1) as f64;
        }
        0.5 * (1.0 + 2.0 / std::f64::consts::PI.sqrt() * sum)
    };
    // One-sided p-values: greater + less = 1 and the two-sided value is twice the smaller.
    for (data, mu) in [
        ("[1,2,3,4,5]", "0"),
        ("[12,15,11,14,18,13,16]", "15"),
        ("[2.5,3.1,2.8,3.6,3.3]", "3"),
    ] {
        let greater = value(&run(&format!("t_test({data}, {mu}, greater)")), "p =");
        let less = value(&run(&format!("t_test({data}, {mu}, less)")), "p =");
        let two = value(&run(&format!("t_test({data}, {mu})")), "p =");
        assert!(
            (greater + less - 1.0).abs() < 1e-5,
            "{data}: {greater} + {less}"
        );
        assert!(
            (two - 2.0 * greater.min(less)).abs() < 1e-5,
            "{data}: {two} vs {greater}, {less}"
        );
    }
    // Goodness of fit: with 2 degrees of freedom the p-value is exp(-chi2 / 2).
    for observed in ["[20,30,50]", "[18,22,60]", "[10,10,10]"] {
        let out = run(&format!("chi2_test({observed}, [25,25,50])"));
        let (statistic, p) = (value(&out, "chi-square ="), value(&out, "p ="));
        let direct: f64 = observed
            .trim_matches(['[', ']'])
            .split(',')
            .zip([25.0f64, 25.0, 50.0])
            .map(|(o, e)| (o.parse::<f64>().unwrap() - e).powi(2) / e)
            .sum();
        assert!((statistic - direct).abs() < 1e-5, "{out}");
        assert!((p - (-direct / 2.0).exp()).abs() < 1e-5, "{out}");
    }
    // A 2x2 independence table has 1 degree of freedom: p = 2 (1 - Phi(sqrt(chi2))).
    for table in ["[[10,20],[30,40]]", "[[12,5],[7,16]]", "[[30,10],[20,40]]"] {
        let out = run(&format!("chi2_test({table})"));
        let (statistic, p) = (value(&out, "chi-square ="), value(&out, "p ="));
        let cells: Vec<f64> = table
            .replace("[[", "")
            .replace("]]", "")
            .replace("],[", ",")
            .split(',')
            .map(|c| c.parse().unwrap())
            .collect();
        let (a, b, c, d) = (cells[0], cells[1], cells[2], cells[3]);
        // The closed form for a 2x2 table.
        let direct =
            (a + b + c + d) * (a * d - b * c).powi(2) / ((a + b) * (c + d) * (a + c) * (b + d));
        assert!((statistic - direct).abs() < 1e-5, "{out} vs {direct}");
        assert!((p - 2.0 * (1.0 - phi(direct.sqrt()))).abs() < 1e-5, "{out}");
    }
    // A proportion test is a z test of (x / n - p0) / sqrt(p0 (1 - p0) / n).
    for (x, n, p0) in [
        (60.0f64, 100.0f64, 0.5f64),
        (18.0, 40.0, 0.3),
        (7.0, 25.0, 0.4),
    ] {
        let z = (x / n - p0) / (p0 * (1.0 - p0) / n).sqrt();
        let out = run(&format!("proportion_test({x:?}, {n:?}, {p0:?})"));
        assert!((value(&out, "z =") - z).abs() < 1e-5, "{out}");
        assert!(
            (value(&out, "p =") - 2.0 * (1.0 - phi(z.abs()))).abs() < 1e-5,
            "{out}"
        );
        let greater = run(&format!("proportion_test({x:?}, {n:?}, {p0:?}, greater)"));
        assert!(
            (value(&greater, "p =") - (1.0 - phi(z))).abs() < 1e-5,
            "{greater}"
        );
    }
    // With two groups the one-way ANOVA F is the square of the pooled t statistic and the p-values agree.
    for (a, b) in [
        ("[1,2,3,4]", "[3,4,5,9]"),
        ("[5.1,4.9,5.6,5.8]", "[4.8,5.0,5.1,4.7,5.2]"),
    ] {
        let out = run(&format!("anova({a}, {b})"));
        let (f, p) = (value(&out, "F ="), value(&out, "p ="));
        let parse = |s: &str| -> Vec<f64> {
            s.trim_matches(['[', ']'])
                .split(',')
                .map(|x| x.parse().unwrap())
                .collect()
        };
        let (x, y) = (parse(a), parse(b));
        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        let ss = |v: &[f64]| v.iter().map(|e| (e - mean(v)).powi(2)).sum::<f64>();
        let (n1, n2) = (x.len() as f64, y.len() as f64);
        let pooled = (ss(&x) + ss(&y)) / (n1 + n2 - 2.0);
        let t = (mean(&x) - mean(&y)) / (pooled * (1.0 / n1 + 1.0 / n2)).sqrt();
        assert!(
            (f - t * t).abs() < 1e-4 * (1.0 + f),
            "F = {f}, t^2 = {}",
            t * t
        );
        assert_eq!(value(&out, "df = ("), 1.0);
        assert!((0.0..=1.0).contains(&p));
    }
    // A group-count example by hand: means 2, 3, 6 give F = 13 with (2, 6) degrees of freedom.
    assert!(run("anova([1,2,3],[2,3,4],[5,6,7])").starts_with("F = 13, df = (2, 6), p = 0.006592"));
    // Refusals.
    assert!(UraeSession::new().execute_line("chi2_test([5])").is_error);
    assert!(
        UraeSession::new()
            .execute_line("chi2_test([1,2],[0,3])")
            .is_error
    );
    assert!(
        UraeSession::new()
            .execute_line("proportion_test(11, 10, 0.5)")
            .is_error
    );
    assert!(
        UraeSession::new()
            .execute_line("anova([1,1],[1,1])")
            .is_error
    );
}

#[test]
fn fisher_correlation_and_rank_sum_tests_match_brute_force() {
    let value = |text: &str, key: &str| -> f64 {
        let after = text
            .split(key)
            .nth(1)
            .unwrap_or_else(|| panic!("{key} not in {text}"));
        after
            .trim_start_matches([' ', '='])
            .split([',', ' ', ')', ';'])
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    // The exact fraction a/b printed after "p = ... = ".
    let fraction = |text: &str| -> (u128, u128) {
        let after = text
            .split("p = ")
            .nth(1)
            .unwrap()
            .split(" = ")
            .nth(1)
            .unwrap()
            .split(' ')
            .next()
            .unwrap();
        let (a, b) = after.split_once('/').unwrap_or((after, "1"));
        (a.parse().unwrap(), b.parse().unwrap())
    };
    let choose = |n: u128, k: u128| -> u128 {
        if k > n {
            0
        } else {
            (1..=k).fold(1u128, |acc, i| acc * (n - k + i) / i)
        }
    };
    // Fisher: enumerate every table with the margins, summing the weights no larger than the observed one.
    for (a, b, c, d) in [
        (3u128, 1, 1, 3),
        (7, 2, 3, 9),
        (0, 5, 6, 1),
        (12, 4, 5, 11),
        (2, 8, 8, 2),
    ] {
        let (r1, r2, c1, n) = (a + b, c + d, a + c, a + b + c + d);
        let weight = |k: u128| choose(r1, k) * choose(r2, c1 - k);
        let observed = weight(a);
        let extreme: u128 = (c1.saturating_sub(r2)..=c1.min(r1))
            .map(weight)
            .filter(|w| *w <= observed)
            .sum();
        let out = run(&format!("fisher_exact([[{a},{b}],[{c},{d}]])"));
        let (num, den) = fraction(&out);
        assert_eq!(
            num * choose(n, c1),
            extreme * den,
            "fisher [[{a},{b}],[{c},{d}]]: {out}, expected {extreme}/{}",
            choose(n, c1)
        );
    }
    assert!(run("fisher_exact([[3,1],[1,3]])").contains("17/35"));
    // Mann-Whitney: every assignment of the pooled values, ties included, by bitmask.
    let cases: [(&[f64], &[f64]); 5] = [
        (&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0]),
        (&[1.0, 3.0, 5.0, 7.0], &[2.0, 4.0, 6.0, 8.0, 10.0]),
        (&[1.0, 2.0, 2.0, 5.0], &[2.0, 3.0, 4.0, 6.0]),
        (&[10.0, 12.0, 15.0], &[9.0, 11.0, 14.0, 16.0, 18.0, 20.0]),
        (&[1.0, 1.0, 2.0], &[1.0, 2.0, 2.0, 3.0]),
    ];
    for (x, y) in cases {
        let pooled: Vec<f64> = x.iter().chain(y).copied().collect();
        let n = pooled.len();
        let ranks: Vec<f64> = (0..n)
            .map(|i| {
                let less = pooled.iter().filter(|v| **v < pooled[i]).count() as f64;
                let equal = pooled.iter().filter(|v| **v == pooled[i]).count() as f64;
                less + (equal + 1.0) / 2.0
            })
            .collect();
        let n1 = x.len();
        let u_of = |mask: u32| -> f64 {
            let w: f64 = (0..n)
                .filter(|i| mask >> i & 1 == 1)
                .map(|i| ranks[i])
                .sum();
            w - (n1 * (n1 + 1)) as f64 / 2.0
        };
        let observed = u_of((1u32 << n1) - 1);
        let mean = (n1 * y.len()) as f64 / 2.0;
        let (mut extreme, mut total) = (0u128, 0u128);
        for mask in 0u32..(1 << n) {
            if mask.count_ones() as usize != n1 {
                continue;
            }
            total += 1;
            if (u_of(mask) - mean).abs() >= (observed - mean).abs() - 1e-9 {
                extreme += 1;
            }
        }
        let list = |v: &[f64]| {
            format!(
                "[{}]",
                v.iter()
                    .map(|e| format!("{e:?}"))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        };
        let out = run(&format!("mann_whitney({}, {})", list(x), list(y)));
        let (num, den) = fraction(&out);
        assert_eq!(
            num * total,
            extreme * den,
            "{x:?} vs {y:?}: {out}, expected {extreme}/{total}"
        );
        assert!((value(&out, "U =") - observed).abs() < 1e-9, "{out}");
    }
    // Pearson against the definition, its p against the t distribution, and Spearman as Pearson on average ranks.
    let pearson = |x: &[f64], y: &[f64]| -> f64 {
        let n = x.len() as f64;
        let (mx, my) = (x.iter().sum::<f64>() / n, y.iter().sum::<f64>() / n);
        let sxy: f64 = x.iter().zip(y).map(|(a, b)| (a - mx) * (b - my)).sum();
        sxy / (x.iter().map(|a| (a - mx).powi(2)).sum::<f64>()
            * y.iter().map(|b| (b - my).powi(2)).sum::<f64>())
        .sqrt()
    };
    let ranks_of = |v: &[f64]| -> Vec<f64> {
        v.iter()
            .map(|e| {
                v.iter().filter(|o| **o < *e).count() as f64
                    + (v.iter().filter(|o| **o == *e).count() as f64 + 1.0) / 2.0
            })
            .collect()
    };
    let pairs: [(&[f64], &[f64]); 3] = [
        (&[1.0, 2.0, 3.0, 4.0, 5.0], &[2.0, 4.0, 5.0, 4.0, 5.0]),
        (
            &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
            &[6.0, 5.0, 7.0, 3.0, 2.0, 2.0],
        ),
        (&[2.0, 4.0, 4.0, 6.0, 9.0], &[1.0, 3.0, 2.0, 2.0, 8.0]),
    ];
    for (x, y) in pairs {
        let list = |v: &[f64]| {
            format!(
                "[{}]",
                v.iter()
                    .map(|e| format!("{e:?}"))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        };
        let out = run(&format!("correlation_test({}, {})", list(x), list(y)));
        let (r, df) = (pearson(x, y), x.len() as f64 - 2.0);
        assert!(
            (value(&out, "Pearson r =") - r).abs() < 1e-5,
            "{out} vs {r}"
        );
        let t = r * (df / (1.0 - r * r)).sqrt();
        assert!((value(&out, "t =") - t).abs() < 1e-4, "{out}");
        let spearman = run(&format!("spearman({}, {})", list(x), list(y)));
        assert!(
            (value(&spearman, "Spearman rho =") - pearson(&ranks_of(x), &ranks_of(y))).abs() < 1e-5,
            "{spearman}"
        );
    }
    // Perfect correlation has p = 0, a constant variable is refused.
    assert!(run("pearson([1,2,3],[2,4,6])").contains("p = 0"));
    assert!(
        UraeSession::new()
            .execute_line("pearson([1,1,1],[1,2,3])")
            .is_error
    );
    assert!(
        UraeSession::new()
            .execute_line("fisher_exact([[1,2],[3]])")
            .is_error
    );
}

#[test]
fn regression_inference_wilcoxon_and_kruskal_wallis_satisfy_their_identities() {
    let value = |text: &str, key: &str| -> f64 {
        let after = text
            .split(key)
            .nth(1)
            .unwrap_or_else(|| panic!("{key} not in {text}"));
        after
            .trim_start_matches([' ', '='])
            .split([',', ' ', ')', ';'])
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    let list = |v: &[f64]| {
        format!(
            "[{}]",
            v.iter()
                .map(|e| format!("{e:?}"))
                .collect::<Vec<_>>()
                .join(",")
        )
    };
    let ranks_of = |v: &[f64]| -> Vec<f64> {
        v.iter()
            .map(|e| {
                v.iter().filter(|o| **o < *e).count() as f64
                    + (v.iter().filter(|o| **o == *e).count() as f64 + 1.0) / 2.0
            })
            .collect()
    };
    // Regression: the slope t statistic and p-value are those of the test of zero correlation; R^2 = r^2; the intercept and
    // slope are the least-squares ones.
    let datasets: [(&[f64], &[f64]); 3] = [
        (&[1.0, 2.0, 3.0, 4.0, 5.0], &[2.0, 4.0, 5.0, 4.0, 5.0]),
        (
            &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
            &[6.0, 5.0, 7.0, 3.0, 2.0, 2.0],
        ),
        (
            &[0.5, 1.5, 2.5, 3.0, 4.5, 6.0, 7.5],
            &[1.0, 2.2, 2.9, 4.1, 5.2, 7.4, 8.1],
        ),
    ];
    for (x, y) in datasets {
        let out = run(&format!("regress({}, {})", list(x), list(y)));
        let correlation = run(&format!("correlation_test({}, {})", list(x), list(y)));
        let slope_part = out.split(';').next().unwrap();
        let (t, p) = (value(slope_part, "t ="), value(slope_part, "p ="));
        assert!(
            (t - value(&correlation, "t =")).abs() < 1e-4,
            "{out} vs {correlation}"
        );
        assert!(
            (p - value(&correlation, "p =")).abs() < 1e-5,
            "{out} vs {correlation}"
        );
        let r = value(&correlation, "Pearson r =");
        let r2_text = out
            .split("R^2 = ")
            .nth(1)
            .unwrap()
            .split(',')
            .next()
            .unwrap();
        let r2 = match r2_text.split_once('/') {
            Some((a, b)) => a.parse::<f64>().unwrap() / b.parse::<f64>().unwrap(),
            None => r2_text.parse().unwrap(),
        };
        assert!((r2 - r * r).abs() < 1e-5, "{out}");
        let n = x.len() as f64;
        let (mx, my) = (x.iter().sum::<f64>() / n, y.iter().sum::<f64>() / n);
        let sxx: f64 = x.iter().map(|a| (a - mx).powi(2)).sum();
        let sxy: f64 = x.iter().zip(y).map(|(a, b)| (a - mx) * (b - my)).sum();
        let slope = sxy / sxx;
        let shown_slope = slope_part
            .split(" = ")
            .nth(1)
            .unwrap()
            .split(' ')
            .next()
            .unwrap();
        let parsed = match shown_slope.split_once('/') {
            Some((a, b)) => a.parse::<f64>().unwrap() / b.parse::<f64>().unwrap(),
            None => shown_slope.parse().unwrap(),
        };
        assert!((parsed - slope).abs() < 1e-12, "{out}");
        // The 95% interval is slope +- t* se with t* the 0.975 quantile of the t distribution.
        let ci: Vec<f64> = slope_part
            .split("95% CI (")
            .nth(1)
            .unwrap()
            .split(')')
            .next()
            .unwrap()
            .split(", ")
            .map(|v| v.parse().unwrap())
            .collect();
        let se = value(slope_part, "se ");
        let critical: f64 = run(&format!("t_quantile(0.975, {})", n - 2.0))
            .trim()
            .parse()
            .unwrap();
        assert!(
            ((ci[1] - ci[0]) / 2.0 / se - critical).abs() < 1e-3,
            "{out}: t* = {critical}"
        );
        assert!((((ci[0] + ci[1]) / 2.0) - slope).abs() < 1e-5, "{out}");
    }
    assert!(run("regress([1,2,3],[2,4,6])").starts_with("slope = 2 (exact fit)"));
    // Wilcoxon signed-rank: every sign assignment of the average ranks, by bitmask.
    let samples: [&[f64]; 4] = [
        &[1.5, -0.5, 2.0, 3.0, -1.0, 2.5],
        &[0.5, -1.5, 2.0, 2.0, -3.0, 4.0, 1.0],
        &[3.0, 3.0, -3.0, 1.0, 2.0, 0.0, 5.0, -1.0],
        &[0.2, 0.4, 0.6, 0.8, 1.0],
    ];
    for d in samples {
        let nonzero: Vec<f64> = d.iter().copied().filter(|v| *v != 0.0).collect();
        let n = nonzero.len();
        let ranks = ranks_of(&nonzero.iter().map(|v| v.abs()).collect::<Vec<_>>());
        let total: f64 = ranks.iter().sum();
        let w_plus = |mask: u32| -> f64 {
            (0..n)
                .filter(|i| mask >> i & 1 == 1)
                .map(|i| ranks[i])
                .sum()
        };
        let observed = w_plus(
            (0..n)
                .filter(|i| nonzero[*i] > 0.0)
                .fold(0u32, |m, i| m | 1 << i),
        );
        let extreme = (0u32..(1 << n))
            .filter(|m| (2.0 * w_plus(*m) - total).abs() >= (2.0 * observed - total).abs() - 1e-9)
            .count() as u128;
        let out = run(&format!("wilcoxon({})", list(d)));
        let after = out
            .split("p = ")
            .nth(1)
            .unwrap()
            .split(" = ")
            .nth(1)
            .unwrap()
            .split(' ')
            .next()
            .unwrap();
        let (num, den) = match after.split_once('/') {
            Some((a, b)) => (a.parse::<u128>().unwrap(), b.parse::<u128>().unwrap()),
            None => (after.parse::<u128>().unwrap(), 1u128),
        };
        assert_eq!(
            num * (1u128 << n),
            extreme * den,
            "wilcoxon({d:?}): {out}, expected {extreme}/{}",
            1u128 << n
        );
        let shown = out
            .split("W+ = ")
            .nth(1)
            .unwrap()
            .split(',')
            .next()
            .unwrap();
        let w = match shown.split_once('/') {
            Some((a, b)) => a.parse::<f64>().unwrap() / b.parse::<f64>().unwrap(),
            None => shown.parse().unwrap(),
        };
        assert!((w - observed).abs() < 1e-9, "{out}");
    }
    // Paired form equals the one-sample form on the differences.
    assert_eq!(
        run("wilcoxon([5,3,8,2],[4,5,6,3])"),
        run("wilcoxon([1,-2,2,-1])")
    );
    // Kruskal-Wallis: H from the rank sums; with 3 groups (2 df) p = exp(-H / 2); the textbook data give H = 0.77143.
    let groups: [&[f64]; 3] = [
        &[2.9, 3.0, 2.5, 2.6, 3.2],
        &[3.8, 2.7, 4.0, 2.4],
        &[2.8, 3.4, 3.7, 2.2, 2.0],
    ];
    let pooled: Vec<f64> = groups.iter().flat_map(|g| g.iter().copied()).collect();
    let ranks = ranks_of(&pooled);
    let n = pooled.len() as f64;
    let (mut offset, mut sum) = (0, 0.0);
    for g in groups {
        let rank_sum: f64 = ranks[offset..offset + g.len()].iter().sum();
        sum += rank_sum * rank_sum / g.len() as f64;
        offset += g.len();
    }
    let h = 12.0 / (n * (n + 1.0)) * sum - 3.0 * (n + 1.0);
    let out = run(&format!(
        "kruskal({}, {}, {})",
        list(groups[0]),
        list(groups[1]),
        list(groups[2])
    ));
    assert!(
        (value(&out, "H =") - h).abs() < 1e-5 && (value(&out, "H =") - 0.771429).abs() < 1e-5,
        "{out}"
    );
    assert!(
        (value(&out, "p =") - (-h / 2.0).exp()).abs() < 1e-5,
        "{out}"
    );
    // Two groups without ties: H is the square of the normal-approximation z of the rank-sum test (no continuity correction).
    let (a, b) = ([1.2, 3.4, 2.2, 5.1], [2.8, 4.4, 6.0, 7.1, 3.9]);
    let h2 = value(&run(&format!("kruskal({}, {})", list(&a), list(&b))), "H =");
    let pooled2: Vec<f64> = a.iter().chain(&b).copied().collect();
    let ranks2 = ranks_of(&pooled2);
    let (n1, n2) = (a.len() as f64, b.len() as f64);
    let u = ranks2[..a.len()].iter().sum::<f64>() - n1 * (n1 + 1.0) / 2.0;
    let z = (u - n1 * n2 / 2.0) / (n1 * n2 * (n1 + n2 + 1.0) / 12.0).sqrt();
    assert!((h2 - z * z).abs() < 1e-4, "H = {h2}, z^2 = {}", z * z);
    // Refusals.
    assert!(
        UraeSession::new()
            .execute_line("wilcoxon([0,0,0])")
            .is_error
    );
    assert!(
        UraeSession::new()
            .execute_line("regress([1,1,1],[1,2,3])")
            .is_error
    );
    assert!(
        UraeSession::new()
            .execute_line("kruskal([1,1],[1,1])")
            .is_error
    );
}

#[test]
fn matrix_commands_satisfy_their_defining_properties() {
    // Exact rationals as (numerator, denominator) with i128 arithmetic.
    type F = (i128, i128);
    fn gcd(a: i128, b: i128) -> i128 {
        if b == 0 { a.abs() } else { gcd(b, a % b) }
    }
    fn norm((n, d): F) -> F {
        let g = gcd(n, d).max(1);
        let s = if d < 0 { -1 } else { 1 };
        (s * n / g, s * d / g)
    }
    fn add(a: F, b: F) -> F {
        norm((a.0 * b.1 + b.0 * a.1, a.1 * b.1))
    }
    fn mul(a: F, b: F) -> F {
        norm((a.0 * b.0, a.1 * b.1))
    }
    let parse_cell = |text: &str| -> F {
        match text.trim().split_once('/') {
            Some((a, b)) => norm((a.trim().parse().unwrap(), b.trim().parse().unwrap())),
            None => (text.trim().parse().unwrap(), 1),
        }
    };
    let parse_matrix = |text: &str| -> Vec<Vec<F>> {
        text.trim()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .split(';')
            .map(|r| r.split(',').map(parse_cell).collect())
            .collect()
    };
    let product = |a: &Vec<Vec<F>>, b: &Vec<Vec<F>>| -> Vec<Vec<F>> {
        (0..a.len())
            .map(|i| {
                (0..b[0].len())
                    .map(|j| (0..b.len()).fold((0, 1), |s, k| add(s, mul(a[i][k], b[k][j]))))
                    .collect()
            })
            .collect()
    };
    let transpose = |a: &Vec<Vec<F>>| -> Vec<Vec<F>> {
        (0..a[0].len())
            .map(|j| a.iter().map(|r| r[j]).collect())
            .collect()
    };
    let literal = |rows: &[Vec<i128>]| {
        format!(
            "[{}]",
            rows.iter()
                .map(|r| format!(
                    "[{}]",
                    r.iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                ))
                .collect::<Vec<_>>()
                .join(",")
        )
    };
    let mut state = 0xABCD_1234_5678_9EF1u64;
    let mut next = move |m: u64| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % m) as i128 - (m as i128) / 2
    };
    // The four Penrose conditions for random rectangular matrices (including rank deficient ones).
    for (rows, cols) in [(3usize, 2usize), (2, 3), (3, 3), (4, 2), (2, 2)] {
        for _ in 0..4 {
            let mut data: Vec<Vec<i128>> = (0..rows)
                .map(|_| (0..cols).map(|_| next(7)).collect())
                .collect();
            if next(2) == 0 && rows > 1 {
                data[rows - 1] = data[0].iter().map(|x| 2 * x).collect();
            }
            let a: Vec<Vec<F>> = data
                .iter()
                .map(|r| r.iter().map(|x| (*x, 1)).collect())
                .collect();
            let p = parse_matrix(&run(&format!("pinv({})", literal(&data))));
            let apa = product(&product(&a, &p), &a);
            let pap = product(&product(&p, &a), &p);
            assert_eq!(apa, a, "A A+ A for {data:?}");
            assert_eq!(pap, p, "A+ A A+ for {data:?}");
            let (ap, pa) = (product(&a, &p), product(&p, &a));
            assert_eq!(ap, transpose(&ap), "A A+ symmetric for {data:?}");
            assert_eq!(pa, transpose(&pa), "A+ A symmetric for {data:?}");
        }
    }
    // The adjugate: A adj(A) = det(A) I, and the cofactor matrix is its transpose.
    for _ in 0..6 {
        let data: Vec<Vec<i128>> = (0..3).map(|_| (0..3).map(|_| next(9)).collect()).collect();
        let det = data[0][0] * (data[1][1] * data[2][2] - data[1][2] * data[2][1])
            - data[0][1] * (data[1][0] * data[2][2] - data[1][2] * data[2][0])
            + data[0][2] * (data[1][0] * data[2][1] - data[1][1] * data[2][0]);
        let a: Vec<Vec<F>> = data
            .iter()
            .map(|r| r.iter().map(|x| (*x, 1)).collect())
            .collect();
        let adjugate = parse_matrix(&run(&format!("adjugate({})", literal(&data))));
        let identity_times_det: Vec<Vec<F>> = (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| if i == j { (det, 1) } else { (0, 1) })
                    .collect()
            })
            .collect();
        assert_eq!(
            product(&a, &adjugate),
            identity_times_det,
            "A adj(A) for {data:?}"
        );
        assert_eq!(
            parse_matrix(&run(&format!("cofactor({})", literal(&data)))),
            transpose(&adjugate)
        );
        // Expansion along the first row by minors: det = sum a_1j (-1)^j minor_1j.
        let by_minors: i128 = (0..3)
            .map(|j| {
                let m: F = parse_cell(&run(&format!("minor({}, 1, {})", literal(&data), j + 1)));
                data[0][j] * m.0 * if j % 2 == 0 { 1 } else { -1 }
            })
            .sum();
        assert_eq!(by_minors, det, "{data:?}");
    }
    // Gram-Schmidt: pairwise orthogonal, as many vectors as the rank; projection leaves a residual orthogonal to v.
    let gs = run("gram_schmidt([[1,2,0],[1,0,1],[1,1,1]])");
    let vectors: Vec<Vec<F>> = gs
        .trim_start_matches("orthogonal basis: ")
        .split("], [")
        .map(|v| {
            v.trim_matches(['[', ']'])
                .split(", ")
                .map(parse_cell)
                .collect()
        })
        .collect();
    assert_eq!(vectors.len(), 3, "{gs}");
    for i in 0..vectors.len() {
        for j in i + 1..vectors.len() {
            assert_eq!(
                (0..3).fold((0, 1), |s, k| add(s, mul(vectors[i][k], vectors[j][k]))),
                (0, 1),
                "{gs}"
            );
        }
    }
    assert_eq!(
        run("gram_schmidt([[1,2],[2,4],[0,1]])")
            .matches('[')
            .count(),
        2,
        "three vectors of rank two"
    );
    for (u, v) in [
        ([1i128, 2], [3i128, 4]),
        ([5, -1], [2, 2]),
        ([0, 3], [4, 0]),
    ] {
        let p = run(&format!(
            "projection([{},{}],[{},{}])",
            u[0], u[1], v[0], v[1]
        ));
        let cells: Vec<F> = p
            .trim_matches(['[', ']'])
            .split(", ")
            .map(parse_cell)
            .collect();
        let residual: Vec<F> = (0..2)
            .map(|i| add((u[i], 1), (-cells[i].0, cells[i].1)))
            .collect();
        assert_eq!(
            (0..2).fold((0, 1), |s, k| add(s, mul(residual[k], (v[k], 1)))),
            (0, 1),
            "projection of {u:?} on {v:?}: {p}"
        );
    }
    // Least squares: the residual is orthogonal to the columns of A (the normal equations hold).
    let x = parse_matrix(&run("lstsq([[1,1],[1,2],[1,3],[1,4]],[1,3,2,5])"));
    let a: Vec<Vec<F>> = [[1, 1], [1, 2], [1, 3], [1, 4]]
        .iter()
        .map(|r| r.iter().map(|v| (*v, 1)).collect())
        .collect();
    let b: Vec<Vec<F>> = [1, 3, 2, 5].iter().map(|v| vec![(*v, 1)]).collect();
    let residual: Vec<Vec<F>> = product(&a, &x)
        .iter()
        .zip(&b)
        .map(|(p, q)| vec![add(p[0], (-q[0].0, q[0].1))])
        .collect();
    assert!(
        product(&transpose(&a), &residual)
            .iter()
            .all(|r| r[0] == (0, 1)),
        "A^T r = 0"
    );
    // Predicates.
    assert_eq!(run("is_symmetric([[1,2],[2,1]])"), "true");
    assert_eq!(run("is_symmetric([[1,2],[3,1]])"), "false");
    assert_eq!(run("is_positive_definite([[2,1],[1,2]])"), "true");
    assert_eq!(run("is_positive_definite([[1,2],[2,1]])"), "false");
    assert_eq!(run("is_orthogonal([[0,1],[1,0]])"), "true");
    assert_eq!(run("is_orthogonal([[1,1],[0,1]])"), "false");
    // cond of a 2x2 matrix: sigma^2 are the roots of l^2 - tr(A^T A) l + det(A)^2.
    for (a, b, c, d) in [
        (1.0f64, 2.0f64, 3.0f64, 4.0f64),
        (2.0, 0.0, 0.0, 5.0),
        (3.0, 1.0, 1.0, 2.0),
    ] {
        let trace = a * a + b * b + c * c + d * d;
        let det = a * d - b * c;
        let disc = (trace * trace - 4.0 * det * det).sqrt();
        let expected = ((trace + disc) / (trace - disc)).sqrt();
        let got: f64 = run(&format!("cond([[{a:?},{b:?}],[{c:?},{d:?}]])"))
            .trim()
            .parse()
            .unwrap();
        assert!(
            (got - expected).abs() < 1e-8 * expected,
            "cond: {got} vs {expected}"
        );
    }
    // The matrix exponential of an upper triangular 2x2: [[e^a, b (e^d - e^a) / (d - a)], [0, e^d]].
    let value = |text: &str| -> f64 { run(&format!("evalf({text})")).trim().parse().unwrap() };
    let out = run("expm([[1,2],[0,3]])");
    let cells: Vec<String> = out
        .trim_matches(['[', ']', ' '])
        .split([',', ';'])
        .map(|c| c.trim().to_string())
        .collect();
    assert!(
        (value(&cells[0]) - 1f64.exp()).abs() < 1e-9
            && (value(&cells[3]) - 3f64.exp()).abs() < 1e-9,
        "{out}"
    );
    assert!(
        (value(&cells[1]) - 2.0 * (3f64.exp() - 1f64.exp()) / 2.0).abs() < 1e-9,
        "{out}"
    );
    assert_eq!(run("expm([[0,1],[0,0]])"), "[ 1, 1 ; 0, 1 ]");
    assert_eq!(
        run("expm([[0,1],[-1,0]])"),
        "[ cos(1), sin(1) ; -sin(1), cos(1) ]"
    );
    // Stored matrices work too.
    let mut session = UraeSession::new();
    session.execute_line("M = [[1,2],[3,4]]");
    assert_eq!(
        session.execute_line("adjugate(M)").output_text,
        "[ 4, -2 ; -3, 1 ]"
    );
    assert_eq!(
        session.execute_line("pinv(M)").output_text,
        "[ -2, 1 ; 3/2, -1/2 ]"
    );
    // Refusals.
    assert!(
        UraeSession::new()
            .execute_line("adjugate([[1,2,3],[4,5,6]])")
            .is_error
    );
    assert!(
        UraeSession::new()
            .execute_line("hadamard([[1,2]],[[1],[2]])")
            .is_error
    );
}

#[test]
fn kendall_tau_matches_pair_counting_and_the_inversion_distribution() {
    let field = |text: &str, key: &str| -> f64 {
        text.split(key)
            .nth(1)
            .unwrap()
            .trim_start_matches([' ', '='])
            .split([',', ' '])
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    let list = |v: &[i32]| {
        format!(
            "[{}]",
            v.iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join(",")
        )
    };
    // Without ties tau = (C - D) / C(n,2) and the exact two-sided p is the Mahonian (inversion count) tail.
    let mut state = 0x1357_9BDF_2468_ACE1u64;
    for n in [4usize, 5, 6, 7] {
        for _ in 0..4 {
            let x: Vec<i32> = (1..=n as i32).collect();
            let mut y = x.clone();
            for i in (1..n).rev() {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                y.swap(i, (state % (i as u64 + 1)) as usize);
            }
            let mut s = 0i64;
            for i in 0..n {
                for j in i + 1..n {
                    s += if y[i] < y[j] { 1 } else { -1 };
                }
            }
            let out = run(&format!("kendall({}, {})", list(&x), list(&y)));
            let pairs = (n * (n - 1) / 2) as f64;
            assert!(
                (field(&out, "tau-b =") - s as f64 / pairs).abs() < 1e-4,
                "{out}"
            );
            // Number of permutations with k inversions: DP over insertion positions.
            let mut counts = vec![1u64];
            for m in 2..=n {
                let mut next = vec![0u64; counts.len() + m - 1];
                for (k, c) in counts.iter().enumerate() {
                    for add in 0..m {
                        next[k + add] += c;
                    }
                }
                counts = next;
            }
            let total: u64 = counts.iter().sum();
            let extreme: u64 = counts
                .iter()
                .enumerate()
                .filter(|(k, _)| (pairs as i64 - 2 * *k as i64).abs() >= s.abs())
                .map(|(_, c)| c)
                .sum();
            assert!(
                (field(&out, "p =") - extreme as f64 / total as f64).abs() < 1e-4,
                "{out}: expected {extreme}/{total}"
            );
        }
    }
    assert!(run("kendall([1,2,3,4,5],[1,2,3,4,5])").contains("tau-b = 1"));
    assert!(run("kendall([1,2,3,4,5],[5,4,3,2,1])").contains("tau-b = -1"));
    // Ties: tau-b = (C - D) / sqrt((n0 - n1)(n0 - n2)); this data has C=4, D=0, one tie in each.
    let out = run("kendall([1,2,2,3],[1,2,3,3])");
    assert!(
        (field(&out, "tau-b =") - 4.0 / (5.0f64 * 5.0).sqrt()).abs() < 1e-4,
        "{out}"
    );
    // Large samples use the normal approximation: perfectly alternating pairs give a small positive tau.
    let x: Vec<i32> = (1..=12).collect();
    let y: Vec<i32> = vec![2, 1, 4, 3, 6, 5, 8, 7, 10, 9, 12, 11];
    let out = run(&format!("kendall({}, {})", list(&x), list(&y)));
    assert!((field(&out, "tau-b =") - 54.0 / 66.0).abs() < 1e-4, "{out}");
    let z = 54.0 / (12.0f64 * 11.0 * 29.0 / 18.0).sqrt();
    assert!((field(&out, "z =") - z).abs() < 1e-3, "{out}");
    assert!(
        UraeSession::new()
            .execute_line("kendall([1,1,1],[1,2,3])")
            .is_error
    );
}

#[test]
fn multiple_regression_reduces_to_simple_regression_and_satisfies_the_normal_equations() {
    let field = |text: &str, key: &str| -> f64 {
        let after = text
            .split(key)
            .nth(1)
            .unwrap_or_else(|| panic!("{key} in {text}"));
        let token = after
            .trim_start_matches([' ', '='])
            .split([',', ' ', ';', ')'])
            .next()
            .unwrap();
        match token.split_once('/') {
            Some((a, b)) => a.parse::<f64>().unwrap() / b.parse::<f64>().unwrap(),
            None => token.parse().unwrap(),
        }
    };
    // One predictor: identical to regress(x, y); F = t^2 for the slope.
    let simple = run("regress([1,2,3,4,5,6],[6,5,7,3,2,2])");
    let multi = run("mregress([[1],[2],[3],[4],[5],[6]],[6,5,7,3,2,2])");
    let slope_part = simple.split(';').next().unwrap();
    let multi_slope = multi.split("x1").nth(1).unwrap();
    assert!(
        (field(slope_part, "slope =") - field(multi_slope, "=")).abs() < 1e-9,
        "{simple} / {multi}"
    );
    assert!(
        (field(slope_part, "t =") - field(multi_slope, "t =")).abs() < 1e-4,
        "{simple} / {multi}"
    );
    assert!(
        (field(slope_part, "p =") - field(multi_slope, "p =")).abs() < 1e-4,
        "{simple} / {multi}"
    );
    assert!(
        (field(&multi, "F =") - field(multi_slope, "t =").powi(2)).abs() < 1e-3,
        "{multi}"
    );
    assert!(
        (field(&multi, "R^2 =") - field(&simple, "R^2 =")).abs() < 1e-5,
        "{multi}"
    );
    // Two predictors with noise: the residual is orthogonal to the constant and to both columns.
    let x: [[i64; 2]; 7] = [[1, 2], [2, 1], [3, 5], [4, 3], [5, 8], [6, 4], [7, 9]];
    let y = [3i64, 4, 8, 7, 14, 9, 16];
    let xs = format!(
        "[{}]",
        x.iter()
            .map(|r| format!("[{},{}]", r[0], r[1]))
            .collect::<Vec<_>>()
            .join(",")
    );
    let ys = format!(
        "[{}]",
        y.iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    let out = run(&format!("mregress({xs}, {ys})"));
    let (b0, b1, b2) = (
        field(&out, "intercept ="),
        field(out.split("x1").nth(1).unwrap(), "="),
        field(out.split("x2").nth(1).unwrap(), "="),
    );
    let residuals: Vec<f64> = (0..7)
        .map(|i| y[i] as f64 - b0 - b1 * x[i][0] as f64 - b2 * x[i][1] as f64)
        .collect();
    assert!(residuals.iter().sum::<f64>().abs() < 1e-9, "{out}");
    assert!(
        (0..7)
            .map(|i| residuals[i] * x[i][0] as f64)
            .sum::<f64>()
            .abs()
            < 1e-9,
        "{out}"
    );
    assert!(
        (0..7)
            .map(|i| residuals[i] * x[i][1] as f64)
            .sum::<f64>()
            .abs()
            < 1e-9,
        "{out}"
    );
    let mean = y.iter().sum::<i64>() as f64 / 7.0;
    let sst: f64 = y.iter().map(|v| (*v as f64 - mean).powi(2)).sum();
    let sse: f64 = residuals.iter().map(|e| e * e).sum();
    assert!(
        (field(&out, "R^2 =") - (1.0 - sse / sst)).abs() < 1e-5,
        "{out}"
    );
    assert!(
        (field(&out, "adjusted R^2 =") - (1.0 - (sse / 4.0) / (sst / 6.0))).abs() < 1e-5,
        "{out}"
    );
    assert!(
        (field(&out, "F =") - (sst - sse) / 2.0 / (sse / 4.0)).abs() < 1e-3,
        "{out}"
    );
    // An exactly linear response gives exact coefficients.
    assert!(
        run("mregress([[1,0],[0,1],[1,1],[2,1],[3,5]],[3,4,6,8,22])")
            .starts_with("intercept = 1 (exact fit); x1 = 2 (exact fit); x2 = 3 (exact fit)")
    );
    // Refusals: collinear predictors, too few observations, mismatched lengths.
    assert!(
        UraeSession::new()
            .execute_line("mregress([[1,2],[2,4],[3,6],[4,8]],[1,2,3,5])")
            .is_error
    );
    assert!(
        UraeSession::new()
            .execute_line("mregress([[1,2],[2,3]],[1,2])")
            .is_error
    );
    assert!(
        UraeSession::new()
            .execute_line("mregress([[1],[2],[3],[4]],[1,2,3])")
            .is_error
    );
}

#[test]
fn clairaut_equations_give_the_family_of_lines_and_a_valid_singular_envelope() {
    let number = |expr: &str, name: &str, value: f64| -> f64 {
        run(&format!(
            "evalf({})",
            expr.replace(name, &format!("({value:?})"))
        ))
        .trim()
        .parse()
        .unwrap_or_else(|_| panic!("{expr}"))
    };
    type Residual = fn(f64, f64, f64) -> f64;
    let cases: [(&str, Residual, f64); 6] = [
        ("y = x*y' + (y')^2", |x, y, p| y - x * p - p * p, 0.7),
        ("y = x*y' + 1/y'", |x, y, p| y - x * p - 1.0 / p, 0.8),
        ("y = x*y' + sin(y')", |x, y, p| y - x * p - p.sin(), 0.6),
        (
            "2*y = 2*x*y' + (y')^3",
            |x, y, p| 2.0 * y - 2.0 * x * p - p.powi(3),
            0.9,
        ),
        ("y = x*y' - ln(y')", |x, y, p| y - x * p + p.ln(), 1.3),
        ("y = x*y' + exp(y')", |x, y, p| y - x * p - p.exp(), 0.4),
    ];
    for (equation, residual, parameter) in cases {
        let out = run(&format!("dsolve({equation})"));
        let general = out
            .split("y = C1*x + (")
            .nth(1)
            .unwrap()
            .split(")  (")
            .next()
            .unwrap()
            .to_string();
        let singular_x = out
            .split("singular solution x = ")
            .nth(1)
            .unwrap()
            .split(", y = ")
            .next()
            .unwrap()
            .to_string();
        let singular_y = out
            .split(", y = ")
            .last()
            .unwrap()
            .split(", parameter")
            .next()
            .unwrap()
            .to_string();
        // Each line y = C x + f(C) solves the equation (y' = C).
        for (c, x) in [
            (parameter, 0.5),
            (parameter + 0.3, -1.1),
            (parameter * 1.5, 2.0),
        ] {
            let y = c * x + number(&general, "C1", c);
            assert!(
                residual(x, y, c).abs() < 1e-9,
                "{equation}: line with C = {c}: {out}"
            );
        }
        // The envelope is a solution: on it dy/dx = p, i.e. dy/dp = p dx/dp, and it satisfies the equation.
        let h = 1e-5;
        let dx = (number(&singular_x, "dydx", parameter + h)
            - number(&singular_x, "dydx", parameter - h))
            / (2.0 * h);
        let dy = (number(&singular_y, "dydx", parameter + h)
            - number(&singular_y, "dydx", parameter - h))
            / (2.0 * h);
        assert!(
            (dy - parameter * dx).abs() < 1e-6,
            "{equation}: dy = {dy}, p dx = {}: {out}",
            parameter * dx
        );
        let (x, y) = (
            number(&singular_x, "dydx", parameter),
            number(&singular_y, "dydx", parameter),
        );
        assert!(
            residual(x, y, parameter).abs() < 1e-9,
            "{equation}: envelope point: {out}"
        );
    }
    // Not Clairaut: the coefficient of x differs from y'.
    assert!(!run("dsolve(y = 2*x*y' + (y')^2)").contains("Clairaut"));
}

#[test]
fn weighted_and_directed_graph_commands_match_brute_force() {
    let mut state = 0x2468_ACE1_1357_9BDFu64;
    let mut next = move |m: u64| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state % m
    };
    let edge_list = |edges: &[(usize, usize, i64)]| {
        format!(
            "[{}]",
            edges
                .iter()
                .map(|(a, b, w)| format!("[v{a},v{b},{w}]"))
                .collect::<Vec<_>>()
                .join(",")
        )
    };
    let first_number = |text: &str, key: &str| -> i64 {
        text.split(key)
            .nth(1)
            .unwrap()
            .trim()
            .split([':', ',', ' '])
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    for _ in 0..12 {
        let n = 5 + next(2) as usize;
        let m = n + 2 + next(4) as usize;
        // Undirected positive weights: shortest paths against Floyd-Warshall.
        let mut edges: Vec<(usize, usize, i64)> = (0..m)
            .map(|_| {
                (
                    next(n as u64) as usize,
                    next(n as u64) as usize,
                    1 + next(9) as i64,
                )
            })
            .filter(|(a, b, _)| a != b)
            .collect();
        // Keep the graph connected through a path of heavy edges.
        for i in 0..n - 1 {
            edges.push((i, i + 1, 10 + next(5) as i64));
        }
        let inf = i64::MAX / 4;
        let mut d = vec![vec![inf; n]; n];
        for i in 0..n {
            d[i][i] = 0;
        }
        for &(a, b, w) in &edges {
            d[a][b] = d[a][b].min(w);
            d[b][a] = d[b][a].min(w);
        }
        for k in 0..n {
            for i in 0..n {
                for j in 0..n {
                    d[i][j] = d[i][j].min(d[i][k] + d[k][j]);
                }
            }
        }
        let text = edge_list(&edges);
        for (a, b) in [(0, n - 1), (1, n - 2), (2, 0)] {
            let out = run(&format!("dijkstra({text}, v{a}, v{b})"));
            assert_eq!(
                first_number(&out, "length"),
                d[a][b],
                "{text} {a}->{b}: {out}"
            );
        }
        // Minimum spanning tree against every (n-1)-subset of edges that connects the graph.
        let mut best = i64::MAX;
        for mask in 0u32..(1 << edges.len()) {
            if mask.count_ones() as usize != n - 1 {
                continue;
            }
            let mut root: Vec<usize> = (0..n).collect();
            fn find(root: &mut Vec<usize>, x: usize) -> usize {
                if root[x] == x {
                    x
                } else {
                    let r = find(root, root[x]);
                    root[x] = r;
                    r
                }
            }
            let mut ok = true;
            let mut weight = 0;
            for (i, &(a, b, w)) in edges.iter().enumerate() {
                if mask >> i & 1 == 1 {
                    let (ra, rb) = (find(&mut root, a), find(&mut root, b));
                    if ra == rb {
                        ok = false;
                        break;
                    }
                    root[ra] = rb;
                    weight += w;
                }
            }
            if ok {
                best = best.min(weight);
            }
        }
        assert_eq!(
            first_number(&run(&format!("minimum_spanning_tree({text})")), "weight"),
            best,
            "{text}"
        );
        // Directed forward edges (so there are no cycles) with negative weights allowed: Bellman-Ford against Floyd-Warshall.
        let dag: Vec<(usize, usize, i64)> = (0..m)
            .map(|_| {
                let a = next(n as u64 - 1) as usize;
                let b = a + 1 + next((n - a - 1) as u64) as usize;
                (a, b, next(11) as i64 - 4)
            })
            .collect();
        let mut f = vec![vec![inf; n]; n];
        for i in 0..n {
            f[i][i] = 0;
        }
        for &(a, b, w) in &dag {
            f[a][b] = f[a][b].min(w);
        }
        for k in 0..n {
            for i in 0..n {
                for j in 0..n {
                    if f[i][k] < inf && f[k][j] < inf {
                        f[i][j] = f[i][j].min(f[i][k] + f[k][j]);
                    }
                }
            }
        }
        let dag_text = edge_list(&dag);
        for b in 1..n {
            if !dag.iter().any(|(x, y, _)| *x == 0 || *y == 0)
                || !dag.iter().any(|(x, y, _)| *x == b || *y == b)
            {
                continue;
            }
            let out = run(&format!("dijkstra({dag_text}, v0, v{b}, directed)"));
            if out.contains("not a vertex") {
                continue;
            }
            if f[0][b] >= inf {
                assert!(
                    out == "no path" || out.contains("not a vertex"),
                    "{dag_text}: {out}"
                );
            } else {
                assert_eq!(
                    first_number(&out, "length"),
                    f[0][b],
                    "{dag_text} v0->v{b}: {out}"
                );
            }
        }
        // Topological order respects every edge.
        let order = run(&format!("topological_sort({dag_text})"));
        let position: Vec<&str> = order.split(", ").collect();
        for &(a, b, _) in &dag {
            let (pa, pb) = (
                position.iter().position(|v| *v == format!("v{a}")),
                position.iter().position(|v| *v == format!("v{b}")),
            );
            assert!(pa < pb, "{dag_text}: {order}");
        }
        // Max flow equals the minimum capacity over every cut; the reported cut has that capacity.
        let capacity: Vec<(usize, usize, i64)> = (0..m + 3)
            .map(|_| {
                (
                    next(n as u64) as usize,
                    next(n as u64) as usize,
                    1 + next(8) as i64,
                )
            })
            .filter(|(a, b, _)| a != b)
            .collect();
        let (s, t) = (0usize, n - 1);
        let mut mincut = i64::MAX;
        for mask in 0u32..(1 << n) {
            if mask >> s & 1 == 1 && mask >> t & 1 == 0 {
                mincut = mincut.min(
                    capacity
                        .iter()
                        .filter(|(a, b, _)| mask >> a & 1 == 1 && mask >> b & 1 == 0)
                        .map(|(_, _, w)| w)
                        .sum(),
                );
            }
        }
        if !capacity.iter().any(|(a, _, _)| *a == s) || !capacity.iter().any(|(_, b, _)| *b == t) {
            continue;
        }
        let cap_text = edge_list(&capacity);
        let out = run(&format!("max_flow({cap_text}, v{s}, v{t})"));
        if out.contains("not a vertex") {
            continue;
        }
        assert_eq!(first_number(&out, "max flow"), mincut, "{cap_text}: {out}");
        let reported: i64 = out
            .split("minimum cut:")
            .nth(1)
            .unwrap()
            .split(", ")
            .filter(|p| p.contains("->"))
            .map(|p| {
                let (a, b) = p.trim().split_once("->").unwrap();
                let (a, b): (usize, usize) = (a[1..].parse().unwrap(), b[1..].parse().unwrap());
                capacity
                    .iter()
                    .filter(|(x, y, _)| *x == a && *y == b)
                    .map(|(_, _, w)| w)
                    .sum::<i64>()
            })
            .sum();
        assert_eq!(reported, mincut, "{cap_text}: {out}");
        // Strongly connected components against mutual reachability.
        let mut r = vec![vec![false; n]; n];
        for i in 0..n {
            r[i][i] = true;
        }
        for &(a, b, _) in &capacity {
            r[a][b] = true;
        }
        for k in 0..n {
            for i in 0..n {
                for j in 0..n {
                    if r[i][k] && r[k][j] {
                        r[i][j] = true;
                    }
                }
            }
        }
        let present: Vec<usize> = (0..n)
            .filter(|v| capacity.iter().any(|(a, b, _)| a == v || b == v))
            .collect();
        let mut classes = 0;
        let mut seen = vec![false; n];
        for &v in &present {
            if !seen[v] {
                classes += 1;
                for &u in &present {
                    if r[v][u] && r[u][v] {
                        seen[u] = true;
                    }
                }
            }
        }
        let scc = run(&format!("scc({cap_text})"));
        assert!(
            scc.starts_with(&format!("{classes} strongly connected components")),
            "{cap_text}: {scc}"
        );
    }
    assert_eq!(
        run("dijkstra([[a,b,1/2],[b,c,1/3],[a,c,1]], a, c)"),
        "length 5/6: a - b - c"
    );
    assert_eq!(
        run("max_flow([[s,a,3],[s,b,2],[a,t,2],[b,t,3],[a,b,1]], s, t)")
            .split(',')
            .next()
            .unwrap(),
        "max flow 5"
    );
    assert!(
        UraeSession::new()
            .execute_line("topological_sort([[a,b],[b,c],[c,a]])")
            .is_error
    );
    assert!(
        UraeSession::new()
            .execute_line("dijkstra([[a,b,-1],[b,a,-1]], a, b, directed)")
            .is_error
    );
    assert!(
        UraeSession::new()
            .execute_line("minimum_spanning_tree([[a,b,1],[c,d,2]])")
            .is_error
    );
}

#[test]
fn inverse_z_transform_reproduces_the_series_of_x_of_z() {
    // (numerator, denominator) as descending coefficients of z; the text form is built from them.
    let cases: [(&[f64], &[f64]); 7] = [
        (&[1.0, 0.0], &[1.0, -1.0]),
        (&[1.0, 0.0], &[1.0, -3.0, 2.0]),
        (&[1.0, 0.0, 0.0], &[1.0, -1.0 / 6.0, -1.0 / 6.0]),
        (&[1.0, 0.0], &[1.0, -4.0, 4.0]),
        (&[1.0, 0.0, 0.0], &[1.0, -3.0, 3.0, -1.0]),
        (&[1.0, 1.0], &[1.0, -1.0, -6.0]),
        (&[2.0, 1.0, 5.0], &[1.0, 0.0, -4.0]),
    ];
    let poly = |c: &[f64]| -> String {
        let d = c.len() - 1;
        c.iter()
            .enumerate()
            .map(|(i, v)| format!("({v:?})*z^{}", d - i))
            .collect::<Vec<_>>()
            .join(" + ")
    };
    for (num, den) in cases {
        // Exact decimals are needed for the engine: 1/6 is not, so scale the denominator to integers.
        let scale = if den
            .iter()
            .any(|v| (v * 6.0 - (v * 6.0).round()).abs() < 1e-12 && (v - v.round()).abs() > 1e-12)
        {
            6.0
        } else {
            1.0
        };
        let (n_text, d_text) = (
            poly(&num.iter().map(|v| v * scale).collect::<Vec<_>>()),
            poly(&den.iter().map(|v| v * scale).collect::<Vec<_>>()),
        );
        let out = run(&format!("inverse_ztransform(({n_text})/({d_text}), z, k)"));
        let closed = out.split("] = ").nth(1).unwrap().to_string();
        // Split off the impulse terms "(c) delta[k-m]".
        let (body, impulses): (String, Vec<(f64, i64)>) =
            match closed.find(" delta[").or_else(|| closed.find("delta[")) {
                Some(at) => {
                    let start = closed[..at].rfind('(').unwrap();
                    let mut list = Vec::new();
                    for piece in closed[start..].split(" + ") {
                        let (c, rest) = piece.split_once(") delta[k").unwrap();
                        let c = c.trim_start_matches('(');
                        let c: f64 = match c.split_once('/') {
                            Some((a, b)) => a.parse::<f64>().unwrap() / b.parse::<f64>().unwrap(),
                            None => c.parse().unwrap(),
                        };
                        let m: i64 = rest
                            .trim_end_matches(']')
                            .trim_start_matches('-')
                            .parse()
                            .unwrap_or(0);
                        list.push((c, m));
                    }
                    (closed[..start].trim_end_matches(" + ").to_string(), list)
                }
                None => (closed.clone(), Vec::new()),
            };
        // Long division in 1/z.
        let width = den.len();
        let mut b = [0.0; 12];
        for (i, v) in num.iter().enumerate() {
            b[i + width - num.len()] = *v;
        }
        let mut x = [0.0f64; 10];
        for n in 0..10 {
            let mut acc = b[n];
            for i in 1..width.min(n + 1) {
                acc -= den[i] * x[n - i];
            }
            x[n] = acc / den[0];
        }
        for (n, expected) in x.iter().enumerate() {
            let mut value = if body.trim().is_empty() || body == "0" && false {
                0.0
            } else {
                run(&format!("evalf({})", body.replace('k', &format!("({n})"))))
                    .trim()
                    .parse()
                    .unwrap_or_else(|_| panic!("{body}"))
            };
            for (c, m) in &impulses {
                if *m == n as i64 {
                    value += c;
                }
            }
            assert!(
                (value - expected).abs() < 1e-6 * (1.0 + expected.abs()),
                "X = ({n_text})/({d_text}), n = {n}: {value} vs {expected}: {out}"
            );
        }
    }
    assert!(
        UraeSession::new()
            .execute_line("inverse_ztransform(z/(z^2+1), z, n)")
            .is_error
    );
}

#[test]
fn graph_paths_and_matchings_match_exhaustive_search() {
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = move |m: u64| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state % m
    };
    let parse_path = |text: &str, prefix: &str| -> Vec<usize> {
        text.trim_start_matches(prefix)
            .trim()
            .split(" - ")
            .map(|v| v[1..].parse().unwrap())
            .collect()
    };
    for round in 0..40 {
        let n = 4 + (round % 4);
        let mut edges: Vec<(usize, usize)> = Vec::new();
        for a in 0..n {
            for b in a + 1..n {
                if next(100) < 45 {
                    edges.push((a, b));
                }
            }
        }
        if edges.len() < 2 {
            continue;
        }
        let has = |a: usize, b: usize| edges.iter().any(|&(x, y)| (x, y) == (a.min(b), a.max(b)));
        let text = format!(
            "[{}]",
            edges
                .iter()
                .map(|(a, b)| format!("[v{a},v{b}]"))
                .collect::<Vec<_>>()
                .join(",")
        );
        let present: Vec<usize> = (0..n)
            .filter(|v| edges.iter().any(|&(a, b)| a == *v || b == *v))
            .collect();
        let m = present.len();
        // Hamiltonian path / cycle through the vertices that occur in the edge list.
        let mut found_path = false;
        let mut found_cycle = false;
        let mut order: Vec<usize> = present.clone();
        fn permute(k: usize, order: &mut Vec<usize>, visit: &mut dyn FnMut(&[usize])) {
            if k == order.len() {
                visit(order);
                return;
            }
            for i in k..order.len() {
                order.swap(k, i);
                permute(k + 1, order, visit);
                order.swap(k, i);
            }
        }
        permute(0, &mut order, &mut |o: &[usize]| {
            if o.windows(2).all(|w| has(w[0], w[1])) {
                found_path = true;
                if m >= 3 && has(o[m - 1], o[0]) {
                    found_cycle = true;
                }
            }
        });
        let path = run(&format!("hamiltonian_path({text})"));
        let cycle = run(&format!("hamiltonian_cycle({text})"));
        assert_eq!(path != "none", found_path, "{text}: {path}");
        assert_eq!(cycle != "none", found_cycle, "{text}: {cycle}");
        if found_path {
            let p = parse_path(&path, "");
            assert!(
                p.len() == m && p.windows(2).all(|w| has(w[0], w[1])),
                "{text}: {path}"
            );
        }
        if found_cycle {
            let c = parse_path(&cycle, "");
            assert!(
                c.len() == m + 1 && c[0] == c[m] && c.windows(2).all(|w| has(w[0], w[1])),
                "{text}: {cycle}"
            );
        }
        // Maximum matching against every edge subset.
        let mut best = 0;
        for mask in 0u32..1 << edges.len() {
            let mut used = 0u32;
            let mut ok = true;
            for (i, &(a, b)) in edges.iter().enumerate() {
                if mask >> i & 1 == 1 {
                    if used >> a & 1 == 1 || used >> b & 1 == 1 {
                        ok = false;
                        break;
                    }
                    used |= 1 << a | 1 << b;
                }
            }
            if ok {
                best = best.max(mask.count_ones() as usize);
            }
        }
        let matching = run(&format!("max_matching({text})"));
        assert_eq!(
            matching
                .split(':')
                .next()
                .unwrap()
                .parse::<usize>()
                .unwrap(),
            best,
            "{text}: {matching}"
        );
        // Euler: parity of the degrees and connectivity of the edges; the trail uses every edge once.
        let degree = |v: usize| edges.iter().filter(|&&(a, b)| a == v || b == v).count();
        let odd = present.iter().filter(|v| degree(**v) % 2 == 1).count();
        let mut seen = vec![present[0]];
        let mut grew = true;
        while grew {
            grew = false;
            for &(a, b) in &edges {
                if seen.contains(&a) != seen.contains(&b) {
                    seen.push(a);
                    seen.push(b);
                    grew = true;
                }
            }
        }
        let connected = present.iter().all(|v| seen.contains(v));
        let eulerian_trail = connected && (odd == 0 || odd == 2);
        assert_eq!(
            run(&format!("is_eulerian({text})")) == "true",
            connected && odd == 0,
            "{text}"
        );
        let trail = run(&format!("euler_path({text})"));
        assert_eq!(trail != "none", eulerian_trail, "{text}: {trail}");
        if eulerian_trail {
            let walk = parse_path(trail.split(": ").nth(1).unwrap(), "");
            assert_eq!(walk.len(), edges.len() + 1, "{text}: {trail}");
            let mut left = edges.clone();
            for w in walk.windows(2) {
                let key = (w[0].min(w[1]), w[0].max(w[1]));
                let at = left
                    .iter()
                    .position(|e| *e == key)
                    .unwrap_or_else(|| panic!("{text}: {trail}"));
                left.remove(at);
            }
            assert!(left.is_empty(), "{text}: {trail}");
        }
    }
    assert_eq!(run("is_eulerian(K5)"), "true");
    assert_eq!(run("is_eulerian(K4)"), "false");
    assert!(run("hamiltonian_cycle(C6)").starts_with('1'));
    assert_eq!(run("hamiltonian_cycle(P4)"), "none");
    assert!(run("max_matching(K6)").starts_with("3:"));
}

#[test]
fn logistic_regression_satisfies_the_score_equations_and_the_two_by_two_closed_form() {
    let field = |text: &str, key: &str| -> f64 {
        let after = text
            .split(key)
            .nth(1)
            .unwrap_or_else(|| panic!("{key} in {text}"));
        after
            .trim_start_matches([' ', '='])
            .split([',', ' ', ';', ')'])
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    // A binary predictor: the slope is the log odds ratio of the 2x2 table and the intercept the log odds of the baseline group.
    // Group 0: 7 successes in 20; group 1: 15 in 25.
    let mut x = Vec::new();
    let mut y = Vec::new();
    for (group, successes, total) in [(0, 7, 20), (1, 15, 25)] {
        for i in 0..total {
            x.push(format!("[{group}]"));
            y.push(if i < successes { "1" } else { "0" });
        }
    }
    let out = run(&format!(
        "logistic_regress([{}], [{}])",
        x.join(","),
        y.join(",")
    ));
    let (b0, b1) = (
        field(&out, "intercept ="),
        field(out.split("x1").nth(1).unwrap(), "="),
    );
    assert!((b0 - (7.0f64 / 13.0).ln()).abs() < 1e-5, "{out}");
    assert!(
        (b1 - ((15.0f64 / 10.0) / (7.0 / 13.0)).ln()).abs() < 1e-5,
        "{out}"
    );
    // Wald standard error of a log odds ratio: sqrt(1/a + 1/b + 1/c + 1/d).
    let se = (1.0f64 / 7.0 + 1.0 / 13.0 + 1.0 / 15.0 + 1.0 / 10.0).sqrt();
    assert!(
        (field(out.split("x1").nth(1).unwrap(), "se ") - se).abs() < 1e-5,
        "{out}"
    );
    // Continuous predictors: the score equations X^T (y - p) = 0 hold at the reported coefficients.
    let data: [(f64, f64, u8); 14] = [
        (0.5, 1.0, 0),
        (1.0, 3.0, 0),
        (1.5, 2.0, 0),
        (2.0, 5.0, 1),
        (2.5, 1.0, 0),
        (3.0, 4.0, 1),
        (3.5, 2.0, 0),
        (4.0, 6.0, 1),
        (4.5, 3.0, 1),
        (5.0, 1.0, 0),
        (5.5, 5.0, 1),
        (6.0, 2.0, 1),
        (2.2, 4.0, 0),
        (4.8, 6.0, 0),
    ];
    let xs = format!(
        "[{}]",
        data.iter()
            .map(|(a, b, _)| format!("[{a:?},{b:?}]"))
            .collect::<Vec<_>>()
            .join(",")
    );
    let ys = format!(
        "[{}]",
        data.iter()
            .map(|d| d.2.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    let out = run(&format!("logit({xs}, {ys})"));
    let beta = [
        field(&out, "intercept ="),
        field(out.split("x1").nth(1).unwrap(), "="),
        field(out.split("x2").nth(1).unwrap(), "="),
    ];
    let mut score = [0.0f64; 3];
    let mut log_likelihood = 0.0;
    for (a, b, outcome) in data {
        let eta = beta[0] + beta[1] * a + beta[2] * b;
        let p = 1.0 / (1.0 + (-eta).exp());
        let r = outcome as f64 - p;
        score[0] += r;
        score[1] += r * a;
        score[2] += r * b;
        log_likelihood += outcome as f64 * eta - (1.0 + eta.exp()).ln();
    }
    assert!(
        score.iter().all(|s| s.abs() < 1e-4),
        "{out}: score {score:?}"
    );
    assert!(
        (field(&out, "log-likelihood =") - log_likelihood).abs() < 1e-4,
        "{out}"
    );
    assert!(
        (field(out.split("x1").nth(1).unwrap(), "odds ratio ") - beta[1].exp()).abs() < 1e-4,
        "{out}"
    );
    // Perfect separation and bad outcomes are refused.
    assert!(
        UraeSession::new()
            .execute_line("logit([[1],[2],[3],[4],[5],[6]], [0,0,0,1,1,1])")
            .is_error
    );
    assert!(
        UraeSession::new()
            .execute_line("logit([[1],[2],[3],[4]], [0,2,1,0])")
            .is_error
    );
}

#[test]
fn prediction_intervals_match_the_closed_form_and_the_intercept_interval() {
    let interval = |text: &str, key: &str| -> (f64, f64) {
        let inner = text
            .split(key)
            .nth(1)
            .unwrap()
            .split('(')
            .nth(1)
            .unwrap()
            .split(')')
            .next()
            .unwrap();
        let (a, b) = inner.split_once(", ").unwrap();
        (a.parse().unwrap(), b.parse().unwrap())
    };
    let list = |v: &[f64]| {
        format!(
            "[{}]",
            v.iter()
                .map(|e| format!("{e:?}"))
                .collect::<Vec<_>>()
                .join(",")
        )
    };
    let datasets: [(&[f64], &[f64]); 3] = [
        (&[1.0, 2.0, 3.0, 4.0, 5.0], &[2.0, 4.0, 5.0, 4.0, 5.0]),
        (
            &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
            &[6.0, 5.0, 7.0, 3.0, 2.0, 2.0],
        ),
        (
            &[0.5, 1.5, 2.5, 3.0, 4.5, 6.0, 7.5],
            &[1.0, 2.2, 2.9, 4.1, 5.2, 7.4, 8.1],
        ),
    ];
    for (x, y) in datasets {
        let n = x.len() as f64;
        let (mx, my) = (x.iter().sum::<f64>() / n, y.iter().sum::<f64>() / n);
        let sxx: f64 = x.iter().map(|a| (a - mx).powi(2)).sum();
        let sxy: f64 = x.iter().zip(y).map(|(a, b)| (a - mx) * (b - my)).sum();
        let slope = sxy / sxx;
        let sse: f64 = x
            .iter()
            .zip(y)
            .map(|(a, b)| (b - my - slope * (a - mx)).powi(2))
            .sum();
        let s2 = sse / (n - 2.0);
        for (x0, level) in [
            (mx, 0.95),
            (x[0] - 1.0, 0.95),
            (x[x.len() - 1] + 2.5, 0.9),
            (0.0, 0.95),
        ] {
            let out = run(&format!(
                "predict({}, {}, {x0:?}, {level:?})",
                list(x),
                list(y)
            ));
            let fit = my + slope * (x0 - mx);
            let critical: f64 = run(&format!(
                "t_quantile({}, {})",
                1.0 - (1.0 - level) / 2.0,
                n - 2.0
            ))
            .trim()
            .parse()
            .unwrap();
            let h = 1.0 / n + (x0 - mx).powi(2) / sxx;
            let (lo, hi) = interval(&out, "confidence interval for the mean");
            assert!(
                (lo - (fit - critical * (s2 * h).sqrt())).abs() < 1e-4
                    && (hi - (fit + critical * (s2 * h).sqrt())).abs() < 1e-4,
                "{out}"
            );
            let (plo, phi) = interval(&out, "prediction interval");
            assert!(
                (plo - (fit - critical * (s2 * (1.0 + h)).sqrt())).abs() < 1e-4
                    && (phi - (fit + critical * (s2 * (1.0 + h)).sqrt())).abs() < 1e-4,
                "{out}"
            );
            assert!(
                plo < lo && phi > hi,
                "the prediction interval contains the mean interval: {out}"
            );
        }
        // At x0 = 0 the confidence interval of the mean is the confidence interval of the intercept from regress().
        let at_zero = interval(
            &run(&format!("predict({}, {}, 0)", list(x), list(y))),
            "confidence interval for the mean",
        );
        let regress = run(&format!("regress({}, {})", list(x), list(y)));
        let intercept = regress.split("intercept").nth(1).unwrap();
        let ci = intercept
            .split("95% CI (")
            .nth(1)
            .unwrap()
            .split(')')
            .next()
            .unwrap();
        let (a, b) = ci.split_once(", ").unwrap();
        assert!(
            (at_zero.0 - a.parse::<f64>().unwrap()).abs() < 1e-4
                && (at_zero.1 - b.parse::<f64>().unwrap()).abs() < 1e-4,
            "{regress}"
        );
    }
    assert!(
        UraeSession::new()
            .execute_line("predict([1,2,3],[1,2,3],2, 1.5)")
            .is_error
    );
    assert!(
        UraeSession::new()
            .execute_line("predict([1,1,1],[1,2,3],2)")
            .is_error
    );
}

#[test]
fn graph_isomorphism_and_automorphism_counts_match_permutation_search() {
    let mut state = 0xDEAD_BEEF_CAFE_F00Du64;
    let mut next = move |m: u64| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state % m
    };
    let text = |edges: &[(usize, usize)]| {
        format!(
            "[{}]",
            edges
                .iter()
                .map(|(a, b)| format!("[v{a},v{b}]"))
                .collect::<Vec<_>>()
                .join(",")
        )
    };
    fn each_permutation(k: usize, order: &mut Vec<usize>, visit: &mut dyn FnMut(&[usize])) {
        if k == order.len() {
            visit(order);
            return;
        }
        for i in k..order.len() {
            order.swap(k, i);
            each_permutation(k + 1, order, visit);
            order.swap(k, i);
        }
    }
    for round in 0..30 {
        let n = 4 + round % 3;
        let random_edges = |next: &mut dyn FnMut(u64) -> u64| -> Vec<(usize, usize)> {
            let mut edges = Vec::new();
            for a in 0..n {
                for b in a + 1..n {
                    if next(100) < 50 {
                        edges.push((a, b));
                    }
                }
            }
            edges
        };
        let g = random_edges(&mut next);
        if g.len() < 2 {
            continue;
        }
        // A relabelled copy is always isomorphic.
        let mut perm: Vec<usize> = (0..n).collect();
        for i in (1..n).rev() {
            perm.swap(i, next(i as u64 + 1) as usize);
        }
        let copy: Vec<(usize, usize)> = g.iter().map(|&(a, b)| (perm[a], perm[b])).collect();
        assert_eq!(
            run(&format!("is_isomorphic({}, {})", text(&g), text(&copy))),
            "true",
            "{g:?} / {copy:?}"
        );
        // An independent random graph: compare with the permutation search over the vertices both lists mention.
        let h = random_edges(&mut next);
        if h.is_empty() {
            continue;
        }
        let present = |e: &[(usize, usize)]| {
            (0..n)
                .filter(|v| e.iter().any(|&(a, b)| a == *v || b == *v))
                .collect::<Vec<_>>()
        };
        let (pg, ph) = (present(&g), present(&h));
        let mut expected = false;
        if pg.len() == ph.len() && g.len() == h.len() {
            let has_h = |a: usize, b: usize| h.contains(&(a.min(b), a.max(b)));
            let mut order = ph.clone();
            each_permutation(0, &mut order, &mut |o: &[usize]| {
                // pg[i] -> o[i]
                let image = |v: usize| o[pg.iter().position(|x| *x == v).unwrap()];
                if g.iter().all(|&(a, b)| has_h(image(a), image(b))) {
                    expected = true;
                }
            });
        }
        assert_eq!(
            run(&format!("is_isomorphic({}, {})", text(&g), text(&h))) == "true",
            expected,
            "{g:?} / {h:?}"
        );
        // Automorphism count over the vertices of g.
        let mut count = 0u64;
        let mut order = pg.clone();
        let has_g = |a: usize, b: usize| g.contains(&(a.min(b), a.max(b)));
        each_permutation(0, &mut order, &mut |o: &[usize]| {
            let image = |v: usize| o[pg.iter().position(|x| *x == v).unwrap()];
            if g.iter().all(|&(a, b)| has_g(image(a), image(b))) {
                count += 1;
            }
        });
        assert_eq!(
            run(&format!("automorphisms({})", text(&g))),
            count.to_string(),
            "{g:?}"
        );
    }
    assert_eq!(run("automorphisms(K4)"), "24");
    assert_eq!(run("automorphisms(C5)"), "10");
    assert_eq!(run("automorphisms(P4)"), "2");
    assert_eq!(run("automorphisms(S4)"), "24");
    assert_eq!(
        run("is_isomorphic(C6, [[1,2],[2,3],[3,4],[4,5],[5,6],[6,1]])"),
        "true"
    );
    assert_eq!(
        run("is_isomorphic(C6, [[1,2],[2,3],[3,1],[4,5],[5,6],[6,4]])"),
        "false"
    );
}

#[test]
fn poisson_regression_satisfies_the_score_equations_and_the_group_closed_form() {
    let field = |text: &str, key: &str| -> f64 {
        let after = text
            .split(key)
            .nth(1)
            .unwrap_or_else(|| panic!("{key} in {text}"));
        after
            .trim_start_matches([' ', '='])
            .split([',', ' ', ';', ')'])
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    // One binary predictor: exp(intercept) is the mean of group 0, the rate ratio the ratio of the group means, and the
    // Wald variance of the slope is 1/sum0 + 1/sum1.
    let group0 = [2, 3, 1, 4, 2, 3];
    let group1 = [6, 8, 5, 9, 7];
    let xs = format!(
        "[{}]",
        group0
            .iter()
            .map(|_| "[0]")
            .chain(group1.iter().map(|_| "[1]"))
            .collect::<Vec<_>>()
            .join(",")
    );
    let ys = format!(
        "[{}]",
        group0
            .iter()
            .chain(&group1)
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    let out = run(&format!("poisson_regress({xs}, {ys})"));
    let (sum0, sum1) = (
        group0.iter().sum::<i32>() as f64,
        group1.iter().sum::<i32>() as f64,
    );
    let (mean0, mean1) = (sum0 / group0.len() as f64, sum1 / group1.len() as f64);
    assert!(
        (field(&out, "intercept =") - mean0.ln()).abs() < 1e-5,
        "{out}"
    );
    let slope_part = out.split("x1").nth(1).unwrap();
    assert!(
        (field(slope_part, "=") - (mean1 / mean0).ln()).abs() < 1e-5,
        "{out}"
    );
    assert!(
        (field(slope_part, "rate ratio ") - mean1 / mean0).abs() < 1e-4,
        "{out}"
    );
    assert!(
        (field(slope_part, "se ") - (1.0 / sum0 + 1.0 / sum1).sqrt()).abs() < 1e-5,
        "{out}"
    );
    // Continuous predictors: X^T (y - mu) = 0 and the log-likelihood.
    let data: [(f64, f64, u32); 12] = [
        (0.5, 1.0, 1),
        (1.0, 3.0, 2),
        (1.5, 2.0, 2),
        (2.0, 5.0, 6),
        (2.5, 1.0, 3),
        (3.0, 4.0, 7),
        (3.5, 2.0, 5),
        (4.0, 6.0, 12),
        (4.5, 3.0, 9),
        (5.0, 1.0, 8),
        (5.5, 5.0, 17),
        (6.0, 2.0, 15),
    ];
    let xs = format!(
        "[{}]",
        data.iter()
            .map(|(a, b, _)| format!("[{a:?},{b:?}]"))
            .collect::<Vec<_>>()
            .join(",")
    );
    let ys = format!(
        "[{}]",
        data.iter()
            .map(|d| d.2.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    let out = run(&format!("poisson_regress({xs}, {ys})"));
    let beta = [
        field(&out, "intercept ="),
        field(out.split("x1").nth(1).unwrap(), "="),
        field(out.split("x2").nth(1).unwrap(), "="),
    ];
    let mut score = [0.0f64; 3];
    let mut log_likelihood = 0.0;
    for (a, b, count) in data {
        let eta = beta[0] + beta[1] * a + beta[2] * b;
        let mu = eta.exp();
        let r = count as f64 - mu;
        score[0] += r;
        score[1] += r * a;
        score[2] += r * b;
        let factorial: f64 = (1..=count).map(|v| (v as f64).ln()).sum();
        log_likelihood += count as f64 * eta - mu - factorial;
    }
    assert!(
        score.iter().all(|s| s.abs() < 1e-3),
        "{out}: score {score:?}"
    );
    assert!(
        (field(&out, "log-likelihood =") - log_likelihood).abs() < 1e-4,
        "{out}"
    );
    assert!(
        UraeSession::new()
            .execute_line("poisson_regress([[1],[2],[3],[4]], [1,-2,3,4])")
            .is_error
    );
    assert!(
        UraeSession::new()
            .execute_line("poisson_regress([[1],[2],[3],[4]], [1,2.5,3,4])")
            .is_error
    );
}

#[test]
fn jordan_form_recovers_the_blocks_of_a_conjugated_matrix() {
    let mut state = 0x0F1E_2D3C_4B5A_6978u64;
    let mut next = move |m: u64| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state % m
    };
    // Block structures as (eigenvalue, size) lists.
    let structures: [&[(i64, usize)]; 8] = [
        &[(2, 2), (3, 1)],
        &[(1, 3)],
        &[(1, 2), (1, 1), (-2, 1)],
        &[(0, 2), (0, 2)],
        &[(4, 1), (4, 1), (5, 2)],
        &[(-1, 3), (2, 1)],
        &[(3, 1), (3, 1), (3, 1)],
        &[(2, 2), (2, 2), (-3, 1)],
    ];
    for blocks in structures {
        let n: usize = blocks.iter().map(|b| b.1).sum();
        // J.
        let mut j = vec![vec![0i64; n]; n];
        let mut at = 0;
        for &(lambda, size) in blocks {
            for k in 0..size {
                j[at + k][at + k] = lambda;
                if k + 1 < size {
                    j[at + k][at + k + 1] = 1;
                }
            }
            at += size;
        }
        // A = P J P^-1 for a product of elementary integer operations (P and P^-1 stay integral).
        let mut a = j.clone();
        for _ in 0..6 {
            let (r, s) = (next(n as u64) as usize, next(n as u64) as usize);
            if r == s {
                continue;
            }
            let factor = next(3) as i64 - 1;
            // Conjugation by E = I + factor e_r e_s^T: A -> E A E^-1.
            for c in 0..n {
                let v = a[s][c];
                a[r][c] += factor * v;
            }
            for row in a.iter_mut() {
                let v = row[r];
                row[s] -= factor * v;
            }
        }
        let text = format!(
            "[{}]",
            a.iter()
                .map(|r| format!(
                    "[{}]",
                    r.iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                ))
                .collect::<Vec<_>>()
                .join(",")
        );
        let out = run(&format!("jordan({text})"));
        let reported: Vec<(i64, usize)> = out
            .split("with blocks ")
            .nth(1)
            .unwrap()
            .split(", ")
            .map(|b| {
                let size: usize = b[1..b.find('(').unwrap()].parse().unwrap();
                let lambda: i64 = b[b.find('(').unwrap() + 1..b.len() - 1].parse().unwrap();
                (lambda, size)
            })
            .collect();
        let mut expected: Vec<(i64, usize)> = blocks.to_vec();
        let mut got = reported.clone();
        expected.sort();
        got.sort();
        assert_eq!(got, expected, "{text}: {out}");
    }
    assert_eq!(
        run("jordan([[2,1],[0,2]])").split(" with").next().unwrap(),
        "[ 2, 1 ; 0, 2 ]"
    );
    assert!(
        UraeSession::new()
            .execute_line("jordan([[0,-1],[1,0]])")
            .is_error
    );
    assert!(
        UraeSession::new()
            .execute_line("jordan([[1,2,3],[4,5,6]])")
            .is_error
    );
}

#[test]
fn hessenberg_reduction_is_a_similarity_to_upper_hessenberg_form() {
    type F = (i128, i128);
    fn gcd(a: i128, b: i128) -> i128 {
        if b == 0 { a.abs() } else { gcd(b, a % b) }
    }
    fn norm((n, d): F) -> F {
        let g = gcd(n, d).max(1);
        let s = if d < 0 { -1 } else { 1 };
        (s * n / g, s * d / g)
    }
    fn add(a: F, b: F) -> F {
        norm((a.0 * b.1 + b.0 * a.1, a.1 * b.1))
    }
    fn mul(a: F, b: F) -> F {
        norm((a.0 * b.0, a.1 * b.1))
    }
    let cell = |t: &str| -> F {
        match t.trim().split_once('/') {
            Some((a, b)) => norm((a.trim().parse().unwrap(), b.trim().parse().unwrap())),
            None => (t.trim().parse().unwrap(), 1),
        }
    };
    let matrix = |t: &str| -> Vec<Vec<F>> {
        t.trim()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .split(';')
            .map(|r| r.split(',').map(cell).collect())
            .collect()
    };
    let product = |a: &Vec<Vec<F>>, b: &Vec<Vec<F>>| -> Vec<Vec<F>> {
        (0..a.len())
            .map(|i| {
                (0..b[0].len())
                    .map(|j| (0..b.len()).fold((0, 1), |s, k| add(s, mul(a[i][k], b[k][j]))))
                    .collect()
            })
            .collect()
    };
    let mut state = 0x5555_AAAA_1234_FEDCu64;
    let mut next = move |m: u64| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % m) as i128 - (m as i128) / 2
    };
    for n in [3usize, 4, 5, 6, 5, 4] {
        let mut data: Vec<Vec<i128>> = (0..n).map(|_| (0..n).map(|_| next(9)).collect()).collect();
        if n == 5 {
            // Zeros in the pivot column exercise the row-swap branch.
            for row in data.iter_mut().skip(1) {
                row[0] = 0;
            }
            data[3][0] = 2;
        }
        let a: Vec<Vec<F>> = data
            .iter()
            .map(|r| r.iter().map(|v| (*v, 1)).collect())
            .collect();
        let text = format!(
            "[{}]",
            data.iter()
                .map(|r| format!(
                    "[{}]",
                    r.iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                ))
                .collect::<Vec<_>>()
                .join(",")
        );
        let out = run(&format!("hessenberg({text})"));
        let (h_text, p_text) = out
            .strip_prefix("H = ")
            .unwrap()
            .split_once("; P = ")
            .unwrap();
        let (h, p) = (matrix(h_text), matrix(p_text));
        for (i, row) in h.iter().enumerate() {
            for (j, v) in row.iter().enumerate() {
                if i > j + 1 {
                    assert_eq!(*v, (0, 1), "H not Hessenberg: {out}");
                }
            }
        }
        // A P = P H, and P is invertible (its inverse exists in the engine).
        assert_eq!(
            product(&a, &p),
            product(&p, &h),
            "A P = P H for {text}: {out}"
        );
        assert_ne!(
            run(&format!("det({p_text})")).trim(),
            "0",
            "P is singular: {out}"
        );
        let trace = |m: &Vec<Vec<F>>| (0..n).fold((0, 1), |s, i| add(s, m[i][i]));
        assert_eq!(trace(&a), trace(&h), "{out}");
        assert_eq!(trace(&product(&a, &a)), trace(&product(&h, &h)), "{out}");
    }
    assert!(
        UraeSession::new()
            .execute_line("hessenberg([[1,2,3],[4,5,6]])")
            .is_error
    );
}

#[test]
fn matrix_square_root_squares_back_and_is_exact_when_the_root_is_rational() {
    let mut state = 0x7777_3333_1111_AAAAu64;
    let mut next = move |m: u64| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % m) as i64
    };
    let literal = |m: &Vec<Vec<i64>>| {
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
    };
    let square = |m: &Vec<Vec<i64>>| -> Vec<Vec<i64>> {
        (0..m.len())
            .map(|i| {
                (0..m.len())
                    .map(|j| (0..m.len()).map(|k| m[i][k] * m[k][j]).sum())
                    .collect()
            })
            .collect()
    };
    // S with positive eigenvalues (triangular with a positive diagonal, then conjugated by an integer shear): the principal root of S^2 is S.
    for n in [2usize, 3, 4] {
        for _ in 0..4 {
            let mut s: Vec<Vec<i64>> = (0..n)
                .map(|i| {
                    (0..n)
                        .map(|j| {
                            if j > i {
                                next(5) - 2
                            } else if i == j {
                                1 + next(4)
                            } else {
                                0
                            }
                        })
                        .collect()
                })
                .collect();
            let (r, c) = (next(n as u64) as usize, next(n as u64) as usize);
            if r != c {
                // Conjugation by E = I + e_r e_c^T keeps the eigenvalues.
                for k in 0..n {
                    let v = s[c][k];
                    s[r][k] += v;
                }
                for row in s.iter_mut() {
                    let v = row[r];
                    row[c] -= v;
                }
            }
            let a = square(&s);
            let out = run(&format!("sqrtm({})", literal(&a)));
            let expected = run(&literal(&s));
            assert_eq!(out, expected, "sqrtm of {a:?}: {out}");
        }
    }
    // Irrational roots: the decimals square back to A.
    let decimals = |text: &str| -> Vec<Vec<f64>> {
        text.trim()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .split(';')
            .map(|r| r.split(',').map(|c| c.trim().parse().unwrap()).collect())
            .collect()
    };
    for a in [
        vec![vec![2i64, 0], vec![0, 3]],
        vec![vec![2, 1], vec![1, 2]],
        vec![vec![5, 2, 0], vec![2, 5, 1], vec![0, 1, 4]],
        vec![vec![1, 1], vec![0, 2]],
    ] {
        let n = a.len();
        let s = decimals(&run(&format!("sqrtm({})", literal(&a))));
        for i in 0..n {
            for j in 0..n {
                let value: f64 = (0..n).map(|k| s[i][k] * s[k][j]).sum();
                assert!((value - a[i][j] as f64).abs() < 1e-8, "{a:?}: {s:?}");
            }
        }
    }
    assert_eq!(run("sqrtm([[4,0],[0,9]])"), "[ 2, 0 ; 0, 3 ]");
    assert!(
        UraeSession::new()
            .execute_line("sqrtm([[-1,0],[0,4]])")
            .is_error
    );
    assert!(
        UraeSession::new()
            .execute_line("sqrtm([[1,2,3],[4,5,6]])")
            .is_error
    );
}
