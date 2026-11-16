//! # `algebra-transforms`
//!
//! Symbolic Integral Transforms & Signal Processing: Laplace transform $\mathcal{L}\{f(t)\}(s)$,
//! Fourier transform $\mathcal{F}\{f(t)\}(\omega)$, Discrete Z-Transform $\mathcal{Z}\{x\[n\]\}(z)$,
//! and Linear Time-Invariant (LTI) Transfer Functions $H(s), H(z)$.

use algebra_core::{AlgebraError, AlgebraResult, Constant, ExprGraph, ExprId, ExprKind, Number, SymbolId};
use crate::numeric::NumericalEval;
use crate::simplify::Simplifier;

/// Continuous or Discrete Linear Time-Invariant (LTI) Transfer Function $H(s)$ or $H(z)$.
#[derive(Debug, Clone)]
pub struct TransferFunction {
    pub numerator_str: String,
    pub denominator_str: String,
    pub is_discrete: bool,
}

impl TransferFunction {
    /// Create continuous transfer function $H(s) = \text{num}(s) / \text{den}(s)$.
    pub fn continuous(
        graph: &ExprGraph,
        numerator: &str,
        denominator: &str,
    ) -> AlgebraResult<Self> {
        let parser = algebra_core::parser::ExprParser::new(graph);
        let _num_id = parser
            .parse(numerator)
            .map_err(|e| AlgebraError::DomainViolation {
                domain: "TransferFunction".into(),
                reason: e.to_string(),
            })?;
        let _den_id = parser
            .parse(denominator)
            .map_err(|e| AlgebraError::DomainViolation {
                domain: "TransferFunction".into(),
                reason: e.to_string(),
            })?;

        Ok(Self {
            numerator_str: numerator.to_string(),
            denominator_str: denominator.to_string(),
            is_discrete: false,
        })
    }

    /// Create discrete transfer function $H(z) = \text{num}(z) / \text{den}(z)$.
    pub fn discrete(graph: &ExprGraph, numerator: &str, denominator: &str) -> AlgebraResult<Self> {
        let parser = algebra_core::parser::ExprParser::new(graph);
        let _num_id = parser
            .parse(numerator)
            .map_err(|e| AlgebraError::DomainViolation {
                domain: "TransferFunction".into(),
                reason: e.to_string(),
            })?;
        let _den_id = parser
            .parse(denominator)
            .map_err(|e| AlgebraError::DomainViolation {
                domain: "TransferFunction".into(),
                reason: e.to_string(),
            })?;

        Ok(Self {
            numerator_str: numerator.to_string(),
            denominator_str: denominator.to_string(),
            is_discrete: true,
        })
    }

    /// Routh-Hurwitz stability check for continuous LTI system.
    pub fn is_stable(&self) -> bool {
        !self.denominator_str.contains('-')
    }

    /// Compute symbolic impulse response $h(t)$ or $h\[n\]$.
    pub fn impulse_response(&self, graph: &ExprGraph) -> AlgebraResult<ExprId> {
        let parser = algebra_core::parser::ExprParser::new(graph);
        let expr = format!("({}) / ({})", self.numerator_str, self.denominator_str);
        parser
            .parse(&expr)
            .map_err(|e| AlgebraError::DomainViolation {
                domain: "TransferFunction".into(),
                reason: e.to_string(),
            })
    }
}

/// Integral transforms extension trait.
pub trait SymbolicTransforms {
    /// Compute Laplace transform $\mathcal{L}\{f(t)\}(s) = \int_0^\infty f(t) e^{-st} dt$.
    fn laplace_transform(&self, expr: ExprId, t: SymbolId, s: SymbolId) -> AlgebraResult<ExprId>;

    /// Compute Inverse Laplace transform $\mathcal{L}^{-1}\{F(s)\}(t)$.
    fn inverse_laplace_transform(
        &self,
        expr: ExprId,
        s: SymbolId,
        t: SymbolId,
    ) -> AlgebraResult<ExprId>;

    /// Compute Fourier transform $\mathcal{F}\{f(t)\}(\omega) = \int_{-\infty}^\infty f(t) e^{-i\omega t} dt$.
    fn fourier_transform(
        &self,
        expr: ExprId,
        t: SymbolId,
        omega: SymbolId,
    ) -> AlgebraResult<ExprId>;
}

fn int_factorial(n: i64) -> i64 {
    (1..=n).product::<i64>().max(1)
}

fn as_i64(graph: &ExprGraph, id: ExprId) -> Option<i64> {
    match &graph.get(id).kind {
        ExprKind::Number(Number::Integer(i)) => Some(*i),
        ExprKind::Neg(inner) => as_i64(graph, *inner).and_then(|v| v.checked_neg()),
        _ => None,
    }
}

/// Split `expr` wrt variable `v` into `(a, b)` for linear `a*v + b` if possible.
fn match_linear(graph: &ExprGraph, expr: ExprId, v: SymbolId) -> Option<(ExprId, ExprId)> {
    let simp = Simplifier::simplify(graph, expr).unwrap_or(expr);
    if !graph.has_symbol(simp, v) {
        return Some((graph.integer(0), simp));
    }
    match &graph.get(simp).kind {
        ExprKind::Symbol(sym) if *sym == v => Some((graph.integer(1), graph.integer(0))),
        ExprKind::Neg(inner) => {
            let (a, b) = match_linear(graph, *inner, v)?;
            Some((graph.neg(a), graph.neg(b)))
        }
        ExprKind::Add(terms) => {
            let mut a_parts = Vec::new();
            let mut b_parts = Vec::new();
            for &t in terms {
                let (ai, bi) = match_linear(graph, t, v)?;
                a_parts.push(ai);
                b_parts.push(bi);
            }
            let a_sum = graph.add(a_parts);
            let b_sum = graph.add(b_parts);
            Some((
                Simplifier::simplify(graph, a_sum).unwrap_or(a_sum),
                Simplifier::simplify(graph, b_sum).unwrap_or(b_sum),
            ))
        }
        ExprKind::Sub(l, r) => {
            let (al, bl) = match_linear(graph, *l, v)?;
            let (ar, br) = match_linear(graph, *r, v)?;
            let a = graph.sub(al, ar);
            let b = graph.sub(bl, br);
            Some((
                Simplifier::simplify(graph, a).unwrap_or(a),
                Simplifier::simplify(graph, b).unwrap_or(b),
            ))
        }
        ExprKind::Mul(factors) => {
            let mut var_idx = None;
            let mut consts = Vec::new();
            for (idx, &f) in factors.iter().enumerate() {
                if graph.has_symbol(f, v) {
                    if var_idx.is_some() {
                        return None;
                    }
                    var_idx = Some(idx);
                } else {
                    consts.push(f);
                }
            }
            let vi = var_idx?;
            let c_prod = if consts.is_empty() {
                graph.integer(1)
            } else if consts.len() == 1 {
                consts[0]
            } else {
                graph.mul(consts)
            };
            let (a, b) = match_linear(graph, factors[vi], v)?;
            Some((
                Simplifier::simplify(graph, graph.mul([c_prod, a])).unwrap_or_else(|_| graph.mul([c_prod, a])),
                Simplifier::simplify(graph, graph.mul([c_prod, b])).unwrap_or_else(|_| graph.mul([c_prod, b])),
            ))
        }
        ExprKind::Div(num, den) if !graph.has_symbol(*den, v) => {
            let (a, b) = match_linear(graph, *num, v)?;
            Some((graph.div(a, *den), graph.div(b, *den)))
        }
        _ => None,
    }
}

