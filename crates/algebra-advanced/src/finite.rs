//! # `algebra-finite`
//!
//! Finite Fields (Galois Fields GF(p^n)) and Modular Arithmetic (ℤ/nℤ).

use num_bigint::BigInt;
use num_traits::{One, Zero};

/// Modulo arithmetic integer element in ℤ/nℤ.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuloInt {
    /// Remainder value in range [0, modulus - 1].
    pub value: BigInt,
    /// Modulus n.
    pub modulus: BigInt,
}

impl ModuloInt {
    /// Create a new modulo integer carrying modulus safely.
    pub fn new(value: BigInt, modulus: BigInt) -> Self {
        let mut rem = value % &modulus;
        if rem < BigInt::zero() {
            rem += &modulus;
        }
        Self {
            value: rem,
            modulus,
        }
    }

    /// Add two modulo integers in same ℤ/nℤ.
    pub fn add(&self, other: &ModuloInt) -> Option<Self> {
        if self.modulus != other.modulus {
            return None;
        }
        Some(Self::new(&self.value + &other.value, self.modulus.clone()))
    }

    /// Multiply two modulo integers in same ℤ/nℤ.
    pub fn mul(&self, other: &ModuloInt) -> Option<Self> {
        if self.modulus != other.modulus {
            return None;
        }
        Some(Self::new(&self.value * &other.value, self.modulus.clone()))
    }

    /// Modular exponentiation (base^exp mod n) using fast binary exponentiation.
    pub fn pow(&self, exp: &BigInt) -> Self {
        let mut base = self.value.clone();
        let mut e = exp.clone();
        let mut result = BigInt::one();

        base %= &self.modulus;
        while e > BigInt::zero() {
            if &e % 2 == BigInt::one() {
                result = (result * &base) % &self.modulus;
            }
            e >>= 1;
            base = (&base * &base) % &self.modulus;
        }
        Self {
            value: result,
            modulus: self.modulus.clone(),
        }
    }
}

/// Galois Field GF(p^n) representation defined by irreducible modulus polynomial coefficients.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GaloisField {
    /// Characteristic prime p.
    pub p: u32,
    /// Field extension degree n.
    pub degree: u32,
    /// Irreducible polynomial coefficients (lowest to highest degree), e.g. x^2 + 1 -> [1, 0, 1].
    pub modulus_poly: Option<Vec<i64>>,
}

impl GaloisField {
    /// Create a new finite Galois field GF(p^n) without explicit polynomial modulus.
    pub fn new(p: u32, degree: u32) -> Self {
        Self {
            p,
            degree,
            modulus_poly: None,
        }
    }

    /// Create a new finite Galois field GF(p^n) with explicit irreducible polynomial modulus.
    pub fn with_modulus(p: u32, degree: u32, modulus_poly: Vec<i64>) -> Self {
        Self {
            p,
            degree,
            modulus_poly: Some(modulus_poly),
        }
    }

    /// Construct GF(p^2) from an inert Gaussian prime $p \equiv 3 \pmod 4$ with modulus $x^2 + 1$.
    pub fn from_gaussian_inert_prime(p: u32) -> Option<Self> {
        if p % 4 == 3 && algebra_engine::numbertheory::is_prime(p as u64) {
            Some(Self::with_modulus(p, 2, vec![1, 0, 1]))
        } else {
            None
        }
    }

    /// Construct GF(p^2) from an inert Eisenstein prime $p \equiv 2 \pmod 3$ with modulus $x^2 - x + 1$.
    pub fn from_eisenstein_inert_prime(p: u32) -> Option<Self> {
        if p % 3 == 2 && algebra_engine::numbertheory::is_prime(p as u64) {
            Some(Self::with_modulus(p, 2, vec![1, -1, 1]))
        } else {
            None
        }
    }

    /// Total order (cardinality) of the field p^n.
    pub fn cardinality(&self) -> u64 {
        (self.p as u64).pow(self.degree)
    }
}

impl ModuloInt {
    /// Like [`ModuloInt::new`] but returns `None` for a modulus below 2 instead of panicking on a zero modulus.
    pub fn checked_new(value: BigInt, modulus: BigInt) -> Option<Self> {
        (modulus > BigInt::one()).then(|| Self::new(value, modulus))
    }

    /// Subtract two modulo integers in the same $\mathbb{Z}/n\mathbb{Z}$.
    pub fn sub(&self, other: &ModuloInt) -> Option<Self> {
        if self.modulus != other.modulus {
            return None;
        }
        Some(Self::new(&self.value - &other.value, self.modulus.clone()))
    }

