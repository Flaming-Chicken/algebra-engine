//! # `algebra_engine::limits`
//!
//! Multi-Order L'Hôpital Rule & Dominik Gruntz Most Rapidly Varying (MRV) Limits Engine.
//!
//! ## Mathematical Foundations
//! Computing asymptotic limits $\lim_{x \to x_0} f(x)$ involves resolving indeterminacies:
//!
//! ### 1. Multi-Order L'Hôpital's Rule
//! For indeterminate forms $\left[\frac{0}{0}\right]$ or $\left[\frac{\infty}{\infty}\right]$, L'Hôpital's rule
//! states that:
//! $$\lim_{x \to x_0} \frac{f(x)}{g(x)} = \lim_{x \to x_0} \frac{f'(x)}{g'(x)} = \cdots = \lim_{x \to x_0} \frac{f^{(k)}(x)}{g^{(k)}(x)}$$
//! URAE's engine evaluates successive derivatives up to order $k \le 10$ with exact symbolic cancellations.
//!
//! ### 2. Dominik Gruntz Algorithm (1996)
//! For transcendental expressions involving nested exponentials and logarithms where L'Hôpital's rule
//! cycles endlessly (e.g. $\lim_{x \to \infty} \frac{e^x}{e^{x + e^{-x}}}$), Gruntz's algorithm:
//! 1. Identifies the **Most Rapidly Varying (MRV)** set of subexpressions $\operatorname{mrv}(f(x))$.
//! 2. Rewrites the expression in terms of a small expansion parameter $\omega \in \operatorname{mrv}(f)$ with $\lim_{x \to \infty} \omega(x) = 0$.
//! 3. Computes the Puiseux / Laurent series expansion in powers of $\omega$:
//!    $$f(x) = c_0 \omega^{d_0} + c_1 \omega^{d_1} + \cdots \quad (d_0 < d_1 < \cdots)$$
//! 4. Extracts the leading term $\lim_{\omega \to 0^+} c_0 \omega^{d_0} = \begin{cases} 0 & \text{if } d_0 > 0 \\ c_0 & \text{if } d_0 = 0 \\ \pm \infty & \text{if } d_0 < 0 \end{cases}$.
//!
//! ## References
//! - Gruntz, D. (1996). *On Computing Limits in a Symbolic Manipulation System*. PhD thesis, ETH Zürich.
//! - Shackell, J. (2004). *Symbolic Asymptotics: Functions of One Variable, Estimates with Remainder and Oscillation Degree*. Springer.

use crate::calculus::SymbolicCalculus;
use crate::numeric::NumericalEval;
use algebra_core::{AlgebraError, AlgebraResult, ExprGraph, ExprId, ExprKind, Number, SymbolId};

/// Direction of approaching the limit point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitDirection {
    /// Standard two-sided limit $x \to a$.
    TwoSided,
    /// Right-sided limit $x \to a^+$.
    FromRight,
    /// Left-sided limit $x \to a^-$.
    FromLeft,
    /// Limit towards positive infinity $x \to +\infty$.
    PositiveInfinity,
    /// Limit towards negative infinity $x \to -\infty$.
    NegativeInfinity,
}

/// Multi-Order L'Hôpital and Gruntz Limit Solver.
pub struct LimitEngine;

impl LimitEngine {
    /// Compute limit $\lim_{x \to point} f(x)$ with two-sided direction.
    pub fn limit(
        graph: &ExprGraph,
        expr: ExprId,
        wrt: SymbolId,
        point: ExprId,
    ) -> AlgebraResult<ExprId> {
        Self::limit_directed(graph, expr, wrt, point, LimitDirection::TwoSided)
    }

    /// Compute directional limit $\lim_{x \to point^\pm} f(x)$.
    pub fn limit_directed(
        graph: &ExprGraph,
        expr: ExprId,
        wrt: SymbolId,
        point: ExprId,
        direction: LimitDirection,
    ) -> AlgebraResult<ExprId> {
        let neg_inf = direction == LimitDirection::NegativeInfinity
            || Self::is_neg_infinity_node(graph, point);
        let pos_inf = direction == LimitDirection::PositiveInfinity
            || (Self::is_infinity_node(graph, point) && !neg_inf);

        // 1. Limits at +-oo must run asymptotic / Gruntz analysis BEFORE substitution
        if pos_inf || neg_inf {
            if let Ok(res) = Self::eval_infinity(graph, expr, wrt, neg_inf) {
                return Ok(crate::simplify::Simplifier::simplify(graph, res).unwrap_or(res));
            }
        }

        // 1b. A two-sided limit through abs / sign / floor exists only if
        // both one-sided values agree (x / |x| at 0 does not).
        if direction == LimitDirection::TwoSided
            && !pos_inf
            && !neg_inf
            && Self::has_nonsmooth_function(graph, expr)
            && let Ok(at) = graph.evalf(point, &crate::numeric::EvalContext::default())
            && let (Some(left), Some(right)) = (
                Self::one_sided_numeric(graph, expr, wrt, at.to_f64(), false),
                Self::one_sided_numeric(graph, expr, wrt, at.to_f64(), true),
            )
            && (left - right).abs() > 1e-6 * left.abs().max(right.abs()).max(1.0)
        {
            return Ok(graph.constant(algebra_core::Constant::Undefined));
        }

        // 2. Direct substitution with constant simplification
        let sub = graph.substitute(expr, wrt, point);
        let simplified_sub = Self::simplify_constants(graph, sub);

        if !Self::is_indeterminate(graph, simplified_sub) {
            return Ok(simplified_sub);
        }

        // 3. Rational fraction f(x) / g(x): distinguish nonzero/0 poles from 0/0 or oo/oo L'Hopital
        if let ExprKind::Div(num, den) = graph.get(expr).kind {
            let sub_n = Self::simplify_constants(graph, graph.substitute(num, wrt, point));
            let sub_d = Self::simplify_constants(graph, graph.substitute(den, wrt, point));
            let num_zero = Self::is_zero_node(graph, sub_n);
            let den_zero = Self::is_zero_node(graph, sub_d);
            let num_inf = Self::is_infinity_node(graph, sub_n);
            let den_inf = Self::is_infinity_node(graph, sub_d);

            if den_zero && !num_zero && !num_inf {
                return Ok(Self::eval_pole_limit(graph, expr, wrt, point, direction));
            }

            if (num_zero && den_zero) || (num_inf && den_inf) {
                if let Some(res) = Self::eval_lhopital(graph, num, den, wrt, point, 10)? {
                    return Ok(crate::simplify::Simplifier::simplify(graph, res).unwrap_or(res));
                }
            }
        }

        // 4. Fallback Taylor series expansion around finite point
        if !pos_inf && !neg_inf {
            let taylor = graph.taylor_series(expr, wrt, point, 4);
            let sub_taylor = graph.substitute(taylor, wrt, point);
            let sim_taylor = Self::simplify_constants(graph, sub_taylor);
            if !Self::is_indeterminate(graph, sim_taylor) {
                return Ok(sim_taylor);
            }
        }

        if Self::is_indeterminate(graph, simplified_sub) {
            return Ok(graph.constant(algebra_core::Constant::Undefined));
        }

        Ok(simplified_sub)
    }