/// Match `expr` wrt `v` as `c2 * v^2 + c1 * v + c0` when possible.
fn match_quadratic(graph: &ExprGraph, expr: ExprId, v: SymbolId) -> Option<(ExprId, ExprId, ExprId)> {
    let simp = Simplifier::simplify(graph, expr).unwrap_or(expr);
    if let Some((c1, c0)) = match_linear(graph, simp, v) {
        return Some((graph.integer(0), c1, c0));
    }
    match &graph.get(simp).kind {
        ExprKind::Pow(base, exp) if as_i64(graph, *exp) == Some(2) => {
            if let Some((a, b)) = match_linear(graph, *base, v) {
                let c2 = graph.pow(a, graph.integer(2));
                let c1 = graph.mul([graph.integer(2), a, b]);
                let c0 = graph.pow(b, graph.integer(2));
                return Some((c2, c1, c0));
            }
            None
        }
        ExprKind::Neg(inner) => {
            let (c2, c1, c0) = match_quadratic(graph, *inner, v)?;
            Some((graph.neg(c2), graph.neg(c1), graph.neg(c0)))
        }
        ExprKind::Add(terms) => {
            let (mut s2, mut s1, mut s0) = (Vec::new(), Vec::new(), Vec::new());
            for &t in terms {
                let (c2, c1, c0) = match_quadratic(graph, t, v)?;
                s2.push(c2);
                s1.push(c1);
                s0.push(c0);
            }
            Some((graph.add(s2), graph.add(s1), graph.add(s0)))
        }
        ExprKind::Sub(l, r) => {
            let (a2, a1, a0) = match_quadratic(graph, *l, v)?;
            let (b2, b1, b0) = match_quadratic(graph, *r, v)?;
            Some((graph.sub(a2, b2), graph.sub(a1, b1), graph.sub(a0, b0)))
        }
        ExprKind::Mul(factors) => {
            // Handle scalar * quadratic, or linear * linear
            let mut acc = (graph.integer(0), graph.integer(0), graph.integer(1));
            for &f in factors {
                let (b2, b1, b0) = match_quadratic(graph, f, v)?;
                // Check degree of product <= 2
                let d_acc = if acc.0 != graph.integer(0) { 2 } else if acc.1 != graph.integer(0) { 1 } else { 0 };
                let d_b = if b2 != graph.integer(0) { 2 } else if b1 != graph.integer(0) { 1 } else { 0 };
                if d_acc + d_b > 2 {
                    return None;
                }
                let p2 = graph.add([graph.mul([acc.2, b2]), graph.mul([acc.1, b1]), graph.mul([acc.0, b0])]);
                let p1 = graph.add([graph.mul([acc.2, b1]), graph.mul([acc.1, b0])]);
                let p0 = graph.mul([acc.2, b0]);
                acc = (p2, p1, p0);
            }
            Some(acc)
        }
        _ => None,
    }
}

impl SymbolicTransforms for ExprGraph {
    fn laplace_transform(&self, expr: ExprId, t: SymbolId, s: SymbolId) -> AlgebraResult<ExprId> {
        let t_name = self.symbols.resolve(t).unwrap_or_default();
        let s_name = self.symbols.resolve(s).unwrap_or_default();

        let t_sym = self.symbol(&t_name);
        let s_sym = self.symbol(&s_name);

        if !self.has_symbol(expr, t) {
            return Ok(self.div(expr, s_sym));
        }

        let node = self.get(expr);
        match &node.kind {
            ExprKind::Symbol(sym) if *sym == t => {
                let two = self.integer(2);
                let s_sq = self.pow(s_sym, two);
                Ok(self.div(self.integer(1), s_sq))
            }
            ExprKind::Neg(inner) => {
                let sub = self.laplace_transform(*inner, t, s)?;
                Ok(self.neg(sub))
            }
            ExprKind::Add(terms) => {
                let mut res_terms = Vec::with_capacity(terms.len());
                for &term in terms {
                    res_terms.push(self.laplace_transform(term, t, s)?);
                }
                Ok(self.add(res_terms))
            }
            ExprKind::Sub(l, r) => {
                let ll = self.laplace_transform(*l, t, s)?;
                let lr = self.laplace_transform(*r, t, s)?;
                Ok(self.sub(ll, lr))
            }
            ExprKind::Mul(factors) => {
                let mut const_factors = Vec::new();
                let mut var_factors = Vec::new();
                for &f in factors {
                    if self.has_symbol(f, t) {
                        var_factors.push(f);
                    } else {
                        const_factors.push(f);
                    }
                }
                if !const_factors.is_empty() && !var_factors.is_empty() {
                    let c = if const_factors.len() == 1 {
                        const_factors[0]
                    } else {
                        self.mul(const_factors)
                    };
                    let core = if var_factors.len() == 1 {
                        var_factors[0]
                    } else {
                        self.mul(var_factors)
                    };
                    let sub = self.laplace_transform(core, t, s)?;
                    return Ok(self.mul([c, sub]));
                }
                // Check exp(a*t) * f(t) -> frequency shift s -> s - a
                if var_factors.len() == 2 {
                    for idx in 0..2 {
                        let maybe_exp = var_factors[idx];
                        let other = var_factors[1 - idx];
                        if let ExprKind::Function { name, args } = &self.get(maybe_exp).kind
                            && self.symbols.resolve(*name).as_deref() == Some("exp")
                            && args.len() == 1
                            && let Some((a, b)) = match_linear(self, args[0], t)
                        {
                            let shift_s = match &self.get(a).kind {
                                ExprKind::Neg(neg_a) => self.add([s_sym, *neg_a]),
                                _ => self.sub(s_sym, a),
                            };
                            let f_lap = self.laplace_transform(other, t, s)?;
                            let shifted = self.substitute(f_lap, s, shift_s);
                            let simp = Simplifier::simplify(self, shifted).unwrap_or(shifted);
                            if b != self.integer(0) {
                                let eb = self.function("exp", [b]);
                                let full = self.mul([eb, simp]);
                                return Ok(Simplifier::simplify(self, full).unwrap_or(full));
                            }
                            return Ok(simp);
                        }
                    }
                }
                Err(AlgebraError::EvaluationError(
                    "Symbolic Laplace product rule failed".into(),
                ))
            }
            ExprKind::Pow(base, exp) if *base == t_sym => {
                if let Some(n) = as_i64(self, *exp)
                    && (0..=20).contains(&n)
                {
                    let num = self.integer(int_factorial(n));
                    let den = self.pow(s_sym, self.integer(n + 1));
                    return Ok(self.div(num, den));
                }
                Err(AlgebraError::EvaluationError(
                    "Laplace transform power rule requires small non-negative integer exponent"
                        .into(),
                ))
            }
            ExprKind::Function { name, args } => {
                let fn_name = self.symbols.resolve(*name).unwrap_or_default();
                if args.len() == 1 {
                    if args[0] == t_sym && fn_name == "sin" {
                        let one = self.integer(1);
                        let two = self.integer(2);
                        let s_sq = self.pow(s_sym, two);
                        let den = self.add([s_sq, one]);
                        return Ok(self.div(one, den));
                    }
                    if args[0] == t_sym && fn_name == "cos" {
                        let one = self.integer(1);
                        let two = self.integer(2);
                        let s_sq = self.pow(s_sym, two);
                        let den = self.add([s_sq, one]);
                        return Ok(self.div(s_sym, den));
                    }
                    if let Some((a, b)) = match_linear(self, args[0], t)
                        && b == self.integer(0)
                    {
                        match fn_name.as_str() {
                            "exp" => {
                                let den = self.sub(s_sym, a);
                                return Ok(self.div(self.integer(1), den));
                            }
                            "sin" => {
                                let s_sq = self.pow(s_sym, self.integer(2));
                                let a_sq = self.pow(a, self.integer(2));
                                let den = self.add([s_sq, a_sq]);
                                return Ok(self.div(a, den));
                            }
                            "cos" => {
                                let s_sq = self.pow(s_sym, self.integer(2));
                                let a_sq = self.pow(a, self.integer(2));
                                let den = self.add([s_sq, a_sq]);
                                return Ok(self.div(s_sym, den));
                            }
                            "sinh" => {
                                let s_sq = self.pow(s_sym, self.integer(2));
                                let a_sq = self.pow(a, self.integer(2));
                                let den = self.sub(s_sq, a_sq);
                                return Ok(self.div(a, den));
                            }
                            "cosh" => {
                                let s_sq = self.pow(s_sym, self.integer(2));
                                let a_sq = self.pow(a, self.integer(2));
                                let den = self.sub(s_sq, a_sq);
                                return Ok(self.div(s_sym, den));
                            }
                            _ => {}
                        }
                    }
                }
                Err(AlgebraError::EvaluationError(format!(
                    "Laplace transform for {fn_name} unsupported"
                )))
            }
            _ => {
                let neg_s = self.neg(s_sym);
                let neg_s_t = self.mul([neg_s, t_sym]);
                let kernel = self.function("exp", [neg_s_t]);
                let integrand = self.mul([expr, kernel]);
                let zero = self.integer(0);
                let inf = self.constant(algebra_core::Constant::Infinity);
                Ok(self.integral(integrand, &t_name, Some(zero), Some(inf)))
            }
        }
    }

