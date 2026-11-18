//! # Zeilberger's creative telescoping, by specialisation and interpolation
//!
//! For a hypergeometric term `F(n, k)` this finds a recurrence in `n` for the definite sum
//! `S(n) = sum_k F(n, k)`,
//!
//! ```text
//! c_0(n) S(n) + c_1(n) S(n+1) + ... + c_r(n) S(n+r) = 0,
//! ```
//!
//! together with the telescoping identity behind it,
//! `sum_i c_i(n) F(n+i, k) = G(n, k+1) - G(n, k)`, summed over `k` (the boundary terms must vanish,
//! as they do for terms with finite support such as binomial coefficients).
//!
//! **How it works.** For one fixed integer `n` the problem is univariate in `k` with exact
//! rational coefficients: write `F(n+i, k) = F(n, k) N_i(k) / B(k)`, put the common denominator
//! into the Gosper normal form of `F(n, k) / B(k)` (see [`crate::gosper`]), and the identity
//! becomes the linear system `a(k) x(k+1) - b(k-1) x(k) = c(k) sum_i c_i N_i(k)` in the unknowns
//! `c_i` and the coefficients of a polynomial `x`. Its solutions are computed exactly. The
//! dependence of the `c_i` on `n` is then recovered by requiring the coefficient vector at every
//! sampled `n` to be proportional to the solution found there (a linear condition on the
//! coefficients of polynomials `c_i(n)`).
//!
//! **What is, and is not, established.** At each sampled `n` the telescoping identity is verified
//! exactly (a polynomial identity in `k` over the rationals). The polynomials `c_i(n)` are
//! interpolated from these samples; requiring more samples than unknowns makes the fit
//! over-determined, but the recurrence is *proved* only at the sampled `n`. For a proof for all
//! `n` see [`crate::zeilberger_proof`], which rebuilds the certificate as a rational function of
//! `n` and checks the telescoping identity symbolically. Tests also check the recurrence against
//! brute-force sums at `n` well beyond the samples.

use crate::gosper::{QPoly, gosper_normal_form};
use crate::guess::{Recurrence, null_space};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};

/// A shift ratio as a function of `n`: `(numerator, denominator)` polynomials in `k`.
pub type ShiftRatio = Box<dyn Fn(&BigRational) -> (QPoly, QPoly)>;

/// The data of a hypergeometric term `F(n, k)`: its two shift ratios as functions of an integer
/// `n`, each a ratio of polynomials in `k`.
pub struct HypergeometricTerm {
    /// `F(n, k+1) / F(n, k)` as `(numerator, denominator)` polynomials in `k`, for the given `n`.
    pub ratio_k: ShiftRatio,
    /// `F(n+1, k) / F(n, k)` as `(numerator, denominator)` polynomials in `k`, for the given `n`.
    pub ratio_n: ShiftRatio,
}

fn rat(n: i64) -> BigRational {
    BigRational::from_integer(BigInt::from(n))
}

/// The recurrence coefficient vectors `(c_0, .., c_order)` for which creative telescoping succeeds
/// at the fixed integer `n`, as a basis of the solution space (empty if none).
///
/// Every returned vector comes with an exact certificate: `sum_i c_i F(n+i, k) = G(k+1) - G(k)`
/// for a hypergeometric `G(k) = F(n, k) b(k-1) x(k) / (c(k) B(k))`.
pub fn telescoping_at(
    term: &HypergeometricTerm,
    n: i64,
    order: usize,
) -> Result<Vec<Vec<BigRational>>, String> {
    let full = telescoping_full(term, n, order)?;
    // Project onto the c part, keeping an independent set of non-zero vectors.
    let basis: Vec<Vec<BigRational>> = full
        .solutions
        .iter()
        .filter(|(c_part, _)| c_part.iter().any(|x| !x.is_zero()))
        .map(|(c_part, _)| c_part.clone())
        .collect();
    // Reduce to a row-echelon basis of the span.
    Ok(null_space_of_rowspace(&basis, order + 1))
}

