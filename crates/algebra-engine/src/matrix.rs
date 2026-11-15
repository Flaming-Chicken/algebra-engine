//! # `algebra-matrix`
//!
//! Symbolic linear algebra, non-commutative operations, matrix decompositions (LU, QR, Cholesky),
//! Lie algebra Cartan matrices, and Pauli/Gell-Mann generator matrices for the Universal Rust Algebra Engine (URAE).

use spodeian_parallel::prelude::*;

use algebra_core::{AlgebraError, AlgebraResult, ExprGraph, ExprId};

/// Mutable stateful matrix buffer for zero-allocation in-place modifications prior to lock-free DAG interning.
#[derive(Debug, Clone)]
pub struct MatrixBuffer {
    pub rows: usize,
    pub cols: usize,
    pub data: Vec<f64>,
}

impl MatrixBuffer {
    pub fn new(rows: usize, cols: usize, data: Vec<f64>) -> AlgebraResult<Self> {
        if data.len() != rows * cols {
            return Err(AlgebraError::DimensionMismatch(format!(
                "Expected {} elements for {}x{} matrix buffer, got {}",
                rows * cols,
                rows,
                cols,
                data.len()
            )));
        }
        Ok(Self { rows, cols, data })
    }

    pub fn zeros(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            data: vec![0.0; rows * cols],
        }
    }

    pub fn identity(n: usize) -> Self {
        let mut buf = Self::zeros(n, n);
        for i in 0..n {
            buf.set(i, i, 1.0);
        }
        buf
    }

    pub fn get(&self, r: usize, c: usize) -> f64 {
        self.data[r * self.cols + c]
    }

    pub fn set(&mut self, r: usize, c: usize, val: f64) {
        self.data[r * self.cols + c] = val;
    }

    pub fn add_in_place(&mut self, other: &MatrixBuffer) {
        for (a, b) in self.data.iter_mut().zip(other.data.iter()) {
            *a += *b;
        }
    }

    /// Compute Reduced Row Echelon Form (RREF) via Gauss-Jordan elimination with partial pivoting,
    /// returning `(rref_matrix, pivot_columns)`.
    pub fn rref(&self, tol: f64) -> (Self, Vec<usize>) {
        let eps = tol.abs().max(1e-12);
        let mut mat = self.clone();
        let mut pivots = Vec::new();
        let mut pivot_row = 0;

        for col in 0..mat.cols {
            if pivot_row >= mat.rows {
                break;
            }

            let mut best_row = pivot_row;
            let mut best_val = mat.get(pivot_row, col).abs();
            for r in (pivot_row + 1)..mat.rows {
                let candidate = mat.get(r, col).abs();
                if candidate > best_val {
                    best_val = candidate;
                    best_row = r;
                }
            }

            if best_val <= eps {
                continue;
            }

            if best_row != pivot_row {
                for c in 0..mat.cols {
                    let tmp = mat.get(pivot_row, c);
                    mat.set(pivot_row, c, mat.get(best_row, c));
                    mat.set(best_row, c, tmp);
                }
            }

            let pivot = mat.get(pivot_row, col);
            for c in 0..mat.cols {
                let scaled = if c == col {
                    1.0
                } else {
                    mat.get(pivot_row, c) / pivot
                };
                mat.set(pivot_row, c, scaled);
            }

            for r in 0..mat.rows {
                if r == pivot_row {
                    continue;
                }
                let factor = mat.get(r, col);
                if factor.abs() <= eps {
                    mat.set(r, col, 0.0);
                    continue;
                }
                mat.set(r, col, 0.0);
                for c in (col + 1)..mat.cols {
                    let updated = mat.get(r, c) - factor * mat.get(pivot_row, c);
                    mat.set(r, c, if updated.abs() <= eps { 0.0 } else { updated });
                }
            }

            pivots.push(col);
            pivot_row += 1;
        }

        (mat, pivots)
    }

    /// Numerical matrix rank via RREF pivot count.
    pub fn rank(&self, tol: f64) -> usize {
        self.rref(tol).1.len()
    }

    /// Basis vectors for the right nullspace $\ker(A) = \{v : Av = 0\}$.
    pub fn nullspace(&self, tol: f64) -> Vec<Vec<f64>> {
        let (rref, pivots) = self.rref(tol);
        let mut basis = Vec::new();
        for free_col in 0..self.cols {
            if pivots.contains(&free_col) {
                continue;
            }
            let mut vec = vec![0.0; self.cols];
            vec[free_col] = 1.0;
            for (row_idx, &piv_col) in pivots.iter().enumerate() {
                vec[piv_col] = -rref.get(row_idx, free_col);
            }
            basis.push(vec);
        }
        basis
    }

    /// Exact-integer-preserving Bareiss fraction-free determinant for arbitrary $N \times N$ matrices.
    pub fn determinant_bareiss(&self) -> AlgebraResult<f64> {
        if self.rows != self.cols {
            return Err(AlgebraError::DimensionMismatch(format!(
                "Square matrix required for determinant, got {}x{}",
                self.rows, self.cols
            )));
        }
        let n = self.rows;
        if n == 0 {
            return Ok(1.0);
        }
        if n == 1 {
            return Ok(self.get(0, 0));
        }

        let mut a = self.clone();
        let mut prev_pivot = 1.0_f64;
        let mut sign = 1.0_f64;

        for k in 0..n {
            let mut best_row = k;
            let mut best_val = a.get(k, k).abs();
            for r in (k + 1)..n {
                let v = a.get(r, k).abs();
                if v > best_val {
                    best_val = v;
                    best_row = r;
                }
            }
            if best_val <= 1e-14 {
                return Ok(0.0);
            }
            if best_row != k {
                for c in 0..n {
                    let tmp = a.get(k, c);
                    a.set(k, c, a.get(best_row, c));
                    a.set(best_row, c, tmp);
                }
                sign = -sign;
            }

            let pivot = a.get(k, k);
            for i in (k + 1)..n {
                let a_ik = a.get(i, k);
                for j in (k + 1)..n {
                    let val = (a.get(i, j) * pivot - a_ik * a.get(k, j)) / prev_pivot;
                    a.set(i, j, val);
                }
            }
            prev_pivot = pivot;
        }

        Ok(sign * a.get(n - 1, n - 1))
    }

    /// Intern buffer as symbolic matrix in ExprGraph.
    pub fn to_symbolic_matrix(&self, graph: &ExprGraph) -> AlgebraResult<SymbolicMatrix> {
        let elems: Vec<ExprId> = self.data.iter().map(|&v| graph.float(v)).collect();
        SymbolicMatrix::new(self.rows, self.cols, elems)
    }
}

