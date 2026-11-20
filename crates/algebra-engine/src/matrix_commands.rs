//! Exact linear algebra on rational matrices written as literals (or stored matrix variables, expanded before this runs):
//! `rowspace`, `columnspace`, `adjugate`, `cofactor`, `minor`, `hadamard`, `is_symmetric`, `is_orthogonal`,
//! `is_positive_definite`, `pinv`, `lstsq`, `gram_schmidt`, `orthonormalize`, `projection`, `frobenius_norm`, `cond`,
//! `matrix_exp` (`expm`) and `eig`.

use crate::session::UraeSession;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};

type Q = BigRational;
type Matrix = Vec<Vec<Q>>;

fn split_top(text: &str) -> Vec<String> {
    let (mut depth, mut cur, mut out) = (0i32, String::new(), Vec::new());
    for c in text.chars() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            _ => {}
        }
        if c == ',' && depth == 0 {
            out.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    out.push(cur);
    out.into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn show_q(q: &Q) -> String {
    if q.is_integer() {
        q.numer().to_string()
    } else {
        format!("{}/{}", q.numer(), q.denom())
    }
}

/// A matrix literal `[[1,2],[3,4]]` (or a vector `[1,2]`, a column) as rational rows.
fn parse(session: &mut UraeSession, text: &str) -> Option<Matrix> {
    let inner = text.trim().strip_prefix('[')?.strip_suffix(']')?;
    let items = split_top(inner);
    let rows: Vec<Vec<String>> =
        if items.iter().all(|i| i.starts_with('[') && i.ends_with(']')) && !items.is_empty() {
            items
                .iter()
                .map(|row| split_top(&row[1..row.len() - 1]))
                .collect()
        } else {
            items.iter().map(|i| vec![i.clone()]).collect()
        };
    if rows.is_empty()
        || rows
            .iter()
            .any(|r| r.len() != rows[0].len() || r.is_empty())
    {
        return None;
    }
    rows.iter()
        .map(|row| {
            row.iter()
                .map(|c| crate::list_utils::rational(session, c))
                .collect::<Option<Vec<Q>>>()
        })
        .collect()
}

fn literal(m: &Matrix) -> String {
    let rows: Vec<String> = m
        .iter()
        .map(|r| format!("[{}]", r.iter().map(show_q).collect::<Vec<_>>().join(",")))
        .collect();
    format!("[{}]", rows.join(","))
}

fn display(session: &mut UraeSession, m: &Matrix) -> Result<String, String> {
    session.eval(&literal(m))
}

fn zeros(r: usize, c: usize) -> Matrix {
    vec![vec![Q::zero(); c]; r]
}

fn identity(n: usize) -> Matrix {
    let mut m = zeros(n, n);
    for (i, row) in m.iter_mut().enumerate() {
        row[i] = Q::one();
    }
    m
}

fn transpose(m: &Matrix) -> Matrix {
    (0..m[0].len())
        .map(|j| m.iter().map(|r| r[j].clone()).collect())
        .collect()
}

fn multiply(a: &Matrix, b: &Matrix) -> Option<Matrix> {
    if a[0].len() != b.len() {
        return None;
    }
    Some(
        a.iter()
            .map(|row| {
                (0..b[0].len())
                    .map(|j| {
                        row.iter()
                            .zip(b)
                            .fold(Q::zero(), |s, (x, brow)| s + x * &brow[j])
                    })
                    .collect()
            })
            .collect(),
    )
}

/// Reduced row echelon form and the pivot columns.
fn rref(m: &Matrix) -> (Matrix, Vec<usize>) {
    let mut a = m.clone();
    let (rows, cols) = (a.len(), a[0].len());
    let (mut row, mut pivots) = (0, Vec::new());
    for col in 0..cols {
        if row >= rows {
            break;
        }
        let Some(p) = (row..rows).find(|&r| !a[r][col].is_zero()) else {
            continue;
        };
        a.swap(row, p);
        let lead = a[row][col].clone();
        for c in 0..cols {
            a[row][c] = &a[row][c] / &lead;
        }
        for r in 0..rows {
            if r != row && !a[r][col].is_zero() {
                let f = a[r][col].clone();
                for c in 0..cols {
                    let delta = &f * &a[row][c];
                    a[r][c] -= delta;
                }
            }
        }
        pivots.push(col);
        row += 1;
    }
    (a, pivots)
}

fn inverse(m: &Matrix) -> Option<Matrix> {
    let n = m.len();
    if m[0].len() != n {
        return None;
    }
    let augmented: Matrix = m
        .iter()
        .enumerate()
        .map(|(i, r)| {
            r.iter()
                .cloned()
                .chain((0..n).map(|j| if i == j { Q::one() } else { Q::zero() }))
                .collect()
        })
        .collect();
    let (reduced, pivots) = rref(&augmented);
    if pivots.len() < n || pivots[n - 1] != n - 1 {
        return None;
    }
    Some(reduced.iter().map(|r| r[n..].to_vec()).collect())
}

fn determinant(m: &Matrix) -> Q {
    let n = m.len();
    let mut a = m.clone();
    let mut det = Q::one();
    for col in 0..n {
        let Some(p) = (col..n).find(|&r| !a[r][col].is_zero()) else {
            return Q::zero();
        };
        if p != col {
            a.swap(p, col);
            det = -det;
        }
        det *= a[col][col].clone();
        for r in col + 1..n {
            let f = &a[r][col] / &a[col][col];
            for c in col..n {
                let delta = &f * &a[col][c];
                a[r][c] -= delta;
            }
        }
    }
    det
}

fn minor_matrix(m: &Matrix, skip_row: usize, skip_col: usize) -> Matrix {
    m.iter()
        .enumerate()
        .filter(|(i, _)| *i != skip_row)
        .map(|(_, r)| {
            r.iter()
                .enumerate()
                .filter(|(j, _)| *j != skip_col)
                .map(|(_, v)| v.clone())
                .collect()
        })
        .collect()
}

fn cofactors(m: &Matrix) -> Matrix {
    let n = m.len();
    if n == 1 {
        return vec![vec![Q::one()]];
    }
    (0..n)
        .map(|i| {
            (0..n)
                .map(|j| {
                    let d = determinant(&minor_matrix(m, i, j));
                    if (i + j) % 2 == 0 { d } else { -d }
                })
                .collect()
        })
        .collect()
}

/// The Moore-Penrose pseudoinverse from a rank factorisation `A = C R`: `R^T (R R^T)^-1 (C^T C)^-1 C^T`.
fn pseudoinverse(a: &Matrix) -> Matrix {
    let (reduced, pivots) = rref(a);
    let rank = pivots.len();
    if rank == 0 {
        return zeros(a[0].len(), a.len());
    }
    let c: Matrix = a
        .iter()
        .map(|row| pivots.iter().map(|&p| row[p].clone()).collect())
        .collect();
    let r: Matrix = reduced[..rank].to_vec();
    let rt = transpose(&r);
    let ct = transpose(&c);
    let rrt_inv = inverse(&multiply(&r, &rt).unwrap_or_default()).unwrap_or_default();
    let ctc_inv = inverse(&multiply(&ct, &c).unwrap_or_default()).unwrap_or_default();
    let left = multiply(&rt, &rrt_inv).unwrap_or_default();
    let middle = multiply(&left, &ctc_inv).unwrap_or_default();
    multiply(&middle, &ct).unwrap_or_default()
}

fn dot(u: &[Q], v: &[Q]) -> Q {
    u.iter().zip(v).fold(Q::zero(), |s, (a, b)| s + a * b)
}

/// The singular values by the Jacobi method on `A^T A` (floating point), largest first.
fn singular_values(a: &Matrix) -> Vec<f64> {
    let f: Vec<Vec<f64>> = a
        .iter()
        .map(|r| r.iter().map(|q| q.to_f64().unwrap_or(f64::NAN)).collect())
        .collect();
    let n = f[0].len();
    let mut s: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| f.iter().map(|row| row[i] * row[j]).sum())
                .collect()
        })
        .collect();
    for _ in 0..100 {
        let mut off = 0.0;
        for p in 0..n {
            for q in p + 1..n {
                off += s[p][q] * s[p][q];
                if s[p][q].abs() < 1e-300 {
                    continue;
                }
                let theta = (s[q][q] - s[p][p]) / (2.0 * s[p][q]);
                let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
                let t = if theta == 0.0 { 1.0 } else { t };
                let (c, sn) = (1.0 / (t * t + 1.0).sqrt(), t / (t * t + 1.0).sqrt());
                for k in 0..n {
                    let (a_kp, a_kq) = (s[k][p], s[k][q]);
                    s[k][p] = c * a_kp - sn * a_kq;
                    s[k][q] = sn * a_kp + c * a_kq;
                }
                for k in 0..n {
                    let (a_pk, a_qk) = (s[p][k], s[q][k]);
                    s[p][k] = c * a_pk - sn * a_qk;
                    s[q][k] = sn * a_pk + c * a_qk;
                }
            }
        }
        if off < 1e-28 {
            break;
        }
    }
    let mut values: Vec<f64> = (0..n).map(|i| s[i][i].max(0.0).sqrt()).collect();
    values.sort_by(|a, b| b.total_cmp(a));
    values
}

