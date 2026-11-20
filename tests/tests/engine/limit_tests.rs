//! Limits against known values. Finite-point limits are derived symbolically; at infinity the asymptotic rules come
//! first and a numerical estimate (labelled as one) covers what they do not.

use algebra_engine::session::UraeSession;

fn limit(f: &str, point: &str) -> (String, Option<String>, bool) {
    let r = UraeSession::new().execute_line(&format!("limit({f}, x, {point})"));
    let value = r.output_text.rsplit(" = ").next().unwrap_or("").to_string();
    (
        value,
        r.domain_info.clone(),
        r.metadata.get("method").is_some_and(|m| m == "numerical"),
    )
}

#[test]
fn finite_point_limits_are_exact() {
    for (f, p, want) in [
        ("sin(x)/x", "0", "1"),
        ("(1-cos(x))/x^2", "0", "1/2"),
        ("(sin(x)-x)/x^3", "0", "-1/6"),
        ("(tan(x)-sin(x))/x^3", "0", "1/2"),
        ("ln(cos(x))/x^2", "0", "-1/2"),
        ("(cos(x)-1+x^2/2)/x^4", "0", "1/24"),
        ("sin(3*x)/sin(5*x)", "0", "3/5"),
        ("(x^3-8)/(x-2)", "2", "12"),
        ("x^x", "0", "1"),
        ("(1+x)^(1/x)", "0", "e"),
    ] {
        let (value, note, numerical) = limit(f, p);
        assert_eq!(value, want, "limit of {f} at {p}");
        assert!(
            note.is_none() && !numerical,
            "{f}: exact limits carry no numerical note"
        );
    }
}

#[test]
fn limits_at_infinity_known_values() {
    for (f, want) in [
        ("(1+1/x)^x", "e"),
        ("(x^2+1)/(2*x^2-3)", "1/2"),
        ("sqrt(x^2+x)-x", "1/2"),
        ("ln(x)/x", "0"),
        ("x^2*exp(-x)", "0"),
        ("atan(x)", "1/2 · π"),
        ("x^(1/x)", "1"),
        ("(3*x+1)/(x+2)", "3"),
    ] {
        assert_eq!(limit(f, "oo").0, want, "limit of {f} at oo");
    }
}

#[test]
fn numerical_estimates_are_labelled_and_correct() {
    for (f, want) in [
        ("x*sin(1/x)", "1"),
        ("x*(sqrt(x^2+1)-x)", "1/2"),
        ("(x+1)^(1/ln(x+1))", "e"),
        ("sin(x)/x", "0"),
        ("exp(x)/x^3", "∞"),
        ("x*(exp(1/x)-1)", "1"),
        ("x^2*(1-cos(1/x))", "1/2"),
    ] {
        let (value, note, numerical) = limit(f, "oo");
        assert_eq!(value, want, "limit of {f} at oo");
        assert!(numerical, "{f}: this one is estimated, and must say so");
        assert!(
            note.as_deref()
                .is_some_and(|n| n.contains("numerical estimate")),
            "{note:?}"
        );
    }
}

#[test]
fn oscillating_or_slowly_growing_functions_are_not_given_a_value() {
    for f in ["sin(x)", "x*sin(x)", "cos(x)^2", "sin(ln(x))"] {
        assert_eq!(limit(f, "oo").0, "Undefined", "{f}");
    }
}
