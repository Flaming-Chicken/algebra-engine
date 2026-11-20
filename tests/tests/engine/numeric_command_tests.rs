//! Numerical and statistical commands against known values and defining properties.

use algebra_engine::session::UraeSession;

fn run(q: &str) -> String {
    let r = UraeSession::new().execute_line(q);
    assert!(!r.is_error, "{q}: {}", r.output_text);
    r.output_text
}

/// The first number after `≈` in the output.
fn approx(text: &str) -> f64 {
    let after = text
        .split('≈')
        .nth(1)
        .unwrap_or_else(|| panic!("no ≈ in {text}"));
    let number: String = after
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | 'e' | 'E' | '+'))
        .collect();
    number
        .parse()
        .unwrap_or_else(|_| panic!("cannot read a number from {text}"))
}

#[test]
fn root_finding() {
    // The Dottie number, the real cube root of 2, sqrt 2 from a bracket, and a function of another variable name.
    assert!((approx(&run("nsolve(cos(x) - x, 1)")) - 0.739_085_133_215_160_7).abs() < 1e-14);
    assert!((approx(&run("findroot(x^3 - 2, x, 1)")) - 2f64.powf(1.0 / 3.0)).abs() < 1e-14);
    assert!((approx(&run("nsolve(x^2 - 2, 1, 2)")) - 2f64.sqrt()).abs() < 1e-14);
    assert!((approx(&run("nsolve(exp(t) - 5, t, 1)")) - 5f64.ln()).abs() < 1e-13);
    // A function with no root in the bracket is an error, not a number.
    assert!(
        UraeSession::new()
            .execute_line("nsolve(x^2 + 1, 0, 3)")
            .is_error
    );
    // A command is not hijacked from an assignment.
    assert!(!UraeSession::new().execute_line("mean = 5").is_error);
}

#[test]
fn numerical_integration() {
    // Known values: the Gaussian integral on [0, 1], sin on [0, pi], the Gaussian over the whole line, exp(-x) on
    // [0, oo), and a function with an integrable singularity at the end point.
    assert!(
        (approx(&run("nintegrate(exp(-x^2), x, 0, 1)")) - 0.746_824_132_812_427_1).abs() < 1e-13
    );
    assert!((approx(&run("nintegrate(sin(x), x, 0, pi)")) - 2.0).abs() < 1e-13);
    assert!(
        (approx(&run("nintegrate(exp(-x^2), x, -oo, oo)")) - std::f64::consts::PI.sqrt()).abs()
            < 1e-10
    );
    assert!((approx(&run("nintegrate(exp(-x), x, 0, oo)")) - 1.0).abs() < 1e-10);
    assert!(
        (approx(&run("nintegrate(1/(1+x^2), x, -oo, oo)")) - std::f64::consts::PI).abs() < 1e-9
    );
    assert!((approx(&run("nintegrate(1/sqrt(x), x, 0, 1)")) - 2.0).abs() < 1e-6);
    // Agrees with the exact symbolic result: integral of x^2 ln(x) on [1, 2] = (8 ln2)/3 - 7/9.
    let exact = 8.0 * 2f64.ln() / 3.0 - 7.0 / 9.0;
    assert!((approx(&run("nintegrate(x^2*ln(x), x, 1, 2)")) - exact).abs() < 1e-13);
}

#[test]
fn optimisation() {
    let min = run("minimize(x^2 - 4*x + 1, x, 0, 5)");
    assert!((approx(&min) - 2.0).abs() < 1e-8, "{min}");
    let max = run("maximize(sin(x), x, 0, 3)");
    assert!(
        (approx(&max) - std::f64::consts::FRAC_PI_2).abs() < 1e-7,
        "{max}"
    );
    let from_start = run("minimize(x^4 - 3*x^2 + x, 1)");
    // x^4 - 3x^2 + x has local minima near 1.13 and -1.30; from 1 Newton finds the one near 1.13.
    assert!((approx(&from_start) - 1.1309).abs() < 1e-3, "{from_start}");
    // A maximum asked for as a minimum is refused.
    assert!(
        UraeSession::new()
            .execute_line("minimize(-x^2, 0.5)")
            .is_error
    );
}

