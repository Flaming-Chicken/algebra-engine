//! Adelic Number Engine (\mathbb{A}_\mathbb{Q}) and p-adic completions.
//!
//! Implements the ring of adeles \mathbb{A}_\mathbb{Q} as the restricted direct product
//! of \mathbb{R} and all p-adic completions \mathbb{Q}_p.
//!
//! Formulates the Artin Product Formula:
//! \prod_{v \le \infty} |x|_v = |x|_\infty \times \prod_p |x|_p = 1 \quad (\forall x \in \mathbb{Q}^\times).

use cat_core::scalar::{gcd_i64, AdeleAtom};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::fmt;

/// p-adic number in \mathbb{Q}_p represented via valuation v_p and base-p fractional expansion.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PadicNumber {
    /// Prime base p.
    pub prime: u64,
    /// p-adic valuation v_p(x).
    pub valuation: i32,
    /// Initial digits a_0, a_1, ... in {0, ..., p-1} for unit u = \sum a_i p^i.
    pub digits: SmallVec<[u8; 8]>,
}

impl PadicNumber {
    /// Zero in \mathbb{Q}_p.
    pub fn zero(prime: u64) -> Self {
        Self {
            prime,
            valuation: i32::MAX,
            digits: smallvec::smallvec![0],
        }
    }

    /// Check if this p-adic number is zero.
    pub fn is_zero(&self) -> bool {
        self.valuation == i32::MAX || self.digits.iter().all(|&d| d == 0)
    }

    /// Construct p-adic number from rational n / d.
    pub fn from_rational(mut n: i64, mut d: i64, prime: u64) -> Self {
        if n == 0 {
            return Self::zero(prime);
        }
        let mut v = 0i32;
        let p = prime as i64;

        while n % p == 0 {
            v += 1;
            n /= p;
        }
        while d % p == 0 {
            v -= 1;
            d /= p;
        }

        // Compute first 4 digits of unit u = n / d in Z_p via modular inverse
        let mut digits = SmallVec::new();
        let rem = ((n % p + p) % p) as u64;
        let den_mod = ((d % p + p) % p) as u64;
        let inv = mod_inv_p(den_mod, prime);
        let a0 = ((rem * inv) % prime) as u8;
        digits.push(a0);

        Self {
            prime,
            valuation: v,
            digits,
        }
    }

    /// p-adic norm: |x|_p = p^{-v_p(x)} (with |0|_p = 0).
    pub fn norm(&self) -> f64 {
        if self.is_zero() {
            0.0
        } else {
            (self.prime as f64).powi(-self.valuation)
        }
    }
}

/// Element in the Ring of Adeles \mathbb{A}_\mathbb{Q} = \mathbb{R} \times \prod'_p \mathbb{Q}_p.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct AdeleNumber {
    /// Archimedean place component x_\infty \in \mathbb{R}.
    pub arch_num: i64,
    pub arch_den: i64,
    /// Non-archimedean places x_p \in \mathbb{Q}_p for non-trivial primes.
    pub local_places: Vec<PadicNumber>,
}

impl AdeleNumber {
    /// Zero in the adele ring.
    pub fn zero() -> Self {
        Self {
            arch_num: 0,
            arch_den: 1,
            local_places: Vec::new(),
        }
    }

    /// One in the adele ring.
    pub fn one() -> Self {
        Self {
            arch_num: 1,
            arch_den: 1,
            local_places: Vec::new(),
        }
    }

