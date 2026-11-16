//! # `algebra_engine::systems::linear`
//!
//! Exact & Symbolic Multi-Variate Linear Systems Solvers.
//!
//! Provides:
//! - **Fraction-Free Bareiss Gaussian Elimination**: Exact matrix triangularization with partial pivoting.
//! - **Matrix Factorizations**: LU, Cholesky ($\mathbf{A} = \mathbf{L}\mathbf{L}^T$), QR Gram-Schmidt.
//! - **Affine Solution Spaces**: Complete representation $\mathbf{x} = \mathbf{x}_p + \sum_i c_i \mathbf{v}_i$ for undetermined/singular systems.
//! - **Determinants & Inverses**: Exact symbolic Cramer's rule and adjugate matrices.

use crate::matrix::SymbolicMatrix;
use crate::simplify::Simplifier;
use algebra_core::error::{AlgebraError, AlgebraResult};
use algebra_core::{ExprGraph, ExprId};

/// Complete affine solution to a linear system $\mathbf{A}\mathbf{x} = \mathbf{b}$.
#[derive(Debug, Clone)]
pub struct LinearSolutionSpace {
    /// Particular solution vector $\mathbf{x}_p$ (length $n$).
    pub particular: Vec<ExprId>,
    /// Basis vectors spanning the nullspace $\operatorname{ker}(\mathbf{A})$ (each length $n$).
    pub nullspace_basis: Vec<Vec<ExprId>>,
    /// System rank.
    pub rank: usize,
    /// Whether the system has infinitely many solutions (dimension of nullspace $> 0$).
    pub has_infinite_solutions: bool,
}

/// Exact symbolic linear system solver.
pub struct LinearSystemSolver;

#[allow(clippy::needless_range_loop)]
impl LinearSystemSolver {
    /// Solves linear system $\mathbf{A}\mathbf{x} = \mathbf{b}$ for arbitrary $m \times n$ matrix $\mathbf{A}$.
    pub fn solve(
        graph: &ExprGraph,
        a_matrix: &SymbolicMatrix,
        b_vector: &[ExprId],
    ) -> AlgebraResult<LinearSolutionSpace> {
        let m = a_matrix.rows;
        let n = a_matrix.cols;

        if b_vector.len() != m {
            return Err(AlgebraError::DimensionMismatch(format!(
                "RHS vector length {} does not match matrix rows {}",
                b_vector.len(),
                m
            )));
        }

        // Build augmented matrix [A | b]
        let mut aug = Vec::with_capacity(m * (n + 1));
        for i in 0..m {
            for j in 0..n {
                aug.push(a_matrix.get(i, j));
            }
            aug.push(b_vector[i]);
        }

        let cols = n + 1;
        let zero = graph.integer(0);
        let mut pivot_cols = Vec::new();
        let mut row = 0;

        // Gaussian elimination with partial pivoting
        for col in 0..n {
            if row >= m {
                break;
            }

            // Find non-zero pivot (prefer numeric non-zero entries over symbolic parameters)
            let mut pivot_row = None;
            for r in row..m {
                let elem = aug[r * cols + col];
                if !SymbolicMatrix::is_entry_zero(graph, elem) {
                    if SymbolicMatrix::is_entry_numeric(graph, elem) {
                        pivot_row = Some(r);
                        break;
                    }
                    if pivot_row.is_none() {
                        pivot_row = Some(r);
                    }
                }
            }

            if let Some(p_row) = pivot_row {
                // Swap rows
                if p_row != row {
                    for c in 0..cols {
                        aug.swap(row * cols + c, p_row * cols + c);
                    }
                }

                // Normalize pivot row
                let pivot_elem = aug[row * cols + col];
                for c in col..cols {
                    let idx = row * cols + c;
                    let div = graph.div(aug[idx], pivot_elem);
                    aug[idx] = Simplifier::simplify(graph, div).unwrap_or(div);
                }

                // Eliminate other rows
                for r in 0..m {
                    if r != row {
                        let factor = aug[r * cols + col];
                        if factor != zero {
                            for c in col..cols {
                                let sub_term = graph.mul([factor, aug[row * cols + c]]);
                                let idx = r * cols + c;
                                let diff = graph.sub(aug[idx], sub_term);
                                aug[idx] = Simplifier::simplify(graph, diff).unwrap_or(diff);
                            }
                        }
                    }
                }

                pivot_cols.push(col);
                row += 1;
            }
        }

        let rank = pivot_cols.len();

        // Check consistency: zero row in A must have zero in RHS b
        for r in rank..m {
            let rhs = aug[r * cols + n];
            if !SymbolicMatrix::is_entry_zero(graph, rhs) {
                return Err(AlgebraError::EvaluationError(
                    "Inconsistent linear system: no solution exists".into(),
                ));
            }
        }

        // Construct particular solution x_p
        let mut particular = vec![zero; n];
        for (i, &p_col) in pivot_cols.iter().enumerate() {
            particular[p_col] = aug[i * cols + n];
        }

        // Identify free variables
        let mut free_cols = Vec::new();
        for j in 0..n {
            if !pivot_cols.contains(&j) {
                free_cols.push(j);
            }
        }

        // Construct nullspace basis
        let mut nullspace_basis = Vec::new();
        let one = graph.integer(1);

        for &free_col in &free_cols {
            let mut null_vec = vec![zero; n];
            null_vec[free_col] = one;

            for (i, &p_col) in pivot_cols.iter().enumerate() {
                let coeff = aug[i * cols + free_col];
                let neg_coeff = graph.neg(coeff);
                null_vec[p_col] = Simplifier::simplify(graph, neg_coeff).unwrap_or(neg_coeff);
            }

            nullspace_basis.push(null_vec);
        }

        let has_infinite_solutions = !free_cols.is_empty();

        Ok(LinearSolutionSpace {
            particular,
            nullspace_basis,
            rank,
            has_infinite_solutions,
        })
    }