/// `Some(result)` when `input` is one of these commands on a matrix literal.
pub fn run(session: &mut UraeSession, input: &str) -> Option<Result<String, String>> {
    let t = input.trim();
    let open = t.find('(')?;
    let name = t[..open].trim().to_ascii_lowercase();
    if !t.ends_with(')') {
        return None;
    }
    let known = [
        "rowspace",
        "row_space",
        "columnspace",
        "column_space",
        "colspace",
        "adjugate",
        "adjoint",
        "cofactor",
        "cofactors",
        "minor",
        "hadamard",
        "elementwise",
        "is_symmetric",
        "is_orthogonal",
        "is_positive_definite",
        "pinv",
        "pseudoinverse",
        "lstsq",
        "least_squares",
        "gram_schmidt",
        "orthonormalize",
        "projection",
        "proj",
        "frobenius_norm",
        "cond",
        "condition_number",
        "matrix_exp",
        "expm",
        "eig",
        "jordan",
        "jordan_form",
        "hessenberg",
        "mregress",
        "multiple_regression",
        "ols",
        "logistic_regress",
        "logistic_regression",
        "logit",
        "poisson_regress",
        "poisson_regression",
    ];
    if !known.contains(&name.as_str()) {
        return None;
    }
    let args = split_top(&t[open + 1..t.len() - 1]);
    let first = parse(session, args.first()?)?;
    Some(evaluate(session, &name, &first, &args))
}

