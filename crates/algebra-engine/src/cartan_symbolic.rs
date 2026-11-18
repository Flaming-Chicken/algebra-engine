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

    /// Hodge star on `R^n` with the standard orientation: `star(f dx^I) = +/- f dx^J`, where `J` is
    /// the complement of `I` and the sign is the parity of the permutation `(I, J)`. With
    /// `is_minkowski`, blades containing `dx^0` also change sign (the convention of
    /// [`crate::cartan::DifferentialForm::hodge_star`]).
    pub fn hodge_star(&self, g: &ExprGraph, is_minkowski: bool) -> AlgebraResult<Self> {
        let n = self.coords.len();
        let mut out = Self::zero(&self.coords, n - self.degree);
        for (basis, &c) in &self.terms {
            let complement: Vec<usize> = (0..n).filter(|i| !basis.indices.contains(i)).collect();
            let mut full = basis.indices.clone();
            full.extend(&complement);
            let (_, sign) = FormBasis::new(full);
            let metric = if is_minkowski && basis.indices.contains(&0) {
                -1
            } else {
                1
            };
            let term = signed(g, c, sign * metric);
            let key = FormBasis {
                indices: complement,
            };
            let merged = match out.terms.get(&key) {
                Some(&existing) => g.add([existing, term]),
                None => term,
            };
            out.terms.insert(key, merged);
        }
        Ok(out)
    }

    /// Hodge star for a *diagonal* metric `g = diag(g_00, ..., g_{n-1,n-1})` whose entries are
    /// expressions in the coordinates (polar, spherical, Minkowski, ...), with the orientation
    /// `dx^0 ^ ... ^ dx^{n-1}`:
    ///
    /// `star(f dx^I) = sqrt(|det g|) (prod_{i in I} 1 / g_ii) * sign(I, J) f dx^J`.
    ///
    /// With `g = diag(1, ..., 1)` this is [`SymbolicForm::hodge_star`]; with `diag(-1, 1, ..., 1)`
    /// it reproduces the Minkowski convention of that function.
    pub fn hodge_star_diagonal(&self, g: &ExprGraph, metric: &[ExprId]) -> AlgebraResult<Self> {
        let n = self.coords.len();
        if metric.len() != n {
            return Err(error("a diagonal metric needs one entry per coordinate"));
        }
        let det = g.mul(metric.iter().copied());
        let volume = g.function("sqrt", [g.function("abs", [det])]);
        let mut out = Self::zero(&self.coords, n - self.degree);
        for (basis, &c) in &self.terms {
            let complement: Vec<usize> = (0..n).filter(|i| !basis.indices.contains(i)).collect();
            let mut full = basis.indices.clone();
            full.extend(&complement);
            let (_, sign) = FormBasis::new(full);
            let inverse = if basis.indices.is_empty() {
                g.integer(1)
            } else {
                g.div(
                    g.integer(1),
                    g.mul(basis.indices.iter().map(|&i| metric[i])),
                )
            };
            let term = signed(g, g.mul([c, volume, inverse]), sign);
            let key = FormBasis {
                indices: complement,
            };
            let merged = match out.terms.get(&key) {
                Some(&existing) => g.add([existing, term]),
                None => term,
            };
            out.terms.insert(key, merged);
        }
        Ok(out)
    }

    /// Hodge star for a general symmetric metric `g_ij(x)` (any entries, up to four coordinates)
    /// with the orientation `dx^0 ^ ... ^ dx^{n-1}`:
    ///
    /// `star(omega)_J = sqrt|det g| sum_I sign(I, J) omega^I`, where the raised components are
    /// `omega^I = sum_L minor_{I,L}(g^{-1}) omega_L` (minors of the inverse metric).
    ///
    /// The inverse is the adjugate over the determinant, built symbolically, so entries may be
    /// expressions in the coordinates; nothing is simplified. For a diagonal metric this equals
    /// [`SymbolicForm::hodge_star_diagonal`].
    pub fn hodge_star_metric(&self, g: &ExprGraph, metric: &[Vec<ExprId>]) -> AlgebraResult<Self> {
        let n = self.coords.len();
        if n > 4 {
            return Err(error(
                "the general metric Hodge star supports up to four coordinates",
            ));
        }
        if metric.len() != n || metric.iter().any(|row| row.len() != n) {
            return Err(error("the metric must be an n x n matrix of expressions"));
        }
        let det = determinant(g, metric);
        // Inverse metric: adj(g) / det(g), with adj_{ij} = (-1)^{i+j} minor_{ji}.
        let inverse: Vec<Vec<ExprId>> = (0..n)
            .map(|i| {
                (0..n)
                    .map(|j| {
                        let m = submatrix(metric, &[j], &[i]);
                        let cofactor = determinant(g, &m);
                        let cofactor = signed(g, cofactor, if (i + j) % 2 == 0 { 1 } else { -1 });
                        g.div(cofactor, det)
                    })
                    .collect()
            })
            .collect();
        let volume = g.function("sqrt", [g.function("abs", [det])]);
        let k = self.degree;
        let mut out = Self::zero(&self.coords, n - k);
        for i_set in subsets(n, k) {
            // omega^I = sum over L of the minor of the inverse metric (rows I, columns L) * omega_L.
            let mut raised = g.integer(0);
            for (basis, &c) in &self.terms {
                let minor = if k == 0 {
                    g.integer(1)
                } else {
                    determinant(g, &submatrix_keep(&inverse, &i_set, &basis.indices))
                };
                raised = g.add([raised, g.mul([minor, c])]);
            }
            let complement: Vec<usize> = (0..n).filter(|i| !i_set.contains(i)).collect();
            let mut full = i_set.clone();
            full.extend(&complement);
            let (_, sign) = FormBasis::new(full);
            let term = signed(g, g.mul([volume, raised]), sign);
            let key = FormBasis {
                indices: complement,
            };
            let merged = match out.terms.get(&key) {
                Some(&existing) => g.add([existing, term]),
                None => term,
            };
            out.terms.insert(key, merged);
        }
        Ok(out)
    }

    /// Codifferential `delta = (-1)^{n(k+1)+1} star d star` (Euclidean metric): it lowers the
    /// degree by one, and `delta` of a 1-form is minus the divergence.
    pub fn codifferential(&self, g: &ExprGraph) -> AlgebraResult<Self> {
        if self.degree == 0 {
            return Ok(Self::zero(&self.coords, 0));
        }
        let n = self.coords.len() as i64;
        let k = self.degree as i64;
        let sign = if (n * (k + 1) + 1) % 2 == 0 { 1 } else { -1 };
        let inner = self.hodge_star(g, false)?.exterior_derivative(g)?;
        let out = inner.hodge_star(g, false)?;
        Ok(Self {
            coords: out.coords,
            degree: out.degree,
            terms: out
                .terms
                .into_iter()
                .map(|(b, c)| (b, signed(g, c, sign)))
                .collect(),
        })
    }

    /// Laplace-de Rham operator `d delta + delta d` (Euclidean metric); on functions it is minus
    /// the usual Laplacian.
    pub fn laplace_de_rham(&self, g: &ExprGraph) -> AlgebraResult<Self> {
        let a = self.exterior_derivative(g)?.codifferential(g)?;
        let b = self.codifferential(g)?.exterior_derivative(g)?;
        if self.degree == 0 {
            return Ok(a);
        }
        a.add(g, &b)
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

    /// Parse a form with symbolic coefficients such as `x^2 dy`, `sin(x) dx^dy - y dz` or `z`.
    ///
    /// Terms are separated by a top-level `+` or `-` (not inside parentheses and not directly
    /// after `^`, `*`, `/` or `(`). A term is a coefficient expression followed by a wedge chain of
    /// coordinate differentials (`dx`, `dy^dz`, with `^`, `*` or `∧` between them); a term with no
    /// differential is a function. All terms must have the same degree. `coord_names` name the
    /// coordinates in order.
    pub fn parse(g: &ExprGraph, input: &str, coord_names: &[String]) -> AlgebraResult<Self> {
        let fail = |why: String| error(&format!("form `{input}`: {why}"));
        let coords: Vec<ExprId> = coord_names.iter().map(|n| g.symbol(n)).collect();
        let text: Vec<char> = input.replace('∧', "^").chars().collect();
        if text.iter().all(|c| c.is_whitespace()) {
            return Err(fail("empty".into()));
        }

        // Split into signed terms at top-level + and -.
        let mut pieces: Vec<(bool, String)> = Vec::new(); // (negative, text)
        let (mut depth, mut start, mut negative) = (0_i32, 0, false);
        let mut prev_sig: Option<char> = None;
        for (i, &c) in text.iter().enumerate() {
            match c {
                '(' => depth += 1,
                ')' => depth -= 1,
                '+' | '-' if depth == 0 => {
                    let unary = matches!(prev_sig, None | Some('^' | '*' | '/' | '(' | '+' | '-'));
                    if !unary {
                        pieces.push((negative, text[start..i].iter().collect()));
                        negative = c == '-';
                        start = i + 1;
                    } else if i == start {
                        // A leading sign of this term.
                        negative ^= c == '-';
                        start = i + 1;
                    }
                }
                _ => {}
            }
            if !c.is_whitespace() {
                prev_sig = Some(c);
            }
        }
        pieces.push((negative, text[start..].iter().collect()));

        let parser = algebra_core::parser::ExprParser::new(g);
        let mut form: Option<Self> = None;
        for (negative, piece) in pieces {
            if piece.trim().is_empty() {
                return Err(fail("dangling operator".into()));
            }
            let chars: Vec<char> = piece.chars().collect();
            // The first differential: `d` + coordinate name, not part of a longer identifier.
            let is_word = |c: char| c.is_alphanumeric() || c == '_';
            let mut first = None;
            for i in 0..chars.len() {
                if chars[i] != 'd' || (i > 0 && chars[i - 1].is_alphabetic()) {
                    continue;
                }
                for name in coord_names {
                    let n: Vec<char> = name.chars().collect();
                    let end = i + 1 + n.len();
                    if end <= chars.len()
                        && chars[i + 1..end] == n[..]
                        && (end == chars.len() || !is_word(chars[end]))
                    {
                        first = Some(i);
                        break;
                    }
                }
                if first.is_some() {
                    break;
                }
            }
            let (coef_text, wedge_text) = match first {
                Some(p) => (
                    chars[..p].iter().collect::<String>(),
                    chars[p..].iter().collect::<String>(),
                ),
                None => (piece.clone(), String::new()),
            };
            let coef_text = coef_text.trim().trim_end_matches('*').trim().to_string();
            // A bare `d` + one letter that is not a coordinate is a mistyped differential, not a
            // coefficient symbol.
            let cs: Vec<char> = coef_text.chars().collect();
            for i in 0..cs.len() {
                let word_start = i == 0 || !(cs[i - 1].is_alphanumeric() || cs[i - 1] == '_');
                let word_end =
                    i + 2 >= cs.len() || !(cs[i + 2].is_alphanumeric() || cs[i + 2] == '_');
                if cs[i] == 'd'
                    && word_start
                    && i + 1 < cs.len()
                    && cs[i + 1].is_alphabetic()
                    && word_end
                {
                    return Err(fail(format!(
                        "unknown coordinate `{}` in `d{}`",
                        cs[i + 1],
                        cs[i + 1]
                    )));
                }
            }
            let coefficient = if coef_text.is_empty() {
                g.integer(1)
            } else {
                parser
                    .parse(&insert_implicit_products(&coef_text))
                    .map_err(|e| fail(format!("cannot read coefficient `{coef_text}`: {e:?}")))?
            };
            let mut indices = Vec::new();
            for factor in wedge_text.split(['^', '*']) {
                let factor = factor.trim();
                if factor.is_empty() {
                    if wedge_text.is_empty() {
                        break;
                    }
                    return Err(fail("a wedge needs a differential on both sides".into()));
                }
                let name = factor
                    .strip_prefix('d')
                    .ok_or_else(|| fail(format!("cannot read `{factor}` as a differential")))?;
                let index = coord_names
                    .iter()
                    .position(|c| c == name)
                    .ok_or_else(|| fail(format!("unknown coordinate `{name}`")))?;
                indices.push(index);
            }
            let term = Self::term(
                g,
                &coords,
                signed(g, coefficient, if negative { -1 } else { 1 }),
                &indices,
            )?;
            let term = if term.terms.is_empty() {
                Self::zero(&coords, indices.len())
            } else {
                term
            };
            form = Some(match form {
                None => term,
                Some(acc) => acc.add(g, &term)?,
            });
        }
        form.ok_or_else(|| fail("no terms".into()))
    }

    /// Render as `coefficient dx∧dy + ...` with each coefficient simplified; blades whose
    /// coefficient simplifies to zero are dropped, and the zero form is `0`.
    pub fn to_text(&self, g: &ExprGraph) -> String {
        use algebra_core::format::{Formatter, UnicodeFormatter};
        let names: Vec<String> = self
            .coords
            .iter()
            .map(|&c| match g.get(c).kind {
                ExprKind::Symbol(id) => g.symbols.resolve(id).unwrap_or_default(),
                _ => "?".to_string(),
            })
            .collect();
        let mut parts: Vec<String> = Vec::new();
        for (basis, &c) in &self.terms {
            let simple = crate::simplify::Simplifier::simplify(g, c).unwrap_or(c);
            if matches!(&g.get(simple).kind, ExprKind::Number(n) if n.is_zero()) {
                continue;
            }
            let coefficient = UnicodeFormatter
                .format(g, simple)
                .unwrap_or_else(|_| "?".to_string());
            let blade: Vec<String> = basis
                .indices
                .iter()
                .map(|&i| format!("d{}", names[i]))
                .collect();
            parts.push(if blade.is_empty() {
                coefficient
            } else {
                format!("({coefficient}) {}", blade.join("∧"))
            });
        }
        if parts.is_empty() {
            "0".to_string()
        } else {
            parts.join(" + ")
        }
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
                Some("abs") => x.abs(),
                other => return Err(error(&format!("unsupported function {other:?}"))),
            }
        }
        other => return Err(error(&format!("unsupported expression node {other:?}"))),
    })
}

