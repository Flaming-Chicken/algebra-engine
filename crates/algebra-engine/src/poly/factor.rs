//! Factorisation of univariate polynomials over $\mathbb{Z}$ (and so over $\mathbb{Q}$).
//!
//! The Zassenhaus algorithm: remove the content and the repeated factors (Yun), factor modulo a small prime $p$
//! (distinct-degree and Cantor–Zassenhaus equal-degree splitting), lift the factorisation to $p^k$ above the
//! Mignotte bound by linear Hensel lifting, and recombine the lifted factors by trial division over $\mathbb{Z}$.
//! The result is the complete factorisation into irreducibles; the recombination step makes it correct for
//! polynomials that factor modulo every prime, such as the Swinnerton-Dyer polynomials.

use super::UniPoly;
use algebra_core::{AlgebraError, AlgebraResult};
use num_bigint::{BigInt, BigUint};
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};

/// The complete factorisation of an integer polynomial: `content * prod(factor^multiplicity)`.
///
/// Every factor is primitive, irreducible over $\mathbb{Q}$ and has a positive leading coefficient; the sign of the
/// polynomial is carried by `content`. Coefficients are stored lowest degree first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegerFactorization {
    pub content: BigInt,
    pub factors: Vec<(Vec<BigInt>, u32)>,
}

impl IntegerFactorization {
    /// Multiply the factorisation back out (the inverse of [`factor_integer_polynomial`]).
    pub fn expand(&self) -> Vec<BigInt> {
        let mut acc = vec![self.content.clone()];
        for (factor, multiplicity) in &self.factors {
            for _ in 0..*multiplicity {
                acc = zmul(&acc, factor);
            }
        }
        trim_z(acc)
    }
}

fn trim_z(mut v: Vec<BigInt>) -> Vec<BigInt> {
    while v.last().is_some_and(|c| c.is_zero()) {
        v.pop();
    }
    v
}

fn zmul(a: &[BigInt], b: &[BigInt]) -> Vec<BigInt> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut out = vec![BigInt::zero(); a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            out[i + j] += x * y;
        }
    }
    out
}

fn content_z(f: &[BigInt]) -> BigInt {
    use num::Integer;
    f.iter().fold(BigInt::zero(), |g, c| g.gcd(c))
}

/// Division of `f` by `g` in $\mathbb{Z}[x]$ if it is exact, else `None`.
fn exact_div(f: &[BigInt], g: &[BigInt]) -> Option<Vec<BigInt>> {
    let (gd, glc) = (g.len().checked_sub(1)?, g.last()?);
    if f.len() < g.len() {
        return if f.is_empty() { Some(Vec::new()) } else { None };
    }
    let mut rem = f.to_vec();
    let mut quot = vec![BigInt::zero(); f.len() - gd];
    for k in (gd..f.len()).rev() {
        if rem[k].is_zero() {
            continue;
        }
        if !(&rem[k] % glc).is_zero() {
            return None;
        }
        let q = &rem[k] / glc;
        for j in 0..=gd {
            let delta = &q * &g[j];
            rem[k - gd + j] -= delta;
        }
        quot[k - gd] = q;
    }
    rem.iter().all(|c| c.is_zero()).then_some(quot)
}

/// Make a polynomial primitive with a positive leading coefficient.
fn primitive_part(f: &[BigInt]) -> Vec<BigInt> {
    let c = content_z(f);
    if c.is_zero() {
        return Vec::new();
    }
    let sign = if f.last().is_some_and(|l| l.is_negative()) {
        -c
    } else {
        c
    };
    f.iter().map(|x| x / &sign).collect()
}

// ---------------------------------------------------------------------------------------------------------------
// Arithmetic in F_p[x] (p an odd prime below 2^31), coefficients lowest degree first without trailing zeros.
// ---------------------------------------------------------------------------------------------------------------

type Fp = Vec<u64>;

fn fp_trim(mut a: Fp) -> Fp {
    while a.last() == Some(&0) {
        a.pop();
    }
    a
}

fn fp_inv(a: u64, p: u64) -> u64 {
    // Fermat: a^(p-2)
    let (mut result, mut base, mut e) = (1u64, a % p, p - 2);
    while e > 0 {
        if e & 1 == 1 {
            result = result * base % p;
        }
        base = base * base % p;
        e >>= 1;
    }
    result
}

fn fp_add(a: &[u64], b: &[u64], p: u64) -> Fp {
    let n = a.len().max(b.len());
    fp_trim(
        (0..n)
            .map(|i| (a.get(i).copied().unwrap_or(0) + b.get(i).copied().unwrap_or(0)) % p)
            .collect(),
    )
}

