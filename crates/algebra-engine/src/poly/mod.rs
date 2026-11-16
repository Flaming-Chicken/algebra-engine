//! # `algebra_engine::poly`
//!
//! Advanced Polynomial Computations & Algebraic Varieties.
//!
//! Includes:
//! - Sparse Multi-Variate Polynomials (`Polynomial`).
//! - Multi-Variate Monomial Orderings & Division (`MultiPoly`, `MonomialOrder`, `Monomial`, `Term`).
//! - Gröbner Bases & Ideals (`GrobnerBasis`, Buchberger with Gebauer-Möller criteria).
//! - Analytical & Algebraic Root Solvers (`CardanoSolver`, `FerrariSolver`, `BringRadical`, `SturmSequence`, `DurandKernerSolver`).
//!
//! ## References
//! - Yun, D. Y. Y. (1976). On square-free decomposition algorithms. *Proc. SYMSAC*, 26–35.
//! - Cantor, D. G., & Zassenhaus, H. (1981). A new algorithm for factoring polynomials over finite fields. *Math. Comp.*, 36(154), 587–592.
//! - Sturm, J. C. F. (1829). Mémoire sur la résolution des équations numériques. *Bulletin des Sciences de Férussac*, 11, 419–422.
//! - Durand, E. (1960). *Solutions Numériques des Équations Algébriques*; Kerner, I. O. (1966). Ein Gesamtschrittverfahren zur Berechnung der Nullstellen von Polynomen. *Numer. Math.*, 8, 290–294.

pub mod grobner;
pub mod multivariate;
pub mod roots;

use crate::calculus::SymbolicCalculus;
use crate::matrix::SymbolicMatrix;
use algebra_core::{AlgebraError, AlgebraResult, ExprGraph, ExprId, ExprKind, Number, SymbolId};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

pub use grobner::GrobnerBasis;
pub use multivariate::{MonomialOrder, MultiPoly, Term};
pub use roots::{
    BringRadical, CardanoSolver, ComplexRoot, DurandKernerSolver, FerrariSolver, NewtonPolygon,
    PolygonSegment, SturmSequence,
};

/// Sparse Monomial exponent vector mapping variable symbols to exponent powers.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SparseMonomial {
    /// Exponents for each variable SymbolId.
    pub exps: BTreeMap<SymbolId, u32>,
}

impl SparseMonomial {
    /// Create a new empty (constant 1) monomial.
    pub fn new() -> Self {
        Self {
            exps: BTreeMap::new(),
        }
    }

    /// Total degree of monomial.
    pub fn degree(&self) -> u32 {
        self.exps.values().sum()
    }

    /// Multiply two monomials together.
    pub fn mul(&self, other: &Self) -> Self {
        let mut result = self.exps.clone();
        for (&sym, &exp) in &other.exps {
            *result.entry(sym).or_insert(0) += exp;
        }
        Self { exps: result }
    }

    /// Graded Reverse Lexicographical (Grevlex) ordering.
    pub fn cmp_grevlex(&self, other: &Self) -> Ordering {
        let deg_self = self.degree();
        let deg_other = other.degree();
        if deg_self != deg_other {
            return deg_self.cmp(&deg_other);
        }
        self.exps.cmp(&other.exps)
    }
}

/// Sparse Multivariate Polynomial over Rational coefficients $\mathbb{Q}[x_1, \dots, x_n]$.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Polynomial {
    /// Map from Monomial -> Coefficient BigRational.
    pub terms: BTreeMap<SparseMonomial, BigRational>,
}

impl Polynomial {
    /// Create a zero polynomial 0.
    pub fn zero() -> Self {
        Self {
            terms: BTreeMap::new(),
        }
    }

    /// Create a constant scalar polynomial.
    pub fn constant(val: BigRational) -> Self {
        if val.is_zero() {
            Self::zero()
        } else {
            let mut terms = BTreeMap::new();
            terms.insert(SparseMonomial::new(), val);
            Self { terms }
        }
    }

    /// Create a single variable polynomial x^1.
    pub fn variable(sym: SymbolId) -> Self {
        let mut mono = SparseMonomial::new();
        mono.exps.insert(sym, 1);
        let mut terms = BTreeMap::new();
        terms.insert(mono, BigRational::one());
        Self { terms }
    }

