//! Exact projection and root isolation for Cylindrical Algebraic Decomposition in two variables.
//!
//! The projection follows McCallum's operator for a set `A` of polynomials in `Q[x][y]`
//! (projecting `R^2` onto `R^1`): the leading coefficient of each `f` in `y` (a constant `f`, of
//! degree 0 in `y`, contributes itself), the discriminant of each `f`, and the resultant of each
//! pair. Over the projected line the roots of these polynomials
//! are the only points where the *number* or *order* of the real roots of the fibres can change,
//! or where a fibre degenerates, so the cells they cut out are sign-invariant for `A`. Everything
//! is computed with exact rational arithmetic; real roots are isolated with Sturm sequences.
//!
//! Limits: two variables only; the input polynomials must be squarefree in `y` and pairwise
//! without a common factor in `y` (otherwise a discriminant or resultant vanishes identically
//! and the operator is not defined), which is reported as an error rather than guessed around.

use crate::gosper::QPoly;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};

fn rat(n: i64) -> BigRational {
    BigRational::from_integer(BigInt::from(n))
}

/// A polynomial in `y` whose coefficients are polynomials in `x`: `coeffs[i]` multiplies `y^i`.
#[derive(Debug, Clone, PartialEq)]
pub struct BiPoly {
    pub coeffs: Vec<QPoly>,
}

impl BiPoly {
    /// Degree in `y` (`None` for the zero polynomial).
    pub fn degree_y(&self) -> Option<usize> {
        let mut d = self.coeffs.len();
        while d > 0 && self.coeffs[d - 1].is_zero() {
            d -= 1;
        }
        d.checked_sub(1)
    }

    /// Largest degree in `x` among the coefficients.
    pub fn degree_x(&self) -> usize {
        self.coeffs
            .iter()
            .filter_map(|c| c.degree())
            .max()
            .unwrap_or(0)
    }

    /// The polynomial in `y` obtained by substituting `x = value`.
    pub fn at_x(&self, value: &BigRational) -> QPoly {
        QPoly::new(self.coeffs.iter().map(|c| c.eval(value)).collect())
    }

    /// Partial derivative with respect to `y`.
    pub fn derivative_y(&self) -> BiPoly {
        BiPoly {
            coeffs: self
                .coeffs
                .iter()
                .enumerate()
                .skip(1)
                .map(|(i, c)| c.scale(&rat(i as i64)))
                .collect(),
        }
    }
}

/// Exact determinant by Gaussian elimination over the rationals.
fn determinant(mut m: Vec<Vec<BigRational>>) -> BigRational {
    let n = m.len();
    let mut det = BigRational::one();
    for col in 0..n {
        let Some(p) = (col..n).find(|&i| !m[i][col].is_zero()) else {
            return BigRational::zero();
        };
        if p != col {
            m.swap(p, col);
            det = -det;
        }
        det *= m[col][col].clone();
        for i in col + 1..n {
            let f = m[i][col].clone() / m[col][col].clone();
            for j in col..n {
                let delta = &f * &m[col][j];
                m[i][j] -= delta;
            }
        }
    }
    det
}

/// Sylvester resultant of two polynomials in `y` with the given *formal* degrees.
fn sylvester_resultant(a: &[BigRational], b: &[BigRational], da: usize, db: usize) -> BigRational {
    let n = da + db;
    if n == 0 {
        return BigRational::one();
    }
    let mut m = vec![vec![BigRational::zero(); n]; n];
    for i in 0..db {
        for (j, c) in a.iter().enumerate().take(da + 1) {
            m[i][i + da - j] = c.clone();
        }
    }
    for i in 0..da {
        for (j, c) in b.iter().enumerate().take(db + 1) {
            m[db + i][i + db - j] = c.clone();
        }
    }
    determinant(m)
}