fn fp_sub(a: &[u64], b: &[u64], p: u64) -> Fp {
    let n = a.len().max(b.len());
    fp_trim(
        (0..n)
            .map(|i| (a.get(i).copied().unwrap_or(0) + p - b.get(i).copied().unwrap_or(0)) % p)
            .collect(),
    )
}

fn fp_mul(a: &[u64], b: &[u64], p: u64) -> Fp {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut out = vec![0u64; a.len() + b.len() - 1];
    for (i, &x) in a.iter().enumerate() {
        for (j, &y) in b.iter().enumerate() {
            out[i + j] = (out[i + j] + x * y) % p;
        }
    }
    fp_trim(out)
}

fn fp_divrem(a: &[u64], b: &[u64], p: u64) -> (Fp, Fp) {
    let mut rem = a.to_vec();
    if a.len() < b.len() {
        return (Vec::new(), fp_trim(rem));
    }
    let inv = fp_inv(*b.last().unwrap_or(&1), p);
    let mut quot = vec![0u64; a.len() - b.len() + 1];
    for k in (b.len() - 1..a.len()).rev() {
        let factor = rem[k] * inv % p;
        if factor == 0 {
            continue;
        }
        let shift = k + 1 - b.len();
        quot[shift] = factor;
        for (j, &bj) in b.iter().enumerate() {
            rem[shift + j] = (rem[shift + j] + p - factor * bj % p) % p;
        }
    }
    (fp_trim(quot), fp_trim(rem))
}

fn fp_monic(a: &[u64], p: u64) -> Fp {
    match a.last() {
        None => Vec::new(),
        Some(&lc) => {
            let inv = fp_inv(lc, p);
            a.iter().map(|&c| c * inv % p).collect()
        }
    }
}

fn fp_gcd(a: &[u64], b: &[u64], p: u64) -> Fp {
    let (mut a, mut b) = (a.to_vec(), b.to_vec());
    while !b.is_empty() {
        let r = fp_divrem(&a, &b, p).1;
        a = b;
        b = r;
    }
    fp_monic(&a, p)
}

fn fp_derivative(a: &[u64], p: u64) -> Fp {
    fp_trim((1..a.len()).map(|i| a[i] * (i as u64 % p) % p).collect())
}

/// `base^exp mod modulus` in $\mathbb{F}_p[x]$.
fn fp_powmod(base: &[u64], exp: &BigUint, modulus: &[u64], p: u64) -> Fp {
    let mut result: Fp = vec![1];
    let mut b = fp_divrem(base, modulus, p).1;
    for i in 0..exp.bits() {
        if exp.bit(i) {
            result = fp_divrem(&fp_mul(&result, &b, p), modulus, p).1;
        }
        b = fp_divrem(&fp_mul(&b, &b, p), modulus, p).1;
    }
    result
}

/// Extended Euclid over $\mathbb{F}_p[x]$: `(s, t)` with `s a + t b = 1` for coprime `a`, `b`.
fn fp_bezout(a: &[u64], b: &[u64], p: u64) -> (Fp, Fp) {
    let (mut r0, mut r1) = (a.to_vec(), b.to_vec());
    let (mut s0, mut s1): (Fp, Fp) = (vec![1], Vec::new());
    let (mut t0, mut t1): (Fp, Fp) = (Vec::new(), vec![1]);
    while !r1.is_empty() {
        let (q, r) = fp_divrem(&r0, &r1, p);
        let s2 = fp_sub(&s0, &fp_mul(&q, &s1, p), p);
        let t2 = fp_sub(&t0, &fp_mul(&q, &t1, p), p);
        r0 = r1;
        r1 = r;
        s0 = s1;
        s1 = s2;
        t0 = t1;
        t1 = t2;
    }
    // r0 is a non-zero constant (the gcd): scale it to 1.
    let inv = fp_inv(r0.first().copied().unwrap_or(1), p);
    let scale = |v: Fp| fp_trim(v.into_iter().map(|c| c * inv % p).collect());
    (scale(s0), scale(t0))
}

/// A small deterministic generator for the random splitting polynomials.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
}

/// Distinct-degree factorisation of a monic squarefree polynomial: pairs `(product of the irreducible factors of
/// degree d, d)`.
fn fp_ddf(f: &[u64], p: u64) -> Vec<(Fp, usize)> {
    let mut out = Vec::new();
    let mut f = f.to_vec();
    let mut h: Fp = vec![0, 1];
    let mut d = 0usize;
    while f.len() > 1 {
        d += 1;
        if f.len() - 1 < 2 * d {
            out.push((f.clone(), f.len() - 1));
            break;
        }
        h = fp_powmod(&h, &BigUint::from(p), &f, p);
        let g = fp_gcd(&fp_sub(&h, &[0, 1], p), &f, p);
        if g.len() > 1 {
            f = fp_divrem(&f, &g, p).0;
            h = fp_divrem(&h, &f, p).1;
            out.push((g, d));
        }
    }
    out
}