    /// The multiplicative inverse, if the value is a unit (coprime to the modulus).
    pub fn inverse(&self) -> Option<Self> {
        use num::Integer;
        let e = self.value.extended_gcd(&self.modulus);
        e.gcd.is_one().then(|| Self::new(e.x, self.modulus.clone()))
    }

    /// Division by a unit: `self * other^-1`.
    pub fn div(&self, other: &ModuloInt) -> Option<Self> {
        self.mul(&other.inverse()?)
    }

    /// Exponentiation with any integer exponent; a negative exponent needs a unit base.
    pub fn checked_pow(&self, exp: &BigInt) -> Option<Self> {
        if exp < &BigInt::zero() {
            Some(self.inverse()?.pow(&-exp))
        } else {
            Some(self.pow(exp))
        }
    }
}

/// Arithmetic in $\mathbb{F}_{p^n} = \mathbb{F}_p[x] / (f)$ for a monic irreducible `f`; elements are coefficient vectors
/// of length `n`, lowest degree first, each below `p`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GfArithmetic {
    pub p: u64,
    pub degree: usize,
    /// The monic irreducible modulus, `degree + 1` coefficients.
    pub modulus: Vec<u64>,
}

fn poly_trim(mut a: Vec<u64>) -> Vec<u64> {
    while a.last() == Some(&0) {
        a.pop();
    }
    a
}

fn poly_mul(a: &[u64], b: &[u64], p: u64) -> Vec<u64> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut out = vec![0u64; a.len() + b.len() - 1];
    for (i, &x) in a.iter().enumerate() {
        for (j, &y) in b.iter().enumerate() {
            out[i + j] = (out[i + j] + x * y) % p;
        }
    }
    poly_trim(out)
}

