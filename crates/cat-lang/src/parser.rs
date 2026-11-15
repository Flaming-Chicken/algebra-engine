//! Unified Polyglot Parser for CatScript (`.cat`) and CatPad (`.pad`),
//! backed by `algebra_core::ExprParser` so URAE math/LaTeX/MATLAB/Desmos syntax
//! and CatScript share a single canonical frontend.

use crate::ast::{CatExpr, CatScriptAst, CatStatement};
use algebra_core::{ExprGraph, ExprId, ExprKind, ExprParser, Number};
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CatParseError {
    #[error("syntax error: {0}")]
    Syntax(String),
}

/// Unified CatScript / CatPad parser delegating expression parsing to `algebra_core::ExprParser`.
pub struct CatScriptParser;

impl CatScriptParser {
    /// Parse a single math/CatScript expression into [`CatExpr`].
    pub fn parse_expr(input: &str) -> Result<CatExpr, CatParseError> {
        let graph = ExprGraph::new();
        let parser = ExprParser::new(&graph);
        let id = parser
            .parse(input.trim())
            .map_err(|e| CatParseError::Syntax(e.to_string()))?;
        Ok(Self::expr_id_to_cat(&graph, id))
    }

    /// Parse a multi-statement CatScript program (`let name = expr;`, bare expressions).
    pub fn parse_script(input: &str) -> Result<CatScriptAst, CatParseError> {
        let mut statements = Vec::new();
        for raw_line in input.lines().flat_map(|l| l.split(';')) {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with("//") || line.starts_with('#') {
                continue;
            }
            if let Some(rest) = line.strip_prefix("let ") {
                let (lhs, rhs) = rest.split_once('=').ok_or_else(|| {
                    CatParseError::Syntax(format!("expected '=' in let binding: {line}"))
                })?;
                let name = lhs.split(':').next().unwrap_or(lhs).trim().to_string();
                let value = Self::parse_expr(rhs)?;
                statements.push(CatStatement::Let {
                    name,
                    type_ann: None,
                    value,
                });
            } else {
                let expr = Self::parse_expr(line)?;
                statements.push(CatStatement::Expr(expr));
            }
        }
        Ok(CatScriptAst { statements })
    }

    fn expr_id_to_cat(graph: &ExprGraph, id: ExprId) -> CatExpr {
        match &graph.get(id).kind {
            ExprKind::Number(n) => match n {
                Number::Integer(i) => CatExpr::Number(*i as f64),
                Number::Rational(num, den) => CatExpr::Number(*num as f64 / *den as f64),
                Number::Float(bits) => CatExpr::Number(f64::from_bits(*bits)),
                _ => CatExpr::Number(0.0),
            },
            ExprKind::Symbol(sym) => {
                CatExpr::Variable(graph.symbols.resolve(*sym).unwrap_or_default())
            }
            ExprKind::Neg(inner) => CatExpr::BinaryOp {
                op: "-".to_string(),
                lhs: Box::new(CatExpr::Number(0.0)),
                rhs: Box::new(Self::expr_id_to_cat(graph, *inner)),
            },
            ExprKind::Add(terms) => Self::fold_nary(graph, "+", terms),
            ExprKind::Mul(factors) => Self::fold_nary(graph, "*", factors),
            ExprKind::Div(a, b) => CatExpr::BinaryOp {
                op: "/".to_string(),
                lhs: Box::new(Self::expr_id_to_cat(graph, *a)),
                rhs: Box::new(Self::expr_id_to_cat(graph, *b)),
            },
            ExprKind::Pow(a, b) => CatExpr::BinaryOp {
                op: "^".to_string(),
                lhs: Box::new(Self::expr_id_to_cat(graph, *a)),
                rhs: Box::new(Self::expr_id_to_cat(graph, *b)),
            },
            ExprKind::Function { name, args } => CatExpr::Call {
                func: graph.symbols.resolve(*name).unwrap_or_default(),
                args: args.iter().map(|&a| Self::expr_id_to_cat(graph, a)).collect(),
            },
            _ => CatExpr::Call {
                func: "opaque".to_string(),
                args: Vec::new(),
            },
        }
    }

    fn fold_nary(graph: &ExprGraph, op: &str, items: &[ExprId]) -> CatExpr {
        let mut iter = items.iter().map(|&id| Self::expr_id_to_cat(graph, id));
        let Some(first) = iter.next() else {
            return CatExpr::Number(if op == "*" { 1.0 } else { 0.0 });
        };
        iter.fold(first, |acc, rhs| CatExpr::BinaryOp {
            op: op.to_string(),
            lhs: Box::new(acc),
            rhs: Box::new(rhs),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CatBinary, Elaborator};

    #[test]
    fn test_unified_catscript_parse_elaborate_lower_and_binary() {
        let ast = CatScriptParser::parse_script("let y = x^2 + 2*x + 1;\nsin(y)").expect("parse");
        assert_eq!(ast.statements.len(), 2);

        let mut elab = Elaborator::new();
        elab.bind("x", cat_core::dimensions::DimVector::DIMENSIONLESS);
        let mut ctx = cat_core::CatRuntimeContext::default();
        if let CatStatement::Let { value, .. } = &ast.statements[0] {
            let _term = elab.lower_to_cat_term(value, &mut ctx).expect("lower");
        } else {
            panic!("expected Let statement");
        }

        let bytes = CatBinary::save_script(&ast).expect("save .catb");
        assert!(bytes.starts_with(b"CATB\x01\x00"));
        let roundtrip = CatBinary::load_script(&bytes).expect("load .catb");
        assert_eq!(roundtrip, ast);
    }
}