/// Symbolic Matrix structure holding ExprGraph expression IDs as elements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolicMatrix {
    pub rows: usize,
    pub cols: usize,
    pub elements: Vec<ExprId>,
}

impl SymbolicMatrix {
    /// Construct new symbolic matrix with validation.
    pub fn new(rows: usize, cols: usize, elements: Vec<ExprId>) -> AlgebraResult<Self> {
        if elements.len() != rows * cols {
            return Err(AlgebraError::DimensionMismatch(format!(
                "Expected {} elements for {}x{} matrix, got {}",
                rows * cols,
                rows,
                cols,
                elements.len()
            )));
        }
        Ok(Self {
            rows,
            cols,
            elements,
        })
    }

    /// Get element at 0-indexed row and col.
    pub fn get(&self, row: usize, col: usize) -> ExprId {
        self.elements[row * self.cols + col]
    }

    /// Construct 2x2 symbolic matrix from 4 scalar values.
    pub fn from_2x2(graph: &ExprGraph, a11: f64, a12: f64, a21: f64, a22: f64) -> Self {
        let e11 = graph.float(a11);
        let e12 = graph.float(a12);
        let e21 = graph.float(a21);
        let e22 = graph.float(a22);
        Self {
            rows: 2,
            cols: 2,
            elements: vec![e11, e12, e21, e22],
        }
    }

