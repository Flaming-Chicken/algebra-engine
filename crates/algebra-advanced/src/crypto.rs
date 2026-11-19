//! # `algebra-crypto`
//!
//! Elliptic Curve Cryptography over finite fields ($E/GF(p^n)$), Error-Correcting Codes
//! (Reed-Solomon, Hamming), and Symbolic Information Theory (Entropy $H(X)$, Mutual Information $I(X; Y)$).

use algebra_core::{AlgebraError, AlgebraResult, ExprGraph, ExprId};
use num_bigint::BigInt;
use num_traits::Zero;

/// Point on an Elliptic Curve $y^2 = x^3 + ax + b \pmod p$.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EllipticPoint {
    Point { x: BigInt, y: BigInt },
    IdentityAtInfinity,
}

/// Elliptic Curve over prime finite field $GF(p)$.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EllipticCurve {
    pub a: BigInt,
    pub b: BigInt,
    pub p: BigInt,
}

impl EllipticCurve {
    pub fn new(a: BigInt, b: BigInt, p: BigInt) -> AlgebraResult<Self> {
        // Verify non-singular discriminant: 4a^3 + 27b^2 != 0 (mod p)
        let a3 = (&a * &a * &a) * 4;
        let b2 = (&b * &b) * 27;
        let disc: BigInt = (a3 + b2) % &p;
        if disc.is_zero() {
            return Err(AlgebraError::DomainViolation {
                domain: "EllipticCurve".to_string(),
                reason: "Elliptic curve is singular (discriminant is 0 mod p)".to_string(),
            });
        }
        Ok(Self { a, b, p })
    }

    /// Check if point $(x, y)$ satisfies $y^2 \equiv x^3 + ax + b \pmod p$.
    pub fn contains_point(&self, point: &EllipticPoint) -> bool {
        match point {
            EllipticPoint::IdentityAtInfinity => true,
            EllipticPoint::Point { x, y } => {
                let lhs = (y * y) % &self.p;
                let rhs = ((x * x * x) + &self.a * x + &self.b) % &self.p;
                let lhs_norm = (lhs + &self.p) % &self.p;
                let rhs_norm = (rhs + &self.p) % &self.p;
                lhs_norm == rhs_norm
            }
        }
    }
}

/// Reduction type of an elliptic curve modulo a prime place $p$.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReductionType {
    /// Non-singular reduction ($p \nmid \Delta$).
    Good,
    /// Multiplicative (nodal) reduction ($p \mid \Delta$ and $p \nmid c_4$).
    Multiplicative { split: bool },
    /// Additive (cuspidal) reduction ($p \mid \Delta$ and $p \mid c_4$).
    Additive,
}

/// Elliptic curve defined over $\mathbb{Q}$ via Weierstrass equation $y^2 = x^3 + ax + b$ with $a, b \in \mathbb{Z}$.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RationalEllipticCurve {
    pub a: i64,
    pub b: i64,
}

impl RationalEllipticCurve {
    pub fn new(a: i64, b: i64) -> AlgebraResult<Self> {
        let Some(disc) = Self::checked_discriminant(a, b) else {
            return Err(AlgebraError::DomainViolation {
                domain: "RationalEllipticCurve".to_string(),
                reason: "the coefficients are too large: the discriminant does not fit in 128 bits"
                    .to_string(),
            });
        };
        if disc == 0 {
            return Err(AlgebraError::DomainViolation {
                domain: "RationalEllipticCurve".to_string(),
                reason: "Discriminant Delta = -16(4a^3 + 27b^2) is zero (curve is singular over Q)"
                    .to_string(),
            });
        }
        Ok(Self { a, b })
    }

    /// Compute discriminant $\Delta = -16(4a^3 + 27b^2)$.
    pub fn discriminant(&self) -> i128 {
        Self::compute_discriminant(self.a, self.b)
    }

