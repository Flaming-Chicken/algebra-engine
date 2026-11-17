//! # `algebra_engine::api`
//!
//! Ergonomic, Lifetime-Aware Symbolic & Numerical API for URAE.
//!
//! Provides composable high-level handles bound by lifetime `'g` to an underlying [`ExprGraph`]
//! arena, alongside compile-time `const` arithmetic primitives and mutable/immutable numerical matrix
//! views:
//! - [`Sym<'g>`] & [`Expr<'g>`]: `Copy` borrowed handles supporting natural Rust operator overloading
//!   (`+`, `-`, `*`, `/`, unary `-`), method chaining (`.diff(&x)`, `.integrate(&x)`, `.simplify()`,
//!   `.expand()`, `.factor()`, `.solve(&x)`, `.limit(&x, p)`, `.series(&x, p, n)`), and exact/numeric
//!   evaluation.
//! - [`ConstRational`] & [`ConstPoly<N>`]: Compile-time `const fn` exact fractions and polynomials with
//!   `const` differentiation, definite integration, and Horner evaluation, liftable into [`ExprGraph`].
//! - [`SymMat<'g>`] & [`NumMat`]: Array & matrix algebra with immutable borrowed views (`&`), mutable
//!   in-place row/entry operations (`&mut`), linear system solving, eigenvalues, RREF, rank, and nullspace.

use crate::calculus::SymbolicCalculus;
use crate::executor::ExecutionContext;
use crate::logic::AssumptionsContext;
use crate::matrix::{MatrixBuffer, SymbolicMatrix};
use crate::numeric::{BigValue, EvalContext, NumericalEval, eval_uncertain, try_eval_exact};
use crate::poly::collect_expr_symbols;
use crate::simplify::{AlgebraicTransformations, Simplifier};
use crate::solver::{SolveOutcome, SymbolicSolver};
use crate::systems::LinearSystemSolver;
use algebra_core::format::{Formatter, LatexFormatter, UnicodeFormatter};
use algebra_core::parser::ExprParser;
use algebra_core::{
    AlgebraError, AlgebraResult, Constant, ExprGraph, ExprId, SymbolId, UncertainValue,
};
use num_rational::BigRational;
use std::collections::{BTreeSet, HashMap};
use std::fmt;
use std::ops::{Add, Div, Index, IndexMut, Mul, Neg, Sub};

// =========================================================================
// Compile-Time `const` Exact Rationals & Polynomials
// =========================================================================

const fn const_gcd_u64(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

/// Compile-time exact rational number `num / den` (`den > 0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConstRational {
    pub num: i64,
    pub den: i64,
}

impl ConstRational {
    /// Zero constant `0 / 1`.
    pub const ZERO: Self = Self { num: 0, den: 1 };
    /// Unit constant `1 / 1`.
    pub const ONE: Self = Self { num: 1, den: 1 };

    /// Construct a normalized `ConstRational` at compile time (`den != 0`).
    pub const fn new(mut num: i64, mut den: i64) -> Self {
        assert!(den != 0, "ConstRational denominator cannot be zero");
        if num == 0 {
            return Self::ZERO;
        }
        if den < 0 {
            num = -num;
            den = -den;
        }
        let g = const_gcd_u64(num.unsigned_abs(), den.unsigned_abs()) as i64;
        Self {
            num: num / g,
            den: den / g,
        }
    }

    /// Construct from an exact integer at compile time.
    pub const fn from_int(n: i64) -> Self {
        Self { num: n, den: 1 }
    }

    /// Compile-time addition.
    pub const fn add(self, rhs: Self) -> Self {
        Self::new(self.num * rhs.den + rhs.num * self.den, self.den * rhs.den)
    }

    /// Compile-time subtraction.
    pub const fn sub(self, rhs: Self) -> Self {
        Self::new(self.num * rhs.den - rhs.num * self.den, self.den * rhs.den)
    }

    /// Compile-time multiplication.
    pub const fn mul(self, rhs: Self) -> Self {
        Self::new(self.num * rhs.num, self.den * rhs.den)
    }

    /// Compile-time division.
    pub const fn div(self, rhs: Self) -> Self {
        assert!(rhs.num != 0, "ConstRational division by zero");
        Self::new(self.num * rhs.den, self.den * rhs.num)
    }

    /// Compile-time negation.
    pub const fn neg(self) -> Self {
        Self {
            num: -self.num,
            den: self.den,
        }
    }

    /// Compile-time integer exponentiation.
    pub const fn pow(self, mut exp: u32) -> Self {
        let mut base = self;
        let mut acc = Self::ONE;
        while exp > 0 {
            if exp & 1 == 1 {
                acc = acc.mul(base);
            }
            exp >>= 1;
            if exp > 0 {
                base = base.mul(base);
            }
        }
        acc
    }

    /// Convert to `f64` at compile time or runtime.
    pub const fn to_f64(self) -> f64 {
        (self.num as f64) / (self.den as f64)
    }

    /// Intern into a symbolic [`ExprGraph`].
    pub fn intern<'g>(self, graph: &'g ExprGraph) -> Expr<'g> {
        Expr {
            graph,
            id: graph.rational(self.num, self.den),
        }
    }
}

/// Compile-time univariate polynomial $\sum_{k=0}^{N-1} c_k x^k$ stored in ascending degree order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstPoly<const N: usize> {
    pub coeffs: [f64; N],
}

impl<const N: usize> ConstPoly<N> {
    /// Create a compile-time polynomial from ascending coefficients `[c_0, c_1, ..., c_{N-1}]`.
    pub const fn new(coeffs: [f64; N]) -> Self {
        Self { coeffs }
    }

    /// Zero polynomial.
    pub const fn zero() -> Self {
        Self { coeffs: [0.0; N] }
    }

    /// Compile-time polynomial evaluation using Horner's method.
    pub const fn eval(self, x: f64) -> f64 {
        let mut acc = 0.0;
        let mut i = N;
        while i > 0 {
            i -= 1;
            acc = acc * x + self.coeffs[i];
        }
        acc
    }

    /// Compile-time symbolic derivative $p'(x)$.
    pub const fn deriv(self) -> Self {
        let mut out = [0.0; N];
        let mut i = 1;
        while i < N {
            out[i - 1] = self.coeffs[i] * (i as f64);
            i += 1;
        }
        Self { coeffs: out }
    }

    /// Compile-time addition.
    pub const fn add(self, rhs: Self) -> Self {
        let mut out = [0.0; N];
        let mut i = 0;
        while i < N {
            out[i] = self.coeffs[i] + rhs.coeffs[i];
            i += 1;
        }
        Self { coeffs: out }
    }

    /// Compile-time subtraction.
    pub const fn sub(self, rhs: Self) -> Self {
        let mut out = [0.0; N];
        let mut i = 0;
        while i < N {
            out[i] = self.coeffs[i] - rhs.coeffs[i];
            i += 1;
        }
        Self { coeffs: out }
    }