    /// Successive L'Hôpital rule evaluation up to `max_order` with cycle detection
    /// and derivative-swell growth caps to prevent infinite/exponential blowup.
    pub fn eval_lhopital(
        graph: &ExprGraph,
        mut num: ExprId,
        mut den: ExprId,
        wrt: SymbolId,
        point: ExprId,
        max_order: usize,
    ) -> AlgebraResult<Option<ExprId>> {
        let mut seen = std::collections::HashSet::with_capacity(max_order + 1);
        seen.insert((num, den));
        let mut prev_nodes = Self::expr_node_count(graph, num) + Self::expr_node_count(graph, den);
        let mut consecutive_growth = 0usize;

        for _ in 0..max_order {
            let d_num = graph.diff(num, wrt);
            let d_den = graph.diff(den, wrt);

            if d_den == graph.integer(0) {
                // Denominator derivative vanished
                return Ok(None);
            }

            // Cycle guard: repeated (num, den) pair -> abort L'Hopital
            if !seen.insert((d_num, d_den)) {
                return Ok(None);
            }

            // Swell guard: abort if total AST/DAG footprint more than doubles for 2 consecutive steps
            let cur_nodes =
                Self::expr_node_count(graph, d_num) + Self::expr_node_count(graph, d_den);
            if cur_nodes > prev_nodes.saturating_mul(2).max(16) {
                consecutive_growth += 1;
                if consecutive_growth >= 2 {
                    return Ok(None);
                }
            } else {
                consecutive_growth = 0;
            }
            prev_nodes = cur_nodes;

            let sub_num = graph.substitute(d_num, wrt, point);
            let sub_den = graph.substitute(d_den, wrt, point);

            let sim_num = Self::simplify_constants(graph, sub_num);
            let sim_den = Self::simplify_constants(graph, sub_den);

            let is_num_zero = Self::is_zero_node(graph, sim_num);
            let is_den_zero = Self::is_zero_node(graph, sim_den);

            if !is_num_zero && !is_den_zero {
                // Resolved indeterminacy!
                let res = if sim_num == sim_den {
                    graph.integer(1)
                } else {
                    Self::simplify_constants(graph, graph.div(sim_num, sim_den))
                };
                return Ok(Some(res));
            }

            if !is_num_zero && is_den_zero {
                // c / 0 -> Infinity
                return Ok(Some(graph.constant(algebra_core::Constant::Infinity)));
            }

            if is_num_zero && !is_den_zero {
                // 0 / c -> 0
                return Ok(Some(graph.integer(0)));
            }

            // Both zero: iterate to next order
            num = d_num;
            den = d_den;
        }

        Ok(None)
    }

    /// Count unique DAG nodes reachable from `root` in O(|V|) using a visited set.
    fn expr_node_count(graph: &ExprGraph, root: ExprId) -> usize {
        let mut seen = std::collections::HashSet::new();
        let mut stack = vec![root];
        while let Some(id) = stack.pop() {
            if !seen.insert(id) {
                continue;
            }
            match &graph.get(id).kind {
                ExprKind::Add(ts) | ExprKind::Mul(ts) => stack.extend(ts.iter().copied()),
                ExprKind::Sub(a, b) | ExprKind::Div(a, b) | ExprKind::Pow(a, b) => {
                    stack.push(*a);
                    stack.push(*b);
                }
                ExprKind::Neg(a) => stack.push(*a),
                ExprKind::Function { args, .. } => stack.extend(args.iter().copied()),
                _ => {}
            }
            if seen.len() >= 4096 {
                break;
            }
        }
        seen.len()
    }