#[test]
fn statistics_are_exact() {
    assert_eq!(run("mean [1, 2, 3, 4]"), "mean = 5/2 ≈ 2.5");
    assert_eq!(run("median [3, 1, 2]"), "median = 2");
    assert_eq!(run("median [4, 1, 3, 2]"), "median = 5/2 ≈ 2.5");
    // The classic data set: population variance 4 and deviation 2; sample variance 32/7.
    assert_eq!(run("pvariance [2, 4, 4, 4, 5, 5, 7, 9]"), "pvariance = 4");
    assert_eq!(run("pstddev [2, 4, 4, 4, 5, 5, 7, 9]"), "pstddev = 2");
    assert!(run("variance [2, 4, 4, 4, 5, 5, 7, 9]").starts_with("variance = 32/7"));
    let sd = run("stddev [2, 4, 4, 4, 5, 5, 7, 9]");
    assert!((approx(&sd) - (32.0f64 / 7.0).sqrt()).abs() < 1e-14, "{sd}");
    assert_eq!(run("mean(1, 2, 3)"), "mean = 2");
    assert_eq!(run("mean [0.5, 1.5]"), "mean = 1");
    assert!(UraeSession::new().execute_line("variance [5]").is_error);
}

#[test]
fn interpolation_and_least_squares() {
    // The interpolating polynomial through points of x^2 is x^2; through points of a cubic it is the cubic.
    assert_eq!(run("interpolate [[0, 0], [1, 1], [2, 4]]"), "x^2");
    let cubic = run("interpolate [[0, 1], [1, 3], [2, 15], [3, 55]]"); // 1 + 2x + 3x^2... checked below
    let r = UraeSession::new();
    let mut s = r;
    for (x, y) in [(0, 1), (1, 3), (2, 15), (3, 55)] {
        s.execute_line(&format!("x = {x}"));
        let value: f64 = s
            .execute_line(&cubic)
            .output_text
            .parse()
            .unwrap_or(f64::NAN);
        assert_eq!(value, y as f64, "{cubic} at {x}");
    }
    // A line through three exactly collinear points is recovered; a noisy fit satisfies the normal equations.
    assert_eq!(run("fit [[0, 1], [1, 3], [2, 5]]"), "2 · x + 1");
    let points = [(0.0, 1.0), (1.0, 2.0), (2.0, 2.0), (3.0, 4.0), (4.0, 4.0)];
    let fitted = run("fit [[0, 1], [1, 2], [2, 2], [3, 4], [4, 4]]");
    // Fit by hand: slope (5*34 - 10*13) / (5*30 - 10^2) = 4/5, intercept (13 - 4/5 * 10) / 5 = 1; the residuals are
    // orthogonal to 1 and to x.
    let (slope, intercept) = (0.8, 1.0);
    let residuals: Vec<f64> = points
        .iter()
        .map(|(x, y)| y - (slope * x + intercept))
        .collect();
    assert!(residuals.iter().sum::<f64>().abs() < 1e-12);
    assert!(
        residuals
            .iter()
            .zip(points)
            .map(|(r, (x, _))| r * x)
            .sum::<f64>()
            .abs()
            < 1e-12
    );
    assert_eq!(fitted, "4/5 · x + 1");
    // Degree two through three points is exact interpolation.
    assert_eq!(run("fit [[0, 0], [1, 1], [2, 4]], 2"), "x^2");
    assert!(
        UraeSession::new()
            .execute_line("interpolate [[1, 2], [1, 3]]")
            .is_error
    );
}

#[test]
fn bad_input_is_an_error_not_a_hang() {
    // These once recursed without end (a non-finite bound or a NaN error estimate never meets the tolerance).
    for q in [
        "nintegrate(1/x, x, -1, 1)",
        "nintegrate(exp(-x^2), x, 1/0, 1)",
        "nintegrate(x, x, 0, a)",
        "nsolve(x, x, 1, 2, 3)",
        "minimize(x^2)",
    ] {
        let started = std::time::Instant::now();
        let r = UraeSession::new().execute_line(q);
        assert!(
            started.elapsed().as_secs_f64() < 20.0,
            "{q} took {:?}",
            started.elapsed()
        );
        // An answer or an error, but not a silent wrong number for the divergent integral.
        if q == "nintegrate(1/x, x, -1, 1)" {
            assert!(
                r.is_error || !r.output_text.contains("≈ 0 "),
                "{q}: {}",
                r.output_text
            );
        }
    }
}
