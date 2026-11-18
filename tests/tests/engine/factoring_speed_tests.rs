//! Factoring of large semiprimes and prime powers (Montgomery Pollard rho up to 127 bits).

use algebra_engine::numbertheory::{big_prime_factors, is_prime_big};
use num_bigint::BigUint;
use std::time::{Duration, Instant};

fn next_prime(mut n: BigUint) -> BigUint {
    n += 1u8;
    while !is_prime_big(&n) {
        n += 1u8;
    }
    n
}

fn prime_near(bits: u32, offset: u32) -> BigUint {
    next_prime((BigUint::from(1u8) << bits) + offset)
}

/// The factorisation must multiply back to `n` and consist of primes.
fn check(n: &BigUint, expected: &[(BigUint, u32)]) {
    let factors = big_prime_factors(n);
    let mut product = BigUint::from(1u8);
    for (p, e) in &factors {
        assert!(is_prime_big(p), "{p} is not prime");
        for _ in 0..*e {
            product *= p;
        }
    }
    assert_eq!(&product, n);
    let mut want: Vec<(BigUint, u32)> = expected.to_vec();
    want.sort();
    assert_eq!(factors, want);
}

#[test]
fn large_semiprimes_factor_quickly() {
    for bits in [40u32, 44, 48] {
        let (p, q) = (prime_near(bits, 0), prime_near(bits, 1000));
        let n = &p * &q;
        let start = Instant::now();
        check(&n, &[(p, 1), (q, 1)]);
        // A budget that the old BigUint iteration missed by a wide margin (an 89-bit product took
        // 7.5 s and a 97-bit one did not finish) and the machine-word one meets in a debug build.
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "{bits}x{bits} bits took {:?}",
            start.elapsed()
        );
    }
}

#[test]
fn prime_powers_and_mixed_products_are_split() {
    let p = prime_near(40, 0);
    let q = prime_near(36, 0);
    let r = prime_near(30, 0);
    // p^2: rho alone cannot split a prime square.
    check(&(&p * &p), &[(p.clone(), 2)]);
    // p^2 q and p q r, each beyond 64 bits.
    check(&(&p * &p * &q), &[(p.clone(), 2), (q.clone(), 1)]);
    check(
        &(&p * &q * &r),
        &[(p.clone(), 1), (q.clone(), 1), (r.clone(), 1)],
    );
    // Small cofactors are stripped first.
    let n = BigUint::from(6u8) * &p * &q;
    check(
        &n,
        &[
            (BigUint::from(2u8), 1),
            (BigUint::from(3u8), 1),
            (p, 1),
            (q, 1),
        ],
    );
}