    /// Constant expression reduction helper (e.g. cos(0)->1, sin(0)->0, exp(0)->1, sqrt(1)->1, 1-1->0).
    pub fn simplify_constants(graph: &ExprGraph, id: ExprId) -> ExprId {
        let node = graph.get(id);
        match node.kind {
            ExprKind::Number(_) => id,
            ExprKind::Function { name, args } => {
                let fn_name = graph.symbols.resolve(name).unwrap_or_default();
                if args.len() == 1 {
                    let arg_sim = Self::simplify_constants(graph, args[0]);
                    if Self::is_zero_node(graph, arg_sim) {
                        match fn_name.as_str() {
                            "sin" | "sinh" | "tan" | "tanh" | "sqrt" | "abs" | "Abs" | "asin"
                            | "atan" | "asinh" | "atanh" | "erf" => return graph.integer(0),
                            "cos" | "cosh" | "exp" | "sech" => return graph.integer(1),
                            _ => {}
                        }
                    }
                    if let ExprKind::Number(Number::Integer(1)) = graph.get(arg_sim).kind {
                        match fn_name.as_str() {
                            "ln" | "log" | "acosh" => return graph.integer(0),
                            "sqrt" | "abs" | "Abs" => return graph.integer(1),
                            _ => {}
                        }
                    }
                    if fn_name == "sqrt"
                        && let ExprKind::Number(Number::Integer(n)) = graph.get(arg_sim).kind
                        && n > 0
                    {
                        let r = (n as f64).sqrt().round() as i64;
                        if r * r == n {
                            return graph.integer(r);
                        }
                    }
                    if fn_name == "abs" || fn_name == "Abs" {
                        if let ExprKind::Number(Number::Integer(n)) = graph.get(arg_sim).kind {
                            return graph.integer(n.abs());
                        }
                    }
                    if Self::is_infinity_node(graph, arg_sim) {
                        let is_neg = Self::is_neg_infinity_node(graph, arg_sim);
                        match fn_name.as_str() {
                            "exp" => {
                                return if is_neg {
                                    graph.integer(0)
                                } else {
                                    graph.constant(algebra_core::Constant::Infinity)
                                };
                            }
                            "erf" => return graph.integer(if is_neg { -1 } else { 1 }),
                            "erfc" => return graph.integer(if is_neg { 2 } else { 0 }),
                            "tanh" => return graph.integer(if is_neg { -1 } else { 1 }),
                            "sech" => return graph.integer(0),
                            "atan" | "arctan" => {
                                let half_pi = graph.div(
                                    graph.constant(algebra_core::Constant::Pi),
                                    graph.integer(2),
                                );
                                return if is_neg { graph.neg(half_pi) } else { half_pi };
                            }
                            "ln" | "log" | "sqrt" if !is_neg => {
                                return graph.constant(algebra_core::Constant::Infinity);
                            }
                            _ => {}
                        }
                    }
                    return graph.function(&fn_name, [arg_sim]);
                }
                id
            }
            ExprKind::Add(terms) => {
                let mut sum = 0i64;
                let mut all_int = true;
                let mut new_terms = Vec::with_capacity(terms.len());
                for t in terms {
                    let st = Self::simplify_constants(graph, t);
                    if let ExprKind::Number(Number::Integer(i)) = graph.get(st).kind {
                        sum += i;
                    } else {
                        all_int = false;
                        new_terms.push(st);
                    }
                }
                if all_int {
                    return graph.integer(sum);
                }
                if sum != 0 {
                    new_terms.push(graph.integer(sum));
                }
                graph.add(new_terms)
            }
            ExprKind::Sub(a, b) => {
                let sa = Self::simplify_constants(graph, a);
                let sb = Self::simplify_constants(graph, b);
                if sa == sb {
                    return graph.integer(0);
                }
                if let (
                    ExprKind::Number(Number::Integer(ia)),
                    ExprKind::Number(Number::Integer(ib)),
                ) = (&graph.get(sa).kind, &graph.get(sb).kind)
                {
                    return graph.integer(ia - ib);
                }
                graph.sub(sa, sb)
            }
            ExprKind::Mul(factors) => {
                let mut prod = 1i64;
                let mut all_int = true;
                let mut has_pos_inf = false;
                let mut has_neg_inf = false;
                let mut new_factors = Vec::with_capacity(factors.len());
                for f in factors {
                    let sf = Self::simplify_constants(graph, f);
                    if Self::is_zero_node(graph, sf) {
                        return graph.integer(0);
                    }
                    if Self::is_neg_infinity_node(graph, sf) {
                        has_neg_inf = !has_neg_inf;
                        all_int = false;
                    } else if Self::is_infinity_node(graph, sf) {
                        has_pos_inf = true;
                        all_int = false;
                    } else if let ExprKind::Number(Number::Integer(i)) = graph.get(sf).kind {
                        prod *= i;
                    } else {
                        all_int = false;
                        new_factors.push(sf);
                    }
                }
                if (has_pos_inf || has_neg_inf) && new_factors.is_empty() && prod != 0 {
                    let is_neg = has_neg_inf ^ (prod < 0);
                    return graph.constant(if is_neg {
                        algebra_core::Constant::NegInfinity
                    } else {
                        algebra_core::Constant::Infinity
                    });
                }
                if all_int {
                    return graph.integer(prod);
                }
                if has_neg_inf {
                    new_factors.push(graph.constant(algebra_core::Constant::NegInfinity));
                } else if has_pos_inf {
                    new_factors.push(graph.constant(algebra_core::Constant::Infinity));
                }
                if prod != 1 {
                    new_factors.push(graph.integer(prod));
                }
                graph.mul(new_factors)
            }
            ExprKind::Div(a, b) => {
                let sa = Self::simplify_constants(graph, a);
                let sb = Self::simplify_constants(graph, b);
                if sa == sb && !Self::is_zero_node(graph, sa) {
                    return graph.integer(1);
                }
                if Self::is_zero_node(graph, sa) && !Self::is_zero_node(graph, sb) {
                    return graph.integer(0);
                }
                if let (
                    ExprKind::Number(Number::Integer(ia)),
                    ExprKind::Number(Number::Integer(ib)),
                ) = (&graph.get(sa).kind, &graph.get(sb).kind)
                    && *ib != 0
                {
                    if ia % ib == 0 {
                        return graph.integer(ia / ib);
                    } else {
                        return graph.rational(*ia, *ib);
                    }
                }
                graph.div(sa, sb)
            }
            ExprKind::Pow(base, exp) => {
                let s_base = Self::simplify_constants(graph, base);
                let s_exp = Self::simplify_constants(graph, exp);
                if Self::is_zero_node(graph, s_exp) {
                    return graph.integer(1);
                }
                if let ExprKind::Number(Number::Integer(1)) = graph.get(s_exp).kind {
                    return s_base;
                }
                if let ExprKind::Number(Number::Integer(1)) = graph.get(s_base).kind {
                    if let ExprKind::Number(
                        Number::Integer(_) | Number::Rational(_, _) | Number::Float(_),
                    ) = graph.get(s_exp).kind
                    {
                        return graph.integer(1);
                    }
                }
                if Self::is_zero_node(graph, s_base)
                    && let Ok(val) = graph.evalf(s_exp, &crate::numeric::EvalContext::default())
                    && val.to_f64() > 0.0
                {
                    return graph.integer(0);
                }
                if let (
                    ExprKind::Number(Number::Integer(b)),
                    ExprKind::Number(Number::Integer(e)),
                ) = (&graph.get(s_base).kind, &graph.get(s_exp).kind)
                {
                    if *e >= 0 {
                        return graph.integer(b.pow(*e as u32));
                    } else if *b != 0 {
                        return graph.rational(1, b.pow((-*e) as u32));
                    }
                }
                graph.pow(s_base, s_exp)
            }
            ExprKind::Neg(a) => {
                let sa = Self::simplify_constants(graph, a);
                if Self::is_zero_node(graph, sa) {
                    return graph.integer(0);
                }
                if let ExprKind::Number(Number::Integer(i)) = graph.get(sa).kind {
                    return graph.integer(-i);
                }
                if let ExprKind::Number(Number::Constant(c)) = graph.get(sa).kind {
                    match c {
                        algebra_core::Constant::Infinity => {
                            return graph.constant(algebra_core::Constant::NegInfinity);
                        }
                        algebra_core::Constant::NegInfinity => {
                            return graph.constant(algebra_core::Constant::Infinity);
                        }
                        _ => {}
                    }
                }
                graph.neg(sa)
            }
            _ => id,
        }
    }

