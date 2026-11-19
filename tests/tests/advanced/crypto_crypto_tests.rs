use algebra_advanced::crypto::{EllipticCurve, EllipticPoint, InformationTheory, ReedSolomonCode};
use algebra_core::{ExprGraph, ExprKind};
use num_bigint::BigInt;

#[test]
fn test_elliptic_curve_validation() {
    // Curve: y^2 = x^3 + 2x + 3 (mod 97)
    let a = BigInt::from(2);
    let b = BigInt::from(3);
    let p = BigInt::from(97);

    let curve = EllipticCurve::new(a, b, p).unwrap();

    // Check point (3, 6): 6^2 = 36. 3^3 + 2*3 + 3 = 27 + 6 + 3 = 36.
    let point = EllipticPoint::Point {
        x: BigInt::from(3),
        y: BigInt::from(6),
    };

    assert!(curve.contains_point(&point));
    assert!(curve.contains_point(&EllipticPoint::IdentityAtInfinity));
}

#[test]
fn test_reed_solomon() {
    let rs = ReedSolomonCode::new(255, 223).unwrap();
    assert_eq!(rs.parity_symbols(), 32);
}

#[test]
fn test_information_entropy() {
    let graph = ExprGraph::new();
    let p1 = graph.symbol("p1");
    let p2 = graph.symbol("p2");

    let info = InformationTheory;
    let entropy = info.entropy(&graph, &[p1, p2]);

    let node = graph.get(entropy);
    if let ExprKind::Mul(factors) = &node.kind {
        assert_eq!(factors.len(), 2);
    } else {
        panic!("Expected Mul node for entropy");
    }
}

#[test]
fn test_elliptic_curve_reduction_and_frobenius() {
    use algebra_advanced::crypto::{RationalEllipticCurve, ReductionType};

    // Consider the curve E: y^2 = x^3 - x (Gauss curve)
    // a = -1, b = 0.
    // Discriminant Delta = -16 * (4*(-1)^3 + 27*0) = -16 * (-4) = 64 = 2^6.
    let curve = RationalEllipticCurve::new(-1, 0).unwrap();
    assert_eq!(curve.discriminant(), 64);
    assert_eq!(
        curve.bad_reduction_places(),
        vec![num_bigint::BigUint::from(2u32)]
    );

    // Bad reduction at p = 2:
    // c_4 = -48 * (-1) = 48 (divisible by 2) -> Additive reduction
    assert_eq!(curve.reduction_at(2), ReductionType::Additive);

    // Good reduction at all odd primes p >= 3:
    assert_eq!(curve.reduction_at(3), ReductionType::Good);
    assert_eq!(curve.reduction_at(5), ReductionType::Good);
    assert_eq!(curve.reduction_at(7), ReductionType::Good);

    // Frobenius traces a_p = p + 1 - #E(F_p):
    // Over F_3: x = 0 -> 0; x = 1 -> 0; x = 2 -> 2^3 - 2 = 6 = 0 mod 3.
    // Points: (0,0), (1,0), (2,0) + O -> 4 points.
    // a_3 = 3 + 1 - 4 = 0.
    assert_eq!(curve.frobenius_trace(3), Some(0));

    // Over F_5:
    // x = 0 -> 0 (1 pt)
    // x = 1 -> 0 (1 pt)
    // x = 2 -> 8 - 2 = 6 = 1 mod 5 (2 pts: y^2 = 1 -> y = 1, 4)
    // x = 3 -> 27 - 3 = 24 = 4 mod 5 (2 pts: y^2 = 4 -> y = 2, 3)
    // x = 4 -> 64 - 4 = 60 = 0 mod 5 (1 pt)
    // Infinity (1 pt)
    // Total #E(F_5) = 1 + 1 + 2 + 2 + 1 + 1 = 8 points.
    // a_5 = 5 + 1 - 8 = -2.
    assert_eq!(curve.frobenius_trace(5), Some(-2));

    // Over F_7:
    // Gauss curve has CM by Z[i]. For p = 3 mod 4, a_p = 0 always!
    assert_eq!(curve.frobenius_trace(7), Some(0));

    // Reduction mod p yields valid EllipticCurve
    let e5 = curve.reduce_mod_p(5).unwrap();
    assert_eq!(e5.p, num_bigint::BigInt::from(5));
}

/// Brute-force count of the points of `y^2 = x^3 + ax + b` over `F_p` (affine solutions plus infinity): the oracle.
fn brute_force_point_count(a: i64, b: i64, p: i64) -> i64 {
    let mut count = 1;
    for x in 0..p {
        for y in 0..p {
            if (y * y - (x * x % p * x + a * x + b)).rem_euclid(p) == 0 {
                count += 1;
            }
        }
    }
    count
}

#[test]
fn frobenius_trace_matches_brute_force_point_counts() {
    use algebra_advanced::crypto::RationalEllipticCurve;
    for a in -6..=6 {
        for b in -6..=6 {
            let Ok(curve) = RationalEllipticCurve::new(a, b) else {
                continue;
            };
            for p in [5i64, 7, 11, 13, 17, 19, 23] {
                let want = p + 1 - brute_force_point_count(a, b, p);
                assert_eq!(
                    curve.frobenius_trace(p as u64),
                    Some(want),
                    "a={a} b={b} p={p}"
                );
            }
        }
    }
}