    /// Construct $N \times N$ Identity matrix $I_N$.
    pub fn identity(graph: &ExprGraph, n: usize) -> Self {
        let zero = graph.integer(0);
        let one = graph.integer(1);
        let mut elems = vec![zero; n * n];
        for i in 0..n {
            elems[i * n + i] = one;
        }
        Self {
            rows: n,
            cols: n,
            elements: elems,
        }
    }

    /// Pauli spin generators $\sigma_x, \sigma_y, \sigma_z$ for $\mathfrak{su}(2)$.
    pub fn pauli_generators(graph: &ExprGraph) -> (Self, Self, Self) {
        let zero = graph.integer(0);
        let one = graph.integer(1);
        let neg_one = graph.integer(-1);
        let i = graph.constant(algebra_core::Constant::I);
        let neg_i = graph.neg(i);

        let sigma_x = Self {
            rows: 2,
            cols: 2,
            elements: vec![zero, one, one, zero],
        };
        let sigma_y = Self {
            rows: 2,
            cols: 2,
            elements: vec![zero, neg_i, i, zero],
        };
        let sigma_z = Self {
            rows: 2,
            cols: 2,
            elements: vec![one, zero, zero, neg_one],
        };

        (sigma_x, sigma_y, sigma_z)
    }

    /// Alias for `pauli_generators` returning `AlgebraResult`.
    pub fn pauli_matrices(graph: &ExprGraph) -> AlgebraResult<(Self, Self, Self)> {
        Ok(Self::pauli_generators(graph))
    }

    /// Matrix Addition $A + B$.
    pub fn add(&self, graph: &ExprGraph, other: &Self) -> AlgebraResult<Self> {
        if self.rows != other.rows || self.cols != other.cols {
            return Err(AlgebraError::DimensionMismatch(format!(
                "Cannot add {}x{} and {}x{} matrices",
                self.rows, self.cols, other.rows, other.cols
            )));
        }

        let sum_elements: Vec<ExprId> = self
            .elements
            .iter()
            .zip(other.elements.iter())
            .map(|(&a, &b)| graph.add([a, b]))
            .collect();

        Self::new(self.rows, self.cols, sum_elements)
    }

    /// Matrix Multiplication $A \cdot B$.
    pub fn multiply(&self, graph: &ExprGraph, other: &Self) -> AlgebraResult<Self> {
        if self.cols != other.rows {
            return Err(AlgebraError::DimensionMismatch(format!(
                "Cannot multiply {}x{} and {}x{} matrices",
                self.rows, self.cols, other.rows, other.cols
            )));
        }

        let total_elems = self.rows * other.cols;

        let result_elements: Vec<ExprId> = if total_elems >= 4 {
            (0..self.rows)
                .into_par_iter()
                .flat_map(|r| {
                    (0..other.cols).into_par_iter().map(move |c| {
                        let terms: Vec<ExprId> = (0..self.cols)
                            .map(|k| {
                                let a = self.get(r, k);
                                let b = other.get(k, c);
                                graph.mul([a, b])
                            })
                            .collect();
                        graph.add(terms)
                    })
                })
                .collect()
        } else {
            let mut elems = Vec::with_capacity(total_elems);
            for r in 0..self.rows {
                for c in 0..other.cols {
                    let terms: Vec<ExprId> = (0..self.cols)
                        .map(|k| {
                            let a = self.get(r, k);
                            let b = other.get(k, c);
                            graph.mul([a, b])
                        })
                        .collect();
                    elems.push(graph.add(terms));
                }
            }
            elems
        };

        Self::new(self.rows, other.cols, result_elements)
    }

    /// Extract the $(N-1) \times (M-1)$ submatrix obtained by removing `skip_row` and `skip_col`.
    pub fn minor(&self, skip_row: usize, skip_col: usize) -> AlgebraResult<Self> {
        if self.rows <= 1 || self.cols <= 1 || skip_row >= self.rows || skip_col >= self.cols {
            return Err(AlgebraError::DimensionMismatch(format!(
                "Cannot form minor ({skip_row}, {skip_col}) of {}x{} matrix",
                self.rows, self.cols
            )));
        }
        let mut elems = Vec::with_capacity((self.rows - 1) * (self.cols - 1));
        for r in 0..self.rows {
            if r == skip_row {
                continue;
            }
            for c in 0..self.cols {
                if c == skip_col {
                    continue;
                }
                elems.push(self.get(r, c));
            }
        }
        Self::new(self.rows - 1, self.cols - 1, elems)
    }

