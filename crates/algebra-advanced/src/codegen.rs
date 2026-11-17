//! # `algebra-codegen`
//!
//! AST-to-Rust/C code generation and JIT numerical closure compilation.

use algebra_core::vm::{BytecodeVM, VmCompiler};
use algebra_core::{AlgebraError, AlgebraResult, ExprGraph, ExprId, ExprKind, SymbolId};
use algebra_engine::numeric::{EvalContext, NumericalEval};

/// Code compilation target format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetLanguage {
    Rust,
    C,
}

/// Compiler turning symbolic expression graphs into executable closures or code strings.
pub struct CodeGenerator;

impl CodeGenerator {
    /// Emit source code string in target language for expression `expr`.
    pub fn emit_code(
        graph: &ExprGraph,
        expr: ExprId,
        target: TargetLanguage,
    ) -> AlgebraResult<String> {
        let node = graph.get(expr);
        match &node.kind {
            ExprKind::Number(n) => match n {
                algebra_core::Number::Integer(i) => Ok(format!("{i}.0")),
                algebra_core::Number::BigInteger(b) => Ok(format!("{b}.0")),
                algebra_core::Number::Float(bits) => Ok(format!("{}", f64::from_bits(*bits))),
                algebra_core::Number::Rational(num, den) => Ok(format!("({num}.0 / {den}.0)")),
                algebra_core::Number::BigRational(r) => {
                    Ok(format!("({}.0 / {}.0)", r.numer(), r.denom()))
                }
                algebra_core::Number::Scientific { mantissa, exponent } => {
                    Ok(format!("({mantissa}.0 * 10.0f64.powi({exponent}))"))
                }
                algebra_core::Number::Constant(c) => match c {
                    algebra_core::Constant::Pi => Ok("std::f64::consts::PI".into()),
                    algebra_core::Constant::E => Ok("std::f64::consts::E".into()),
                    algebra_core::Constant::I => Ok("1.0".into()),
                    _ => Ok("0.0".into()),
                },
            },
            ExprKind::Symbol(s) => Ok(graph.symbols.resolve(*s).unwrap_or_default()),
            ExprKind::Add(terms) => {
                let code_terms: Vec<String> = terms
                    .iter()
                    .map(|&t| Self::emit_code(graph, t, target))
                    .collect::<AlgebraResult<_>>()?;
                Ok(format!("({})", code_terms.join(" + ")))
            }
            ExprKind::Sub(lhs, rhs) => {
                let cl = Self::emit_code(graph, *lhs, target)?;
                let cr = Self::emit_code(graph, *rhs, target)?;
                Ok(format!("({cl} - {cr})"))
            }
            ExprKind::Mul(factors) => {
                let code_factors: Vec<String> = factors
                    .iter()
                    .map(|&f| Self::emit_code(graph, f, target))
                    .collect::<AlgebraResult<_>>()?;
                Ok(format!("({})", code_factors.join(" * ")))
            }
            ExprKind::Div(num, den) => {
                let cn = Self::emit_code(graph, *num, target)?;
                let cd = Self::emit_code(graph, *den, target)?;
                Ok(format!("({cn} / {cd})"))
            }
            ExprKind::Pow(base, exp) => {
                let cb = Self::emit_code(graph, *base, target)?;
                let ce = Self::emit_code(graph, *exp, target)?;
                match target {
                    TargetLanguage::Rust => Ok(format!("{cb}.powf({ce})")),
                    TargetLanguage::C => Ok(format!("pow({cb}, {ce})")),
                }
            }
            ExprKind::Function { name, args } => {
                let fn_name = graph.symbols.resolve(*name).unwrap_or_default();
                let code_args: Vec<String> = args
                    .iter()
                    .map(|&a| Self::emit_code(graph, a, target))
                    .collect::<AlgebraResult<_>>()?;
                match target {
                    TargetLanguage::Rust => Ok(format!("{fn_name}({})", code_args.join(", "))),
                    TargetLanguage::C => Ok(format!("{fn_name}({})", code_args.join(", "))),
                }
            }
            _ => Err(AlgebraError::EvaluationError(
                "Unsupported expression for codegen".into(),
            )),
        }
    }

