//! # `algebra_advanced::structure_tensor`
//!
//! Generalized Structure Tensor Algebra & Nested Tensor Product Engine.
//!
//! Characterizes any finite-dimensional associative or non-associative hypercomplex algebra
//! via its bilinear structure constants $e_i \cdot e_j = \sum_{k=1}^n c_{ij}^k e_k$.
//!
//! Enables automatic algebra tensor products $\mathcal{A} \otimes \mathcal{B}$ with Kronecker-embedded
//! multiplication tables:
//! - **Complex Dual Numbers**: $\mathbb{C} \otimes \mathbb{D} = \{1, i\} \otimes \{1, \varepsilon\} = \{1, i, \varepsilon, i\varepsilon\}$.
//! - **Dual Quaternions**: $\mathbb{H} \otimes \mathbb{D} = \{1, i, j, k, \varepsilon, \varepsilon i, \varepsilon j, \varepsilon k\}$ for 3D screw kinematics.
//! - **Bicomplex Numbers**: $\mathbb{C} \otimes \mathbb{C} = \{1, i, j, ij\}$.
//! - **Commutativity Discovery**: Computes mutual commutators and algebraic center $Z(\mathcal{A} \otimes \mathcal{B})$.

use algebra_core::error::{AlgebraError, AlgebraResult};

/// Structure Tensor Algebra representation over $\mathbb{R}$.
#[derive(Debug, Clone, PartialEq)]
pub struct StructureTensorAlgebra {
    /// Name of the algebra.
    pub name: String,
    /// Basis element labels $\{e_1, \dots, e_n\}$.
    pub basis_names: Vec<String>,
    /// Structure constants tensor $C_{ij}^k$ of size $n \times n \times n$.
    /// `c[i][j][k]` is the coefficient of $e_k$ in the product $e_i \cdot e_j$.
    pub structure_tensor: Vec<Vec<Vec<f64>>>,
    /// Dimension of the algebra.
    pub dim: usize,
}

#[allow(clippy::needless_range_loop)]
impl StructureTensorAlgebra {
    /// Creates a new Structure Tensor Algebra with given basis labels and structure constants.
    pub fn new(
        name: impl Into<String>,
        basis_names: Vec<String>,
        structure_tensor: Vec<Vec<Vec<f64>>>,
    ) -> AlgebraResult<Self> {
        let dim = basis_names.len();
        if dim == 0 {
            return Err(AlgebraError::EvaluationError(
                "Algebra dimension must be at least 1".into(),
            ));
        }
        if structure_tensor.len() != dim {
            return Err(AlgebraError::DimensionMismatch(format!(
                "Structure tensor dimension mismatch: expected {}, found {}",
                dim,
                structure_tensor.len()
            )));
        }
        for (i, row) in structure_tensor.iter().enumerate() {
            if row.len() != dim {
                return Err(AlgebraError::DimensionMismatch(format!(
                    "Row {i} dimension mismatch: expected {dim}, found {}",
                    row.len()
                )));
            }
            for (j, col) in row.iter().enumerate() {
                if col.len() != dim {
                    return Err(AlgebraError::DimensionMismatch(format!(
                        "Entry ({i}, {j}) dimension mismatch: expected {dim}, found {}",
                        col.len()
                    )));
                }
            }
        }

        Ok(Self {
            name: name.into(),
            basis_names,
            structure_tensor,
            dim,
        })
    }