    /// Compile-time scalar multiplication.
    pub const fn scale(self, s: f64) -> Self {
        let mut out = [0.0; N];
        let mut i = 0;
        while i < N {
            out[i] = self.coeffs[i] * s;
            i += 1;
        }
        Self { coeffs: out }
    }

    /// Compile-time definite integral $\int_a^b p(x)\,dx$.
    pub const fn definite_integral(self, a: f64, b: f64) -> f64 {
        let mut acc_b = 0.0;
        let mut acc_a = 0.0;
        let mut pow_b = b;
        let mut pow_a = a;
        let mut i = 0;
        while i < N {
            let term_coeff = self.coeffs[i] / ((i + 1) as f64);
            acc_b += term_coeff * pow_b;
            acc_a += term_coeff * pow_a;
            pow_b *= b;
            pow_a *= a;
            i += 1;
        }
        acc_b - acc_a
    }

    /// Intern this compile-time polynomial into a runtime [`ExprGraph`] in variable `var`.
    pub fn intern<'g>(self, graph: &'g ExprGraph, var: &Sym<'g>) -> Expr<'g> {
        let x = var.as_expr();
        let mut terms = Vec::new();
        for (deg, &c) in self.coeffs.iter().enumerate() {
            if c == 0.0 {
                continue;
            }
            let c_expr = Expr {
                graph,
                id: if c.fract() == 0.0 && c.abs() < 9e15 {
                    graph.integer(c as i64)
                } else {
                    graph.float(c)
                },
            };
            let term = match deg {
                0 => c_expr,
                1 => c_expr * x,
                _ => c_expr * x.pow(deg as i64),
            };
            terms.push(term.id);
        }
        let id = if terms.is_empty() {
            graph.integer(0)
        } else {
            graph.add(terms)
        };
        Expr { graph, id }
    }
}

// =========================================================================
// Borrowed Lifetime-Tracked Symbols (`Sym<'g>`) and Expressions (`Expr<'g>`)
// =========================================================================

/// Conversion trait allowing scalars, symbols, and expressions to compose smoothly inside [`Expr<'g>`] methods.
pub trait IntoExpr<'g> {
    fn into_expr(self, graph: &'g ExprGraph) -> Expr<'g>;
}

/// Borrowed handle to an interned symbolic variable bound to `'g` (`&'g ExprGraph`).
#[derive(Clone, Copy)]
pub struct Sym<'g> {
    graph: &'g ExprGraph,
    sym_id: SymbolId,
    expr_id: ExprId,
}

/// Borrowed `Copy` handle to a symbolic expression node bound to `'g` (`&'g ExprGraph`).
#[derive(Clone, Copy)]
pub struct Expr<'g> {
    graph: &'g ExprGraph,
    id: ExprId,
}

impl<'g> Sym<'g> {
    /// Create or retrieve a symbol named `name` inside `graph`.
    pub fn new(graph: &'g ExprGraph, name: &str) -> Self {
        let sym_id = graph.symbols.get_or_intern(name);
        let expr_id = graph.symbol(name);
        Self {
            graph,
            sym_id,
            expr_id,
        }
    }

    /// Construct multiple symbols at once (e.g. `let [x, y, z] = Sym::many(&g, ["x", "y", "z"]);`).
    pub fn many<const N: usize>(graph: &'g ExprGraph, names: [&str; N]) -> [Self; N] {
        names.map(|n| Self::new(graph, n))
    }

    /// Borrow underlying graph.
    pub fn graph(&self) -> &'g ExprGraph {
        self.graph
    }

    /// Internal [`SymbolId`].
    pub fn sym_id(&self) -> SymbolId {
        self.sym_id
    }

    /// Internal [`ExprId`].
    pub fn id(&self) -> ExprId {
        self.expr_id
    }

    /// Resolve symbol name.
    pub fn name(&self) -> String {
        self.graph.symbols.resolve(self.sym_id).unwrap_or_default()
    }

    /// View this symbol as a general [`Expr<'g>`].
    pub fn as_expr(&self) -> Expr<'g> {
        Expr {
            graph: self.graph,
            id: self.expr_id,
        }
    }

    /// Symbolic power `self^exp`.
    pub fn pow<R: IntoExpr<'g>>(&self, exp: R) -> Expr<'g> {
        self.as_expr().pow(exp)
    }

    pub fn sin(&self) -> Expr<'g> {
        self.as_expr().sin()
    }
    pub fn cos(&self) -> Expr<'g> {
        self.as_expr().cos()
    }
    pub fn tan(&self) -> Expr<'g> {
        self.as_expr().tan()
    }
    pub fn asin(&self) -> Expr<'g> {
        self.as_expr().asin()
    }
    pub fn acos(&self) -> Expr<'g> {
        self.as_expr().acos()
    }
    pub fn atan(&self) -> Expr<'g> {
        self.as_expr().atan()
    }
    pub fn sinh(&self) -> Expr<'g> {
        self.as_expr().sinh()
    }
    pub fn cosh(&self) -> Expr<'g> {
        self.as_expr().cosh()
    }
    pub fn tanh(&self) -> Expr<'g> {
        self.as_expr().tanh()
    }
    pub fn exp(&self) -> Expr<'g> {
        self.as_expr().exp()
    }
    pub fn ln(&self) -> Expr<'g> {
        self.as_expr().ln()
    }
    pub fn sqrt(&self) -> Expr<'g> {
        self.as_expr().sqrt()
    }
    pub fn abs(&self) -> Expr<'g> {
        self.as_expr().abs()
    }

    pub fn diff(&self, wrt: &Sym<'g>) -> Expr<'g> {
        self.as_expr().diff(wrt)
    }
    pub fn integrate(&self, wrt: &Sym<'g>) -> AlgebraResult<Expr<'g>> {
        self.as_expr().integrate(wrt)
    }
    pub fn integrate_definite<L: IntoExpr<'g>, U: IntoExpr<'g>>(
        &self,
        wrt: &Sym<'g>,
        lower: L,
        upper: U,
    ) -> AlgebraResult<Expr<'g>> {
        self.as_expr().integrate_definite(wrt, lower, upper)
    }
    pub fn limit<P: IntoExpr<'g>>(&self, wrt: &Sym<'g>, point: P) -> AlgebraResult<Expr<'g>> {
        self.as_expr().limit(wrt, point)
    }
    pub fn series<P: IntoExpr<'g>>(&self, wrt: &Sym<'g>, point: P, order: u32) -> Expr<'g> {
        self.as_expr().series(wrt, point, order)
    }
    pub fn solve(&self, wrt: &Sym<'g>) -> AlgebraResult<Vec<Expr<'g>>> {
        self.as_expr().solve(wrt)
    }
    pub fn subs<V: IntoExpr<'g>>(&self, target: &Sym<'g>, val: V) -> Expr<'g> {
        self.as_expr().subs(target, val)
    }
}

impl<'g> From<Sym<'g>> for Expr<'g> {
    fn from(s: Sym<'g>) -> Self {
        s.as_expr()
    }
}

