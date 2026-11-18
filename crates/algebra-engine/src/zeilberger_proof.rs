//! # Proving a Zeilberger recurrence for all `n`
//!
//! [`crate::zeilberger::zeilberger`] solves the telescoping problem at sampled integers `n` and
//! fits the recurrence `sum_i c_i(n) S(n+i) = 0` to them, so the recurrence is established only
//! at those `n`. This module turns it into a statement for **all** `n` by *guess and prove*:
//!
//! 1. For each sample the certificate `R(n, k) = G(n, k) / F(n, k)` of the telescoping identity
//!    `sum_i c_i(n) F(n+i, k) = G(n, k+1) - G(n, k)` is rebuilt exactly (it is a rational function
//!    of `k`), reduced to lowest terms with a monic denominator, and each of its coefficients is
//!    reconstructed as a rational function of `n` by exact interpolation. The result is a
//!    certificate `R = U(n, k) / V(n, k)` with polynomial `U`, `V`.
//! 2. The identity is then checked **symbolically**: with the shift ratios of the term as
//!    polynomials in `(n, k)`, both sides, cleared of denominators, are polynomials in `(n, k)`,
//!    and their difference must be the zero polynomial. No sampling is involved in this step, so
//!    when it succeeds the identity holds for every `n` and `k` at which no denominator vanishes.
//!
//! What this does **not** prove is the boundary behaviour: summing the identity over `k` gives
//! the recurrence for `S(n) = sum_k F(n, k)` only if `G(n, k)` vanishes at both ends of the sum
//! (true for terms with finite support, such as binomial coefficients), which the caller must
//! know. The guessed certificate is only a candidate; a wrong guess fails the symbolic check and is
//! reported as an error rather than accepted.

use crate::gosper::QPoly;
use crate::guess::{Recurrence, null_space};
use crate::zeilberger::{HypergeometricTerm, telescoping_full};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};

fn rat(n: i64) -> BigRational {
    BigRational::from_integer(BigInt::from(n))
}

/// A polynomial in `k` and `n` over the rationals: `coeffs[j][i]` multiplies `k^j n^i`.
#[derive(Debug, Clone, PartialEq)]
pub struct Poly2 {
    coeffs: Vec<Vec<BigRational>>,
}

impl Poly2 {
    fn trimmed(mut coeffs: Vec<Vec<BigRational>>) -> Self {
        for row in &mut coeffs {
            while row.last().is_some_and(|c| c.is_zero()) {
                row.pop();
            }
        }
        while coeffs.last().is_some_and(|r| r.is_empty()) {
            coeffs.pop();
        }
        Self { coeffs }
    }

    /// The zero polynomial.
    pub fn zero() -> Self {
        Self { coeffs: Vec::new() }
    }

    /// Whether this is the zero polynomial.
    pub fn is_zero(&self) -> bool {
        self.coeffs.is_empty()
    }

    /// A polynomial in `k` alone.
    pub fn from_k(p: &QPoly) -> Self {
        Self::trimmed(
            (0..=p.degree().unwrap_or(0))
                .map(|j| vec![p.coeff(j)])
                .collect(),
        )
    }

    /// A polynomial in `n` alone.
    pub fn from_n(p: &QPoly) -> Self {
        Self::trimmed(vec![
            (0..=p.degree().unwrap_or(0)).map(|i| p.coeff(i)).collect(),
        ])
    }

    /// Build from integer coefficients `(k power, n power, coefficient)`.
    pub fn from_terms(terms: &[(usize, usize, i64)]) -> Self {
        let mut coeffs: Vec<Vec<BigRational>> = Vec::new();
        for &(j, i, c) in terms {
            while coeffs.len() <= j {
                coeffs.push(Vec::new());
            }
            while coeffs[j].len() <= i {
                coeffs[j].push(BigRational::zero());
            }
            coeffs[j][i] += rat(c);
        }
        Self::trimmed(coeffs)
    }

    fn coeff(&self, j: usize, i: usize) -> BigRational {
        self.coeffs
            .get(j)
            .and_then(|r| r.get(i))
            .cloned()
            .unwrap_or_else(BigRational::zero)
    }

    /// Degree in `k`.
    pub fn degree_k(&self) -> Option<usize> {
        self.coeffs.len().checked_sub(1)
    }

    /// Sum.
    pub fn add(&self, other: &Self) -> Self {
        let rows = self.coeffs.len().max(other.coeffs.len());
        let cols = self
            .coeffs
            .iter()
            .chain(other.coeffs.iter())
            .map(Vec::len)
            .max()
            .unwrap_or(0);
        Self::trimmed(
            (0..rows)
                .map(|j| {
                    (0..cols)
                        .map(|i| self.coeff(j, i) + other.coeff(j, i))
                        .collect()
                })
                .collect(),
        )
    }