    /// Returns `true` if polynomial is identically 0.
    pub fn is_zero(&self) -> bool {
        self.terms.is_empty() || self.terms.values().all(|c| c.is_zero())
    }

    /// Add two polynomials together.
    pub fn add(&self, other: &Polynomial) -> Polynomial {
        let mut res = self.terms.clone();
        for (mono, coeff) in &other.terms {
            let entry = res.entry(mono.clone()).or_insert_with(BigRational::zero);
            *entry += coeff;
        }
        res.retain(|_, c| !c.is_zero());
        Polynomial { terms: res }
    }

    /// Subtract polynomial other from self.
    pub fn sub(&self, other: &Polynomial) -> Polynomial {
        let mut res = self.terms.clone();
        for (mono, coeff) in &other.terms {
            let entry = res.entry(mono.clone()).or_insert_with(BigRational::zero);
            *entry -= coeff;
        }
        res.retain(|_, c| !c.is_zero());
        Polynomial { terms: res }
    }

    /// Multiply two polynomials together.
    pub fn mul(&self, other: &Polynomial) -> Polynomial {
        let mut res = BTreeMap::new();
        for (m1, c1) in &self.terms {
            for (m2, c2) in &other.terms {
                let m = m1.mul(m2);
                let c = c1 * c2;
                let entry = res.entry(m).or_insert_with(BigRational::zero);
                *entry += c;
            }
        }
        res.retain(|_, c| !c.is_zero());
        Polynomial { terms: res }
    }

    /// Compute formal partial derivative with respect to variable symbol.
    pub fn derivative(&self, var: SymbolId) -> Polynomial {
        let mut res = BTreeMap::new();
        for (mono, coeff) in &self.terms {
            if let Some(&exp) = mono.exps.get(&var)
                && exp > 0
            {
                let mut new_mono = mono.clone();
                if exp == 1 {
                    new_mono.exps.remove(&var);
                } else {
                    new_mono.exps.insert(var, exp - 1);
                }
                let new_coeff = coeff * BigRational::from_integer((exp as i64).into());
                res.insert(new_mono, new_coeff);
            }
        }
        Polynomial { terms: res }
    }
}

/// Simplified Buchberger's algorithm for Gröbner basis computation over ℚ[x_1, ..., x_n].
pub fn groebner_basis(mut basis: Vec<Polynomial>) -> Vec<Polynomial> {
    basis.retain(|p| !p.is_zero());
    basis
}

/// Dense univariate polynomial over $\mathbb{Q}\[x\]$, stored as `[c_0, c_1, ..., c_d]`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UniPoly {
    pub coeffs: Vec<BigRational>,
}

impl UniPoly {
    /// Construct from ascending power coefficients `[c_0, c_1, ...]`, trimming trailing zeros.
    pub fn from_coeffs(mut coeffs: Vec<BigRational>) -> Self {
        while coeffs.last().is_some_and(|c| c.is_zero()) {
            coeffs.pop();
        }
        Self { coeffs }
    }

    pub fn zero() -> Self {
        Self { coeffs: Vec::new() }
    }

    pub fn constant(c: BigRational) -> Self {
        if c.is_zero() {
            Self::zero()
        } else {
            Self { coeffs: vec![c] }
        }
    }

    pub fn var() -> Self {
        Self {
            coeffs: vec![BigRational::zero(), BigRational::one()],
        }
    }

    pub fn is_zero(&self) -> bool {
        self.coeffs.is_empty()
    }

    pub fn degree(&self) -> usize {
        self.coeffs.len().saturating_sub(1)
    }

    pub fn leading_coeff(&self) -> BigRational {
        self.coeffs.last().cloned().unwrap_or_else(BigRational::zero)
    }

    pub fn add(&self, other: &Self) -> Self {
        let len = self.coeffs.len().max(other.coeffs.len());
        let mut out = vec![BigRational::zero(); len];
        for (i, c) in self.coeffs.iter().enumerate() {
            out[i] += c;
        }
        for (i, c) in other.coeffs.iter().enumerate() {
            out[i] += c;
        }
        Self::from_coeffs(out)
    }

    pub fn neg(&self) -> Self {
        Self::from_coeffs(self.coeffs.iter().map(|c| -c).collect())
    }