/// Newton interpolation through `(0, v0), (1, v1), ...`.
fn interpolate(values: &[BigRational]) -> QPoly {
    let n = values.len();
    let mut table = values.to_vec();
    let mut newton = vec![table[0].clone()];
    for level in 1..n {
        for i in (level..n).rev() {
            table[i] = (table[i].clone() - table[i - 1].clone()) / rat(level as i64);
        }
        newton.push(table[level].clone());
    }
    // p(x) = sum_k newton[k] x (x - 1) ... (x - k + 1)
    let mut result = QPoly::zero();
    let mut basis = QPoly::one();
    for (k, c) in newton.iter().enumerate() {
        result = result.add(&basis.scale(c));
        basis = basis.mul(&QPoly::new(vec![rat(-(k as i64)), BigRational::one()]));
    }
    result
}

/// The resultant of `f` and `g` with respect to `y`, as a polynomial in `x`.
pub fn resultant_y(f: &BiPoly, g: &BiPoly) -> Option<QPoly> {
    let (df, dg) = (f.degree_y()?, g.degree_y()?);
    let bound = dg * f.degree_x() + df * g.degree_x();
    let values: Vec<BigRational> = (0..=bound as i64)
        .map(|x0| {
            let x0 = rat(x0);
            let a: Vec<BigRational> = (0..=df).map(|i| f.coeffs[i].eval(&x0)).collect();
            let b: Vec<BigRational> = (0..=dg).map(|i| g.coeffs[i].eval(&x0)).collect();
            sylvester_resultant(&a, &b, df, dg)
        })
        .collect();
    Some(interpolate(&values))
}

/// The McCallum projection of `polys` onto the `x` line: the non-constant polynomials among the
/// leading coefficients, discriminants (`res(f, f_y)`) and pairwise resultants. In two variables
/// the leading coefficient suffices: a fibre in which `f` vanishes identically needs *every*
/// coefficient to vanish, in particular the leading one. `Err` if a discriminant or resultant vanishes identically (a repeated or common
/// factor in `y`).
pub fn project(polys: &[BiPoly]) -> Result<Vec<QPoly>, String> {
    let mut out: Vec<QPoly> = Vec::new();
    let mut push = |p: QPoly| {
        if p.degree().is_some_and(|d| d > 0) {
            out.push(p);
        }
    };
    for f in polys {
        let Some(df) = f.degree_y() else { continue };
        push(f.coeffs[df].clone());
        if df >= 2 {
            let disc = resultant_y(f, &f.derivative_y()).unwrap_or_else(QPoly::zero);
            if disc.is_zero() {
                return Err(
                    "a polynomial has a repeated factor in y (its discriminant is \
                            identically zero); pass a squarefree polynomial"
                        .into(),
                );
            }
            push(disc);
        }
    }
    for (i, f) in polys.iter().enumerate() {
        for g in polys.iter().skip(i + 1) {
            if f.degree_y().is_none_or(|d| d == 0) || g.degree_y().is_none_or(|d| d == 0) {
                continue;
            }
            let res = resultant_y(f, g).unwrap_or_else(QPoly::zero);
            if res.is_zero() {
                return Err(
                    "two polynomials have a common factor in y (their resultant is \
                            identically zero); pass a coprime set"
                        .into(),
                );
            }
            push(res);
        }
    }
    Ok(out)
}

/// The squarefree part of `p`: `p / gcd(p, p')`.
pub fn squarefree_part(p: &QPoly) -> QPoly {
    let derivative = QPoly::new(
        (1..=p.degree().unwrap_or(0))
            .map(|i| p.coeff(i) * rat(i as i64))
            .collect(),
    );
    let g = p.gcd(&derivative);
    if g.degree().is_none_or(|d| d == 0) {
        return p.clone();
    }
    p.div_rem(&g).map(|(q, _)| q).unwrap_or_else(|| p.clone())
}

