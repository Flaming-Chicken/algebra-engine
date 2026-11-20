use algebra_advanced::finite::{GaloisField, ModuloInt};
use num_bigint::BigInt;

#[test]
fn test_modulo_arithmetic() {
    let m1 = ModuloInt::new(BigInt::from(7), BigInt::from(5)); // 7 mod 5 = 2
    let m2 = ModuloInt::new(BigInt::from(4), BigInt::from(5)); // 4 mod 5 = 4

    let sum = m1.add(&m2).unwrap(); // (2 + 4) mod 5 = 1
    assert_eq!(sum.value, BigInt::from(1));

    let pow = m1.pow(&BigInt::from(3)); // 2^3 mod 5 = 8 mod 5 = 3
    assert_eq!(pow.value, BigInt::from(3));
}

#[test]
fn test_galois_field_cardinality() {
    let gf = GaloisField::new(2, 8); // GF(2^8) AES field
    assert_eq!(gf.cardinality(), 256);
}

#[test]
fn test_inert_primes_to_galois_fields() {
    // p = 3 is inert in Z[i] (3 = 3 mod 4) -> GF(3^2) with modulus x^2 + 1
    let gf3 = GaloisField::from_gaussian_inert_prime(3).expect("3 is inert in Z[i]");
    assert_eq!(gf3.p, 3);
    assert_eq!(gf3.degree, 2);
    assert_eq!(gf3.cardinality(), 9);
    assert_eq!(gf3.modulus_poly, Some(vec![1, 0, 1]));

    // p = 7 is inert in Z[i] (7 = 3 mod 4) -> GF(7^2) with modulus x^2 + 1
    let gf7 = GaloisField::from_gaussian_inert_prime(7).expect("7 is inert in Z[i]");
    assert_eq!(gf7.cardinality(), 49);

    // p = 5 splits in Z[i] (5 = 1 mod 4) -> should return None
    assert!(GaloisField::from_gaussian_inert_prime(5).is_none());

    // p = 2 is inert in Z[omega] (2 = 2 mod 3) -> GF(2^2) with modulus x^2 - x + 1
    let gf2_omega = GaloisField::from_eisenstein_inert_prime(2).expect("2 is inert in Z[omega]");
    assert_eq!(gf2_omega.p, 2);
    assert_eq!(gf2_omega.degree, 2);
    assert_eq!(gf2_omega.cardinality(), 4);
    assert_eq!(gf2_omega.modulus_poly, Some(vec![1, -1, 1]));

    // p = 5 is inert in Z[omega] (5 = 2 mod 3) -> GF(5^2)
    let gf5_omega = GaloisField::from_eisenstein_inert_prime(5).expect("5 is inert in Z[omega]");
    assert_eq!(gf5_omega.cardinality(), 25);

    // p = 7 splits in Z[omega] (7 = 1 mod 3) -> should return None
    assert!(GaloisField::from_eisenstein_inert_prime(7).is_none());
}

#[test]
fn modular_inverse_division_and_negative_powers() {
    let m = |v: i64, n: i64| ModuloInt::new(BigInt::from(v), BigInt::from(n));
    // Units of Z/15Z are exactly the residues coprime to 15.
    for a in 0..15i64 {
        let inv = m(a, 15).inverse();
        let coprime = (1..=a.max(1)).all(|d| !(a % d == 0 && 15 % d == 0 && d > 1)) && a != 0;
        assert_eq!(inv.is_some(), coprime, "a = {a}");
        if let Some(i) = inv {
            assert_eq!(i.mul(&m(a, 15)).unwrap().value, BigInt::from(1));
        }
    }
    assert_eq!(m(3, 7).div(&m(5, 7)).unwrap(), m(2, 7)); // 3 / 5 = 3 * 3 = 9 = 2
    assert!(m(3, 6).div(&m(2, 6)).is_none()); // 2 is not a unit mod 6
    assert_eq!(m(3, 7).checked_pow(&BigInt::from(-1)).unwrap(), m(5, 7));
    assert!(ModuloInt::checked_new(BigInt::from(3), BigInt::from(0)).is_none());
    assert!(ModuloInt::checked_new(BigInt::from(3), BigInt::from(1)).is_none());
    assert_eq!(m(2, 9).sub(&m(5, 9)).unwrap(), m(6, 9));
}