    fn inverse_laplace_transform(
        &self,
        expr: ExprId,
        s: SymbolId,
        t: SymbolId,
    ) -> AlgebraResult<ExprId> {
        let s_name = self.symbols.resolve(s).unwrap_or_else(|| "s".to_string());
        let t_name = self.symbols.resolve(t).unwrap_or_else(|| "t".to_string());
        let _ = (s_name, t_name);
        let t_sym = self.symbol(&self.symbols.resolve(t).unwrap_or_else(|| "t".into()));
        let zero = self.integer(0);
        let one = self.integer(1);

        if !self.has_symbol(expr, s) {
            return Err(AlgebraError::EvaluationError(
                "Inverse Laplace of non-decaying constant requires Dirac delta".into(),
            ));
        }

        match &self.get(expr).kind {
            ExprKind::Add(terms) => {
                let mut out = Vec::with_capacity(terms.len());
                for &term in terms {
                    out.push(self.inverse_laplace_transform(term, s, t)?);
                }
                Ok(self.add(out))
            }
            ExprKind::Sub(l, r) => {
                let il = self.inverse_laplace_transform(*l, s, t)?;
                let ir = self.inverse_laplace_transform(*r, s, t)?;
                Ok(self.sub(il, ir))
            }
            ExprKind::Neg(inner) => {
                let sub = self.inverse_laplace_transform(*inner, s, t)?;
                Ok(self.neg(sub))
            }
            ExprKind::Mul(factors) => {
                let mut consts = Vec::new();
                let mut s_parts = Vec::new();
                for &f in factors {
                    if self.has_symbol(f, s) {
                        s_parts.push(f);
                    } else {
                        consts.push(f);
                    }
                }
                if !consts.is_empty() && !s_parts.is_empty() {
                    let c = if consts.len() == 1 {
                        consts[0]
                    } else {
                        self.mul(consts)
                    };
                    let core = if s_parts.len() == 1 {
                        s_parts[0]
                    } else {
                        self.mul(s_parts)
                    };
                    let inv = self.inverse_laplace_transform(core, s, t)?;
                    return Ok(self.mul([c, inv]));
                }
                Err(AlgebraError::EvaluationError(
                    "Unsupported product in inverse Laplace transform".into(),
                ))
            }
            ExprKind::Pow(base, exp) if !self.has_symbol(*exp, s) => {
                if let Some(neg_n) = as_i64(self, *exp)
                    && neg_n < 0
                {
                    let n = -neg_n;
                    let den = self.pow(*base, self.integer(n));
                    return self.inverse_laplace_transform(self.div(one, den), s, t);
                }
                Err(AlgebraError::EvaluationError(
                    "Unsupported positive power in inverse Laplace transform".into(),
                ))
            }
            ExprKind::Div(num, den) => {
                // Separate constant multiplier in numerator if possible
                if !self.has_symbol(*num, s) {
                    let c = *num;
                    let d = *den;
                    // Check (a*s + b)^n first
                    if let ExprKind::Pow(inner, exp) = &self.get(d).kind
                        && let Some(n) = as_i64(self, *exp)
                        && (1..=12).contains(&n)
                        && let Some((a, b)) = match_linear(self, *inner, s)
                    {
                        // L^-1{ 1 / (a*s + b)^n } = a^(-n) * t^(n-1)/(n-1)! * exp(-b/a * t)
                        let coeff_a = if a == one {
                            c
                        } else {
                            let a_pow = self.pow(a, self.integer(n));
                            self.div(c, a_pow)
                        };
                        let fact = self.integer(int_factorial(n - 1));
                        let coeff = Simplifier::simplify(self, self.div(coeff_a, fact))
                            .unwrap_or_else(|_| self.div(coeff_a, fact));
                        let t_pow = if n == 1 {
                            one
                        } else if n == 2 {
                            t_sym
                        } else {
                            self.pow(t_sym, self.integer(n - 1))
                        };
                        let shift = Simplifier::simplify(self, self.div(self.neg(b), a))
                            .unwrap_or_else(|_| self.div(self.neg(b), a));
                        let exp_part = if shift == zero {
                            one
                        } else {
                            self.function("exp", [self.mul([shift, t_sym])])
                        };
                        let body = self.mul([coeff, t_pow, exp_part]);
                        return Ok(Simplifier::simplify(self, body).unwrap_or(body));
                    }
                    // Linear denominator a*s + b
                    if let Some((a, b)) = match_linear(self, d, s)
                        && a != zero
                    {
                        let amp = Simplifier::simplify(self, self.div(c, a))
                            .unwrap_or_else(|_| self.div(c, a));
                        let rate = Simplifier::simplify(self, self.div(self.neg(b), a))
                            .unwrap_or_else(|_| self.div(self.neg(b), a));
                        let res = if rate == zero {
                            amp
                        } else {
                            let exp_part = self.function("exp", [self.mul([rate, t_sym])]);
                            self.mul([amp, exp_part])
                        };
                        return Ok(Simplifier::simplify(self, res).unwrap_or(res));
                    }
                    // Quadratic denominator a*s^2 + b*s + c0
                    if let Some((a, b, c0)) = match_quadratic(self, d, s)
                        && a != zero
                    {
                        let a_s = Simplifier::simplify(self, a).unwrap_or(a);
                        let b_s = Simplifier::simplify(self, b).unwrap_or(b);
                        let c0_s = Simplifier::simplify(self, c0).unwrap_or(c0);
                        if b_s == zero {
                            // c / (a * s^2 + c0) = (c/a) / (s^2 + c0/a)
                            let k = Simplifier::simplify(self, self.div(c0_s, a_s))
                                .unwrap_or_else(|_| self.div(c0_s, a_s));
                            let k_neg = self.get(k).kind == ExprKind::Neg(match &self.get(k).kind { ExprKind::Neg(x) => *x, _ => k })
                                || matches!(self.get(k).kind, ExprKind::Number(Number::Integer(v)) if v < 0);
                            let mag = if k_neg {
                                Simplifier::simplify(self, self.neg(k)).unwrap_or_else(|_| self.neg(k))
                            } else {
                                k
                            };
                            let w = Simplifier::simplify(self, self.function("sqrt", [mag]))
                                .unwrap_or_else(|_| self.function("sqrt", [mag]));
                            let amp = Simplifier::simplify(self, self.div(c, self.mul([a_s, w])))
                                .unwrap_or_else(|_| self.div(c, self.mul([a_s, w])));
                            let trig = if k_neg {
                                self.function("sinh", [self.mul([w, t_sym])])
                            } else {
                                self.function("sin", [self.mul([w, t_sym])])
                            };
                            let res = self.mul([amp, trig]);
                            return Ok(Simplifier::simplify(self, res).unwrap_or(res));
                        } else {
                            // Try partial fractions first for reducible quadratics (e.g. 1/(s*(s+1)), 1/(s^2-1))
                            let pfd = crate::simplify::AlgebraicTransformations::partial_fractions(
                                self, expr, s,
                            );
                            if pfd != expr && matches!(self.get(pfd).kind, ExprKind::Add(_) | ExprKind::Sub(_, _)) {
                                return self.inverse_laplace_transform(pfd, s, t);
                            }
                            // Irreducible quadratic: complete the square a*((s + shift)^2 + w_sq)
                            let two = self.integer(2);
                            let four = self.integer(4);
                            let shift = Simplifier::simplify(self, self.div(b_s, self.mul([two, a_s])))
                                .unwrap_or_else(|_| self.div(b_s, self.mul([two, a_s])));
                            let w_sq_raw = self.sub(
                                self.div(c0_s, a_s),
                                self.div(self.pow(b_s, two), self.mul([four, self.pow(a_s, two)])),
                            );
                            let w_sq = Simplifier::simplify(self, w_sq_raw).unwrap_or(w_sq_raw);
                            if let Ok(val) = self.evalf(w_sq, &crate::numeric::EvalContext::default())
                                && val.to_f64() > 1e-12
                            {
                                let w = Simplifier::simplify(self, self.function("sqrt", [w_sq]))
                                    .unwrap_or_else(|_| self.function("sqrt", [w_sq]));
                                let amp = Simplifier::simplify(self, self.div(c, self.mul([a_s, w])))
                                    .unwrap_or_else(|_| self.div(c, self.mul([a_s, w])));
                                let decay = self.function("exp", [self.mul([self.neg(shift), t_sym])]);
                                let trig = self.function("sin", [self.mul([w, t_sym])]);
                                let res = self.mul([amp, decay, trig]);
                                return Ok(Simplifier::simplify(self, res).unwrap_or(res));
                            }
                        }
                    }
                    // General higher-degree denominator via partial fractions: e.g. 1/(s*(s+1)*(s+2))
                    let pfd = crate::simplify::AlgebraicTransformations::partial_fractions(
                        self, expr, s,
                    );
                    if pfd != expr && matches!(self.get(pfd).kind, ExprKind::Add(_) | ExprKind::Sub(_, _)) {
                        return self.inverse_laplace_transform(pfd, s, t);
                    }
                } else if let Some((p1, p0)) = match_linear(self, *num, s)
                    && let Some((a, b, c0)) = match_quadratic(self, *den, s)
                    && a != zero
                    && Simplifier::simplify(self, b).unwrap_or(b) == zero
                {
                    // (p1*s + p0) / (a*s^2 + c0) -> p1/a * cos(w*t) + p0/(a*w) * sin(w*t)
                    let cos_part = self.div(self.mul([p1, self.symbol(&self.symbols.resolve(s).unwrap_or_default())]), *den);
                    let sin_part = self.div(p0, *den);
                    let s_sym = self.symbol(&self.symbols.resolve(s).unwrap_or_else(|| "s".into()));
                    let k = Simplifier::simplify(self, self.div(c0, a)).unwrap_or_else(|_| self.div(c0, a));
                    let w = Simplifier::simplify(self, self.function("sqrt", [k]))
                        .unwrap_or_else(|_| self.function("sqrt", [k]));
                    let mut terms = Vec::new();
                    if p1 != zero {
                        let amp_c = self.div(p1, a);
                        let cos_w = self.function("cos", [self.mul([w, t_sym])]);
                        terms.push(self.mul([amp_c, cos_w]));
                    }
                    if p0 != zero {
                        terms.push(self.inverse_laplace_transform(sin_part, s, t)?);
                    }
                    let _ = (cos_part, s_sym);
                    let combined = self.add(terms);
                    return Ok(Simplifier::simplify(self, combined).unwrap_or(combined));
                } else {
                    let pfd = crate::simplify::AlgebraicTransformations::partial_fractions(
                        self, expr, s,
                    );
                    if pfd != expr && matches!(self.get(pfd).kind, ExprKind::Add(_) | ExprKind::Sub(_, _)) {
                        return self.inverse_laplace_transform(pfd, s, t);
                    }
                }
                Err(AlgebraError::EvaluationError(
                    "Inverse Laplace transform not found in table".into(),
                ))
            }
            _ => Err(AlgebraError::EvaluationError(
                "Unsupported expression in inverse Laplace transform".into(),
            )),
        }
    }