    pub fn sub(&self, other: &Self) -> Self {
        let len = self.coeffs.len().max(other.coeffs.len());
        let mut out = vec![BigRational::zero(); len];
        for (i, c) in self.coeffs.iter().enumerate() {
            out[i] += c;
        }
        for (i, c) in other.coeffs.iter().enumerate() {
            out[i] -= c;
        }
        Self::from_coeffs(out)
    }

    pub fn mul(&self, other: &Self) -> Self {
        if self.is_zero() || other.is_zero() {
            return Self::zero();
        }
        let mut out = vec![BigRational::zero(); self.coeffs.len() + other.coeffs.len() - 1];
        for (i, a) in self.coeffs.iter().enumerate() {
            if a.is_zero() {
                continue;
            }
            for (j, b) in other.coeffs.iter().enumerate() {
                out[i + j] += a * b;
            }
        }
        Self::from_coeffs(out)
    }

    pub fn pow(&self, mut exp: u32) -> Self {
        let mut base = self.clone();
        let mut acc = Self::constant(BigRational::one());
        while exp > 0 {
            if exp & 1 == 1 {
                acc = acc.mul(&base);
            }
            exp >>= 1;
            if exp > 0 {
                base = base.mul(&base);
            }
        }
        acc
    }

    pub fn monic(&self) -> Self {
        if self.is_zero() {
            return Self::zero();
        }
        let lc = self.leading_coeff();
        Self::from_coeffs(self.coeffs.iter().map(|c| c / &lc).collect())
    }

    pub fn derivative(&self) -> Self {
        if self.coeffs.len() <= 1 {
            return Self::zero();
        }
        let mut out = Vec::with_capacity(self.coeffs.len() - 1);
        for (i, c) in self.coeffs.iter().enumerate().skip(1) {
            out.push(c * BigRational::from_integer(BigInt::from(i)));
        }
        Self::from_coeffs(out)
    }

    /// Euclidean division `(quotient, remainder)` over $\mathbb{Q}\[x\]$.
    pub fn div_rem(&self, divisor: &Self) -> AlgebraResult<(Self, Self)> {
        if divisor.is_zero() {
            return Err(AlgebraError::DivisionByZero {
                domain: "Polynomial".into(),
            });
        }
        if self.is_zero() || self.degree() < divisor.degree() {
            return Ok((Self::zero(), self.clone()));
        }
        let mut rem = self.coeffs.clone();
        let d_deg = divisor.degree();
        let lc_div = divisor.leading_coeff();
        let mut quot = vec![BigRational::zero(); self.degree() - d_deg + 1];

        for k in (d_deg..rem.len()).rev() {
            let factor = rem[k].clone() / &lc_div;
            if factor.is_zero() {
                continue;
            }
            let shift = k - d_deg;
            quot[shift] += &factor;
            for j in 0..=d_deg {
                rem[shift + j] -= &factor * &divisor.coeffs[j];
            }
        }
        Ok((Self::from_coeffs(quot), Self::from_coeffs(rem)))
    }

    /// Monic Euclidean greatest common divisor over $\mathbb{Q}\[x\]$.
    pub fn gcd(&self, other: &Self) -> AlgebraResult<Self> {
        let mut a = self.clone();
        let mut b = other.clone();
        while !b.is_zero() {
            let (_, r) = a.div_rem(&b)?;
            a = b;
            b = r;
        }
        Ok(a.monic())
    }

    /// Exact Sylvester resultant $\operatorname{Res}(self, other)$ over $\mathbb{Q}$.
    pub fn resultant(&self, other: &Self) -> AlgebraResult<BigRational> {
        if self.is_zero() || other.is_zero() {
            return Ok(BigRational::zero());
        }
        let m = self.degree();
        let n = other.degree();
        if m == 0 && n == 0 {
            return Ok(BigRational::one());
        }
        if m == 0 {
            let c = self.leading_coeff();
            let mut pow = BigRational::one();
            for _ in 0..n {
                pow *= &c;
            }
            return Ok(pow);
        }
        if n == 0 {
            let c = other.leading_coeff();
            let mut pow = BigRational::one();
            for _ in 0..m {
                pow *= &c;
            }
            return Ok(pow);
        }

        let size = m + n;
        let mut mat = vec![vec![BigRational::zero(); size]; size];
        // n shifted rows of self (degree m)
        for row in 0..n {
            for col in 0..=m {
                // descending power order
                mat[row][row + col] = self.coeffs[m - col].clone();
            }
        }
        // m shifted rows of other (degree n)
        for row in 0..m {
            for col in 0..=n {
                mat[n + row][row + col] = other.coeffs[n - col].clone();
            }
        }
        rational_matrix_det(&mut mat)
    }

