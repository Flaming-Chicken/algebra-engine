//! # `algebra_engine::numbertheory`
//!
//! Computational Number Theory & Infinite Continued Fractions:
//! - Continued Fraction Expansion $[a_0; a_1, a_2, \dots]$
//! - Miller-Rabin Primality Testing
//! - Greatest Common Divisor (Extended Euclidean Algorithm)
//! - Legendre and Jacobi Symbols $(a/p)$
//! - Modular Exponentiation & Inverses
//! - Elliptic Curve Method (Lenstra), Pollard Rho, & Tonelli-Shanks Quadratic Residues
//!
//! ## References
//! - Rabin, M. O. (1980). Probabilistic algorithm for testing primality. *J. Number Theory*, 12(1), 128–138.
//! - Pollard, J. M. (1975). A Monte Carlo method for factorization. *BIT Numer. Math.*, 15(3), 331–334.
//! - Lenstra, H. W., Jr. (1987). Factoring integers with elliptic curves. *Annals of Mathematics*, 126(3), 649–673.
//! - Cohen, H. (1993). *A Course in Computational Algebraic Number Theory*. Springer.

use algebra_core::traits::ValuationProvider;
use num_bigint::BigUint;
use num_traits::One;
use std::collections::BTreeMap;

/// A global valuation profile over $\mathbb{Q}$, storing all non-zero $p$-adic valuations and the Archimedean absolute value.
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalValuationProfile {
    pub num: i64,
    pub den: i64,
    pub finite_places: BTreeMap<u64, i32>,
    pub infinite_place_abs: f64,
}

impl GlobalValuationProfile {
    /// Creates a valuation profile by fully factoring the rational $num/den$.
    pub fn from_rational(num: i64, den: i64) -> Option<Self> {
        if num == 0 || den == 0 {
            return None;
        }
        let abs_num = num.unsigned_abs();
        let abs_den = den.unsigned_abs();

        let num_factors = integer_prime_factors(abs_num);
        let den_factors = integer_prime_factors(abs_den);

        let mut finite_places = BTreeMap::new();
        for (p, exp) in num_factors {
            *finite_places.entry(p).or_insert(0) += exp as i32;
        }
        for (p, exp) in den_factors {
            *finite_places.entry(p).or_insert(0) -= exp as i32;
        }
        finite_places.retain(|_, v| *v != 0);

        let infinite_place_abs = (num as f64 / den as f64).abs();

        Some(Self {
            num,
            den,
            finite_places,
            infinite_place_abs,
        })
    }
}

impl ValuationProvider for GlobalValuationProfile {
    fn finite_places(&self) -> BTreeMap<u64, i32> {
        self.finite_places.clone()
    }

    fn infinite_place_abs(&self) -> f64 {
        self.infinite_place_abs
    }
}

/// Continued Fraction expansion representation $[a_0; a_1, a_2, \dots, a_n]$.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinuedFraction {
    pub terms: Vec<i64>,
}

impl ContinuedFraction {
    /// Compute continued fraction expansion of a floating-point real $x$.
    pub fn from_f64(mut x: f64, max_terms: usize) -> Self {
        let mut terms = Vec::new();
        for _ in 0..max_terms {
            let a = x.floor() as i64;
            terms.push(a);
            let frac = x - (a as f64);
            if frac.abs() < 1e-12 {
                break;
            }
            x = 1.0 / frac;
        }
        Self { terms }
    }

    /// Reconstruct rational convergent $p_n / q_n$.
    pub fn convergent(&self) -> (i64, i64) {
        if self.terms.is_empty() {
            return (0, 1);
        }
        let mut h_prev2 = 0i64;
        let mut h_prev1 = 1i64;
        let mut k_prev2 = 1i64;
        let mut k_prev1 = 0i64;

        for &a in &self.terms {
            let h = a * h_prev1 + h_prev2;
            let k = a * k_prev1 + k_prev2;
            h_prev2 = h_prev1;
            h_prev1 = h;
            k_prev2 = k_prev1;
            k_prev1 = k;
        }
        (h_prev1, k_prev1)
    }
}

/// Extended Euclidean Algorithm $a \cdot x + b \cdot y = \gcd(a, b)$.
pub fn extended_gcd(a: i64, b: i64) -> (i64, i64, i64) {
    if b == 0 {
        (a.abs(), if a >= 0 { 1 } else { -1 }, 0)
    } else {
        let (gcd, x1, y1) = extended_gcd(b, a % b);
        let x = y1;
        let y = x1 - (a / b) * y1;
        (gcd, x, y)
    }
}

fn mul_mod_u64(a: u64, b: u64, m: u64) -> u64 {
    ((a as u128 * b as u128) % m as u128) as u64
}

fn pow_mod_u64(mut base: u64, mut exp: u64, m: u64) -> u64 {
    if m == 1 {
        return 0;
    }
    let mut res = 1u64;
    base %= m;
    while exp > 0 {
        if exp & 1 == 1 {
            res = mul_mod_u64(res, base, m);
        }
        base = mul_mod_u64(base, base, m);
        exp >>= 1;
    }
    res
}

fn gcd_u64(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

/// Deterministic primality check for all $n < 2^{64}$ via 7-base Miller-Rabin.
pub fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    const SMALL_PRIMES: [u64; 12] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37];
    for &p in &SMALL_PRIMES {
        if n == p {
            return true;
        }
        if n.is_multiple_of(p) {
            return false;
        }
    }
    // For n < 37^2 = 1369, divisibility by SMALL_PRIMES already decides primality.
    if n < 1369 {
        return true;
    }
    let mut d = n - 1;
    let s = d.trailing_zeros();
    d >>= s;
    // Deterministic witness set valid for all n < 2^64 (Sinclair 2011).
    const BASES: [u64; 7] = [2, 325, 9375, 28178, 450775, 9780504, 1795265022];
    'witness: for &a in &BASES {
        let a = a % n;
        if a == 0 {
            continue;
        }
        let mut x = pow_mod_u64(a, d, n);
        if x == 1 || x == n - 1 {
            continue;
        }
        for _ in 1..s {
            x = mul_mod_u64(x, x, n);
            if x == n - 1 {
                continue 'witness;
            }
        }
        return false;
    }
    true
}

/// Compute Legendre symbol $(a/p) \in \{-1, 0, 1\}$ for an odd prime $p$.
pub fn legendre_symbol(mut a: i64, p: u64) -> i32 {
    a = a.rem_euclid(p as i64);
    if a == 0 {
        return 0;
    }
    let exp = (p - 1) / 2;
    let pow_mod = BigUint::from(a as u64).modpow(&BigUint::from(exp), &BigUint::from(p));
    if pow_mod == BigUint::one() { 1 } else { -1 }
}

/// Detailed classification of algebraic and number-theoretic primes across all number systems.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrimeClassification {
    EvenPrime,
    GaussianSplit { u: i64, v: i64 },
    GaussianInert,
    EisensteinSplit { u: i64, v: i64 },
    EisensteinInert,
    EisensteinRamified,
    Mersenne { k: u32 },
    Fermat { k: u32 },
    SophieGermain,
    SafePrime,
    TwinPrime { paired: i64 },
    GaussianRamifiedPrime,
    GaussianSplitPrime,
    GaussianInertPrime,
    EisensteinRamifiedPrime,
    EisensteinSplitPrime,
    EisensteinInertPrime,
    ModularUnit { order: u64 },
    ModularZeroDivisor { gcd: u64 },
    ModularNilpotent { index: u32 },
    ModularIdempotent,
    ModularPrimeIdeal,
    PadicUniformizer { valuation: i32 },
    PadicUnit,
    PadicFractionalPole { order: u32 },
    GaloisGenerator { order: u64 },
    GaloisSubfieldElement,
    RationalNumeratorPrime,
    RationalDenominatorPrime,
}