impl<'g> From<&Sym<'g>> for Expr<'g> {
    fn from(s: &Sym<'g>) -> Self {
        s.as_expr()
    }
}

impl<'g> Expr<'g> {
    /// Wrap an existing [`ExprId`] from `graph`.
    pub fn from_id(graph: &'g ExprGraph, id: ExprId) -> Self {
        Self { graph, id }
    }

    /// Parse a mathematical expression string into an [`Expr<'g>`].
    pub fn parse(graph: &'g ExprGraph, input: &str) -> AlgebraResult<Self> {
        let id = ExprParser::new(graph)
            .parse(input)
            .map_err(|e| AlgebraError::EvaluationError(e.to_string()))?;
        Ok(Self { graph, id })
    }

    /// Integer literal inside `graph`.
    pub fn int(graph: &'g ExprGraph, v: i64) -> Self {
        Self {
            graph,
            id: graph.integer(v),
        }
    }

    /// Exact rational literal inside `graph`.
    pub fn rational(graph: &'g ExprGraph, num: i64, den: i64) -> Self {
        Self {
            graph,
            id: graph.rational(num, den),
        }
    }

    /// Float literal (losslessly decomposed where exact) inside `graph`.
    pub fn real(graph: &'g ExprGraph, v: f64) -> Self {
        Self {
            graph,
            id: graph.float(v),
        }
    }

    /// Mathematical constant (`Constant::Pi`, `Constant::E`, `Constant::I`, etc.).
    pub fn constant(graph: &'g ExprGraph, c: Constant) -> Self {
        Self {
            graph,
            id: graph.constant(c),
        }
    }