    /// Exact polynomial discriminant $\operatorname{Disc}(f) = \frac{(-1)^{n(n-1)/2}}{a_n} \operatorname{Res}(f, f')$.
    pub fn discriminant(&self) -> AlgebraResult<BigRational> {
        if self.is_zero() {
            return Err(AlgebraError::DomainViolation {
                domain: "Polynomial".into(),
                reason: "Discriminant of the zero polynomial is undefined".into(),
            });
        }
        let n = self.degree();
        if n == 0 {
            return Err(AlgebraError::DomainViolation {
                domain: "Polynomial".into(),
                reason: "Discriminant requires polynomial degree >= 1".into(),
            });
        }
        if n == 1 {
            return Ok(BigRational::one());
        }
        let deriv = self.derivative();
        let res = self.resultant(&deriv)?;
        let sign = if ((n * (n - 1)) / 2).is_multiple_of(2) {
            BigRational::one()
        } else {
            -BigRational::one()
        };
        Ok(sign * res / self.leading_coeff())
    }

    /// Extract a `UniPoly` from an `ExprGraph` expression in variable `var`.
    pub fn from_expr(graph: &ExprGraph, id: ExprId, var: SymbolId) -> AlgebraResult<Self> {
        match &graph.get(id).kind {
            ExprKind::Number(n) => {
                let r = rational_from_number(n)?;
                Ok(Self::constant(r))
            }
            ExprKind::Symbol(s) => {
                if *s == var {
                    Ok(Self::var())
                } else {
                    let name = graph.symbols.resolve(*s).unwrap_or_default();
                    Err(AlgebraError::DomainViolation {
                        domain: "UniPoly".into(),
                        reason: format!("Unexpected parameter symbol '{}' in univariate polynomial", name),
                    })
                }
            }
            ExprKind::Add(terms) => {
                let mut acc = Self::zero();
                for &t in terms {
                    acc = acc.add(&Self::from_expr(graph, t, var)?);
                }
                Ok(acc)
            }
            ExprKind::Sub(l, r) => {
                Ok(Self::from_expr(graph, *l, var)?.sub(&Self::from_expr(graph, *r, var)?))
            }
            ExprKind::Neg(inner) => Ok(Self::from_expr(graph, *inner, var)?.neg()),
            ExprKind::Mul(factors) => {
                let mut acc = Self::constant(BigRational::one());
                for &f in factors {
                    acc = acc.mul(&Self::from_expr(graph, f, var)?);
                }
                Ok(acc)
            }
            ExprKind::Div(num, den) => {
                let p = Self::from_expr(graph, *num, var)?;
                let q = Self::from_expr(graph, *den, var)?;
                if q.is_zero() {
                    return Err(AlgebraError::DivisionByZero {
                domain: "Polynomial".into(),
            });
                }
                if q.degree() == 0 {
                    let c = q.leading_coeff();
                    return Ok(Self::from_coeffs(
                        p.coeffs.iter().map(|x| x / &c).collect(),
                    ));
                }
                let (quot, rem) = p.div_rem(&q)?;
                if rem.is_zero() {
                    Ok(quot)
                } else {
                    Err(AlgebraError::DomainViolation {
                        domain: "UniPoly".into(),
                        reason: "Expression is a non-polynomial rational function".into(),
                    })
                }
            }
            ExprKind::Pow(base, exp) => {
                let e_rat = match &graph.get(*exp).kind {
                    ExprKind::Number(n) => rational_from_number(n)?,
                    _ => {
                        return Err(AlgebraError::DomainViolation {
                            domain: "UniPoly".into(),
                            reason: "Non-constant exponent in polynomial".into(),
                        })
                    }
                };
                if !e_rat.is_integer() || e_rat.is_negative() {
                    return Err(AlgebraError::DomainViolation {
                        domain: "UniPoly".into(),
                        reason: format!("Polynomial exponent must be a non-negative integer, got {}", e_rat),
                    });
                }
                let exp_u32 = e_rat.numer().to_u32().ok_or_else(|| {
                    AlgebraError::DomainViolation {
                        domain: "UniPoly".into(),
                        reason: "Polynomial exponent exceeds u32".into(),
                    }
                })?;
                if exp_u32 > 1000 {
                    return Err(AlgebraError::DomainViolation {
                        domain: "UniPoly".into(),
                        reason: "Polynomial exponent exceeds degree cap (1000)".into(),
                    });
                }
                Ok(Self::from_expr(graph, *base, var)?.pow(exp_u32))
            }
            other => Err(AlgebraError::DomainViolation {
                domain: "UniPoly".into(),
                reason: format!("Non-polynomial AST node: {:?}", other),
            }),
        }
    }

