//! Categorical sorts and Layer 0 object descriptors.
//!
//! Object descriptors specify the mathematical and dimensional typing of
//! vertices, wires, and boundary ports in the categorical computation engine.

use crate::dimensions::{DimVector, UnitSpaceKind};
use crate::handles::ObjId;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

/// Structural descriptor for categorical objects (Layer 0).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObjectDescriptor {
    /// 1-dimensional scalar sort with physical dimension and space geometry.
    Scalar(DimVector, UnitSpaceKind),

    /// Vector space of given rank with a shared physical dimension.
    Vector { dim: DimVector, rank: u32 },

    /// Matrix space with explicit row and column dimensions.
    /// Supports dimensional grading across rows and columns.
    Matrix {
        rows: u32,
        cols: u32,
        row_dims: SmallVec<[DimVector; 4]>,
        col_dims: SmallVec<[DimVector; 4]>,
    },

    /// Differential geometric fiber bundle E \to B.
    FiberBundle { base: ObjId, fiber: ObjId },

    /// Morphism hom-space Hom(Source, Target) in a category.
    HomSpace { source: ObjId, target: ObjId },

    /// Graded direct sum / biproduct space: \bigoplus_i (V_i, d_i).
    GradedBiproductSlot {
        slots: SmallVec<[(ObjId, DimVector); 4]>,
    },
}

impl Default for ObjectDescriptor {
    fn default() -> Self {
        Self::Scalar(DimVector::DIMENSIONLESS, UnitSpaceKind::LinearVectorSpace)
    }
}

impl ObjectDescriptor {
    /// Standard dimensionless scalar descriptor.
    pub const DIMENSIONLESS_SCALAR: Self =
        Self::Scalar(DimVector::DIMENSIONLESS, UnitSpaceKind::LinearVectorSpace);

    /// Create a standard linear scalar with physical dimension.
    pub fn linear_scalar(dim: DimVector) -> Self {
        Self::Scalar(dim, UnitSpaceKind::LinearVectorSpace)
    }

    /// Create an affine scalar with physical dimension (e.g. Celsius or epoch time).
    pub fn affine_scalar(dim: DimVector) -> Self {
        Self::Scalar(dim, UnitSpaceKind::AffineSpace)
    }

    /// Create a vector space descriptor.
    pub fn vector(dim: DimVector, rank: u32) -> Self {
        Self::Vector { dim, rank }
    }

    /// Create a homogeneous matrix descriptor where all entries share dimension `dim`.
    pub fn homogeneous_matrix(rows: u32, cols: u32, dim: DimVector) -> Self {
        let row_dims = smallvec::smallvec![dim; rows as usize];
        let col_dims = smallvec::smallvec![DimVector::DIMENSIONLESS; cols as usize];
        Self::Matrix {
            rows,
            cols,
            row_dims,
            col_dims,
        }
    }

    /// Create a graded matrix descriptor for mixed-unit transformations (e.g. \mathfrak{se}(3) twist matrices).
    pub fn graded_matrix(
        rows: u32,
        cols: u32,
        row_dims: SmallVec<[DimVector; 4]>,
        col_dims: SmallVec<[DimVector; 4]>,
    ) -> Self {
        Self::Matrix {
            rows,
            cols,
            row_dims,
            col_dims,
        }
    }

    /// Returns `true` if this object represents a scalar.
    pub fn is_scalar(&self) -> bool {
        matches!(self, Self::Scalar(_, _))
    }

    /// Returns the primary dimension vector associated with this object, if any.
    pub fn dimension(&self) -> Option<DimVector> {
        match self {
            Self::Scalar(dim, _) => Some(*dim),
            Self::Vector { dim, .. } => Some(*dim),
            Self::Matrix {
                row_dims, col_dims, ..
            } => {
                if !row_dims.is_empty() && !col_dims.is_empty() {
                    Some(row_dims[0] - col_dims[0])
                } else {
                    Some(DimVector::DIMENSIONLESS)
                }
            }
            _ => None,
        }
    }

    /// Invariant Check: Verify whether matrix exponentiation \exp(M) is mathematically valid.
    ///
    /// For a graded matrix, \exp(M) = \sum_{k=0}^\infty M^k / k! is dimensional-consistent
    /// if and only if each entry [M_{ij}] has dimension \mathbf{d}_i - \mathbf{d}_j,
    /// so that (M^k)_{ij} retains the exact same dimension for all powers k \ge 1.
    pub fn check_matrix_exp_validity(&self) -> Result<(), &'static str> {
        match self {
            Self::Matrix {
                rows,
                cols,
                row_dims,
                col_dims,
            } => {
                if rows != cols {
                    return Err("Matrix exponentiation requires a square matrix (rows == cols)");
                }
                let n = *rows as usize;
                if row_dims.len() < n || col_dims.len() < n {
                    return Err("Incomplete row or column dimension grading vectors");
                }
                // Verify entry dimensions [M_{ij}] == row_dims[i] - col_dims[j]
                // For powers M^k to be dimensional-consistent, row grading must align with col grading
                for i in 0..n {
                    if row_dims[i] != col_dims[i] {
                        return Err(
                            "Mixed-unit matrix exp failed: row basis dimension does not match column basis dimension",
                        );
                    }
                }
                Ok(())
            }
            Self::Scalar(dim, space) => {
                if !dim.is_dimensionless() {
                    return Err("Scalar exponentiation exp(x) requires dimensionless argument");
                }
                if *space == UnitSpaceKind::AffineSpace {
                    return Err(
                        "Scalar exponentiation cannot be applied to an AffineSpace coordinate",
                    );
                }
                Ok(())
            }
            _ => Err("Exponentiation is only supported on Scalars and Square Matrices"),
        }
    }
}
