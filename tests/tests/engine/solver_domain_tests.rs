//! The solver says which solutions are real, which are not, and when only the principal branch is given.

fn solve(q: &str) -> algebra_engine::executor::OperationResult {
    algebra_engine::session::UraeSession::new().execute_line(q)
}

#[test]
fn real_and_complex_solutions_are_counted() {
    let all_real = solve("solve x^2-4=0");
    assert_eq!(all_real.metadata["real_solutions"], "2");
    assert_eq!(all_real.metadata["nonreal_solutions"], "0");
    assert!(all_real.domain_info.is_none(), "{:?}", all_real.domain_info);

    let none_real = solve("solve x^2+1=0");
    assert_eq!(none_real.metadata["real_solutions"], "0");
    assert_eq!(none_real.metadata["nonreal_solutions"], "2");
    assert_eq!(
        none_real.domain_info.as_deref(),
        Some("0 real, 2 non-real (complex) solutions")
    );

    // x^3 - 1: one real root and two complex ones.
    let cubic = solve("solve x^3-1=0");
    assert_eq!(cubic.metadata["real_solutions"], "1");
    assert_eq!(cubic.metadata["nonreal_solutions"], "2");
    assert_eq!(
        cubic.domain_info.as_deref(),
        Some("1 real, 2 non-real (complex) solutions")
    );
}

#[test]
fn periodic_equations_are_marked_as_principal_only() {
    let r = solve("solve exp(x)=-1");
    assert_eq!(
        r.metadata.get("coverage").map(String::as_str),
        Some("principal")
    );
    assert!(
        r.domain_info.as_deref().unwrap().contains("principal"),
        "{:?}",
        r.domain_info
    );
    // A polynomial equation is complete.
    assert!(!solve("solve x^2-4=0").metadata.contains_key("coverage"));
    // ln is invertible on its domain: no note.
    assert!(!solve("solve ln(x)=-1").metadata.contains_key("coverage"));
}
