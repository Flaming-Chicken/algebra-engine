//! `cat-lang`: Dual languages (CatScript `.cat` and CatPad `.pad`),
//! Hindley-Milner + Colored Operad elaborator, and `.catb` binary persistence.

pub mod ast;
pub mod binary;
pub mod elaborator;

pub use ast::{CatExpr, CatPadNotebook, CatScriptAst, CatStatement, CatTypeAnnotation, PadCell};
pub use binary::CatBinary;
pub use elaborator::{Elaborator, ElaboratorError};