fn evaluate(
    session: &mut UraeSession,
    name: &str,
    a: &Matrix,
    args: &[String],
) -> Result<String, String> {
    let (rows, cols) = (a.len(), a[0].len());
    let square = rows == cols;
    match name {
        "mregress" | "multiple_regression" | "ols" => multiple_regression(session, a, args),
        "logistic_regress" | "logistic_regression" | "logit" => {
            logistic_regression(session, a, args)
        }
        "poisson_regress" | "poisson_regression" => poisson_regression(session, a, args),
        "jordan" | "jordan_form" => jordan(session, a),
        "hessenberg" => hessenberg(session, a),
        "rowspace" | "row_space" => {
            let (reduced, pivots) = rref(a);
            display(session, &reduced[..pivots.len()].to_vec())
        }
        "columnspace" | "column_space" | "colspace" => {
            let (_, pivots) = rref(a);
            let basis: Matrix = (0..pivots.len())
                .map(|k| a.iter().map(|row| row[pivots[k]].clone()).collect())
                .collect();
            // Each basis vector is printed as a row of the result, as a list of column vectors.
            let listing: Vec<String> = basis
                .iter()
                .map(|v| format!("[{}]", v.iter().map(show_q).collect::<Vec<_>>().join(", ")))
                .collect();
            Ok(format!("basis of the column space: {}", listing.join(", ")))
        }
        "adjugate" | "adjoint" => {
            if !square {
                return Err("the adjugate needs a square matrix".into());
            }
            display(session, &transpose(&cofactors(a)))
        }
        "cofactor" | "cofactors" => {
            if !square {
                return Err("the cofactor matrix needs a square matrix".into());
            }
            display(session, &cofactors(a))
        }
        "minor" => {
            let (i, j) = (
                crate::list_utils::rational(session, args.get(1).ok_or("minor(A, i, j)")?)
                    .and_then(|q| q.to_integer().to_usize()),
                crate::list_utils::rational(session, args.get(2).ok_or("minor(A, i, j)")?)
                    .and_then(|q| q.to_integer().to_usize()),
            );
            let (Some(i), Some(j)) = (i, j) else {
                return Err("the indices must be positive integers (from 1)".into());
            };
            if !square || i == 0 || j == 0 || i > rows || j > cols || rows < 2 {
                return Err(
                    "minor(A, i, j) needs a square matrix of size 2 or more and indices within it"
                        .into(),
                );
            }
            Ok(show_q(&determinant(&minor_matrix(a, i - 1, j - 1))))
        }
        "hadamard" | "elementwise" => {
            let b = parse(session, args.get(1).ok_or("hadamard(A, B)")?)
                .ok_or("the second argument must be a matrix")?;
            if b.len() != rows || b[0].len() != cols {
                return Err("the matrices must have the same size".into());
            }
            let product: Matrix = a
                .iter()
                .zip(&b)
                .map(|(x, y)| x.iter().zip(y).map(|(p, q)| p * q).collect())
                .collect();
            display(session, &product)
        }
        "is_symmetric" => Ok((square && a == &transpose(a)).to_string()),
        "is_orthogonal" => Ok((square
            && multiply(a, &transpose(a)).is_some_and(|p| p == identity(rows)))
        .to_string()),
        "is_positive_definite" => {
            if !square || a != &transpose(a) {
                return Ok("false".into());
            }
            // Sylvester's criterion: every leading principal minor is positive.
            let positive = (1..=rows).all(|k| {
                let leading: Matrix = a[..k].iter().map(|r| r[..k].to_vec()).collect();
                determinant(&leading).is_positive()
            });
            Ok(positive.to_string())
        }
        "pinv" | "pseudoinverse" => display(session, &pseudoinverse(a)),
        "lstsq" | "least_squares" => {
            let b =
                parse(session, args.get(1).ok_or("lstsq(A, b)")?).ok_or("b must be a vector")?;
            if b.len() != rows || b[0].len() != 1 {
                return Err("b must be a vector with one entry per row of A".into());
            }
            let x = multiply(&pseudoinverse(a), &b).ok_or("size mismatch")?;
            display(session, &x)
        }
        "gram_schmidt" | "orthonormalize" => {
            // The columns of A are the vectors; dependent ones are dropped.
            let columns: Vec<Vec<Q>> = transpose(a);
            let mut basis: Vec<Vec<Q>> = Vec::new();
            for v in columns {
                let mut w = v.clone();
                for b in &basis {
                    let f = dot(&v, b) / dot(b, b);
                    for (x, y) in w.iter_mut().zip(b) {
                        *x -= &f * y;
                    }
                }
                if w.iter().any(|x| !x.is_zero()) {
                    basis.push(w);
                }
            }
            if name == "gram_schmidt" {
                let listing: Vec<String> = basis
                    .iter()
                    .map(|v| format!("[{}]", v.iter().map(show_q).collect::<Vec<_>>().join(", ")))
                    .collect();
                return Ok(format!("orthogonal basis: {}", listing.join(", ")));
            }
            let mut listing = Vec::new();
            for v in &basis {
                let norm_squared = dot(v, v);
                let mut cells = Vec::new();
                for x in v {
                    cells.push(session.eval(&format!(
                        "({})/sqrt({})",
                        show_q(x),
                        show_q(&norm_squared)
                    ))?);
                }
                listing.push(format!("[{}]", cells.join(", ")));
            }
            Ok(format!("orthonormal basis: {}", listing.join(", ")))
        }
        "projection" | "proj" => {
            let v = parse(session, args.get(1).ok_or("projection(u, v)")?)
                .ok_or("v must be a vector")?;
            let (u, v): (Vec<Q>, Vec<Q>) = (
                a.iter().map(|r| r[0].clone()).collect(),
                v.iter().map(|r| r[0].clone()).collect(),
            );
            if u.len() != v.len() || cols != 1 && rows != 1 {
                return Err("projection(u, v) takes two vectors of the same length".into());
            }
            let (u, v) = if cols != 1 { (a[0].clone(), v) } else { (u, v) };
            if dot(&v, &v).is_zero() {
                return Err("cannot project onto the zero vector".into());
            }
            let f = dot(&u, &v) / dot(&v, &v);
            let result: Vec<Q> = v.iter().map(|x| &f * x).collect();
            Ok(format!(
                "[{}]",
                result.iter().map(show_q).collect::<Vec<_>>().join(", ")
            ))
        }
        "frobenius_norm" => {
            let sum = a.iter().flatten().fold(Q::zero(), |s, x| s + x * x);
            session.eval(&format!("sqrt({})", show_q(&sum)))
        }
        "cond" | "condition_number" => {
            let values = singular_values(a);
            let (largest, smallest) = (values[0], *values.last().unwrap_or(&0.0));
            if smallest <= 1e-14 * largest {
                return Ok("infinity (the matrix is singular)".into());
            }
            Ok(format!("{}", largest / smallest))
        }
        "matrix_exp" | "expm" => {
            if !square {
                return Err("the matrix exponential needs a square matrix".into());
            }
            // e^A = Phi(1) with Phi(t) = L^-1{(s I - A)^-1}, entry by entry.
            let n = rows;
            let resolvent_input: Vec<String> = (0..n)
                .map(|i| {
                    let cells: Vec<String> = (0..n)
                        .map(|j| {
                            if i == j {
                                format!("s - ({})", show_q(&a[i][j]))
                            } else {
                                format!("-({})", show_q(&a[i][j]))
                            }
                        })
                        .collect();
                    format!("[{}]", cells.join(", "))
                })
                .collect();
            let resolvent = session.eval(&format!("inverse([{}])", resolvent_input.join(", ")))?;
            let inner = resolvent
                .trim()
                .strip_prefix('[')
                .and_then(|r| r.strip_suffix(']'))
                .ok_or("could not form the resolvent")?;
            let entries: Vec<Vec<String>> = inner.split(';').map(split_top).collect();
            let mut cells = Vec::new();
            for row in &entries {
                let mut out = Vec::new();
                for entry in row {
                    let at_zero = if entry.trim() == "0" {
                        "0".to_string()
                    } else {
                        let value = crate::inverse_laplace::invert(session, entry, "s", "t")?;
                        session.eval(&format!("subs(({value}), t = 1)"))?
                    };
                    out.push(at_zero);
                }
                cells.push(format!("[{}]", out.join(", ")));
            }
            session.eval(&format!("[{}]", cells.join(", ")))
        }
        _ => {
            // eig: eigenvalues with eigenvectors.
            session.eval(&format!("eigenvectors({})", literal(a)))
        }
    }
}

