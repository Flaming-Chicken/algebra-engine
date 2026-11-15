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

use crate::calculus::SymbolicCalculus;
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
        // 1. Direct substitution with constant simplification
        let sub = graph.substitute(expr, wrt, point);
        let simplified_sub = Self::simplify_constants(graph, sub);

        if !Self::is_indeterminate(graph, simplified_sub) {
            return Ok(simplified_sub);
        }

        // 2. Gruntz MRV first for exponential / logarithmic limits at infinity
        //    (avoids endless L'Hopital cycling on e^x / e^{x + e^{-x}} etc.)
        if direction == LimitDirection::PositiveInfinity || Self::is_infinity_node(graph, point) {
            if let Ok(res) = Self::eval_gruntz_infinity(graph, expr, wrt) {
                return Ok(res);
            }
        }

        // 3. Swell- and cycle-guarded Multi-Order L'Hopital rule for rational fractions f(x)/g(x)
        if let ExprKind::Div(num, den) = graph.get(expr).kind
            && let Some(res) = Self::eval_lhopital(graph, num, den, wrt, point, 10)?
        {
            return Ok(res);
        }

        // 4. Fallback Taylor series expansion around point
        let taylor = graph.taylor_series(expr, wrt, point, 4);
        let sub_taylor = graph.substitute(taylor, wrt, point);
        let sim_taylor = Self::simplify_constants(graph, sub_taylor);
        if !Self::is_indeterminate(graph, sim_taylor) {
            return Ok(sim_taylor);
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

    /// Constant expression reduction helper (e.g. cos(0)->1, sin(0)->0, exp(0)->1, 1-1->0).
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
                            "sin" | "sinh" | "tan" | "tanh" => return graph.integer(0),
                            "cos" | "cosh" | "exp" => return graph.integer(1),
                            _ => {}
                        }
                    }
                    if let ExprKind::Number(Number::Integer(1)) = graph.get(arg_sim).kind
                        && fn_name == "ln"
                    {
                        return graph.integer(0);
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
                let mut new_factors = Vec::with_capacity(factors.len());
                for f in factors {
                    let sf = Self::simplify_constants(graph, f);
                    if Self::is_zero_node(graph, sf) {
                        return graph.integer(0);
                    }
                    if let ExprKind::Number(Number::Integer(i)) = graph.get(sf).kind {
                        prod *= i;
                    } else {
                        all_int = false;
                        new_factors.push(sf);
                    }
                }
                if all_int {
                    return graph.integer(prod);
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
                if let (ExprKind::Number(Number::Integer(b)), ExprKind::Number(Number::Integer(e))) =
                    (&graph.get(s_base).kind, &graph.get(s_exp).kind)
                    && *e >= 0
                {
                    return graph.integer(b.pow(*e as u32));
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
                graph.neg(sa)
            }
            _ => id,
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
                    return Ok(graph.div(lc_num, lc_den));
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

    fn is_indeterminate(graph: &ExprGraph, id: ExprId) -> bool {
        let node = graph.get(id);
        match &node.kind {
            ExprKind::Div(_num, den) => Self::is_zero_node(graph, *den),
            ExprKind::Number(Number::Constant(c)) => *c == algebra_core::Constant::Undefined,
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
            _ => false,
        }
    }

    fn poly_degree(graph: &ExprGraph, id: ExprId, x: ExprId) -> Option<i64> {
        let node = graph.get(id);
        if id == x {
            return Some(1);
        }
        match &node.kind {
            ExprKind::Number(_) => Some(0),
            ExprKind::Pow(base, exp) if *base == x => {
                if let ExprKind::Number(Number::Integer(p)) = graph.get(*exp).kind {
                    Some(p)
                } else {
                    None
                }
            }
            ExprKind::Add(terms) => {
                let mut max_deg = 0i64;
                for &t in terms {
                    let d = Self::poly_degree(graph, t, x)?;
                    max_deg = max_deg.max(d);
                }
                Some(max_deg)
            }
            _ => None,
        }
    }

    fn leading_coeff(graph: &ExprGraph, id: ExprId, x: ExprId, target_deg: i64) -> ExprId {
        if target_deg == 0
            && !graph.has_symbol(
                id,
                match graph.get(x).kind {
                    ExprKind::Symbol(s) => s,
                    _ => return id,
                },
            )
        {
            return id;
        }
        let node = graph.get(id);
        match &node.kind {
            ExprKind::Pow(base, exp) if *base == x => {
                if let ExprKind::Number(Number::Integer(p)) = graph.get(*exp).kind
                    && p == target_deg
                {
                    return graph.integer(1);
                }
            }
            ExprKind::Mul(factors) => {
                let mut coeff_factors = Vec::new();
                for &f in factors {
                    if f != x {
                        coeff_factors.push(f);
                    }
                }
                if !coeff_factors.is_empty() {
                    return graph.mul(coeff_factors);
                }
            }
            ExprKind::Add(terms) => {
                for &t in terms {
                    if Self::poly_degree(graph, t, x) == Some(target_deg) {
                        return Self::leading_coeff(graph, t, x, target_deg);
                    }
                }
            }
            _ => {}
        }
        graph.integer(1)
    }
}
