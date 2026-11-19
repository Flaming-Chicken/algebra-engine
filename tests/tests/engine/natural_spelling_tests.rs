//! The call spellings people reach for first must evaluate, not echo the input back: `series(f, x, 0, 5)`,
//! `isprime(97)`, `factorint(360)`, `inverse(M)`, `limit(f, x, 0)`, `17 mod 5` (they were parsed as expressions and
//! returned unevaluated).

use algebra_engine::session::UraeSession;

fn run(q: &str) -> String {
    let r = UraeSession::new().execute_line(q);
    assert!(!r.is_error, "{q}: {}", r.output_text);
    r.output_text
}

#[test]
fn call_spellings_evaluate_like_the_command_spellings() {
    // Series: the same text as the command form, and the known coefficients.
    assert_eq!(
        run("series(exp(x), x, 0, 5)"),
        run("series exp(x), x, 0, 5")
    );
    assert_eq!(
        run("taylor(sin(x), x, 0, 7)"),
        "x - 1/6 · x^3 + 1/120 · x^5 - 1/5040 · x^7"
    );
    // Primality and factorisation.
    assert_eq!(run("is_prime(97)"), "is_prime(97) = true");
    assert_eq!(run("isprime(91)"), "is_prime(91) = false"); // 7 * 13
    assert!(run("factorint(360)").starts_with("360 = 2^3 * 3^2 * 5"));
    // Linear algebra.
    assert_eq!(run("inverse([[1,2],[3,4]])"), "[ -2, 1 ; 3/2, -1/2 ]");
    assert_eq!(run("det([[1,2],[3,4]])"), "-2");
    assert_eq!(run("rank([[1,2],[2,4]])"), "1");
    // Calculus.
    assert!(run("limit(sin(x)/x, x, 0)").ends_with("= 1"));
    assert_eq!(run("apart(1/(x^2-1))"), "1/2 / (x - 1) - 1/2 / (x + 1)");
    // The factor order inside each term varies from run to run, so check the content.
    let ode = run("dsolve(y'' + y = 0)");
    assert!(ode.starts_with("y(x) = "), "{ode}");
    for piece in ["cos(x)", "sin(x)", "C1", "C2"] {
        assert!(ode.contains(piece), "{ode}");
    }
}

#[test]
fn infix_mod_uses_the_non_negative_remainder() {
    assert_eq!(run("17 mod 5"), "2");
    assert_eq!(run("-3 mod 5"), "2"); // the remainder is non-negative, as in number theory
    assert_eq!(run("100 mod 7"), "2");
}

#[test]
fn a_call_is_only_rewritten_when_it_is_the_whole_input() {
    // `det(A) + 1` is an expression, not the command `det`.
    let r = UraeSession::new().execute_line("series(x, x, 0, 2) + 1");
    assert!(!r.output_text.starts_with("x +"), "{}", r.output_text);
    // Unknown function calls stay expressions.
    assert_eq!(run("f(2)"), "f(2)");
}