    fn compute_discriminant(a: i64, b: i64) -> i128 {
        // `new` has checked that this does not overflow.
        Self::checked_discriminant(a, b).unwrap_or(i128::MAX)
    }

    fn checked_discriminant(a: i64, b: i64) -> Option<i128> {
        let (a, b) = (a as i128, b as i128);
        let a3 = a.checked_mul(a)?.checked_mul(a)?.checked_mul(4)?;
        let b2 = b.checked_mul(b)?.checked_mul(27)?;
        a3.checked_add(b2)?.checked_mul(-16)
    }

    /// Compute invariants $c_4 = -48a$.
    pub fn c4(&self) -> i128 {
        -48 * (self.a as i128)
    }

    /// Identify the bad reduction places (primes dividing $\Delta$) using prime factorization of the full
    /// discriminant (which can exceed 64 bits, so the primes are returned as big integers).
    pub fn bad_reduction_places(&self) -> Vec<num_bigint::BigUint> {
        let disc_abs = self.discriminant().unsigned_abs();
        if disc_abs <= 1 {
            return Vec::new();
        }
        algebra_engine::numbertheory::big_prime_factors(&num_bigint::BigUint::from(disc_abs))
            .into_iter()
            .map(|(p, _)| p)
            .collect()
    }

    /// Determine the reduction type at a given prime place $p$.
    pub fn reduction_at(&self, p: u64) -> ReductionType {
        let disc = self.discriminant();
        let p_i128 = p as i128;
        if disc % p_i128 != 0 {
            ReductionType::Good
        } else {
            let c4 = self.c4();
            if c4 % p_i128 != 0 {
                // Multiplicative reduction is split when the tangents at the node are defined over F_p, which
                // happens exactly when -c6 is a square mod p, with c6 = -864 b. Since 864 = 6 * 12^2 that is
                // when 6b is a non-zero square mod p (for odd p; p = 3 has no such short model).
                let split = p > 3 && {
                    let six_b = (6 * (self.b as i128)).rem_euclid(p_i128) as i64;
                    algebra_engine::numbertheory::legendre_symbol(six_b, p) == 1
                };
                ReductionType::Multiplicative { split }
            } else {
                ReductionType::Additive
            }
        }
    }

    /// Compute the Frobenius trace $a_p = p + 1 - \#E(\mathbb{F}_p) = -\sum_{x=0}^{p-1} \left(\frac{x^3+ax+b}{p}\right)$
    /// for an odd prime $p$.
    pub fn frobenius_trace(&self, p: u64) -> Option<i64> {
        if p < 2 {
            return None;
        }
        if p == 2 {
            // Count the solutions of y^2 = x^3 + ax + b over F_2 directly (the Legendre sum needs odd p).
            let (a, b) = (self.a.rem_euclid(2), self.b.rem_euclid(2));
            let mut count = 1i64; // the point at infinity
            for x in 0..2i64 {
                for y in 0..2i64 {
                    if (y * y - (x * x * x + a * x + b)).rem_euclid(2) == 0 {
                        count += 1;
                    }
                }
            }
            return Some(3 - count);
        }

        let mut sum_legendre = 0i64;
        let a_mod = ((self.a % p as i64) + p as i64) % p as i64;
        let b_mod = ((self.b % p as i64) + p as i64) % p as i64;

        for x in 0..p as i64 {
            let x3 = (x * x % p as i64) * x % p as i64;
            let rhs = (x3 + a_mod * x + b_mod) % p as i64;
            let leg = algebra_engine::numbertheory::legendre_symbol(rhs, p);
            sum_legendre += leg as i64;
        }

        Some(-sum_legendre)
    }

