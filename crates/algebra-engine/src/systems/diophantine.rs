//! # `algebra_engine::systems::diophantine`
//!
//! Multi-Variable Integer Diophantine Systems Solvers.
//!
//! Provides:
//! - **Smith Normal Form (SNF)**: Exact unimodular factorization $\mathbf{U}\mathbf{A}\mathbf{V} = \mathbf{D}$
//!   for integer matrix $\mathbf{A} \in \mathbb{Z}^{m \times n}$ with invariant factors $d_i \mid d_{i+1}$.
//! - **Linear Diophantine System Solver**: Determines integer lattice solutions $\mathbf{A}\mathbf{x} = \mathbf{b}$ over $\mathbb{Z}$.

use algebra_core::error::{AlgebraError, AlgebraResult};

/// Result of Smith Normal Form decomposition $\mathbf{U}\mathbf{A}\mathbf{V} = \mathbf{D}$.
#[derive(Debug, Clone)]
pub struct SmithNormalForm {
    /// Invariant diagonal factors $(d_1, \dots, d_r)$ where $d_i \mid d_{i+1}$.
    pub diagonal: Vec<i64>,
    /// Unimodular left transformation matrix $\mathbf{U} \in \operatorname{GL}_m(\mathbb{Z})$ ($m \times m$).
    pub u: Vec<Vec<i64>>,
    /// Unimodular right transformation matrix $\mathbf{V} \in \operatorname{GL}_n(\mathbb{Z})$ ($n \times n$).
    pub v: Vec<Vec<i64>>,
    /// Rank of the matrix.
    pub rank: usize,
}

/// Integer solution space for Diophantine system $\mathbf{A}\mathbf{x} = \mathbf{b}$ over $\mathbb{Z}$.
#[derive(Debug, Clone)]
pub struct DiophantineSolution {
    /// Particular integer solution $\mathbf{x}_p \in \mathbb{Z}^n$.
    pub particular: Vec<i64>,
    /// Basis vectors spanning the homogeneous integer nullspace lattice $\operatorname{ker}_\mathbb{Z}(\mathbf{A})$.
    pub nullspace_basis: Vec<Vec<i64>>,
}

/// Integer Diophantine systems solver.
pub struct DiophantineSystemSolver;

#[allow(clippy::needless_range_loop)]
impl DiophantineSystemSolver {
    /// Computes the Smith Normal Form (SNF) $\mathbf{U}\mathbf{A}\mathbf{V} = \mathbf{D}$ for integer matrix $\mathbf{A} \in \mathbb{Z}^{m \times n}$.
    ///
    /// The diagonal satisfies the divisibility chain $d_1 \mid d_2 \mid \dots \mid d_r$: when the pivot
    /// $d_k$ does not divide an entry of the remaining block, that entry's row is added to the pivot row
    /// and the block is reduced again (so the form of $\operatorname{diag}(2, 3)$ is $\operatorname{diag}(1, 6)$).
    /// The work is done in `i128` with checked arithmetic; an intermediate value or a final entry that does
    /// not fit is an error, not a wrapped number.
    pub fn smith_normal_form(matrix: &[Vec<i64>]) -> AlgebraResult<SmithNormalForm> {
        let m = matrix.len();
        if m == 0 {
            return Err(AlgebraError::EvaluationError(
                "Matrix must have at least one row".into(),
            ));
        }
        let n = matrix[0].len();
        if n == 0 {
            return Err(AlgebraError::EvaluationError(
                "Matrix must have at least one column".into(),
            ));
        }
        if matrix.iter().any(|row| row.len() != n) {
            return Err(AlgebraError::EvaluationError(
                "All rows must have the same length".into(),
            ));
        }
        let overflow = || {
            AlgebraError::EvaluationError(
                "Smith normal form: integer coefficients grew beyond 128 bits".into(),
            )
        };
        // dst -= q * src, with checked arithmetic.
        let sub_scaled = |dst: &mut [i128], src: &[i128], q: i128| -> AlgebraResult<()> {
            for (d, s) in dst.iter_mut().zip(src) {
                *d = q
                    .checked_mul(*s)
                    .and_then(|x| d.checked_sub(x))
                    .ok_or_else(overflow)?;
            }
            Ok(())
        };

        let mut a: Vec<Vec<i128>> = matrix
            .iter()
            .map(|row| row.iter().map(|&x| x as i128).collect())
            .collect();
        // U starts as the m x m identity, V as the n x n identity; V is kept transposed (its rows are
        // the columns of V) so that a column operation on A is a row operation on it.
        let mut u: Vec<Vec<i128>> = (0..m)
            .map(|i| (0..m).map(|j| i128::from(i == j)).collect())
            .collect();
        let mut vt: Vec<Vec<i128>> = (0..n)
            .map(|i| (0..n).map(|j| i128::from(i == j)).collect())
            .collect();

        let min_dim = m.min(n);
        let mut rank = 0;
        'pivots: for k in 0..min_dim {
            loop {
                // Smallest non-zero entry of the remaining block.
                let mut pivot: Option<(usize, usize)> = None;
                let mut best = i128::MAX;
                for i in k..m {
                    for j in k..n {
                        let v = a[i][j].unsigned_abs();
                        if v != 0 && v < best as u128 {
                            best = v as i128;
                            pivot = Some((i, j));
                        }
                    }
                }
                let Some((pi, pj)) = pivot else {
                    break 'pivots; // the rest is zero
                };
                a.swap(k, pi);
                u.swap(k, pi);
                if pj != k {
                    for row in a.iter_mut() {
                        row.swap(k, pj);
                    }
                    vt.swap(k, pj);
                }
                if a[k][k] < 0 {
                    for x in a[k].iter_mut() {
                        *x = -*x;
                    }
                    for x in u[k].iter_mut() {
                        *x = -*x;
                    }
                }
                let d = a[k][k];

                // Clear column k below the pivot (row operations), then row k to its right (column operations).
                let mut residue = false;
                let pivot_row = a[k].clone();
                let pivot_u = u[k].clone();
                for i in (k + 1)..m {
                    let q = a[i][k].div_euclid(d);
                    if q != 0 {
                        sub_scaled(&mut a[i], &pivot_row, q)?;
                        sub_scaled(&mut u[i], &pivot_u, q)?;
                    }
                    residue |= a[i][k] != 0;
                }
                for j in (k + 1)..n {
                    let q = a[k][j].div_euclid(d);
                    if q != 0 {
                        for row in a.iter_mut() {
                            row[j] = q
                                .checked_mul(row[k])
                                .and_then(|x| row[j].checked_sub(x))
                                .ok_or_else(overflow)?;
                        }
                        let src = vt[k].clone();
                        sub_scaled(&mut vt[j], &src, q)?;
                    }
                    residue |= a[k][j] != 0;
                }
                if residue {
                    continue; // a smaller non-zero entry now exists: pick it as the pivot
                }
                // The row and column of the pivot are clear. The divisibility chain needs the pivot to
                // divide every remaining entry; if it does not, fold that entry's row into the pivot row.
                let offender = (k + 1..m).find(|&i| (k + 1..n).any(|j| a[i][j] % d != 0));
                if let Some(i) = offender {
                    let (row, urow) = (a[i].clone(), u[i].clone());
                    for (x, y) in a[k].iter_mut().zip(&row) {
                        *x = x.checked_add(*y).ok_or_else(overflow)?;
                    }
                    for (x, y) in u[k].iter_mut().zip(&urow) {
                        *x = x.checked_add(*y).ok_or_else(overflow)?;
                    }
                    continue;
                }
                rank += 1;
                break;
            }
        }