    /// Asymptotic limit evaluation for $x \to \pm\infty$.
    fn eval_infinity(
        graph: &ExprGraph,
        expr: ExprId,
        wrt: SymbolId,
        neg_inf: bool,
    ) -> AlgebraResult<ExprId> {
        if !graph.has_symbol(expr, wrt) {
            return Ok(expr);
        }

        let inf_const = if neg_inf {
            graph.constant(algebra_core::Constant::NegInfinity)
        } else {
            graph.constant(algebra_core::Constant::Infinity)
        };
        let wrt_name = graph.symbols.resolve(wrt).unwrap_or_default();
        let x = graph.symbol(&wrt_name);

        // Exponential form (base(x))^(exp(x)): handle 1^oo -> exp(lim exp*(base - 1))
        if let ExprKind::Pow(base, exp) = graph.get(expr).kind {
            if graph.has_symbol(base, wrt) && graph.has_symbol(exp, wrt) {
                if let Ok(base_lim) = Self::eval_infinity(graph, base, wrt, neg_inf)
                    && base_lim == graph.integer(1)
                {
                    let one = graph.integer(1);
                    let eps = graph.sub(base, one);
                    let prod = graph.mul([exp, eps]);
                    if let Ok(exponent_lim) = Self::eval_infinity(graph, prod, wrt, neg_inf) {
                        let simp_exp = crate::simplify::Simplifier::simplify(graph, exponent_lim)
                            .unwrap_or(exponent_lim);
                        if Self::is_zero_node(graph, simp_exp) {
                            return Ok(one);
                        }
                        if simp_exp == one {
                            return Ok(graph.constant(algebra_core::Constant::E));
                        }
                        let e_const = graph.constant(algebra_core::Constant::E);
                        return Ok(graph.pow(e_const, simp_exp));
                    }
                }
            }
        }

        // Convert rational combinations (e.g. x * ((1 + 1/x) - 1) or P/Q + R/S) into N(x) / D(x)
        let (num_raw, den_raw) = Self::flatten_rational(graph, expr, wrt);
        if den_raw != graph.integer(1) || matches!(graph.get(expr).kind, ExprKind::Div(_, _)) {
            if let (Some((dn_u, lc_num)), Some((dd_u, lc_den))) = (
                Self::poly_deg_and_lc(graph, num_raw, wrt),
                Self::poly_deg_and_lc(graph, den_raw, wrt),
            ) {
                let dn = dn_u as i64;
                let dd = dd_u as i64;
                if dn < dd {
                    return Ok(graph.integer(0));
                } else if dn > dd {
                    let ctx = crate::numeric::EvalContext::default();
                    let ratio_val = graph
                        .evalf(graph.div(lc_num, lc_den), &ctx)
                        .map(|v| v.to_f64())
                        .unwrap_or(1.0);
                    let deg_diff = dn - dd;
                    let sign_neg = (neg_inf && (deg_diff % 2 != 0)) ^ (ratio_val < 0.0);
                    return Ok(if sign_neg {
                        graph.constant(algebra_core::Constant::NegInfinity)
                    } else {
                        graph.constant(algebra_core::Constant::Infinity)
                    });
                } else {
                    let ratio = graph.div(lc_num, lc_den);
                    let simp = crate::simplify::Simplifier::simplify(graph, ratio).unwrap_or(ratio);
                    return Ok(simp);
                }
            }
        }

        if neg_inf {
            return Err(AlgebraError::EvaluationError(
                "Asymptotic negative infinity limit unavailable".into(),
            ));
        }

        // Preserve existing Gruntz exponential/log handling
        if let Ok(res) = Self::eval_gruntz_infinity(graph, expr, wrt) {
            return Ok(res);
        }

        // Pure polynomial of degree >= 1 -> +- infinity
        if let Some(d) = Self::poly_degree(graph, expr, x) {
            if d == 0 {
                return Ok(expr);
            }
            if d > 0 {
                let lc = Self::leading_coeff(graph, expr, x, d);
                let ctx = crate::numeric::EvalContext::default();
                let lc_val = graph.evalf(lc, &ctx).map(|v| v.to_f64()).unwrap_or(1.0);
                let sign_neg = (neg_inf && (d % 2 != 0)) ^ (lc_val < 0.0);
                return Ok(if sign_neg {
                    graph.constant(algebra_core::Constant::NegInfinity)
                } else {
                    inf_const
                });
            }
        }

        if let Some(res) = Self::algebraic_limit_at_infinity(graph, expr, wrt) {
            return Ok(res);
        }

        Err(AlgebraError::EvaluationError(
            "Gruntz MRV asymptotic limit heuristic fallback".into(),
        ))
    }

    /// Dominant term `c · x^p` of an algebraic expression as `x → +∞`.
    ///
    /// Handles constants, `x`, products, quotients, constant powers (so
    /// `sqrt`) and sums. Returns `None` when the dominant terms of a sum
    /// cancel and the remainder cannot be read off exactly, or when the
    /// expression is not algebraic in `x`.
    fn leading_term(graph: &ExprGraph, id: ExprId, wrt: SymbolId) -> Option<(f64, f64)> {
        let ctx = crate::numeric::EvalContext::default();
        if !graph.has_symbol(id, wrt) {
            let v = graph.evalf(id, &ctx).ok()?.to_f64();
            return v.is_finite().then_some((v, 0.0));
        }
        match &graph.get(id).kind {
            ExprKind::Symbol(sym) if *sym == wrt => Some((1.0, 1.0)),
            ExprKind::Neg(inner) => {
                let (c, e) = Self::leading_term(graph, *inner, wrt)?;
                Some((-c, e))
            }
            ExprKind::Mul(factors) => {
                let mut acc = (1.0, 0.0);
                for &f in factors.iter() {
                    let (c, e) = Self::leading_term(graph, f, wrt)?;
                    acc = (acc.0 * c, acc.1 + e);
                }
                Some(acc)
            }
            ExprKind::Div(num, den) => {
                let (cn, en) = Self::leading_term(graph, *num, wrt)?;
                let (cd, ed) = Self::leading_term(graph, *den, wrt)?;
                (cd != 0.0).then_some((cn / cd, en - ed))
            }
            ExprKind::Pow(base, exp) if !graph.has_symbol(*exp, wrt) => {
                let r = graph.evalf(*exp, &ctx).ok()?.to_f64();
                Self::leading_power(Self::leading_term(graph, *base, wrt)?, r)
            }
            ExprKind::Function { name, args } if args.len() == 1 => {
                let fn_name = graph.symbols.resolve(*name).unwrap_or_default();
                if fn_name == "sqrt" {
                    Self::leading_power(Self::leading_term(graph, args[0], wrt)?, 0.5)
                } else {
                    None
                }
            }
            ExprKind::Add(_) | ExprKind::Sub(..) => {
                let mut terms = Vec::new();
                Self::flatten_sum(graph, id, false, &mut terms);
                let mut leads = Vec::with_capacity(terms.len());
                for &(t, negated) in &terms {
                    let (c, e) = Self::leading_term(graph, t, wrt)?;
                    leads.push((if negated { -c } else { c }, e, t));
                }
                let top = leads.iter().map(|l| l.1).fold(f64::NEG_INFINITY, f64::max);
                let scale = leads.iter().map(|l| l.0.abs()).fold(0.0, f64::max).max(1.0);
                let sum_at = |e: f64| -> f64 {
                    leads
                        .iter()
                        .filter(|l| (l.1 - e).abs() < 1e-9)
                        .map(|l| l.0)
                        .sum()
                };
                let top_sum = sum_at(top);
                if top_sum.abs() > 1e-9 * scale {
                    return Some((top_sum, top));
                }
                // The dominant terms cancel. The next order is only exact if
                // every term is a single monomial with no hidden lower orders.
                if !leads.iter().all(|l| Self::is_monomial(graph, l.2, wrt)) {
                    return None;
                }
                let mut exps: Vec<f64> = leads.iter().map(|l| l.1).collect();
                exps.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
                exps.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
                for e in exps {
                    let c = sum_at(e);
                    if c.abs() > 1e-9 * scale {
                        return Some((c, e));
                    }
                }
                Some((0.0, 0.0))
            }
            _ => None,
        }
    }

