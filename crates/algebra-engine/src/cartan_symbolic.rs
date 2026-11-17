//! Differential forms with **symbolic coefficients**.
//!
//! [`crate::cartan::DifferentialForm`] holds constant `f64` coefficients, so its exterior
//! derivative is always zero. A [`SymbolicForm`] keeps each coefficient as an expression in the
//! coordinate symbols, which makes `d` meaningful: `d(f dx^I) = sum_j (df/dx_j) dx^j ^ dx^I`.
//!
//! The operations are the algebraic core of the exterior calculus: sum, scaling, wedge product,
//! exterior derivative, interior product with a vector field and the Lie derivative from Cartan's
//! formula `L_X = i_X d + d i_X`. Coefficients are built with the graph's constructors and
//! differentiated with [`SymbolicCalculus::diff`]; they are not algebraically simplified, so tests
//! and callers should compare by numerical evaluation (or run the simplifier) rather than by node
//! identity.

use crate::calculus::SymbolicCalculus;
use crate::cartan::FormBasis;
use algebra_core::error::{AlgebraError, AlgebraResult};
use algebra_core::{ExprGraph, ExprId, ExprKind};
use std::collections::BTreeMap;

/// A differential `k`-form on `R^n` whose coefficients are expressions in the coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolicForm {
    /// Coordinate symbols `x_0 .. x_{n-1}` (each an [`ExprKind::Symbol`] node).
    pub coords: Vec<ExprId>,
    /// Degree `k`.
    pub degree: usize,
    /// Basis blade to coefficient expression. A blade that is absent has coefficient zero.
    pub terms: BTreeMap<FormBasis, ExprId>,
}

fn error(msg: &str) -> AlgebraError {
    AlgebraError::EvaluationError(format!("symbolic form: {msg}"))
}

impl SymbolicForm {
    /// The zero `degree`-form on the given coordinates.
    pub fn zero(coords: &[ExprId], degree: usize) -> Self {
        Self {
            coords: coords.to_vec(),
            degree,
            terms: BTreeMap::new(),
        }
    }

    /// A 0-form (function) `f`.
    pub fn function(coords: &[ExprId], f: ExprId) -> Self {
        let mut form = Self::zero(coords, 0);
        form.terms.insert(FormBasis::scalar(), f);
        form
    }

    /// The 1-form `sum_i c_i dx^i`; `components` must have one entry per coordinate.
    pub fn one_form(coords: &[ExprId], components: &[ExprId]) -> AlgebraResult<Self> {
        if components.len() != coords.len() {
            return Err(error("a 1-form needs one component per coordinate"));
        }
        let mut form = Self::zero(coords, 1);
        for (i, &c) in components.iter().enumerate() {
            form.terms.insert(FormBasis::one_form(i), c);
        }
        Ok(form)
    }

    /// The single term `coefficient * dx^{i_1} ^ ... ^ dx^{i_k}` (indices in any order; a repeated
    /// index gives the zero form, a swap flips the sign).
    pub fn term(
        g: &ExprGraph,
        coords: &[ExprId],
        coefficient: ExprId,
        indices: &[usize],
    ) -> AlgebraResult<Self> {
        if indices.iter().any(|&i| i >= coords.len()) {
            return Err(error("a coordinate index is out of range"));
        }
        let mut form = Self::zero(coords, indices.len());
        let (basis, sign) = FormBasis::new(indices.to_vec());
        if sign != 0 {
            form.terms.insert(basis, signed(g, coefficient, sign));
        }
        Ok(form)
    }

    fn check_same_space(&self, other: &Self) -> AlgebraResult<()> {
        if self.coords != other.coords {
            return Err(error("the forms live on different coordinates"));
        }
        Ok(())
    }

    /// Sum of two forms of the same degree.
    pub fn add(&self, g: &ExprGraph, other: &Self) -> AlgebraResult<Self> {
        self.check_same_space(other)?;
        if self.degree != other.degree {
            return Err(error("cannot add forms of different degree"));
        }
        let mut out = self.clone();
        for (basis, &c) in &other.terms {
            let merged = match out.terms.get(basis) {
                Some(&existing) => g.add([existing, c]),
                None => c,
            };
            out.terms.insert(basis.clone(), merged);
        }
        Ok(out)
    }

    /// The form multiplied by the function `f`.
    pub fn scale(&self, g: &ExprGraph, f: ExprId) -> Self {
        Self {
            coords: self.coords.clone(),
            degree: self.degree,
            terms: self
                .terms
                .iter()
                .map(|(b, &c)| (b.clone(), g.mul([f, c])))
                .collect(),
        }
    }

    /// Wedge product; the degree is the sum of the degrees (and a product whose degree exceeds the
    /// dimension is zero).
    pub fn wedge(&self, g: &ExprGraph, other: &Self) -> AlgebraResult<Self> {
        self.check_same_space(other)?;
        let mut out = Self::zero(&self.coords, self.degree + other.degree);
        for (ba, &ca) in &self.terms {
            for (bb, &cb) in &other.terms {
                let mut idx = ba.indices.clone();
                idx.extend(&bb.indices);
                let (basis, sign) = FormBasis::new(idx);
                if sign == 0 {
                    continue;
                }
                let product = signed(g, g.mul([ca, cb]), sign);
                let merged = match out.terms.get(&basis) {
                    Some(&existing) => g.add([existing, product]),
                    None => product,
                };
                out.terms.insert(basis, merged);
            }
        }
        Ok(out)
    }

    fn coord_symbol(&self, g: &ExprGraph, j: usize) -> AlgebraResult<algebra_core::SymbolId> {
        match g.get(self.coords[j]).kind {
            ExprKind::Symbol(id) => Ok(id),
            _ => Err(error("every coordinate must be a symbol")),
        }
    }

