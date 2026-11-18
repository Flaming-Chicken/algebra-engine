//! Gosper's algorithm: closed-form indefinite sums of hypergeometric terms, verified by checking
//! `z(k+1) - z(k) = t(k)` exactly with rational arithmetic.

use algebra_engine::gosper::{GosperSum, QPoly, gosper};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};

fn q(n: i64) -> BigRational {
    BigRational::from_integer(BigInt::from(n))
}

/// The terms t(k0..k0+len) of the hypergeometric sequence with t(k0) = 1 and the given ratio.
fn terms(num: &QPoly, den: &QPoly, k0: i64, len: usize) -> Vec<BigRational> {
    let mut t = vec![BigRational::one()];
    for i in 0..len {
        let k = q(k0 + i as i64);
        t.push(t[i].clone() * num.eval(&k) / den.eval(&k));
    }
    t
}

/// Assert that `sum` is an antidifference of the term with this ratio over k0..k0+len.
fn assert_antidifference(num: &QPoly, den: &QPoly, sum: &GosperSum, k0: i64, len: usize) {
    let t = terms(num, den, k0, len);
    for i in 0..len {
        let (k, k1) = (q(k0 + i as i64), q(k0 + i as i64 + 1));
        let z = |k: &BigRational, term: &BigRational| sum.eval(k).expect("pole of R") * term;
        let diff = z(&k1, &t[i + 1]) - z(&k, &t[i]);
        assert_eq!(diff, t[i], "z(k+1) - z(k) != t(k) at k = {}", k0 + i as i64);
    }
}

#[test]
fn sums_with_known_closed_forms_are_found() {
    // t(k) = 2^k: ratio 2, antidifference 2^k (R = 1).
    let (num, den) = (QPoly::from_ints(&[2]), QPoly::one());
    let s = gosper(&num, &den).unwrap().expect("geometric");
    assert_antidifference(&num, &den, &s, 0, 8);
    assert_eq!(s.eval(&q(5)).unwrap(), q(1));

    // t(k) = k 2^k: ratio 2(k+1)/k; z(k) = (k - 2) 2^k = R(k) t(k), so R(k) = (k - 2) / k.
    let (num, den) = (QPoly::from_ints(&[2, 2]), QPoly::from_ints(&[0, 1]));
    let s = gosper(&num, &den).unwrap().expect("k 2^k");
    assert_antidifference(&num, &den, &s, 1, 8);
    for k in 1..6 {
        assert_eq!(
            s.eval(&q(k)).unwrap(),
            BigRational::new((k - 2).into(), BigInt::from(k)),
            "R({k})"
        );
    }

    // t(k) = k k!: ratio (k+1)^2 / k; z(k) = k!, so R(k) = 1/k and sum_{j<n} j j! = n! - 1.
    let (num, den) = (QPoly::from_ints(&[1, 2, 1]), QPoly::from_ints(&[0, 1]));
    let s = gosper(&num, &den).unwrap().expect("k k!");
    assert_antidifference(&num, &den, &s, 1, 8);
    for k in 1..6 {
        assert_eq!(
            s.eval(&q(k)).unwrap(),
            BigRational::new(1.into(), BigInt::from(k)),
            "R({k})"
        );
    }

    // t(k) = 1 / (k (k+1)): ratio k / (k+2); z(k) = -1/k, so R(k) = -(k+1) (a telescoping sum).
    let (num, den) = (QPoly::from_ints(&[0, 1]), QPoly::from_ints(&[2, 1]));
    let s = gosper(&num, &den).unwrap().expect("1/(k(k+1))");
    assert_antidifference(&num, &den, &s, 1, 8);

    // t(k) = k^2 (a polynomial: ratio (k+1)^2 / k^2), antidifference a cubic.
    let (num, den) = (QPoly::from_ints(&[1, 2, 1]), QPoly::from_ints(&[0, 0, 1]));
    let s = gosper(&num, &den).unwrap().expect("k^2");
    assert_antidifference(&num, &den, &s, 1, 8);
    // sum_{j=1}^{n} j^2 = z(n+1) - z(1) with t(1) = 1 and t(k) = k^2.
    let z = |k: i64| s.eval(&q(k)).unwrap() * q(k * k);
    for n in 1..=8i64 {
        assert_eq!(
            z(n + 1) - z(1),
            q(n * (n + 1) * (2 * n + 1) / 6),
            "sum of squares to {n}"
        );
    }
}