/// Replace `polys` by squarefree, pairwise coprime, non-constant polynomials whose roots together
/// are exactly the roots of the originals.
pub fn coprime_basis(polys: &[QPoly]) -> Vec<QPoly> {
    let mut basis: Vec<QPoly> = polys
        .iter()
        .filter(|p| p.degree().is_some_and(|d| d > 0))
        .map(squarefree_part)
        .collect();
    loop {
        let mut changed = false;
        'outer: for i in 0..basis.len() {
            for j in i + 1..basis.len() {
                let g = basis[i].gcd(&basis[j]);
                if g.degree().is_some_and(|d| d > 0) {
                    let a = basis[i]
                        .div_rem(&g)
                        .map(|(q, _)| q)
                        .unwrap_or_else(QPoly::zero);
                    let b = basis[j]
                        .div_rem(&g)
                        .map(|(q, _)| q)
                        .unwrap_or_else(QPoly::zero);
                    let mut next: Vec<QPoly> = basis
                        .iter()
                        .enumerate()
                        .filter(|(k, _)| *k != i && *k != j)
                        .map(|(_, p)| p.clone())
                        .collect();
                    next.extend(
                        [a, g, b]
                            .into_iter()
                            .filter(|p| p.degree().is_some_and(|d| d > 0)),
                    );
                    basis = next;
                    changed = true;
                    break 'outer;
                }
            }
        }
        if !changed {
            return basis;
        }
    }
}

/// The Sturm chain of a squarefree polynomial.
fn sturm_chain(p: &QPoly) -> Vec<QPoly> {
    let derivative = QPoly::new(
        (1..=p.degree().unwrap_or(0))
            .map(|i| p.coeff(i) * rat(i as i64))
            .collect(),
    );
    let mut chain = vec![p.clone(), derivative];
    while let [.., a, b] = chain.as_slice() {
        if b.is_zero() {
            chain.pop();
            break;
        }
        let Some((_, r)) = a.div_rem(b) else { break };
        if r.is_zero() {
            break;
        }
        let next = r.scale(&-BigRational::one());
        chain.push(next);
    }
    chain
}

fn sign_changes(chain: &[QPoly], at: &BigRational) -> usize {
    let mut last = 0i32;
    let mut changes = 0;
    for p in chain {
        let v = p.eval(at);
        let s = if v.is_zero() {
            0
        } else if v.is_positive() {
            1
        } else {
            -1
        };
        if s != 0 {
            if last != 0 && s != last {
                changes += 1;
            }
            last = s;
        }
    }
    changes
}

/// Real roots of `p`, sorted, each isolated to width below `2^-70` and returned as the exact
/// rational midpoint of its isolating interval (a root that is itself rational and found at an
/// interval endpoint is returned exactly).
pub fn real_roots(p: &QPoly) -> Vec<BigRational> {
    let p = squarefree_part(p);
    let Some(d) = p.degree().filter(|&d| d > 0) else {
        return Vec::new();
    };
    let lead = p.lead().cloned().unwrap_or_else(BigRational::one);
    // Cauchy bound on the magnitude of every root.
    let bound = (0..d)
        .map(|i| (p.coeff(i) / lead.clone()).abs())
        .fold(BigRational::zero(), |m, v| if v > m { v } else { m })
        + BigRational::one();
    let chain = sturm_chain(&p);
    let count =
        |a: &BigRational, b: &BigRational| sign_changes(&chain, a) - sign_changes(&chain, b);
    let mut roots = Vec::new();
    // Interval stack of (lo, hi, expected number of roots in (lo, hi]).
    let (lo, hi) = (-bound.clone(), bound);
    let total = count(&lo, &hi);
    let mut stack = vec![(lo, hi, total)];
    let width = BigRational::new(BigInt::from(1), BigInt::from(1) << 70usize);
    while let Some((a, b, n)) = stack.pop() {
        if n == 0 {
            continue;
        }
        if p.eval(&b).is_zero() && n == 1 {
            roots.push(b);
            continue;
        }
        if n == 1 && &b - &a < width {
            roots.push((a + b) / rat(2));
            continue;
        }
        let mid = (a.clone() + b.clone()) / rat(2);
        let left = count(&a, &mid);
        stack.push((a, mid.clone(), left));
        stack.push((mid, b, n - left));
    }
    roots.sort();
    roots
}

/// Largest absolute value among `values` as an `f64` (for tolerances).
pub fn scale_of(p: &QPoly) -> f64 {
    (0..=p.degree().unwrap_or(0))
        .filter_map(|i| p.coeff(i).abs().to_f64())
        .fold(1.0, f64::max)
}