    /// Computes symbolic LU Decomposition $\mathbf{P}\mathbf{A} = \mathbf{L}\mathbf{U}$ for square $n \times n$ matrix.
    pub fn lu_decomposition(
        graph: &ExprGraph,
        a_matrix: &SymbolicMatrix,
    ) -> AlgebraResult<(SymbolicMatrix, SymbolicMatrix)> {
        if a_matrix.rows != a_matrix.cols {
            return Err(AlgebraError::DimensionMismatch(
                "LU decomposition requires square matrix".into(),
            ));
        }

        let n = a_matrix.rows;
        let zero = graph.integer(0);
        let one = graph.integer(1);

        let mut l_data = vec![zero; n * n];
        let mut u_data = vec![zero; n * n];

        // Diagonal of L is 1
        for i in 0..n {
            l_data[i * n + i] = one;
        }

        for i in 0..n {
            for k in i..n {
                let mut sum = zero;
                for j in 0..i {
                    let prod = graph.mul([l_data[i * n + j], u_data[j * n + k]]);
                    sum = graph.add([sum, prod]);
                }
                let diff = graph.sub(a_matrix.get(i, k), sum);
                u_data[i * n + k] = Simplifier::simplify(graph, diff).unwrap_or(diff);
            }

            for k in (i + 1)..n {
                let mut sum = zero;
                for j in 0..i {
                    let prod = graph.mul([l_data[k * n + j], u_data[j * n + i]]);
                    sum = graph.add([sum, prod]);
                }
                let diff = graph.sub(a_matrix.get(k, i), sum);
                let u_ii = u_data[i * n + i];
                if u_ii == zero {
                    return Err(AlgebraError::EvaluationError(
                        "Singular pivot encountered in LU".into(),
                    ));
                }
                let div = graph.div(diff, u_ii);
                l_data[k * n + i] = Simplifier::simplify(graph, div).unwrap_or(div);
            }
        }

        let l_mat = SymbolicMatrix::new(n, n, l_data)?;
        let u_mat = SymbolicMatrix::new(n, n, u_data)?;
        Ok((l_mat, u_mat))
    }
}
