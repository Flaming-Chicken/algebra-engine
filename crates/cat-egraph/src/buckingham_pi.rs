//! Buckingham-Pi Theorem dimensionless pre-reducer via integer nullspace reduction.
//!
//! Given n physical variables with 8D DimVectors, computes the r-dimensional basis
//! and synthesizes n - r independent dimensionless Pi-groups (\Pi_k).

use cat_core::dimensions::DimVector;
use smallvec::SmallVec;

/// Dimensionless Pi group resulting from Buckingham-Pi reduction:
/// \Pi_k = \prod_{i=1}^n x_i^{a_i}.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PiGroup {
    /// Powers a_i for each input variable.
    pub powers: SmallVec<[i32; 8]>,
}

/// Buckingham-Pi reduction engine.
pub struct BuckinghamPi;

impl BuckinghamPi {
    /// Compute independent dimensionless Pi-groups for a set of physical variables.
    ///
    /// Dimension matrix M has 8 rows (dimensions) and n columns (variables).
    /// Finds integer nullspace vectors v \in \mathbb{Z}^n such that M v = 0.
    pub fn find_dimensionless_groups(variables: &[DimVector]) -> Vec<PiGroup> {
        let n = variables.len();
        if n == 0 {
            return Vec::new();
        }

        // Construct 8 x n matrix of scaled twelfths
        let mut mat = vec![vec![0i64; n]; 8];
        for (col, v) in variables.iter().enumerate() {
            for row in 0..8 {
                mat[row][col] = v.scaled_twelfths[row] as i64;
            }
        }

        // Gaussian elimination over rationals to find nullspace
        let mut pivot_cols = Vec::new();
        let mut pivot_row = 0;

        for col in 0..n {
            if pivot_row >= 8 {
                break;
            }
            // Find pivot
            let mut found = None;
            for r in pivot_row..8 {
                if mat[r][col] != 0 {
                    found = Some(r);
                    break;
                }
            }

            if let Some(r) = found {
                mat.swap(pivot_row, r);
                pivot_cols.push(col);

                let pivot_val = mat[pivot_row][col];
                for r in 0..8 {
                    if r != pivot_row && mat[r][col] != 0 {
                        let factor = mat[r][col];
                        for c in 0..n {
                            mat[r][c] = mat[r][c] * pivot_val - mat[pivot_row][c] * factor;
                        }
                    }
                }
                pivot_row += 1;
            }
        }

        let _rank = pivot_cols.len();
        let mut pi_groups = Vec::new();

        // For each free column (not a pivot), construct nullspace vector
        for free_col in 0..n {
            if !pivot_cols.contains(&free_col) {
                let mut powers = smallvec::smallvec![0i32; n];
                powers[free_col] = 1;

                // Back-substitute to find integer coefficients for pivot columns
                for (i, &p_col) in pivot_cols.iter().enumerate() {
                    let pivot_val = mat[i][p_col];
                    let free_coeff = mat[i][free_col];
                    if pivot_val != 0 {
                        powers[p_col] = (-free_coeff / pivot_val) as i32;
                    }
                }

                // Verify resulting dimension vector is indeed dimensionless
                let mut check = DimVector::DIMENSIONLESS;
                for (i, &p) in powers.iter().enumerate() {
                    let scaled = variables[i].pow_rational(p, 1).unwrap_or(DimVector::DIMENSIONLESS);
                    check = check + scaled;
                }

                if check.is_dimensionless() {
                    pi_groups.push(PiGroup { powers });
                }
            }
        }

        pi_groups
    }
}