/// Make a product written by juxtaposition explicit: whitespace between two operands becomes `*`,
/// so `2 x y` reads as `2*x*y` and `(a + b) (c - d)` as `(a + b)*(c - d)`. Only whitespace
/// triggers it: `sin(x)` and `x^2` are untouched, and so is whitespace next to an operator
/// (`x + 1`, `x ^ 2`).
fn insert_implicit_products(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let ends_operand = |c: char| c.is_alphanumeric() || c == '_' || c == ')' || c == '.';
    let starts_operand = |c: char| c.is_alphanumeric() || c == '_' || c == '(' || c == '.';
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            let mut j = i;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            let prev = out.chars().last();
            let next = chars.get(j).copied();
            match (prev, next) {
                (Some(p), Some(n)) if ends_operand(p) && starts_operand(n) => out.push('*'),
                _ => {}
            }
            i = j;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// Determinant of a square matrix of expressions by cofactor expansion along the first row
/// (the empty matrix has determinant 1).
fn determinant(g: &ExprGraph, m: &[Vec<ExprId>]) -> ExprId {
    match m.len() {
        0 => g.integer(1),
        1 => m[0][0],
        n => {
            let terms: Vec<ExprId> = (0..n)
                .map(|j| {
                    let minor = determinant(g, &submatrix(m, &[0], &[j]));
                    signed(g, g.mul([m[0][j], minor]), if j % 2 == 0 { 1 } else { -1 })
                })
                .collect();
            g.add(terms)
        }
    }
}

/// `m` without the listed rows and columns.
fn submatrix(m: &[Vec<ExprId>], drop_rows: &[usize], drop_cols: &[usize]) -> Vec<Vec<ExprId>> {
    m.iter()
        .enumerate()
        .filter(|(i, _)| !drop_rows.contains(i))
        .map(|(_, row)| {
            row.iter()
                .enumerate()
                .filter(|(j, _)| !drop_cols.contains(j))
                .map(|(_, &v)| v)
                .collect()
        })
        .collect()
}

/// The submatrix of `m` with exactly the listed rows and columns, in the given order.
fn submatrix_keep(m: &[Vec<ExprId>], rows: &[usize], cols: &[usize]) -> Vec<Vec<ExprId>> {
    rows.iter()
        .map(|&i| cols.iter().map(|&j| m[i][j]).collect())
        .collect()
}

/// All sorted `k`-element subsets of `0..n`.
fn subsets(n: usize, k: usize) -> Vec<Vec<usize>> {
    fn go(start: usize, n: usize, k: usize, current: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if current.len() == k {
            out.push(current.clone());
            return;
        }
        for i in start..n {
            current.push(i);
            go(i + 1, n, k, current, out);
            current.pop();
        }
    }
    let mut out = Vec::new();
    go(0, n, k, &mut Vec::new(), &mut out);
    out
}