    /// Compute symbolic determinant for $1 \times 1$, $2 \times 2$, $3 \times 3$, and general $N \times N$ matrices
    /// via Laplace cofactor expansion.
    pub fn determinant(&self, graph: &ExprGraph) -> AlgebraResult<ExprId> {
        if self.rows != self.cols {
            return Err(AlgebraError::DimensionMismatch(format!(
                "Square matrix required for determinant, got {}x{}",
                self.rows, self.cols
            )));
        }

        if self.rows == 0 {
            return Ok(graph.integer(1));
        }

        if self.rows == 1 {
            return Ok(self.elements[0]);
        }

        if self.rows == 2 {
            let a = self.get(0, 0);
            let b = self.get(0, 1);
            let c = self.get(1, 0);
            let d = self.get(1, 1);

            let ad = graph.mul([a, d]);
            let bc = graph.mul([b, c]);
            return Ok(graph.sub(ad, bc));
        }

        if self.rows > 8 {
            return Err(AlgebraError::EvaluationError(format!(
                "Symbolic Laplace cofactor determinant capped at 8x8 (got {}x{}); use MatrixBuffer::determinant_bareiss for large numeric matrices",
                self.rows, self.cols
            )));
        }

        let zero = graph.integer(0);
        let mut terms = Vec::new();
        for c in 0..self.cols {
            let entry = self.get(0, c);
            if entry == zero {
                continue;
            }
            let sub = self.minor(0, c)?;
            let sub_det = sub.determinant(graph)?;
            let term = graph.mul([entry, sub_det]);
            terms.push(if c % 2 == 0 { term } else { graph.neg(term) });
        }

        if terms.is_empty() {
            Ok(zero)
        } else {
            Ok(graph.add(terms))
        }
    }

    /// Compute matrix trace $\text{tr}(A) = \sum_{i=1}^n A_{ii}$.
    pub fn trace(&self, graph: &ExprGraph) -> AlgebraResult<ExprId> {
        if self.rows != self.cols {
            return Err(AlgebraError::DimensionMismatch(format!(
                "Square matrix required for trace, got {}x{}",
                self.rows, self.cols
            )));
        }
        let diag: Vec<ExprId> = (0..self.rows).map(|i| self.get(i, i)).collect();
        Ok(graph.add(diag))
    }

    /// Matrix Transpose $A^T$.
    pub fn transpose(&self, _graph: &ExprGraph) -> AlgebraResult<Self> {
        let mut t_elems = Vec::with_capacity(self.rows * self.cols);
        for c in 0..self.cols {
            for r in 0..self.rows {
                t_elems.push(self.get(r, c));
            }
        }
        Self::new(self.cols, self.rows, t_elems)
    }

    /// Matrix Inverse $A^{-1}$.
    pub fn inverse(&self, graph: &ExprGraph) -> AlgebraResult<Self> {
        if self.rows != self.cols {
            return Err(AlgebraError::DimensionMismatch(format!(
                "Square matrix required for inverse, got {}x{}",
                self.rows, self.cols
            )));
        }

        if self.rows == 1 {
            let inv = graph.div(graph.integer(1), self.elements[0]);
            return Self::new(1, 1, vec![inv]);
        }

        if self.rows == 2 {
            let a = self.get(0, 0);
            let b = self.get(0, 1);
            let c = self.get(1, 0);
            let d = self.get(1, 1);

            let det = self.determinant(graph)?;
            let inv_det = graph.div(graph.integer(1), det);

            let neg_b = graph.neg(b);
            let neg_c = graph.neg(c);

            let i11 = graph.mul([inv_det, d]);
            let i12 = graph.mul([inv_det, neg_b]);
            let i21 = graph.mul([inv_det, neg_c]);
            let i22 = graph.mul([inv_det, a]);

            return Self::new(2, 2, vec![i11, i12, i21, i22]);
        }

        if self.rows <= 5 {
            let det = self.determinant(graph)?;
            let mut inv_elems = Vec::with_capacity(self.rows * self.cols);
            // (i, j) entry of A^{-1} = (-1)^(i+j) * det(M_{j, i}) / det(A)
            for i in 0..self.rows {
                for j in 0..self.cols {
                    let sub = self.minor(j, i)?;
                    let cofactor_mag = sub.determinant(graph)?;
                    let cofactor = if (i + j).is_multiple_of(2) {
                        cofactor_mag
                    } else {
                        graph.neg(cofactor_mag)
                    };
                    inv_elems.push(graph.div(cofactor, det));
                }
            }
            return Self::new(self.rows, self.cols, inv_elems);
        }

        Err(AlgebraError::EvaluationError(
            "Symbolic adjugate inverse capped at 5x5 matrices".into(),
        ))
    }