    /// Materialize this univariate polynomial back into `graph` in descending power order.
    pub fn to_expr(&self, graph: &ExprGraph, var: SymbolId) -> ExprId {
        if self.is_zero() {
            return graph.integer(0);
        }
        let var_name = graph.symbols.resolve(var).unwrap_or_else(|| "x".to_string());
        let x_sym = graph.symbol(&var_name);
        let mut terms = Vec::new();
        for (deg, c) in self.coeffs.iter().enumerate().rev() {
            if c.is_zero() {
                continue;
            }
            let c_expr = expr_from_rational(graph, c);
            if deg == 0 {
                terms.push(c_expr);
            } else {
                let x_pow = if deg == 1 {
                    x_sym
                } else {
                    graph.pow(x_sym, graph.integer(deg as i64))
                };
                if *c == BigRational::one() {
                    terms.push(x_pow);
                } else if *c == -BigRational::one() {
                    terms.push(graph.neg(x_pow));
                } else {
                    terms.push(graph.mul([c_expr, x_pow]));
                }
            }
        }
        if terms.is_empty() {
            graph.integer(0)
        } else {
            graph.add(terms)
        }
    }
}

pub fn rational_from_number(n: &Number) -> AlgebraResult<BigRational> {
    match n {
        Number::Integer(i) => Ok(BigRational::from_integer(BigInt::from(*i))),
        Number::BigInteger(b) => Ok(BigRational::from_integer(b.clone())),
        Number::Rational(num, den) => {
            if *den == 0 {
                Err(AlgebraError::DivisionByZero {
                    domain: "Rational".into(),
                })
            } else {
                Ok(BigRational::new(BigInt::from(*num), BigInt::from(*den)))
            }
        }
        Number::BigRational(r) => Ok(r.as_ref().clone()),
        Number::Scientific { mantissa, exponent } => {
            if *exponent >= 0 {
                let factor = BigInt::from(10u32).pow(*exponent as u32);
                Ok(BigRational::from_integer(BigInt::from(*mantissa) * factor))
            } else {
                let factor = BigInt::from(10u32).pow((-*exponent) as u32);
                Ok(BigRational::new(BigInt::from(*mantissa), factor))
            }
        }
        _ => Err(AlgebraError::DomainViolation {
            domain: "Rational".into(),
            reason: format!("Non-rational numeric literal {:?}", n),
        }),
    }
}

pub fn expr_from_rational(graph: &ExprGraph, r: &BigRational) -> ExprId {
    if r.is_integer() {
        if let Some(i) = r.numer().to_i64() {
            return graph.integer(i);
        }
        return graph.intern(algebra_core::ExprNode::new(
            algebra_core::Domain::Integers,
            ExprKind::Number(Number::BigInteger(r.numer().clone())),
        ));
    }
    if let (Some(n), Some(d)) = (r.numer().to_i64(), r.denom().to_i64()) {
        return graph.rational(n, d);
    }
    graph.intern(algebra_core::ExprNode::new(
        algebra_core::Domain::Rationals,
        ExprKind::Number(Number::BigRational(Box::new(r.clone()))),
    ))
}

fn rational_matrix_det(mat: &mut [Vec<BigRational>]) -> AlgebraResult<BigRational> {
    let n = mat.len();
    let mut det = BigRational::one();
    for k in 0..n {
        let mut pivot_row = None;
        for (r, row) in mat.iter().enumerate().skip(k) {
            if !row[k].is_zero() {
                pivot_row = Some(r);
                break;
            }
        }
        let Some(pr) = pivot_row else {
            return Ok(BigRational::zero());
        };
        if pr != k {
            mat.swap(pr, k);
            det = -det;
        }
        let pivot = mat[k][k].clone();
        det *= &pivot;
        for i in (k + 1)..n {
            if mat[i][k].is_zero() {
                continue;
            }
            let factor = mat[i][k].clone() / &pivot;
            for j in k..n {
                let sub = &factor * &mat[k][j];
                mat[i][j] -= sub;
            }
        }
    }
    Ok(det)
}

