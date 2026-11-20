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

#[test]
fn a_product_prints_the_same_whatever_was_typed_before() {
    // The stored factor order follows the order in which symbols were first seen, which made the same expression
    // print differently depending on the session's history. Printing is now in a fixed order: numbers, then
    // symbols and their powers by name, then function calls by name.
    let primed_forwards = {
        let mut s = UraeSession::new();
        s.execute_line("a + b + c");
        s.execute_line("simplify c*sin(x)*b*a*3*x^2")
    };
    let primed_backwards = {
        let mut s = UraeSession::new();
        s.execute_line("x + sin(x) + c + b + a");
        s.execute_line("simplify c*sin(x)*b*a*3*x^2")
    };
    assert_eq!(primed_forwards.output_text, primed_backwards.output_text);
    assert_eq!(primed_forwards.output_text, "3 · a · b · c · x^2 · sin(x)");
    // Matrix products do not commute and keep their order.
    let mut s = UraeSession::new();
    let ab = s.execute_line("simplify [[0,1],[0,0]]*[[0,0],[1,0]]");
    let ba = s.execute_line("simplify [[0,0],[1,0]]*[[0,1],[0,0]]");
    assert_ne!(ab.output_text, ba.output_text);
}

#[test]
fn a_product_of_matrix_literals_is_a_matrix_product() {
    // Oracle: the product computed by hand. [[1,2],[3,4]] * [[0,1],[1,0]] = [[2,1],[4,3]].
    let product = run("[[1,2],[3,4]]*[[0,1],[1,0]]");
    assert_eq!(product, run("matmul [[1,2],[3,4]], [[0,1],[1,0]]"));
    assert!(product.contains('2') && product.contains('4'), "{product}");
    // Multiplying by the identity returns the matrix.
    assert_eq!(run("[[1,2],[3,4]]*[[1,0],[0,1]]"), "[ 1, 2 ; 3, 4 ]");
    assert_eq!(product, "[ 2, 1 ; 4, 3 ]");
    // Not commutative: the order matters.
    assert_ne!(product, run("[[0,1],[1,0]]*[[1,2],[3,4]]"));
}

#[test]
fn a_bare_matrix_literal_prints_as_a_grid() {
    assert_eq!(run("[[1,2],[3,4]]"), "[ 1, 2 ; 3, 4 ]");
    assert_eq!(
        run("[[1,0,0],[0,1,0],[0,0,1]]"),
        "[ 1, 0, 0 ; 0, 1, 0 ; 0, 0, 1 ]"
    );
    // Symbolic entries survive.
    let m = run("[[a,b],[c,d]]");
    assert!(
        m.starts_with("[ ") && m.contains(" ; ") && m.contains('a') && m.contains('d'),
        "{m}"
    );
    // A row vector prints as a one-row grid.
    assert!(!run("[1,2,3]").contains("matrix(row"));
}

#[test]
fn bracket_groups_that_are_not_one_literal_are_left_alone() {
    // `[1,2]+[3,4]` starts with `[` and ends with `]` but is a sum of two literals, not one literal: it must not
    // be shown as the grid of its first group.
    let r = UraeSession::new().execute_line("[1,2]+[3,4]");
    assert!(!r.is_error, "{}", r.output_text);
    assert_ne!(r.output_text, "[ 1, 2 ]");
}
