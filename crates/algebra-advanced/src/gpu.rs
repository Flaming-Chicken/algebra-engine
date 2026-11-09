//! # `algebra-gpu`
//!
//! SIMD-vectorized operations and GPU compute shader offloading abstraction for
//! large-scale polynomial multiplication, dense matrix blocks, and massive tensor contractions.

use spodeian_parallel::prelude::*;

use algebra_core::{AlgebraError, AlgebraResult, ExprGraph};
use algebra_engine::matrix::SymbolicMatrix;

/// SIMD-vectorized polynomial block operations.
#[derive(Debug, Clone)]
pub struct SimdPolyVector {
    pub coeffs: Vec<i64>,
}

impl SimdPolyVector {
    pub fn new(coeffs: Vec<i64>) -> Self {
        Self { coeffs }
    }

    /// Parallel chunked vector addition.
    pub fn add_simd(&self, other: &Self) -> Self {
        let max_len = self.coeffs.len().max(other.coeffs.len());
        let mut result = vec![0i64; max_len];

        result.par_iter_mut().enumerate().for_each(|(idx, out)| {
            let a = self.coeffs.get(idx).copied().unwrap_or(0);
            let b = other.coeffs.get(idx).copied().unwrap_or(0);
            *out = a.wrapping_add(b);
        });

        Self { coeffs: result }
    }

    /// Parallel chunked polynomial multiplication using vectorized component products.
    pub fn multiply_simd(&self, other: &Self) -> Self {
        if self.coeffs.is_empty() || other.coeffs.is_empty() {
            return Self { coeffs: Vec::new() };
        }

        let res_len = self.coeffs.len() + other.coeffs.len() - 1;
        let mut result = vec![0i64; res_len];

        result.par_iter_mut().enumerate().for_each(|(k, out)| {
            let mut sum = 0i64;
            let start = if k >= other.coeffs.len() {
                k + 1 - other.coeffs.len()
            } else {
                0
            };
            let end = (k + 1).min(self.coeffs.len());
            for i in start..end {
                let j = k - i;
                sum = sum.wrapping_add(self.coeffs[i].wrapping_mul(other.coeffs[j]));
            }
            *out = sum;
        });

        Self { coeffs: result }
    }
}


/// Compute Shader / GPU Execution Kernel Abstraction.
#[derive(Debug, Clone, Default)]
pub struct GpuComputeKernel {
    pub enabled: bool,
}

impl GpuComputeKernel {
    pub fn new() -> Self {
        Self { enabled: true }
    }

    /// Offload massive dense matrix multiplication ($N \times N$) using parallel block kernels.
    pub fn multiply_matrix_blocks(
        &self,
        graph: &ExprGraph,
        a: &SymbolicMatrix,
        b: &SymbolicMatrix,
    ) -> AlgebraResult<SymbolicMatrix> {
        if a.cols != b.rows {
            return Err(AlgebraError::DimensionMismatch(format!(
                "GPU matrix multiply dimension mismatch: {}x{} and {}x{}",
                a.rows, a.cols, b.rows, b.cols
            )));
        }

        let rows = a.rows;
        let cols = b.cols;
        let k_dim = a.cols;

        let mut data = vec![graph.integer(0); rows * cols];

        #[cfg(not(target_arch = "wasm32"))]
        data.par_chunks_mut(cols)
            .enumerate()
            .for_each(|(r, row_slice)| {
                for (c, item) in row_slice.iter_mut().enumerate() {
                    let mut terms = Vec::with_capacity(k_dim);
                    for k in 0..k_dim {
                        let elem_a = a.elements[r * k_dim + k];
                        let elem_b = b.elements[k * cols + c];
                        let prod = graph.mul([elem_a, elem_b]);
                        terms.push(prod);
                    }
                    *item = graph.add(terms);
                }
            });

        #[cfg(target_arch = "wasm32")]
        data.chunks_mut(cols)
            .enumerate()
            .for_each(|(r, row_slice)| {
                for (c, item) in row_slice.iter_mut().enumerate() {
                    let mut terms = Vec::with_capacity(k_dim);
                    for k in 0..k_dim {
                        let elem_a = a.elements[r * k_dim + k];
                        let elem_b = b.elements[k * cols + c];
                        let prod = graph.mul([elem_a, elem_b]);
                        terms.push(prod);
                    }
                    *item = graph.add(terms);
                }
            });

        SymbolicMatrix::new(rows, cols, data)
    }
}