impl std::fmt::Display for PrimeClassification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EvenPrime => write!(f, "Even prime (only even prime; ramifies in ℤ[i] and ℤ[ω])"),
            Self::GaussianSplit { u, v } => write!(
                f,
                "Pythagorean / Gaussian-split prime (p ≡ 1 mod 4; {}² + {}² = p; splits in ℤ[i])",
                u, v
            ),
            Self::GaussianInert => write!(
                f,
                "Gaussian-inert prime (p ≡ 3 mod 4; remains prime in ℤ[i])"
            ),
            Self::EisensteinSplit { u, v } => write!(
                f,
                "Eisenstein-split prime (p ≡ 1 mod 3; {}² - {}·{} + {}² = p; splits in ℤ[ω])",
                u, u, v, v
            ),
            Self::EisensteinInert => write!(
                f,
                "Eisenstein-inert prime (p ≡ 2 mod 3; remains prime in ℤ[ω])"
            ),
            Self::EisensteinRamified => write!(
                f,
                "Eisenstein-ramified prime (p = 3; ramifies as -ω²(1-ω)²)"
            ),
            Self::Mersenne { k } => write!(f, "Mersenne prime (2^{} - 1)", k),
            Self::Fermat { k } => write!(f, "Fermat prime (2^(2^{}) + 1)", k),
            Self::SophieGermain => write!(f, "Sophie Germain prime (2p + 1 is prime)"),
            Self::SafePrime => write!(f, "Safe prime ((p - 1)/2 is prime)"),
            Self::TwinPrime { paired } => write!(f, "Twin prime (paired with {})", paired),
            Self::GaussianRamifiedPrime => {
                write!(f, "Gaussian-ramified prime (associate of 1 + i; norm 2)")
            }
            Self::GaussianSplitPrime => write!(
                f,
                "Gaussian-split prime (norm is rational prime p ≡ 1 mod 4)"
            ),
            Self::GaussianInertPrime => write!(
                f,
                "Gaussian-inert prime (rational prime q ≡ 3 mod 4; norm q²)"
            ),
            Self::EisensteinRamifiedPrime => {
                write!(f, "Eisenstein-ramified prime (associate of 1 - ω; norm 3)")
            }
            Self::EisensteinSplitPrime => write!(
                f,
                "Eisenstein-split prime (norm is rational prime p ≡ 1 mod 3)"
            ),
            Self::EisensteinInertPrime => write!(
                f,
                "Eisenstein-inert prime (rational prime q ≡ 2 mod 3; norm q²)"
            ),
            Self::ModularUnit { order } => write!(
                f,
                "Modular unit (invertible residue with multiplicative order {})",
                order
            ),
            Self::ModularZeroDivisor { gcd } => {
                write!(f, "Modular zero divisor (gcd(x, n) = {})", gcd)
            }
            Self::ModularNilpotent { index } => {
                write!(f, "Modular nilpotent element (x^{} ≡ 0 mod n)", index)
            }
            Self::ModularIdempotent => write!(f, "Modular idempotent element (x² ≡ x mod n)"),
            Self::ModularPrimeIdeal => write!(f, "Modular prime/maximal ideal generator"),
            Self::PadicUniformizer { valuation } => write!(
                f,
                "p-adic uniformizer prime (valuation v_p = {})",
                valuation
            ),
            Self::PadicUnit => write!(f, "p-adic unit (|u|_p = 1)"),
            Self::PadicFractionalPole { order } => {
                write!(f, "p-adic fractional pole (order {})", order)
            }
            Self::GaloisGenerator { order } => {
                write!(f, "Galois field primitive generator (order {})", order)
            }
            Self::GaloisSubfieldElement => write!(f, "Galois subfield element"),
            Self::RationalNumeratorPrime => write!(f, "Rational numerator prime factor"),
            Self::RationalDenominatorPrime => write!(f, "Rational denominator pole factor"),
        }
    }
}

/// An individual prime factor in a decomposition.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct PrimeFactor {
    pub factor_repr: String,
    pub exponent: i32,
    pub norm_or_val: Option<f64>,
    pub classifications: Vec<PrimeClassification>,
}

/// Complete prime decomposition result with formatted representations and classifications.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct PrimeDecompositionResult {
    pub number_system: String,
    pub unit_part: String,
    pub factors: Vec<PrimeFactor>,
    pub formatted_equation: String,
    pub latex_equation: String,
    pub classification_summary: Vec<String>,
}

fn pollard_rho_u64(n: u64) -> u64 {
    if n.is_multiple_of(2) {
        return 2;
    }
    for c_init in [1u64, 3, 5, 7, 11, 13, 17, 19, 23, 29] {
        let mut x = 2u64 % n;
        let mut y = x;
        let mut d = 1u64;
        let f = |v: u64| -> u64 {
            let sq = mul_mod_u64(v, v, n);
            let sum = sq.checked_add(c_init % n).unwrap_or_else(|| {
                ((sq as u128 + (c_init % n) as u128) % n as u128) as u64
            });
            sum % n
        };
        // Brent-style cycle detection with batched gcd for speed
        let mut q = 1u64;
        let mut r = 1u64;
        let mut ys = x;
        while d == 1 {
            x = y;
            for _ in 0..r {
                y = f(y);
            }
            let mut k = 0u64;
            while k < r && d == 1 {
                ys = y;
                let lim = (r - k).min(128);
                for _ in 0..lim {
                    y = f(y);
                    let diff = x.abs_diff(y);
                    q = mul_mod_u64(q, diff, n);
                }
                d = gcd_u64(q, n);
                k += lim;
            }
            r <<= 1;
        }
        if d == n {
            loop {
                ys = f(ys);
                d = gcd_u64(x.abs_diff(ys), n);
                if d > 1 {
                    break;
                }
            }
        }
        if d > 1 && d < n {
            return d;
        }
    }
    // Fallback small odd divisor
    let mut d = 3u64;
    while d <= n / d {
        if n.is_multiple_of(d) {
            return d;
        }
        d += 2;
    }
    n
}

fn factor_u64_recursive(n: u64, counts: &mut BTreeMap<u64, u32>) {
    if n <= 1 {
        return;
    }
    if is_prime(n) {
        *counts.entry(n).or_insert(0) += 1;
        return;
    }
    let d = pollard_rho_u64(n);
    if d == n {
        *counts.entry(n).or_insert(0) += 1;
    } else {
        factor_u64_recursive(d, counts);
        factor_u64_recursive(n / d, counts);
    }
}

/// Factor a non-negative integer into its prime components $(p_i, e_i)$
/// using wheel/small-prime trial division + Miller-Rabin + Brent Pollard rho.
pub fn integer_prime_factors(mut n: u64) -> Vec<(u64, u32)> {
    let mut counts: BTreeMap<u64, u32> = BTreeMap::new();
    if n < 2 {
        return Vec::new();
    }
    for p in [2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47] {
        let mut e = 0u32;
        while n.is_multiple_of(p) {
            e += 1;
            n /= p;
        }
        if e > 0 {
            counts.insert(p, e);
        }
    }
    if n > 1 {
        factor_u64_recursive(n, &mut counts);
    }
    counts.into_iter().collect()
}

/// Arbitrary-precision Extended Euclidean Algorithm $a \cdot x + b \cdot y = \gcd(a, b)$.
pub fn extended_gcd_big(
    a: &num_bigint::BigInt,
    b: &num_bigint::BigInt,
) -> (num_bigint::BigInt, num_bigint::BigInt, num_bigint::BigInt) {
    use num_bigint::BigInt;
    use num_traits::{Signed as _, Zero as _};
    let mut old_r = a.clone();
    let mut r = b.clone();
    let mut old_s = BigInt::from(1u8);
    let mut s = BigInt::zero();
    let mut old_t = BigInt::zero();
    let mut t = BigInt::from(1u8);
    while !r.is_zero() {
        let q = &old_r / &r;
        let next_r = &old_r - &q * &r;
        old_r = r;
        r = next_r;
        let next_s = &old_s - &q * &s;
        old_s = s;
        s = next_s;
        let next_t = &old_t - &q * &t;
        old_t = t;
        t = next_t;
    }
    if old_r.is_negative() {
        (-old_r, -old_s, -old_t)
    } else {
        (old_r, old_s, old_t)
    }
}