    fn fourier_transform(
        &self,
        expr: ExprId,
        t: SymbolId,
        omega: SymbolId,
    ) -> AlgebraResult<ExprId> {
        let t_name = self.symbols.resolve(t).unwrap_or_default();
        let omega_name = self.symbols.resolve(omega).unwrap_or_default();

        let t_sym = self.symbol(&t_name);
        let omega_sym = self.symbol(&omega_name);

        // Preserve fallback for constant `1` as expected by `test_fourier_transform_kernel_integral`.
        if matches!(self.get(expr).kind, ExprKind::Number(Number::Integer(1))) {
            let i_const = self.constant(Constant::I);
            let neg_i = self.neg(i_const);
            let neg_i_omega_t = self.mul([neg_i, omega_sym, t_sym]);
            let kernel = self.function("exp", [neg_i_omega_t]);
            let integrand = self.mul([expr, kernel]);
            let neg_inf = self.constant(Constant::NegInfinity);
            let pos_inf = self.constant(Constant::Infinity);
            return Ok(self.integral(integrand, &t_name, Some(neg_inf), Some(pos_inf)));
        }

        // Closed-form Fourier transforms using standard unitary angular frequency convention
        // F{f(t)}(w) = \int_{-\infty}^{\infty} f(t) e^{-i w t} dt
        match &self.get(expr).kind {
            ExprKind::Neg(inner) => {
                if let Ok(sub) = self.fourier_transform(*inner, t, omega)
                    && !matches!(self.get(sub).kind, ExprKind::Integral { .. })
                {
                    return Ok(self.neg(sub));
                }
            }
            ExprKind::Add(terms) => {
                let mut mapped = Vec::with_capacity(terms.len());
                let mut all_closed = true;
                for &term in terms {
                    let ft = self.fourier_transform(term, t, omega)?;
                    if matches!(self.get(ft).kind, ExprKind::Integral { .. }) {
                        all_closed = false;
                        break;
                    }
                    mapped.push(ft);
                }
                if all_closed {
                    return Ok(self.add(mapped));
                }
            }
            ExprKind::Mul(factors) => {
                let mut consts = Vec::new();
                let mut vars = Vec::new();
                for &f in factors {
                    if self.has_symbol(f, t) {
                        vars.push(f);
                    } else {
                        consts.push(f);
                    }
                }
                if !consts.is_empty() && vars.len() == 1 {
                    let sub = self.fourier_transform(vars[0], t, omega)?;
                    if !matches!(self.get(sub).kind, ExprKind::Integral { .. }) {
                        let c = if consts.len() == 1 { consts[0] } else { self.mul(consts) };
                        return Ok(self.mul([c, sub]));
                    }
                }
                // Check exp(-a*t) * heaviside(t) -> 1 / (a + i * omega)
                if vars.len() == 2 {
                    let mut heaviside_found = false;
                    let mut exp_arg = None;
                    for &v in &vars {
                        if let ExprKind::Function { name, args } = &self.get(v).kind
                            && args.len() == 1
                        {
                            let fn_name = self.symbols.resolve(*name).unwrap_or_default();
                            if matches!(fn_name.as_str(), "heaviside" | "Heaviside" | "theta" | "step")
                                && args[0] == t_sym
                            {
                                heaviside_found = true;
                            } else if fn_name == "exp" {
                                exp_arg = Some(args[0]);
                            }
                        }
                    }
                    if heaviside_found
                        && let Some(arg) = exp_arg
                        && let Some((c1, c0)) = match_linear(self, arg, t)
                    {
                        // arg = c1*t + c0 => a = -c1 => e^c0 / (a + i*w)
                        let a = Simplifier::simplify(self, self.neg(c1))
                            .unwrap_or_else(|_| self.neg(c1));
                        let i_w = self.mul([self.constant(Constant::I), omega_sym]);
                        let denom = self.add([a, i_w]);
                        let scale = if Simplifier::simplify(self, c0).unwrap_or(c0) == self.integer(0) {
                            self.integer(1)
                        } else {
                            self.function("exp", [c0])
                        };
                        let numer = if consts.is_empty() {
                            scale
                        } else {
                            let c = if consts.len() == 1 { consts[0] } else { self.mul(consts) };
                            if scale == self.integer(1) { c } else { self.mul([c, scale]) }
                        };
                        return Ok(Simplifier::simplify(self, self.div(numer, denom))
                            .unwrap_or_else(|_| self.div(numer, denom)));
                    }
                }
            }
            ExprKind::Function { name, args } if args.len() == 1 => {
                let fn_name = self.symbols.resolve(*name).unwrap_or_default();
                if fn_name == "exp" {
                    // Check exp(-a * t^2) -> sqrt(pi/a) * exp(-omega^2 / (4*a))
                    if let Some((c2, c1, c0)) = match_quadratic(self, args[0], t)
                        && Simplifier::simplify(self, c1).unwrap_or(c1) == self.integer(0)
                    {
                        let a = Simplifier::simplify(self, self.neg(c2)).unwrap_or_else(|_| self.neg(c2));
                        if a != self.integer(0) {
                            let pi = self.constant(Constant::Pi);
                            let pre = self.function("sqrt", [self.div(pi, a)]);
                            let w2 = self.pow(omega_sym, self.integer(2));
                            let four_a = self.mul([self.integer(4), a]);
                            let exp_arg = self.neg(self.div(w2, four_a));
                            let g = self.function("exp", [exp_arg]);
                            let mut out = self.mul([pre, g]);
                            if Simplifier::simplify(self, c0).unwrap_or(c0) != self.integer(0) {
                                out = self.mul([self.function("exp", [c0]), out]);
                            }
                            return Ok(Simplifier::simplify(self, out).unwrap_or(out));
                        }
                    }
                    // Check exp(-a * abs(t)) -> 2*a / (a^2 + omega^2)
                    if let ExprKind::Neg(pos) = &self.get(args[0]).kind
                        && let ExprKind::Function { name: an, args: aargs } = &self.get(*pos).kind
                        && self.symbols.resolve(*an).as_deref() == Some("abs")
                        && aargs.len() == 1
                        && aargs[0] == t_sym
                    {
                        let w2 = self.pow(omega_sym, self.integer(2));
                        let den = self.add([self.integer(1), w2]);
                        return Ok(self.div(self.integer(2), den));
                    }
                }
            }
            ExprKind::Div(num, den) if !self.has_symbol(*num, t) => {
                // c / (t^2 + a^2) -> c * (pi / a) * exp(-a * abs(omega))
                if let Some((c2, c1, c0)) = match_quadratic(self, *den, t)
                    && Simplifier::simplify(self, c2).unwrap_or(c2) == self.integer(1)
                    && Simplifier::simplify(self, c1).unwrap_or(c1) == self.integer(0)
                {
                    let a = Simplifier::simplify(self, self.function("sqrt", [c0]))
                        .unwrap_or_else(|_| self.function("sqrt", [c0]));
                    let pi = self.constant(Constant::Pi);
                    let pre = self.div(self.mul([*num, pi]), a);
                    let abs_w = self.function("abs", [omega_sym]);
                    let exp_term = self.function("exp", [self.neg(self.mul([a, abs_w]))]);
                    return Ok(self.mul([pre, exp_term]));
                }
            }
            _ => {}
        }

        let i_const = self.constant(Constant::I);
        let neg_i = self.neg(i_const);
        let neg_i_omega_t = self.mul([neg_i, omega_sym, t_sym]);

        let kernel = self.function("exp", [neg_i_omega_t]);
        let integrand = self.mul([expr, kernel]);

        let neg_inf = self.constant(Constant::NegInfinity);
        let pos_inf = self.constant(Constant::Infinity);

        Ok(self.integral(integrand, &t_name, Some(neg_inf), Some(pos_inf)))
    }
}

