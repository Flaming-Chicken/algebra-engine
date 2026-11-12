//! # `algebra_engine::poly::multivariate`
//!
//! Multi-Variate Polynomial Engine & Monomial Orderings.
//!
//! Implements multi-variate polynomials $K[x_1, \dots, x_n]$ over real/rational coefficients with:
//! - Monomial Orderings: Lexicographic ($\operatorname{Lex}$), Graded Lexicographical ($\operatorname{GradedLex}$),
//!   and Graded Reverse Lexicographical ($\operatorname{GrevLex}$).
//! - Multi-variate polynomial arithmetic: Addition, Subtraction, Multiplication.
//! - Generalized Multi-Variate Polynomial Division Algorithm by a list of divisors $(f_1, \dots, f_s)$.

use algebra_core::error::{AlgebraError, AlgebraResult};
use algebra_core::{ExprGraph, ExprId, ExprKind, SymbolId};
use std::cmp::Ordering;

/// Admissible monomial orderings for multi-variate polynomials.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MonomialOrder {
    /// Lexicographical order: $x^\alpha \succ x^\beta$ if the leftmost non-zero entry of $\alpha - \beta$ is positive.
    /// Optimal for elimination ideals and solving triangularized polynomial systems.
    #[default]
    Lex,
    /// Graded Lexicographical order: compares total degrees first, then breaks ties via Lex.
    GradedLex,
    /// Graded Reverse Lexicographical order: compares total degrees first, then breaks ties if the rightmost non-zero entry of $\alpha - \beta$ is negative.
    /// Optimal for fast Gröbner basis computation and minimal intermediate polynomial growth.
    GrevLex,
}

/// A multi-variate monomial $x_1^{\alpha_1} x_2^{\alpha_2} \cdots x_n^{\alpha_n}$.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Monomial {
    /// Exponent vector $(\alpha_1, \dots, \alpha_n)$.
    pub exponents: Vec<usize>,
    /// Total degree $\sum_i \alpha_i$.
    pub total_degree: usize,
}

#[allow(clippy::needless_range_loop)]
impl Monomial {
    /// Creates a monomial from an exponent vector.
    pub fn new(exponents: Vec<usize>) -> Self {
        let total_degree = exponents.iter().sum();
        Self {
            exponents,
            total_degree,
        }
    }

    /// The constant monomial $1 = x_1^0 \cdots x_n^0$ of given number of variables.
    pub fn one(num_vars: usize) -> Self {
        Self {
            exponents: vec![0; num_vars],
            total_degree: 0,
        }
    }

    /// Monomial representing a single variable $x_i^1$.
    pub fn var(var_index: usize, num_vars: usize) -> Self {
        let mut exponents = vec![0; num_vars];
        if var_index < num_vars {
            exponents[var_index] = 1;
        }
        Self {
            exponents,
            total_degree: 1,
        }
    }

    /// Multiply two monomials: $x^\alpha \cdot x^\beta = x^{\alpha + \beta}$.
    pub fn mul(&self, other: &Self) -> Self {
        let n = self.exponents.len().max(other.exponents.len());
        let mut exponents = vec![0; n];
        for i in 0..n {
            let e1 = self.exponents.get(i).copied().unwrap_or(0);
            let e2 = other.exponents.get(i).copied().unwrap_or(0);
            exponents[i] = e1 + e2;
        }
        let total_degree = self.total_degree + other.total_degree;
        Self {
            exponents,
            total_degree,
        }
    }

    /// Checks if `self` is divisible by `other`: $\beta \le \alpha$ componentwise.
    pub fn is_divisible_by(&self, other: &Self) -> bool {
        let n = self.exponents.len().max(other.exponents.len());
        for i in 0..n {
            let e1 = self.exponents.get(i).copied().unwrap_or(0);
            let e2 = other.exponents.get(i).copied().unwrap_or(0);
            if e1 < e2 {
                return false;
            }
        }
        true
    }