/// The solutions of the telescoping system at one `n`, with the data needed to rebuild the
/// certificate: `sum_i c_i F(n+i, k) = G(k+1) - G(k)` with `G = F * R` and
/// `R(k) = b(k-1) x(k) / (c(k) bp(k))`.
pub(crate) struct TelescopingSolutions {
    /// Basis of the solution space: `(c_0..c_order, x(k))`.
    pub solutions: Vec<(Vec<BigRational>, QPoly)>,
    /// `b(k - 1)` of the Gosper normal form.
    pub b_prev: QPoly,
    /// The `c(k)` of the Gosper normal form.
    pub normal_c: QPoly,
    /// The common denominator `bp(k)` with `F(n+i, k) = F(n, k) N_i(k) / bp(k)`.
    pub bp: QPoly,
}

pub(crate) fn telescoping_full(
    term: &HypergeometricTerm,
    n: i64,
    order: usize,
) -> Result<TelescopingSolutions, String> {
    let nq = rat(n);
    let (p, q) = (term.ratio_k)(&nq);
    // Shift ratios F(n+j+1, k) / F(n+j, k) = A_j / B_j for j < order.
    let shifts: Vec<(QPoly, QPoly)> = (0..order as i64)
        .map(|j| (term.ratio_n)(&rat(n + j)))
        .collect();
    // F(n+i, k) = F(n, k) N_i / Bp with N_i = prod_{j<i} A_j prod_{i<=j<order} B_j.
    let bp = shifts.iter().fold(QPoly::one(), |acc, (_, b)| acc.mul(b));
    let numerators: Vec<QPoly> = (0..=order)
        .map(|i| {
            let mut v = QPoly::one();
            for (j, (a, b)) in shifts.iter().enumerate() {
                v = v.mul(if j < i { a } else { b });
            }
            v
        })
        .collect();
    // u(k) = F(n, k) / Bp(k) has ratio P(k) Bp(k) / (Q(k) Bp(k+1)).
    let one = BigRational::one();
    let (a, b, c) = gosper_normal_form(&p.mul(&bp), &q.mul(&bp.shift(&one)))?;
    let b_prev = b.shift(&rat(-1));
    let degree_n = numerators
        .iter()
        .filter_map(|v| v.degree())
        .max()
        .unwrap_or(0);
    let degree_x = c.degree().unwrap_or(0)
        + degree_n
        + a.degree().unwrap_or(0).max(b.degree().unwrap_or(0))
        + 2;

    // Unknowns: x_0..x_degree_x, then c_0..c_order. Row m collects the coefficient of k^m in
    //   a(k) x(k+1) - b(k-1) x(k) - c(k) sum_i c_i N_i(k) = 0.
    let rows = c.degree().unwrap_or(0)
        + degree_n
        + degree_x
        + a.degree().unwrap_or(0).max(b.degree().unwrap_or(0))
        + 2;
    let mut columns: Vec<Vec<BigRational>> = Vec::new();
    let mut up = QPoly::one(); // (k + 1)^j
    let mut pw = QPoly::one(); // k^j
    for _ in 0..=degree_x {
        let col = a.mul(&up).sub(&b_prev.mul(&pw));
        columns.push((0..rows).map(|m| col.coeff(m)).collect());
        up = up.mul(&QPoly::from_ints(&[1, 1]));
        pw = pw.mul(&QPoly::k());
    }
    for v in &numerators {
        let col = c.mul(v).scale(&(-BigRational::one()));
        columns.push((0..rows).map(|m| col.coeff(m)).collect());
    }
    let unknowns = columns.len();
    let matrix: Vec<Vec<BigRational>> = (0..rows)
        .map(|m| (0..unknowns).map(|j| columns[j][m].clone()).collect())
        .collect();
    let null = null_space(matrix, unknowns);
    let solutions = null
        .into_iter()
        .map(|v| {
            (
                v[degree_x + 1..].to_vec(),
                QPoly::new(v[..=degree_x].to_vec()),
            )
        })
        .collect();
    Ok(TelescopingSolutions {
        solutions,
        b_prev,
        normal_c: c,
        bp,
    })
}