    /// Borrow underlying graph.
    pub fn graph(&self) -> &'g ExprGraph {
        self.graph
    }

    /// Underlying DAG node handle.
    pub fn id(&self) -> ExprId {
        self.id
    }

    /// Apply a unary symbolic function (`fn_name(self)`).
    pub fn func(&self, fn_name: &str) -> Self {
        Self {
            graph: self.graph,
            id: self.graph.function(fn_name, [self.id]),
        }
    }

    pub fn sin(&self) -> Self {
        self.func("sin")
    }
    pub fn cos(&self) -> Self {
        self.func("cos")
    }
    pub fn tan(&self) -> Self {
        self.func("tan")
    }
    pub fn asin(&self) -> Self {
        self.func("asin")
    }
    pub fn acos(&self) -> Self {
        self.func("acos")
    }
    pub fn atan(&self) -> Self {
        self.func("atan")
    }
    pub fn sinh(&self) -> Self {
        self.func("sinh")
    }
    pub fn cosh(&self) -> Self {
        self.func("cosh")
    }
    pub fn tanh(&self) -> Self {
        self.func("tanh")
    }
    pub fn exp(&self) -> Self {
        self.func("exp")
    }
    pub fn ln(&self) -> Self {
        self.func("ln")
    }
    pub fn sqrt(&self) -> Self {
        self.func("sqrt")
    }
    pub fn abs(&self) -> Self {
        self.func("abs")
    }
    pub fn gamma(&self) -> Self {
        self.func("gamma")
    }
    pub fn erf(&self) -> Self {
        self.func("erf")
    }

    /// Exponentiation `self^exp`.
    pub fn pow<R: IntoExpr<'g>>(&self, exp: R) -> Self {
        let r = exp.into_expr(self.graph);
        Self {
            graph: self.graph,
            id: self.graph.pow(self.id, r.id),
        }
    }

    /// Logarithm in arbitrary base: `log(self, base)`.
    pub fn log<R: IntoExpr<'g>>(&self, base: R) -> Self {
        let b = base.into_expr(self.graph);
        Self {
            graph: self.graph,
            id: self.graph.function("log", [self.id, b.id]),
        }
    }

    /// First partial derivative $\frac{\partial}{\partial x} \text{self}$.
    pub fn diff(&self, wrt: &Sym<'g>) -> Self {
        let raw = self.graph.diff(self.id, wrt.sym_id);
        let simp = Simplifier::simplify(self.graph, raw).unwrap_or(raw);
        Self {
            graph: self.graph,
            id: simp,
        }
    }

    /// Higher-order derivative $\frac{\partial^n}{\partial x^n} \text{self}$.
    pub fn diff_n(&self, wrt: &Sym<'g>, order: u32) -> Self {
        let mut cur = self.id;
        for _ in 0..order {
            cur = self.graph.diff(cur, wrt.sym_id);
        }
        let simp = Simplifier::simplify(self.graph, cur).unwrap_or(cur);
        Self {
            graph: self.graph,
            id: simp,
        }
    }

    /// Indefinite symbolic integral $\int \text{self} \, d x$.
    pub fn integrate(&self, wrt: &Sym<'g>) -> AlgebraResult<Self> {
        let raw = self.graph.integrate(self.id, wrt.sym_id)?;
        let simp = Simplifier::simplify(self.graph, raw).unwrap_or(raw);
        Ok(Self {
            graph: self.graph,
            id: simp,
        })
    }

    /// Definite symbolic integral $\int_a^b \text{self} \, d x$.
    pub fn integrate_definite<L: IntoExpr<'g>, U: IntoExpr<'g>>(
        &self,
        wrt: &Sym<'g>,
        lower: L,
        upper: U,
    ) -> AlgebraResult<Self> {
        let l = lower.into_expr(self.graph);
        let u = upper.into_expr(self.graph);
        let raw = self
            .graph
            .integrate_definite(self.id, wrt.sym_id, l.id, u.id)?;
        let simp = Simplifier::simplify(self.graph, raw).unwrap_or(raw);
        Ok(Self {
            graph: self.graph,
            id: simp,
        })
    }

    /// Analytical limit $\lim_{x \to p} \text{self}$.
    pub fn limit<P: IntoExpr<'g>>(&self, wrt: &Sym<'g>, point: P) -> AlgebraResult<Self> {
        let p = point.into_expr(self.graph);
        let lim = self.graph.limit(self.id, wrt.sym_id, p.id)?;
        let simp = Simplifier::simplify(self.graph, lim).unwrap_or(lim);
        Ok(Self {
            graph: self.graph,
            id: simp,
        })
    }

    /// Taylor / Maclaurin series expansion around `point` up to `order`.
    pub fn series<P: IntoExpr<'g>>(&self, wrt: &Sym<'g>, point: P, order: u32) -> Self {
        let p = point.into_expr(self.graph);
        let ser = self.graph.taylor_series(self.id, wrt.sym_id, p.id, order);
        let simp = Simplifier::simplify(self.graph, ser).unwrap_or(ser);
        Self {
            graph: self.graph,
            id: simp,
        }
    }

    /// Closed-form symbolic summation $\sum_{k=\text{lo}}^{\text{hi}} \text{self}$.
    pub fn summation<L: IntoExpr<'g>, U: IntoExpr<'g>>(
        &self,
        wrt: &Sym<'g>,
        lower: L,
        upper: U,
    ) -> AlgebraResult<Self> {
        let l = lower.into_expr(self.graph);
        let u = upper.into_expr(self.graph);
        let sum = self.graph.summation(self.id, wrt.sym_id, l.id, u.id)?;
        let simp = Simplifier::simplify(self.graph, sum).unwrap_or(sum);
        Ok(Self {
            graph: self.graph,
            id: simp,
        })
    }

    /// E-Graph algebraic simplification.
    pub fn simplify(&self) -> Self {
        let simp = Simplifier::simplify(self.graph, self.id).unwrap_or(self.id);
        Self {
            graph: self.graph,
            id: simp,
        }
    }

    /// Simplification under explicit logical/domain assumptions (`x > 0`, etc.).
    pub fn simplify_with(&self, assumptions: &AssumptionsContext) -> Self {
        let simp = Simplifier::simplify_with_assumptions(self.graph, self.id, assumptions)
            .unwrap_or(self.id);
        Self {
            graph: self.graph,
            id: simp,
        }
    }

    pub fn expand(&self) -> Self {
        Self {
            graph: self.graph,
            id: AlgebraicTransformations::expand(self.graph, self.id),
        }
    }

    pub fn factor(&self) -> Self {
        Self {
            graph: self.graph,
            id: AlgebraicTransformations::factor(self.graph, self.id),
        }
    }

    pub fn together(&self) -> Self {
        Self {
            graph: self.graph,
            id: AlgebraicTransformations::together(self.graph, self.id),
        }
    }

    pub fn cancel(&self) -> Self {
        Self {
            graph: self.graph,
            id: AlgebraicTransformations::cancel(self.graph, self.id),
        }
    }

    pub fn collect(&self, wrt: &Sym<'g>) -> Self {
        Self {
            graph: self.graph,
            id: AlgebraicTransformations::collect(self.graph, self.id, wrt.sym_id),
        }
    }

    pub fn horner(&self, wrt: &Sym<'g>) -> Self {
        Self {
            graph: self.graph,
            id: AlgebraicTransformations::horner(self.graph, self.id, wrt.sym_id),
        }
    }

    pub fn partial_fractions(&self, wrt: &Sym<'g>) -> Self {
        Self {
            graph: self.graph,
            id: AlgebraicTransformations::partial_fractions(self.graph, self.id, wrt.sym_id),
        }
    }

    /// Structural substitution $\text{self}|_{x = \text{val}}$.
    pub fn subs<V: IntoExpr<'g>>(&self, target: &Sym<'g>, val: V) -> Self {
        let v = val.into_expr(self.graph);
        let subbed = self.graph.substitute(self.id, target.sym_id, v.id);
        Self {
            graph: self.graph,
            id: subbed,
        }
    }

    /// Batch substitution across `[(sym, val), ...]`.
    pub fn subs_many(&self, pairs: &[(&Sym<'g>, Expr<'g>)]) -> Self {
        let mut cur = self.id;
        for &(s, v) in pairs {
            cur = self.graph.substitute(cur, s.sym_id, v.id);
        }
        Self {
            graph: self.graph,
            id: cur,
        }
    }

    /// Solve `self == 0` for `wrt`.
    pub fn solve(&self, wrt: &Sym<'g>) -> AlgebraResult<Vec<Expr<'g>>> {
        let roots = self.graph.solveset(self.id, wrt.sym_id)?;
        Ok(roots
            .into_iter()
            .map(|r| Self {
                graph: self.graph,
                id: Simplifier::simplify(self.graph, r).unwrap_or(r),
            })
            .collect())
    }

    /// Solve `self == 0` returning typed coverage diagnostic outcome.
    pub fn solve_outcome(&self, wrt: &Sym<'g>) -> SolveOutcome<Vec<Expr<'g>>> {
        match self.graph.solveset_outcome(self.id, wrt.sym_id) {
            SolveOutcome::Solved { solution, method } => SolveOutcome::Solved {
                solution: solution
                    .into_iter()
                    .map(|id| Self {
                        graph: self.graph,
                        id,
                    })
                    .collect(),
                method,
            },
            SolveOutcome::IncompleteCoverage { partial, reason } => {
                SolveOutcome::IncompleteCoverage {
                    partial: partial
                        .into_iter()
                        .map(|id| Self {
                            graph: self.graph,
                            id,
                        })
                        .collect(),
                    reason,
                }
            }
            SolveOutcome::Unsupported {
                reason,
                failing_subexpr,
            } => SolveOutcome::Unsupported {
                reason,
                failing_subexpr,
            },
        }
    }

    /// Evaluate numerically with empty bindings (50-digit target precision).
    pub fn evalf(&self) -> AlgebraResult<BigValue> {
        self.graph.evalf(self.id, &EvalContext::default())
    }

    /// Evaluate numerically with explicit symbol bindings.
    pub fn eval_with(&self, bindings: &HashMap<String, f64>) -> AlgebraResult<BigValue> {
        let ctx = EvalContext::with_bindings(50, bindings.clone());
        self.graph.evalf(self.id, &ctx)
    }

    /// Attempt exact arbitrary-precision rational evaluation.
    pub fn eval_exact(
        &self,
        bindings: &HashMap<String, f64>,
    ) -> AlgebraResult<Option<BigRational>> {
        try_eval_exact(self.graph, self.id, bindings)
    }

    /// Evaluate with GUM Gaussian uncertainty and rigorous interval propagation.
    pub fn eval_uncertain(&self, ctx: &ExecutionContext) -> AlgebraResult<UncertainValue> {
        let eval_ctx = EvalContext::with_bindings(ctx.precision_digits, ctx.bindings.clone())
            .with_uncertainties(ctx.uncertainties.clone())
            .with_intervals(ctx.intervals.clone())
            .with_branch_sheet(ctx.branch_sheet);
        eval_uncertain(self.graph, self.id, &eval_ctx)
    }

    /// Free variable names appearing in this expression.
    pub fn free_symbols(&self) -> BTreeSet<String> {
        let mut syms = BTreeSet::new();
        collect_expr_symbols(self.graph, self.id, &mut syms);
        syms.into_iter()
            .filter_map(|id| self.graph.symbols.resolve(id))
            .collect()
    }

    /// Check symbolic or high-confidence numerical equivalence between `self` and `other`.
    pub fn equiv(&self, other: &Self) -> bool {
        let diff = self.graph.sub(self.id, other.id);
        let zero = self.graph.integer(0);
        if let Ok(simp) = Simplifier::simplify(self.graph, diff)
            && simp == zero
        {
            return true;
        }
        let exp = AlgebraicTransformations::expand(self.graph, diff);
        if let Ok(simp_exp) = Simplifier::simplify(self.graph, exp)
            && simp_exp == zero
        {
            return true;
        }
        let free: Vec<String> = Self {
            graph: self.graph,
            id: diff,
        }
        .free_symbols()
        .into_iter()
        .filter(|n| !matches!(n.as_str(), "pi" | "π" | "e" | "i" | "I"))
        .collect();
        let seeds = [0.37_f64, 0.91, 1.43, 2.17];
        let mut matched = 0usize;
        for (p_idx, &base_val) in seeds.iter().enumerate() {
            let mut bindings = HashMap::new();
            for (v_idx, name) in free.iter().enumerate() {
                bindings.insert(
                    name.clone(),
                    base_val + 0.19 * (v_idx as f64) + 0.07 * (p_idx as f64),
                );
            }
            let ctx = EvalContext::with_bindings(15, bindings);
            if let (Ok(lv), Ok(rv)) = (
                self.graph.evalf(self.id, &ctx),
                self.graph.evalf(other.id, &ctx),
            ) {
                let (lr, li) = lv.as_complex();
                let (rr, ri) = rv.as_complex();
                if (lr - rr).hypot(li - ri) > 1e-6 {
                    return false;
                }
                matched += 1;
            }
        }
        matched > 0
    }

    /// Render expression in LaTeX notation.
    pub fn to_latex(&self) -> String {
        LatexFormatter
            .format(self.graph, self.id)
            .unwrap_or_default()
    }

    /// Render expression in 2D Unicode notation.
    pub fn to_unicode(&self) -> String {
        UnicodeFormatter
            .format(self.graph, self.id)
            .unwrap_or_default()
    }
}

impl<'g> IntoExpr<'g> for Expr<'g> {
    fn into_expr(self, _graph: &'g ExprGraph) -> Expr<'g> {
        self
    }
}
impl<'g> IntoExpr<'g> for &Expr<'g> {
    fn into_expr(self, _graph: &'g ExprGraph) -> Expr<'g> {
        *self
    }
}
impl<'g> IntoExpr<'g> for Sym<'g> {
    fn into_expr(self, _graph: &'g ExprGraph) -> Expr<'g> {
        self.as_expr()
    }
}
impl<'g> IntoExpr<'g> for &Sym<'g> {
    fn into_expr(self, _graph: &'g ExprGraph) -> Expr<'g> {
        self.as_expr()
    }
}
impl<'g> IntoExpr<'g> for i64 {
    fn into_expr(self, graph: &'g ExprGraph) -> Expr<'g> {
        Expr::int(graph, self)
    }
}
impl<'g> IntoExpr<'g> for i32 {
    fn into_expr(self, graph: &'g ExprGraph) -> Expr<'g> {
        Expr::int(graph, self as i64)
    }
}
impl<'g> IntoExpr<'g> for f64 {
    fn into_expr(self, graph: &'g ExprGraph) -> Expr<'g> {
        Expr::real(graph, self)
    }
}
impl<'g> IntoExpr<'g> for (i64, i64) {
    fn into_expr(self, graph: &'g ExprGraph) -> Expr<'g> {
        Expr::rational(graph, self.0, self.1)
    }
}
impl<'g> IntoExpr<'g> for ConstRational {
    fn into_expr(self, graph: &'g ExprGraph) -> Expr<'g> {
        self.intern(graph)
    }
}

impl fmt::Debug for Sym<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Sym({})", self.name())
    }
}
impl fmt::Display for Sym<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name())
    }
}
impl fmt::Debug for Expr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Expr({})", self.to_unicode())
    }
}
impl fmt::Display for Expr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_unicode())
    }
}