/// `mregress(X, y)`: ordinary least squares with an intercept; the rows of `X` are observations, its columns the predictors.
fn multiple_regression(
    session: &mut UraeSession,
    x: &Matrix,
    args: &[String],
) -> Result<String, String> {
    let y = parse(session, args.get(1).ok_or("mregress(X, y)")?)
        .ok_or("y must be a list of numbers")?;
    let (n, k) = (x.len(), x[0].len());
    if y.len() != n || y[0].len() != 1 {
        return Err("y needs one value per row of X".into());
    }
    if n < k + 2 {
        return Err(format!("need more than {} observations", k + 1));
    }
    let design: Matrix = x
        .iter()
        .map(|row| {
            std::iter::once(Q::from_integer(1.into()))
                .chain(row.iter().cloned())
                .collect()
        })
        .collect();
    let dt = transpose(&design);
    let gram = multiply(&dt, &design).ok_or("shape mismatch")?;
    let inv = inverse(&gram).ok_or("the predictors are linearly dependent")?;
    let beta =
        multiply(&inv, &multiply(&dt, &y).ok_or("shape mismatch")?).ok_or("shape mismatch")?;
    let fitted = multiply(&design, &beta).ok_or("shape mismatch")?;
    let f = |q: &Q| q.to_f64().unwrap_or(f64::NAN);
    let mean_y = y.iter().map(|r| f(&r[0])).sum::<f64>() / n as f64;
    let sse: Q = (0..n).fold(Q::from_integer(0.into()), |acc, i| {
        let e = &y[i][0] - &fitted[i][0];
        acc + &e * &e
    });
    let sst: f64 = y.iter().map(|r| (f(&r[0]) - mean_y).powi(2)).sum();
    let df = (n - k - 1) as f64;
    let sigma2 = f(&sse) / df;
    let num = |v: f64| {
        let t = format!("{v:.6}");
        t.trim_end_matches('0').trim_end_matches('.').to_string()
    };
    let names: Vec<String> = std::iter::once("intercept".to_string())
        .chain((1..=k).map(|i| format!("x{i}")))
        .collect();
    let parts: Vec<String> = (0..=k)
        .map(|i| {
            let se = (sigma2 * f(&inv[i][i])).sqrt();
            if se == 0.0 {
                return format!("{} = {} (exact fit)", names[i], show_q(&beta[i][0]));
            }
            let t = f(&beta[i][0]) / se;
            let upper = crate::distributions_f64::eval("t_cdf", &[t.abs(), df]).unwrap_or(f64::NAN);
            format!(
                "{} = {} (se {}, t = {}, p = {})",
                names[i],
                show_q(&beta[i][0]),
                num(se),
                num(t),
                num((2.0 * (1.0 - upper)).clamp(0.0, 1.0))
            )
        })
        .collect();
    let r2 = if sst == 0.0 { 1.0 } else { 1.0 - f(&sse) / sst };
    let adjusted = 1.0 - (1.0 - r2) * (n as f64 - 1.0) / df;
    let fstat = if f(&sse) == 0.0 {
        f64::INFINITY
    } else {
        (sst - f(&sse)) / k as f64 / sigma2
    };
    let fp = if fstat.is_finite() {
        1.0 - crate::distributions_f64::eval("f_cdf", &[fstat, k as f64, df]).unwrap_or(f64::NAN)
    } else {
        0.0
    };
    Ok(format!(
        "{}; R^2 = {}, adjusted R^2 = {}, F = {} on ({}, {}) df, p = {}",
        parts.join("; "),
        num(r2),
        num(adjusted),
        num(fstat),
        k,
        df,
        num(fp.clamp(0.0, 1.0))
    ))
}