#[test]
fn split_and_non_split_multiplicative_reduction_follow_the_trace() {
    use algebra_advanced::crypto::{RationalEllipticCurve, ReductionType};
    // At a prime of multiplicative reduction the Frobenius trace is +1 for split and -1 for non-split reduction.
    let mut checked = 0;
    for a in -40..=40i64 {
        for b in -40..=40i64 {
            let Ok(curve) = RationalEllipticCurve::new(a, b) else {
                continue;
            };
            for p in [5u64, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
                if let ReductionType::Multiplicative { split } = curve.reduction_at(p) {
                    let trace = curve.frobenius_trace(p).unwrap();
                    assert_eq!(trace, if split { 1 } else { -1 }, "a={a} b={b} p={p}");
                    checked += 1;
                }
            }
        }
    }
    assert!(
        checked > 20,
        "only {checked} multiplicative cases were exercised"
    );
}

#[test]
fn large_coefficients_are_refused_or_handled_without_overflow() {
    use algebra_advanced::crypto::RationalEllipticCurve;
    // 4a^3 needs about 190 bits here: an error, not a panic or a wrapped value.
    assert!(RationalEllipticCurve::new(i64::MAX, 1).is_err());
    // A discriminant beyond 64 bits is factored in full: y^2 = x^3 + 2^20 x + 3 has Delta = -16 (4 * 2^60 + 243).
    let big = RationalEllipticCurve::new(1 << 20, 3).unwrap();
    let product = big
        .bad_reduction_places()
        .iter()
        .fold(num_bigint::BigUint::from(1u32), |acc, p| acc * p);
    assert!(product > num_bigint::BigUint::from(1u32));
    let disc = big.discriminant().unsigned_abs();
    for p in big.bad_reduction_places() {
        assert!((num_bigint::BigUint::from(disc) % &p) == num_bigint::BigUint::from(0u32));
    }
}

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn byte(&mut self) -> u8 {
        (self.next() & 0xff) as u8
    }
}

#[test]
fn reed_solomon_codewords_have_zero_syndromes_and_correct_errors() {
    use algebra_advanced::crypto::ReedSolomonCode;
    let mut rng = Lcg(7);
    for (n, k) in [(255usize, 223usize), (31, 21), (15, 9), (7, 3), (10, 8)] {
        let code = ReedSolomonCode::new(n, k).unwrap();
        let t = code.correctable_errors();
        assert_eq!(t, (n - k) / 2);
        for trial in 0..20 {
            let message: Vec<u8> = (0..k).map(|_| rng.byte()).collect();
            let word = code.encode(&message).unwrap();
            assert_eq!(&word[..k], &message[..], "systematic");
            assert!(
                code.syndromes(&word).unwrap().iter().all(|&s| s == 0),
                "RS({n},{k}) trial {trial}"
            );
            // Corrupt exactly t distinct positions with non-zero error values.
            let mut received = word.clone();
            let mut hit = std::collections::BTreeSet::new();
            while hit.len() < t {
                let pos = (rng.next() as usize) % n;
                if hit.insert(pos) {
                    let mut e = rng.byte();
                    if e == 0 {
                        e = 1;
                    }
                    received[pos] ^= e;
                }
            }
            assert_eq!(
                code.decode(&received).unwrap(),
                message,
                "RS({n},{k}) with {t} errors, trial {trial}"
            );
        }
    }
}

#[test]
fn reed_solomon_is_linear_and_maximum_distance_separable() {
    use algebra_advanced::crypto::ReedSolomonCode;
    let code = ReedSolomonCode::new(7, 3).unwrap();
    assert_eq!(code.minimum_distance(), 5);
    let mut rng = Lcg(99);
    for _ in 0..300 {
        let a: Vec<u8> = (0..3).map(|_| rng.byte()).collect();
        let b: Vec<u8> = (0..3).map(|_| rng.byte()).collect();
        let (ca, cb) = (code.encode(&a).unwrap(), code.encode(&b).unwrap());
        // Linearity over GF(2^8) (addition is XOR).
        let sum: Vec<u8> = a.iter().zip(&b).map(|(x, y)| x ^ y).collect();
        let csum: Vec<u8> = ca.iter().zip(&cb).map(|(x, y)| x ^ y).collect();
        assert_eq!(code.encode(&sum).unwrap(), csum);
        // Distinct codewords differ in at least n - k + 1 positions.
        if a != b {
            let distance = ca.iter().zip(&cb).filter(|(x, y)| x != y).count();
            assert!(
                distance >= 5,
                "distance {distance} below the Singleton bound"
            );
        }
    }
}

#[test]
fn reed_solomon_reports_what_it_cannot_do() {
    use algebra_advanced::crypto::{ReedSolomonCode, ReedSolomonError};
    let code = ReedSolomonCode::new(15, 9).unwrap(); // corrects 3
    assert!(matches!(
        code.encode(&[1, 2]),
        Err(ReedSolomonError::WrongLength {
            expected: 9,
            found: 2
        })
    ));
    assert!(ReedSolomonCode::new(300, 10).is_err());
    let message: Vec<u8> = (1..=9).collect();
    let word = code.encode(&message).unwrap();
    // Seven wrong symbols cannot be corrected back to the message: the decoder either says so or (rarely)
    // returns a different codeword, but never the original message.
    let mut bad = word.clone();
    for symbol in bad.iter_mut().take(7) {
        *symbol ^= 0x55;
    }
    assert_ne!(code.decode(&bad).ok(), Some(message));
}