#[test]
fn sums_without_a_closed_form_are_rejected() {
    // The harmonic numbers: t(k) = 1/k, ratio k/(k+1).
    assert!(
        gosper(&QPoly::from_ints(&[0, 1]), &QPoly::from_ints(&[1, 1]))
            .unwrap()
            .is_none()
    );
    // Partial sums of a binomial row: t(k) = C(5, k), ratio (5 - k)/(k + 1). The full sum is 2^5,
    // but there is no antidifference; that needs Zeilberger's creative telescoping.
    assert!(
        gosper(&QPoly::from_ints(&[5, -1]), &QPoly::from_ints(&[1, 1]))
            .unwrap()
            .is_none()
    );
    // t(k) = 1/k^2.
    assert!(
        gosper(&QPoly::from_ints(&[0, 0, 1]), &QPoly::from_ints(&[1, 2, 1]))
            .unwrap()
            .is_none()
    );
    // t(k) = k!: ratio k + 1 (the sum of factorials is not closed-form).
    assert!(
        gosper(&QPoly::from_ints(&[1, 1]), &QPoly::one())
            .unwrap()
            .is_none()
    );
}

#[test]
fn degenerate_ratios_and_arithmetic() {
    assert!(gosper(&QPoly::one(), &QPoly::zero()).is_err());
    // A term that vanishes after one step telescopes trivially.
    let s = gosper(&QPoly::zero(), &QPoly::one()).unwrap().unwrap();
    assert_eq!(s.eval(&q(3)).unwrap(), q(1));

    // Polynomial arithmetic: division with remainder, gcd, shift.
    let (a, b) = (QPoly::from_ints(&[-1, 0, 1]), QPoly::from_ints(&[1, 1])); // k^2-1, k+1
    let (quot, rem) = a.div_rem(&b).unwrap();
    assert_eq!(quot, QPoly::from_ints(&[-1, 1]));
    assert!(rem.is_zero());
    assert_eq!(a.gcd(&b), QPoly::from_ints(&[1, 1]));
    assert_eq!(
        QPoly::from_ints(&[0, 0, 1]).shift(&q(1)),
        QPoly::from_ints(&[1, 2, 1])
    );
    assert_eq!(a.eval(&q(3)), q(8));
    assert!(QPoly::one().div_rem(&QPoly::zero()).is_none());
}

#[test]
fn every_polynomial_times_geometric_term_is_summable() {
    // t(k) = p(k) r^k has ratio r p(k+1) / p(k) and is always Gosper-summable (the antidifference
    // is q(k) r^k with deg q = deg p, or deg p + 1 when r = 1). Check the identity exactly.
    let polys: [&[i64]; 6] = [
        &[0, 1],
        &[1, 1],
        &[0, 0, 1],
        &[2, 0, 1],
        &[0, 1, 1, 1],
        &[-1, 3, 0, 2],
    ];
    let ratios = [
        BigRational::new(2.into(), 1.into()),
        BigRational::new(3.into(), 1.into()),
        BigRational::new(1.into(), 2.into()),
        BigRational::new((-1).into(), 1.into()),
        BigRational::new(5.into(), 3.into()),
        BigRational::one(),
    ];
    for coeffs in polys {
        let p = QPoly::from_ints(coeffs);
        let shifted = p.shift(&q(1));
        for r in &ratios {
            let (num, den) = (shifted.scale(r), p.clone());
            // Start where p is non-zero and the ratio is defined.
            let k0 = 4;
            if den.eval(&q(k0)).is_zero() {
                continue;
            }
            let sum = gosper(&num, &den)
                .unwrap()
                .unwrap_or_else(|| panic!("p = {coeffs:?}, r = {r} must be summable"));
            assert_antidifference(&num, &den, &sum, k0, 7);
        }
    }
}