    /// Multiply by a rational scalar.
    pub fn scale(&self, s: &BigRational) -> Self {
        Self::trimmed(
            self.coeffs
                .iter()
                .map(|r| r.iter().map(|c| c * s).collect())
                .collect(),
        )
    }

    /// Difference.
    pub fn sub(&self, other: &Self) -> Self {
        self.add(&other.scale(&-BigRational::one()))
    }

    /// Product.
    pub fn mul(&self, other: &Self) -> Self {
        if self.is_zero() || other.is_zero() {
            return Self::zero();
        }
        let rows = self.coeffs.len() + other.coeffs.len() - 1;
        let cols = self.coeffs.iter().map(Vec::len).max().unwrap_or(0)
            + other.coeffs.iter().map(Vec::len).max().unwrap_or(0)
            - 1;
        let mut out = vec![vec![BigRational::zero(); cols]; rows];
        for (j1, r1) in self.coeffs.iter().enumerate() {
            for (i1, a) in r1.iter().enumerate() {
                if a.is_zero() {
                    continue;
                }
                for (j2, r2) in other.coeffs.iter().enumerate() {
                    for (i2, b) in r2.iter().enumerate() {
                        out[j1 + j2][i1 + i2] += a * b;
                    }
                }
            }
        }
        Self::trimmed(out)
    }

    /// The polynomial in `k` obtained by substituting the integer or rational `n`.
    pub fn eval_n(&self, n: &BigRational) -> QPoly {
        QPoly::new(
            self.coeffs
                .iter()
                .map(|row| {
                    row.iter()
                        .rev()
                        .fold(BigRational::zero(), |acc, c| acc * n + c)
                })
                .collect(),
        )
    }

    /// `p(n + h, k)`.
    pub fn shift_n(&self, h: i64) -> Self {
        let h = rat(h);
        Self::trimmed(
            self.coeffs
                .iter()
                .map(|row| {
                    let q = QPoly::new(row.clone()).shift(&h);
                    (0..=q.degree().unwrap_or(0)).map(|i| q.coeff(i)).collect()
                })
                .collect(),
        )
    }

    /// `p(n, k + h)`.
    pub fn shift_k(&self, h: i64) -> Self {
        let linear = Self::from_terms(&[(1, 0, 1), (0, 0, h)]);
        let mut out = Self::zero();
        for row in self.coeffs.iter().rev() {
            out = out
                .mul(&linear)
                .add(&Self::from_n(&QPoly::new(row.clone())));
        }
        out
    }
}

/// A hypergeometric term `F(n, k)` given by its two shift ratios as polynomials in `(n, k)`:
/// `F(n, k+1) / F(n, k) = ratio_k.0 / ratio_k.1` and `F(n+1, k) / F(n, k) = ratio_n.0 / ratio_n.1`.
pub struct SymbolicTerm {
    pub ratio_k: (Poly2, Poly2),
    pub ratio_n: (Poly2, Poly2),
}

impl SymbolicTerm {
    /// The same term for the integer-`n` interface of [`crate::zeilberger`].
    pub fn to_term(&self) -> HypergeometricTerm {
        let (rk, rn) = (self.ratio_k.clone_pair(), self.ratio_n.clone_pair());
        HypergeometricTerm {
            ratio_k: Box::new(move |n| (rk.0.eval_n(n), rk.1.eval_n(n))),
            ratio_n: Box::new(move |n| (rn.0.eval_n(n), rn.1.eval_n(n))),
        }
    }
}

trait ClonePair {
    fn clone_pair(&self) -> (Poly2, Poly2);
}
impl ClonePair for (Poly2, Poly2) {
    fn clone_pair(&self) -> (Poly2, Poly2) {
        (self.0.clone(), self.1.clone())
    }
}

/// The result of a successful proof.
#[derive(Debug, Clone, PartialEq)]
pub struct Proof {
    /// The recurrence that was proved.
    pub recurrence: Recurrence,
    /// Certificate numerator `U(n, k)`.
    pub certificate_numerator: Poly2,
    /// Certificate denominator `V(n, k)`: `G = F U / V`.
    pub certificate_denominator: Poly2,
}

