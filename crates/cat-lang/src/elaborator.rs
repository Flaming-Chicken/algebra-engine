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

    /// Lower a [`CatExpr`] directly into a `cat-core` `CatTerm` inside `CatRuntimeContext`
    /// using `cat-bridge` (`Lift ⊣ Lower`) after dimensional elaboration.
    pub fn lower_to_cat_term(
        &self,
        expr: &CatExpr,
        ctx: &mut cat_core::CatRuntimeContext,
    ) -> Result<cat_core::CatTerm, ElaboratorError> {
        let _dim = self.infer_dimension(expr)?;
        let legacy = Self::to_legacy_ast(expr);
        let mut bridge = cat_bridge::LegacyBridgeContext::default();
        use cat_bridge::RetrofittableAst;
        legacy
            .lift_to_cat(ctx, &mut bridge)
            .map_err(|_| ElaboratorError::UnknownVariable("cat_bridge_lift".to_string()))
    }

    fn to_legacy_ast(expr: &CatExpr) -> cat_bridge::LegacyAstTree {
        match expr {
            CatExpr::Number(v) => {
                cat_bridge::LegacyAstTree::Number(algebra_core::Number::from_f64_lossless(*v))
            }
            CatExpr::Complex { re, .. } => {
                cat_bridge::LegacyAstTree::Number(algebra_core::Number::from_f64_lossless(*re))
            }
            CatExpr::Variable(name) => cat_bridge::LegacyAstTree::Symbol(name.clone()),
            CatExpr::BinaryOp { op, lhs, rhs } => {
                let l = Self::to_legacy_ast(lhs);
                let r = Self::to_legacy_ast(rhs);
                match op.as_str() {
                    "+" => cat_bridge::LegacyAstTree::Add(vec![l, r]),
                    "*" => cat_bridge::LegacyAstTree::Mul(vec![l, r]),
                    "-" => cat_bridge::LegacyAstTree::Add(vec![
                        l,
                        cat_bridge::LegacyAstTree::Neg(Box::new(r)),
                    ]),
                    "/" => cat_bridge::LegacyAstTree::Div(Box::new(l), Box::new(r)),
                    "^" => cat_bridge::LegacyAstTree::Pow(Box::new(l), Box::new(r)),
                    other => cat_bridge::LegacyAstTree::Function {
                        name: other.to_string(),
                        args: vec![l, r],
                    },
                }
            }
            CatExpr::Call { func, args } => cat_bridge::LegacyAstTree::Function {
                name: func.clone(),
                args: args.iter().map(Self::to_legacy_ast).collect(),
            },
        }
    }
}