/// An independent basis (reduced row echelon form) of the span of `vectors` in `Q^dim`.
fn null_space_of_rowspace(vectors: &[Vec<BigRational>], dim: usize) -> Vec<Vec<BigRational>> {
    let mut m: Vec<Vec<BigRational>> = vectors.to_vec();
    let rows = m.len();
    let mut r = 0;
    for col in 0..dim {
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
                for j in 0..dim {
                    let delta = &f * &m[r][j];
                    m[i][j] -= delta;
                }
            }
        }
        r += 1;
        if r == rows {
            break;
        }
    }
    m.truncate(r);
    m
}

/// A recurrence found by [`zeilberger`], with the sample points it was fitted and checked on.
#[derive(Debug, Clone, PartialEq)]
pub struct ZeilbergerResult {
    /// The recurrence `sum_i c_i(n) S(n+i) = 0`.
    pub recurrence: Recurrence,
    /// The values of `n` at which the telescoping identity was solved exactly.
    pub certified_at: Vec<i64>,
}

/// Find a recurrence of the given `order` with coefficients of degree at most `degree` in `n`,
/// from the exact telescoping solutions at each `n` in `samples`.
///
/// Returns `Ok(None)` if the samples admit no such recurrence (the term has no telescoping
/// relation of this shape, or the bounds are too small), and `Err` if there are too few samples
/// (fewer than `(order + 1)(degree + 1)` plus a few spare) or a sample has no usable solution.
pub fn zeilberger(
    term: &HypergeometricTerm,
    samples: &[i64],
    order: usize,
    degree: usize,
) -> Result<Option<ZeilbergerResult>, String> {
    let unknowns = (order + 1) * (degree + 1);
    // Each sample contributes up to `order` independent proportionality equations.
    if samples.is_empty() {
        return Err("no sample points".into());
    }
    // For each sample the solution space V_n (it may have dimension above one at a fixed n);
    // c(n) must lie in it, i.e. be orthogonal to every vector w of its orthogonal complement.
    let mut conditions: Vec<(i64, Vec<Vec<BigRational>>)> = Vec::new();
    for &n in samples {
        let space = telescoping_at(term, n, order)?;
        if space.is_empty() {
            return Ok(None);
        }
        let complement = null_space(space, order + 1);
        if !complement.is_empty() {
            conditions.push((n, complement));
        }
    }
    let equations: usize = conditions.iter().map(|(_, c)| c.len()).sum();
    if equations < unknowns + 3 {
        return Err(format!(
            "only {equations} independent conditions for {unknowns} unknown coefficients"
        ));
    }
    let mut rows: Vec<Vec<BigRational>> = Vec::new();
    for (n, complement) in &conditions {
        let nq = rat(*n);
        let mut powers = vec![BigRational::one()];
        for _ in 0..degree {
            let next = powers.last().cloned().unwrap_or_else(BigRational::one) * &nq;
            powers.push(next);
        }
        for w in complement {
            let mut row = vec![BigRational::zero(); unknowns];
            for i in 0..=order {
                for (e, pw) in powers.iter().enumerate() {
                    row[i * (degree + 1) + e] += pw * &w[i];
                }
            }
            rows.push(row);
        }
    }
    let basis = null_space(rows, unknowns);
    // A usable solution has a non-zero top coefficient polynomial.
    for w in basis {
        let coefficients: Vec<QPoly> = (0..=order)
            .map(|i| QPoly::new(w[i * (degree + 1)..(i + 1) * (degree + 1)].to_vec()))
            .collect();
        if coefficients[order].is_zero() {
            continue;
        }
        let lead = coefficients[order]
            .lead()
            .cloned()
            .unwrap_or_else(BigRational::one);
        let inv = BigRational::one() / lead;
        let coefficients = coefficients.iter().map(|c| c.scale(&inv)).collect();
        return Ok(Some(ZeilbergerResult {
            recurrence: Recurrence { coefficients },
            certified_at: samples.to_vec(),
        }));
    }
    Ok(None)
}
