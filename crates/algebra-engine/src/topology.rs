//! # `algebra_engine::topology`
//!
//! Algebraic Topology & Simplicial Homology:
//! - Simplicial Complexes
//! - Boundary Operators $\partial_k: C_k \to C_{k-1}$
//! - Betti Numbers $\beta_k = \dim H_k(X)$
//! - Euler Characteristic $\chi = \sum (-1)^k \beta_k$

use std::collections::BTreeSet;

/// Abstract Simplicial Complex representation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SimplicialComplex {
    /// Dimension-indexed faces (0-simplices, 1-simplices, 2-simplices, ...).
    pub simplices: Vec<BTreeSet<BTreeSet<usize>>>,
}

impl SimplicialComplex {
    pub fn new() -> Self {
        Self {
            simplices: Vec::new(),
        }
    }

    /// Add a simplex defined by vertex indices (e.g. {0, 1, 2} for a triangle).
    pub fn add_simplex(&mut self, vertices: &[usize]) {
        // A simplex is a set: repeated vertices are one vertex, and the dimension is the size of the set minus one.
        let mut unique: Vec<usize> = vertices.to_vec();
        unique.sort_unstable();
        unique.dedup();
        let vertices = &unique[..];
        if vertices.is_empty() {
            return; // the empty simplex is not stored
        }
        let dim = vertices.len() - 1;
        while self.simplices.len() <= dim {
            self.simplices.push(BTreeSet::new());
        }
        let set: BTreeSet<usize> = vertices.iter().copied().collect();
        self.simplices[dim].insert(set);

        // Ensure all subfaces are present (simplicial closure)
        if dim > 0 {
            for i in 0..vertices.len() {
                let subface: Vec<usize> = vertices
                    .iter()
                    .enumerate()
                    .filter(|&(idx, _)| idx != i)
                    .map(|(_, &v)| v)
                    .collect();
                self.add_simplex(&subface);
            }
        }
    }

    /// Number of $k$-dimensional simplices.
    pub fn count_simplices(&self, dim: usize) -> usize {
        if dim < self.simplices.len() {
            self.simplices[dim].len()
        } else {
            0
        }
    }

    /// Euler characteristic $\chi = \sum_{k=0}^n (-1)^k f_k$.
    pub fn euler_characteristic(&self) -> i64 {
        let mut chi = 0i64;
        for (dim, set) in self.simplices.iter().enumerate() {
            let count = set.len() as i64;
            if dim % 2 == 0 {
                chi += count;
            } else {
                chi -= count;
            }
        }
        chi
    }

    /// The simplices of dimension `k` in a fixed order (each as its sorted vertex list).
    fn ordered(&self, k: usize) -> Vec<Vec<usize>> {
        self.simplices
            .get(k)
            .map(|set| set.iter().map(|s| s.iter().copied().collect()).collect())
            .unwrap_or_default()
    }

    /// The boundary matrix $\partial_k : C_k \to C_{k-1}$ with rows the $(k-1)$-simplices and columns the
    /// $k$-simplices, in the order of the sorted vertex lists. An oriented simplex $[v_0, \dots, v_k]$ (vertices
    /// ascending) has boundary $\sum_i (-1)^i [v_0, \dots, \hat{v_i}, \dots, v_k]$. Empty for `k = 0`.
    pub fn boundary_matrix(&self, k: usize) -> Vec<Vec<i64>> {
        if k == 0 {
            return Vec::new();
        }
        let faces = self.ordered(k - 1);
        let cells = self.ordered(k);
        let mut m = vec![vec![0i64; cells.len()]; faces.len()];
        for (col, cell) in cells.iter().enumerate() {
            for i in 0..cell.len() {
                let mut face = cell.clone();
                face.remove(i);
                if let Ok(row) = faces.binary_search(&face) {
                    m[row][col] = if i % 2 == 0 { 1 } else { -1 };
                }
            }
        }
        m
    }

    /// Rank over the rationals of an integer matrix (exact fraction arithmetic).
    fn rank_over_q(matrix: &[Vec<i64>]) -> usize {
        use num_bigint::BigInt;
        use num_rational::BigRational;
        use num_traits::Zero;
        if matrix.is_empty() || matrix[0].is_empty() {
            return 0;
        }
        let (rows, cols) = (matrix.len(), matrix[0].len());
        let mut a: Vec<Vec<BigRational>> = matrix
            .iter()
            .map(|r| {
                r.iter()
                    .map(|&v| BigRational::from_integer(BigInt::from(v)))
                    .collect()
            })
            .collect();
        let mut rank = 0;
        for col in 0..cols {
            if rank == rows {
                break;
            }
            let Some(p) = (rank..rows).find(|&r| !a[r][col].is_zero()) else {
                continue;
            };
            a.swap(rank, p);
            let pivot = a[rank][col].clone();
            for r in (rank + 1)..rows {
                if a[r][col].is_zero() {
                    continue;
                }
                let factor = &a[r][col] / &pivot;
                for c in col..cols {
                    let delta = &factor * &a[rank][c];
                    a[r][c] -= delta;
                }
            }
            rank += 1;
        }
        rank
    }

    /// Betti numbers $\beta_k = \dim H_k(X; \mathbb{Q})$, one per dimension of the complex:
    /// $\beta_k = f_k - \operatorname{rank}\partial_k - \operatorname{rank}\partial_{k+1}$, with the ranks of the
    /// boundary matrices computed exactly over the rationals. Torsion (the $\mathbb{Z}/2$ in the first homology of the
    /// projective plane) is invisible over $\mathbb{Q}$ and does not count. A circle gives `[1, 1]`; the Euler
    /// characteristic equals $\sum_k (-1)^k \beta_k$.
    pub fn betti_numbers(&self) -> Vec<usize> {
        let dims = self.simplices.len();
        let ranks: Vec<usize> = (0..=dims)
            .map(|k| {
                if k == 0 || k >= dims {
                    0
                } else {
                    Self::rank_over_q(&self.boundary_matrix(k))
                }
            })
            .collect();
        (0..dims)
            .map(|k| self.count_simplices(k) - ranks[k] - ranks[k + 1])
            .collect()
    }
}
