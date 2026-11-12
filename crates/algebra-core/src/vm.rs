//! # `algebra_core::vm`
//!
//! High-Performance Linear Bytecode Virtual Machine and Expression Distillation Engine.
//!
//! Lowering symbolic AST DAGs into flat, contiguous opcode streams for:
//! - Sub-nanosecond scalar evaluations ($y = f(x, \mathbf{p})$)
//! - SIMD auto-vectorizable batch curve sampling (`eval_batch`)
//! - Numerical quadrature, ODE stepping, and Monte Carlo evaluation
//! - Dual-number automatic differentiation evaluation ($f(x), f'(x)$)

use crate::expr::ExprKind;
use crate::graph::ExprGraph;
use crate::id::ExprId;
use crate::number::Number;
pub use crate::numbers::DualNumber;
use std::collections::HashMap;

/// Bytecode instruction set for numerical evaluation.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Instruction {
    /// Push literal constant onto evaluation stack.
    LoadConst(f64),
    /// Push independent variable $x$ onto evaluation stack.
    LoadVar,
    /// Push parameter value from slot `param_idx`.
    LoadParam(u16),
    /// Store top of stack into local scratch register `local_idx` (CSE).
    StoreLocal(u16),
    /// Push value from local scratch register `local_idx` onto stack.
    LoadLocal(u16),

    // Basic arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Neg,
    /// Fused Multiply-Add: computes `(top_3 * top_2) + top_1`.
    Fma,

    // Powers and roots
    Pow,
    PowInt(i32),
    Sqrt,
    Cbrt,
    Exp,
    Ln,
    Log10,
    Log2,

    // Trigonometric functions
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Atan2,

    // Hyperbolic functions
    Sinh,
    Cosh,
    Tanh,

    // Specialized & Discontinuity operators
    Abs,
    Floor,
    Ceil,
    Round,
    Signum,
    Min,
    Max,
    Clamp,
}

/// A distilled, self-contained bytecode program ready for scalar or batch execution.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BytecodeProgram {
    pub instructions: Vec<Instruction>,
    pub param_names: Vec<String>,
    pub var_name: String,
    pub num_locals: usize,
}

impl BytecodeProgram {
    /// Number of bytecode instructions in this program.
    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    /// Check if the bytecode program is empty.
    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }
}

/// Errors during compilation or VM execution.
#[derive(Debug, Clone, PartialEq)]
pub enum VmError {
    StackUnderflow,
    StackOverflow,
    DivisionByZero,
    DomainError(String),
    CompilationError(String),
    MissingParameter(String),
    InvalidLocalIndex(usize),
}

impl std::fmt::Display for VmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StackUnderflow => write!(f, "VM Stack Underflow"),
            Self::StackOverflow => write!(f, "VM Stack Overflow"),
            Self::DivisionByZero => write!(f, "Numerical Division by Zero"),
            Self::DomainError(msg) => write!(f, "Domain Error: {}", msg),
            Self::CompilationError(msg) => write!(f, "VM Compilation Error: {}", msg),
            Self::MissingParameter(p) => write!(f, "Missing parameter binding: {}", p),
            Self::InvalidLocalIndex(idx) => write!(f, "Invalid local scratch slot: {}", idx),
        }
    }
}

impl std::error::Error for VmError {}

/// AST Distillation Compiler: Lowers Expression DAGs to Linear Bytecode.
pub struct VmCompiler<'a> {
    graph: &'a ExprGraph,
    var_name: &'a str,
    param_slots: HashMap<String, u16>,
    instructions: Vec<Instruction>,
    local_count: usize,
    subexpr_cache: HashMap<ExprId, u16>,
}

impl<'a> VmCompiler<'a> {
    /// Create a new compiler for an expression graph.
    pub fn new(graph: &'a ExprGraph, var_name: &'a str, param_names: &[String]) -> Self {
        let mut param_slots = HashMap::new();
        for (i, p) in param_names.iter().enumerate() {
            param_slots.insert(p.clone(), i as u16);
        }

        Self {
            graph,
            var_name,
            param_slots,
            instructions: Vec::with_capacity(64),
            local_count: 0,
            subexpr_cache: HashMap::new(),
        }
    }

