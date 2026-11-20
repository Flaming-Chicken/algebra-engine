use algebra_engine::session::UraeSession;
fn run(q: &str) -> String {
    let r = UraeSession::new().execute_line(q);
    r.output_text
}

#[test]
fn integer_functions_match_hand_values() {
    assert_eq!(run("totient(36)"), "totient(36) = 12");
    assert_eq!(run("totient 97"), "totient(97) = 96");
    assert_eq!(run("euler_phi(1)"), "totient(1) = 1");
    assert_eq!(run("nextprime(100)"), "next_prime(100) = 101");
    assert_eq!(run("next_prime(7)"), "next_prime(7) = 11");
    assert_eq!(run("prevprime(100)"), "prev_prime(100) = 97");
    assert_eq!(
        run("divisors(36)"),
        "divisors(36) = [1, 2, 3, 4, 6, 9, 12, 18, 36]"
    );
    assert_eq!(run("sigma(28)"), "sigma(28) = 56"); // 28 is perfect
    assert_eq!(run("tau(36)"), "tau(36) = 9");
    assert_eq!(run("mobius(30)"), "mobius(30) = -1");
    assert_eq!(run("mobius(12)"), "mobius(12) = 0");
    assert_eq!(run("mobius(1)"), "mobius(1) = 1");
    assert_eq!(run("mod_inverse(3, 7)"), "mod_inverse(3, 7) = 5");
    assert_eq!(run("pow_mod(2, 10, 1000)"), "pow_mod(2, 10, 1000) = 24");
    assert_eq!(run("pow_mod(3, 0, 7)"), "pow_mod(3, 0, 7) = 1");
    assert_eq!(run("primitive_root(7)"), "primitive_root(7) = 3");
}

#[test]
fn integer_functions_report_errors() {
    for q in [
        "mod_inverse(2, 6)",
        "totient(0)",
        "totient(-5)",
        "primitive_root(8)",
        "pow_mod(2, 3)",
        "divisors(0)",
    ] {
        let r = UraeSession::new().execute_line(q);
        assert!(r.is_error, "{q}: {}", r.output_text);
    }
}

/// Independent oracles by brute force, not by the same factorisation code.
#[test]
fn totient_sigma_and_mobius_agree_with_brute_force() {
    fn gcd(a: u64, b: u64) -> u64 {
        if b == 0 { a } else { gcd(b, a % b) }
    }
    let mut s = UraeSession::new();
    for n in 1..=120u64 {
        let phi = (1..=n).filter(|&k| gcd(k, n) == 1).count();
        assert_eq!(
            s.execute_line(&format!("totient({n})")).output_text,
            format!("totient({n}) = {phi}")
        );
        let sigma: u64 = (1..=n).filter(|d| n % d == 0).sum();
        assert_eq!(
            s.execute_line(&format!("sigma({n})")).output_text,
            format!("sigma({n}) = {sigma}")
        );
        let tau = (1..=n).filter(|d| n % d == 0).count();
        assert_eq!(
            s.execute_line(&format!("tau({n})")).output_text,
            format!("tau({n}) = {tau}")
        );
    }
    // Gauss: the totients of the divisors of n sum to n.
    for n in [12u64, 30, 36, 60, 97, 100] {
        let total: u64 = (1..=n)
            .filter(|d| n % d == 0)
            .map(|d| (1..=d).filter(|&k| gcd(k, d) == 1).count() as u64)
            .sum();
        assert_eq!(total, n);
    }
}