/// Arbitrary-precision probable primality check via Miller-Rabin (exact for $n < 2^{64}$).
pub fn is_prime_big(n: &BigUint) -> bool {
    use num_traits::{ToPrimitive as _, Zero as _};
    if let Some(u) = n.to_u64() {
        return is_prime(u);
    }
    if n.is_zero() || n.is_one() {
        return false;
    }
    for &p in &[2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47] {
        let bp = BigUint::from(p);
        if (n % &bp).is_zero() {
            return n == &bp;
        }
    }
    let one = BigUint::from(1u8);
    let n_minus_1 = n - &one;
    let s = n_minus_1.trailing_zeros().unwrap_or(0);
    let d = &n_minus_1 >> s;
    const BASES: [u64; 18] = [
        2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61,
    ];
    'witness: for &base in &BASES {
        let a = BigUint::from(base) % n;
        if a.is_zero() {
            continue;
        }
        let mut x = a.modpow(&d, n);
        if x == one || x == n_minus_1 {
            continue;
        }
        for _ in 1..s {
            x = (&x * &x) % n;
            if x == n_minus_1 {
                continue 'witness;
            }
        }
        return false;
    }
    true
}

fn gcd_biguint(mut a: BigUint, mut b: BigUint) -> BigUint {
    use num_traits::Zero as _;
    while !b.is_zero() {
        let t = &a % &b;
        a = b;
        b = t;
    }
    a
}

fn pollard_rho_big(n: &BigUint) -> BigUint {
    use num_traits::{ToPrimitive as _, Zero as _};
    if let Some(u) = n.to_u64() {
        return BigUint::from(pollard_rho_u64(u));
    }
    let two = BigUint::from(2u8);
    if (n % &two).is_zero() {
        return two;
    }
    let one = BigUint::from(1u8);
    for c_init in [1u64, 3, 5, 7, 11, 13, 17, 19, 23, 29] {
        let c = BigUint::from(c_init) % n;
        let mut x = two.clone() % n;
        let mut y = x.clone();
        let mut d = one.clone();
        let mut q = one.clone();
        let mut r = 1usize;
        let mut ys = x.clone();
        let step = |v: &BigUint| -> BigUint { ((v * v) + &c) % n };
        while d == one {
            x = y.clone();
            for _ in 0..r {
                y = step(&y);
            }
            let mut k = 0usize;
            while k < r && d == one {
                ys = y.clone();
                let lim = (r - k).min(64);
                for _ in 0..lim {
                    y = step(&y);
                    let diff = if x >= y { &x - &y } else { &y - &x };
                    q = (&q * &diff) % n;
                }
                d = gcd_biguint(q.clone(), n.clone());
                k += lim;
            }
            r <<= 1;
            if r > (1 << 22) {
                break;
            }
        }
        if &d == n {
            for _ in 0..10000 {
                ys = step(&ys);
                let diff = if x >= ys { &x - &ys } else { &ys - &x };
                d = gcd_biguint(diff, n.clone());
                if d > one {
                    break;
                }
            }
        }
        if d > one && &d < n {
            return d;
        }
    }
    n.clone()
}

fn factor_big_recursive(n: BigUint, counts: &mut BTreeMap<BigUint, u32>) {
    use num_traits::ToPrimitive as _;
    let one = BigUint::from(1u8);
    if n <= one {
        return;
    }
    if let Some(u) = n.to_u64() {
        for (p, e) in integer_prime_factors(u) {
            *counts.entry(BigUint::from(p)).or_insert(0) += e;
        }
        return;
    }
    if is_prime_big(&n) {
        *counts.entry(n).or_insert(0) += 1;
        return;
    }
    let d = pollard_rho_big(&n);
    if d == n {
        *counts.entry(n).or_insert(0) += 1;
    } else {
        let q = &n / &d;
        factor_big_recursive(d, counts);
        factor_big_recursive(q, counts);
    }
}

/// Factor an arbitrary-precision positive integer into sorted `(factor, exponent)` pairs.
///
/// Every factor is prime unless Pollard rho ran out of budget on it, in which
/// case the unfactored composite is returned as-is; check with [`is_prime_big`].
pub fn big_prime_factors(n: &BigUint) -> Vec<(BigUint, u32)> {
    use num_traits::Zero as _;
    let two = BigUint::from(2u8);
    if n < &two {
        return Vec::new();
    }
    let mut cur = n.clone();
    let mut counts: BTreeMap<BigUint, u32> = BTreeMap::new();
    for p in [
        2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79,
        83, 89, 97,
    ] {
        let bp = BigUint::from(p);
        let mut e = 0u32;
        while (&cur % &bp).is_zero() {
            e += 1;
            cur /= &bp;
        }
        if e > 0 {
            counts.insert(bp, e);
        }
    }
    if cur > BigUint::from(1u8) {
        factor_big_recursive(cur, &mut counts);
    }
    counts.into_iter().collect()
}

/// Decompose an arbitrary-precision integer in $\mathbb{Z}$ into primes.
pub fn decompose_bigint(n: &num_bigint::BigInt) -> PrimeDecompositionResult {
    use num_traits::{Signed as _, ToPrimitive as _, Zero as _};
    if let Some(small) = n.to_i64() {
        return decompose_integer(small);
    }
    let unit = if n.is_negative() { "-1" } else { "1" };
    if n.is_zero() {
        return decompose_integer(0);
    }
    let abs_n = n.magnitude();
    let raw_factors = big_prime_factors(abs_n);
    let mut factors = Vec::new();
    let mut summary = Vec::new();
    let mut eq_parts = Vec::new();
    let mut latex_parts = Vec::new();
    if n.is_negative() {
        eq_parts.push("-1".to_string());
        latex_parts.push("-1".to_string());
    }
    for (p, exp) in raw_factors {
        let classes = if let Some(u) = p.to_u64() {
            classify_rational_prime(u)
        } else {
            Vec::new()
        };
        // Pollard rho can run out of budget on a number Miller-Rabin has
        // already shown composite. Say so rather than list it as a prime.
        let unfactored = p.to_u64().is_none() && !is_prime_big(&p);
        let desc = if unfactored {
            "Composite: no factor found within the Pollard rho budget".to_string()
        } else if classes.is_empty() {
            "Arbitrary-precision probable prime (Miller-Rabin)".to_string()
        } else {
            classes
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join("; ")
        };
        summary.push(format!("{}^{}: {}", p, exp, desc));
        let (text_note, latex_note) = if unfactored {
            (
                " [composite, not factored]",
                "\\,\\text{[composite, not factored]}",
            )
        } else {
            ("", "")
        };
        if exp == 1 {
            eq_parts.push(format!("{p}{text_note}"));
            latex_parts.push(format!("{p}{latex_note}"));
        } else {
            eq_parts.push(format!("{p}^{exp}{text_note}"));
            latex_parts.push(format!("{p}^{{{exp}}}{latex_note}"));
        }
        factors.push(PrimeFactor {
            factor_repr: p.to_string(),
            exponent: exp as i32,
            norm_or_val: p.to_f64(),
            classifications: classes,
        });
    }
    let formatted_equation = format!("{} = {}", n, eq_parts.join(" * "));
    let latex_equation = format!("{} = {}", n, latex_parts.join(" \\cdot "));
    PrimeDecompositionResult {
        number_system: "Integers (ℤ)".to_string(),
        unit_part: unit.to_string(),
        factors,
        formatted_equation,
        latex_equation,
        classification_summary: summary,
    }
}

/// Find sum of two squares $u^2 + v^2 = p$ for a prime $p \equiv 1 \pmod 4$.
pub fn find_sum_of_two_squares(p: u64) -> Option<(i64, i64)> {
    if p == 2 {
        return Some((1, 1));
    }
    if p % 4 != 1 {
        return None;
    }
    let max_u = (p as f64).sqrt() as u64;
    for u in 1..=max_u {
        let rem = p - u * u;
        let v = (rem as f64).sqrt() as u64;
        if v * v == rem {
            return Some((u as i64, v as i64));
        }
    }
    None
}

/// Find coordinates $u^2 - uv + v^2 = p$ for an Eisenstein split prime $p \equiv 1 \pmod 3$.
pub fn find_eisenstein_prime_coords(p: u64) -> Option<(i64, i64)> {
    if p == 3 {
        return Some((2, 1)); // 2^2 - 2(1) + 1^2 = 3
    }
    if p % 3 != 1 {
        return None;
    }
    // 4p = X^2 + 3Y^2 with X = |2u - v|, Y = v
    let target = 4 * p;
    let max_y = ((target / 3) as f64).sqrt() as u64;
    for y in 1..=max_y {
        let rem = target - 3 * y * y;
        let x = (rem as f64).sqrt() as u64;
        if x * x == rem && x % 2 == y % 2 {
            let u = ((x + y) / 2) as i64;
            return Some((u, y as i64));
        }
    }
    None
}