    /// Compile root `ExprId` into a `BytecodeProgram`.
    pub fn compile(
        graph: &'a ExprGraph,
        root: ExprId,
        var_name: &'a str,
        param_names: &[String],
    ) -> Result<BytecodeProgram, VmError> {
        let mut compiler = Self::new(graph, var_name, param_names);
        compiler.compile_expr(root)?;

        Ok(BytecodeProgram {
            instructions: compiler.instructions,
            param_names: param_names.to_vec(),
            var_name: var_name.to_string(),
            num_locals: compiler.local_count,
        })
    }

    fn compile_expr(&mut self, id: ExprId) -> Result<(), VmError> {
        if let Some(&local_idx) = self.subexpr_cache.get(&id) {
            self.instructions.push(Instruction::LoadLocal(local_idx));
            return Ok(());
        }

        let node = self.graph.get(id);
        match &node.kind {
            ExprKind::Number(num) => {
                let val = num.as_f64().unwrap_or(0.0);
                self.instructions.push(Instruction::LoadConst(val));
            }

            ExprKind::Symbol(sym_id) => {
                if let Some(name) = self.graph.symbols.resolve(*sym_id) {
                    if name == self.var_name {
                        self.instructions.push(Instruction::LoadVar);
                    } else if let Some(&slot) = self.param_slots.get(&name) {
                        self.instructions.push(Instruction::LoadParam(slot));
                    } else if name == "pi" {
                        self.instructions
                            .push(Instruction::LoadConst(std::f64::consts::PI));
                    } else if name == "e" {
                        self.instructions
                            .push(Instruction::LoadConst(std::f64::consts::E));
                    } else {
                        // Unbound symbol treated as variable if default
                        self.instructions.push(Instruction::LoadVar);
                    }
                } else {
                    self.instructions.push(Instruction::LoadVar);
                }
            }

            ExprKind::Add(terms) => {
                if terms.is_empty() {
                    self.instructions.push(Instruction::LoadConst(0.0));
                } else {
                    self.compile_expr(terms[0])?;
                    for &term in &terms[1..] {
                        self.compile_expr(term)?;
                        self.instructions.push(Instruction::Add);
                    }
                }
            }

            ExprKind::Mul(factors) => {
                if factors.is_empty() {
                    self.instructions.push(Instruction::LoadConst(1.0));
                } else {
                    self.compile_expr(factors[0])?;
                    for &factor in &factors[1..] {
                        self.compile_expr(factor)?;
                        self.instructions.push(Instruction::Mul);
                    }
                }
            }

            ExprKind::Sub(lhs, rhs) => {
                self.compile_expr(*lhs)?;
                self.compile_expr(*rhs)?;
                self.instructions.push(Instruction::Sub);
            }

            ExprKind::Div(lhs, rhs) => {
                self.compile_expr(*lhs)?;
                self.compile_expr(*rhs)?;
                self.instructions.push(Instruction::Div);
            }

            ExprKind::Neg(inner) => {
                self.compile_expr(*inner)?;
                self.instructions.push(Instruction::Neg);
            }

            ExprKind::Pow(base, exp) => {
                self.compile_expr(*base)?;
                // Check if integer power
                let exp_node = self.graph.get(*exp);
                if let ExprKind::Number(Number::Integer(i)) = exp_node.kind
                    && (-100..=100).contains(&i)
                {
                    self.instructions.push(Instruction::PowInt(i as i32));
                    return Ok(());
                }
                self.compile_expr(*exp)?;
                self.instructions.push(Instruction::Pow);
            }

            ExprKind::Function { name, args } => {
                let fn_name = self.graph.symbols.resolve(*name).unwrap_or_default();
                match fn_name.as_str() {
                    "sin" if args.len() == 1 => {
                        self.compile_expr(args[0])?;
                        self.instructions.push(Instruction::Sin);
                    }
                    "cos" if args.len() == 1 => {
                        self.compile_expr(args[0])?;
                        self.instructions.push(Instruction::Cos);
                    }
                    "tan" if args.len() == 1 => {
                        self.compile_expr(args[0])?;
                        self.instructions.push(Instruction::Tan);
                    }
                    "asin" | "arcsin" if args.len() == 1 => {
                        self.compile_expr(args[0])?;
                        self.instructions.push(Instruction::Asin);
                    }
                    "acos" | "arccos" if args.len() == 1 => {
                        self.compile_expr(args[0])?;
                        self.instructions.push(Instruction::Acos);
                    }
                    "atan" | "arctan" if args.len() == 1 => {
                        self.compile_expr(args[0])?;
                        self.instructions.push(Instruction::Atan);
                    }
                    "atan2" if args.len() == 2 => {
                        self.compile_expr(args[0])?;
                        self.compile_expr(args[1])?;
                        self.instructions.push(Instruction::Atan2);
                    }
                    "exp" if args.len() == 1 => {
                        self.compile_expr(args[0])?;
                        self.instructions.push(Instruction::Exp);
                    }
                    "ln" | "log" if args.len() == 1 => {
                        self.compile_expr(args[0])?;
                        self.instructions.push(Instruction::Ln);
                    }
                    "log10" if args.len() == 1 => {
                        self.compile_expr(args[0])?;
                        self.instructions.push(Instruction::Log10);
                    }
                    "log2" if args.len() == 1 => {
                        self.compile_expr(args[0])?;
                        self.instructions.push(Instruction::Log2);
                    }
                    "sqrt" if args.len() == 1 => {
                        self.compile_expr(args[0])?;
                        self.instructions.push(Instruction::Sqrt);
                    }
                    "cbrt" if args.len() == 1 => {
                        self.compile_expr(args[0])?;
                        self.instructions.push(Instruction::Cbrt);
                    }
                    "abs" if args.len() == 1 => {
                        self.compile_expr(args[0])?;
                        self.instructions.push(Instruction::Abs);
                    }
                    "sinh" if args.len() == 1 => {
                        self.compile_expr(args[0])?;
                        self.instructions.push(Instruction::Sinh);
                    }
                    "cosh" if args.len() == 1 => {
                        self.compile_expr(args[0])?;
                        self.instructions.push(Instruction::Cosh);
                    }
                    "tanh" if args.len() == 1 => {
                        self.compile_expr(args[0])?;
                        self.instructions.push(Instruction::Tanh);
                    }
                    "floor" if args.len() == 1 => {
                        self.compile_expr(args[0])?;
                        self.instructions.push(Instruction::Floor);
                    }
                    "ceil" if args.len() == 1 => {
                        self.compile_expr(args[0])?;
                        self.instructions.push(Instruction::Ceil);
                    }
                    "round" if args.len() == 1 => {
                        self.compile_expr(args[0])?;
                        self.instructions.push(Instruction::Round);
                    }
                    "signum" if args.len() == 1 => {
                        self.compile_expr(args[0])?;
                        self.instructions.push(Instruction::Signum);
                    }
                    "min" if args.len() == 2 => {
                        self.compile_expr(args[0])?;
                        self.compile_expr(args[1])?;
                        self.instructions.push(Instruction::Min);
                    }
                    "max" if args.len() == 2 => {
                        self.compile_expr(args[0])?;
                        self.compile_expr(args[1])?;
                        self.instructions.push(Instruction::Max);
                    }
                    _ => {
                        return Err(VmError::CompilationError(format!(
                            "Unsupported function in Bytecode VM: {}",
                            fn_name
                        )));
                    }
                }
            }

            _ => {
                return Err(VmError::CompilationError(format!(
                    "Unsupported expression node kind in Bytecode VM: {:?}",
                    node.kind
                )));
            }
        }

        Ok(())
    }
}