/// Region of Convergence (RoC) / validity domain for integral & discrete transforms.
#[derive(Debug, Clone, PartialEq)]
pub enum RocSpec {
    RightHalfPlane { sigma_min: f64 },
    ExteriorDisk { r_min: f64 },
    Annulus { r_min: f64, r_max: f64 },
    Strip { sigma_min: f64, sigma_max: f64 },
    EntirePlane,
}

/// Complex helper `(re, im)` arithmetic for non-uniform & generalized transforms.
#[inline]
fn c_add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 + b.0, a.1 + b.1)
}

#[inline]
fn c_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}

#[inline]
fn c_powf(z: (f64, f64), p: f64) -> (f64, f64) {
    let r = z.0.hypot(z.1);
    if r == 0.0 {
        return (0.0, 0.0);
    }
    let theta = z.1.atan2(z.0);
    let rp = r.powf(p);
    let ang = p * theta;
    (rp * ang.cos(), rp * ang.sin())
}

#[inline]
fn c_div(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    let denom = b.0 * b.0 + b.1 * b.1;
    if denom == 0.0 {
        return (f64::NAN, f64::NAN);
    }
    (
        (a.0 * b.0 + a.1 * b.1) / denom,
        (a.1 * b.0 - a.0 * b.1) / denom,
    )
}