/// A rational function `P(n) / Q(n)` through the points `(xs[s], ys[s])`, of the smallest total
/// degree for which one exists with at least three spare points, in lowest terms with monic `Q`.
fn rational_fit(xs: &[BigRational], ys: &[BigRational]) -> Option<(QPoly, QPoly)> {
    let m = xs.len();
    for total in 0..m.saturating_sub(3) {
        for dp in 0..=total {
            let dq = total - dp;
            let unknowns = dp + dq + 2;
            if unknowns + 2 >= m {
                continue;
            }
            let matrix: Vec<Vec<BigRational>> = xs
                .iter()
                .zip(ys)
                .map(|(x, y)| {
                    let mut row = Vec::with_capacity(unknowns);
                    let mut pw = BigRational::one();
                    for _ in 0..=dp {
                        row.push(pw.clone());
                        pw *= x;
                    }
                    let mut pw = BigRational::one();
                    for _ in 0..=dq {
                        row.push(-(&pw * y));
                        pw *= x;
                    }
                    row
                })
                .collect();
            for v in null_space(matrix, unknowns) {
                let p = QPoly::new(v[..=dp].to_vec());
                let q = QPoly::new(v[dp + 1..].to_vec());
                if q.is_zero() {
                    continue;
                }
                let g = p.gcd(&q);
                let (p, q) = match (p.div_rem(&g), q.div_rem(&g)) {
                    (Some((p, _)), Some((q, _))) => (p, q),
                    _ => continue,
                };
                let lead = q.lead()?.clone();
                let inv = BigRational::one() / lead;
                return Some((p.scale(&inv), q.scale(&inv)));
            }
        }
    }
    None
}

/// Prove `sum_i c_i(n) F(n+i, k) = G(n, k+1) - G(n, k)` for all `n`, with `G = F * U / V`, for the
/// recurrence `rec`, using the integer samples `samples` to rebuild the certificate (they must be
/// many enough, and large enough to avoid degenerate small `n`).
pub fn prove_recurrence(
    term: &SymbolicTerm,
    rec: &Recurrence,
    samples: &[i64],
) -> Result<Proof, String> {
    let order = rec.order();
    let hyper = term.to_term();
    // 1. The certificate at each sample, normalised: R = num / den with monic den.
    let mut per_sample: Vec<(BigRational, QPoly, QPoly)> = Vec::new();
    for &n0 in samples {
        let nq = rat(n0);
        let c_target: Vec<BigRational> = rec.coefficients.iter().map(|c| c.eval(&nq)).collect();
        if c_target.iter().all(|c| c.is_zero()) {
            return Err(format!("the recurrence vanishes identically at n = {n0}"));
        }
        let full = telescoping_full(&hyper, n0, order)?;
        // Find the solution whose c part is proportional to c(n0) and scale it to equal it.
        let pivot = c_target
            .iter()
            .position(|c| !c.is_zero())
            .ok_or_else(|| "zero coefficient vector".to_string())?;
        let mut found: Option<QPoly> = None;
        for (c_part, x) in &full.solutions {
            if c_part[pivot].is_zero() {
                continue;
            }
            let factor = &c_target[pivot] / &c_part[pivot];
            if c_part
                .iter()
                .zip(&c_target)
                .all(|(a, b)| &(a * &factor) == b)
            {
                found = Some(x.scale(&factor));
                break;
            }
        }
        // A combination of several basis vectors can also match; solve for it when needed.
        if found.is_none() {
            found = combine(&full.solutions, &c_target);
        }
        let x = found.ok_or_else(|| {
            format!("the recurrence coefficients at n = {n0} are not a telescoping solution")
        })?;
        let num = full.b_prev.mul(&x);
        let den = full.normal_c.mul(&full.bp);
        if den.is_zero() {
            return Err(format!("degenerate certificate denominator at n = {n0}"));
        }
        let g = num.gcd(&den);
        let (num, den) = if g.is_zero() {
            (num, den)
        } else {
            match (num.div_rem(&g), den.div_rem(&g)) {
                (Some((a, _)), Some((b, _))) => (a, b),
                _ => (num, den),
            }
        };
        let lead = den
            .lead()
            .cloned()
            .ok_or_else(|| "zero denominator".to_string())?;
        let inv = BigRational::one() / lead;
        per_sample.push((nq, num.scale(&inv), den.scale(&inv)));
    }
    // 2. Each coefficient of the numerator and denominator as a rational function of n.
    let xs: Vec<BigRational> = per_sample.iter().map(|s| s.0.clone()).collect();
    let deg_num = per_sample
        .iter()
        .filter_map(|s| s.1.degree())
        .max()
        .unwrap_or(0);
    let deg_den = per_sample
        .iter()
        .filter_map(|s| s.2.degree())
        .max()
        .unwrap_or(0);
    if per_sample.iter().any(|s| s.2.degree() != Some(deg_den)) {
        return Err("the certificate denominator changes degree between samples".into());
    }
    let mut fits_num = Vec::new();
    for j in 0..=deg_num {
        let ys: Vec<BigRational> = per_sample.iter().map(|s| s.1.coeff(j)).collect();
        fits_num.push(
            rational_fit(&xs, &ys).ok_or_else(|| {
                format!("no rational function of n fits numerator coefficient {j}")
            })?,
        );
    }
    let mut fits_den = Vec::new();
    for j in 0..=deg_den {
        let ys: Vec<BigRational> = per_sample.iter().map(|s| s.2.coeff(j)).collect();
        fits_den.push(rational_fit(&xs, &ys).ok_or_else(|| {
            format!("no rational function of n fits denominator coefficient {j}")
        })?);
    }
    // Clear the denominators in n: multiply U and V by the lcm of all the Q's.
    let mut lcm = QPoly::one();
    for (_, q) in fits_num.iter().chain(fits_den.iter()) {
        let g = lcm.gcd(q);
        if let Some((quotient, _)) = q.div_rem(&g) {
            lcm = lcm.mul(&quotient);
        }
    }
    let build = |fits: &[(QPoly, QPoly)]| -> Result<Poly2, String> {
        let mut total = Poly2::zero();
        for (j, (p, q)) in fits.iter().enumerate() {
            let (scale, rem) = lcm.div_rem(q).ok_or("zero denominator")?;
            if !rem.is_zero() {
                return Err("internal error: lcm is not a multiple".into());
            }
            let coefficient = Poly2::from_n(&p.mul(&scale));
            let k_power = Poly2::from_terms(&[(j, 0, 1)]);
            total = total.add(&coefficient.mul(&k_power));
        }
        Ok(total)
    };
    let (u, v) = (build(&fits_num)?, build(&fits_den)?);

    // 3. The symbolic identity.
    check_certificate(term, rec, &u, &v)?;
    Ok(Proof {
        recurrence: rec.clone(),
        certificate_numerator: u,
        certificate_denominator: v,
    })
}