    /// Divide two monomials: $x^\alpha / x^\beta = x^{\alpha - \beta}$. Returns None if not divisible.
    pub fn div(&self, other: &Self) -> Option<Self> {
        if !self.is_divisible_by(other) {
            return None;
        }
        let n = self.exponents.len().max(other.exponents.len());
        let mut exponents = vec![0; n];
        for i in 0..n {
            let e1 = self.exponents.get(i).copied().unwrap_or(0);
            let e2 = other.exponents.get(i).copied().unwrap_or(0);
            exponents[i] = e1 - e2;
        }
        let total_degree = self.total_degree.saturating_sub(other.total_degree);
        Some(Self {
            exponents,
            total_degree,
        })
    }

    /// Least Common Multiple: $\operatorname{lcm}(x^\alpha, x^\beta) = x^{\max(\alpha, \beta)}$.
    pub fn lcm(&self, other: &Self) -> Self {
        let n = self.exponents.len().max(other.exponents.len());
        let mut exponents = vec![0; n];
        for i in 0..n {
            let e1 = self.exponents.get(i).copied().unwrap_or(0);
            let e2 = other.exponents.get(i).copied().unwrap_or(0);
            exponents[i] = e1.max(e2);
        }
        let total_degree = exponents.iter().sum();
        Self {
            exponents,
            total_degree,
        }
    }

    /// Compare two monomials according to a given `MonomialOrder`.
    pub fn cmp_with_order(&self, other: &Self, order: MonomialOrder) -> Ordering {
        let n = self.exponents.len().max(other.exponents.len());
        match order {
            MonomialOrder::Lex => {
                for i in 0..n {
                    let e1 = self.exponents.get(i).copied().unwrap_or(0);
                    let e2 = other.exponents.get(i).copied().unwrap_or(0);
                    if e1 != e2 {
                        return e1.cmp(&e2);
                    }
                }
                Ordering::Equal
            }
            MonomialOrder::GradedLex => {
                if self.total_degree != other.total_degree {
                    return self.total_degree.cmp(&other.total_degree);
                }
                self.cmp_with_order(other, MonomialOrder::Lex)
            }
            MonomialOrder::GrevLex => {
                if self.total_degree != other.total_degree {
                    return self.total_degree.cmp(&other.total_degree);
                }
                // Break ties in reverse lexicographical order with opposite comparison
                for i in (0..n).rev() {
                    let e1 = self.exponents.get(i).copied().unwrap_or(0);
                    let e2 = other.exponents.get(i).copied().unwrap_or(0);
                    if e1 != e2 {
                        return e2.cmp(&e1);
                    }
                }
                Ordering::Equal
            }
        }
    }
}

/// A term $c \cdot x^\alpha$ consisting of a scalar coefficient and a monomial.
#[derive(Debug, Clone, PartialEq)]
pub struct Term {
    /// Scalar coefficient.
    pub coeff: f64,
    /// Monomial.
    pub monomial: Monomial,
}

impl Term {
    /// Creates a term.
    pub fn new(coeff: f64, monomial: Monomial) -> Self {
        Self { coeff, monomial }
    }
}

/// Multi-Variate Polynomial in $n$ variables with a defined monomial ordering.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiPoly {
    /// List of non-zero terms sorted descending by `order`.
    pub terms: Vec<Term>,
    /// Number of variables $n$.
    pub num_vars: usize,
    /// Variable symbols.
    pub var_names: Vec<String>,
    /// Monomial term ordering.
    pub order: MonomialOrder,
}

impl MultiPoly {
    /// Creates the zero polynomial.
    pub fn zero(num_vars: usize, var_names: Vec<String>, order: MonomialOrder) -> Self {
        Self {
            terms: Vec::new(),
            num_vars,
            var_names,
            order,
        }
    }

