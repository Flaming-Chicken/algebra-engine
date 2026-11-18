//! # Gosper's algorithm
//!
//! Indefinite summation of hypergeometric terms in closed form, with exact rational arithmetic.
//!
//! A term `t(k)` is *hypergeometric* when `t(k+1) / t(k) = P(k) / Q(k)` is a rational function
//! of `k`. [`gosper`] decides whether `t` has a hypergeometric antidifference, that is a
//! rational function `R(k)` with
//!
//! ```text
//! z(k) = R(k) t(k),    z(k+1) - z(k) = t(k),
//! ```
//!
//! and returns `R` when it exists (so that `sum_{j=a}^{b-1} t(j) = z(b) - z(a)`). When no such
//! antidifference exists it says so: Gosper's algorithm is a decision procedure, so `None` is a
//! proof that the sum has no closed form of this kind (for example the harmonic numbers, or the
//! partial sums of the binomial coefficients).
//!
//! The steps follow Petkovšek, Wilf and Zeilberger, *A = B*, ch. 5:
//!
//! 1. Write the ratio as `a(k)/b(k) * c(k+1)/c(k)` with `gcd(a(k), b(k+h)) = 1` for every
//!    integer `h >= 0`, by cancelling each shift `h` at which the two share a factor.
//! 2. Find a polynomial `x(k)` with `a(k) x(k+1) - b(k-1) x(k) = c(k)` (a linear system over Q
//!    once the degree is bounded); then `R(k) = b(k-1) x(k) / c(k)`.
//!
//! This is the building block of Zeilberger's creative telescoping, which applies it to a term
//! with an extra parameter; creative telescoping itself is not implemented here.

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};

/// A univariate polynomial over the rationals, lowest degree first, with no trailing zeros.
#[derive(Debug, Clone, PartialEq)]
pub struct QPoly {
    coeffs: Vec<BigRational>,
}

fn rat(n: i64) -> BigRational {
    BigRational::from_integer(BigInt::from(n))
}

impl QPoly {
    /// The polynomial with the given integer coefficients, lowest degree first.
    pub fn from_ints(coeffs: &[i64]) -> Self {
        Self::new(coeffs.iter().map(|&c| rat(c)).collect())
    }

    /// The polynomial with the given rational coefficients, lowest degree first.
    pub fn new(mut coeffs: Vec<BigRational>) -> Self {
        while coeffs.last().is_some_and(|c| c.is_zero()) {
            coeffs.pop();
        }
        Self { coeffs }
    }

    /// The zero polynomial.
    pub fn zero() -> Self {
        Self { coeffs: Vec::new() }
    }

    /// The constant `1`.
    pub fn one() -> Self {
        Self::from_ints(&[1])
    }

    /// The polynomial `k`.
    pub fn k() -> Self {
        Self::from_ints(&[0, 1])
    }

    /// Whether this is the zero polynomial.
    pub fn is_zero(&self) -> bool {
        self.coeffs.is_empty()
    }

    /// Degree; `None` for the zero polynomial.
    pub fn degree(&self) -> Option<usize> {
        self.coeffs.len().checked_sub(1)
    }

    /// Coefficient of `k^i` (zero beyond the degree).
    pub fn coeff(&self, i: usize) -> BigRational {
        self.coeffs
            .get(i)
            .cloned()
            .unwrap_or_else(BigRational::zero)
    }

    /// Leading coefficient.
    pub fn lead(&self) -> Option<&BigRational> {
        self.coeffs.last()
    }

    /// Sum.
    pub fn add(&self, other: &Self) -> Self {
        let n = self.coeffs.len().max(other.coeffs.len());
        Self::new((0..n).map(|i| self.coeff(i) + other.coeff(i)).collect())
    }

    /// Difference.
    pub fn sub(&self, other: &Self) -> Self {
        let n = self.coeffs.len().max(other.coeffs.len());
        Self::new((0..n).map(|i| self.coeff(i) - other.coeff(i)).collect())
    }

    /// Product.
    pub fn mul(&self, other: &Self) -> Self {
        if self.is_zero() || other.is_zero() {
            return Self::zero();
        }
        let mut out = vec![BigRational::zero(); self.coeffs.len() + other.coeffs.len() - 1];
        for (i, a) in self.coeffs.iter().enumerate() {
            for (j, b) in other.coeffs.iter().enumerate() {
                out[i + j] += a * b;
            }
        }
        Self::new(out)
    }

    /// Multiply by a rational scalar.
    pub fn scale(&self, s: &BigRational) -> Self {
        Self::new(self.coeffs.iter().map(|c| c * s).collect())
    }