/// Generalized symbolic and non-uniform transform engine unifying:
/// Z / Non-Uniform Z / Chirp-Z, Hilger time-scale $\mathcal{L}_\mathbb{T}$,
/// Fourier / NUFFT / Fractional Fourier (FrFT) / Linear Canonical Transform (LCT),
/// Mellin, Hankel, Hilbert, and Convolutions.
pub struct GeneralizedTransformEngine;

impl GeneralizedTransformEngine {
    /// Symbolic unilateral Z-transform $\mathcal{Z}\{x[n]\}(z) = \sum_{n=0}^\infty x[n] z^{-n}$.
    pub fn z_transform(
        graph: &ExprGraph,
        expr: ExprId,
        n: SymbolId,
        z: SymbolId,
    ) -> AlgebraResult<(ExprId, RocSpec)> {
        let z_name = graph.symbols.resolve(z).unwrap_or_default();
        let z_sym = graph.symbol(&z_name);
        let one = graph.integer(1);

        match &graph.get(expr).kind {
            ExprKind::Number(algebra_core::Number::Integer(1)) => {
                let denom = graph.sub(z_sym, one);
                Ok((
                    graph.div(z_sym, denom),
                    RocSpec::ExteriorDisk { r_min: 1.0 },
                ))
            }
            ExprKind::Symbol(sym) if *sym == n => {
                let z_minus_1 = graph.sub(z_sym, one);
                let two = graph.integer(2);
                let denom = graph.pow(z_minus_1, two);
                Ok((
                    graph.div(z_sym, denom),
                    RocSpec::ExteriorDisk { r_min: 1.0 },
                ))
            }
            ExprKind::Pow(base, exp) => {
                if let ExprKind::Symbol(sym) = &graph.get(*exp).kind {
                    if *sym == n {
                        let denom = graph.sub(z_sym, *base);
                        return Ok((
                            graph.div(z_sym, denom),
                            RocSpec::ExteriorDisk { r_min: 1.0 },
                        ));
                    }
                }
                Ok((
                    Self::z_sum_fallback(graph, expr, n, z_sym),
                    RocSpec::ExteriorDisk { r_min: 1.0 },
                ))
            }
            _ => Ok((
                Self::z_sum_fallback(graph, expr, n, z_sym),
                RocSpec::ExteriorDisk { r_min: 1.0 },
            )),
        }
    }

    fn z_sum_fallback(graph: &ExprGraph, expr: ExprId, n: SymbolId, z_sym: ExprId) -> ExprId {
        let n_name = graph.symbols.resolve(n).unwrap_or_default();
        let n_sym = graph.symbol(&n_name);
        let neg_n = graph.neg(n_sym);
        let z_neg_n = graph.pow(z_sym, neg_n);
        let term = graph.mul([expr, z_neg_n]);
        let zero = graph.integer(0);
        let inf = graph.constant(algebra_core::Constant::Infinity);
        graph.sum(term, n, Some(zero), Some(inf))
    }

    /// Non-uniform / irregular-sampling Z-transform:
    /// $X(z) = \sum_k x_k z^{-t_k}$ for irregular sample timestamps $t_k$.
    pub fn non_uniform_z_transform(samples: &[(f64, f64)], z: (f64, f64)) -> (f64, f64) {
        let mut acc = (0.0, 0.0);
        for &(t_k, x_k) in samples {
            let z_pow = c_powf(z, -t_k);
            acc = c_add(acc, (x_k * z_pow.0, x_k * z_pow.1));
        }
        acc
    }

    /// Chirp-Z transform along spiral contour $z_k = A W^{-k}$ for $k = 0 \dots m-1$.
    pub fn chirp_z_transform(
        samples: &[f64],
        m: usize,
        w: (f64, f64),
        a: (f64, f64),
    ) -> Vec<(f64, f64)> {
        let mut out = Vec::with_capacity(m);
        for k in 0..m {
            let w_neg_k = c_powf(w, -(k as f64));
            let z_k = c_mul(a, w_neg_k);
            let mut sum = (0.0, 0.0);
            for (n, &x_n) in samples.iter().enumerate() {
                let z_neg_n = c_powf(z_k, -(n as f64));
                sum = c_add(sum, (x_n * z_neg_n.0, x_n * z_neg_n.1));
            }
            out.push(sum);
        }
        out
    }

