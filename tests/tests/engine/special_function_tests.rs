//! Gamma, log-gamma, erf, erfc and zeta: exact values where they are elementary, and correctly rounded numerical
//! values checked against published constants and identities.

use algebra_engine::session::UraeSession;

fn run(q: &str) -> String {
    let r = UraeSession::new().execute_line(q);
    assert!(!r.is_error, "{q}: {}", r.output_text);
    r.output_text
}

fn value(q: &str) -> f64 {
    run(q)
        .parse()
        .unwrap_or_else(|_| panic!("{q} -> {}", run(q)))
}

fn close(a: f64, b: f64, ulps: f64) -> bool {
    (a - b).abs() <= ulps * f64::EPSILON * b.abs().max(f64::MIN_POSITIVE)
}

#[test]
fn exact_values() {
    assert_eq!(run("gamma(5)"), "24");
    assert_eq!(run("gamma(1)"), "1");
    assert_eq!(run("gamma(10)"), "362880");
    assert_eq!(run("gamma(1/2)"), "√(π)");
    assert_eq!(run("gamma(5/2)"), "3/4 · √(π)");
    assert_eq!(run("gamma(7/2)"), "15/8 · √(π)");
    assert_eq!(run("gamma(-1/2)"), "-2 · √(π)");
    assert_eq!(run("gamma(-3/2)"), "4/3 · √(π)");
    assert_eq!(run("zeta(2)"), "1/6 · π^2");
    assert_eq!(run("zeta(4)"), "1/90 · π^4");
    assert_eq!(run("zeta(6)"), "1/945 · π^6");
    assert_eq!(run("zeta(0)"), "-1/2");
    assert_eq!(run("zeta(-1)"), "-1/12");
    assert_eq!(run("zeta(-3)"), "1/120");
    assert_eq!(run("zeta(-2)"), "0");
    assert_eq!(run("erf(0)"), "0");
    assert_eq!(run("erfc(0)"), "1");
}

#[test]
fn numerical_values_match_published_constants() {
    // gamma(1/2) = sqrt(pi) used to come out with a relative error of 1e-13.
    assert!(close(
        value("N(gamma(1/2))"),
        std::f64::consts::PI.sqrt(),
        2.0
    ));
    assert!(close(value("N(gamma(0.3))"), 2.991_568_987_687_59, 4.0));
    // Gamma(21/2) = 19!! / 2^10 * sqrt(pi) = 654729075 / 1024 * sqrt(pi)
    assert!(close(
        value("N(gamma(10.5))"),
        654_729_075.0 / 1024.0 * std::f64::consts::PI.sqrt(),
        4.0
    ));
    assert!(close(value("N(erf(1))"), 0.842_700_792_949_714_9, 2.0));
    assert!(close(value("N(erf(0.5))"), 0.520_499_877_813_046_5, 2.0));
    assert!(close(value("N(erfc(1))"), 0.157_299_207_050_285_13, 2.0));
    // The tail: erfc(5) = 1.5374597944280349e-12 needs relative accuracy beyond 1 - erf.
    assert!(close(value("N(erfc(5))"), 1.537_459_794_428_035e-12, 4.0));
    assert!(close(value("N(lgamma(10))"), 12.801_827_480_081_469, 4.0)); // ln 9! = ln 362880
}

#[test]
fn identities_at_high_precision() {
    // Gamma(x + 1) = x Gamma(x); Gamma(x) Gamma(1 - x) = pi / sin(pi x); erf(x) + erfc(x) = 1; erf is odd;
    // Gamma(1/2)^2 = pi; Legendre duplication. Each difference is below 10^-45 of 60 digits.
    let tiny = |expr: &str| {
        let r = run(&format!("N({expr}, 60)"));
        let t = r.trim_start_matches('-');
        t == "0" || t.starts_with(&format!("0.{}", "0".repeat(44)))
    };
    assert!(tiny("gamma(7/3 + 1) - (7/3)*gamma(7/3)"));
    assert!(tiny("gamma(1/3)*gamma(2/3) - pi/sin(pi/3)"));
    assert!(tiny("erf(3/2) + erfc(3/2) - 1"));
    assert!(tiny("erf(-7/5) + erf(7/5)"));
    assert!(tiny("gamma(1/2)^2 - pi"));
    assert!(tiny(
        "gamma(2*(4/3))*sqrt(pi) - 2^(2*(4/3) - 1)*gamma(4/3)*gamma(4/3 + 1/2)"
    ));
    assert!(tiny("exp(lgamma(21/4)) - gamma(21/4)"));
    // Published digits: Gamma(1/3) = 2.6789385347077476337..., erf(1) = 0.84270079294971486934...
    assert!(run("N(gamma(1/3), 20)").starts_with("2.6789385347077476337"));
    assert!(run("N(erf(1), 20)").starts_with("0.84270079294971486934"));
}
