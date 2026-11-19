//! Arbitrary-precision evaluation: published digits of the standard constants, and identities that hold to far more
//! digits than the working precision of an `f64` (the oracle is the identity, not the implementation).

use algebra_engine::session::UraeSession;

fn n(expression: &str, digits: usize) -> String {
    let r = UraeSession::new().execute_line(&format!("N({expression}, {digits})"));
    assert!(!r.is_error, "{expression}: {}", r.output_text);
    r.output_text
}

/// `|x|` below `10^-k` for a decimal string.
fn below(text: &str, k: usize) -> bool {
    let t = text.trim_start_matches('-');
    t == "0" || t.starts_with(&format!("0.{}", "0".repeat(k - 1)))
}

#[test]
fn standard_constants_to_fifty_digits() {
    assert_eq!(
        n("pi", 50),
        "3.1415926535897932384626433832795028841971693993751"
    );
    assert_eq!(
        n("e", 50),
        "2.7182818284590452353602874713526624977572470937000"
    );
    assert_eq!(
        n("sqrt(2)", 50),
        "1.4142135623730950488016887242096980785696718753769"
    );
    assert_eq!(n("ln(2)", 40), "0.6931471805599453094172321214581765680755");
    assert_eq!(
        n("atan(1)", 40),
        "0.7853981633974483096156608458198757210493"
    );
    assert_eq!(
        n("sin(1)", 40),
        "0.8414709848078965066525023216302989996226"
    );
    assert_eq!(
        n("cos(1)", 40),
        "0.5403023058681397174009366074429766037323"
    );
    assert_eq!(n("1/3", 30), "0.333333333333333333333333333333");
    // The golden ratio and the Euler-Mascheroni-free classics.
    assert_eq!(n("(1 + sqrt(5))/2", 30), "1.61803398874989484820458683437");
}

#[test]
fn identities_hold_to_a_hundred_digits() {
    // sin^2 + cos^2 = 1, exp(ln x) = x, tan = sin / cos, atan(tan(x)) = x on (-pi/2, pi/2), sqrt(x)^2 = x,
    // Machin-type: 4 atan(1) = pi, e^(i pi)-free: cosh^2 - sinh^2 = 1.
    for (expression, k) in [
        ("sin(1)^2 + cos(1)^2 - 1", 95),
        ("sin(12345/1000)^2 + cos(12345/1000)^2 - 1", 90),
        ("exp(ln(7)) - 7", 90),
        ("exp(ln(1/3)) - 1/3", 90),
        ("tan(7/10) - sin(7/10)/cos(7/10)", 90),
        ("atan(tan(1/2)) - 1/2", 90),
        ("sqrt(10)^2 - 10", 90),
        ("4*atan(1) - pi", 90),
        ("cosh(2)^2 - sinh(2)^2 - 1", 85),
        ("exp(1)*exp(-1) - 1", 90),
        ("sin(2*(3/10)) - 2*sin(3/10)*cos(3/10)", 90),
        ("ln(2^100) - 100*ln(2)", 85),
        ("sin(100)^2 + cos(100)^2 - 1", 85), // argument reduction over many periods
        ("asin(1/3) + acos(1/3) - pi/2", 85),
    ] {
        let value = n(expression, 100);
        assert!(below(&value, k), "{expression} = {value}");
    }
}

#[test]
fn digits_are_correctly_rounded_and_formatted() {
    assert_eq!(n("1/7", 12), "0.142857142857");
    assert_eq!(n("2/3", 5), "0.66667"); // rounded, not truncated
    assert_eq!(n("-2/3", 5), "-0.66667");
    assert_eq!(n("1000000/7", 12), "142857.142857");
    assert_eq!(n("exp(-10)", 10), "0.00004539992976"); // ten significant digits
    assert_eq!(n("99999/100000", 3), "1.00"); // a carry into a new leading digit
    // Large powers keep every requested digit.
    assert_eq!(n("2^100", 31), "1267650600228229401496703205376");
    // 100 digits of pi (published): ...
    assert!(n("pi", 100).starts_with("3.14159265358979323846264338327950288419716939937510582097494459230781640628620899862803482534211706"));
}

#[test]
fn what_cannot_be_evaluated_is_an_error_not_a_guess() {
    // A free symbol and a function with no arbitrary-precision implementation fall back to the f64 path (which says so).
    let r = UraeSession::new().execute_line("N(gamma(1/2), 40)");
    assert!(
        !r.output_text.starts_with("1.77245385090551602729816748334"),
        "should not claim 40 digits: {}",
        r.output_text
    );
}