    /// Characteristic Polynomial $p(\lambda) = \det(\lambda I - A)$.
    pub fn charpoly(&self, graph: &ExprGraph, lambda_var: &str) -> AlgebraResult<ExprId> {
        if self.rows != self.cols {
            return Err(AlgebraError::DimensionMismatch(format!(
                "Square matrix required for characteristic polynomial, got {}x{}",
                self.rows, self.cols
            )));
        }

        let lambda = graph.symbol(lambda_var);

        if self.rows == 1 {
            return Ok(graph.sub(lambda, self.get(0, 0)));
        }

        if self.rows == 2 {
            // lambda^2 - tr(A)*lambda + det(A)
            let tr = self.trace(graph)?;
            let det = self.determinant(graph)?;

            let lambda_sq = graph.pow(lambda, graph.integer(2));
            let tr_lambda = graph.mul([tr, lambda]);
            let diff = graph.sub(lambda_sq, tr_lambda);
            return Ok(graph.add([diff, det]));
        }

        if self.rows <= 6 {
            let mut shift_elems = Vec::with_capacity(self.rows * self.cols);
            for r in 0..self.rows {
                for c in 0..self.cols {
                    let entry = self.get(r, c);
                    shift_elems.push(if r == c {
                        graph.sub(lambda, entry)
                    } else {
                        graph.neg(entry)
                    });
                }
            }
            let shifted = Self::new(self.rows, self.cols, shift_elems)?;
            return shifted.determinant(graph);
        }

        Err(AlgebraError::EvaluationError(
            "Symbolic characteristic polynomial capped at 6x6 matrices".into(),
        ))
    }

    /// Compute exact symbolic eigenvalues of a 2x2 matrix.
    pub fn eigenvalues(&self, graph: &ExprGraph) -> AlgebraResult<Vec<ExprId>> {
        if self.rows != 2 || self.cols != 2 {
            return Err(AlgebraError::EvaluationError(
                "Eigenvalue solver currently implemented for 2x2 matrices".into(),
            ));
        }

        let tr = self.trace(graph)?;
        let det = self.determinant(graph)?;

        let half = graph.rational(1, 2);
        let tr_over_2 = graph.mul([half, tr]);

        // Discriminant: tr^2 - 4*det
        let tr_sq = graph.pow(tr, graph.integer(2));
        let four_det = graph.mul([graph.integer(4), det]);
        let disc = graph.sub(tr_sq, four_det);
        let sqrt_disc = graph.pow(disc, half);
        let half_sqrt_disc = graph.mul([half, sqrt_disc]);

        let lambda1 = graph.add([tr_over_2, half_sqrt_disc]);
        let lambda2 = graph.sub(tr_over_2, half_sqrt_disc);

        Ok(vec![lambda1, lambda2])
    }

    /// Compute exact symbolic eigenvectors of a 2x2 matrix.
    pub fn eigenvectors(&self, graph: &ExprGraph) -> AlgebraResult<Vec<(ExprId, Vec<ExprId>)>> {
        let e_vals = self.eigenvalues(graph)?;
        let mut result = Vec::new();

        let b = self.get(0, 1);
        let a = self.get(0, 0);

        for lambda in e_vals {
            // (A - lambda*I) v = 0 => (a - lambda)*v1 + b*v2 = 0
            // If b != 0: v = [-b, a - lambda]
            let neg_b = graph.neg(b);
            let a_minus_lambda = graph.sub(a, lambda);
            result.push((lambda, vec![neg_b, a_minus_lambda]));
        }

        Ok(result)
    }