#[test]
fn irreducible_polynomial_counts_match_the_necklace_formula() {
    use algebra_advanced::finite::count_monic_irreducibles;
    // (1/n) sum_{d | n} mu(d) p^(n/d)
    let expected = |p: u64, n: u32| -> u64 {
        let mu = |d: u32| match d {
            1 => 1i64,
            2 | 3 | 5 | 7 => -1,
            4 | 8 | 9 => 0,
            6 | 10 => 1,
            _ => 0,
        };
        let sum: i64 = (1..=n)
            .filter(|d| n.is_multiple_of(*d))
            .map(|d| mu(d) * (p as i64).pow(n / d))
            .sum();
        (sum / n as i64) as u64
    };
    for (p, n) in [
        (2u64, 1u32),
        (2, 2),
        (2, 3),
        (2, 4),
        (2, 5),
        (2, 6),
        (3, 2),
        (3, 3),
        (3, 4),
        (5, 2),
        (5, 3),
    ] {
        assert_eq!(
            count_monic_irreducibles(p, n as usize),
            expected(p, n),
            "p={p} n={n}"
        );
    }
}

#[test]
fn finite_field_axioms_hold_exhaustively() {
    for (p, n) in [(2u32, 2u32), (2, 3), (3, 2), (2, 4), (5, 2), (3, 3)] {
        let field = GaloisField::new(p, n).arithmetic().unwrap();
        let q = field.order();
        let all = field.elements();
        let one = field.one();
        let zero = field.zero();
        let mut primitive = 0;
        for a in &all {
            // additive inverse
            assert_eq!(field.add(a, &field.neg(a)), zero);
            if *a != zero {
                let inv = field.inverse(a).unwrap();
                assert_eq!(field.mul(a, &inv), one, "GF({p}^{n})");
                // Lagrange: a^(q - 1) = 1, and count generators of the cyclic group.
                assert_eq!(field.pow(a, q - 1), one);
                let mut order = 1;
                let mut x = a.clone();
                while x != one {
                    x = field.mul(&x, a);
                    order += 1;
                }
                if order == q - 1 {
                    primitive += 1;
                }
            }
            for b in &all {
                // commutativity and Frobenius additivity (a + b)^p = a^p + b^p
                assert_eq!(field.mul(a, b), field.mul(b, a));
                assert_eq!(
                    field.frobenius(&field.add(a, b)),
                    field.add(&field.frobenius(a), &field.frobenius(b))
                );
            }
        }
        // The multiplicative group is cyclic: phi(q - 1) generators.
        let phi = (1..q).filter(|k| gcd(*k, q - 1) == 1).count();
        assert_eq!(primitive, phi, "GF({p}^{n}) primitive elements");
        // x^q = x for every element (the field is the splitting field of x^q - x).
        for a in &all {
            assert_eq!(field.pow(a, q), *a);
        }
    }
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

#[test]
fn gf4_multiplication_table_and_bad_inputs() {
    // GF(4) = F_2[x]/(x^2 + x + 1): x * x = x + 1, x * (x + 1) = 1.
    let field = GaloisField::with_modulus(2, 2, vec![1, 1, 1])
        .arithmetic()
        .unwrap();
    let x = field.element(&[0, 1]);
    let x1 = field.element(&[1, 1]);
    assert_eq!(field.mul(&x, &x), x1);
    assert_eq!(field.mul(&x, &x1), field.one());
    // A reducible modulus, a wrong degree, a composite characteristic and degree 0 are refused.
    assert!(
        GaloisField::with_modulus(2, 2, vec![1, 0, 1])
            .arithmetic()
            .is_none()
    ); // (x+1)^2
    assert!(
        GaloisField::with_modulus(2, 3, vec![1, 1, 1])
            .arithmetic()
            .is_none()
    );
    assert!(GaloisField::new(4, 2).arithmetic().is_none());
    assert!(GaloisField::new(3, 0).arithmetic().is_none());
    // The inert-prime constructors really give fields: x^2 + 1 is irreducible mod 7 but not mod 5.
    assert!(
        GaloisField::from_gaussian_inert_prime(7)
            .unwrap()
            .arithmetic()
            .is_some()
    );
    assert!(
        GaloisField::with_modulus(5, 2, vec![1, 0, 1])
            .arithmetic()
            .is_none()
    );
}