    /// Canonical diagonal embedding of rational q = num/den into \mathbb{A}_\mathbb{Q}.
    pub fn from_rational(num: i64, den: i64) -> Self {
        if den == 0 {
            return Self::zero();
        }
        let g = gcd_i64(num, den);
        let n = num / g;
        let d = den / g;

        let mut local = Vec::new();
        // Dynamically find all unique prime factors of |n| and |d|
        let mut p_set = Vec::new();
        let mut rem_n = n.unsigned_abs();
        let mut rem_d = d.unsigned_abs();

        if rem_n > 0 || rem_d > 0 {
            // Factor out 2
            let mut div2 = false;
            while rem_n > 0 && rem_n % 2 == 0 {
                rem_n /= 2;
                div2 = true;
            }
            while rem_d > 0 && rem_d % 2 == 0 {
                rem_d /= 2;
                div2 = true;
            }
            if div2 {
                p_set.push(2);
            }

            // Factor out odd primes
            let mut p = 3u64;
            while (p.saturating_mul(p) <= rem_n && rem_n > 0) || (p.saturating_mul(p) <= rem_d && rem_d > 0) {
                let mut div_p = false;
                while rem_n > 0 && rem_n % p == 0 {
                    rem_n /= p;
                    div_p = true;
                }
                while rem_d > 0 && rem_d % p == 0 {
                    rem_d /= p;
                    div_p = true;
                }
                if div_p {
                    p_set.push(p);
                }
                p += 2;
            }

            if rem_n > 1 {
                p_set.push(rem_n);
            }
            if rem_d > 1 {
                p_set.push(rem_d);
            }
            p_set.sort_unstable();
            p_set.dedup();
        }

        for p in p_set {
            local.push(PadicNumber::from_rational(n, d, p));
        }

        Self {
            arch_num: n,
            arch_den: d,
            local_places: local,
        }
    }

    /// Verify the Artin Product Formula:
    /// \prod_{v \le \infty} |x|_v = |x|_\infty \times \prod_p |x|_p = 1 \quad (\forall x \in \mathbb{Q}^\times).
    pub fn verify_artin_product_formula(&self) -> bool {
        if self.arch_num == 0 {
            return false;
        }
        let arch_norm = (self.arch_num as f64 / self.arch_den as f64).abs();
        let mut total_norm = arch_norm;

        for p_elem in &self.local_places {
            total_norm *= p_elem.norm();
        }

        (total_norm - 1.0).abs() < 1e-11
    }

    /// Idele norm: \prod_v |x|_v.
    pub fn idele_norm(&self) -> f64 {
        if self.arch_num == 0 {
            return 0.0;
        }
        let mut prod = (self.arch_num as f64 / self.arch_den as f64).abs();
        for p_elem in &self.local_places {
            prod *= p_elem.norm();
        }
        prod
    }

    /// Adele addition (componentwise).
    pub fn add(&self, other: &Self) -> Self {
        let n = self.arch_num * other.arch_den + other.arch_num * self.arch_den;
        let d = self.arch_den * other.arch_den;
        Self::from_rational(n, d)
    }

    /// Adele multiplication (componentwise).
    pub fn mul(&self, other: &Self) -> Self {
        let n = self.arch_num * other.arch_num;
        let d = self.arch_den * other.arch_den;
        Self::from_rational(n, d)
    }

    /// Convert to lightweight `cat_core::scalar::AdeleAtom`.
    pub fn to_atom(&self) -> AdeleAtom {
        let mut vals = smallvec::SmallVec::new();
        for p in &self.local_places {
            if !p.is_zero() && p.valuation != 0 {
                vals.push((p.prime, p.valuation));
            }
        }
        AdeleAtom {
            arch_num: self.arch_num,
            arch_den: self.arch_den,
            p_adic_valuations: vals,
        }
    }
}

/// Modular inverse helper modulo prime p.
fn mod_inv_p(mut a: u64, p: u64) -> u64 {
    let mut exp = p - 2;
    let mut res = 1u64;
    a %= p;
    while exp > 0 {
        if exp % 2 == 1 {
            res = (res * a) % p;
        }
        a = (a * a) % p;
        exp /= 2;
    }
    res
}

impl fmt::Debug for AdeleNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Adele({}/{}, places={})",
            self.arch_num,
            self.arch_den,
            self.local_places.len()
        )
    }
}

impl fmt::Display for AdeleNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}/{} in A_Q)", self.arch_num, self.arch_den)
    }
}