    /// Creates a constant polynomial $c$.
    pub fn constant(c: f64, num_vars: usize, var_names: Vec<String>, order: MonomialOrder) -> Self {
        if c.abs() < 1e-12 {
            Self::zero(num_vars, var_names, order)
        } else {
            Self {
                terms: vec![Term::new(c, Monomial::one(num_vars))],
                num_vars,
                var_names,
                order,
            }
        }
    }

    /// Creates a single variable polynomial $x_i$.
    pub fn var(
        var_index: usize,
        num_vars: usize,
        var_names: Vec<String>,
        order: MonomialOrder,
    ) -> Self {
        Self {
            terms: vec![Term::new(1.0, Monomial::var(var_index, num_vars))],
            num_vars,
            var_names,
            order,
        }
    }

    /// Creates a polynomial from raw terms, canonicalizing and sorting descending by `order`.
    pub fn from_terms(
        mut terms: Vec<Term>,
        num_vars: usize,
        var_names: Vec<String>,
        order: MonomialOrder,
    ) -> Self {
        // Sort descending by monomial order
        terms.sort_by(|a, b| b.monomial.cmp_with_order(&a.monomial, order));

        // Combine like terms and remove zeroes
        let mut clean_terms = Vec::with_capacity(terms.len());
        for term in terms {
            if term.coeff.abs() < 1e-12 {
                continue;
            }
            if let Some(last) = clean_terms.last_mut() {
                let last_term: &mut Term = last;
                if last_term.monomial == term.monomial {
                    last_term.coeff += term.coeff;
                    continue;
                }
            }
            clean_terms.push(term);
        }

        // Filter out zero coefficients created by cancellation
        clean_terms.retain(|t| t.coeff.abs() >= 1e-12);

        Self {
            terms: clean_terms,
            num_vars,
            var_names,
            order,
        }
    }

    /// Checks if the polynomial is zero.
    pub fn is_zero(&self) -> bool {
        self.terms.is_empty()
    }

    /// Returns the Leading Term $\operatorname{LT}(f)$.
    pub fn leading_term(&self) -> Option<&Term> {
        self.terms.first()
    }

    /// Returns the Leading Monomial $\operatorname{LM}(f)$.
    pub fn leading_monomial(&self) -> Option<&Monomial> {
        self.terms.first().map(|t| &t.monomial)
    }

    /// Returns the Leading Coefficient $\operatorname{LC}(f)$.
    pub fn leading_coeff(&self) -> Option<f64> {
        self.terms.first().map(|t| t.coeff)
    }

    /// Returns the monic polynomial with leading coefficient normalized to 1.
    pub fn to_monic(&self) -> Self {
        if let Some(lc) = self.leading_coeff()
            && lc.abs() > 1e-12
        {
            let mut monic = self.clone();
            for t in &mut monic.terms {
                t.coeff /= lc;
            }
            return monic;
        }
        self.clone()
    }

    /// Polynomial addition: $f + g$.
    pub fn add(&self, other: &Self) -> Self {
        let mut all_terms = self.terms.clone();
        all_terms.extend(other.terms.clone());
        Self::from_terms(all_terms, self.num_vars, self.var_names.clone(), self.order)
    }

    /// Polynomial subtraction: $f - g$.
    pub fn sub(&self, other: &Self) -> Self {
        let mut all_terms = self.terms.clone();
        for t in &other.terms {
            all_terms.push(Term::new(-t.coeff, t.monomial.clone()));
        }
        Self::from_terms(all_terms, self.num_vars, self.var_names.clone(), self.order)
    }

    /// Polynomial multiplication: $f \cdot g$.
    pub fn mul(&self, other: &Self) -> Self {
        let mut prod_terms = Vec::with_capacity(self.terms.len() * other.terms.len());
        for t1 in &self.terms {
            for t2 in &other.terms {
                let coeff = t1.coeff * t2.coeff;
                let mono = t1.monomial.mul(&t2.monomial);
                prod_terms.push(Term::new(coeff, mono));
            }
        }
        Self::from_terms(
            prod_terms,
            self.num_vars,
            self.var_names.clone(),
            self.order,
        )
    }