/// A solution `x` whose `c` part equals `target`, as a linear combination of the basis.
fn combine(solutions: &[(Vec<BigRational>, QPoly)], target: &[BigRational]) -> Option<QPoly> {
    let m = solutions.len();
    if m == 0 {
        return None;
    }
    // Solve sum_s w_s c_s = target: nullspace of [c_s columns | -target].
    let dim = target.len();
    let matrix: Vec<Vec<BigRational>> = (0..dim)
        .map(|i| {
            let mut row: Vec<BigRational> = solutions.iter().map(|(c, _)| c[i].clone()).collect();
            row.push(-target[i].clone());
            row
        })
        .collect();
    for v in null_space(matrix, m + 1) {
        let last = v[m].clone();
        if last.is_zero() {
            continue;
        }
        let inv = BigRational::one() / last;
        let mut x = QPoly::zero();
        for (w, (_, xs)) in v[..m].iter().zip(solutions) {
            x = x.add(&xs.scale(&(w * &inv)));
        }
        return Some(x);
    }
    None
}

/// Check the candidate certificate `R = u / v` symbolically (no sampling): `Ok` only if
/// `sum_i c_i(n) F(n+i, k) = F(n, k+1) R(n, k+1) - F(n, k) R(n, k)` holds as an identity in `(n, k)`.
///
/// Equivalently, `sum_i c_i(n) N_i / Bp = (A / B) U(k+1) / V(k+1) - U / V` as an identity of polynomials,
/// where `F(n+i, k) = F(n, k) N_i / Bp`.
pub fn check_certificate(
    term: &SymbolicTerm,
    rec: &Recurrence,
    u: &Poly2,
    v: &Poly2,
) -> Result<(), String> {
    let order = rec.order();
    // F(n+j+1, k) / F(n+j, k) = C(n+j, k) / D(n+j, k).
    let shifts: Vec<(Poly2, Poly2)> = (0..order as i64)
        .map(|j| (term.ratio_n.0.shift_n(j), term.ratio_n.1.shift_n(j)))
        .collect();
    let one = Poly2::from_terms(&[(0, 0, 1)]);
    let bp = shifts.iter().fold(one.clone(), |acc, (_, d)| acc.mul(d));
    let mut numerator = Poly2::zero();
    for (i, c) in rec.coefficients.iter().enumerate() {
        let mut ni = one.clone();
        for (j, (a, b)) in shifts.iter().enumerate() {
            ni = ni.mul(if j < i { a } else { b });
        }
        numerator = numerator.add(&Poly2::from_n(c).mul(&ni));
    }
    let (a, b) = (&term.ratio_k.0, &term.ratio_k.1);
    let (u1, v1) = (u.shift_k(1), v.shift_k(1));
    // Right side: (A U1 V - B U V1) / (B V1 V).
    let rhs_num = a.mul(&u1).mul(v).sub(&b.mul(u).mul(&v1));
    let rhs_den = b.mul(&v1).mul(v);
    // numerator / bp == rhs_num / rhs_den
    let residual = numerator.mul(&rhs_den).sub(&rhs_num.mul(&bp));
    if residual.is_zero() {
        Ok(())
    } else {
        Err("the guessed certificate does not satisfy the telescoping identity symbolically".into())
    }
}
