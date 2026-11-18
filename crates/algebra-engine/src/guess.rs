//! # Recurrence guessing
//!
//! Given exact values `S(n0), S(n0+1), ...` of a sequence (typically a definite sum
//! `S(n) = sum_k F(n, k)` of a hypergeometric term), find a linear recurrence with polynomial
//! coefficients
//!
//! ```text
//! c_0(n) S(n) + c_1(n) S(n+1) + ... + c_r(n) S(n+r) = 0,    deg c_i <= d.
//! ```
//!
//! This is *guessing*, not proving: the coefficients are the solution of a linear system over the
//! rationals built from the data, and the result is accepted only if the system has strictly more
//! equations than unknowns (so that a spurious fit is unlikely: at least
//! [`MIN_EXTRA_EQUATIONS`] equations are left over). A recurrence returned here holds on all the
//! supplied values; it is a conjecture beyond them. Proving it is what Zeilberger's creative
//! telescoping does, which this crate does not implement (see [`crate::gosper`] for the
//! indefinite-summation half).

use crate::gosper::QPoly;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};

/// Equations required beyond the number of unknowns before a fit is believed.
pub const MIN_EXTRA_EQUATIONS: usize = 3;

/// A guessed recurrence `sum_i coefficients[i](n) S(n + i) = 0`.
#[derive(Debug, Clone, PartialEq)]
pub struct Recurrence {
    /// `coefficients[i]` multiplies `S(n + i)`.
    pub coefficients: Vec<QPoly>,
}

impl Recurrence {
    /// Order `r` (the highest shift with a non-zero coefficient).
    pub fn order(&self) -> usize {
        self.coefficients.len().saturating_sub(1)
    }

    /// Largest coefficient degree.
    pub fn degree(&self) -> usize {
        self.coefficients
            .iter()
            .filter_map(|c| c.degree())
            .max()
            .unwrap_or(0)
    }

    /// Whether the recurrence holds at index `n` of `values` (`values[n - n0]` is `S(n)`).
    pub fn holds_at(&self, values: &[BigRational], n0: i64, n: i64) -> bool {
        let index = (n - n0) as usize;
        if index + self.order() >= values.len() {
            return false;
        }
        let point = BigRational::from_integer(BigInt::from(n));
        let total = self
            .coefficients
            .iter()
            .enumerate()
            .fold(BigRational::zero(), |acc, (i, c)| {
                acc + c.eval(&point) * &values[index + i]
            });
        total.is_zero()
    }
}

/// Null space of `rows x cols` matrix `m` (rational), as a list of basis vectors.
fn null_space(mut m: Vec<Vec<BigRational>>, cols: usize) -> Vec<Vec<BigRational>> {
    let rows = m.len();
    let mut pivot_cols: Vec<usize> = Vec::new();
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
                for j in 0..cols {
                    let delta = &f * &m[r][j];
                    m[i][j] -= delta;
                }
            }
        }
        pivot_cols.push(col);
        r += 1;
        if r == rows {
            break;
        }
    }
    let free: Vec<usize> = (0..cols).filter(|c| !pivot_cols.contains(c)).collect();
    free.iter()
        .map(|&f| {
            let mut v = vec![BigRational::zero(); cols];
            v[f] = BigRational::one();
            for (row, &pc) in pivot_cols.iter().enumerate() {
                v[pc] = -m[row][f].clone();
            }
            v
        })
        .collect()
}

/// Find a recurrence of exactly the given `order` and coefficient degree at most `degree`, or
/// `Ok(None)` if the data admits none, or `Err` if there are too few values to decide (fewer than
/// `(order + 1)(degree + 1) + MIN_EXTRA_EQUATIONS` equations).
///
/// When the solution space has dimension above one (a lower-order or lower-degree recurrence also
/// fits) the one found is not unique; use [`guess_minimal_recurrence`] to get the smallest.
pub fn guess_recurrence(
    values: &[BigRational],
    n0: i64,
    order: usize,
    degree: usize,
) -> Result<Option<Recurrence>, String> {
    let unknowns = (order + 1) * (degree + 1);
    let equations = values.len().saturating_sub(order);
    if equations < unknowns + MIN_EXTRA_EQUATIONS {
        return Err(format!(
            "{} values give {equations} equations for {unknowns} unknowns; need at least {}",
            values.len(),
            unknowns + MIN_EXTRA_EQUATIONS
        ));
    }
    // Row for index n: sum over (i, j) of n^j S(n + i) * c_{ij}.
    let matrix: Vec<Vec<BigRational>> = (0..equations)
        .map(|e| {
            let n = BigRational::from_integer(BigInt::from(n0 + e as i64));
            let mut row = Vec::with_capacity(unknowns);
            for i in 0..=order {
                let mut power = BigRational::one();
                for _ in 0..=degree {
                    row.push(&power * &values[e + i]);
                    power *= &n;
                }
            }
            row
        })
        .collect();
    let basis = null_space(matrix, unknowns);
    // Require a solution that really uses the top order (otherwise a lower order was found).
    for vector in basis {
        let coefficients: Vec<QPoly> = (0..=order)
            .map(|i| QPoly::new(vector[i * (degree + 1)..(i + 1) * (degree + 1)].to_vec()))
            .collect();
        if coefficients[order].is_zero() {
            continue;
        }
        return Ok(Some(Recurrence { coefficients }));
    }
    Ok(None)
}

/// The recurrence of smallest order, then smallest degree, within the bounds, normalised so
/// that the leading coefficient of the top-order polynomial is 1. `Ok(None)` if there is none
/// with the available data; `Err` only if the data is too short even for order 1, degree 0.
pub fn guess_minimal_recurrence(
    values: &[BigRational],
    n0: i64,
    max_order: usize,
    max_degree: usize,
) -> Result<Option<Recurrence>, String> {
    let mut decided_any = false;
    let mut last_error = String::new();
    for order in 1..=max_order {
        for degree in 0..=max_degree {
            match guess_recurrence(values, n0, order, degree) {
                Ok(Some(mut rec)) => {
                    if let Some(lead) = rec.coefficients[order].lead().cloned() {
                        let inv = BigRational::one() / lead;
                        rec.coefficients = rec.coefficients.iter().map(|c| c.scale(&inv)).collect();
                    }
                    return Ok(Some(rec));
                }
                Ok(None) => decided_any = true,
                Err(e) => last_error = e,
            }
        }
    }
    if decided_any {
        Ok(None)
    } else {
        Err(last_error)
    }
}