/// Classify an integer prime $p$ across all algebraic dimensions.
pub fn classify_rational_prime(p: u64) -> Vec<PrimeClassification> {
    let mut classes = Vec::new();
    if p == 2 {
        classes.push(PrimeClassification::EvenPrime);
        classes.push(PrimeClassification::EisensteinInert);
    } else {
        // Modulo 4 (Gaussian behavior)
        if p % 4 == 1 {
            if let Some((u, v)) = find_sum_of_two_squares(p) {
                classes.push(PrimeClassification::GaussianSplit { u, v });
            }
        } else if p % 4 == 3 {
            classes.push(PrimeClassification::GaussianInert);
        }

        // Modulo 3 (Eisenstein behavior)
        if p == 3 {
            classes.push(PrimeClassification::EisensteinRamified);
        } else if p % 3 == 1 {
            if let Some((u, v)) = find_eisenstein_prime_coords(p) {
                classes.push(PrimeClassification::EisensteinSplit { u, v });
            }
        } else if p % 3 == 2 {
            classes.push(PrimeClassification::EisensteinInert);
        }
    }

    // Mersenne prime: p = 2^k - 1
    if let Some(p_plus_1) = p.checked_add(1)
        && p_plus_1.is_power_of_two()
    {
        let k = p_plus_1.trailing_zeros();
        classes.push(PrimeClassification::Mersenne { k });
    }

    // Fermat prime: p = 2^(2^k) + 1
    if p > 2 && (p - 1).is_power_of_two() {
        let m = (p - 1).trailing_zeros();
        if m.is_power_of_two() || m == 0 {
            let k = if m == 0 { 0 } else { m.trailing_zeros() };
            classes.push(PrimeClassification::Fermat { k });
        }
    }

    // Sophie Germain prime: 2p + 1 is prime
    if let Some(sg) = p.checked_mul(2).and_then(|v| v.checked_add(1))
        && is_prime(sg)
    {
        classes.push(PrimeClassification::SophieGermain);
    }

    // Safe prime: (p - 1)/2 is prime
    if p > 2 && (p - 1).is_multiple_of(2) && is_prime((p - 1) / 2) {
        classes.push(PrimeClassification::SafePrime);
    }

    // Twin prime: p - 2 or p + 2 is prime
    if p > 2 && is_prime(p - 2) {
        classes.push(PrimeClassification::TwinPrime {
            paired: (p - 2) as i64,
        });
    } else if let Some(tp) = p.checked_add(2)
        && is_prime(tp)
        && let Ok(paired) = i64::try_from(tp)
    {
        classes.push(PrimeClassification::TwinPrime { paired });
    }

    classes
}

/// Decompose an integer in $\mathbb{Z}$ into primes with classification.
pub fn decompose_integer(n: i64) -> PrimeDecompositionResult {
    let unit = if n < 0 { "-1" } else { "1" };
    let abs_n = n.unsigned_abs();

    if abs_n == 0 {
        return PrimeDecompositionResult {
            number_system: "Integers (ℤ)".to_string(),
            unit_part: "0".to_string(),
            factors: Vec::new(),
            formatted_equation: "0 has no prime decomposition".to_string(),
            latex_equation: "0".to_string(),
            classification_summary: vec!["0 is neither prime nor composite".to_string()],
        };
    }
    if abs_n == 1 {
        return PrimeDecompositionResult {
            number_system: "Integers (ℤ)".to_string(),
            unit_part: unit.to_string(),
            factors: Vec::new(),
            formatted_equation: format!("{} = {} (unit)", n, unit),
            latex_equation: format!("{} = {}", n, unit),
            classification_summary: vec![format!("{} is a unit in ℤ", unit)],
        };
    }

    let raw_factors = integer_prime_factors(abs_n);
    let mut factors = Vec::new();
    let mut summary = Vec::new();
    let mut eq_parts = Vec::new();
    let mut latex_parts = Vec::new();

    if n < 0 {
        eq_parts.push("-1".to_string());
        latex_parts.push("-1".to_string());
    }

    for (p, exp) in raw_factors {
        let classes = classify_rational_prime(p);
        let class_desc = classes
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join("; ");
        summary.push(format!("{}^{}: {}", p, exp, class_desc));

        if exp == 1 {
            eq_parts.push(p.to_string());
            latex_parts.push(p.to_string());
        } else {
            eq_parts.push(format!("{}^{}", p, exp));
            latex_parts.push(format!("{}^{{{}}}", p, exp));
        }

        factors.push(PrimeFactor {
            factor_repr: p.to_string(),
            exponent: exp as i32,
            norm_or_val: Some(p as f64),
            classifications: classes,
        });
    }

    let formatted_equation = format!("{} = {}", n, eq_parts.join(" * "));
    let latex_equation = format!("{} = {}", n, latex_parts.join(" \\cdot "));

    PrimeDecompositionResult {
        number_system: "Integers (ℤ)".to_string(),
        unit_part: unit.to_string(),
        factors,
        formatted_equation,
        latex_equation,
        classification_summary: summary,
    }
}

/// Decompose a rational number $q = \frac{a}{b} \in \mathbb{Q}$ into signed prime powers.
pub fn decompose_rational(num: i64, den: i64) -> PrimeDecompositionResult {
    if den == 0 {
        return PrimeDecompositionResult {
            number_system: "Rationals (ℚ)".to_string(),
            unit_part: "undefined".to_string(),
            factors: Vec::new(),
            formatted_equation: "Division by zero".to_string(),
            latex_equation: "\\text{undefined}".to_string(),
            classification_summary: vec!["Division by zero is undefined".to_string()],
        };
    }

    let sign = if (num < 0) ^ (den < 0) { -1 } else { 1 };
    let unit = if sign < 0 { "-1" } else { "1" };
    let abs_num = num.unsigned_abs();
    let abs_den = den.unsigned_abs();

    let num_factors = integer_prime_factors(abs_num);
    let den_factors = integer_prime_factors(abs_den);

    let mut combined_map: std::collections::BTreeMap<u64, i32> = std::collections::BTreeMap::new();
    for (p, exp) in num_factors {
        *combined_map.entry(p).or_default() += exp as i32;
    }
    for (p, exp) in den_factors {
        *combined_map.entry(p).or_default() -= exp as i32;
    }

    let mut factors = Vec::new();
    let mut summary = Vec::new();
    let mut eq_parts = Vec::new();
    let mut latex_parts = Vec::new();

    if sign < 0 {
        eq_parts.push("-1".to_string());
        latex_parts.push("-1".to_string());
    }

    for (p, exp) in combined_map {
        if exp == 0 {
            continue;
        }
        let mut classes = classify_rational_prime(p);
        if exp > 0 {
            classes.insert(0, PrimeClassification::RationalNumeratorPrime);
        } else {
            classes.insert(0, PrimeClassification::RationalDenominatorPrime);
        }

        let class_desc = classes
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join("; ");
        summary.push(format!("{}^{}: {}", p, exp, class_desc));

        if exp == 1 {
            eq_parts.push(p.to_string());
            latex_parts.push(p.to_string());
        } else {
            eq_parts.push(format!("{}^{}", p, exp));
            latex_parts.push(format!("{}^{{{}}}", p, exp));
        }

        factors.push(PrimeFactor {
            factor_repr: p.to_string(),
            exponent: exp,
            norm_or_val: Some(p as f64),
            classifications: classes,
        });
    }

    let eq_rhs = if eq_parts.is_empty() {
        "1".to_string()
    } else {
        eq_parts.join(" * ")
    };
    let latex_rhs = if latex_parts.is_empty() {
        "1".to_string()
    } else {
        latex_parts.join(" \\cdot ")
    };

    let formatted_equation = format!("{}/{} = {}", num, den, eq_rhs);
    let latex_equation = format!("\\frac{{{}}}{{{}}} = {}", num, den, latex_rhs);

    PrimeDecompositionResult {
        number_system: "Rationals (ℚ)".to_string(),
        unit_part: unit.to_string(),
        factors,
        formatted_equation,
        latex_equation,
        classification_summary: summary,
    }
}