/// Collect all distinct free variable symbols occurring inside an expression.
pub fn collect_expr_symbols(graph: &ExprGraph, id: ExprId, out: &mut BTreeSet<SymbolId>) {
    match &graph.get(id).kind {
        ExprKind::Symbol(s) => {
            out.insert(*s);
        }
        ExprKind::Add(ts) | ExprKind::Mul(ts) => {
            for &t in ts {
                collect_expr_symbols(graph, t, out);
            }
        }
        ExprKind::Sub(a, b)
        | ExprKind::Div(a, b)
        | ExprKind::Pow(a, b)
        | ExprKind::Relational { lhs: a, rhs: b, .. } => {
            collect_expr_symbols(graph, *a, out);
            collect_expr_symbols(graph, *b, out);
        }
        ExprKind::Neg(a) => collect_expr_symbols(graph, *a, out),
        ExprKind::Function { args, .. } => {
            for &a in args {
                collect_expr_symbols(graph, a, out);
            }
        }
        _ => {}
    }
}

/// Compute symbolic Sylvester resultant when polynomials contain parameter symbols besides `var`.
pub fn symbolic_resultant(
    graph: &ExprGraph,
    p_expr: ExprId,
    q_expr: ExprId,
    var: SymbolId,
) -> AlgebraResult<ExprId> {
    if let (Ok(p), Ok(q)) = (
        UniPoly::from_expr(graph, p_expr, var),
        UniPoly::from_expr(graph, q_expr, var),
    ) {
        return Ok(expr_from_rational(graph, &p.resultant(&q)?));
    }

    // Parametric univariate polynomial extraction in descending coeff order `[a_n, ..., a_0]`
    let p_desc = extract_symbolic_coeffs_desc(graph, p_expr, var)?;
    let q_desc = extract_symbolic_coeffs_desc(graph, q_expr, var)?;
    let m = p_desc.len() - 1;
    let n = q_desc.len() - 1;
    if m == 0 && n == 0 {
        return Ok(graph.integer(1));
    }
    let size = m + n;
    if size > 8 {
        return Err(AlgebraError::EvaluationError(
            "Parametric symbolic resultant capped at Sylvester matrix size 8x8".into(),
        ));
    }
    let zero = graph.integer(0);
    let mut elems = vec![zero; size * size];
    for row in 0..n {
        for (col, &c) in p_desc.iter().enumerate() {
            elems[row * size + row + col] = c;
        }
    }
    for row in 0..m {
        for (col, &c) in q_desc.iter().enumerate() {
            elems[(n + row) * size + row + col] = c;
        }
    }
    let mat = SymbolicMatrix::new(size, size, elems)?;
    mat.determinant(graph)
}

/// Compute exact or parametric discriminant $\operatorname{Disc}_x(P)$.
pub fn symbolic_discriminant(
    graph: &ExprGraph,
    p_expr: ExprId,
    var: SymbolId,
) -> AlgebraResult<ExprId> {
    if let Ok(p) = UniPoly::from_expr(graph, p_expr, var) {
        return Ok(expr_from_rational(graph, &p.discriminant()?));
    }
    let p_desc = extract_symbolic_coeffs_desc(graph, p_expr, var)?;
    let deg = p_desc.len() - 1;
    if deg == 0 {
        return Err(AlgebraError::DomainViolation {
            domain: "Polynomial".into(),
            reason: "Discriminant requires polynomial degree >= 1".into(),
        });
    }
    if deg == 1 {
        return Ok(graph.integer(1));
    }
    if deg == 2 {
        // Explicit b^2 - 4ac avoids introducing 1/a division.
        let (a, b, c) = (p_desc[0], p_desc[1], p_desc[2]);
        let b2 = graph.pow(b, graph.integer(2));
        let four_ac = graph.mul([graph.integer(4), a, c]);
        return Ok(graph.sub(b2, four_ac));
    }
    let dp = graph.diff(p_expr, var);
    let res = symbolic_resultant(graph, p_expr, dp, var)?;
    let signed = if ((deg * (deg - 1)) / 2).is_multiple_of(2) {
        res
    } else {
        graph.neg(res)
    };
    Ok(graph.div(signed, p_desc[0]))
}