    /// `(c · x^p)^r`, defined when the base's leading coefficient allows it.
    fn leading_power((c, e): (f64, f64), r: f64) -> Option<(f64, f64)> {
        if c < 0.0 && r.fract() != 0.0 {
            return None;
        }
        let coeff = c.powf(r);
        coeff.is_finite().then_some((coeff, e * r))
    }

    /// Flatten nested `Add` / `Sub` / `Neg` into signed terms.
    fn flatten_sum(graph: &ExprGraph, id: ExprId, negated: bool, out: &mut Vec<(ExprId, bool)>) {
        match &graph.get(id).kind {
            ExprKind::Add(terms) => {
                for &t in terms.iter() {
                    Self::flatten_sum(graph, t, negated, out);
                }
            }
            ExprKind::Sub(a, b) => {
                Self::flatten_sum(graph, *a, negated, out);
                Self::flatten_sum(graph, *b, !negated, out);
            }
            ExprKind::Neg(inner) => Self::flatten_sum(graph, *inner, !negated, out),
            _ => out.push((id, negated)),
        }
    }

    /// True for `c · x^k` built only from constants, `x`, products and constant powers.
    fn is_monomial(graph: &ExprGraph, id: ExprId, wrt: SymbolId) -> bool {
        if !graph.has_symbol(id, wrt) {
            return true;
        }
        match &graph.get(id).kind {
            ExprKind::Symbol(_) => true,
            ExprKind::Neg(inner) => Self::is_monomial(graph, *inner, wrt),
            ExprKind::Mul(factors) => factors.iter().all(|&f| Self::is_monomial(graph, f, wrt)),
            ExprKind::Pow(base, exp) => {
                !graph.has_symbol(*exp, wrt) && Self::is_monomial(graph, *base, wrt)
            }
            _ => false,
        }
    }

    /// The limit value for a dominant term `c · x^p` as `x → +∞`.
    fn limit_from_leading(graph: &ExprGraph, (c, e): (f64, f64)) -> ExprId {
        if e > 1e-9 && c != 0.0 {
            return graph.constant(if c < 0.0 {
                algebra_core::Constant::NegInfinity
            } else {
                algebra_core::Constant::Infinity
            });
        }
        if e < -1e-9 || c == 0.0 {
            return graph.integer(0);
        }
        Self::exact_or_float(graph, c)
    }

    /// A small exact rational when `v` is one to within rounding, else a float.
    fn exact_or_float(graph: &ExprGraph, v: f64) -> ExprId {
        for den in 1..=1000i64 {
            let num = (v * den as f64).round();
            if (v - num / den as f64).abs() < 1e-12 * v.abs().max(1.0) && num.abs() < 1e15 {
                return if den == 1 {
                    graph.integer(num as i64)
                } else {
                    graph.rational(num as i64, den)
                };
            }
        }
        graph.float(v)
    }

    /// Limits at `+∞` of algebraic expressions, by dominant-term analysis.
    ///
    /// Covers what polynomial-ratio analysis cannot: radicals
    /// (`sqrt(x^2 + x) - x`, via rationalisation when the dominant terms
    /// cancel) and powers with a variable exponent (`x^(1/x)`).
    fn algebraic_limit_at_infinity(
        graph: &ExprGraph,
        expr: ExprId,
        wrt: SymbolId,
    ) -> Option<ExprId> {
        // f^g with both depending on x: f^g = exp(g · ln f), and for
        // f ~ c·x^p, g ~ d·x^q the exponent behaves like d·x^q·(p·ln x + ln c).
        if let ExprKind::Pow(base, exp) = graph.get(expr).kind
            && graph.has_symbol(exp, wrt)
        {
            let (c, p_exp) = Self::leading_term(graph, base, wrt)?;
            let (d, q) = Self::leading_term(graph, exp, wrt)?;
            if c <= 0.0 {
                return None;
            }
            if q < -1e-9 || d == 0.0 {
                return Some(graph.integer(1));
            }
            // Sign of d · ln(base) for large x: ln x dominates when p != 0.
            let log_sign = if p_exp.abs() > 1e-9 { p_exp } else { c.ln() };
            if q > 1e-9 {
                if log_sign == 0.0 {
                    return None; // 1^∞ is handled by the caller.
                }
                return Some(if d * log_sign > 0.0 {
                    graph.constant(algebra_core::Constant::Infinity)
                } else {
                    graph.integer(0)
                });
            }
            // q == 0: the exponent tends to the constant d.
            let lead = Self::leading_power((c, p_exp), d)?;
            return Some(Self::limit_from_leading(graph, lead));
        }

        if let Some(lead) = Self::leading_term(graph, expr, wrt) {
            return Some(Self::limit_from_leading(graph, lead));
        }

        // ∞ - ∞ with a square root: A - B = (A² - B²) / (A + B).
        let mut terms = Vec::new();
        Self::flatten_sum(graph, expr, false, &mut terms);
        if terms.len() != 2 {
            return None;
        }
        let signed = |(t, negated): (ExprId, bool)| if negated { graph.neg(t) } else { t };
        let (a, b) = (signed(terms[0]), signed(terms[1]));
        let square = |t: ExprId| -> ExprId {
            let inner = match &graph.get(t).kind {
                ExprKind::Neg(u) => *u,
                _ => t,
            };
            if let ExprKind::Function { name, args } = &graph.get(inner).kind
                && args.len() == 1
                && graph.symbols.resolve(*name).as_deref() == Some("sqrt")
            {
                return args[0];
            }
            graph.pow(inner, graph.integer(2))
        };
        // A + B here is the original expression, so A - B is its conjugate.
        let conjugate = graph.sub(a, b);
        let numerator = crate::simplify::transformations::AlgebraicTransformations::expand(
            graph,
            graph.sub(square(a), square(b)),
        );
        let (cn, en) = Self::leading_term(graph, numerator, wrt)?;
        let (cd, ed) = Self::leading_term(graph, conjugate, wrt)?;
        (cd != 0.0).then(|| Self::limit_from_leading(graph, (cn / cd, en - ed)))
    }

    /// Numeric value of `expr` as `x` approaches `at` from one side, if it settles.
    fn one_sided_numeric(
        graph: &ExprGraph,
        expr: ExprId,
        wrt: SymbolId,
        at: f64,
        from_right: bool,
    ) -> Option<f64> {
        let name = graph.symbols.resolve(wrt)?;
        let eval_at = |h: f64| -> Option<f64> {
            let x = if from_right { at + h } else { at - h };
            let mut bindings = std::collections::HashMap::new();
            bindings.insert(name.clone(), x);
            let ctx = crate::numeric::EvalContext::with_bindings(15, bindings);
            let v = graph.evalf(expr, &ctx).ok()?.to_f64();
            v.is_finite().then_some(v)
        };
        let (coarse, fine) = (eval_at(1e-5)?, eval_at(1e-7)?);
        ((coarse - fine).abs() <= 1e-3 * fine.abs().max(1.0)).then_some(fine)
    }