/// Gaussian integer division $\alpha / \beta$ in $\mathbb{Z}\[i\]$. Returns quotient if exact division.
pub fn gaussian_div_exact(a: i64, b: i64, c: i64, d: i64) -> Option<(i64, i64)> {
    let norm = c * c + d * d;
    if norm == 0 {
        return None;
    }
    let num_re = a * c + b * d;
    let num_im = b * c - a * d;
    if num_re % norm == 0 && num_im % norm == 0 {
        Some((num_re / norm, num_im / norm))
    } else {
        None
    }
}

/// Decompose a Gaussian integer $\alpha = a + bi \in \mathbb{Z}\[i\]$ into Gaussian primes with classification.
pub fn decompose_gaussian(mut a: i64, mut b: i64) -> PrimeDecompositionResult {
    let original = if b == 0 {
        format!("{}", a)
    } else if a == 0 {
        format!("{}i", b)
    } else if b > 0 {
        format!("{} + {}i", a, b)
    } else {
        format!("{} - {}i", a, -b)
    };

    let norm = a * a + b * b;
    if norm == 0 {
        return PrimeDecompositionResult {
            number_system: "Gaussian Integers (ℤ[i])".to_string(),
            unit_part: "0".to_string(),
            factors: Vec::new(),
            formatted_equation: "0 has no prime decomposition".to_string(),
            latex_equation: "0".to_string(),
            classification_summary: vec!["0 is zero in ℤ[i]".to_string()],
        };
    }
    if norm == 1 {
        let unit_str = match (a, b) {
            (1, 0) => "1",
            (-1, 0) => "-1",
            (0, 1) => "i",
            (0, -1) => "-i",
            _ => "unit",
        };
        return PrimeDecompositionResult {
            number_system: "Gaussian Integers (ℤ[i])".to_string(),
            unit_part: unit_str.to_string(),
            factors: Vec::new(),
            formatted_equation: format!("{} is a unit in ℤ[i]", original),
            latex_equation: format!("{} \\in \\mathbb{{Z}}[i]^\\times", original),
            classification_summary: vec![format!("{} is a unit in ℤ[i] (norm 1)", original)],
        };
    }

    let norm_factors = integer_prime_factors(norm as u64);
    let mut factors = Vec::new();
    let mut summary = Vec::new();
    let mut factor_strs = Vec::new();

    for (p, _) in norm_factors {
        if p == 2 {
            // Factor out ramified prime (1 + i)
            let mut count = 0;
            while let Some((qa, qb)) = gaussian_div_exact(a, b, 1, 1) {
                a = qa;
                b = qb;
                count += 1;
            }
            if count > 0 {
                let classes = vec![PrimeClassification::GaussianRamifiedPrime];
                summary.push(format!("(1 + i)^{}: {}", count, classes[0]));
                factor_strs.push(if count == 1 {
                    "(1 + i)".to_string()
                } else {
                    format!("(1 + i)^{}", count)
                });
                factors.push(PrimeFactor {
                    factor_repr: "(1 + i)".to_string(),
                    exponent: count,
                    norm_or_val: Some(2.0),
                    classifications: classes,
                });
            }
        } else if p % 4 == 1 {
            // Splits into (u + vi)(u - vi)
            if let Some((u, v)) = find_sum_of_two_squares(p) {
                // Try (u + vi)
                let mut count1 = 0;
                while let Some((qa, qb)) = gaussian_div_exact(a, b, u, v) {
                    a = qa;
                    b = qb;
                    count1 += 1;
                }
                if count1 > 0 {
                    let classes = vec![PrimeClassification::GaussianSplitPrime];
                    let repr = format!("({} + {}i)", u, v);
                    summary.push(format!("{}^{}: {}", repr, count1, classes[0]));
                    factor_strs.push(if count1 == 1 {
                        repr.clone()
                    } else {
                        format!("{}^{}", repr, count1)
                    });
                    factors.push(PrimeFactor {
                        factor_repr: repr,
                        exponent: count1,
                        norm_or_val: Some(p as f64),
                        classifications: classes,
                    });
                }

                // Try (u - vi)
                let mut count2 = 0;
                while let Some((qa, qb)) = gaussian_div_exact(a, b, u, -v) {
                    a = qa;
                    b = qb;
                    count2 += 1;
                }
                if count2 > 0 {
                    let classes = vec![PrimeClassification::GaussianSplitPrime];
                    let repr = format!("({} - {}i)", u, v);
                    summary.push(format!("{}^{}: {}", repr, count2, classes[0]));
                    factor_strs.push(if count2 == 1 {
                        repr.clone()
                    } else {
                        format!("{}^{}", repr, count2)
                    });
                    factors.push(PrimeFactor {
                        factor_repr: repr,
                        exponent: count2,
                        norm_or_val: Some(p as f64),
                        classifications: classes,
                    });
                }
            }
        } else if p % 4 == 3 {
            // Inert rational prime p
            let p_i64 = p as i64;
            let mut count = 0;
            while let Some((qa, qb)) = gaussian_div_exact(a, b, p_i64, 0) {
                a = qa;
                b = qb;
                count += 1;
            }
            if count > 0 {
                let classes = vec![PrimeClassification::GaussianInertPrime];
                summary.push(format!("{}^{}: {}", p, count, classes[0]));
                factor_strs.push(if count == 1 {
                    p.to_string()
                } else {
                    format!("{}^{}", p, count)
                });
                factors.push(PrimeFactor {
                    factor_repr: p.to_string(),
                    exponent: count,
                    norm_or_val: Some((p * p) as f64),
                    classifications: classes,
                });
            }
        }
    }

    let unit_part = match (a, b) {
        (1, 0) => "1",
        (-1, 0) => "-1",
        (0, 1) => "i",
        (0, -1) => "-i",
        _ => "1",
    };

    let mut all_factors_str = Vec::new();
    if unit_part != "1" {
        all_factors_str.push(unit_part.to_string());
    }
    all_factors_str.extend(factor_strs);
    let rhs = if all_factors_str.is_empty() {
        "1".to_string()
    } else {
        all_factors_str.join(" * ")
    };
    let formatted_equation = format!("{} = {}", original, rhs);
    let latex_equation = format!("{} = {}", original, rhs.replace('*', "\\cdot"));

    PrimeDecompositionResult {
        number_system: "Gaussian Integers (ℤ[i])".to_string(),
        unit_part: unit_part.to_string(),
        factors,
        formatted_equation,
        latex_equation,
        classification_summary: summary,
    }
}

/// Eisenstein integer division $\alpha / \beta$ in $\mathbb{Z}[\omega]$ where $\omega = \frac{-1+\sqrt{3}i}{2}$.
pub fn eisenstein_div_exact(a: i64, b: i64, c: i64, d: i64) -> Option<(i64, i64)> {
    let norm = c * c - c * d + d * d;
    if norm == 0 {
        return None;
    }
    let num_a = a * c - a * d + b * d;
    let num_b = b * c - a * d;
    if num_a % norm == 0 && num_b % norm == 0 {
        Some((num_a / norm, num_b / norm))
    } else {
        None
    }
}