    /// Reduce the curve modulo $p$ into an `EllipticCurve` over $\mathbb{F}_p$.
    pub fn reduce_mod_p(&self, p: u64) -> AlgebraResult<EllipticCurve> {
        let p_big = BigInt::from(p);
        let a_mod = (BigInt::from(self.a) % &p_big + &p_big) % &p_big;
        let b_mod = (BigInt::from(self.b) % &p_big + &p_big) % &p_big;
        EllipticCurve::new(a_mod, b_mod, p_big)
    }
}

/// Arithmetic in $GF(2^8)$ with the primitive polynomial $x^8 + x^4 + x^3 + x^2 + 1$ (0x11d) and primitive element $\alpha = 2$.
struct Gf256 {
    exp: [u8; 512],
    log: [u8; 256],
}

impl Gf256 {
    fn new() -> Self {
        let mut exp = [0u8; 512];
        let mut log = [0u8; 256];
        let mut x: u16 = 1;
        for i in 0..255 {
            exp[i] = x as u8;
            log[x as usize] = i as u8;
            x <<= 1;
            if x & 0x100 != 0 {
                x ^= 0x11d;
            }
        }
        for i in 255..512 {
            exp[i] = exp[i - 255];
        }
        Self { exp, log }
    }

    fn mul(&self, a: u8, b: u8) -> u8 {
        if a == 0 || b == 0 {
            0
        } else {
            self.exp[self.log[a as usize] as usize + self.log[b as usize] as usize]
        }
    }

    fn div(&self, a: u8, b: u8) -> u8 {
        if a == 0 {
            0
        } else {
            self.exp[self.log[a as usize] as usize + 255 - self.log[b as usize] as usize]
        }
    }

    /// $\alpha^e$ for any integer exponent.
    fn alpha_pow(&self, e: i64) -> u8 {
        self.exp[e.rem_euclid(255) as usize]
    }

    /// Evaluate a polynomial given highest-degree coefficient first (Horner).
    fn eval_high_first(&self, poly: &[u8], x: u8) -> u8 {
        poly.iter().fold(0, |acc, &c| self.mul(acc, x) ^ c)
    }

    /// Evaluate a polynomial given lowest-degree coefficient first.
    fn eval_low_first(&self, poly: &[u8], x: u8) -> u8 {
        poly.iter().rev().fold(0, |acc, &c| self.mul(acc, x) ^ c)
    }
}

/// Why a Reed-Solomon word could not be encoded or decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReedSolomonError {
    /// The word or message has the wrong length.
    WrongLength { expected: usize, found: usize },
    /// More than `(n - k) / 2` symbols are wrong, so the word cannot be corrected.
    TooManyErrors,
}

/// Reed-Solomon code $RS(n, k)$ over $GF(2^8)$ ($n \le 255$) with generator $g(x) = \prod_{i=0}^{n-k-1} (x - \alpha^i)$.
///
/// A word is stored as `[message (k symbols), parity (n - k symbols)]`, the first symbol being the coefficient of
/// $x^{n-1}$. The code is maximum distance separable: its minimum distance is $n - k + 1$, so it corrects
/// $\lfloor (n - k) / 2 \rfloor$ wrong symbols.
#[derive(Debug, Clone)]
pub struct ReedSolomonCode {
    pub n: usize,
    pub k: usize,
}

impl ReedSolomonCode {
    pub fn new(n: usize, k: usize) -> AlgebraResult<Self> {
        if k >= n {
            return Err(AlgebraError::DomainViolation {
                domain: "ReedSolomonCode".to_string(),
                reason: format!(
                    "Reed-Solomon k ({}) must be strictly less than n ({})",
                    k, n
                ),
            });
        }
        if n > 255 || k == 0 {
            return Err(AlgebraError::DomainViolation {
                domain: "ReedSolomonCode".to_string(),
                reason: format!(
                    "Reed-Solomon over GF(256) needs 1 <= k < n <= 255, got n = {n}, k = {k}"
                ),
            });
        }
        Ok(Self { n, k })
    }

    pub fn parity_symbols(&self) -> usize {
        self.n - self.k
    }