/// Invert a small dense matrix in floating point (Gauss-Jordan with partial pivoting).
fn invert_f64(m: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = m.len();
    let mut a: Vec<Vec<f64>> = m
        .iter()
        .enumerate()
        .map(|(i, row)| {
            row.iter()
                .copied()
                .chain((0..n).map(|j| if i == j { 1.0 } else { 0.0 }))
                .collect()
        })
        .collect();
    for col in 0..n {
        let pivot = (col..n).max_by(|&x, &y| a[x][col].abs().total_cmp(&a[y][col].abs()))?;
        if a[pivot][col].abs() < 1e-12 {
            return None;
        }
        a.swap(col, pivot);
        let p = a[col][col];
        for v in a[col].iter_mut() {
            *v /= p;
        }
        for r in 0..n {
            if r != col {
                let f = a[r][col];
                let pivot_row = a[col].clone();
                for (v, w) in a[r].iter_mut().zip(&pivot_row) {
                    *v -= f * w;
                }
            }
        }
    }
    Some(a.into_iter().map(|row| row[n..].to_vec()).collect())
}

/// `logistic_regress(X, y)`: maximum likelihood by iteratively reweighted least squares (Newton), with Wald standard errors.
fn logistic_regression(
    session: &mut UraeSession,
    x: &Matrix,
    args: &[String],
) -> Result<String, String> {
    let y = parse(session, args.get(1).ok_or("logistic_regress(X, y)")?)
        .ok_or("y must be a list of 0s and 1s")?;
    let (n, k) = (x.len(), x[0].len());
    let f = |q: &Q| q.to_f64().unwrap_or(f64::NAN);
    let outcome: Vec<f64> = y.iter().map(|r| f(&r[0])).collect();
    if outcome.len() != n || y[0].len() != 1 || outcome.iter().any(|v| *v != 0.0 && *v != 1.0) {
        return Err("y needs one 0 or 1 per row of X".into());
    }
    if n < k + 2 {
        return Err("too few observations".into());
    }
    let design: Vec<Vec<f64>> = x
        .iter()
        .map(|row| std::iter::once(1.0).chain(row.iter().map(f)).collect())
        .collect();
    let p = k + 1;
    let mut beta = vec![0.0; p];
    let mut information = vec![vec![0.0; p]; p];
    let mut converged = false;
    for _ in 0..100 {
        let probability: Vec<f64> = design
            .iter()
            .map(|row| {
                1.0 / (1.0 + (-row.iter().zip(&beta).map(|(a, b)| a * b).sum::<f64>()).exp())
            })
            .collect();
        let mut gradient = vec![0.0; p];
        information = vec![vec![0.0; p]; p];
        for (i, row) in design.iter().enumerate() {
            let w = probability[i] * (1.0 - probability[i]);
            for a in 0..p {
                gradient[a] += row[a] * (outcome[i] - probability[i]);
                for b in 0..p {
                    information[a][b] += w * row[a] * row[b];
                }
            }
        }
        let inverse = invert_f64(&information).ok_or("the likelihood is degenerate: the classes are perfectly separable or the predictors are dependent")?;
        let step: Vec<f64> = (0..p)
            .map(|a| (0..p).map(|b| inverse[a][b] * gradient[b]).sum())
            .collect();
        for a in 0..p {
            beta[a] += step[a];
        }
        if beta.iter().any(|b| b.abs() > 40.0) {
            return Err("the classes are perfectly separable: the maximum likelihood estimate does not exist".into());
        }
        if step.iter().all(|s| s.abs() < 1e-11) {
            converged = true;
            break;
        }
    }
    if !converged {
        return Err("the iteration did not converge".into());
    }
    let covariance = invert_f64(&information).ok_or("singular information matrix")?;
    let num = |v: f64| {
        let t = format!("{v:.6}");
        t.trim_end_matches('0').trim_end_matches('.').to_string()
    };
    let names: Vec<String> = std::iter::once("intercept".to_string())
        .chain((1..=k).map(|i| format!("x{i}")))
        .collect();
    let parts: Vec<String> = (0..p)
        .map(|i| {
            let se = covariance[i][i].sqrt();
            let z = beta[i] / se;
            let tail = crate::inference::normal_cdf(z.abs());
            format!(
                "{} = {} (se {}, z = {}, p = {}, odds ratio {})",
                names[i],
                num(beta[i]),
                num(se),
                num(z),
                num((2.0 * (1.0 - tail)).clamp(0.0, 1.0)),
                num(beta[i].exp())
            )
        })
        .collect();
    let log_likelihood: f64 = design
        .iter()
        .zip(&outcome)
        .map(|(row, yi)| {
            let eta: f64 = row.iter().zip(&beta).map(|(a, b)| a * b).sum();
            yi * eta - (1.0 + eta.exp()).ln()
        })
        .sum();
    Ok(format!(
        "{}; log-likelihood = {}",
        parts.join("; "),
        num(log_likelihood)
    ))
}

