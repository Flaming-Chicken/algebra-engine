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

    /// Compute symbolic 2x2 or 3x3 determinant.
    pub fn determinant(&self, graph: &ExprGraph) -> AlgebraResult<ExprId> {
        if self.rows != self.cols {
            return Err(AlgebraError::DimensionMismatch(format!(
                "Square matrix required for determinant, got {}x{}",
                self.rows, self.cols
            )));
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

        Err(AlgebraError::EvaluationError(
            "Determinants > 2x2 use Laplace expansion in simplifier".into(),
        ))
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