    /// Quotient and remainder of Euclidean division; `None` when dividing by zero.
    pub fn div_rem(&self, divisor: &Self) -> Option<(Self, Self)> {
        let dd = divisor.degree()?;
        let lead = divisor.lead()?.clone();
        let mut rem = self.clone();
        let mut quotient = vec![BigRational::zero(); self.coeffs.len().saturating_sub(dd).max(1)];
        while let Some(dr) = rem.degree() {
            if dr < dd {
                break;
            }
            let factor = rem.coeffs[dr].clone() / lead.clone();
            quotient[dr - dd] = factor.clone();
            let mut shifted = vec![BigRational::zero(); dr - dd];
            shifted.extend(divisor.coeffs.iter().map(|c| c * &factor));
            rem = rem.sub(&Self::new(shifted));
        }
        Some((Self::new(quotient), rem))
    }

    /// Monic greatest common divisor (the zero polynomial if both are zero).
    pub fn gcd(&self, other: &Self) -> Self {
        let (mut a, mut b) = (self.clone(), other.clone());
        while !b.is_zero() {
            let Some((_, r)) = a.div_rem(&b) else { break };
            a = b;
            b = r;
        }
        match a.lead().cloned() {
            Some(lead) => a.scale(&(BigRational::one() / lead)),
            None => a,
        }
    }

    /// `p(k + h)`.
    pub fn shift(&self, h: &BigRational) -> Self {
        // Horner on the polynomial `k + h`.
        let linear = Self::new(vec![h.clone(), BigRational::one()]);
        let mut out = Self::zero();
        for c in self.coeffs.iter().rev() {
            out = out.mul(&linear).add(&Self::new(vec![c.clone()]));
        }
        out
    }

    /// Value at a rational point.
    pub fn eval(&self, x: &BigRational) -> BigRational {
        self.coeffs
            .iter()
            .rev()
            .fold(BigRational::zero(), |acc, c| acc * x + c)
    }
}

/// The antidifference found by [`gosper`]: `z(k) = R(k) t(k)` with `R = numerator / denominator`.
#[derive(Debug, Clone, PartialEq)]
pub struct GosperSum {
    /// Numerator of `R(k)`.
    pub numerator: QPoly,
    /// Denominator of `R(k)`.
    pub denominator: QPoly,
}

impl GosperSum {
    /// `R(k)` at a rational point (`None` at a pole).
    pub fn eval(&self, k: &BigRational) -> Option<BigRational> {
        let d = self.denominator.eval(k);
        (!d.is_zero()).then(|| self.numerator.eval(k) / d)
    }
}

/// Largest shift searched for common factors; a larger bound is reported as an error rather than
/// guessed.
const MAX_SHIFT: i64 = 10_000;