impl PartialEq for Expr<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for Expr<'_> {}

impl PartialEq for Sym<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.sym_id == other.sym_id
    }
}
impl Eq for Sym<'_> {}

macro_rules! impl_expr_bin_op {
    ($trait:ident, $method:ident, $graph_fn:ident, $rhs_ty:ty) => {
        impl<'g> $trait<$rhs_ty> for Expr<'g> {
            type Output = Expr<'g>;
            fn $method(self, rhs: $rhs_ty) -> Expr<'g> {
                let r = rhs.into_expr(self.graph);
                Expr {
                    graph: self.graph,
                    id: $graph_fn(self.graph, self.id, r.id),
                }
            }
        }
        impl<'g> $trait<$rhs_ty> for &Expr<'g> {
            type Output = Expr<'g>;
            fn $method(self, rhs: $rhs_ty) -> Expr<'g> {
                (*self).$method(rhs)
            }
        }
        impl<'g> $trait<$rhs_ty> for Sym<'g> {
            type Output = Expr<'g>;
            fn $method(self, rhs: $rhs_ty) -> Expr<'g> {
                self.as_expr().$method(rhs)
            }
        }
        impl<'g> $trait<$rhs_ty> for &Sym<'g> {
            type Output = Expr<'g>;
            fn $method(self, rhs: $rhs_ty) -> Expr<'g> {
                self.as_expr().$method(rhs)
            }
        }
    };
}

macro_rules! impl_scalar_lhs_op {
    ($trait:ident, $method:ident, $graph_fn:ident, $scalar:ty) => {
        impl<'g> $trait<Expr<'g>> for $scalar {
            type Output = Expr<'g>;
            fn $method(self, rhs: Expr<'g>) -> Expr<'g> {
                let l = self.into_expr(rhs.graph);
                Expr {
                    graph: rhs.graph,
                    id: $graph_fn(rhs.graph, l.id, rhs.id),
                }
            }
        }
        impl<'g> $trait<&Expr<'g>> for $scalar {
            type Output = Expr<'g>;
            fn $method(self, rhs: &Expr<'g>) -> Expr<'g> {
                $trait::<Expr<'g>>::$method(self, *rhs)
            }
        }
        impl<'g> $trait<Sym<'g>> for $scalar {
            type Output = Expr<'g>;
            fn $method(self, rhs: Sym<'g>) -> Expr<'g> {
                $trait::<Expr<'g>>::$method(self, rhs.as_expr())
            }
        }
        impl<'g> $trait<&Sym<'g>> for $scalar {
            type Output = Expr<'g>;
            fn $method(self, rhs: &Sym<'g>) -> Expr<'g> {
                $trait::<Expr<'g>>::$method(self, rhs.as_expr())
            }
        }
    };
}

#[inline]
fn g_add(g: &ExprGraph, a: ExprId, b: ExprId) -> ExprId {
    g.add([a, b])
}
#[inline]
fn g_sub(g: &ExprGraph, a: ExprId, b: ExprId) -> ExprId {
    g.sub(a, b)
}
#[inline]
fn g_mul(g: &ExprGraph, a: ExprId, b: ExprId) -> ExprId {
    g.mul([a, b])
}
#[inline]
fn g_div(g: &ExprGraph, a: ExprId, b: ExprId) -> ExprId {
    g.div(a, b)
}

macro_rules! impl_all_ops_for_op {
    ($trait:ident, $method:ident, $graph_fn:ident) => {
        impl_expr_bin_op!($trait, $method, $graph_fn, Expr<'g>);
        impl_expr_bin_op!($trait, $method, $graph_fn, &Expr<'g>);
        impl_expr_bin_op!($trait, $method, $graph_fn, Sym<'g>);
        impl_expr_bin_op!($trait, $method, $graph_fn, &Sym<'g>);
        impl_expr_bin_op!($trait, $method, $graph_fn, i64);
        impl_expr_bin_op!($trait, $method, $graph_fn, f64);
        impl_expr_bin_op!($trait, $method, $graph_fn, ConstRational);
        impl_scalar_lhs_op!($trait, $method, $graph_fn, i64);
        impl_scalar_lhs_op!($trait, $method, $graph_fn, f64);
        impl_scalar_lhs_op!($trait, $method, $graph_fn, ConstRational);
    };
}

impl_all_ops_for_op!(Add, add, g_add);
impl_all_ops_for_op!(Sub, sub, g_sub);
impl_all_ops_for_op!(Mul, mul, g_mul);
impl_all_ops_for_op!(Div, div, g_div);

impl<'g> Neg for Expr<'g> {
    type Output = Expr<'g>;
    fn neg(self) -> Expr<'g> {
        Expr {
            graph: self.graph,
            id: self.graph.neg(self.id),
        }
    }
}
impl<'g> Neg for &Expr<'g> {
    type Output = Expr<'g>;
    fn neg(self) -> Expr<'g> {
        -(*self)
    }
}
impl<'g> Neg for Sym<'g> {
    type Output = Expr<'g>;
    fn neg(self) -> Expr<'g> {
        -(self.as_expr())
    }
}
impl<'g> Neg for &Sym<'g> {
    type Output = Expr<'g>;
    fn neg(self) -> Expr<'g> {
        -(self.as_expr())
    }
}

/// Shorthand constructor for a single symbolic variable.
pub fn sym<'g>(graph: &'g ExprGraph, name: &str) -> Sym<'g> {
    Sym::new(graph, name)
}

/// Shorthand constructor for parsing an expression string into [`Expr<'g>`].
pub fn parse_expr<'g>(graph: &'g ExprGraph, raw: &str) -> AlgebraResult<Expr<'g>> {
    Expr::parse(graph, raw)
}