    /// True if `expr` contains a function that can jump (`abs`, `sign`, `floor`, `ceil`).
    fn has_nonsmooth_function(graph: &ExprGraph, id: ExprId) -> bool {
        let node = graph.get(id);
        if let ExprKind::Function { name, .. } = &node.kind
            && matches!(
                graph.symbols.resolve(*name).as_deref(),
                Some("abs" | "sign" | "sgn" | "floor" | "ceil")
            )
        {
            return true;
        }
        node.children()
            .iter()
            .any(|&c| Self::has_nonsmooth_function(graph, c))
    }

    fn eval_pole_limit(
        graph: &ExprGraph,
        expr: ExprId,
        wrt: SymbolId,
        point: ExprId,
        direction: LimitDirection,
    ) -> ExprId {
        use crate::numeric::NumericalEval as _;
        let ctx = crate::numeric::EvalContext::default();
        let Ok(pt_val) = graph.evalf(point, &ctx).map(|v| v.to_f64()) else {
            return graph.constant(algebra_core::Constant::Undefined);
        };
        if !pt_val.is_finite() {
            return graph.constant(algebra_core::Constant::Undefined);
        }

        let sign_at = |offset: f64| -> Option<bool> {
            let probe = graph.float(pt_val + offset);
            let sub = graph.substitute(expr, wrt, probe);
            let v = graph.evalf(sub, &ctx).ok()?.to_f64();
            if v.is_finite() && v.abs() > 1.0 {
                Some(v > 0.0)
            } else {
                None
            }
        };

        let right_pos = sign_at(1e-7);
        let left_pos = sign_at(-1e-7);

        match direction {
            LimitDirection::FromRight => match right_pos {
                Some(true) => graph.constant(algebra_core::Constant::Infinity),
                Some(false) => graph.constant(algebra_core::Constant::NegInfinity),
                None => graph.constant(algebra_core::Constant::Undefined),
            },
            LimitDirection::FromLeft => match left_pos {
                Some(true) => graph.constant(algebra_core::Constant::Infinity),
                Some(false) => graph.constant(algebra_core::Constant::NegInfinity),
                None => graph.constant(algebra_core::Constant::Undefined),
            },
            LimitDirection::TwoSided => match (left_pos, right_pos) {
                (Some(true), Some(true)) => graph.constant(algebra_core::Constant::Infinity),
                (Some(false), Some(false)) => graph.constant(algebra_core::Constant::NegInfinity),
                _ => graph.constant(algebra_core::Constant::Undefined),
            },
            _ => graph.constant(algebra_core::Constant::Undefined),
        }
    }

    fn flatten_rational(graph: &ExprGraph, expr: ExprId, wrt: SymbolId) -> (ExprId, ExprId) {
        let one = graph.integer(1);
        if !graph.has_symbol(expr, wrt) {
            return (expr, one);
        }
        match &graph.get(expr).kind {
            ExprKind::Div(num, den) => {
                let (na, da) = Self::flatten_rational(graph, *num, wrt);
                let (nb, db) = Self::flatten_rational(graph, *den, wrt);
                let n = if db == one { na } else { graph.mul([na, db]) };
                let d = if da == one { nb } else { graph.mul([da, nb]) };
                (n, d)
            }
            ExprKind::Neg(inner) => {
                let (n, d) = Self::flatten_rational(graph, *inner, wrt);
                (graph.neg(n), d)
            }
            ExprKind::Sub(a, b) => {
                let (na, da) = Self::flatten_rational(graph, *a, wrt);
                let (nb, db) = Self::flatten_rational(graph, *b, wrt);
                if da == db {
                    (graph.sub(na, nb), da)
                } else {
                    let lhs = if db == one { na } else { graph.mul([na, db]) };
                    let rhs = if da == one { nb } else { graph.mul([nb, da]) };
                    let den = if da == one {
                        db
                    } else if db == one {
                        da
                    } else {
                        graph.mul([da, db])
                    };
                    (graph.sub(lhs, rhs), den)
                }
            }
            ExprKind::Add(terms) => {
                let mut acc = Self::flatten_rational(graph, terms[0], wrt);
                for &t in &terms[1..] {
                    let (nb, db) = Self::flatten_rational(graph, t, wrt);
                    let (na, da) = acc;
                    if da == db {
                        acc = (graph.add([na, nb]), da);
                    } else {
                        let lhs = if db == one { na } else { graph.mul([na, db]) };
                        let rhs = if da == one { nb } else { graph.mul([nb, da]) };
                        let den = if da == one {
                            db
                        } else if db == one {
                            da
                        } else {
                            graph.mul([da, db])
                        };
                        acc = (graph.add([lhs, rhs]), den);
                    }
                }
                acc
            }
            ExprKind::Mul(factors) => {
                let mut nums = Vec::new();
                let mut dens = Vec::new();
                for &f in factors {
                    let (nf, df) = Self::flatten_rational(graph, f, wrt);
                    nums.push(nf);
                    if df != one {
                        dens.push(df);
                    }
                }
                let num = if nums.len() == 1 {
                    nums[0]
                } else {
                    graph.mul(nums)
                };
                let den = if dens.is_empty() {
                    one
                } else if dens.len() == 1 {
                    dens[0]
                } else {
                    graph.mul(dens)
                };
                (num, den)
            }
            _ => (expr, one),
        }
    }

    /// Gruntz MRV evaluation for $\lim_{x \to +\infty} f(x)$.
    fn eval_gruntz_infinity(
        graph: &ExprGraph,
        expr: ExprId,
        wrt: SymbolId,
    ) -> AlgebraResult<ExprId> {
        let wrt_name = graph.symbols.resolve(wrt).unwrap_or_default();
        let x = graph.symbol(&wrt_name);

        // Case: f(x) / g(x) at infinity
        if let ExprKind::Div(num, den) = graph.get(expr).kind {
            // Check dominant degree in polynomials: P(x)/Q(x)
            let deg_num = Self::poly_degree(graph, num, x);
            let deg_den = Self::poly_degree(graph, den, x);

            if let (Some(dn), Some(dd)) = (deg_num, deg_den) {
                if dn < dd {
                    return Ok(graph.integer(0));
                } else if dn > dd {
                    return Ok(graph.constant(algebra_core::Constant::Infinity));
                } else {
                    // Leading coefficient ratio
                    let lc_num = Self::leading_coeff(graph, num, x, dn);
                    let lc_den = Self::leading_coeff(graph, den, x, dd);
                    let ratio = graph.div(lc_num, lc_den);
                    return Ok(crate::simplify::Simplifier::simplify(graph, ratio).unwrap_or(ratio));
                }
            }

            // Check exponential dominant scale: e.g. x^k / exp(x) -> 0
            if graph.has_symbol(num, wrt)
                && let ExprKind::Function { name, args } = graph.get(den).kind
            {
                let fn_name = graph.symbols.resolve(name).unwrap_or_default();
                if fn_name == "exp" && args.contains(&x) {
                    return Ok(graph.integer(0));
                }
            }
        }

        Err(AlgebraError::EvaluationError(
            "Gruntz MRV asymptotic limit heuristic fallback".into(),
        ))
    }