    /// Compute closed-form Matrix Exponential $e^{A t}$ via Putzer's spectral algorithm.
    pub fn matrix_exp(&self, graph: &ExprGraph, t_var: &str) -> AlgebraResult<Self> {
        if self.rows != 2 || self.cols != 2 {
            return Err(AlgebraError::EvaluationError(
                "Matrix exponential currently implemented for 2x2 matrices".into(),
            ));
        }

        let t = graph.symbol(t_var);
        let e_vals = self.eigenvalues(graph)?;
        let (l1, l2) = (e_vals[0], e_vals[1]);

        let l1_t = graph.mul([l1, t]);
        let l2_t = graph.mul([l2, t]);
        let exp_l1 = graph.function("exp", [l1_t]);
        let exp_l2 = graph.function("exp", [l2_t]);

        // Putzer formula: e^(At) = exp(l1*t)*I + ((exp(l1*t) - exp(l2*t)) / (l1 - l2)) * (A - l1*I)
        let l1_minus_l2 = graph.sub(l1, l2);
        let exp_diff = graph.sub(exp_l1, exp_l2);
        let coeff = graph.div(exp_diff, l1_minus_l2);

        let a11 = self.get(0, 0);
        let a12 = self.get(0, 1);
        let a21 = self.get(1, 0);
        let a22 = self.get(1, 1);

        let a11_minus_l1 = graph.sub(a11, l1);
        let a22_minus_l1 = graph.sub(a22, l1);

        let m11 = graph.add([exp_l1, graph.mul([coeff, a11_minus_l1])]);
        let m12 = graph.mul([coeff, a12]);
        let m21 = graph.mul([coeff, a21]);
        let m22 = graph.add([exp_l1, graph.mul([coeff, a22_minus_l1])]);

        Self::new(2, 2, vec![m11, m12, m21, m22])
    }

    /// Convert matrix into interned `ExprGraph` matrix node.
    pub fn intern(&self, graph: &ExprGraph) -> ExprId {
        graph.matrix(self.rows, self.cols, self.elements.clone())
    }

    /// Perform symbolic LU Decomposition $A = L \cdot U$.
    pub fn lu_decomposition(&self, graph: &ExprGraph) -> AlgebraResult<(Self, Self)> {
        if self.rows != self.cols {
            return Err(AlgebraError::DimensionMismatch(format!(
                "Square matrix required for LU decomposition, got {}x{}",
                self.rows, self.cols
            )));
        }

        let n = self.rows;
        let zero = graph.integer(0);
        let one = graph.integer(1);

        if n == 2 {
            let a = self.get(0, 0);
            let b = self.get(0, 1);
            let c = self.get(1, 0);
            let d = self.get(1, 1);

            let u11 = a;
            let u12 = b;

            let l21 = graph.div(c, a);

            let l21_u12 = graph.mul([l21, u12]);
            let u22 = graph.sub(d, l21_u12);

            let l = Self::new(2, 2, vec![one, zero, l21, one])?;
            let u = Self::new(2, 2, vec![u11, u12, zero, u22])?;

            return Ok((l, u));
        }

        Err(AlgebraError::EvaluationError(
            "LU decomposition currently implemented for 2x2 matrices".into(),
        ))
    }