// =========================================================================
// Symbolic Matrix (`SymMat<'g>`) with Immutable Views & Mutable In-Place Ops
// =========================================================================

/// Ergonomic lifetime-bound symbolic matrix supporting both immutable algebra and in-place mutation.
#[derive(Clone)]
pub struct SymMat<'g> {
    graph: &'g ExprGraph,
    inner: SymbolicMatrix,
}

impl<'g> SymMat<'g> {
    /// Wrap a [`SymbolicMatrix`].
    pub fn from_inner(graph: &'g ExprGraph, inner: SymbolicMatrix) -> Self {
        Self { graph, inner }
    }

    /// Borrow underlying [`SymbolicMatrix`].
    pub fn as_inner(&self) -> &SymbolicMatrix {
        &self.inner
    }

    /// Parse a matrix literal (e.g. `"[[1, 2], [3, 4]]"` or semicolon-delimited `"[1 2; 3 4]"`).
    pub fn parse(graph: &'g ExprGraph, raw: &str) -> AlgebraResult<Self> {
        let mat = SymbolicMatrix::from_expr_str(graph, raw)?;
        Ok(Self { graph, inner: mat })
    }

    /// Construct from row slices of expressions.
    pub fn from_rows(graph: &'g ExprGraph, rows: &[&[Expr<'g>]]) -> AlgebraResult<Self> {
        if rows.is_empty() {
            return Ok(Self {
                graph,
                inner: SymbolicMatrix::new(0, 0, Vec::new())?,
            });
        }
        let n_cols = rows[0].len();
        let mut elems = Vec::with_capacity(rows.len() * n_cols);
        for row in rows {
            if row.len() != n_cols {
                return Err(AlgebraError::DimensionMismatch(format!(
                    "Ragged matrix row: expected {} cols, got {}",
                    n_cols,
                    row.len()
                )));
            }
            elems.extend(row.iter().map(|e| e.id));
        }
        Ok(Self {
            graph,
            inner: SymbolicMatrix::new(rows.len(), n_cols, elems)?,
        })
    }

    /// Construct from a fixed-size `const` array of floats.
    pub fn from_const<const R: usize, const C: usize>(
        graph: &'g ExprGraph,
        data: &[[f64; C]; R],
    ) -> AlgebraResult<Self> {
        let mut elems = Vec::with_capacity(R * C);
        for row in data {
            for &val in row {
                let id = if val.fract() == 0.0 && val.abs() < 9e15 {
                    graph.integer(val as i64)
                } else {
                    graph.float(val)
                };
                elems.push(id);
            }
        }
        Ok(Self {
            graph,
            inner: SymbolicMatrix::new(R, C, elems)?,
        })
    }

    /// Matrix of all zeros (`rows x cols`).
    pub fn zeros(graph: &'g ExprGraph, rows: usize, cols: usize) -> AlgebraResult<Self> {
        let z = graph.integer(0);
        Ok(Self {
            graph,
            inner: SymbolicMatrix::new(rows, cols, vec![z; rows * cols])?,
        })
    }

    /// Matrix of all ones (`rows x cols`).
    pub fn ones(graph: &'g ExprGraph, rows: usize, cols: usize) -> AlgebraResult<Self> {
        let one = graph.integer(1);
        Ok(Self {
            graph,
            inner: SymbolicMatrix::new(rows, cols, vec![one; rows * cols])?,
        })
    }

    /// Identity matrix $I_n$.
    pub fn eye(graph: &'g ExprGraph, n: usize) -> AlgebraResult<Self> {
        let z = graph.integer(0);
        let one = graph.integer(1);
        let mut elems = vec![z; n * n];
        for i in 0..n {
            elems[i * n + i] = one;
        }
        Ok(Self {
            graph,
            inner: SymbolicMatrix::new(n, n, elems)?,
        })
    }

    /// Diagonal matrix from slice of entries.
    pub fn diag(graph: &'g ExprGraph, diag_entries: &[Expr<'g>]) -> AlgebraResult<Self> {
        let n = diag_entries.len();
        let z = graph.integer(0);
        let mut elems = vec![z; n * n];
        for (i, d) in diag_entries.iter().enumerate() {
            elems[i * n + i] = d.id;
        }
        Ok(Self {
            graph,
            inner: SymbolicMatrix::new(n, n, elems)?,
        })
    }

    pub fn rows(&self) -> usize {
        self.inner.rows
    }
    pub fn cols(&self) -> usize {
        self.inner.cols
    }
    pub fn shape(&self) -> (usize, usize) {
        (self.inner.rows, self.inner.cols)
    }

    /// Immutable entry access returning [`Expr<'g>`].
    pub fn get(&self, r: usize, c: usize) -> AlgebraResult<Expr<'g>> {
        if r >= self.inner.rows || c >= self.inner.cols {
            return Err(AlgebraError::DimensionMismatch(format!(
                "Matrix index ({r}, {c}) out of bounds for {}x{}",
                self.inner.rows, self.inner.cols
            )));
        }
        Ok(Expr::from_id(self.graph, self.inner.get(r, c)))
    }

    /// Extract row slice `r`.
    pub fn row(&self, r: usize) -> AlgebraResult<Vec<Expr<'g>>> {
        if r >= self.inner.rows {
            return Err(AlgebraError::DimensionMismatch(format!(
                "Row {r} out of bounds"
            )));
        }
        Ok((0..self.inner.cols)
            .map(|c| Expr::from_id(self.graph, self.inner.get(r, c)))
            .collect())
    }

    /// Extract column slice `c`.
    pub fn col(&self, c: usize) -> AlgebraResult<Vec<Expr<'g>>> {
        if c >= self.inner.cols {
            return Err(AlgebraError::DimensionMismatch(format!(
                "Column {c} out of bounds"
            )));
        }
        Ok((0..self.inner.rows)
            .map(|r| Expr::from_id(self.graph, self.inner.get(r, c)))
            .collect())
    }

    /// Mutable in-place entry assignment (`&mut self`).
    pub fn set<V: IntoExpr<'g>>(&mut self, r: usize, c: usize, val: V) -> AlgebraResult<()> {
        if r >= self.inner.rows || c >= self.inner.cols {
            return Err(AlgebraError::DimensionMismatch(format!(
                "Matrix index ({r}, {c}) out of bounds for {}x{}",
                self.inner.rows, self.inner.cols
            )));
        }
        let v = val.into_expr(self.graph);
        let cols = self.inner.cols;
        self.inner.elements[r * cols + c] = v.id;
        Ok(())
    }

    /// Mutable in-place row swap (`&mut self`).
    pub fn swap_rows(&mut self, r1: usize, r2: usize) -> AlgebraResult<()> {
        if r1 >= self.inner.rows || r2 >= self.inner.rows {
            return Err(AlgebraError::DimensionMismatch(
                "Row swap index out of bounds".into(),
            ));
        }
        if r1 != r2 {
            let cols = self.inner.cols;
            for c in 0..cols {
                self.inner.elements.swap(r1 * cols + c, r2 * cols + c);
            }
        }
        Ok(())
    }

    /// Mutable in-place row scaling $R_r \leftarrow k \cdot R_r$ (`&mut self`).
    pub fn scale_row<K: IntoExpr<'g>>(&mut self, r: usize, factor: K) -> AlgebraResult<()> {
        if r >= self.inner.rows {
            return Err(AlgebraError::DimensionMismatch(
                "Row index out of bounds".into(),
            ));
        }
        let k = factor.into_expr(self.graph);
        let cols = self.inner.cols;
        for c in 0..cols {
            let cur = self.inner.elements[r * cols + c];
            let prod = self.graph.mul([cur, k.id]);
            self.inner.elements[r * cols + c] =
                Simplifier::simplify(self.graph, prod).unwrap_or(prod);
        }
        Ok(())
    }

    /// Mutable in-place elementary row addition $R_{\text{dst}} \leftarrow R_{\text{dst}} + k \cdot R_{\text{src}}$.
    pub fn add_scaled_row<K: IntoExpr<'g>>(
        &mut self,
        dst: usize,
        src: usize,
        factor: K,
    ) -> AlgebraResult<()> {
        if dst >= self.inner.rows || src >= self.inner.rows {
            return Err(AlgebraError::DimensionMismatch(
                "Row index out of bounds".into(),
            ));
        }
        let k = factor.into_expr(self.graph);
        let cols = self.inner.cols;
        for c in 0..cols {
            let src_val = self.inner.elements[src * cols + c];
            let term = self.graph.mul([k.id, src_val]);
            let dst_val = self.inner.elements[dst * cols + c];
            let sum = self.graph.add([dst_val, term]);
            self.inner.elements[dst * cols + c] =
                Simplifier::simplify(self.graph, sum).unwrap_or(sum);
        }
        Ok(())
    }

    /// Simplify every matrix entry in place (`&mut self`).
    pub fn simplify_in_place(&mut self) {
        for elem in &mut self.inner.elements {
            *elem = Simplifier::simplify(self.graph, *elem).unwrap_or(*elem);
        }
    }

    pub fn try_add(&self, rhs: &Self) -> AlgebraResult<Self> {
        let sum = self.inner.add(self.graph, &rhs.inner)?;
        Ok(Self::from_inner(self.graph, sum))
    }

    pub fn try_sub(&self, rhs: &Self) -> AlgebraResult<Self> {
        let neg_rhs_elems: Vec<ExprId> = rhs
            .inner
            .elements
            .iter()
            .map(|&e| self.graph.neg(e))
            .collect();
        let neg_rhs = SymbolicMatrix::new(rhs.inner.rows, rhs.inner.cols, neg_rhs_elems)?;
        let diff = self.inner.add(self.graph, &neg_rhs)?;
        Ok(Self::from_inner(self.graph, diff))
    }

    pub fn try_mul(&self, rhs: &Self) -> AlgebraResult<Self> {
        let prod = self.inner.multiply(self.graph, &rhs.inner)?;
        Ok(Self::from_inner(self.graph, prod))
    }

    pub fn transpose(&self) -> AlgebraResult<Self> {
        Ok(Self::from_inner(
            self.graph,
            self.inner.transpose(self.graph)?,
        ))
    }

    pub fn det(&self) -> AlgebraResult<Expr<'g>> {
        let d = self.inner.determinant(self.graph)?;
        Ok(Expr::from_id(
            self.graph,
            Simplifier::simplify(self.graph, d).unwrap_or(d),
        ))
    }

    pub fn inv(&self) -> AlgebraResult<Self> {
        let mut res = Self::from_inner(self.graph, self.inner.inverse(self.graph)?);
        res.simplify_in_place();
        Ok(res)
    }

    pub fn trace(&self) -> AlgebraResult<Expr<'g>> {
        let tr = self.inner.trace(self.graph)?;
        Ok(Expr::from_id(
            self.graph,
            Simplifier::simplify(self.graph, tr).unwrap_or(tr),
        ))
    }

    pub fn rref(&self) -> AlgebraResult<(Self, Vec<usize>)> {
        let (mat, pivots) = self.inner.rref(self.graph)?;
        Ok((Self::from_inner(self.graph, mat), pivots))
    }

    pub fn rank(&self) -> AlgebraResult<usize> {
        self.inner.rank(self.graph)
    }

    pub fn nullspace(&self) -> AlgebraResult<Vec<Vec<Expr<'g>>>> {
        let basis = self.inner.nullspace(self.graph)?;
        Ok(basis
            .into_iter()
            .map(|col| {
                col.into_iter()
                    .map(|id| Expr::from_id(self.graph, id))
                    .collect()
            })
            .collect())
    }

    pub fn charpoly(&self, lambda: &Sym<'g>) -> AlgebraResult<Expr<'g>> {
        let cp = self.inner.charpoly(self.graph, &lambda.name())?;
        Ok(Expr::from_id(
            self.graph,
            Simplifier::simplify(self.graph, cp).unwrap_or(cp),
        ))
    }

    pub fn eigvals(&self) -> AlgebraResult<Vec<Expr<'g>>> {
        Ok(self
            .inner
            .eigenvalues(self.graph)?
            .into_iter()
            .map(|id| Expr::from_id(self.graph, id))
            .collect())
    }

    /// Solve linear system $A \mathbf{x} = \mathbf{b}$ cleanly via exact symbolic Gaussian elimination.
    pub fn solve(&self, b_rhs: &[Expr<'g>]) -> AlgebraResult<Vec<Expr<'g>>> {
        let b_ids: Vec<ExprId> = b_rhs.iter().map(|e| e.id).collect();
        let space = LinearSystemSolver::solve(self.graph, &self.inner, &b_ids)?;
        if space.has_infinite_solutions {
            return Err(AlgebraError::EvaluationError(
                "Linear system is underdetermined; use `LinearSystemSolver::solve` for full affine space".into(),
            ));
        }
        Ok(space
            .particular
            .into_iter()
            .map(|id| Expr::from_id(self.graph, id))
            .collect())
    }

    pub fn to_latex(&self) -> String {
        LatexFormatter
            .format(self.graph, self.inner.to_expr_id(self.graph))
            .unwrap_or_default()
    }

    pub fn to_unicode(&self) -> String {
        UnicodeFormatter
            .format(self.graph, self.inner.to_expr_id(self.graph))
            .unwrap_or_default()
    }
}

impl fmt::Debug for SymMat<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SymMat({}x{}: {})",
            self.rows(),
            self.cols(),
            self.to_unicode()
        )
    }
}
impl fmt::Display for SymMat<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_unicode())
    }
}