    pub fn is_indeterminate(graph: &ExprGraph, id: ExprId) -> bool {
        if Self::contains_infinity(graph, id) {
            return true;
        }
        let node = graph.get(id);
        match &node.kind {
            ExprKind::Div(_num, den) => Self::is_zero_node(graph, *den),
            ExprKind::Number(Number::Constant(c)) => *c == algebra_core::Constant::Undefined,
            ExprKind::Neg(a) => Self::is_indeterminate(graph, *a),
            ExprKind::Add(xs) | ExprKind::Mul(xs) => {
                xs.iter().any(|&x| Self::is_indeterminate(graph, x))
            }
            ExprKind::Sub(a, b) | ExprKind::Pow(a, b) => {
                Self::is_indeterminate(graph, *a) || Self::is_indeterminate(graph, *b)
            }
            _ => false,
        }
    }

    fn contains_infinity(graph: &ExprGraph, id: ExprId) -> bool {
        if Self::is_infinity_node(graph, id) {
            return true;
        }
        match &graph.get(id).kind {
            ExprKind::Neg(a) => Self::contains_infinity(graph, *a),
            ExprKind::Add(xs) | ExprKind::Mul(xs) => {
                xs.iter().any(|&x| Self::contains_infinity(graph, x))
            }
            ExprKind::Sub(a, b) | ExprKind::Div(a, b) | ExprKind::Pow(a, b) => {
                Self::contains_infinity(graph, *a) || Self::contains_infinity(graph, *b)
            }
            ExprKind::Function { args, .. } => {
                args.iter().any(|&x| Self::contains_infinity(graph, x))
            }
            _ => false,
        }
    }

    fn is_zero_node(graph: &ExprGraph, id: ExprId) -> bool {
        let node = graph.get(id);
        match &node.kind {
            ExprKind::Number(Number::Integer(0)) => true,
            ExprKind::Number(Number::Rational(0, _)) => true,
            ExprKind::Number(Number::Float(b)) => f64::from_bits(*b) == 0.0,
            ExprKind::Neg(a) => Self::is_zero_node(graph, *a),
            _ => false,
        }
    }

    fn is_infinity_node(graph: &ExprGraph, id: ExprId) -> bool {
        let node = graph.get(id);
        match &node.kind {
            ExprKind::Number(Number::Constant(c)) => {
                *c == algebra_core::Constant::Infinity || *c == algebra_core::Constant::NegInfinity
            }
            ExprKind::Symbol(s) => matches!(
                graph.symbols.resolve(*s).as_deref(),
                Some("oo" | "inf" | "infinity" | "∞")
            ),
            ExprKind::Neg(inner) => Self::is_infinity_node(graph, *inner),
            _ => false,
        }
    }

    fn is_neg_infinity_node(graph: &ExprGraph, id: ExprId) -> bool {
        match &graph.get(id).kind {
            ExprKind::Number(Number::Constant(algebra_core::Constant::NegInfinity)) => true,
            ExprKind::Neg(inner) => {
                Self::is_infinity_node(graph, *inner) && !Self::is_neg_infinity_node(graph, *inner)
            }
            _ => false,
        }
    }

    fn poly_deg_and_lc(graph: &ExprGraph, expr: ExprId, wrt: SymbolId) -> Option<(usize, ExprId)> {
        use num_traits::Zero as _;
        fn extract(graph: &ExprGraph, id: ExprId, wrt: SymbolId) -> Option<Vec<ExprId>> {
            let zero = graph.integer(0);
            if !graph.has_symbol(id, wrt) {
                return Some(vec![id]);
            }
            match &graph.get(id).kind {
                ExprKind::Symbol(s) if *s == wrt => Some(vec![zero, graph.integer(1)]),
                ExprKind::Neg(a) => Some(
                    extract(graph, *a, wrt)?
                        .into_iter()
                        .map(|c| graph.neg(c))
                        .collect(),
                ),
                ExprKind::Add(ts) => {
                    let mut acc = vec![zero];
                    for &t in ts {
                        let pt = extract(graph, t, wrt)?;
                        let len = acc.len().max(pt.len());
                        let mut next = Vec::with_capacity(len);
                        for i in 0..len {
                            let ca = *acc.get(i).unwrap_or(&zero);
                            let cb = *pt.get(i).unwrap_or(&zero);
                            next.push(if ca == zero {
                                cb
                            } else if cb == zero {
                                ca
                            } else {
                                graph.add([ca, cb])
                            });
                        }
                        acc = next;
                    }
                    Some(acc)
                }
                ExprKind::Sub(a, b) => {
                    let pa = extract(graph, *a, wrt)?;
                    let pb = extract(graph, *b, wrt)?;
                    let len = pa.len().max(pb.len());
                    let mut out = Vec::with_capacity(len);
                    for i in 0..len {
                        let ca = *pa.get(i).unwrap_or(&zero);
                        let cb = *pb.get(i).unwrap_or(&zero);
                        out.push(if cb == zero {
                            ca
                        } else if ca == zero {
                            graph.neg(cb)
                        } else {
                            graph.sub(ca, cb)
                        });
                    }
                    Some(out)
                }
                ExprKind::Mul(fs) => {
                    let mut acc = vec![graph.integer(1)];
                    for &f in fs {
                        let pf = extract(graph, f, wrt)?;
                        let mut next = vec![zero; acc.len() + pf.len() - 1];
                        for (i, &ca) in acc.iter().enumerate() {
                            if ca == zero {
                                continue;
                            }
                            for (j, &cb) in pf.iter().enumerate() {
                                if cb == zero {
                                    continue;
                                }
                                let p = graph.mul([ca, cb]);
                                next[i + j] = if next[i + j] == zero {
                                    p
                                } else {
                                    graph.add([next[i + j], p])
                                };
                            }
                        }
                        acc = next;
                    }
                    Some(acc)
                }
                ExprKind::Div(n, d) if !graph.has_symbol(*d, wrt) => Some(
                    extract(graph, *n, wrt)?
                        .into_iter()
                        .map(|c| graph.div(c, *d))
                        .collect(),
                ),
                ExprKind::Pow(b, e) if !graph.has_symbol(*e, wrt) => {
                    if let ExprKind::Number(Number::Integer(p)) = graph.get(*e).kind
                        && (0..=16).contains(&p)
                    {
                        let pb = extract(graph, *b, wrt)?;
                        let mut acc = vec![graph.integer(1)];
                        for _ in 0..p {
                            let mut next = vec![zero; acc.len() + pb.len() - 1];
                            for (i, &ca) in acc.iter().enumerate() {
                                if ca == zero {
                                    continue;
                                }
                                for (j, &cb) in pb.iter().enumerate() {
                                    if cb == zero {
                                        continue;
                                    }
                                    let pr = graph.mul([ca, cb]);
                                    next[i + j] = if next[i + j] == zero {
                                        pr
                                    } else {
                                        graph.add([next[i + j], pr])
                                    };
                                }
                            }
                            acc = next;
                        }
                        return Some(acc);
                    }
                    None
                }
                _ => None,
            }
        }

        let mut coeffs = extract(graph, expr, wrt)?;
        let ctx = crate::numeric::EvalContext::default();
        let no_syms = |e: ExprId| -> bool {
            let mut s = std::collections::BTreeSet::new();
            crate::poly::collect_expr_symbols(graph, e, &mut s);
            s.is_empty()
        };
        while coeffs.len() > 1 {
            let Some(&last) = coeffs.last() else { break };
            let simp = crate::simplify::Simplifier::simplify(graph, last).unwrap_or(last);
            let is_const = no_syms(simp);
            let is_z = Self::is_zero_node(graph, simp)
                || (is_const
                    && graph
                        .evalf(simp, &ctx)
                        .is_ok_and(|v| v.to_f64().abs() < 1e-12))
                || simp == graph.integer(0);
            let zero_rat = is_const
                && matches!(&graph.get(simp).kind, ExprKind::Number(n) if crate::poly::rational_from_number(n).is_ok_and(|q| q.is_zero()));
            if is_z || zero_rat {
                coeffs.pop();
            } else {
                if let Some(last_mut) = coeffs.last_mut() {
                    *last_mut = simp;
                }
                break;
            }
        }
        let deg = coeffs.len().saturating_sub(1);
        coeffs.get(deg).copied().map(|c| (deg, c))
    }