    /// Exterior derivative `d`: `(k)`-form to `(k+1)`-form, `d(f dx^I) = sum_j (df/dx_j) dx^j ^ dx^I`.
    pub fn exterior_derivative(&self, g: &ExprGraph) -> AlgebraResult<Self> {
        let mut out = Self::zero(&self.coords, self.degree + 1);
        for (basis, &c) in &self.terms {
            for j in 0..self.coords.len() {
                let dc = g.diff(c, self.coord_symbol(g, j)?);
                let mut idx = vec![j];
                idx.extend(&basis.indices);
                let (b, sign) = FormBasis::new(idx);
                if sign == 0 {
                    continue;
                }
                let term = signed(g, dc, sign);
                let merged = match out.terms.get(&b) {
                    Some(&existing) => g.add([existing, term]),
                    None => term,
                };
                out.terms.insert(b, merged);
            }
        }
        Ok(out)
    }

    /// Interior product `i_X` with the vector field `X = sum_j v_j d/dx_j`:
    /// `i_X(f dx^{i_1} ^ ... ^ dx^{i_k}) = sum_m (-1)^m f v_{i_m} dx^{i_1} ^ .. (omit i_m) .. ^ dx^{i_k}`.
    pub fn interior_product(&self, g: &ExprGraph, vector: &[ExprId]) -> AlgebraResult<Self> {
        if vector.len() != self.coords.len() {
            return Err(error("a vector field needs one component per coordinate"));
        }
        if self.degree == 0 {
            return Ok(Self::zero(&self.coords, 0));
        }
        let mut out = Self::zero(&self.coords, self.degree - 1);
        for (basis, &c) in &self.terms {
            for (m, &i) in basis.indices.iter().enumerate() {
                let mut rest = basis.indices.clone();
                rest.remove(m);
                // `rest` stays sorted and duplicate-free, so it is already a valid basis.
                let sign = if m % 2 == 0 { 1 } else { -1 };
                let term = signed(g, g.mul([c, vector[i]]), sign);
                let key = FormBasis { indices: rest };
                let merged = match out.terms.get(&key) {
                    Some(&existing) => g.add([existing, term]),
                    None => term,
                };
                out.terms.insert(key, merged);
            }
        }
        Ok(out)
    }

    /// Lie derivative along `X` by Cartan's formula `L_X = i_X d + d i_X`.
    pub fn lie_derivative(&self, g: &ExprGraph, vector: &[ExprId]) -> AlgebraResult<Self> {
        let a = self.exterior_derivative(g)?.interior_product(g, vector)?;
        if self.degree == 0 {
            // i_X of a function is zero, so L_X f = i_X df = X(f).
            return Ok(a);
        }
        let b = self.interior_product(g, vector)?.exterior_derivative(g)?;
        a.add(g, &b)
    }

    /// Evaluate every coefficient at the point `values` (one value per coordinate), using the
    /// evaluator `eval(graph, expr, coordinate values)`; returns blade to number.
    pub fn evaluate(
        &self,
        g: &ExprGraph,
        values: &[f64],
    ) -> AlgebraResult<BTreeMap<FormBasis, f64>> {
        if values.len() != self.coords.len() {
            return Err(error("one value per coordinate is required"));
        }
        let mut names = BTreeMap::new();
        for (j, &v) in values.iter().enumerate() {
            names.insert(self.coord_symbol(g, j)?, v);
        }
        self.terms
            .iter()
            .map(|(b, &c)| Ok((b.clone(), evaluate_expr(g, c, &names)?)))
            .collect()
    }
}

fn signed(g: &ExprGraph, e: ExprId, sign: i64) -> ExprId {
    if sign < 0 { g.neg(e) } else { e }
}

/// Numerical value of an expression with the given symbol values (elementary functions only).
fn evaluate_expr(
    g: &ExprGraph,
    e: ExprId,
    vars: &BTreeMap<algebra_core::SymbolId, f64>,
) -> AlgebraResult<f64> {
    let go = |x: ExprId| evaluate_expr(g, x, vars);
    Ok(match &g.get(e).kind {
        ExprKind::Number(n) => n.as_f64().ok_or_else(|| error("a non-numeric literal"))?,
        ExprKind::Symbol(id) => *vars
            .get(id)
            .ok_or_else(|| error("a symbol that is not a coordinate"))?,
        ExprKind::Add(ts) => ts.iter().map(|&t| go(t)).sum::<AlgebraResult<f64>>()?,
        ExprKind::Mul(fs) => fs.iter().map(|&f| go(f)).product::<AlgebraResult<f64>>()?,
        ExprKind::Sub(l, r) => go(*l)? - go(*r)?,
        ExprKind::Div(l, r) => go(*l)? / go(*r)?,
        ExprKind::Neg(i) => -go(*i)?,
        ExprKind::Pow(b, x) => go(*b)?.powf(go(*x)?),
        ExprKind::Function { name, args } if args.len() == 1 => {
            let x = go(args[0])?;
            match g.symbols.resolve(*name).as_deref() {
                Some("exp") => x.exp(),
                Some("sin") => x.sin(),
                Some("cos") => x.cos(),
                Some("sinh") => x.sinh(),
                Some("cosh") => x.cosh(),
                Some("tan") => x.tan(),
                Some("ln") | Some("log") => x.ln(),
                Some("sqrt") => x.sqrt(),
                other => return Err(error(&format!("unsupported function {other:?}"))),
            }
        }
        other => return Err(error(&format!("unsupported expression node {other:?}"))),
    })
}
