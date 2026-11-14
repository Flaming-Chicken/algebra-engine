//! Generalized Tensor Algebra Engine competing with SymPy tensor modules.
//!
//! Supports mixed-valence tensors (r, s), Einstein summation contractions,
//! tensor products, symmetrizations, and arbitrary dimension spaces.

use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

/// Index variance in tensor algebra.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IndexVariance {
    /// Contravariant (upper index, transforms with Jacobian).
    Contravariant,
    /// Covariant (lower index, transforms with inverse Jacobian).
    Covariant,
}

/// Generalized Tensor of arbitrary rank and valence (r contravariant, s covariant).
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Tensor {
    /// Dimension D of the underlying vector space.
    pub dim: usize,
    /// Valence: (contravariant_count r, covariant_count s).
    pub valence: (usize, usize),
    /// Variances for each slot.
    pub index_variances: SmallVec<[IndexVariance; 6]>,
    /// Dense flattened components array of length D^(r + s).
    pub components: Vec<f64>,
}

impl Tensor {
    /// Total rank of the tensor: r + s.
    pub fn rank(&self) -> usize {
        self.valence.0 + self.valence.1
    }

    /// Construct a zero tensor of given dimension and valence (r, s).
    pub fn zeros(dim: usize, contravariant: usize, covariant: usize) -> Self {
        let rank = contravariant + covariant;
        let total_size = dim.pow(rank as u32);
        let mut variances = SmallVec::new();
        for _ in 0..contravariant {
            variances.push(IndexVariance::Contravariant);
        }
        for _ in 0..covariant {
            variances.push(IndexVariance::Covariant);
        }
        Self {
            dim,
            valence: (contravariant, covariant),
            index_variances: variances,
            components: vec![0.0; total_size],
        }
    }

    /// Kronecker delta tensor \delta^\mu_\nu of valence (1, 1).
    pub fn kronecker_delta(dim: usize) -> Self {
        let mut t = Self::zeros(dim, 1, 1);
        for i in 0..dim {
            t.set(&[i, i], 1.0);
        }
        t
    }

    /// Flat index computation from multidimensional index coordinates.
    pub fn flat_index(&self, indices: &[usize]) -> usize {
        assert_eq!(indices.len(), self.rank(), "Index rank mismatch");
        let mut idx = 0;
        for &i in indices {
            assert!(i < self.dim, "Index out of bounds");
            idx = idx * self.dim + i;
        }
        idx
    }

    /// Retrieve component value at given indices.
    pub fn get(&self, indices: &[usize]) -> f64 {
        let idx = self.flat_index(indices);
        self.components[idx]
    }

    /// Set component value at given indices.
    pub fn set(&mut self, indices: &[usize], val: f64) {
        let idx = self.flat_index(indices);
        self.components[idx] = val;
    }

    /// Tensor Product: C = A \otimes B.
    /// Resulting valence is (r_A + r_B, s_A + s_B).
    pub fn tensor_product(&self, other: &Self) -> Self {
        assert_eq!(self.dim, other.dim, "Dimension mismatch in tensor product");
        let out_r = self.valence.0 + other.valence.0;
        let out_s = self.valence.1 + other.valence.1;
        let mut out = Self::zeros(self.dim, out_r, out_s);

        let size_a = self.components.len();
        let size_b = other.components.len();

        for i in 0..size_a {
            let ca = self.components[i];
            if ca == 0.0 {
                continue;
            }
            for j in 0..size_b {
                let cb = other.components[j];
                out.components[i * size_b + j] = ca * cb;
            }
        }
        out
    }

    /// Einstein Contraction: contract contravariant index `contra_idx` with covariant index `covar_idx`.
    pub fn contract(&self, contra_idx: usize, covar_idx: usize) -> Self {
        let rank = self.rank();
        assert!(rank >= 2, "Cannot contract tensor of rank < 2");
        assert!(contra_idx < self.valence.0, "contra_idx must be in contravariant slots");
        assert!(
            covar_idx >= self.valence.0 && covar_idx < rank,
            "covar_idx must be in covariant slots"
        );

        let out_r = self.valence.0 - 1;
        let out_s = self.valence.1 - 1;
        let mut out = Self::zeros(self.dim, out_r, out_s);
        let out_rank = out.rank();

        let mut coord = vec![0usize; rank];

        for (flat, &val) in self.components.iter().enumerate() {
            if val == 0.0 {
                continue;
            }
            // Deconstruct flat index into coordinates
            let mut rem = flat;
            for k in (0..rank).rev() {
                coord[k] = rem % self.dim;
                rem /= self.dim;
            }

            // Only contract when the two contracted slots match: coord[contra_idx] == coord[covar_idx]
            if coord[contra_idx] == coord[covar_idx] {
                // Form coordinates for contracted output tensor
                let mut out_coord = Vec::with_capacity(out_rank);
                for k in 0..rank {
                    if k != contra_idx && k != covar_idx {
                        out_coord.push(coord[k]);
                    }
                }
                let out_flat = out.flat_index(&out_coord);
                out.components[out_flat] += val;
            }
        }

        out
    }

    /// Antisymmetrize two indices: T_{[i j]} = 1/2 (T_{i j} - T_{j i}).
    pub fn antisymmetrize(&self, i: usize, j: usize) -> Self {
        let mut out = self.clone();
        let rank = self.rank();
        let mut coord = vec![0usize; rank];

        for flat in 0..self.components.len() {
            let mut rem = flat;
            for k in (0..rank).rev() {
                coord[k] = rem % self.dim;
                rem /= self.dim;
            }
            let mut swapped = coord.clone();
            swapped.swap(i, j);

            let v1 = self.get(&coord);
            let v2 = self.get(&swapped);
            out.set(&coord, 0.5 * (v1 - v2));
        }
        out
    }
}
