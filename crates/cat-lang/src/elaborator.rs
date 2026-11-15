//! Hindley-Milner + Colored Operad elaborator for CatScript and CatPad.

use crate::ast::CatExpr;
use cat_core::dimensions::DimVector;
use std::collections::HashMap;
use thiserror::Error;

/// Type inference and elaboration errors.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ElaboratorError {
    #[error("Dimensional mismatch: cannot add {0} and {1}")]
    DimensionAddMismatch(DimVector, DimVector),
    #[error("Unknown variable: {0}")]
    UnknownVariable(String),
}

/// Elaborator inferring dimensions and sorts across expressions.
#[derive(Default)]
pub struct Elaborator {
    pub env: HashMap<String, DimVector>,
}

impl Elaborator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a typed variable in the elaboration context.
    pub fn bind(&mut self, name: &str, dim: DimVector) {
        self.env.insert(name.to_string(), dim);
    }

    /// Infer physical dimension vector of an expression.
    pub fn infer_dimension(&self, expr: &CatExpr) -> Result<DimVector, ElaboratorError> {
        match expr {
            CatExpr::Number(_) | CatExpr::Complex { .. } => Ok(DimVector::DIMENSIONLESS),
            CatExpr::Variable(name) => self
                .env
                .get(name)
                .cloned()
                .ok_or_else(|| ElaboratorError::UnknownVariable(name.clone())),
            CatExpr::BinaryOp { op, lhs, rhs } => {
                let d_lhs = self.infer_dimension(lhs)?;
                let d_rhs = self.infer_dimension(rhs)?;
                match op.as_str() {
                    "+" | "-" => {
                        if d_lhs != d_rhs {
                            return Err(ElaboratorError::DimensionAddMismatch(d_lhs, d_rhs));
                        }
                        Ok(d_lhs)
                    }
                    "*" => Ok(d_lhs * d_rhs),
                    "/" => Ok(d_lhs / d_rhs),
                    _ => Ok(DimVector::DIMENSIONLESS),
                }
            }
            CatExpr::Call { .. } => Ok(DimVector::DIMENSIONLESS),
        }
    }
}
