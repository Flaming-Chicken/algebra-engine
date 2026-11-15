//! CatScript (.cat) and CatPad (.pad) AST definitions.

use cat_core::dimensions::DimVector;
use serde::{Deserialize, Serialize};

/// Type annotation in CatScript / CatPad.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CatTypeAnnotation {
    Dimensionless,
    Dimension(DimVector),
    Named(String),
}

/// Abstract expression in CatScript.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CatExpr {
    Number(f64),
    Complex {
        re: f64,
        im: f64,
    },
    Variable(String),
    BinaryOp {
        op: String,
        lhs: Box<CatExpr>,
        rhs: Box<CatExpr>,
    },
    Call {
        func: String,
        args: Vec<CatExpr>,
    },
}

/// Statement or declaration in CatScript (.cat).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CatStatement {
    Let {
        name: String,
        type_ann: Option<CatTypeAnnotation>,
        value: CatExpr,
    },
    Expr(CatExpr),
    FunctionDef {
        name: String,
        params: Vec<(String, CatTypeAnnotation)>,
        return_type: CatTypeAnnotation,
        body: Vec<CatStatement>,
    },
}

/// Program script in CatScript (.cat).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CatScriptAst {
    pub statements: Vec<CatStatement>,
}

/// Interactive notebook cell in CatPad (.pad).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PadCell {
    Markdown(String),
    Code {
        source: String,
        expr: CatExpr,
        last_estimate: Option<String>,
    },
    Slider {
        name: String,
        min: f64,
        max: f64,
        value: f64,
    },
}

/// Interactive Notebook document in CatPad (.pad).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CatPadNotebook {
    pub cells: Vec<PadCell>,
}

impl CatPadNotebook {
    /// 1-Click promotion from interactive CatPad notebook (.pad) to standalone CatScript (.cat).
    pub fn promote_to_script(&self) -> CatScriptAst {
        let mut statements = Vec::new();
        for cell in &self.cells {
            match cell {
                PadCell::Code { expr, .. } => {
                    statements.push(CatStatement::Expr(expr.clone()));
                }
                PadCell::Slider { name, value, .. } => {
                    statements.push(CatStatement::Let {
                        name: name.clone(),
                        type_ann: Some(CatTypeAnnotation::Dimensionless),
                        value: CatExpr::Number(*value),
                    });
                }
                PadCell::Markdown(_) => {
                    // Comments or documentation in promoted script
                }
            }
        }
        CatScriptAst { statements }
    }
}