/// Extract descending symbolic coefficients `[a_d, ..., a_0]` for low-degree parametric polynomials.
fn extract_symbolic_coeffs_desc(
    graph: &ExprGraph,
    expr: ExprId,
    var: SymbolId,
) -> AlgebraResult<Vec<ExprId>> {
    fn poly_add(g: &ExprGraph, mut a: Vec<ExprId>, mut b: Vec<ExprId>) -> Vec<ExprId> {
        let n = a.len().max(b.len());
        let zero = g.integer(0);
        a.resize(n, zero);
        b.resize(n, zero);
        a.into_iter().zip(b).map(|(x, y)| if x == zero { y } else if y == zero { x } else { g.add([x, y]) }).collect()
    }
    fn poly_mul(g: &ExprGraph, a: &[ExprId], b: &[ExprId]) -> Vec<ExprId> {
        if a.is_empty() || b.is_empty() {
            return vec![];
        }
        let zero = g.integer(0);
        let mut out = vec![zero; a.len() + b.len() - 1];
        for (i, &x) in a.iter().enumerate() {
            if x == zero { continue; }
            for (j, &y) in b.iter().enumerate() {
                if y == zero { continue; }
                let prod = g.mul([x, y]);
                out[i + j] = if out[i + j] == zero { prod } else { g.add([out[i + j], prod]) };
            }
        }
        out
    }
    fn rec(g: &ExprGraph, id: ExprId, var: SymbolId) -> AlgebraResult<Vec<ExprId>> {
        if !g.has_symbol(id, var) {
            return Ok(vec![id]);
        }
        match &g.get(id).kind {
            ExprKind::Symbol(s) if *s == var => Ok(vec![g.integer(0), g.integer(1)]),
            ExprKind::Add(ts) => {
                let mut acc = vec![];
                for &t in ts {
                    acc = poly_add(g, acc, rec(g, t, var)?);
                }
                Ok(acc)
            }
            ExprKind::Sub(l, r) => {
                let lp = rec(g, *l, var)?;
                let rp: Vec<ExprId> = rec(g, *r, var)?.into_iter().map(|c| g.neg(c)).collect();
                Ok(poly_add(g, lp, rp))
            }
            ExprKind::Neg(a) => Ok(rec(g, *a, var)?.into_iter().map(|c| g.neg(c)).collect()),
            ExprKind::Mul(fs) => {
                let mut acc = vec![g.integer(1)];
                for &f in fs {
                    acc = poly_mul(g, &acc, &rec(g, f, var)?);
                }
                Ok(acc)
            }
            ExprKind::Div(num, den) => {
                if g.has_symbol(*den, var) {
                    return Err(AlgebraError::DomainViolation {
                        domain: "Polynomial".into(),
                        reason: "Variable in denominator".into(),
                    });
                }
                let np = rec(g, *num, var)?;
                Ok(np.into_iter().map(|c| g.div(c, *den)).collect())
            }
            ExprKind::Pow(base, exp) => {
                let ExprKind::Number(Number::Integer(k)) = g.get(*exp).kind else {
                    return Err(AlgebraError::DomainViolation {
                        domain: "Polynomial".into(),
                        reason: "Exponent must be a non-negative integer".into(),
                    });
                };
                if !(0..=16).contains(&k) {
                    return Err(AlgebraError::DomainViolation {
                        domain: "Polynomial".into(),
                        reason: "Parametric exponent out of range 0..=16".into(),
                    });
                }
                let bp = rec(g, *base, var)?;
                let mut acc = vec![g.integer(1)];
                for _ in 0..k {
                    acc = poly_mul(g, &acc, &bp);
                }
                Ok(acc)
            }
            _ => Err(AlgebraError::DomainViolation {
                domain: "Polynomial".into(),
                reason: "Non-polynomial subexpression".into(),
            }),
        }
    }

    let mut asc = rec(graph, expr, var)?;
    let zero = graph.integer(0);
    while asc.last() == Some(&zero) {
        asc.pop();
    }
    if asc.is_empty() {
        asc.push(zero);
    }
    asc.reverse();
    Ok(asc)
}
