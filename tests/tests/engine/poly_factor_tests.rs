//! Factorisation of integer polynomials (Zassenhaus). The oracles are independent of the factoriser: multiplying
//! the factors back out, known factorisations, and products built from known irreducible pieces.

use algebra_engine::poly::UniPoly;
use algebra_engine::poly::factor::{factor_integer_polynomial, factor_rational_polynomial};
use num_bigint::BigInt;
use num_rational::BigRational;

fn z(c: &[i64]) -> Vec<BigInt> {
    c.iter().map(|&x| BigInt::from(x)).collect()
}

fn factors_of(c: &[i64]) -> Vec<(Vec<i64>, u32)> {
    let f = factor_integer_polynomial(&z(c)).unwrap();
    assert_eq!(
        f.expand(),
        z(c),
        "the factors must multiply back to the input {c:?}"
    );
    f.factors
        .into_iter()
        .map(|(p, m)| {
            (
                p.iter()
                    .map(|x| i64::try_from(x.clone()).unwrap())
                    .collect(),
                m,
            )
        })
        .collect()
}

fn sorted(mut v: Vec<(Vec<i64>, u32)>) -> Vec<(Vec<i64>, u32)> {
    v.sort();
    v
}

#[test]
fn known_factorisations() {
    // x^4 + 4 = (x^2 + 2x + 2)(x^2 - 2x + 2)  (Sophie Germain)
    assert_eq!(
        sorted(factors_of(&[4, 0, 0, 0, 1])),
        sorted(vec![(vec![2, -2, 1], 1), (vec![2, 2, 1], 1)])
    );
    // x^2 - 1 = (x - 1)(x + 1)
    assert_eq!(
        sorted(factors_of(&[-1, 0, 1])),
        sorted(vec![(vec![-1, 1], 1), (vec![1, 1], 1)])
    );
    // x^4 + 1 is irreducible over Q although it factors modulo every prime.
    assert_eq!(factors_of(&[1, 0, 0, 0, 1]), vec![(vec![1, 0, 0, 0, 1], 1)]);
    // Repeated factors: (x - 1)^3 (x + 2) = x^4 - x^3 - 3x^2 + 5x - 2
    assert_eq!(
        sorted(factors_of(&[-2, 5, -3, -1, 1])),
        sorted(vec![(vec![-1, 1], 3), (vec![2, 1], 1)])
    );
    // x^6 - 1 = (x-1)(x+1)(x^2+x+1)(x^2-x+1): the cyclotomic factorisation
    assert_eq!(
        sorted(factors_of(&[-1, 0, 0, 0, 0, 0, 1])),
        sorted(vec![
            (vec![-1, 1], 1),
            (vec![1, 1], 1),
            (vec![1, 1, 1], 1),
            (vec![1, -1, 1], 1)
        ])
    );
}

#[test]
fn content_sign_and_multiplicity() {
    // -6 (x - 1)^2 (x + 2) = -6 (x^3 - 3x + 2)
    let f = factor_integer_polynomial(&z(&[-12, 18, 0, -6])).unwrap();
    assert_eq!(f.content, BigInt::from(-6));
    assert_eq!(f.expand(), z(&[-12, 18, 0, -6]));
    let mut fs: Vec<(Vec<BigInt>, u32)> = f.factors;
    fs.sort();
    assert_eq!(fs, vec![(z(&[-1, 1]), 2), (z(&[2, 1]), 1)]);
    // A constant has no factors.
    let c = factor_integer_polynomial(&z(&[7])).unwrap();
    assert!(c.factors.is_empty());
    assert_eq!(c.content, BigInt::from(7));
    assert!(factor_integer_polynomial(&[]).is_err());
}

#[test]
fn swinnerton_dyer_polynomial_is_irreducible_although_it_factors_modulo_every_prime() {
    // The minimal polynomial of sqrt(2) + sqrt(3) + sqrt(5): degree 8, irreducible over Q, but reducible modulo
    // every prime, so the recombination step has to do all the work.
    // x^8 - 40x^6 + 352x^4 - 960x^2 + 576
    let c = [576, 0, -960, 0, 352, 0, -40, 0, 1];
    let f = factors_of(&c);
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f[0].1, 1);
    assert_eq!(f[0].0, c.to_vec());
}

#[test]
fn products_of_known_irreducibles_are_recovered() {
    // Build products from irreducible pieces known to be irreducible (Eisenstein or by hand) and check that exactly
    // those pieces come back, with the right multiplicities.
    let pieces: Vec<Vec<i64>> = vec![
        vec![2, 0, 1],           // x^2 + 2
        vec![-2, 0, 1],          // x^2 - 2
        vec![-2, 0, 0, 1],       // x^3 - 2
        vec![3, 0, 0, 0, 1],     // x^4 + 3 (Eisenstein at 3)
        vec![-5, 0, 0, 0, 0, 1], // x^5 - 5
        vec![1, 1, 1, 1, 1],     // 5th cyclotomic
        vec![-3, 1],             // x - 3
        vec![7, 2],              // 2x + 7 (non-monic linear)
    ];
    let mul = |a: &Vec<i64>, b: &Vec<i64>| {
        let mut out = vec![0i64; a.len() + b.len() - 1];
        for (i, x) in a.iter().enumerate() {
            for (j, y) in b.iter().enumerate() {
                out[i + j] += x * y;
            }
        }
        out
    };
    // Subsets by bitmask, with the piece x^2 - 2 taken twice when its bit is set.
    for mask in 1u32..(1 << pieces.len()) {
        let mut product = vec![1i64];
        let mut want = Vec::new();
        for (i, piece) in pieces.iter().enumerate() {
            if mask & (1 << i) != 0 {
                let times = if i == 1 { 2 } else { 1 };
                for _ in 0..times {
                    product = mul(&product, piece);
                }
                want.push((piece.clone(), times as u32));
            }
        }
        if product.iter().map(|c| c.unsigned_abs() as f64).sum::<f64>() > 1e15 {
            continue;
        }
        // The factoriser normalises each factor to a positive leading coefficient and primitive content.
        let got = sorted(factors_of(&product));
        assert_eq!(got, sorted(want), "mask {mask:b}");
    }
}

#[test]
fn wilkinson_like_product_of_linear_factors() {
    // prod_{i=1}^{10} (x - i): ten linear factors, many factors modulo every small prime.
    let mut p = vec![1i64];
    for i in 1..=10i64 {
        let mut next = vec![0i64; p.len() + 1];
        for (j, c) in p.iter().enumerate() {
            next[j + 1] += c;
            next[j] -= i * c;
        }
        p = next;
    }
    let got = factors_of(&p);
    assert_eq!(got.len(), 10);
    let want: Vec<(Vec<i64>, u32)> = (1..=10).map(|i| (vec![-i, 1], 1)).collect();
    assert_eq!(sorted(got), sorted(want));
}

#[test]
fn factoring_over_the_rationals() {
    // (1/2) x^2 - 2 = (1/2)(x - 2)(x + 2)
    let half = |n: i64, d: i64| BigRational::new(BigInt::from(n), BigInt::from(d));
    let f = UniPoly::from_coeffs(vec![half(-2, 1), half(0, 1), half(1, 2)]);
    let (constant, factors) = factor_rational_polynomial(&f).unwrap();
    assert_eq!(constant, half(1, 2));
    assert_eq!(factors.len(), 2);
    let mut product = UniPoly::constant(constant);
    for (g, m) in &factors {
        for _ in 0..*m {
            product = product.mul(g);
        }
    }
    assert_eq!(product, f);
}