    /// The minimum distance $n - k + 1$ (the Singleton bound is attained).
    pub fn minimum_distance(&self) -> usize {
        self.n - self.k + 1
    }

    /// The number of wrong symbols the decoder is guaranteed to correct.
    pub fn correctable_errors(&self) -> usize {
        (self.n - self.k) / 2
    }

    /// The generator polynomial, lowest degree first (monic, degree `n - k`).
    fn generator(&self, gf: &Gf256) -> Vec<u8> {
        let mut g = vec![1u8];
        for i in 0..self.parity_symbols() {
            let root = gf.alpha_pow(i as i64);
            let mut next = vec![0u8; g.len() + 1];
            for (j, &c) in g.iter().enumerate() {
                next[j + 1] ^= c; // x * g
                next[j] ^= gf.mul(c, root); // root * g (minus is plus in characteristic 2)
            }
            g = next;
        }
        g
    }

    /// Systematic encoding: the message followed by its `n - k` parity symbols.
    pub fn encode(&self, message: &[u8]) -> Result<Vec<u8>, ReedSolomonError> {
        if message.len() != self.k {
            return Err(ReedSolomonError::WrongLength {
                expected: self.k,
                found: message.len(),
            });
        }
        let gf = Gf256::new();
        let g = self.generator(&gf); // low first, monic
        let parity = self.parity_symbols();
        // Remainder of m(x) x^(n-k) modulo g(x), by long division with the message highest degree first.
        let mut rem = vec![0u8; parity];
        for &m in message {
            let feedback = m ^ rem[0];
            rem.rotate_left(1);
            rem[parity - 1] = 0;
            if feedback != 0 {
                for (i, r) in rem.iter_mut().enumerate() {
                    // coefficient of x^(parity - 1 - i) in g
                    *r ^= gf.mul(feedback, g[parity - 1 - i]);
                }
            }
        }
        let mut word = message.to_vec();
        word.extend(rem);
        Ok(word)
    }

    /// The syndromes $S_j = c(\alpha^j)$, $j = 0, \dots, n - k - 1$; all zero exactly for codewords.
    pub fn syndromes(&self, word: &[u8]) -> Result<Vec<u8>, ReedSolomonError> {
        if word.len() != self.n {
            return Err(ReedSolomonError::WrongLength {
                expected: self.n,
                found: word.len(),
            });
        }
        let gf = Gf256::new();
        Ok((0..self.parity_symbols())
            .map(|j| gf.eval_high_first(word, gf.alpha_pow(j as i64)))
            .collect())
    }