/// Equal-degree splitting (Cantor–Zassenhaus) of a monic squarefree polynomial whose irreducible factors all have
/// degree `d`.
fn fp_edf(f: &[u64], d: usize, p: u64, rng: &mut Lcg) -> Vec<Fp> {
    let degree = f.len() - 1;
    if degree == d {
        return vec![f.to_vec()];
    }
    let exponent = (BigUint::from(p).pow(d as u32) - BigUint::one()) / BigUint::from(2u32);
    loop {
        let a: Fp = fp_trim((0..degree).map(|_| rng.next() % p).collect());
        if a.len() < 2 {
            continue;
        }
        let b = fp_sub(&fp_powmod(&a, &exponent, f, p), &[1], p);
        let g = fp_gcd(&b, f, p);
        if g.len() > 1 && g.len() < f.len() {
            let other = fp_divrem(f, &g, p).0;
            let mut out = fp_edf(&g, d, p, rng);
            out.extend(fp_edf(&fp_monic(&other, p), d, p, rng));
            return out;
        }
    }
}

/// The monic irreducible factors of a monic squarefree polynomial modulo `p`.
fn fp_factor(f: &[u64], p: u64) -> Vec<Fp> {
    let mut rng = Lcg(0x9e37_79b9_7f4a_7c15);
    let mut out = Vec::new();
    for (g, d) in fp_ddf(f, p) {
        out.extend(fp_edf(&g, d, p, &mut rng));
    }
    out
}

// ---------------------------------------------------------------------------------------------------------------
// Hensel lifting
// ---------------------------------------------------------------------------------------------------------------

fn centered(c: &BigInt, modulus: &BigInt) -> BigInt {
    let r = ((c % modulus) + modulus) % modulus;
    if &r * 2 > *modulus { r - modulus } else { r }
}

fn to_fp(f: &[BigInt], p: u64) -> Fp {
    let pb = BigInt::from(p);
    fp_trim(
        f.iter()
            .map(|c| (((c % &pb) + &pb) % &pb).to_u64().unwrap_or(0))
            .collect(),
    )
}

/// Lift `f = g0 * h0 (mod p)` (monic factors, coprime) to `f = g * h (mod p^k)` with `g`, `h` monic.
fn hensel_pair(f: &[BigInt], g0: &[u64], h0: &[u64], p: u64, k: u32) -> (Vec<BigInt>, Vec<BigInt>) {
    let (s, t) = fp_bezout(g0, h0, p);
    let to_big = |v: &[u64]| -> Vec<BigInt> { v.iter().map(|&c| BigInt::from(c)).collect() };
    let (mut g, mut h) = (to_big(g0), to_big(h0));
    let mut modulus = BigInt::from(p);
    for _ in 1..k {
        let diff: Vec<BigInt> = {
            let prod = zmul(&g, &h);
            (0..f.len().max(prod.len()))
                .map(|i| {
                    f.get(i).cloned().unwrap_or_default() - prod.get(i).cloned().unwrap_or_default()
                })
                .collect()
        };
        // e = (f - g h) / modulus mod p
        let e: Fp = fp_trim(
            diff.iter()
                .map(|c| {
                    let q = c / &modulus;
                    (((q % p) + p) % p).to_u64().unwrap_or(0)
                })
                .collect(),
        );
        let (q, sigma) = fp_divrem(&fp_mul(&s, &e, p), h0, p);
        let tau = fp_add(&fp_mul(&t, &e, p), &fp_mul(&q, g0, p), p);
        for (i, &c) in tau.iter().enumerate() {
            if i >= g.len() {
                g.resize(i + 1, BigInt::zero());
            }
            g[i] += &modulus * c;
        }
        for (i, &c) in sigma.iter().enumerate() {
            if i >= h.len() {
                h.resize(i + 1, BigInt::zero());
            }
            h[i] += &modulus * c;
        }
        modulus *= p;
    }
    (g, h)
}