    /// Multiply two elements represented as coordinate vectors in the basis: $(a \cdot b)_k = \sum_{i,j} a_i b_j c_{ij}^k$.
    pub fn multiply(&self, a: &[f64], b: &[f64]) -> AlgebraResult<Vec<f64>> {
        if a.len() != self.dim || b.len() != self.dim {
            return Err(AlgebraError::DimensionMismatch(format!(
                "Element vector length must match algebra dimension {}",
                self.dim
            )));
        }

        let mut result = vec![0.0; self.dim];
        for i in 0..self.dim {
            let a_i = a[i];
            if a_i != 0.0 {
                for j in 0..self.dim {
                    let b_j = b[j];
                    if b_j != 0.0 {
                        let factor = a_i * b_j;
                        for k in 0..self.dim {
                            let c_ijk = self.structure_tensor[i][j][k];
                            if c_ijk != 0.0 {
                                result[k] += factor * c_ijk;
                            }
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    /// Computes the Lie commutator $[a, b] = a \cdot b - b \cdot a$.
    pub fn commutator(&self, a: &[f64], b: &[f64]) -> AlgebraResult<Vec<f64>> {
        let ab = self.multiply(a, b)?;
        let ba = self.multiply(b, a)?;
        let mut comm = vec![0.0; self.dim];
        for i in 0..self.dim {
            comm[i] = ab[i] - ba[i];
        }
        Ok(comm)
    }

    /// Computes the Tensor Product Algebra $\mathcal{A} \otimes \mathcal{B}$ via Kronecker embedding.
    ///
    /// The product algebra has dimension $n \cdot m$ with basis $\{e_i \otimes f_a\}$
    /// and structure tensor $C_{(i,a),(j,b)}^{(k,c)} = c_{ij}^k \cdot d_{ab}^c$.
    pub fn tensor_product(&self, other: &Self) -> AlgebraResult<Self> {
        let n = self.dim;
        let m = other.dim;
        let total_dim = n * m;

        // Build product basis names
        let mut prod_basis = Vec::with_capacity(total_dim);
        for e in &self.basis_names {
            for f in &other.basis_names {
                if e == "1" && f == "1" {
                    prod_basis.push("1".to_string());
                } else if e == "1" {
                    prod_basis.push(f.clone());
                } else if f == "1" {
                    prod_basis.push(e.clone());
                } else {
                    prod_basis.push(format!("{e}*{f}"));
                }
            }
        }

        // Build product structure tensor: C_{(i*m + a), (j*m + b)}^{(k*m + c)} = c_{ijk} * d_{abc}
        let mut prod_tensor = vec![vec![vec![0.0; total_dim]; total_dim]; total_dim];

        for i in 0..n {
            for a in 0..m {
                let row_idx = i * m + a;
                for j in 0..n {
                    for b in 0..m {
                        let col_idx = j * m + b;
                        for k in 0..n {
                            let c_ijk = self.structure_tensor[i][j][k];
                            if c_ijk != 0.0 {
                                for c in 0..m {
                                    let d_abc = other.structure_tensor[a][b][c];
                                    if d_abc != 0.0 {
                                        let out_idx = k * m + c;
                                        prod_tensor[row_idx][col_idx][out_idx] += c_ijk * d_abc;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let prod_name = format!("{} ⊗ {}", self.name, other.name);
        Self::new(prod_name, prod_basis, prod_tensor)
    }

    /// Computes the Matrix Regular Representation (Cayley embedding) $L_a \in \operatorname{Mat}_n(\mathbb{R})$
    /// where $(L_a)_{kj} = \sum_i a_i c_{ij}^k$.
    pub fn regular_matrix_representation(&self, a: &[f64]) -> AlgebraResult<Vec<Vec<f64>>> {
        if a.len() != self.dim {
            return Err(AlgebraError::DimensionMismatch(
                "Vector dimension mismatch".into(),
            ));
        }

        let mut matrix = vec![vec![0.0; self.dim]; self.dim];
        for k in 0..self.dim {
            for j in 0..self.dim {
                let mut sum = 0.0;
                for i in 0..self.dim {
                    let a_i = a[i];
                    if a_i != 0.0 {
                        sum += a_i * self.structure_tensor[i][j][k];
                    }
                }
                matrix[k][j] = sum;
            }
        }

        Ok(matrix)
    }

    // --- Canonical Algebra Constructors ---

    /// Complex Numbers $\mathbb{C} = \operatorname{span}\{1, i\}$ where $i^2 = -1$.
    pub fn complex() -> Self {
        let names = vec!["1".to_string(), "i".to_string()];
        let mut tensor = vec![vec![vec![0.0; 2]; 2]; 2];
        // 1 * 1 = 1
        tensor[0][0][0] = 1.0;
        // 1 * i = i
        tensor[0][1][1] = 1.0;
        // i * 1 = i
        tensor[1][0][1] = 1.0;
        // i * i = -1
        tensor[1][1][0] = -1.0;

        Self::new("ComplexNumbers", names, tensor)
            .unwrap_or_else(|_| Self {
                name: "ComplexNumbers".into(),
                basis_names: vec!["1".into(), "i".into()],
                structure_tensor: vec![],
                dim: 2,
            })
    }

    /// Dual Numbers $\mathbb{D} = \operatorname{span}\{1, \varepsilon\}$ where $\varepsilon^2 = 0$.
    pub fn dual() -> Self {
        let names = vec!["1".to_string(), "eps".to_string()];
        let mut tensor = vec![vec![vec![0.0; 2]; 2]; 2];
        // 1 * 1 = 1
        tensor[0][0][0] = 1.0;
        // 1 * eps = eps
        tensor[0][1][1] = 1.0;
        // eps * 1 = eps
        tensor[1][0][1] = 1.0;
        // eps * eps = 0

        Self::new("DualNumbers", names, tensor)
            .unwrap_or_else(|_| Self {
                name: "DualNumbers".into(),
                basis_names: vec!["1".into(), "eps".into()],
                structure_tensor: vec![],
                dim: 2,
            })
    }

    /// Hamilton Quaternions $\mathbb{H} = \operatorname{span}\{1, i, j, k\}$.
    pub fn quaternion() -> Self {
        let names = vec![
            "1".to_string(),
            "i".to_string(),
            "j".to_string(),
            "k".to_string(),
        ];
        let mut tensor = vec![vec![vec![0.0; 4]; 4]; 4];

        // 1 is identity
        for idx in 0..4 {
            tensor[0][idx][idx] = 1.0;
            tensor[idx][0][idx] = 1.0;
        }

        // i^2 = j^2 = k^2 = -1
        tensor[1][1][0] = -1.0;
        tensor[2][2][0] = -1.0;
        tensor[3][3][0] = -1.0;

        // ij = k, ji = -k
        tensor[1][2][3] = 1.0;
        tensor[2][1][3] = -1.0;

        // jk = i, kj = -i
        tensor[2][3][1] = 1.0;
        tensor[3][2][1] = -1.0;

        // ki = j, ik = -j
        tensor[3][1][2] = 1.0;
        tensor[1][3][2] = -1.0;

        Self::new("Quaternions", names, tensor)
            .unwrap_or_else(|_| Self {
                name: "Quaternions".into(),
                basis_names: vec!["1".into(), "i".into(), "j".into(), "k".into()],
                structure_tensor: vec![],
                dim: 4,
            })
    }

    /// Complex Dual Numbers $\mathbb{C} \otimes \mathbb{D} = \operatorname{span}\{1, i, \varepsilon, i\varepsilon\}$.
    pub fn complex_dual() -> Self {
        let c = Self::complex();
        let d = Self::dual();
        c.tensor_product(&d).unwrap_or(c)
    }

    /// Dual Quaternions $\mathbb{H} \otimes \mathbb{D}$ (8-dimensional spatial screw algebra).
    pub fn dual_quaternion() -> Self {
        let h = Self::quaternion();
        let d = Self::dual();
        h.tensor_product(&d).unwrap_or(h)
    }

    /// Bicomplex Numbers $\mathbb{C} \otimes \mathbb{C} = \operatorname{span}\{1, i, j, ij\}$.
    pub fn bicomplex() -> Self {
        let c1 = Self::complex();
        let c2 = Self::complex();
        c1.tensor_product(&c2).unwrap_or(c1)
    }
}