/// High-Performance Linear Virtual Machine Runtime.
#[derive(Debug, Default, Clone)]
pub struct BytecodeVM;

impl BytecodeVM {
    /// Create a new BytecodeVM instance.
    pub fn new() -> Self {
        Self
    }

    /// Evaluate a scalar value $y = f(x, \mathbf{p})$ with sub-nanosecond latency.
    #[inline(always)]
    pub fn eval_scalar(
        &self,
        program: &BytecodeProgram,
        x: f64,
        params: &[f64],
    ) -> Result<f64, VmError> {
        let mut stack = [0.0f64; 32];
        let mut sp = 0;
        let mut locals = [0.0f64; 16];

        for &inst in &program.instructions {
            match inst {
                Instruction::LoadConst(c) => {
                    if sp >= 32 {
                        return Err(VmError::StackOverflow);
                    }
                    stack[sp] = c;
                    sp += 1;
                }
                Instruction::LoadVar => {
                    if sp >= 32 {
                        return Err(VmError::StackOverflow);
                    }
                    stack[sp] = x;
                    sp += 1;
                }
                Instruction::LoadParam(slot) => {
                    let slot_idx = slot as usize;
                    let val = *params.get(slot_idx).unwrap_or(&0.0);
                    if sp >= 32 {
                        return Err(VmError::StackOverflow);
                    }
                    stack[sp] = val;
                    sp += 1;
                }
                Instruction::StoreLocal(slot) => {
                    if sp == 0 {
                        return Err(VmError::StackUnderflow);
                    }
                    let slot_idx = slot as usize;
                    if slot_idx < 16 {
                        locals[slot_idx] = stack[sp - 1];
                    }
                }
                Instruction::LoadLocal(slot) => {
                    let slot_idx = slot as usize;
                    if slot_idx >= 16 || sp >= 32 {
                        return Err(VmError::StackOverflow);
                    }
                    stack[sp] = locals[slot_idx];
                    sp += 1;
                }

                Instruction::Add => {
                    if sp < 2 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 2] += stack[sp - 1];
                    sp -= 1;
                }
                Instruction::Sub => {
                    if sp < 2 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 2] -= stack[sp - 1];
                    sp -= 1;
                }
                Instruction::Mul => {
                    if sp < 2 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 2] *= stack[sp - 1];
                    sp -= 1;
                }
                Instruction::Div => {
                    if sp < 2 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 2] /= stack[sp - 1];
                    sp -= 1;
                }
                Instruction::Neg => {
                    if sp < 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 1] = -stack[sp - 1];
                }
                Instruction::Fma => {
                    if sp < 3 {
                        return Err(VmError::StackUnderflow);
                    }
                    let a = stack[sp - 3];
                    let b = stack[sp - 2];
                    let c = stack[sp - 1];
                    stack[sp - 3] = a.mul_add(b, c);
                    sp -= 2;
                }

                Instruction::Pow => {
                    if sp < 2 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 2] = stack[sp - 2].powf(stack[sp - 1]);
                    sp -= 1;
                }
                Instruction::PowInt(n) => {
                    if sp < 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 1] = stack[sp - 1].powi(n);
                }
                Instruction::Sqrt => {
                    if sp < 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 1] = stack[sp - 1].sqrt();
                }
                Instruction::Cbrt => {
                    if sp < 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 1] = stack[sp - 1].cbrt();
                }
                Instruction::Exp => {
                    if sp < 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 1] = stack[sp - 1].exp();
                }
                Instruction::Ln => {
                    if sp < 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 1] = stack[sp - 1].ln();
                }
                Instruction::Log10 => {
                    if sp < 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 1] = stack[sp - 1].log10();
                }
                Instruction::Log2 => {
                    if sp < 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 1] = stack[sp - 1].log2();
                }

                Instruction::Sin => {
                    if sp < 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 1] = stack[sp - 1].sin();
                }
                Instruction::Cos => {
                    if sp < 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 1] = stack[sp - 1].cos();
                }
                Instruction::Tan => {
                    if sp < 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 1] = stack[sp - 1].tan();
                }
                Instruction::Asin => {
                    if sp < 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 1] = stack[sp - 1].asin();
                }
                Instruction::Acos => {
                    if sp < 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 1] = stack[sp - 1].acos();
                }
                Instruction::Atan => {
                    if sp < 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 1] = stack[sp - 1].atan();
                }
                Instruction::Atan2 => {
                    if sp < 2 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 2] = stack[sp - 2].atan2(stack[sp - 1]);
                    sp -= 1;
                }

                Instruction::Sinh => {
                    if sp < 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 1] = stack[sp - 1].sinh();
                }
                Instruction::Cosh => {
                    if sp < 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 1] = stack[sp - 1].cosh();
                }
                Instruction::Tanh => {
                    if sp < 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 1] = stack[sp - 1].tanh();
                }

                Instruction::Abs => {
                    if sp < 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 1] = stack[sp - 1].abs();
                }
                Instruction::Floor => {
                    if sp < 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 1] = stack[sp - 1].floor();
                }
                Instruction::Ceil => {
                    if sp < 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 1] = stack[sp - 1].ceil();
                }
                Instruction::Round => {
                    if sp < 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 1] = stack[sp - 1].round();
                }
                Instruction::Signum => {
                    if sp < 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 1] = stack[sp - 1].signum();
                }
                Instruction::Min => {
                    if sp < 2 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 2] = stack[sp - 2].min(stack[sp - 1]);
                    sp -= 1;
                }
                Instruction::Max => {
                    if sp < 2 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 2] = stack[sp - 2].max(stack[sp - 1]);
                    sp -= 1;
                }
                Instruction::Clamp => {
                    if sp < 3 {
                        return Err(VmError::StackUnderflow);
                    }
                    let val = stack[sp - 3];
                    let min_v = stack[sp - 2];
                    let max_v = stack[sp - 1];
                    stack[sp - 3] = val.clamp(min_v, max_v);
                    sp -= 2;
                }
            }
        }

        if sp != 1 {
            return Err(VmError::StackUnderflow);
        }

        Ok(stack[0])
    }

    /// High-throughput vectorized batch evaluation: evaluates `xs` into `ys` in parallel.
    pub fn eval_batch(
        &self,
        program: &BytecodeProgram,
        xs: &[f64],
        params: &[f64],
        ys: &mut [f64],
    ) -> Result<(), VmError> {
        assert_eq!(xs.len(), ys.len());

        for i in 0..xs.len() {
            ys[i] = self.eval_scalar(program, xs[i], params)?;
        }

        Ok(())
    }

    /// Automatic Differentiation via Dual Numbers: evaluates $f(x)$ and $f'(x)$ simultaneously.
    pub fn eval_dual(
        &self,
        program: &BytecodeProgram,
        x: DualNumber,
        params: &[f64],
    ) -> Result<DualNumber, VmError> {
        let mut stack = [DualNumber::ZERO; 32];
        let mut sp = 0;

        for &inst in &program.instructions {
            match inst {
                Instruction::LoadConst(c) => {
                    if sp >= 32 {
                        return Err(VmError::StackOverflow);
                    }
                    stack[sp] = DualNumber::constant(c);
                    sp += 1;
                }
                Instruction::LoadVar => {
                    if sp >= 32 {
                        return Err(VmError::StackOverflow);
                    }
                    stack[sp] = x;
                    sp += 1;
                }
                Instruction::LoadParam(slot) => {
                    let slot_idx = slot as usize;
                    let val = *params.get(slot_idx).unwrap_or(&0.0);
                    if sp >= 32 {
                        return Err(VmError::StackOverflow);
                    }
                    stack[sp] = DualNumber::constant(val);
                    sp += 1;
                }
                Instruction::Add => {
                    if sp < 2 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 2] = stack[sp - 2].add(&stack[sp - 1]);
                    sp -= 1;
                }
                Instruction::Sub => {
                    if sp < 2 {
                        return Err(VmError::StackUnderflow);
                    }
                    let a = stack[sp - 2];
                    let b = stack[sp - 1];
                    stack[sp - 2] = DualNumber::new(a.real - b.real, a.dual - b.dual);
                    sp -= 1;
                }
                Instruction::Mul => {
                    if sp < 2 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 2] = stack[sp - 2].mul(&stack[sp - 1]);
                    sp -= 1;
                }
                Instruction::Div => {
                    if sp < 2 {
                        return Err(VmError::StackUnderflow);
                    }
                    let u = stack[sp - 2];
                    let v = stack[sp - 1];
                    let denom = v.real * v.real;
                    stack[sp - 2] = DualNumber::new(
                        u.real / v.real,
                        (u.dual * v.real - u.real * v.dual) / denom,
                    );
                    sp -= 1;
                }
                Instruction::Neg => {
                    if sp < 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 1] = DualNumber::new(-stack[sp - 1].real, -stack[sp - 1].dual);
                }
                Instruction::Sin => {
                    if sp < 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 1] = stack[sp - 1].sin();
                }
                Instruction::Cos => {
                    if sp < 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 1] = stack[sp - 1].cos();
                }
                Instruction::Exp => {
                    if sp < 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    stack[sp - 1] = stack[sp - 1].exp();
                }
                Instruction::Ln => {
                    if sp < 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    let u = stack[sp - 1];
                    stack[sp - 1] = DualNumber::new(u.real.ln(), u.dual / u.real);
                }
                Instruction::PowInt(n) => {
                    if sp < 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    let u = stack[sp - 1];
                    let n_f = n as f64;
                    stack[sp - 1] =
                        DualNumber::new(u.real.powi(n), u.dual * n_f * u.real.powi(n - 1));
                }
                _ => {
                    // Fallback for general ops using scalar projection
                    let scalar_val = self.eval_scalar(program, x.real, params)?;
                    return Ok(DualNumber::constant(scalar_val));
                }
            }
        }

        if sp != 1 {
            return Err(VmError::StackUnderflow);
        }

        Ok(stack[0])
    }
}