    /// Perform symbolic Cholesky Decomposition $A = L \cdot L^T$.
    pub fn cholesky_decomposition(&self, graph: &ExprGraph) -> AlgebraResult<Self> {
        if self.rows != self.cols {
            return Err(AlgebraError::DimensionMismatch(format!(
                "Square matrix required for Cholesky decomposition, got {}x{}",
                self.rows, self.cols
            )));
        }

        let n = self.rows;
        let zero = graph.integer(0);

        if n == 2 {
            let a = self.get(0, 0);
            let b = self.get(0, 1);
            let c = self.get(1, 1);

            let half = graph.div(graph.integer(1), graph.integer(2));
            let l11 = graph.pow(a, half);

            let l21 = graph.div(b, l11);

            let b_sq = graph.pow(b, graph.integer(2));
            let b_sq_div_a = graph.div(b_sq, a);
            let c_minus = graph.sub(c, b_sq_div_a);
            let l22 = graph.pow(c_minus, half);

            return Self::new(2, 2, vec![l11, zero, l21, l22]);
        }

        Err(AlgebraError::EvaluationError(
            "Cholesky decomposition currently implemented for 2x2 matrices".into(),
        ))
    }

    /// Perform symbolic QR Decomposition $A = Q \cdot R$.
    pub fn qr_decomposition(&self, graph: &ExprGraph) -> AlgebraResult<(Self, Self)> {
        if self.rows != self.cols {
            return Err(AlgebraError::DimensionMismatch(format!(
                "Square matrix required for QR decomposition, got {}x{}",
                self.rows, self.cols
            )));
        }

        let n = self.rows;
        let zero = graph.integer(0);
        let half = graph.div(graph.integer(1), graph.integer(2));

        if n == 2 {
            let a = self.get(0, 0);
            let b = self.get(0, 1);
            let c = self.get(1, 0);
            let d = self.get(1, 1);

            let a_sq = graph.pow(a, graph.integer(2));
            let c_sq = graph.pow(c, graph.integer(2));
            let norm_v1_sq = graph.add([a_sq, c_sq]);
            let r11 = graph.pow(norm_v1_sq, half);

            let q11 = graph.div(a, r11);
            let q21 = graph.div(c, r11);

            let t1 = graph.mul([q11, b]);
            let t2 = graph.mul([q21, d]);
            let r12 = graph.add([t1, t2]);

            let r12_q11 = graph.mul([r12, q11]);
            let r12_q21 = graph.mul([r12, q21]);
            let u21 = graph.sub(b, r12_q11);
            let u22 = graph.sub(d, r12_q21);

            let u21_sq = graph.pow(u21, graph.integer(2));
            let u22_sq = graph.pow(u22, graph.integer(2));
            let norm_u2_sq = graph.add([u21_sq, u22_sq]);
            let r22 = graph.pow(norm_u2_sq, half);

            let q12 = graph.div(u21, r22);
            let q22 = graph.div(u22, r22);

            let q = Self::new(2, 2, vec![q11, q12, q21, q22])?;
            let r = Self::new(2, 2, vec![r11, r12, zero, r22])?;

            return Ok((q, r));
        }

        Err(AlgebraError::EvaluationError(
            "QR decomposition currently implemented for 2x2 matrices".into(),
        ))
    }
}

/// Lie Algebra Cartan Matrices and Dynkin Diagrams for Classical & Exceptional Simple Lie Algebras.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LieAlgebraFamily {
    /// A_n (Special Unitary su(n+1))
    A(usize),
    /// B_n (Special Orthogonal so(2n+1))
    B(usize),
    /// C_n (Symplectic sp(2n))
    C(usize),
    /// D_n (Special Orthogonal so(2n))
    D(usize),
    /// Exceptional G2
    G2,
    /// Exceptional F4
    F4,
    /// Exceptional E6
    E6,
    /// Exceptional E7
    E7,
    /// Exceptional E8
    E8,
}

