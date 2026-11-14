//! Divided Power PD-Algebra spine for positive characteristic p > 0.
//!
//! Evaluates divided powers \gamma_n(x) = x^n / n! without division by n!,
//! using Lucas's theorem for combinatorial binomial coefficients modulo p.

/// Lucas's Theorem for binomial coefficients modulo prime p:
/// \binom{n}{k} \equiv \prod_{i=0}^m \binom{n_i}{k_i} \pmod p
pub fn lucas_binomial_mod_p(mut n: u64, mut k: u64, p: u64) -> u64 {
    if k > n {
        return 0;
    }
    if k == 0 || k == n {
        return 1;
    }

    let mut result = 1u64;
    while n > 0 || k > 0 {
        let ni = n % p;
        let ki = k % p;
        if ki > ni {
            return 0;
        }
        result = (result * small_binomial_mod_p(ni, ki, p)) % p;
        n /= p;
        k /= p;
    }
    result
}

/// Helper for small binomial coefficients modulo prime p where n, k < p.
fn small_binomial_mod_p(n: u64, k: u64, p: u64) -> u64 {
    if k > n {
        return 0;
    }
    if k == 0 || k == n {
        return 1;
    }
    let k = k.min(n - k);
    let mut num = 1u64;
    let mut den = 1u64;
    for i in 0..k {
        num = (num * (n - i)) % p;
        den = (den * (i + 1)) % p;
    }
    // Multiply by modular inverse of den
    (num * mod_inverse_prime(den, p)) % p
}

/// Modular inverse modulo prime p via Fermat's Little Theorem: a^{p-2} \equiv a^{-1} \pmod p.
fn mod_inverse_prime(mut base: u64, p: u64) -> u64 {
    let mut exp = p - 2;
    let mut res = 1u64;
    base %= p;
    while exp > 0 {
        if exp % 2 == 1 {
            res = (res * base) % p;
        }
        base = (base * base) % p;
        exp /= 2;
    }
    res
}

/// Divided power element: \gamma_n(x).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct DividedPowerAtom {
    /// Degree n of the divided power \gamma_n(x).
    pub degree: u32,
    /// Characteristic p of the underlying base field \mathbb{F}_p.
    pub characteristic: u32,
    /// Coefficient in \mathbb{F}_p.
    pub coeff: u64,
}

impl DividedPowerAtom {
    /// Create new divided power term.
    pub fn new(degree: u32, characteristic: u32, coeff: u64) -> Self {
        Self {
            degree,
            characteristic,
            coeff: coeff % (characteristic as u64),
        }
    }

    /// Product in Divided Power Algebra:
    /// \gamma_n(x) \cdot \gamma_m(x) = \binom{n+m}{n} \gamma_{n+m}(x)
    /// Evaluated purely via Lucas's theorem without any division by factorials!
    pub fn mul(&self, other: &Self) -> Option<Self> {
        if self.characteristic != other.characteristic {
            return None;
        }
        let p = self.characteristic as u64;
        let binom = lucas_binomial_mod_p(
            (self.degree + other.degree) as u64,
            self.degree as u64,
            p,
        );
        let c = (((self.coeff * other.coeff) % p) * binom) % p;
        Some(Self {
            degree: self.degree + other.degree,
            characteristic: self.characteristic,
            coeff: c,
        })
    }
}