fn inv_mod(a: u64, p: u64) -> u64 {
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

/// Remainder of `a` modulo a polynomial `f` with a non-zero leading coefficient.
fn poly_rem(a: &[u64], f: &[u64], p: u64) -> Vec<u64> {
    let mut rem = a.to_vec();
    let inv = inv_mod(*f.last().unwrap_or(&1), p);
    while rem.len() >= f.len() {
        let factor = rem[rem.len() - 1] * inv % p;
        let shift = rem.len() - f.len();
        for (j, &fj) in f.iter().enumerate() {
            rem[shift + j] = (rem[shift + j] + p - factor * fj % p) % p;
        }
        rem = poly_trim(rem);
    }
    rem
}

fn poly_gcd(a: &[u64], b: &[u64], p: u64) -> Vec<u64> {
    let (mut a, mut b) = (poly_trim(a.to_vec()), poly_trim(b.to_vec()));
    while !b.is_empty() {
        let r = poly_rem(&a, &b, p);
        a = b;
        b = r;
    }
    a
}

/// `x^(p^k) mod f`.
fn x_to_p_power(k: usize, f: &[u64], p: u64) -> Vec<u64> {
    let mut h = poly_rem(&[0, 1], f, p);
    for _ in 0..k {
        // h = h^p mod f
        let mut result = vec![1u64];
        let mut base = h.clone();
        let mut e = p;
        while e > 0 {
            if e & 1 == 1 {
                result = poly_rem(&poly_mul(&result, &base, p), f, p);
            }
            base = poly_rem(&poly_mul(&base, &base, p), f, p);
            e >>= 1;
        }
        h = result;
    }
    h
}

/// `x^(p^k) - x mod f`, trimmed.
fn x_minus_power(k: usize, f: &[u64], p: u64) -> Vec<u64> {
    let mut h = x_to_p_power(k, f, p);
    if h.len() < 2 {
        h.resize(2, 0);
    }
    h[1] = (h[1] + p - 1) % p;
    poly_trim(h)
}

/// Rabin's irreducibility test for a monic polynomial of degree `n` over $\mathbb{F}_p$.
fn is_irreducible(f: &[u64], p: u64) -> bool {
    let n = f.len() - 1;
    if n == 0 {
        return false;
    }
    if n == 1 {
        return true;
    }
    if !x_minus_power(n, f, p).is_empty() {
        return false;
    }
    let mut m = n;
    let mut q = 2;
    let mut primes = Vec::new();
    while m > 1 {
        if m.is_multiple_of(q) {
            primes.push(q);
            while m.is_multiple_of(q) {
                m /= q;
            }
        }
        q += 1;
    }
    primes
        .into_iter()
        .all(|q| poly_gcd(&x_minus_power(n / q, f, p), f, p).len() == 1)
}

impl GaloisField {
    /// The field with a checked characteristic and degree: `p` must be prime, `n >= 1`, and $p^n$ must fit in a `u64`.
    /// An irreducible modulus is found if none was given.
    pub fn arithmetic(&self) -> Option<GfArithmetic> {
        let p = self.p as u64;
        let n = self.degree as usize;
        if n == 0
            || !algebra_engine::numbertheory::is_prime(p)
            || p.checked_pow(self.degree).is_none()
        {
            return None;
        }
        let modulus = match &self.modulus_poly {
            Some(m) => {
                let reduced: Vec<u64> = m.iter().map(|&c| c.rem_euclid(p as i64) as u64).collect();
                if reduced.len() != n + 1 || reduced[n] != 1 || !is_irreducible(&reduced, p) {
                    return None;
                }
                reduced
            }
            None => find_irreducible(p, n)?,
        };
        Some(GfArithmetic {
            p,
            degree: n,
            modulus,
        })
    }
}

/// The first monic irreducible polynomial of degree `n` over $\mathbb{F}_p$ in lexicographic order of coefficients.
fn find_irreducible(p: u64, n: usize) -> Option<Vec<u64>> {
    let total = p.checked_pow(n as u32)?;
    for code in 0..total {
        let mut f = Vec::with_capacity(n + 1);
        let mut c = code;
        for _ in 0..n {
            f.push(c % p);
            c /= p;
        }
        f.push(1);
        if is_irreducible(&f, p) {
            return Some(f);
        }
    }
    None
}

/// All monic irreducible polynomials of degree `n` over $\mathbb{F}_p$ (for counting checks).
pub fn count_monic_irreducibles(p: u64, n: usize) -> u64 {
    let Some(total) = p.checked_pow(n as u32) else {
        return 0;
    };
    (0..total)
        .filter(|&code| {
            let mut f = Vec::with_capacity(n + 1);
            let mut c = code;
            for _ in 0..n {
                f.push(c % p);
                c /= p;
            }
            f.push(1);
            is_irreducible(&f, p)
        })
        .count() as u64
}

impl GfArithmetic {
    /// Number of elements, $p^n$.
    pub fn order(&self) -> u64 {
        self.p.pow(self.degree as u32)
    }

    fn normalise(&self, mut a: Vec<u64>) -> Vec<u64> {
        a = poly_rem(&poly_trim(a), &self.modulus, self.p);
        a.resize(self.degree, 0);
        a
    }

    /// An element from any coefficient list (reduced modulo `p` and the modulus).
    pub fn element(&self, coeffs: &[i64]) -> Vec<u64> {
        self.normalise(
            coeffs
                .iter()
                .map(|&c| c.rem_euclid(self.p as i64) as u64)
                .collect(),
        )
    }

    pub fn zero(&self) -> Vec<u64> {
        vec![0; self.degree]
    }

    pub fn one(&self) -> Vec<u64> {
        self.element(&[1])
    }

    pub fn add(&self, a: &[u64], b: &[u64]) -> Vec<u64> {
        (0..self.degree).map(|i| (a[i] + b[i]) % self.p).collect()
    }

    pub fn sub(&self, a: &[u64], b: &[u64]) -> Vec<u64> {
        (0..self.degree)
            .map(|i| (a[i] + self.p - b[i]) % self.p)
            .collect()
    }

    pub fn neg(&self, a: &[u64]) -> Vec<u64> {
        a.iter().map(|&c| (self.p - c) % self.p).collect()
    }

    pub fn mul(&self, a: &[u64], b: &[u64]) -> Vec<u64> {
        self.normalise(poly_mul(
            &poly_trim(a.to_vec()),
            &poly_trim(b.to_vec()),
            self.p,
        ))
    }

    /// `a^e` by square-and-multiply.
    pub fn pow(&self, a: &[u64], mut e: u64) -> Vec<u64> {
        let (mut result, mut base) = (self.one(), a.to_vec());
        while e > 0 {
            if e & 1 == 1 {
                result = self.mul(&result, &base);
            }
            base = self.mul(&base, &base);
            e >>= 1;
        }
        result
    }

    /// The multiplicative inverse of a non-zero element: $a^{-1} = a^{q - 2}$ (the group has order $q - 1$).
    pub fn inverse(&self, a: &[u64]) -> Option<Vec<u64>> {
        if a.iter().all(|&c| c == 0) {
            return None;
        }
        Some(self.pow(a, self.order() - 2))
    }

    /// The Frobenius map $a \mapsto a^p$.
    pub fn frobenius(&self, a: &[u64]) -> Vec<u64> {
        self.pow(a, self.p)
    }

    /// Every element (for exhaustive checks on small fields).
    pub fn elements(&self) -> Vec<Vec<u64>> {
        (0..self.order())
            .map(|code| {
                let mut c = code;
                (0..self.degree)
                    .map(|_| {
                        let d = c % self.p;
                        c /= self.p;
                        d
                    })
                    .collect()
            })
            .collect()
    }
}