    /// Multi-variate polynomial division algorithm of $f$ by divisors $(f_1, \dots, f_s)$.
    ///
    /// Computes quotients $(a_1, \dots, a_s)$ and remainder $r$ such that:
    /// $$f = a_1 f_1 + \dots + a_s f_s + r$$
    /// where no term of $r$ is divisible by any $\operatorname{LM}(f_i)$.
    pub fn div_rem(&self, divisors: &[Self]) -> (Vec<Self>, Self) {
        let s = divisors.len();
        let mut quotients = vec![Self::zero(self.num_vars, self.var_names.clone(), self.order); s];
        let mut remainder_terms = Vec::new();
        let mut p = self.clone();

        while !p.is_zero() {
            let Some(lt_p_ref) = p.leading_term() else {
                break;
            };
            let lt_p = lt_p_ref.clone();
            let mut division_occurred = false;

            for (i, fi) in divisors.iter().enumerate() {
                if fi.is_zero() {
                    continue;
                }
                let Some(lt_fi) = fi.leading_term() else {
                    continue;
                };
                if let Some(mono_div) = lt_p.monomial.div(&lt_fi.monomial) {
                    let coeff_div = lt_p.coeff / lt_fi.coeff;
                    let factor = Self::from_terms(
                        vec![Term::new(coeff_div, mono_div)],
                        self.num_vars,
                        self.var_names.clone(),
                        self.order,
                    );

                    // quotients[i] += factor
                    quotients[i] = quotients[i].add(&factor);
                    // p -= factor * fi
                    let sub_poly = factor.mul(fi);
                    p = p.sub(&sub_poly);

                    division_occurred = true;
                    break;
                }
            }

            if !division_occurred {
                // lt_p goes into remainder
                remainder_terms.push(lt_p.clone());
                // p -= lt_p
                let lt_poly = Self::from_terms(
                    vec![lt_p],
                    self.num_vars,
                    self.var_names.clone(),
                    self.order,
                );
                p = p.sub(&lt_poly);
            }
        }

        let remainder = Self::from_terms(
            remainder_terms,
            self.num_vars,
            self.var_names.clone(),
            self.order,
        );

        (quotients, remainder)
    }

    /// Converts an expression from `ExprGraph` into a `MultiPoly`.
    pub fn from_expr(
        graph: &ExprGraph,
        expr: ExprId,
        var_names: &[String],
        order: MonomialOrder,
    ) -> AlgebraResult<Self> {
        let num_vars = var_names.len();
        let var_symbols: Vec<SymbolId> = var_names
            .iter()
            .map(|name| graph.symbols.get_or_intern(name))
            .collect();

        Self::expr_to_poly_recursive(graph, expr, &var_symbols, var_names, num_vars, order)
    }

