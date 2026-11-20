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
    #[error("The exponent of a power of the quantity {0} must be a rational number literal")]
    NonConstantExponent(DimVector),
    #[error("The argument of {func} must be dimensionless, but has dimension {dim}")]
    DimensionfulArgument { func: String, dim: DimVector },
    #[error("The result of {0} has a dimension that is not representable")]
    UnrepresentableDimension(String),
}

/// A rational with a small denominator close to `x` (an exponent written as a decimal literal), if there is one.
fn small_rational(x: f64) -> Option<(i32, i32)> {
    if !x.is_finite() {
        return None;
    }
    (1..=12).find_map(|den| {
        let num = x * den as f64;
        ((num - num.round()).abs() < 1e-9 && num.abs() < 1e6).then_some((num.round() as i32, den))
    })
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
                    "^" => {
                        if d_lhs.is_dimensionless() {
                            if !d_rhs.is_dimensionless() {
                                return Err(ElaboratorError::DimensionfulArgument {
                                    func: "^".into(),
                                    dim: d_rhs,
                                });
                            }
                            return Ok(d_lhs);
                        }
                        // A power of a quantity with units: the exponent has to be a known rational, and
                        // every dimension exponent is multiplied by it.
                        let exponent = match &**rhs {
                            CatExpr::Number(v) => Some(*v),
                            CatExpr::BinaryOp { op, lhs, rhs } if op == "/" => {
                                match (&**lhs, &**rhs) {
                                    (CatExpr::Number(a), CatExpr::Number(b)) if *b != 0.0 => {
                                        Some(a / b)
                                    }
                                    _ => None,
                                }
                            }
                            _ => None,
                        };
                        let (num, den) = exponent
                            .and_then(small_rational)
                            .ok_or(ElaboratorError::NonConstantExponent(d_lhs))?;
                        d_lhs.pow_rational(num, den).map_err(|_| {
                            ElaboratorError::UnrepresentableDimension(format!(
                                "{d_lhs}^({num}/{den})"
                            ))
                        })
                    }
                    // Any other operator is taken to combine pure numbers.
                    other if d_lhs.is_dimensionless() && d_rhs.is_dimensionless() => {
                        let _ = other;
                        Ok(DimVector::DIMENSIONLESS)
                    }
                    other => Err(ElaboratorError::DimensionfulArgument {
                        func: other.to_string(),
                        dim: if d_lhs.is_dimensionless() {
                            d_rhs
                        } else {
                            d_lhs
                        },
                    }),
                }
            }
            CatExpr::Call { func, args } => {
                let dims: Vec<DimVector> = args
                    .iter()
                    .map(|a| self.infer_dimension(a))
                    .collect::<Result<_, _>>()?;
                match func.as_str() {
                    "sqrt" if dims.len() == 1 => dims[0].sqrt().map_err(|_| {
                        ElaboratorError::UnrepresentableDimension(format!("sqrt({})", dims[0]))
                    }),
                    "cbrt" if dims.len() == 1 => dims[0].pow_rational(1, 3).map_err(|_| {
                        ElaboratorError::UnrepresentableDimension(format!("cbrt({})", dims[0]))
                    }),
                    // These keep the dimension of their arguments, which must agree.
                    "abs" | "neg" | "floor" | "ceil" | "round" | "min" | "max" | "re" | "im"
                    | "conj" => {
                        let first = dims.first().copied().unwrap_or(DimVector::DIMENSIONLESS);
                        match dims.iter().find(|d| **d != first) {
                            Some(&other) => {
                                Err(ElaboratorError::DimensionAddMismatch(first, other))
                            }
                            None => Ok(first),
                        }
                    }
                    // Transcendental (sin, exp, ln, ...) and unknown functions need pure numbers.
                    name => match dims.iter().find(|d| !d.is_dimensionless()) {
                        Some(&dim) => Err(ElaboratorError::DimensionfulArgument {
                            func: name.to_string(),
                            dim,
                        }),
                        None => Ok(DimVector::DIMENSIONLESS),
                    },
                }
            }
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
            CatExpr::Complex { re, im } => {
                let real =
                    cat_bridge::LegacyAstTree::Number(algebra_core::Number::from_f64_lossless(*re));
                if *im == 0.0 {
                    real
                } else {
                    // re + im * i: the imaginary part is kept, not dropped.
                    let imaginary = cat_bridge::LegacyAstTree::Mul(vec![
                        cat_bridge::LegacyAstTree::Number(algebra_core::Number::from_f64_lossless(
                            *im,
                        )),
                        cat_bridge::LegacyAstTree::Symbol("i".into()),
                    ]);
                    if *re == 0.0 {
                        imaginary
                    } else {
                        cat_bridge::LegacyAstTree::Add(vec![real, imaginary])
                    }
                }
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