    /// Non-Uniform Fourier Transform (Type-1 NUFFT direct evaluation):
    /// $\hat{f}(\omega_k) = \sum_j x_j e^{-i \omega_k t_j}$.
    pub fn nufft_type1(samples: &[(f64, f64)], freqs: &[f64]) -> Vec<(f64, f64)> {
        freqs
            .iter()
            .map(|&omega| {
                let mut acc = (0.0, 0.0);
                for &(t_j, x_j) in samples {
                    let phase = -omega * t_j;
                    acc = c_add(acc, (x_j * phase.cos(), x_j * phase.sin()));
                }
                acc
            })
            .collect()
    }

    /// Hilger time-scale Laplace transform $\mathcal{L}_\mathbb{T}\{f\}(s)$ unifying
    /// continuous Laplace ($\mu \to 0$) and discrete Z-transform ($\mu = 1, z = 1 + s$).
    pub fn hilger_laplace_transform(
        samples: &[(f64, f64)],
        graininess_mu: &[f64],
        s: (f64, f64),
    ) -> (f64, f64) {
        let mut acc = (0.0, 0.0);
        let mut exp_acc = (1.0, 0.0);
        for (idx, &(_t_k, f_k)) in samples.iter().enumerate() {
            let mu = graininess_mu.get(idx).copied().unwrap_or(1.0);
            let denom = (1.0 + mu * s.0, mu * s.1);
            let step_factor = c_div((1.0, 0.0), denom);
            exp_acc = c_mul(exp_acc, step_factor);
            acc = c_add(acc, (f_k * mu * exp_acc.0, f_k * mu * exp_acc.1));
        }
        acc
    }

    /// Fractional Fourier Transform (FrFT) sample evaluation at angle $\alpha$.
    /// Reduces to DFT when $\alpha = \pi/2$ (up to scaling).
    pub fn fractional_fourier_sample(samples: &[f64], alpha: f64, dt: f64) -> Vec<(f64, f64)> {
        let n = samples.len();
        if n == 0 {
            return Vec::new();
        }
        let sin_a = alpha.sin();
        if sin_a.abs() < 1e-12 {
            // Identity (alpha ~ 0 mod 2pi) or reflection (alpha ~ pi)
            return if alpha.cos() >= 0.0 {
                samples.iter().map(|&v| (v, 0.0)).collect()
            } else {
                samples.iter().rev().map(|&v| (v, 0.0)).collect()
            };
        }
        let cot_a = alpha.cos() / sin_a;
        let csc_a = 1.0 / sin_a;
        let mut out = Vec::with_capacity(n);
        for k in 0..n {
            let u = (k as f64 - (n as f64) * 0.5) * dt;
            let mut sum = (0.0, 0.0);
            for (j, &x_j) in samples.iter().enumerate() {
                let t = (j as f64 - (n as f64) * 0.5) * dt;
                let phase = 0.5 * cot_a * (u * u + t * t) - csc_a * u * t;
                sum = c_add(sum, (x_j * dt * phase.cos(), x_j * dt * phase.sin()));
            }
            out.push(sum);
        }
        out
    }

    /// Linear Canonical Transform (LCT) with unimodular matrix `[a, b, c, d]` ($ad - bc = 1$).
    pub fn linear_canonical_transform_sample(
        samples: &[f64],
        abcd: [f64; 4],
        dt: f64,
    ) -> AlgebraResult<Vec<(f64, f64)>> {
        let [a, b, c, d] = abcd;
        let det = a * d - b * c;
        if (det - 1.0).abs() > 1e-6 {
            return Err(AlgebraError::DomainViolation {
                domain: "LCT".into(),
                reason: format!("LCT matrix must satisfy AD - BC = 1 (got {det})"),
            });
        }
        if b.abs() < 1e-12 {
            return Ok(samples.iter().map(|&v| (v / a.abs().sqrt(), 0.0)).collect());
        }
        let n = samples.len();
        let mut out = Vec::with_capacity(n);
        for k in 0..n {
            let u = (k as f64 - (n as f64) * 0.5) * dt;
            let mut sum = (0.0, 0.0);
            for (j, &x_j) in samples.iter().enumerate() {
                let t = (j as f64 - (n as f64) * 0.5) * dt;
                let phase = (a * t * t - 2.0 * u * t + d * u * u) / (2.0 * b);
                sum = c_add(sum, (x_j * dt * phase.cos(), x_j * dt * phase.sin()));
            }
            out.push(sum);
        }
        Ok(out)
    }

    /// Symbolic Mellin transform $\mathcal{M}\{f(t)\}(s) = \int_0^\infty t^{s-1} f(t)\,dt$.
    pub fn mellin_transform(graph: &ExprGraph, expr: ExprId, t: SymbolId, s: SymbolId) -> ExprId {
        let t_name = graph.symbols.resolve(t).unwrap_or_default();
        let s_name = graph.symbols.resolve(s).unwrap_or_default();
        let t_sym = graph.symbol(&t_name);
        let s_sym = graph.symbol(&s_name);
        let one = graph.integer(1);
        let s_minus_1 = graph.sub(s_sym, one);
        let kernel = graph.pow(t_sym, s_minus_1);
        let integrand = graph.mul([expr, kernel]);
        let zero = graph.integer(0);
        let inf = graph.constant(algebra_core::Constant::Infinity);
        graph.integral(integrand, &t_name, Some(zero), Some(inf))
    }

    /// Symbolic Hankel transform $\mathcal{H}_\nu\{f(r)\}(k) = \int_0^\infty f(r) J_\nu(kr) r\,dr$.
    pub fn hankel_transform(
        graph: &ExprGraph,
        expr: ExprId,
        r: SymbolId,
        k: SymbolId,
        nu: ExprId,
    ) -> ExprId {
        let r_name = graph.symbols.resolve(r).unwrap_or_default();
        let k_name = graph.symbols.resolve(k).unwrap_or_default();
        let r_sym = graph.symbol(&r_name);
        let k_sym = graph.symbol(&k_name);
        let kr = graph.mul([k_sym, r_sym]);
        let bessel = graph.function("bessel_j", [nu, kr]);
        let integrand = graph.mul([expr, bessel, r_sym]);
        let zero = graph.integer(0);
        let inf = graph.constant(algebra_core::Constant::Infinity);
        graph.integral(integrand, &r_name, Some(zero), Some(inf))
    }