/// `poisson_regress(X, y)`: counts with a log link, by Newton / IRLS, with Wald inference and rate ratios.
fn poisson_regression(
    session: &mut UraeSession,
    x: &Matrix,
    args: &[String],
) -> Result<String, String> {
    let y = parse(session, args.get(1).ok_or("poisson_regress(X, y)")?)
        .ok_or("y must be a list of counts")?;
    let (n, k) = (x.len(), x[0].len());
    let f = |q: &Q| q.to_f64().unwrap_or(f64::NAN);
    let counts: Vec<f64> = y.iter().map(|r| f(&r[0])).collect();
    if counts.len() != n || y[0].len() != 1 || counts.iter().any(|v| *v < 0.0 || v.fract() != 0.0) {
        return Err("y needs one non-negative integer count per row of X".into());
    }
    if n < k + 2 {
        return Err("too few observations".into());
    }
    let design: Vec<Vec<f64>> = x
        .iter()
        .map(|row| std::iter::once(1.0).chain(row.iter().map(f)).collect())
        .collect();
    let p = k + 1;
    let mut beta = vec![0.0; p];
    beta[0] = (counts.iter().sum::<f64>() / n as f64).max(1e-3).ln();
    let mut information = vec![vec![0.0; p]; p];
    let mut converged = false;
    for _ in 0..200 {
        let mean: Vec<f64> = design
            .iter()
            .map(|row| row.iter().zip(&beta).map(|(a, b)| a * b).sum::<f64>().exp())
            .collect();
        let mut gradient = vec![0.0; p];
        information = vec![vec![0.0; p]; p];
        for (i, row) in design.iter().enumerate() {
            for a in 0..p {
                gradient[a] += row[a] * (counts[i] - mean[i]);
                for b in 0..p {
                    information[a][b] += mean[i] * row[a] * row[b];
                }
            }
        }
        let inverse = invert_f64(&information).ok_or("the predictors are linearly dependent")?;
        let step: Vec<f64> = (0..p)
            .map(|a| (0..p).map(|b| inverse[a][b] * gradient[b]).sum())
            .collect();
        for a in 0..p {
            beta[a] += step[a];
        }
        if beta.iter().any(|b| !b.is_finite() || b.abs() > 50.0) {
            return Err("the maximum likelihood estimate does not exist for this data".into());
        }
        if step.iter().all(|s| s.abs() < 1e-11) {
            converged = true;
            break;
        }
    }
    if !converged {
        return Err("the iteration did not converge".into());
    }
    let covariance = invert_f64(&information).ok_or("singular information matrix")?;
    let num = |v: f64| {
        let t = format!("{v:.6}");
        t.trim_end_matches('0').trim_end_matches('.').to_string()
    };
    let names: Vec<String> = std::iter::once("intercept".to_string())
        .chain((1..=k).map(|i| format!("x{i}")))
        .collect();
    let parts: Vec<String> = (0..p)
        .map(|i| {
            let se = covariance[i][i].sqrt();
            let z = beta[i] / se;
            let tail = crate::inference::normal_cdf(z.abs());
            format!(
                "{} = {} (se {}, z = {}, p = {}, rate ratio {})",
                names[i],
                num(beta[i]),
                num(se),
                num(z),
                num((2.0 * (1.0 - tail)).clamp(0.0, 1.0)),
                num(beta[i].exp())
            )
        })
        .collect();
    let log_gamma = |v: f64| crate::distributions_f64::ln_gamma(v + 1.0);
    let log_likelihood: f64 = design
        .iter()
        .zip(&counts)
        .map(|(row, yi)| {
            let eta: f64 = row.iter().zip(&beta).map(|(a, b)| a * b).sum();
            yi * eta - eta.exp() - log_gamma(*yi)
        })
        .sum();
    Ok(format!(
        "{}; log-likelihood = {}",
        parts.join("; "),
        num(log_likelihood)
    ))
}