/// Decompose an Eisenstein integer $\alpha = a + b\omega \in \mathbb{Z}[\omega]$ into Eisenstein primes with classification.
pub fn decompose_eisenstein(mut a: i64, mut b: i64) -> PrimeDecompositionResult {
    let original = if b == 0 {
        format!("{}", a)
    } else if a == 0 {
        format!("{}ω", b)
    } else if b > 0 {
        format!("{} + {}ω", a, b)
    } else {
        format!("{} - {}ω", a, -b)
    };

    let norm = a * a - a * b + b * b;
    if norm == 0 {
        return PrimeDecompositionResult {
            number_system: "Eisenstein Integers (ℤ[ω])".to_string(),
            unit_part: "0".to_string(),
            factors: Vec::new(),
            formatted_equation: "0 has no prime decomposition".to_string(),
            latex_equation: "0".to_string(),
            classification_summary: vec!["0 is zero in ℤ[ω]".to_string()],
        };
    }
    if norm == 1 {
        return PrimeDecompositionResult {
            number_system: "Eisenstein Integers (ℤ[ω])".to_string(),
            unit_part: original.clone(),
            factors: Vec::new(),
            formatted_equation: format!("{} is a unit in ℤ[ω]", original),
            latex_equation: format!("{} \\in \\mathbb{{Z}}[\\omega]^\\times", original),
            classification_summary: vec![format!("{} is a unit in ℤ[ω] (norm 1)", original)],
        };
    }

    let norm_factors = integer_prime_factors(norm as u64);
    let mut factors = Vec::new();
    let mut summary = Vec::new();
    let mut factor_strs = Vec::new();

    for (p, _) in norm_factors {
        if p == 3 {
            // Factor out ramified prime 1 - ω
            let mut count = 0;
            while let Some((qa, qb)) = eisenstein_div_exact(a, b, 1, -1) {
                a = qa;
                b = qb;
                count += 1;
            }
            if count > 0 {
                let classes = vec![PrimeClassification::EisensteinRamifiedPrime];
                summary.push(format!("(1 - ω)^{}: {}", count, classes[0]));
                factor_strs.push(if count == 1 {
                    "(1 - ω)".to_string()
                } else {
                    format!("(1 - ω)^{}", count)
                });
                factors.push(PrimeFactor {
                    factor_repr: "(1 - ω)".to_string(),
                    exponent: count,
                    norm_or_val: Some(3.0),
                    classifications: classes,
                });
            }
        } else if p % 3 == 1 {
            // Splits into (u + vω)(u + vω²)
            if let Some((u, v)) = find_eisenstein_prime_coords(p) {
                // Try (u + vω)
                let mut count1 = 0;
                while let Some((qa, qb)) = eisenstein_div_exact(a, b, u, v) {
                    a = qa;
                    b = qb;
                    count1 += 1;
                }
                if count1 > 0 {
                    let classes = vec![PrimeClassification::EisensteinSplitPrime];
                    let repr = format!("({} + {}ω)", u, v);
                    summary.push(format!("{}^{}: {}", repr, count1, classes[0]));
                    factor_strs.push(if count1 == 1 {
                        repr.clone()
                    } else {
                        format!("{}^{}", repr, count1)
                    });
                    factors.push(PrimeFactor {
                        factor_repr: repr,
                        exponent: count1,
                        norm_or_val: Some(p as f64),
                        classifications: classes,
                    });
                }

                // Try (u - v) - vω (conjugate)
                let (cu, cv) = (u - v, -v);
                let mut count2 = 0;
                while let Some((qa, qb)) = eisenstein_div_exact(a, b, cu, cv) {
                    a = qa;
                    b = qb;
                    count2 += 1;
                }
                if count2 > 0 {
                    let classes = vec![PrimeClassification::EisensteinSplitPrime];
                    let repr = if cv >= 0 {
                        format!("({} + {}ω)", cu, cv)
                    } else {
                        format!("({} - {}ω)", cu, -cv)
                    };
                    summary.push(format!("{}^{}: {}", repr, count2, classes[0]));
                    factor_strs.push(if count2 == 1 {
                        repr.clone()
                    } else {
                        format!("{}^{}", repr, count2)
                    });
                    factors.push(PrimeFactor {
                        factor_repr: repr,
                        exponent: count2,
                        norm_or_val: Some(p as f64),
                        classifications: classes,
                    });
                }
            }
        } else if p % 3 == 2 {
            // Inert rational prime p
            let p_i64 = p as i64;
            let mut count = 0;
            while let Some((qa, qb)) = eisenstein_div_exact(a, b, p_i64, 0) {
                a = qa;
                b = qb;
                count += 1;
            }
            if count > 0 {
                let classes = vec![PrimeClassification::EisensteinInertPrime];
                summary.push(format!("{}^{}: {}", p, count, classes[0]));
                factor_strs.push(if count == 1 {
                    p.to_string()
                } else {
                    format!("{}^{}", p, count)
                });
                factors.push(PrimeFactor {
                    factor_repr: p.to_string(),
                    exponent: count,
                    norm_or_val: Some((p * p) as f64),
                    classifications: classes,
                });
            }
        }
    }

    let unit_part = if (a, b) == (1, 0) {
        "1"
    } else if (a, b) == (-1, 0) {
        "-1"
    } else if (a, b) == (0, 1) {
        "ω"
    } else if (a, b) == (0, -1) {
        "-ω"
    } else if (a, b) == (-1, -1) {
        "ω²"
    } else if (a, b) == (1, 1) {
        "-ω²"
    } else {
        "1"
    };

    let mut all_factors_str = Vec::new();
    if unit_part != "1" {
        all_factors_str.push(unit_part.to_string());
    }
    all_factors_str.extend(factor_strs);
    let rhs = if all_factors_str.is_empty() {
        "1".to_string()
    } else {
        all_factors_str.join(" * ")
    };
    let formatted_equation = format!("{} = {}", original, rhs);
    let latex_equation = format!("{} = {}", original, rhs.replace('*', "\\cdot"));

    PrimeDecompositionResult {
        number_system: "Eisenstein Integers (ℤ[ω])".to_string(),
        unit_part: unit_part.to_string(),
        factors,
        formatted_equation,
        latex_equation,
        classification_summary: summary,
    }
}

/// Decompose an element $x$ in a modular ring $\mathbb{Z}/n\mathbb{Z}$ into ring-theoretic and prime components.
pub fn decompose_modulo(val: i64, n: u64) -> PrimeDecompositionResult {
    let mod_n = n.max(2);
    let residue = val.rem_euclid(mod_n as i64) as u64;
    let (gcd_val, _, _) = extended_gcd(residue as i64, mod_n as i64);
    let gcd_u = gcd_val as u64;

    let crt_primes = integer_prime_factors(mod_n);
    let crt_desc = crt_primes
        .iter()
        .map(|(p, e)| format!("ℤ/{}^{}ℤ", p, e))
        .collect::<Vec<_>>()
        .join(" ⊕ ");

    let mut classes = Vec::new();
    if gcd_u == 1 {
        // Multiplicative order in (Z/nZ)*
        let mut ord = 1u64;
        let mut cur = residue % mod_n;
        while cur != 1 && ord <= mod_n {
            cur = ((cur as u128 * residue as u128) % mod_n as u128) as u64;
            ord += 1;
        }
        classes.push(PrimeClassification::ModularUnit { order: ord });
    } else {
        classes.push(PrimeClassification::ModularZeroDivisor { gcd: gcd_u });

        // Check nilpotent: every prime dividing n must divide residue
        let is_nilpotent = crt_primes.iter().all(|(p, _)| residue.is_multiple_of(*p));
        if is_nilpotent {
            let mut idx = 1;
            let mut pow_val = residue;
            while !pow_val.is_multiple_of(mod_n) && idx < 32 {
                pow_val = ((pow_val as u128 * residue as u128) % mod_n as u128) as u64;
                idx += 1;
            }
            classes.push(PrimeClassification::ModularNilpotent { index: idx });
        }

        // Check idempotent: x^2 == x (mod n)
        if (residue as u128 * residue as u128) % mod_n as u128 == residue as u128 {
            classes.push(PrimeClassification::ModularIdempotent);
        }

        // Check prime ideal generator
        if is_prime(gcd_u) {
            classes.push(PrimeClassification::ModularPrimeIdeal);
        }
    }

    let mut summary = Vec::new();
    summary.push(format!(
        "Ring CRT Primary Decomposition: ℤ/{}ℤ ≅ {}",
        mod_n, crt_desc
    ));
    for c in &classes {
        summary.push(c.to_string());
    }

    let gcd_factors = integer_prime_factors(gcd_u);
    let mut factors = Vec::new();
    for (p, exp) in gcd_factors {
        factors.push(PrimeFactor {
            factor_repr: p.to_string(),
            exponent: exp as i32,
            norm_or_val: Some(p as f64),
            classifications: classify_rational_prime(p),
        });
    }

    let status_str = if gcd_u == 1 {
        "Invertible Unit"
    } else {
        "Zero Divisor"
    };
    let formatted_equation = format!(
        "{} ≡ {} (mod {}): {} with gcd({}, {}) = {}",
        val, residue, mod_n, status_str, residue, mod_n, gcd_u
    );
    let latex_equation = format!(
        "{} \\equiv {} \\pmod{{{}}} \\quad [\\gcd={}, {}]",
        val, residue, mod_n, gcd_u, status_str
    );

    PrimeDecompositionResult {
        number_system: format!("Modular Ring (ℤ/{}ℤ)", mod_n),
        unit_part: if gcd_u == 1 {
            "Unit".to_string()
        } else {
            "Zero-Divisor".to_string()
        },
        factors,
        formatted_equation,
        latex_equation,
        classification_summary: summary,
    }
}