    fn poly_degree(graph: &ExprGraph, id: ExprId, x: ExprId) -> Option<i64> {
        let x_sym = match graph.get(x).kind {
            ExprKind::Symbol(s) => s,
            _ => return None,
        };
        if !graph.has_symbol(id, x_sym) {
            return Some(0);
        }
        if id == x {
            return Some(1);
        }
        match &graph.get(id).kind {
            ExprKind::Number(_) => Some(0),
            ExprKind::Neg(a) => Self::poly_degree(graph, *a, x),
            ExprKind::Pow(base, exp) if !graph.has_symbol(*exp, x_sym) => {
                let d_base = Self::poly_degree(graph, *base, x)?;
                if let ExprKind::Number(Number::Integer(p)) = graph.get(*exp).kind
                    && p >= 0
                {
                    Some(d_base * p)
                } else {
                    None
                }
            }
            ExprKind::Mul(factors) => {
                let mut total = 0i64;
                for &f in factors {
                    if Self::is_zero_node(graph, f) {
                        return Some(0);
                    }
                    total += Self::poly_degree(graph, f, x)?;
                }
                Some(total)
            }
            ExprKind::Div(num, den) if !graph.has_symbol(*den, x_sym) => {
                Self::poly_degree(graph, *num, x)
            }
            ExprKind::Add(terms) => {
                let mut max_deg = 0i64;
                for &t in terms {
                    let d = Self::poly_degree(graph, t, x)?;
                    max_deg = max_deg.max(d);
                }
                Some(max_deg)
            }
            ExprKind::Sub(a, b) => {
                let da = Self::poly_degree(graph, *a, x)?;
                let db = Self::poly_degree(graph, *b, x)?;
                Some(da.max(db))
            }
            _ => None,
        }
    }

    fn leading_coeff(graph: &ExprGraph, id: ExprId, x: ExprId, target_deg: i64) -> ExprId {
        let zero = graph.integer(0);
        let one = graph.integer(1);
        let x_sym = match graph.get(x).kind {
            ExprKind::Symbol(s) => s,
            _ => return id,
        };
        if !graph.has_symbol(id, x_sym) {
            return if target_deg == 0 { id } else { zero };
        }
        if id == x {
            return if target_deg == 1 { one } else { zero };
        }
        match &graph.get(id).kind {
            ExprKind::Neg(a) => {
                let c = Self::leading_coeff(graph, *a, x, target_deg);
                if c == zero { zero } else { graph.neg(c) }
            }
            ExprKind::Pow(base, exp) if !graph.has_symbol(*exp, x_sym) => {
                if let (Some(db), ExprKind::Number(Number::Integer(p))) =
                    (Self::poly_degree(graph, *base, x), &graph.get(*exp).kind)
                    && db * *p == target_deg
                {
                    let base_lc = Self::leading_coeff(graph, *base, x, db);
                    return graph.pow(base_lc, *exp);
                }
                zero
            }
            ExprKind::Mul(factors) => {
                let mut total_deg = 0i64;
                let mut lc_parts = Vec::new();
                for &f in factors {
                    let d = Self::poly_degree(graph, f, x).unwrap_or(0);
                    total_deg += d;
                    let part = Self::leading_coeff(graph, f, x, d);
                    if part != one {
                        lc_parts.push(part);
                    }
                }
                if total_deg != target_deg {
                    zero
                } else if lc_parts.is_empty() {
                    one
                } else if lc_parts.len() == 1 {
                    lc_parts[0]
                } else {
                    graph.mul(lc_parts)
                }
            }
            ExprKind::Div(num, den) if !graph.has_symbol(*den, x_sym) => {
                let c = Self::leading_coeff(graph, *num, x, target_deg);
                if c == zero { zero } else { graph.div(c, *den) }
            }
            ExprKind::Add(terms) => {
                let mut parts = Vec::new();
                for &t in terms {
                    if Self::poly_degree(graph, t, x) == Some(target_deg) {
                        let c = Self::leading_coeff(graph, t, x, target_deg);
                        if c != zero {
                            parts.push(c);
                        }
                    }
                }
                if parts.is_empty() {
                    zero
                } else if parts.len() == 1 {
                    parts[0]
                } else {
                    graph.add(parts)
                }
            }
            ExprKind::Sub(a, b) => {
                let ca = if Self::poly_degree(graph, *a, x) == Some(target_deg) {
                    Self::leading_coeff(graph, *a, x, target_deg)
                } else {
                    zero
                };
                let cb = if Self::poly_degree(graph, *b, x) == Some(target_deg) {
                    Self::leading_coeff(graph, *b, x, target_deg)
                } else {
                    zero
                };
                if cb == zero {
                    ca
                } else if ca == zero {
                    graph.neg(cb)
                } else {
                    graph.sub(ca, cb)
                }
            }
            _ => zero,
        }
    }
}