    /// Decode a received word, correcting up to [`Self::correctable_errors`] wrong symbols (Berlekamp–Massey, Chien
    /// search, Forney's formula), and return the corrected message.
    pub fn decode(&self, received: &[u8]) -> Result<Vec<u8>, ReedSolomonError> {
        let syndromes = self.syndromes(received)?;
        let mut word = received.to_vec();
        if syndromes.iter().any(|&s| s != 0) {
            let gf = Gf256::new();
            let nsym = self.parity_symbols();
            // Berlekamp–Massey: the error locator polynomial lambda (lowest degree first).
            let mut lambda = vec![1u8];
            let mut prev = vec![1u8];
            let (mut l, mut m, mut b) = (0usize, 1usize, 1u8);
            for n in 0..nsym {
                let mut d = syndromes[n];
                for i in 1..=l.min(lambda.len() - 1) {
                    d ^= gf.mul(lambda[i], syndromes[n - i]);
                }
                if d == 0 {
                    m += 1;
                    continue;
                }
                let scale = gf.div(d, b);
                let old = lambda.clone();
                lambda.resize(lambda.len().max(prev.len() + m), 0);
                for (i, &c) in prev.iter().enumerate() {
                    lambda[i + m] ^= gf.mul(scale, c);
                }
                if 2 * l <= n {
                    l = n + 1 - l;
                    prev = old;
                    b = d;
                    m = 1;
                } else {
                    m += 1;
                }
            }
            while lambda.len() > 1 && lambda[lambda.len() - 1] == 0 {
                lambda.pop();
            }
            if l > self.correctable_errors() || lambda.len() - 1 != l {
                return Err(ReedSolomonError::TooManyErrors);
            }
            // Chien search: an error at the coefficient of x^p has locator X = alpha^p and lambda(X^-1) = 0.
            let positions: Vec<usize> = (0..self.n)
                .filter(|&p| gf.eval_low_first(&lambda, gf.alpha_pow(-(p as i64))) == 0)
                .collect();
            if positions.len() != l {
                return Err(ReedSolomonError::TooManyErrors);
            }
            // Forney: omega = S(x) lambda(x) mod x^nsym, error value e = X * omega(X^-1) / lambda'(X^-1).
            let mut omega = vec![0u8; nsym];
            for (i, &li) in lambda.iter().enumerate() {
                for (j, &sj) in syndromes.iter().enumerate() {
                    if i + j < nsym {
                        omega[i + j] ^= gf.mul(li, sj);
                    }
                }
            }
            // Formal derivative in characteristic 2: only the odd-degree terms survive.
            let derivative: Vec<u8> = (1..lambda.len())
                .map(|i| if i % 2 == 1 { lambda[i] } else { 0 })
                .collect();
            for &p in &positions {
                let x = gf.alpha_pow(p as i64);
                let x_inv = gf.alpha_pow(-(p as i64));
                let denominator = gf.eval_low_first(&derivative, x_inv);
                if denominator == 0 {
                    return Err(ReedSolomonError::TooManyErrors);
                }
                let value = gf.mul(x, gf.div(gf.eval_low_first(&omega, x_inv), denominator));
                word[self.n - 1 - p] ^= value;
            }
            if self.syndromes(&word)?.iter().any(|&s| s != 0) {
                return Err(ReedSolomonError::TooManyErrors);
            }
        }
        Ok(word[..self.k].to_vec())
    }
}

/// Symbolic Information Theory utilities.
#[derive(Debug, Clone, Default)]
pub struct InformationTheory;

impl InformationTheory {
    /// Symbolic Shannon Entropy $H(X) = -\sum_{i} P(x_i) \log_2 P(x_i)$.
    pub fn entropy(&self, graph: &ExprGraph, probabilities: &[ExprId]) -> ExprId {
        let mut terms = Vec::with_capacity(probabilities.len());

        for &p in probabilities {
            let log_p = graph.function("log2", [p]);
            let p_log_p = graph.mul([p, log_p]);
            terms.push(p_log_p);
        }

        let sum = graph.add(terms);
        graph.mul([graph.integer(-1), sum])
    }

    /// Symbolic Mutual Information $I(X; Y) = H(X) + H(Y) - H(X, Y)$.
    pub fn mutual_information(
        &self,
        graph: &ExprGraph,
        h_x: ExprId,
        h_y: ExprId,
        h_xy: ExprId,
    ) -> ExprId {
        let neg_h_xy = graph.mul([graph.integer(-1), h_xy]);
        graph.add([h_x, h_y, neg_h_xy])
    }

    /// Symbolic Kullback-Leibler Divergence $D_{KL}(P \parallel Q) = \sum P(x) \log \left(\frac{P(x)}{Q(x)}\right)$.
    pub fn kl_divergence(&self, graph: &ExprGraph, p_dist: &[ExprId], q_dist: &[ExprId]) -> ExprId {
        let mut terms = Vec::new();

        for (&p, &q) in p_dist.iter().zip(q_dist) {
            let ratio = graph.div(p, q);
            let log_ratio = graph.function("log", [ratio]);
            let term = graph.mul([p, log_ratio]);
            terms.push(term);
        }

        graph.add(terms)
    }
}