impl<'g> Add<&SymMat<'g>> for &SymMat<'g> {
    type Output = SymMat<'g>;
    fn add(self, rhs: &SymMat<'g>) -> SymMat<'g> {
        self.try_add(rhs)
            .unwrap_or_else(|e| panic!("SymMat add failed: {e}"))
    }
}
impl<'g> Sub<&SymMat<'g>> for &SymMat<'g> {
    type Output = SymMat<'g>;
    fn sub(self, rhs: &SymMat<'g>) -> SymMat<'g> {
        self.try_sub(rhs)
            .unwrap_or_else(|e| panic!("SymMat sub failed: {e}"))
    }
}
impl<'g> Mul<&SymMat<'g>> for &SymMat<'g> {
    type Output = SymMat<'g>;
    fn mul(self, rhs: &SymMat<'g>) -> SymMat<'g> {
        self.try_mul(rhs)
            .unwrap_or_else(|e| panic!("SymMat mul failed: {e}"))
    }
}

// =========================================================================
// Dense Numerical Array / Matrix (`NumMat`) with Slice Views & Indexing
// =========================================================================

/// Owned dense numerical matrix & vector container with grid constructors, `&`/`&mut` views, and indexing.
#[derive(Debug, Clone)]
pub struct NumMat {
    buf: MatrixBuffer,
}