/// `jordan(A)`: the Jordan normal form of a square rational matrix whose eigenvalues are all rational. The number of blocks of
/// size at least `k` for an eigenvalue `l` is `rank (A - l I)^(k-1) - rank (A - l I)^k`.
fn jordan(session: &mut UraeSession, a: &Matrix) -> Result<String, String> {
    let n = a.len();
    if a[0].len() != n {
        return Err("jordan needs a square matrix".into());
    }
    // Characteristic polynomial by Faddeev-LeVerrier, then its rational roots from the rational factorisation.
    let mut coefficients = vec![Q::zero(); n + 1]; // ascending powers of x
    coefficients[n] = Q::one();
    let mut m = zeros(n, n);
    for k in 1..=n {
        let previous = coefficients[n - k + 1].clone();
        let mut next = multiply(a, &m).ok_or("shape mismatch")?;
        for (r, row) in next.iter_mut().enumerate() {
            row[r] += &previous;
        }
        let product = multiply(a, &next).ok_or("shape mismatch")?;
        let trace = (0..n).fold(Q::zero(), |acc, r| acc + &product[r][r]);
        coefficients[n - k] = -trace / Q::from_integer(BigInt::from(k));
        m = next;
    }
    let polynomial = crate::poly::UniPoly::from_coeffs(coefficients);
    let (_, factors) =
        crate::poly::factor::factor_rational_polynomial(&polynomial).map_err(|e| e.to_string())?;
    let mut values: Vec<Q> = Vec::new();
    for (factor, _) in &factors {
        if factor.degree() != 1 {
            return Err("jordan needs rational eigenvalues: the characteristic polynomial has an irreducible factor of higher degree".into());
        }
        let root = -factor.coeffs[0].clone() / factor.coeffs[1].clone();
        if !values.contains(&root) {
            values.push(root);
        }
    }
    values.sort();
    let rank = |m: &Matrix| rref(m).1.len();
    let mut blocks: Vec<(Q, usize)> = Vec::new();
    for lambda in &values {
        let shifted: Matrix = (0..n)
            .map(|i| {
                (0..n)
                    .map(|j| {
                        if i == j {
                            &a[i][j] - lambda
                        } else {
                            a[i][j].clone()
                        }
                    })
                    .collect()
            })
            .collect();
        let mut power = identity(n);
        let mut ranks = vec![n];
        for _ in 0..n {
            power = multiply(&power, &shifted).ok_or("shape mismatch")?;
            ranks.push(rank(&power));
            if *ranks.last().unwrap_or(&0) == ranks[ranks.len() - 2] {
                break;
            }
        }
        // at_least[k] = blocks of size >= k+1.
        let at_least: Vec<usize> = (0..ranks.len() - 1)
            .map(|k| ranks[k] - ranks[k + 1])
            .collect();
        for size in (1..=at_least.len()).rev() {
            let count = at_least[size - 1] - at_least.get(size).copied().unwrap_or(0);
            for _ in 0..count {
                blocks.push((lambda.clone(), size));
            }
        }
    }
    let total: usize = blocks.iter().map(|b| b.1).sum();
    if total != n {
        return Err(
            "the eigenvalues are not all rational: the Jordan form needs the full spectrum".into(),
        );
    }
    let mut j = zeros(n, n);
    let mut at = 0;
    for (lambda, size) in &blocks {
        for k in 0..*size {
            j[at + k][at + k] = lambda.clone();
            if k + 1 < *size {
                j[at + k][at + k + 1] = Q::from_integer(1.into());
            }
        }
        at += size;
    }
    let sizes: Vec<String> = blocks
        .iter()
        .map(|(l, s)| format!("J{s}({})", show_q(l)))
        .collect();
    Ok(format!(
        "{} with blocks {}",
        display(session, &j)?,
        sizes.join(", ")
    ))
}