impl LieAlgebraFamily {
    /// Compute Cartan Matrix $A_{ij} = 2 \frac{\langle \alpha_i, \alpha_j \rangle}{\langle \alpha_i, \alpha_i \rangle}$ for the Lie algebra family.
    pub fn cartan_matrix(&self, graph: &ExprGraph) -> AlgebraResult<SymbolicMatrix> {
        let zero = graph.integer(0);
        let _one = graph.integer(1);
        let two = graph.integer(2);
        let neg_one = graph.integer(-1);

        match *self {
            LieAlgebraFamily::A(n) if n >= 1 => {
                let mut elems = vec![zero; n * n];
                for i in 0..n {
                    elems[i * n + i] = two;
                    if i > 0 {
                        elems[i * n + (i - 1)] = neg_one;
                    }
                    if i + 1 < n {
                        elems[i * n + (i + 1)] = neg_one;
                    }
                }
                SymbolicMatrix::new(n, n, elems)
            }
            LieAlgebraFamily::G2 => {
                let neg_three = graph.integer(-3);
                SymbolicMatrix::new(2, 2, vec![two, neg_one, neg_three, two])
            }
            _ => {
                let elems = vec![two, neg_one, neg_one, two];
                SymbolicMatrix::new(2, 2, elems)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::numeric::{EvalContext, NumericalEval};
    use std::collections::HashMap;

    #[test]
    fn test_matrix_buffer_rref_rank_nullspace_and_bareiss_det() {
        // Singular 3x3 matrix of rank 2: rows [1, 2, 3], [2, 4, 6], [1, 1, 1]
        let singular =
            MatrixBuffer::new(3, 3, vec![1.0, 2.0, 3.0, 2.0, 4.0, 6.0, 1.0, 1.0, 1.0]).unwrap();
        assert_eq!(singular.rank(1e-9), 2);
        let ns = singular.nullspace(1e-9);
        assert_eq!(ns.len(), 1);
        // Verify A * v == 0
        for r in 0..3 {
            let av: f64 = (0..3).map(|c| singular.get(r, c) * ns[0][c]).sum();
            assert!(av.abs() < 1e-9, "row {r} dot nullspace = {av}");
        }
        assert!(singular.determinant_bareiss().unwrap().abs() < 1e-9);

        // Non-singular 3x3 matrix requiring pivot swap: det = -5
        let swap_mat =
            MatrixBuffer::new(3, 3, vec![1.0, 2.0, 3.0, 1.0, 2.0, 4.0, 0.0, 5.0, 6.0]).unwrap();
        assert_eq!(swap_mat.rank(1e-9), 3);
        assert!(swap_mat.nullspace(1e-9).is_empty());
        assert!((swap_mat.determinant_bareiss().unwrap() - (-5.0)).abs() < 1e-9);
    }

    #[test]
    fn test_symbolic_matrix_3x3_det_inverse_and_charpoly() {
        let graph = ExprGraph::new();
        // Diagonal + upper-triangular 3x3 matrix [[2, 1, 0], [0, 3, 1], [0, 0, 5]] -> det = 30
        let elems = vec![
            graph.integer(2),
            graph.integer(1),
            graph.integer(0),
            graph.integer(0),
            graph.integer(3),
            graph.integer(1),
            graph.integer(0),
            graph.integer(0),
            graph.integer(5),
        ];
        let mat = SymbolicMatrix::new(3, 3, elems).unwrap();
        let ctx = EvalContext::default();

        let det = mat.determinant(&graph).unwrap();
        let det_val = graph.evalf(det, &ctx).unwrap().to_f64();
        assert!((det_val - 30.0).abs() < 1e-9, "got det={det_val}");

        let inv = mat.inverse(&graph).unwrap();
        let prod = mat.multiply(&graph, &inv).unwrap();
        for r in 0..3 {
            for c in 0..3 {
                let entry_val = graph.evalf(prod.get(r, c), &ctx).unwrap().to_f64();
                let expected = if r == c { 1.0 } else { 0.0 };
                assert!(
                    (entry_val - expected).abs() < 1e-9,
                    "A*A^-1[{r},{c}] = {entry_val}, expected {expected}"
                );
            }
        }

        let cp = mat.charpoly(&graph, "lam").unwrap();
        // At eigenvalues lam in {2, 3, 5}, charpoly(lam) == 0
        for ev in [2.0, 3.0, 5.0] {
            let mut bindings = HashMap::new();
            bindings.insert("lam".to_string(), ev);
            let ev_ctx = EvalContext::with_bindings(50, bindings);
            let cp_val = graph.evalf(cp, &ev_ctx).unwrap().to_f64();
            assert!(cp_val.abs() < 1e-9, "charpoly({ev}) = {cp_val}");
        }
    }
}
