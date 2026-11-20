//! Side conditions of simplification: which points does the result no longer exclude?

use algebra_core::ExprGraph;
use algebra_core::parser::ExprParser;
use algebra_engine::Simplifier;

fn excluded(input: &str) -> (String, Vec<String>) {
    let g = ExprGraph::new();
    let id = ExprParser::new(&g).parse(input).unwrap();
    let r = Simplifier::simplify_with_conditions(&g, id).unwrap();
    let text = algebra_core::format::Formatter::format(
        &algebra_core::format::UnicodeFormatter,
        &g,
        r.expr,
    )
    .unwrap();
    (text, r.describe_excluded(&g))
}

#[test]
fn cancelled_denominators_are_reported() {
    assert_eq!(
        excluded("x/x"),
        ("1".to_string(), vec!["x ≠ 0".to_string()])
    );
    let (result, ex) = excluded("(x^2-1)/(x-1)");
    assert_eq!(result, "x + 1");
    assert_eq!(ex.len(), 1, "{ex:?}");
    assert!(ex[0].ends_with("≠ 0") && ex[0].contains('x'), "{ex:?}");
    // Only the factor that was cancelled is reported, not the one that stays in the denominator.
    let (result, ex) = excluded("(x*y)/(x*z)");
    assert_eq!(result, "y / z");
    assert_eq!(ex, vec!["x ≠ 0".to_string()]);
}

#[test]
fn lost_domain_restrictions_are_reported() {
    assert_eq!(
        excluded("exp(ln(x))"),
        ("x".to_string(), vec!["x > 0".to_string()])
    );
    let (result, ex) = excluded("tan(x)*cos(x)");
    assert_eq!(result, "sin(x)");
    assert_eq!(ex, vec!["cos(x) ≠ 0".to_string()]);
}

#[test]
fn unconditional_identities_and_kept_restrictions_report_nothing() {
    for input in [
        "sin(x)^2+cos(x)^2",
        "x + 0",
        "2*x + 3*x",
        "1/(x^2+1)",
        "1/x",
        "ln(x)",
        "sqrt(x) + 1",
    ] {
        let (_, ex) = excluded(input);
        assert!(ex.is_empty(), "{input}: {ex:?}");
    }
}

#[test]
fn the_session_result_carries_the_conditions_of_a_simplify_command() {
    let mut session = algebra_engine::session::UraeSession::new();
    let cancelled = session.execute_line("simplify x/x");
    assert!(!cancelled.is_error, "{cancelled:?}");
    assert_eq!(cancelled.output_text, "1");
    assert_eq!(
        cancelled.metadata.get("conditions").map(String::as_str),
        Some("x ≠ 0")
    );
    // An unconditional identity carries no conditions.
    let plain = session.execute_line("simplify sin(x)^2 + cos(x)^2");
    assert!(!plain.metadata.contains_key("conditions"), "{plain:?}");
}