/// Gosper's algorithm for the hypergeometric term with ratio `t(k+1)/t(k) = ratio_num / ratio_den`.
///
/// Returns `Ok(Some(R))` when `z(k) = R(k) t(k)` is an antidifference, `Ok(None)` when none
/// exists (the sum is not Gosper-summable), and `Err` for a zero denominator or when the shifts
/// to search exceed an internal bound.
pub fn gosper(ratio_num: &QPoly, ratio_den: &QPoly) -> Result<Option<GosperSum>, String> {
    if ratio_den.is_zero() {
        return Err("the ratio has a zero denominator".into());
    }
    if ratio_num.is_zero() {
        // t(k+1) = 0: the term vanishes after one step, so t itself telescopes with R = 1.
        return Ok(Some(GosperSum {
            numerator: QPoly::one(),
            denominator: QPoly::one(),
        }));
    }
    let (mut a, mut b, mut c) = (ratio_num.clone(), ratio_den.clone(), QPoly::one());

    // Step 1: cancel every non-negative integer shift h at which a(k) and b(k+h) share a factor.
    // Such an h is a difference of a root of a and a root of b, so it is at most the sum of their
    // Cauchy root bounds.
    let bound = |p: &QPoly| -> BigRational {
        match (p.degree(), p.lead()) {
            (Some(d), Some(lead)) if d > 0 => {
                let m = (0..d)
                    .map(|i| (p.coeff(i) / lead.clone()).abs())
                    .fold(BigRational::zero(), |acc, v| if v > acc { v } else { acc });
                m + BigRational::one()
            }
            _ => BigRational::zero(),
        }
    };
    let limit = (bound(&a) + bound(&b)).ceil().to_integer();
    let limit = limit.to_i64().filter(|l| *l <= MAX_SHIFT).ok_or_else(|| {
        "the coefficients are too large to bound the shifts to search".to_string()
    })?;
    for h in 0..=limit {
        let hh = rat(h);
        let g = a.gcd(&b.shift(&hh));
        if g.degree().is_none_or(|d| d == 0) {
            continue;
        }
        // a <- a / g,  b <- b / g(k - h),  c <- c * prod_{i=1}^{h} g(k - i).
        let g_back = g.shift(&rat(-h));
        a = a.div_rem(&g).map(|(q, _)| q).unwrap_or_else(QPoly::zero);
        b = b
            .div_rem(&g_back)
            .map(|(q, _)| q)
            .unwrap_or_else(QPoly::zero);
        for i in 1..=h {
            c = c.mul(&g.shift(&rat(-i)));
        }
    }

    // Step 2: a(k) x(k+1) - b(k-1) x(k) = c(k) for a polynomial x(k).
    let a_k = a.clone();
    let b_prev = b.shift(&rat(-1));
    let (da, db) = (a.degree().unwrap_or(0), b.degree().unwrap_or(0));
    let dc = c.degree().unwrap_or(0);
    let n = da.max(db);
    let mut candidates: Vec<usize> = Vec::new();
    if da != db || a.lead() != b.lead() {
        if dc >= n {
            candidates.push(dc - n);
        }
    } else {
        // Leading terms cancel in a(k) x(k+1) - b(k-1) x(k): the left side has degree at most
        // n + deg(x) - 1, so deg(x) can reach deg(c) - n + 1, and a special degree (the root of
        // the next-to-leading coefficient) may also solve it.
        if dc + 1 >= n {
            candidates.push(dc + 1 - n);
        }
        let diff = a_k.sub(&b_prev);
        if n >= 1 {
            let delta = diff.coeff(n - 1);
            let m0 = -(delta / a.lead().cloned().unwrap_or_else(BigRational::one));
            if m0.is_integer() && !m0.is_negative() {
                if let Some(m) = m0.to_integer().to_usize() {
                    candidates.push(m);
                }
            }
        }
    }
    let Some(degree) = candidates.into_iter().max() else {
        return Ok(None);
    };

    // Unknown coefficients x_0..x_degree; column j is a(k)(k+1)^j - b(k-1) k^j.
    let rows = dc.max(n + degree) + 1;
    let k = QPoly::k();
    let mut columns: Vec<Vec<BigRational>> = Vec::new();
    let mut power_next = QPoly::one(); // (k + 1)^j
    let mut power = QPoly::one(); // k^j
    for _ in 0..=degree {
        let column = a_k.mul(&power_next).sub(&b_prev.mul(&power));
        columns.push((0..rows).map(|i| column.coeff(i)).collect());
        power_next = power_next.mul(&QPoly::from_ints(&[1, 1]));
        power = power.mul(&k);
    }
    let rhs: Vec<BigRational> = (0..rows).map(|i| c.coeff(i)).collect();
    let Some(x) = solve_linear(&columns, &rhs) else {
        return Ok(None);
    };
    let x = QPoly::new(x);
    Ok(Some(GosperSum {
        numerator: b_prev.mul(&x),
        denominator: c,
    }))
}

/// Solve `sum_j x_j columns[j] = rhs` over the rationals by Gaussian elimination (free unknowns
/// set to zero). `None` if the system is inconsistent.
fn solve_linear(columns: &[Vec<BigRational>], rhs: &[BigRational]) -> Option<Vec<BigRational>> {
    let (rows, cols) = (rhs.len(), columns.len());
    let mut m: Vec<Vec<BigRational>> = (0..rows)
        .map(|i| {
            let mut row: Vec<BigRational> = columns.iter().map(|c| c[i].clone()).collect();
            row.push(rhs[i].clone());
            row
        })
        .collect();
    let mut pivots: Vec<usize> = Vec::new();
    let mut r = 0;
    for col in 0..cols {
        let Some(p) = (r..rows).find(|&i| !m[i][col].is_zero()) else {
            continue;
        };
        m.swap(r, p);
        let inv = BigRational::one() / m[r][col].clone();
        for v in m[r].iter_mut() {
            *v *= &inv;
        }
        for i in 0..rows {
            if i != r && !m[i][col].is_zero() {
                let f = m[i][col].clone();
                for j in col..=cols {
                    let delta = &f * &m[r][j];
                    m[i][j] -= delta;
                }
            }
        }
        pivots.push(col);
        r += 1;
        if r == rows {
            break;
        }
    }
    // A row with no pivot but a non-zero right-hand side is a contradiction.
    if (r..rows).any(|i| !m[i][cols].is_zero()) {
        return None;
    }
    let mut x = vec![BigRational::zero(); cols];
    for (row, &col) in pivots.iter().enumerate() {
        x[col] = m[row][cols].clone();
    }
    Some(x)
}