/// Decompose a number in the $p$-adic field $\mathbb{Q}_p$ into uniformizer prime $p$ and $p$-adic unit.
pub fn decompose_padic(val: i64, p: u32) -> PrimeDecompositionResult {
    let prime = p.max(2) as u64;
    if val == 0 {
        return PrimeDecompositionResult {
            number_system: format!("p-Adic Field (ℚ_{})", prime),
            unit_part: "0".to_string(),
            factors: Vec::new(),
            formatted_equation: "v_p(0) = +∞".to_string(),
            latex_equation: format!("v_{{{}}}(0) = +\\infty", prime),
            classification_summary: vec!["0 has infinite p-adic valuation".to_string()],
        };
    }

    let mut temp = val.unsigned_abs();
    let mut valuation = 0i32;
    while temp.is_multiple_of(prime) {
        valuation += 1;
        temp /= prime;
    }
    let unit_val = if val < 0 { -(temp as i64) } else { temp as i64 };

    let mut classes = vec![PrimeClassification::PadicUniformizer { valuation }];
    if valuation == 0 {
        classes.push(PrimeClassification::PadicUnit);
    }

    let summary = vec![
        format!("p-adic Base Uniformizer Prime: {}", prime),
        format!("Valuation v_{}({}) = {}", prime, val, valuation),
        format!(
            "p-adic Norm |{}|_{} = p^(-{}) = {:.6}",
            val,
            prime,
            valuation,
            (prime as f64).powi(-valuation)
        ),
        format!(
            "Decomposition: {} = {}^{} · ({}) [where |{}|_{} = 1]",
            val, prime, valuation, unit_val, unit_val, prime
        ),
    ];

    let factor = PrimeFactor {
        factor_repr: prime.to_string(),
        exponent: valuation,
        norm_or_val: Some((prime as f64).powi(-valuation)),
        classifications: classes,
    };

    let formatted_equation = format!(
        "{} = {}^{} · ({}) in ℚ_{}",
        val, prime, valuation, unit_val, prime
    );
    let latex_equation = format!(
        "{} = {}^{{{}}} \\cdot ({}) \\in \\mathbb{{Q}}_{{{}}}",
        val, prime, valuation, unit_val, prime
    );

    PrimeDecompositionResult {
        number_system: format!("p-Adic Field (ℚ_{})", prime),
        unit_part: unit_val.to_string(),
        factors: vec![factor],
        formatted_equation,
        latex_equation,
        classification_summary: summary,
    }
}

/// Decompose an element in a finite Galois field $\mathrm{GF}(p^k)$.
pub fn decompose_galois(val: u64, p: u64, power: u32) -> PrimeDecompositionResult {
    let prime = p.max(2);
    let pow = power.max(1);
    let q = prime.pow(pow);
    let residue = val % q;

    if residue == 0 {
        return PrimeDecompositionResult {
            number_system: format!("Galois Field GF({}^{})", prime, pow),
            unit_part: "0".to_string(),
            factors: Vec::new(),
            formatted_equation: "0 is the additive zero element".to_string(),
            latex_equation: "0 \\in \\mathrm{GF}(q)".to_string(),
            classification_summary: vec!["Additive zero in finite field".to_string()],
        };
    }

    // Units group order is q - 1
    let group_order = q - 1;
    let _group_factors = integer_prime_factors(group_order);

    // Find multiplicative order of element
    let mut ord = 1u64;
    let mut cur = residue % q;
    while cur != 1 && ord <= group_order {
        cur = ((cur as u128 * residue as u128) % q as u128) as u64;
        ord += 1;
    }

    let is_generator = ord == group_order;
    let mut classes = Vec::new();
    if is_generator {
        classes.push(PrimeClassification::GaloisGenerator { order: ord });
    } else {
        classes.push(PrimeClassification::GaloisSubfieldElement);
    }

    let ord_factors = integer_prime_factors(ord);
    let mut factors = Vec::new();
    for (pf, exp) in ord_factors {
        factors.push(PrimeFactor {
            factor_repr: pf.to_string(),
            exponent: exp as i32,
            norm_or_val: Some(pf as f64),
            classifications: classify_rational_prime(pf),
        });
    }

    let summary = vec![
        format!("Characteristic Prime: {}", prime),
        format!("Field Order: q = {}^{} = {}", prime, pow, q),
        format!("Multiplicative Group Order: |GF(q)*| = {}", group_order),
        format!(
            "Element {} Multiplicative Order: ord({}) = {}",
            residue, residue, ord
        ),
        if is_generator {
            format!("{} is a Primitive Generator of GF({}*)", residue, q)
        } else {
            format!("{} generates a proper subgroup of order {}", residue, ord)
        },
    ];

    let formatted_equation = format!(
        "ord({}) = {} dividing (q - 1 = {}) in GF({}^{})",
        residue, ord, group_order, prime, pow
    );
    let latex_equation = format!(
        "\\mathrm{{ord}}({}) = {} \\mid (q - 1 = {}) \\in \\mathrm{{GF}}({}^{{{}}})",
        residue, ord, group_order, prime, pow
    );

    PrimeDecompositionResult {
        number_system: format!("Galois Field GF({}^{})", prime, pow),
        unit_part: residue.to_string(),
        factors,
        formatted_equation,
        latex_equation,
        classification_summary: summary,
    }
}

/// Helper to parse standard Gaussian integer string e.g. "3 + 4i", "3 - 4i", "5", "2i", "-i".
pub fn parse_gaussian_str(s: &str) -> Option<(i64, i64)> {
    let clean = s.trim().replace(' ', "");
    if clean.is_empty() {
        return None;
    }
    if clean == "i" {
        return Some((0, 1));
    }
    if clean == "-i" {
        return Some((0, -1));
    }
    if !clean.contains('i') {
        return clean.parse::<i64>().ok().map(|a| (a, 0));
    }

    // Has 'i'
    let without_i = clean.trim_end_matches('i');
    if without_i.is_empty() {
        return Some((0, 1));
    }
    if without_i == "-" {
        return Some((0, -1));
    }
    if without_i == "+" {
        return Some((0, 1));
    }

    // Split on last '+' or '-'
    let last_plus = without_i.rfind('+');
    let last_minus = without_i.rfind('-');

    match (last_plus, last_minus) {
        (Some(p), None) => {
            let re_str = &without_i[..p];
            let im_str = &without_i[p + 1..];
            let re = if re_str.is_empty() {
                0
            } else {
                re_str.parse::<i64>().ok()?
            };
            let im = if im_str.is_empty() {
                1
            } else {
                im_str.parse::<i64>().ok()?
            };
            Some((re, im))
        }
        (None, Some(0)) => {
            // Pure imaginary negative e.g. "-2i"
            let im = if without_i == "-" {
                -1
            } else {
                without_i.parse::<i64>().ok()?
            };
            Some((0, im))
        }
        (None, Some(m)) => {
            let re_str = &without_i[..m];
            let im_str = &without_i[m + 1..];
            let re = re_str.parse::<i64>().ok()?;
            let im = if im_str.is_empty() {
                -1
            } else {
                -im_str.parse::<i64>().ok()?
            };
            Some((re, im))
        }
        (Some(p), Some(m)) => {
            let split_idx = p.max(m);
            if split_idx == 0 {
                let im = without_i.parse::<i64>().ok()?;
                return Some((0, im));
            }
            let re_str = &without_i[..split_idx];
            let im_str = &without_i[split_idx..];
            let re = re_str.parse::<i64>().ok()?;
            let im = if im_str == "+" {
                1
            } else if im_str == "-" {
                -1
            } else {
                im_str.parse::<i64>().ok()?
            };
            Some((re, im))
        }
        (None, None) => {
            let im = without_i.parse::<i64>().ok()?;
            Some((0, im))
        }
    }
}

