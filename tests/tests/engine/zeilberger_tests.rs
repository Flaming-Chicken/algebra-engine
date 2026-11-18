//! Creative telescoping by specialisation and interpolation, checked against brute-force sums at
//! values of n far beyond the samples used for fitting.

use algebra_engine::gosper::QPoly;
use algebra_engine::zeilberger::{HypergeometricTerm, telescoping_at, zeilberger};
use num_bigint::BigInt;
use num_rational::BigRational;

fn q(n: i64) -> BigRational {
    BigRational::from_integer(BigInt::from(n))
}

/// `n + c` as a rational.
fn np(n: &BigRational, c: i64) -> BigRational {
    n + q(c)
}

fn poly(c: &[BigRational]) -> QPoly {
    QPoly::new(c.to_vec())
}

fn binomial(n: i64, k: i64) -> BigInt {
    if k < 0 || k > n {
        return BigInt::from(0);
    }
    (0..k).fold(BigInt::from(1), |acc, i| {
        acc * BigInt::from(n - i) / BigInt::from(i + 1)
    })
}

fn sum_binomial(n: i64) -> BigRational {
    BigRational::from_integer((0..=n).map(|k| binomial(n, k)).sum())
}

fn sum_squares(n: i64) -> BigRational {
    BigRational::from_integer((0..=n).map(|k| binomial(n, k) * binomial(n, k)).sum())
}

fn delannoy(n: i64) -> BigRational {
    BigRational::from_integer((0..=n).map(|k| binomial(n, k) * binomial(n + k, k)).sum())
}

/// C(n, k): ratios (n - k) / (k + 1) in k and (n + 1) / (n + 1 - k) in n.
fn binomial_term() -> HypergeometricTerm {
    HypergeometricTerm {
        ratio_k: Box::new(|n| (poly(&[n.clone(), q(-1)]), poly(&[q(1), q(1)]))),
        ratio_n: Box::new(|n| (poly(&[np(n, 1)]), poly(&[np(n, 1), q(-1)]))),
    }
}

/// C(n, k)^2.
fn squared_binomial_term() -> HypergeometricTerm {
    HypergeometricTerm {
        ratio_k: Box::new(|n| {
            let (p, d) = (poly(&[n.clone(), q(-1)]), poly(&[q(1), q(1)]));
            (p.mul(&p), d.mul(&d))
        }),
        ratio_n: Box::new(|n| {
            let (p, d) = (poly(&[np(n, 1)]), poly(&[np(n, 1), q(-1)]));
            (p.mul(&p), d.mul(&d))
        }),
    }
}

/// C(n, k) C(n + k, k) (central Delannoy numbers).
fn delannoy_term() -> HypergeometricTerm {
    HypergeometricTerm {
        // (n - k)(n + k + 1) / (k + 1)^2
        ratio_k: Box::new(|n| {
            let a = poly(&[n.clone(), q(-1)]);
            let b = poly(&[np(n, 1), q(1)]);
            let d = poly(&[q(1), q(1)]);
            (a.mul(&b), d.mul(&d))
        }),
        // (n + k + 1) / (n + 1 - k)
        ratio_n: Box::new(|n| (poly(&[np(n, 1), q(1)]), poly(&[np(n, 1), q(-1)]))),
    }
}

fn check_against(
    result: &algebra_engine::zeilberger::ZeilbergerResult,
    sums: impl Fn(i64) -> BigRational,
    upto: i64,
) {
    let r = &result.recurrence;
    let values: Vec<BigRational> = (0..=upto + r.order() as i64).map(&sums).collect();
    for n in 0..=upto {
        assert!(r.holds_at(&values, 0, n), "the recurrence fails at n = {n}");
    }
}

#[test]
fn the_binomial_row_sum_gets_its_recurrence() {
    let samples: Vec<i64> = (2..10).collect();
    let r = zeilberger(&binomial_term(), &samples, 1, 0)
        .unwrap()
        .expect("order 1");
    // S(n+1) - 2 S(n) = 0 (S(n) = 2^n).
    assert_eq!(r.recurrence.coefficients[0], QPoly::from_ints(&[-2]));
    assert_eq!(r.recurrence.coefficients[1], QPoly::from_ints(&[1]));
    assert_eq!(r.certified_at, samples);
    check_against(&r, sum_binomial, 25);
}

#[test]
fn sums_of_squared_binomials_and_delannoy_numbers() {
    // sum C(n,k)^2 = C(2n, n): (n + 1) S(n+1) - (4n + 2) S(n) = 0, order 1 and degree 1.
    let samples: Vec<i64> = (2..12).collect();
    let r = zeilberger(&squared_binomial_term(), &samples, 1, 1)
        .unwrap()
        .expect("order 1, degree 1");
    assert_eq!(r.recurrence.coefficients[1], QPoly::from_ints(&[1, 1]));
    assert_eq!(r.recurrence.coefficients[0], QPoly::from_ints(&[-2, -4]));
    check_against(&r, sum_squares, 25);
    // Degree 0 is not enough for this sum: no relation of that shape.
    assert!(
        zeilberger(&squared_binomial_term(), &samples, 1, 0)
            .unwrap()
            .is_none()
    );

    // Central Delannoy numbers need order 2 (and degree 1).
    let samples: Vec<i64> = (2..14).collect();
    assert!(
        zeilberger(&delannoy_term(), &samples, 1, 2)
            .unwrap()
            .is_none(),
        "no order-1 relation"
    );
    let r = zeilberger(&delannoy_term(), &samples, 2, 1)
        .unwrap()
        .expect("order 2, degree 1");
    assert_eq!(r.recurrence.order(), 2);
    check_against(&r, delannoy, 25);
}

#[test]
fn the_telescoping_solution_at_one_n_and_error_cases() {
    // At n = 5 the binomial term has a one-dimensional solution space for order 1:
    // S(6) - 2 S(5) = 0 up to scale.
    let space = telescoping_at(&binomial_term(), 5, 1).unwrap();
    assert_eq!(space.len(), 1);
    // (The basis is normalised to start with 1: compare the ratio c_0 / c_1.)
    assert_eq!(space[0][0].clone() / space[0][1].clone(), q(-2));
    // Too few samples for the unknowns is an error, not a guess.
    assert!(zeilberger(&binomial_term(), &[3, 4], 2, 2).is_err());
}