    /// Compile an $N$-variable expression $f(x_1, \dots, x_n)$ into a Rust closure
    /// `Fn(&[f64]) -> f64`.
    ///
    /// The expression is lowered once to bytecode and each call runs the bytecode virtual
    /// machine on `f64`: the first variable is the machine's variable and the others are its
    /// parameters, so a call does no allocation and no symbolic substitution. If the expression
    /// uses something the bytecode compiler does not support, or a symbol that is not one of
    /// `vars`, the closure falls back to substituting the arguments and evaluating the tree
    /// (an unbound symbol then yields `NaN`, never a wrong number). Missing arguments yield `NaN`.
    pub fn compile_fn_n(
        graph: &ExprGraph,
        expr: ExprId,
        vars: &[SymbolId],
    ) -> impl Fn(&[f64]) -> f64 + use<> {
        let names: Vec<String> = vars
            .iter()
            .map(|&v| graph.symbols.resolve(v).unwrap_or_default())
            .collect();
        let bound = free_symbols(graph, expr)
            .into_iter()
            .all(|symbol| vars.contains(&symbol) || is_builtin_constant(graph, symbol));
        let program = if bound && !vars.is_empty() {
            let params: Vec<String> = names[1..].to_vec();
            VmCompiler::compile(graph, expr, &names[0], &params).ok()
        } else {
            None
        };

        let graph_clone = graph.clone();
        let vars_vec = vars.to_vec();
        let arity = vars.len();
        move |args: &[f64]| {
            if args.len() < arity {
                return f64::NAN;
            }
            if let Some(program) = &program {
                return BytecodeVM::new()
                    .eval_scalar(program, args[0], &args[1..arity])
                    .unwrap_or(f64::NAN);
            }
            let mut sub = expr;
            for (&v_sym, &val) in vars_vec.iter().zip(args.iter()) {
                let val_node = graph_clone.float(val);
                sub = graph_clone.substitute(sub, v_sym, val_node);
            }
            let ctx = EvalContext::new(53);
            match graph_clone.evalf(sub, &ctx) {
                Ok(big_val) => big_val.to_f64(),
                Err(_) => f64::NAN,
            }
        }
    }

    /// Compile a single variable expression $f(x)$ into a closure `Fn(f64) -> f64`
    /// (bytecode-compiled, see [`CodeGenerator::compile_fn_n`]).
    pub fn compile_fn1(
        graph: &ExprGraph,
        expr: ExprId,
        var: SymbolId,
    ) -> impl Fn(f64) -> f64 + use<> {
        let f = Self::compile_fn_n(graph, expr, &[var]);
        move |x| f(&[x])
    }

    /// Compile a two-variable expression $f(x, y)$ into a closure `Fn(f64, f64) -> f64`
    /// (bytecode-compiled, see [`CodeGenerator::compile_fn_n`]).
    pub fn compile_fn2(
        graph: &ExprGraph,
        expr: ExprId,
        var_x: SymbolId,
        var_y: SymbolId,
    ) -> impl Fn(f64, f64) -> f64 + use<> {
        let f = Self::compile_fn_n(graph, expr, &[var_x, var_y]);
        move |x, y| f(&[x, y])
    }
}

/// The symbols that occur in `expr`.
fn free_symbols(graph: &ExprGraph, expr: ExprId) -> Vec<SymbolId> {
    fn walk(graph: &ExprGraph, id: ExprId, out: &mut Vec<SymbolId>) {
        match &graph.get(id).kind {
            ExprKind::Symbol(s) => {
                if !out.contains(s) {
                    out.push(*s);
                }
            }
            ExprKind::Add(items) | ExprKind::Mul(items) => {
                items.iter().for_each(|&i| walk(graph, i, out))
            }
            ExprKind::Sub(a, b) | ExprKind::Div(a, b) | ExprKind::Pow(a, b) => {
                walk(graph, *a, out);
                walk(graph, *b, out);
            }
            ExprKind::Neg(inner) => walk(graph, *inner, out),
            ExprKind::Function { args, .. } => args.iter().for_each(|&a| walk(graph, a, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(graph, expr, &mut out);
    out
}

/// `pi` and `e` are bound by the bytecode compiler itself.
fn is_builtin_constant(graph: &ExprGraph, symbol: SymbolId) -> bool {
    matches!(graph.symbols.resolve(symbol).as_deref(), Some("pi" | "e"))
}
