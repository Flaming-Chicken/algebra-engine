//! Recurrence guessing for definite sums and classic sequences (exact rational data).

use algebra_engine::guess::{guess_minimal_recurrence, guess_recurrence};
use num_bigint::BigInt;
use num_rational::BigRational;

fn q(n: i64) -> BigRational {
    BigRational::from_integer(BigInt::from(n))
}

fn binomial(n: i64, k: i64) -> BigInt {
    if k < 0 || k > n {
        return BigInt::from(0);
    }
    let mut acc = BigInt::from(1);
    for i in 0..k {
        acc = acc * BigInt::from(n - i) / BigInt::from(i + 1);
    }
    acc
}

fn sequence(len: i64, f: impl Fn(i64) -> BigInt) -> Vec<BigRational> {
    (0..len).map(|n| BigRational::from_integer(f(n))).collect()
}

/// Render coefficient i as a list of integers when they are all integers (for exact checks).
fn ints(rec: &algebra_engine::guess::Recurrence, i: usize, degree: usize) -> Vec<i64> {
    (0..=degree)
        .map(|j| {
            let c = rec.coefficients[i].coeff(j);
            assert!(c.is_integer(), "non-integer coefficient {c}");
            c.to_integer().try_into().unwrap()
        })
        .collect()
}

#[test]
fn classic_definite_sums_are_guessed() {
    // sum_k C(n, k) = 2^n: S(n+1) - 2 S(n) = 0.
    let s = sequence(14, |n| (0..=n).map(|k| binomial(n, k)).sum());
    let r = guess_minimal_recurrence(&s, 0, 3, 3).unwrap().unwrap();
    assert_eq!((r.order(), r.degree()), (1, 0));
    assert_eq!((ints(&r, 0, 0), ints(&r, 1, 0)), (vec![-2], vec![1]));

    // sum_k C(n, k)^2 = C(2n, n): (n + 1) S(n+1) - 2 (2n + 1) S(n) = 0, normalised to a monic
    // top coefficient: S(n+1) - 2(2n+1)/(n+1) S(n), i.e. (n+1) S(n+1) = (4n+2) S(n).
    let s = sequence(16, |n| {
        (0..=n).map(|k| binomial(n, k) * binomial(n, k)).sum()
    });
    let r = guess_minimal_recurrence(&s, 0, 3, 3).unwrap().unwrap();
    assert_eq!((r.order(), r.degree()), (1, 1));
    // Direct check against the textbook relation (not just the guess's own consistency).
    for n in 0..14i64 {
        let lhs = q(n + 1) * &s[(n + 1) as usize];
        let rhs = q(4 * n + 2) * &s[n as usize];
        assert_eq!(lhs, rhs);
    }
    // The guessed recurrence is proportional to (n+1) S(n+1) - (4n+2) S(n): c_0 : c_1 = -(4n+2) : (n+1).
    let (c0, c1) = (&r.coefficients[0], &r.coefficients[1]);
    for n in 0..10 {
        let at = q(n);
        assert_eq!(c0.eval(&at) * (q(n) + q(1)), -c1.eval(&at) * (q(4 * n + 2)));
    }

    // sum_k k C(n, k) = n 2^(n-1): n S(n+1) - 2 (n + 1) S(n) = 0.
    let s = sequence(16, |n| {
        (0..=n).map(|k| BigInt::from(k) * binomial(n, k)).sum()
    });
    let r = guess_minimal_recurrence(&s, 0, 3, 3).unwrap().unwrap();
    assert_eq!((r.order(), r.degree()), (1, 1));
    for n in 1..12 {
        assert!(r.holds_at(&s, 0, n));
    }

    // Fibonacci: F(n+2) - F(n+1) - F(n) = 0 (order 2, constant coefficients).
    let fib = |n: i64| {
        let (mut a, mut b) = (BigInt::from(0), BigInt::from(1));
        for _ in 0..n {
            (a, b) = (b.clone(), a + b);
        }
        a
    };
    let s = sequence(16, fib);
    let r = guess_minimal_recurrence(&s, 0, 3, 2).unwrap().unwrap();
    assert_eq!((r.order(), r.degree()), (2, 0));
    assert_eq!(
        (ints(&r, 0, 0), ints(&r, 1, 0), ints(&r, 2, 0)),
        (vec![-1], vec![-1], vec![1])
    );

    // Central Delannoy numbers D(n): (n+1) D(n+1) = 3 (2n+1) D(n) - n D(n-1) (order 2, degree 1).
    let delannoy =
        |n: i64| -> BigInt { (0..=n).map(|k| binomial(n, k) * binomial(n + k, k)).sum() };
    let s = sequence(20, delannoy);
    let r = guess_minimal_recurrence(&s, 0, 3, 2).unwrap().unwrap();
    assert_eq!(r.order(), 2);
    for n in 0..16 {
        assert!(r.holds_at(&s, 0, n), "Delannoy recurrence fails at n = {n}");
    }
}

#[test]
fn sequences_without_a_small_recurrence_and_short_data_are_reported() {
    // The primes have no holonomic recurrence of small order and degree.
    let primes: Vec<i64> = (2..200)
        .filter(|n| (2..*n).all(|d| n % d != 0))
        .take(30)
        .collect();
    let s: Vec<BigRational> = primes.iter().map(|&p| q(p)).collect();
    assert!(guess_minimal_recurrence(&s, 1, 2, 2).unwrap().is_none());

    // Too little data for the requested shape is an error, not a spurious fit.
    let short = sequence(5, |n| BigInt::from(2).pow(n as u32));
    assert!(guess_recurrence(&short, 0, 2, 2).is_err());
    assert!(guess_minimal_recurrence(&short[..2], 0, 1, 0).is_err());

    // An exact order is asked for: 2^n has an order-1 recurrence, and asking for order 2 with
    // degree 0 still returns a (non-minimal) relation that holds on the data.
    let pow2 = sequence(14, |n| BigInt::from(2).pow(n as u32));
    let r = guess_recurrence(&pow2, 0, 2, 0).unwrap().unwrap();
    for n in 0..10 {
        assert!(r.holds_at(&pow2, 0, n));
    }
    assert!(
        !r.holds_at(&pow2, 0, 100),
        "indices beyond the data are not claimed"
    );
}