        let to_i64 = |x: i128| i64::try_from(x).map_err(|_| overflow());
        let diagonal = (0..rank)
            .map(|i| to_i64(a[i][i]))
            .collect::<AlgebraResult<Vec<i64>>>()?;
        let u = u
            .iter()
            .map(|row| row.iter().map(|&x| to_i64(x)).collect())
            .collect::<AlgebraResult<Vec<Vec<i64>>>>()?;
        // V has the stored rows as its columns.
        let v = (0..n)
            .map(|i| (0..n).map(|j| to_i64(vt[j][i])).collect())
            .collect::<AlgebraResult<Vec<Vec<i64>>>>()?;

        Ok(SmithNormalForm {
            diagonal,
            u,
            v,
            rank,
        })
    }

    /// Solves linear integer Diophantine system $\mathbf{A}\mathbf{x} = \mathbf{b}$ over $\mathbb{Z}$.
    pub fn solve(matrix: &[Vec<i64>], b_vector: &[i64]) -> AlgebraResult<DiophantineSolution> {
        let m = matrix.len();
        if m == 0 || b_vector.len() != m {
            return Err(AlgebraError::DimensionMismatch(
                "Matrix rows and RHS vector length mismatch".into(),
            ));
        }
        let n = matrix[0].len();

        let snf = Self::smith_normal_form(matrix)?;

        // Compute c = U * b
        let mut c = vec![0i64; m];
        for i in 0..m {
            let mut sum = 0;
            for j in 0..m {
                sum += snf.u[i][j] * b_vector[j];
            }
            c[i] = sum;
        }

        // Verify divisibility and consistency
        let mut y_part = vec![0i64; n];
        for i in 0..snf.rank {
            let d_i = snf.diagonal[i];
            if d_i == 0 || c[i] % d_i != 0 {
                return Err(AlgebraError::EvaluationError(
                    "No integer solution: RHS element not divisible by invariant factor".into(),
                ));
            }
            y_part[i] = c[i] / d_i;
        }

        for i in snf.rank..m {
            if c[i] != 0 {
                return Err(AlgebraError::EvaluationError(
                    "Inconsistent integer system: zero row has non-zero RHS".into(),
                ));
            }
        }

        // Particular solution x_p = V * y_part
        let mut particular = vec![0i64; n];
        for i in 0..n {
            let mut sum = 0;
            for j in 0..n {
                sum += snf.v[i][j] * y_part[j];
            }
            particular[i] = sum;
        }

        // Nullspace basis: columns of V corresponding to free variables (rank..n)
        let mut nullspace_basis = Vec::new();
        for col in snf.rank..n {
            let mut basis_vec = vec![0i64; n];
            for i in 0..n {
                basis_vec[i] = snf.v[i][col];
            }
            nullspace_basis.push(basis_vec);
        }

        Ok(DiophantineSolution {
            particular,
            nullspace_basis,
        })
    }
}
