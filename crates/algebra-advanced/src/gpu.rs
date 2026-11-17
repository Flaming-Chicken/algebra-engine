//! # `algebra-gpu`
//!
//! Multi-threaded (Rayon) polynomial and dense-matrix kernels. Despite the type names, nothing
//! here uses SIMD intrinsics or a GPU: the work is spread across CPU threads, and
//! [`GpuComputeKernel::enabled`] chooses between that parallel path and a sequential one. The
//! WGSL text generators live in `algebra_engine::gpu`.
//!
//! Integer polynomial arithmetic wraps on overflow (arithmetic modulo 2^64); use the `checked_`
//! methods when overflow must be an error.

use spodeian_parallel::prelude::*;

use algebra_core::{AlgebraError, AlgebraResult, ExprGraph};
use algebra_engine::matrix::SymbolicMatrix;

/// Integer polynomial block operations, parallelised across threads (no SIMD intrinsics).
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

impl SimdPolyVector {
    /// Coefficient-wise sum, or `None` if any coefficient overflows `i64`.
    pub fn checked_add(&self, other: &Self) -> Option<Self> {
        let len = self.coeffs.len().max(other.coeffs.len());
        (0..len)
            .map(|i| {
                self.coeffs
                    .get(i)
                    .copied()
                    .unwrap_or(0)
                    .checked_add(other.coeffs.get(i).copied().unwrap_or(0))
            })
            .collect::<Option<Vec<i64>>>()
            .map(|coeffs| Self { coeffs })
    }

    /// Polynomial product, or `None` if any intermediate product or sum overflows `i64`.
    pub fn checked_multiply(&self, other: &Self) -> Option<Self> {
        if self.coeffs.is_empty() || other.coeffs.is_empty() {
            return Some(Self { coeffs: Vec::new() });
        }
        let mut result = vec![0i64; self.coeffs.len() + other.coeffs.len() - 1];
        for (i, a) in self.coeffs.iter().enumerate() {
            for (j, b) in other.coeffs.iter().enumerate() {
                result[i + j] = result[i + j].checked_add(a.checked_mul(*b)?)?;
            }
        }
        Some(Self { coeffs: result })
    }
}

/// Dense-matrix kernel with a switchable parallel path (CPU threads; it does not use a GPU).
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

        // `enabled` selects the multi-threaded path; otherwise (and always on wasm) run
        // sequentially.
        #[cfg(not(target_arch = "wasm32"))]
        if self.enabled {
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
        } else {
            data.chunks_mut(cols)
                .enumerate()
                .for_each(|(r, row_slice)| {
                    for (c, item) in row_slice.iter_mut().enumerate() {
                        let mut terms = Vec::with_capacity(k_dim);
                        for k in 0..k_dim {
                            let elem_a = a.elements[r * k_dim + k];
                            let elem_b = b.elements[k * cols + c];
                            terms.push(graph.mul([elem_a, elem_b]));
                        }
                        *item = graph.add(terms);
                    }
                });
        }

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