    fn expr_to_poly_recursive(
        graph: &ExprGraph,
        expr: ExprId,
        var_symbols: &[SymbolId],
        var_names: &[String],
        num_vars: usize,
        order: MonomialOrder,
    ) -> AlgebraResult<Self> {
        let node = graph.get(expr);
        match &node.kind {
            ExprKind::Number(num) => {
                let c = match num {
                    algebra_core::Number::Integer(i) => *i as f64,
                    algebra_core::Number::Rational(n, d) => *n as f64 / *d as f64,
                    algebra_core::Number::Float(f) => f64::from_bits(*f),
                    _ => 0.0,
                };
                Ok(Self::constant(c, num_vars, var_names.to_vec(), order))
            }
            ExprKind::Symbol(sym) => {
                for (i, &v_sym) in var_symbols.iter().enumerate() {
                    if v_sym == *sym {
                        return Ok(Self::var(i, num_vars, var_names.to_vec(), order));
                    }
                }
                // Non-target symbol treated as variable or constant
                let s_str = graph.symbols.resolve(*sym).unwrap_or_default();
                Err(AlgebraError::EvaluationError(format!(
                    "Unrecognized variable symbol '{s_str}' in polynomial conversion"
                )))
            }
            ExprKind::Add(terms) => {
                let mut sum = Self::zero(num_vars, var_names.to_vec(), order);
                for &t in terms {
                    let p = Self::expr_to_poly_recursive(
                        graph,
                        t,
                        var_symbols,
                        var_names,
                        num_vars,
                        order,
                    )?;
                    sum = sum.add(&p);
                }
                Ok(sum)
            }
            ExprKind::Sub(l, r) => {
                let p1 = Self::expr_to_poly_recursive(
                    graph,
                    *l,
                    var_symbols,
                    var_names,
                    num_vars,
                    order,
                )?;
                let p2 = Self::expr_to_poly_recursive(
                    graph,
                    *r,
                    var_symbols,
                    var_names,
                    num_vars,
                    order,
                )?;
                Ok(p1.sub(&p2))
            }
            ExprKind::Mul(factors) => {
                let mut prod = Self::constant(1.0, num_vars, var_names.to_vec(), order);
                for &f in factors {
                    let p = Self::expr_to_poly_recursive(
                        graph,
                        f,
                        var_symbols,
                        var_names,
                        num_vars,
                        order,
                    )?;
                    prod = prod.mul(&p);
                }
                Ok(prod)
            }
            ExprKind::Neg(inner) => {
                let p = Self::expr_to_poly_recursive(
                    graph,
                    *inner,
                    var_symbols,
                    var_names,
                    num_vars,
                    order,
                )?;
                let zero = Self::zero(num_vars, var_names.to_vec(), order);
                Ok(zero.sub(&p))
            }
            ExprKind::Pow(base, exp) => {
                let p_base = Self::expr_to_poly_recursive(
                    graph,
                    *base,
                    var_symbols,
                    var_names,
                    num_vars,
                    order,
                )?;
                if let ExprKind::Number(algebra_core::Number::Integer(exp_val)) =
                    graph.get(*exp).kind
                    && exp_val >= 0
                {
                    let mut res = Self::constant(1.0, num_vars, var_names.to_vec(), order);
                    for _ in 0..exp_val {
                        res = res.mul(&p_base);
                    }
                    return Ok(res);
                }
                Err(AlgebraError::EvaluationError(
                    "Polynomial exponent must be a non-negative integer".into(),
                ))
            }
            _ => Err(AlgebraError::EvaluationError(
                "Non-polynomial expression node encountered".into(),
            )),
        }
    }

    /// Converts the `MultiPoly` back into an `ExprId` in `ExprGraph`.
    pub fn to_expr(&self, graph: &ExprGraph) -> ExprId {
        if self.is_zero() {
            return graph.integer(0);
        }

        let mut term_exprs = Vec::with_capacity(self.terms.len());
        for t in &self.terms {
            let mut factor_exprs = Vec::new();
            if (t.coeff - 1.0).abs() > 1e-12 || t.monomial.total_degree == 0 {
                // Integer or float constant
                if (t.coeff.fract()).abs() < 1e-12 {
                    factor_exprs.push(graph.integer(t.coeff.round() as i64));
                } else {
                    factor_exprs.push(graph.float(t.coeff));
                }
            }

            for (i, &exp) in t.monomial.exponents.iter().enumerate() {
                if exp > 0 {
                    let var_name = self
                        .var_names
                        .get(i)
                        .cloned()
                        .unwrap_or_else(|| format!("x{i}"));
                    let var_node = graph.symbol(&var_name);
                    if exp == 1 {
                        factor_exprs.push(var_node);
                    } else {
                        let exp_node = graph.integer(exp as i64);
                        factor_exprs.push(graph.pow(var_node, exp_node));
                    }
                }
            }

            if factor_exprs.len() == 1 {
                term_exprs.push(factor_exprs[0]);
            } else {
                term_exprs.push(graph.mul(factor_exprs));
            }
        }

        if term_exprs.len() == 1 {
            term_exprs[0]
        } else {
            graph.add(term_exprs)
        }
    }
}