/// `hessenberg(A)`: an upper Hessenberg matrix `H` similar to `A` with `A = P H P^-1`, by exact elimination with row and column
/// swaps (a similarity transform at every step).
fn hessenberg(session: &mut UraeSession, a: &Matrix) -> Result<String, String> {
    let n = a.len();
    if a[0].len() != n {
        return Err("hessenberg needs a square matrix".into());
    }
    let mut h = a.clone();
    let mut p = identity(n);
    for k in 0..n.saturating_sub(2) {
        let Some(pivot) = (k + 1..n).find(|&r| !h[r][k].is_zero()) else {
            continue;
        };
        if pivot != k + 1 {
            h.swap(pivot, k + 1);
            for row in h.iter_mut() {
                row.swap(pivot, k + 1);
            }
            for row in p.iter_mut() {
                row.swap(pivot, k + 1);
            }
        }
        for i in k + 2..n {
            if h[i][k].is_zero() {
                continue;
            }
            let factor = &h[i][k] / &h[k + 1][k];
            for c in 0..n {
                let delta = &factor * &h[k + 1][c];
                h[i][c] -= delta;
            }
            for row in h.iter_mut() {
                let delta = &factor * &row[i];
                row[k + 1] += delta;
            }
            for row in p.iter_mut() {
                let delta = &factor * &row[i];
                row[k + 1] += delta;
            }
        }
    }
    Ok(format!(
        "H = {}; P = {}",
        display(session, &h)?,
        display(session, &p)?
    ))
}