/// Lift a monic polynomial's factorisation modulo `p` (`factors`, monic, pairwise coprime) to modulo `p^k`.
fn hensel_lift(f: &[BigInt], factors: &[Fp], p: u64, k: u32) -> Vec<Vec<BigInt>> {
    if factors.len() == 1 {
        return vec![f.to_vec()];
    }
    let mid = factors.len() / 2;
    let product = |fs: &[Fp]| fs.iter().fold(vec![1u64], |acc, g| fp_mul(&acc, g, p));
    let (left, right) = factors.split_at(mid);
    let (g, h) = hensel_pair(f, &product(left), &product(right), p, k);
    let mut out = hensel_lift(&g, left, p, k);
    out.extend(hensel_lift(&h, right, p, k));
    out
}

// ---------------------------------------------------------------------------------------------------------------
// The factorisation of a squarefree primitive polynomial
// ---------------------------------------------------------------------------------------------------------------

const PRIMES: [u64; 16] = [3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59];

/// Factor a squarefree primitive polynomial of positive degree and positive leading coefficient.
fn factor_squarefree(f: &[BigInt]) -> Vec<Vec<BigInt>> {
    let degree = f.len() - 1;
    if degree == 1 {
        return vec![f.to_vec()];
    }
    let lc = f[degree].clone();
    // Choose a prime with a squarefree reduction and as few factors as possible.
    let mut best: Option<(u64, Vec<Fp>)> = None;
    let mut tried = 0;
    for &p in &PRIMES {
        if (&lc % p).is_zero() {
            continue;
        }
        let fp = fp_monic(&to_fp(f, p), p);
        if fp.len() != f.len() {
            continue;
        }
        if fp_gcd(&fp, &fp_derivative(&fp, p), p).len() > 1 {
            continue;
        }
        let factors = fp_factor(&fp, p);
        if factors.len() == 1 {
            return vec![f.to_vec()]; // irreducible modulo p, so irreducible over Q
        }
        if best.as_ref().is_none_or(|(_, b)| factors.len() < b.len()) {
            best = Some((p, factors));
        }
        tried += 1;
        if tried >= 5 {
            break;
        }
    }
    // Every small prime divides the discriminant or the leading coefficient: fall back to the bigger primes.
    let (p, mut mod_factors) = match best {
        Some(found) => found,
        None => {
            let mut p = 61u64;
            loop {
                if is_prime_small(p) && !(&lc % p).is_zero() {
                    let fp = fp_monic(&to_fp(f, p), p);
                    if fp.len() == f.len() && fp_gcd(&fp, &fp_derivative(&fp, p), p).len() <= 1 {
                        break (p, fp_factor(&fp, p));
                    }
                }
                p += 2;
            }
        }
    };
    mod_factors.sort();

    // Mignotte-style bound on the coefficients of any factor times the leading coefficient.
    let norm1: BigInt = f.iter().map(|c| c.abs()).sum();
    let bound = BigInt::from(2).pow(degree as u32) * &norm1 * &lc.abs() * 2 + 1;
    let mut k = 1u32;
    let mut modulus = BigInt::from(p);
    while modulus <= bound {
        modulus *= p;
        k += 1;
    }

    // Make f monic modulo p^k and lift.
    let lc_inv = mod_inverse(&lc, &modulus);
    let monic: Vec<BigInt> = f
        .iter()
        .map(|c| ((c * &lc_inv) % &modulus + &modulus) % &modulus)
        .collect();
    let lifted = hensel_lift(&monic, &mod_factors, p, k);

    // Recombination.
    let mut remaining: Vec<usize> = (0..lifted.len()).collect();
    let mut rest = f.to_vec();
    let mut found: Vec<Vec<BigInt>> = Vec::new();
    let mut size = 1;
    while remaining.len() >= 2 * size {
        let mut progressed = false;
        for subset in combinations(&remaining, size) {
            let mut candidate = vec![lc.clone()];
            for &i in &subset {
                candidate = zmul(&candidate, &lifted[i])
                    .into_iter()
                    .map(|c| centered(&c, &modulus))
                    .collect();
            }
            let candidate = primitive_part(&trim_z(candidate));
            if candidate.len() < 2 {
                continue;
            }
            if let Some(quotient) = exact_div(&rest, &candidate) {
                found.push(candidate);
                rest = primitive_part(&quotient);
                remaining.retain(|i| !subset.contains(i));
                progressed = true;
                break;
            }
        }
        if !progressed {
            size += 1;
        }
    }
    if rest.len() > 1 {
        found.push(rest);
    }
    found
}

fn is_prime_small(n: u64) -> bool {
    n >= 2 && (2..).take_while(|d| d * d <= n).all(|d| !n.is_multiple_of(d))
}

fn mod_inverse(a: &BigInt, modulus: &BigInt) -> BigInt {
    use num::Integer;
    let e = a.extended_gcd(modulus);
    ((e.x % modulus) + modulus) % modulus
}