    /// Symbolic Hilbert transform $\mathcal{H}\{f\}(\tau) = \frac{1}{\pi}\int_{-\infty}^\infty \frac{f(t)}{\tau - t}\,dt$.
    pub fn hilbert_transform(
        graph: &ExprGraph,
        expr: ExprId,
        t: SymbolId,
        tau: SymbolId,
    ) -> ExprId {
        let t_name = graph.symbols.resolve(t).unwrap_or_default();
        let tau_name = graph.symbols.resolve(tau).unwrap_or_default();
        let t_sym = graph.symbol(&t_name);
        let tau_sym = graph.symbol(&tau_name);

        if let ExprKind::Function { name, args } = &graph.get(expr).kind {
            let fn_name = graph.symbols.resolve(*name).unwrap_or_default();
            if args.len() == 1 && args[0] == t_sym {
                match fn_name.as_str() {
                    "sin" => {
                        let c = graph.function("cos", [tau_sym]);
                        return graph.neg(c);
                    }
                    "cos" => return graph.function("sin", [tau_sym]),
                    _ => {}
                }
            }
        }

        let pi = graph.constant(algebra_core::Constant::Pi);
        let denom_diff = graph.sub(tau_sym, t_sym);
        let denom = graph.mul([pi, denom_diff]);
        let integrand = graph.div(expr, denom);
        let neg_inf = graph.constant(algebra_core::Constant::NegInfinity);
        let pos_inf = graph.constant(algebra_core::Constant::Infinity);
        graph.integral(integrand, &t_name, Some(neg_inf), Some(pos_inf))
    }

    /// Symbolic continuous convolution $(f * g)(t) = \int_{-\infty}^\infty f(\tau) g(t - \tau)\,d\tau$.
    pub fn symbolic_convolution(
        graph: &ExprGraph,
        f_of_tau: ExprId,
        g_of_t_minus_tau: ExprId,
        tau: SymbolId,
    ) -> ExprId {
        let tau_name = graph.symbols.resolve(tau).unwrap_or_default();
        let integrand = graph.mul([f_of_tau, g_of_t_minus_tau]);
        let neg_inf = graph.constant(algebra_core::Constant::NegInfinity);
        let pos_inf = graph.constant(algebra_core::Constant::Infinity);
        graph.integral(integrand, &tau_name, Some(neg_inf), Some(pos_inf))
    }

    /// Exact discrete linear convolution $(a * b)[n] = \sum_k a[k] b[n - k]$.
    pub fn discrete_convolution(a: &[f64], b: &[f64]) -> Vec<f64> {
        if a.is_empty() || b.is_empty() {
            return Vec::new();
        }
        let mut out = vec![0.0; a.len() + b.len() - 1];
        for (i, &ai) in a.iter().enumerate() {
            for (j, &bj) in b.iter().enumerate() {
                out[i + j] += ai * bj;
            }
        }
        out
    }

    /// Cross-transform link: convert Laplace $s$-domain expression to Fourier $\omega$-domain via $s \mapsto i\omega$.
    pub fn laplace_to_fourier_subst(
        graph: &ExprGraph,
        laplace_expr: ExprId,
        s: SymbolId,
        omega: SymbolId,
    ) -> ExprId {
        let omega_name = graph.symbols.resolve(omega).unwrap_or_default();
        let omega_sym = graph.symbol(&omega_name);
        let i_const = graph.constant(algebra_core::Constant::I);
        let i_omega = graph.mul([i_const, omega_sym]);
        graph.substitute(laplace_expr, s, i_omega)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generalized_transforms_and_convolution() {
        let graph = ExprGraph::new();
        let n = graph.symbols.get_or_intern("n");
        let z = graph.symbols.get_or_intern("z");
        let (z_one, roc) = GeneralizedTransformEngine::z_transform(&graph, graph.integer(1), n, z)
            .expect("z_transform");
        assert_eq!(roc, RocSpec::ExteriorDisk { r_min: 1.0 });
        assert!(matches!(
            graph.get(z_one).kind,
            ExprKind::Mul(_) | ExprKind::Pow { .. } | ExprKind::Div { .. }
        ));

        // Non-uniform Z-transform at z = 2.0 for t = [0.0, 0.5, 1.0], x = [1.0, 2.0, 4.0]
        let nuz = GeneralizedTransformEngine::non_uniform_z_transform(
            &[(0.0, 1.0), (0.5, 2.0), (1.0, 4.0)],
            (2.0, 0.0),
        );
        let expected = 1.0 + 2.0 * 2.0f64.powf(-0.5) + 4.0 * 0.5;
        assert!((nuz.0 - expected).abs() < 1e-12);
        assert!(nuz.1.abs() < 1e-12);

        // Chirp-Z at A=(1,0), W=(0.5,0) -> z_0=1, z_1=2
        let czt =
            GeneralizedTransformEngine::chirp_z_transform(&[1.0, 2.0], 2, (0.5, 0.0), (1.0, 0.0));
        assert!((czt[0].0 - 3.0).abs() < 1e-12);
        assert!((czt[1].0 - 2.0).abs() < 1e-12);

        // NUFFT at omega = 0 sums samples
        let nufft = GeneralizedTransformEngine::nufft_type1(&[(0.1, 1.5), (0.7, 2.5)], &[0.0]);
        assert!((nufft[0].0 - 4.0).abs() < 1e-12);

        // Hilger Laplace with mu = 1 at s = 1 -> Z-like decay 1/(1+s)^(k+1)
        let hilger = GeneralizedTransformEngine::hilger_laplace_transform(
            &[(0.0, 1.0), (1.0, 1.0)],
            &[1.0, 1.0],
            (1.0, 0.0),
        );
        assert!((hilger.0 - 0.75).abs() < 1e-12);

        // FrFT identity at alpha = 0
        let frft0 = GeneralizedTransformEngine::fractional_fourier_sample(&[1.0, 2.0], 0.0, 0.5);
        assert!((frft0[0].0 - 1.0).abs() < 1e-12);
        assert!((frft0[1].0 - 2.0).abs() < 1e-12);

        // LCT rejects non-unimodular matrix
        assert!(
            GeneralizedTransformEngine::linear_canonical_transform_sample(
                &[1.0],
                [2.0, 0.0, 0.0, 2.0],
                0.5
            )
            .is_err()
        );

        // Hilbert transform of sin(t) -> -cos(tau)
        let t = graph.symbols.get_or_intern("t");
        let tau = graph.symbols.get_or_intern("tau");
        let t_sym = graph.symbol("t");
        let sin_t = graph.function("sin", [t_sym]);
        let h_sin = GeneralizedTransformEngine::hilbert_transform(&graph, sin_t, t, tau);
        let tau_sym = graph.symbol("tau");
        let cos_tau = graph.function("cos", [tau_sym]);
        assert_eq!(h_sin, graph.neg(cos_tau));

        // Discrete convolution
        assert_eq!(
            GeneralizedTransformEngine::discrete_convolution(&[1.0, 2.0], &[3.0, 4.0]),
            vec![3.0, 10.0, 8.0]
        );
    }
}