/// Helper to parse standard Eisenstein integer string e.g. "3 + 4w", "3 + 4ω", "5", "w", "-ω".
pub fn parse_eisenstein_str(s: &str) -> Option<(i64, i64)> {
    let clean = s.trim().replace(' ', "").replace('ω', "w");
    if clean.is_empty() {
        return None;
    }
    if clean == "w" {
        return Some((0, 1));
    }
    if clean == "-w" {
        return Some((0, -1));
    }
    if !clean.contains('w') {
        return clean.parse::<i64>().ok().map(|a| (a, 0));
    }

    let without_w = clean.trim_end_matches('w');
    if without_w.is_empty() {
        return Some((0, 1));
    }
    if without_w == "-" {
        return Some((0, -1));
    }
    if without_w == "+" {
        return Some((0, 1));
    }

    let last_plus = without_w.rfind('+');
    let last_minus = without_w.rfind('-');

    match (last_plus, last_minus) {
        (Some(p), None) => {
            let re_str = &without_w[..p];
            let im_str = &without_w[p + 1..];
            let re = if re_str.is_empty() {
                0
            } else {
                re_str.parse::<i64>().ok()?
            };
            let im = if im_str.is_empty() {
                1
            } else {
                im_str.parse::<i64>().ok()?
            };
            Some((re, im))
        }
        (None, Some(0)) => {
            let im = without_w.parse::<i64>().ok()?;
            Some((0, im))
        }
        (None, Some(m)) => {
            let re_str = &without_w[..m];
            let im_str = &without_w[m + 1..];
            let re = re_str.parse::<i64>().ok()?;
            let im = if im_str.is_empty() {
                -1
            } else {
                -im_str.parse::<i64>().ok()?
            };
            Some((re, im))
        }
        (Some(p), Some(m)) => {
            let split_idx = p.max(m);
            if split_idx == 0 {
                let im = without_w.parse::<i64>().ok()?;
                return Some((0, im));
            }
            let re_str = &without_w[..split_idx];
            let im_str = &without_w[split_idx..];
            let re = re_str.parse::<i64>().ok()?;
            let im = if im_str == "+" {
                1
            } else if im_str == "-" {
                -1
            } else {
                im_str.parse::<i64>().ok()?
            };
            Some((re, im))
        }
        (None, None) => {
            let im = without_w.parse::<i64>().ok()?;
            Some((0, im))
        }
    }
}

/// Universal prime decomposition entry point for any number system.
pub fn decompose_universal(
    expr: &str,
    domain_hint: Option<&str>,
) -> Result<PrimeDecompositionResult, String> {
    let clean_expr = expr.trim();
    let hint_str = domain_hint.map(|h| h.trim()).unwrap_or("");

    // 1. Explicit domain hint or domain keyword in hint
    if !hint_str.is_empty() {
        if hint_str.starts_with("Gaussian")
            || hint_str.starts_with("ℤ[i]")
            || hint_str.starts_with("Z[i]")
        {
            let (a, b) = parse_gaussian_str(clean_expr).ok_or_else(|| {
                format!(
                    "Could not parse '{}' as Gaussian integer a + bi",
                    clean_expr
                )
            })?;
            return Ok(decompose_gaussian(a, b));
        }
        if hint_str.starts_with("Eisenstein")
            || hint_str.starts_with("ℤ[ω]")
            || hint_str.starts_with("Z[w]")
            || hint_str.starts_with("Z[omega]")
        {
            let (a, b) = parse_eisenstein_str(clean_expr).ok_or_else(|| {
                format!(
                    "Could not parse '{}' as Eisenstein integer a + bω",
                    clean_expr
                )
            })?;
            return Ok(decompose_eisenstein(a, b));
        }
        if hint_str.starts_with("Modulo")
            || hint_str.starts_with("ℤ/")
            || hint_str.starts_with("Z/")
        {
            let n = if let Some(open) = hint_str.find('(') {
                let inner = &hint_str[open + 1..hint_str.find(')').unwrap_or(hint_str.len())];
                let num_str = inner.split('=').next_back().unwrap_or("2").trim();
                num_str.parse::<u64>().unwrap_or(2)
            } else if let Some((_, n_s)) = hint_str.split_once('/') {
                n_s.trim_end_matches('Z')
                    .trim_end_matches('ℤ')
                    .trim()
                    .parse::<u64>()
                    .unwrap_or(2)
            } else {
                2
            };
            let val = clean_expr
                .parse::<i64>()
                .map_err(|e| format!("Invalid integer residue for Modulo: {}", e))?;
            return Ok(decompose_modulo(val, n));
        }
        if hint_str.starts_with("PAdics")
            || hint_str.starts_with("Padics")
            || hint_str.starts_with("ℚ_")
            || hint_str.starts_with("Q_")
        {
            let p = if let Some(open) = hint_str.find('(') {
                let inner = &hint_str[open + 1..hint_str.find(')').unwrap_or(hint_str.len())];
                let p_s = inner.split('=').next_back().unwrap_or("2").trim();
                p_s.parse::<u32>().unwrap_or(2)
            } else if let Some((_, p_s)) = hint_str.split_once('_') {
                p_s.trim().parse::<u32>().unwrap_or(2)
            } else {
                2
            };
            let val = clean_expr
                .parse::<i64>()
                .map_err(|e| format!("Invalid value for p-adic decomposition: {}", e))?;
            return Ok(decompose_padic(val, p));
        }
        if hint_str.starts_with("GaloisField")
            || hint_str.starts_with("GF")
            || hint_str.starts_with("𝔽")
        {
            let (p, power) = if let Some(open) = hint_str.find('(') {
                let inner = &hint_str[open + 1..hint_str.find(')').unwrap_or(hint_str.len())];
                let parts: Vec<&str> = inner.split(',').collect();
                let p_val = parts
                    .first()
                    .and_then(|s| s.split('=').next_back())
                    .and_then(|s| s.trim().parse::<u64>().ok())
                    .unwrap_or(2);
                let pow_val = parts
                    .get(1)
                    .and_then(|s| s.split('=').next_back())
                    .and_then(|s| s.trim().parse::<u32>().ok())
                    .unwrap_or(1);
                (p_val, pow_val)
            } else {
                (2, 1)
            };
            let val = clean_expr
                .parse::<u64>()
                .map_err(|e| format!("Invalid value for Galois field decomposition: {}", e))?;
            return Ok(decompose_galois(val, p, power));
        }
        if hint_str.starts_with("Rationals")
            || hint_str.starts_with("ℚ")
            || hint_str.starts_with("Q")
        {
            if let Some((n_s, d_s)) = clean_expr.split_once('/') {
                let num = n_s
                    .trim()
                    .parse::<i64>()
                    .map_err(|e| format!("Invalid rational numerator: {}", e))?;
                let den = d_s
                    .trim()
                    .parse::<i64>()
                    .map_err(|e| format!("Invalid rational denominator: {}", e))?;
                return Ok(decompose_rational(num, den));
            } else {
                let num = clean_expr
                    .parse::<i64>()
                    .map_err(|e| format!("Invalid rational integer: {}", e))?;
                return Ok(decompose_rational(num, 1));
            }
        }
    }

    // 2. Automatic domain inference based on expression syntax
    if clean_expr.contains('i')
        && let Some((a, b)) = parse_gaussian_str(clean_expr)
    {
        return Ok(decompose_gaussian(a, b));
    }
    if (clean_expr.contains('w') || clean_expr.contains('ω'))
        && let Some((a, b)) = parse_eisenstein_str(clean_expr)
    {
        return Ok(decompose_eisenstein(a, b));
    }
    if clean_expr.contains('/')
        && let Some((n_s, d_s)) = clean_expr.split_once('/')
        && let (Ok(num), Ok(den)) = (n_s.trim().parse::<i64>(), d_s.trim().parse::<i64>())
    {
        return Ok(decompose_rational(num, den));
    }

    // 3. Default to integers (arbitrary-precision BigInt supported)
    let n = clean_expr.parse::<num_bigint::BigInt>().map_err(|_| {
        format!(
            "Could not parse '{}' as an integer or algebraic number for prime decomposition",
            clean_expr
        )
    })?;
    Ok(decompose_bigint(&n))
}