/// All subsets of `items` with exactly `size` elements.
fn combinations(items: &[usize], size: usize) -> Vec<Vec<usize>> {
    fn go(
        items: &[usize],
        size: usize,
        start: usize,
        current: &mut Vec<usize>,
        out: &mut Vec<Vec<usize>>,
    ) {
        if current.len() == size {
            out.push(current.clone());
            return;
        }
        for i in start..items.len() {
            current.push(items[i]);
            go(items, size, i + 1, current, out);
            current.pop();
        }
    }
    let mut out = Vec::new();
    go(items, size, 0, &mut Vec::new(), &mut out);
    out
}

// ---------------------------------------------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------------------------------------------

/// Squarefree decomposition over $\mathbb{Q}$ (Yun): pairs `(squarefree factor, multiplicity)` of a non-constant
/// polynomial.
fn yun(f: &UniPoly) -> AlgebraResult<Vec<(UniPoly, u32)>> {
    let mut out = Vec::new();
    let derivative = f.derivative();
    let c = f.gcd(&derivative)?;
    let mut w = f.div_rem(&c)?.0;
    let mut y = derivative.div_rem(&c)?.0;
    let mut z = y.sub(&w.derivative());
    let mut i = 1u32;
    while w.degree() > 0 {
        let g = w.gcd(&z)?;
        if g.degree() > 0 {
            out.push((g.clone(), i));
        }
        w = w.div_rem(&g)?.0;
        y = z.div_rem(&g)?.0;
        z = y.sub(&w.derivative());
        i += 1;
    }
    Ok(out)
}

/// Factor a polynomial with integer coefficients (lowest degree first) completely over $\mathbb{Q}$.
pub fn factor_integer_polynomial(coeffs: &[BigInt]) -> AlgebraResult<IntegerFactorization> {
    let f = trim_z(coeffs.to_vec());
    if f.is_empty() {
        return Err(AlgebraError::DomainViolation {
            domain: "Polynomial".into(),
            reason: "the zero polynomial has no factorisation".into(),
        });
    }
    let content = content_z(&f);
    let sign = if f.last().is_some_and(|l| l.is_negative()) {
        -BigInt::one()
    } else {
        BigInt::one()
    };
    let primitive: Vec<BigInt> = f.iter().map(|c| c / &content * &sign).collect();
    let mut factors: Vec<(Vec<BigInt>, u32)> = Vec::new();
    if primitive.len() > 1 {
        let as_rational = UniPoly::from_coeffs(
            primitive
                .iter()
                .map(|c| BigRational::from_integer(c.clone()))
                .collect(),
        );
        for (part, multiplicity) in yun(&as_rational)? {
            // Clear denominators and the content: the squarefree part as a primitive integer polynomial.
            let denominator = part
                .coeffs
                .iter()
                .fold(BigInt::one(), |acc, c| num::Integer::lcm(&acc, c.denom()));
            let integral: Vec<BigInt> = part
                .coeffs
                .iter()
                .map(|c| (c * BigRational::from_integer(denominator.clone())).to_integer())
                .collect();
            for factor in factor_squarefree(&primitive_part(&integral)) {
                factors.push((factor, multiplicity));
            }
        }
    }
    factors.sort();
    Ok(IntegerFactorization {
        content: &content * &sign,
        factors,
    })
}

/// Factor a polynomial over $\mathbb{Q}$: the rational constant and the monic irreducible factors with multiplicities.
pub fn factor_rational_polynomial(
    f: &UniPoly,
) -> AlgebraResult<(BigRational, Vec<(UniPoly, u32)>)> {
    if f.is_zero() {
        return Err(AlgebraError::DomainViolation {
            domain: "Polynomial".into(),
            reason: "the zero polynomial has no factorisation".into(),
        });
    }
    let denominator = f
        .coeffs
        .iter()
        .fold(BigInt::one(), |acc, c| num::Integer::lcm(&acc, c.denom()));
    let integral: Vec<BigInt> = f
        .coeffs
        .iter()
        .map(|c| (c * BigRational::from_integer(denominator.clone())).to_integer())
        .collect();
    let z = factor_integer_polynomial(&integral)?;
    let mut constant = BigRational::new(z.content, denominator);
    let mut out = Vec::new();
    for (factor, multiplicity) in z.factors {
        let poly =
            UniPoly::from_coeffs(factor.into_iter().map(BigRational::from_integer).collect());
        constant *= poly.leading_coeff().pow(multiplicity as i32);
        out.push((poly.monic(), multiplicity));
    }
    Ok((constant, out))
}