impl NumMat {
    pub fn zeros(rows: usize, cols: usize) -> Self {
        Self {
            buf: MatrixBuffer::zeros(rows, cols),
        }
    }

    pub fn ones(rows: usize, cols: usize) -> AlgebraResult<Self> {
        Ok(Self {
            buf: MatrixBuffer::new(rows, cols, vec![1.0; rows * cols])?,
        })
    }

    pub fn eye(n: usize) -> Self {
        Self {
            buf: MatrixBuffer::identity(n),
        }
    }

    pub fn from_rows(rows: &[&[f64]]) -> AlgebraResult<Self> {
        if rows.is_empty() {
            return Ok(Self::zeros(0, 0));
        }
        let cols = rows[0].len();
        let mut data = Vec::with_capacity(rows.len() * cols);
        for row in rows {
            if row.len() != cols {
                return Err(AlgebraError::DimensionMismatch(
                    "Ragged row in NumMat::from_rows".into(),
                ));
            }
            data.extend_from_slice(row);
        }
        Ok(Self {
            buf: MatrixBuffer::new(rows.len(), cols, data)?,
        })
    }

    /// Generate `n` linearly spaced samples over `[start, end]` as a `1 x n` row vector.
    pub fn linspace(start: f64, end: f64, n: usize) -> Self {
        let mut data = Vec::with_capacity(n);
        if n == 1 {
            data.push(start);
        } else if n > 1 {
            let step = (end - start) / ((n - 1) as f64);
            for i in 0..n {
                data.push(start + step * (i as f64));
            }
        }
        Self {
            buf: MatrixBuffer::new(1, n, data).unwrap_or_else(|_| MatrixBuffer::zeros(1, 0)),
        }
    }

    /// Generate samples `[start, start + step, ...)` strictly below `stop`.
    pub fn arange(start: f64, stop: f64, step: f64) -> AlgebraResult<Self> {
        if step == 0.0 || !step.is_finite() {
            return Err(AlgebraError::EvaluationError(
                "arange step must be finite and non-zero".into(),
            ));
        }
        let mut data = Vec::new();
        let mut cur = start;
        let max_len = 1_000_000usize;
        while ((step > 0.0 && cur < stop - 1e-12) || (step < 0.0 && cur > stop + 1e-12))
            && data.len() < max_len
        {
            data.push(cur);
            cur += step;
        }
        let len = data.len();
        Ok(Self {
            buf: MatrixBuffer::new(1, len, data)?,
        })
    }

    pub fn rows(&self) -> usize {
        self.buf.rows
    }
    pub fn cols(&self) -> usize {
        self.buf.cols
    }
    pub fn as_slice(&self) -> &[f64] {
        &self.buf.data
    }
    pub fn as_mut_slice(&mut self) -> &mut [f64] {
        &mut self.buf.data
    }
    pub fn rref(&self, tol: f64) -> (Self, Vec<usize>) {
        let (r, p) = self.buf.rref(tol);
        (Self { buf: r }, p)
    }
    pub fn rank(&self, tol: f64) -> usize {
        self.buf.rank(tol)
    }
    pub fn nullspace(&self, tol: f64) -> Vec<Vec<f64>> {
        self.buf.nullspace(tol)
    }
    pub fn norm_fro(&self) -> f64 {
        self.buf.data.iter().map(|&v| v * v).sum::<f64>().sqrt()
    }
}

impl Index<(usize, usize)> for NumMat {
    type Output = f64;
    fn index(&self, (r, c): (usize, usize)) -> &f64 {
        &self.buf.data[r * self.buf.cols + c]
    }
}

impl IndexMut<(usize, usize)> for NumMat {
    fn index_mut(&mut self, (r, c): (usize, usize)) -> &mut f64 {
        &mut self.buf.data[r * self.buf.cols + c]
    }
}
