use algebra_advanced::surreal::SurrealNo;

#[test]
fn test_surreal_basic_constants_and_order() {
    let zero = SurrealNo::zero();
    let one = SurrealNo::one();
    let minus_one = SurrealNo::minus_one();
    let half = SurrealNo::half();

    assert_eq!(zero.birthday, 0);
    assert_eq!(one.birthday, 1);
    assert_eq!(minus_one.birthday, 1);
    assert_eq!(half.birthday, 2);

    assert!(minus_one.leq(&zero));
    assert!(zero.leq(&half));
    assert!(half.leq(&one));
    assert!(!one.leq(&zero));
}

#[test]
fn test_surreal_arithmetic() {
    let zero = SurrealNo::zero();
    let one = SurrealNo::one();
    let minus_one = SurrealNo::minus_one();

    // 1 + (-1) == 0
    let sum = one.add(&minus_one);
    assert!(sum.equiv(&zero));

    // 1 + 1 == 2
    let two = SurrealNo::from_int(2);
    let one_plus_one = one.add(&one);
    assert!(one_plus_one.equiv(&two));

    // 1 * 1 == 1
    let prod = one.mul(&one);
    assert!(prod.equiv(&one));
}

#[test]
fn test_surreal_transfinite_and_infinitesimal() {
    let zero = SurrealNo::zero();
    let one = SurrealNo::one();
    let omega = SurrealNo::omega(4);
    let epsilon = SurrealNo::epsilon(4);

    // 0 < epsilon < 1 < omega
    assert!(zero.leq(&epsilon));
    assert!(epsilon.leq(&one));
    assert!(one.leq(&omega));
}

// --- B16: exact values, no floating-point shortcut -----------------------------------------------

use num_rational::BigRational;

fn q(n: i64, d: i64) -> BigRational {
    BigRational::new(n.into(), d.into())
}

fn cut(left: &[&SurrealNo], right: &[&SurrealNo]) -> SurrealNo {
    SurrealNo::new(
        left.iter().map(|s| (*s).clone()).collect(),
        right.iter().map(|s| (*s).clone()).collect(),
    )
}

#[test]
fn a_surreal_is_the_simplest_number_between_its_options() {
    let (zero, one, half) = (SurrealNo::zero(), SurrealNo::one(), SurrealNo::half());
    let (two, three) = (SurrealNo::from_int(2), SurrealNo::from_int(3));
    let (m1, m3) = (SurrealNo::minus_one(), SurrealNo::from_int(-3));
    let check = |s: &SurrealNo, want: BigRational| assert_eq!(s.value(), Some(&want), "{s}"); // exact
    check(&cut(&[&zero], &[&three]), q(1, 1)); // {0 | 3} = 1, not 3/2
    check(&cut(&[&one], &[&three]), q(2, 1));
    check(&cut(&[&m3], &[&m1]), q(-2, 1));
    check(&cut(&[&half], &[&one]), q(3, 4));
    check(&cut(&[&two], &[]), q(3, 1));
    check(&cut(&[], &[&m3]), q(-4, 1));
    check(&cut(&[&m1], &[&one]), q(0, 1));
    check(&cut(&[&zero], &[&SurrealNo::half()]), q(1, 4));
    check(&SurrealNo::epsilon(4), q(1, 16)); // {0 | 1, 1/2, 1/4, 1/8} = 1/16
    check(&SurrealNo::omega(5), q(5, 1)); // a finite truncation of omega is the integer
    // Not a number: a left option that is not below a right option has no value.
    let game = cut(&[&one], &[&zero]);
    assert!(game.value().is_none());
}

#[test]
fn the_order_agrees_with_exact_values_where_the_old_float_shortcut_failed() {
    let (zero, one) = (SurrealNo::zero(), SurrealNo::one());
    let three = SurrealNo::from_int(3);
    let one_via_cut = cut(&[&zero], &[&three]); // = 1, with a midpoint "approximation" of 1.5
    let five_quarters = one.add(&SurrealNo::new(vec![zero.clone()], vec![SurrealNo::half()]));
    assert_eq!(five_quarters.value(), Some(&q(5, 4)));
    assert!(one_via_cut.leq(&five_quarters), "1 <= 5/4");
    assert!(!five_quarters.leq(&one_via_cut));
    assert!(one_via_cut.equiv(&one));
    assert_eq!(
        one_via_cut.partial_cmp(&five_quarters),
        Some(std::cmp::Ordering::Less)
    );
    // omega(3) is the integer 3: omega(3) <= 4 is TRUE (the audit's claim that this was wrong was itself wrong).
    assert!(SurrealNo::omega(3).leq(&SurrealNo::from_int(4)));
    assert!(!SurrealNo::omega(5).leq(&SurrealNo::from_int(4)));
    // A game that is not a number still compares by the recursive definition and never panics.
    let game = cut(&[&one], &[&zero]);
    let _ = game.leq(&zero);
    let _ = zero.leq(&game);
}

#[test]
fn arithmetic_matches_exact_dyadic_arithmetic() {
    let atoms: Vec<(SurrealNo, BigRational)> = vec![
        (SurrealNo::zero(), q(0, 1)),
        (SurrealNo::one(), q(1, 1)),
        (SurrealNo::minus_one(), q(-1, 1)),
        (SurrealNo::half(), q(1, 2)),
        (SurrealNo::from_int(2), q(2, 1)),
        (SurrealNo::from_int(-2), q(-2, 1)),
        (cut(&[&SurrealNo::zero()], &[&SurrealNo::half()]), q(1, 4)),
    ];
    for (x, vx) in &atoms {
        assert_eq!(x.value(), Some(vx));
        assert_eq!(x.neg().value(), Some(&-vx.clone()), "negation of {vx}");
        for (y, vy) in &atoms {
            assert_eq!(x.add(y).value(), Some(&(vx + vy)), "{vx} + {vy}");
            assert_eq!(x.sub(y).value(), Some(&(vx - vy)), "{vx} - {vy}");
            assert_eq!(x.leq(y), vx <= vy, "{vx} <= {vy}");
        }
    }
}

#[test]
fn multiplication_matches_exact_dyadic_multiplication() {
    let atoms: Vec<(SurrealNo, BigRational)> = vec![
        (SurrealNo::zero(), q(0, 1)),
        (SurrealNo::one(), q(1, 1)),
        (SurrealNo::minus_one(), q(-1, 1)),
        (SurrealNo::half(), q(1, 2)),
        (SurrealNo::from_int(2), q(2, 1)),
        (SurrealNo::from_int(-2), q(-2, 1)),
    ];
    for (x, vx) in &atoms {
        for (y, vy) in &atoms {
            let product = x.mul(y);
            assert_eq!(product.value(), Some(&(vx * vy)), "{vx} * {vy}");
        }
    }
}
