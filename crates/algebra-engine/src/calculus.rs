//! # `algebra-calculus`
//!
//! Analytical calculus (differentiation, Taylor series, Gruntz limits, Risch integration)
//! and Discrete calculus (finite differences, Gosper summation).

use crate::simplify::Simplifier;
use algebra_core::{
    AlgebraError, AlgebraResult, ExprGraph, ExprId, ExprKind, MathStep, Number, StepByStepResult,
    SymbolId,
};

/// Analytical calculus extension trait for symbolic expressions.
pub trait SymbolicCalculus {
    /// Compute partial derivative d(f)/d(x) with respect to variable symbol `wrt` (Fast zero-overhead pathway).
    fn diff(&self, expr: ExprId, wrt: SymbolId) -> ExprId;

    /// Compute partial derivative d(f)/d(x) with respect to variable symbol `wrt`,
    /// returning the result alongside explicit step-by-step educational derivation steps.
    fn diff_with_steps(&self, expr: ExprId, wrt: SymbolId) -> StepByStepResult<ExprId>;

    /// Compute forward finite difference Δ f(x) = f(x + h) - f(x).
    fn forward_difference(&self, expr: ExprId, wrt: SymbolId, step: ExprId) -> ExprId;

    /// Compute Taylor series expansion of `expr` around `point` up to `order`.
    fn taylor_series(&self, expr: ExprId, wrt: SymbolId, point: ExprId, order: u32) -> ExprId;

    /// Compute symbolic limit $\lim_{x \to point} f(x)$ using Gruntz algorithm heuristics.
    fn limit(&self, expr: ExprId, wrt: SymbolId, point: ExprId) -> AlgebraResult<ExprId>;

    /// Compute elementary symbolic integral $\int f(x) dx$ using Risch integration heuristics.
    fn integrate(&self, expr: ExprId, wrt: SymbolId) -> AlgebraResult<ExprId>;

    /// Compute elementary symbolic integral $\int f(x) dx$ alongside step-by-step derivation steps.
    fn integrate_with_steps(
        &self,
        expr: ExprId,
        wrt: SymbolId,
    ) -> AlgebraResult<StepByStepResult<ExprId>>;

    /// Compute definite integral $\int_a^b f(x) dx$ via the Fundamental Theorem of Calculus ($F(b) - F(a)$).
    fn integrate_definite(
        &self,
        expr: ExprId,
        wrt: SymbolId,
        lower: ExprId,
        upper: ExprId,
    ) -> AlgebraResult<ExprId>;

    /// Compute symbolic summation $\sum_{k=lower}^{upper} f(k)$ using Gosper summation heuristics.
    fn summation(
        &self,
        expr: ExprId,
        wrt: SymbolId,
        lower: ExprId,
        upper: ExprId,
    ) -> AlgebraResult<ExprId>;
}

impl SymbolicCalculus for ExprGraph {
    fn diff(&self, expr: ExprId, wrt: SymbolId) -> ExprId {
        let node = self.get(expr);
        match &node.kind {
            ExprKind::Number(_) => self.integer(0),
            ExprKind::Symbol(s) => {
                if *s == wrt {
                    self.integer(1)
                } else {
                    self.integer(0)
                }
            }
            ExprKind::Add(terms) => {
                let diff_terms: Vec<ExprId> = terms.iter().map(|&t| self.diff(t, wrt)).collect();
                self.add(diff_terms)
            }
            ExprKind::Sub(lhs, rhs) => {
                let dl = self.diff(*lhs, wrt);
                let dr = self.diff(*rhs, wrt);
                self.sub(dl, dr)
            }
            ExprKind::Mul(factors) => {
                if factors.len() == 2 {
                    let u = factors[0];
                    let v = factors[1];
                    let du = self.diff(u, wrt);
                    let dv = self.diff(v, wrt);
                    let t1 = self.mul([du, v]);
                    let t2 = self.mul([u, dv]);
                    self.add([t1, t2])
                } else if factors.is_empty() {
                    self.integer(0)
                } else {
                    let u = factors[0];
                    let rest_factors: Vec<ExprId> = factors[1..].to_vec();
                    let rest = self.mul(rest_factors);
                    let du = self.diff(u, wrt);
                    let drest = self.diff(rest, wrt);
                    let t1 = self.mul([du, rest]);
                    let t2 = self.mul([u, drest]);
                    self.add([t1, t2])
                }
            }
            ExprKind::Div(num, den) => {
                let u = *num;
                let v = *den;
                let du = self.diff(u, wrt);
                let dv = self.diff(v, wrt);

                let du_v = self.mul([du, v]);
                let u_dv = self.mul([u, dv]);
                let num_diff = self.sub(du_v, u_dv);

                let two = self.integer(2);
                let v_sq = self.pow(v, two);
                self.div(num_diff, v_sq)
            }
            ExprKind::Pow(base, exp) => {
                let b = *base;
                let e = *exp;
                if !self.has_symbol(e, wrt) {
                    let db = self.diff(b, wrt);
                    if db == self.integer(0) {
                        return self.integer(0);
                    }
                    let one = self.integer(1);
                    let e_minus_1 = self.sub(e, one);
                    let b_pow = self.pow(b, e_minus_1);
                    let term = self.mul([e, b_pow]);
                    self.mul([term, db])
                } else {
                    let db = self.diff(b, wrt);
                    let de = self.diff(e, wrt);
                    let zero = self.integer(0);
                    let one = self.integer(1);

                    let mut inner_terms = Vec::new();
                    if de != zero {
                        let ln_b = self.function("ln", [b]);
                        if de == one {
                            inner_terms.push(ln_b);
                        } else {
                            inner_terms.push(self.mul([de, ln_b]));
                        }
                    }
                    if db != zero {
                        let b_term = if e == b && db == one {
                            one
                        } else if db == one {
                            self.div(e, b)
                        } else {
                            self.div(self.mul([e, db]), b)
                        };
                        inner_terms.push(b_term);
                    }

                    if inner_terms.is_empty() {
                        zero
                    } else if inner_terms.len() == 1 {
                        self.mul([expr, inner_terms[0]])
                    } else {
                        let sum_inner = self.add(inner_terms);
                        self.mul([expr, sum_inner])
                    }
                }
            }
            ExprKind::Neg(inner) => {
                let d_inner = self.diff(*inner, wrt);
                self.neg(d_inner)
            }
            ExprKind::Function { name, args } => {
                let fn_name = self.symbols.resolve(*name).unwrap_or_default();
                if args.len() == 1 {
                    let arg = args[0];
                    let darg = self.diff(arg, wrt);
                    match fn_name.as_str() {
                        "sin" => {
                            let cos_arg = self.function("cos", [arg]);
                            self.mul([cos_arg, darg])
                        }
                        "cos" => {
                            let sin_arg = self.function("sin", [arg]);
                            let neg_sin = self.neg(sin_arg);
                            self.mul([neg_sin, darg])
                        }
                        "tan" => {
                            let sec_sq = self.pow(self.function("cos", [arg]), self.integer(-2));
                            self.mul([sec_sq, darg])
                        }
                        "sinh" => {
                            let cosh_arg = self.function("cosh", [arg]);
                            self.mul([cosh_arg, darg])
                        }
                        "cosh" => {
                            let sinh_arg = self.function("sinh", [arg]);
                            self.mul([sinh_arg, darg])
                        }
                        "tanh" => {
                            let cosh_sq = self.pow(self.function("cosh", [arg]), self.integer(-2));
                            self.mul([cosh_sq, darg])
                        }
                        "sec" => {
                            let sec_arg = self.function("sec", [arg]);
                            let tan_arg = self.function("tan", [arg]);
                            self.mul([sec_arg, tan_arg, darg])
                        }
                        "csc" => {
                            let csc_arg = self.function("csc", [arg]);
                            let cot_arg = self.function("cot", [arg]);
                            let neg_csc_cot = self.neg(self.mul([csc_arg, cot_arg]));
                            self.mul([neg_csc_cot, darg])
                        }
                        "cot" => {
                            let csc_sq = self.pow(self.function("csc", [arg]), self.integer(2));
                            let neg_csc_sq = self.neg(csc_sq);
                            self.mul([neg_csc_sq, darg])
                        }
                        "sech" => {
                            let sech_arg = self.function("sech", [arg]);
                            let tanh_arg = self.function("tanh", [arg]);
                            let neg_sech_tanh = self.neg(self.mul([sech_arg, tanh_arg]));
                            self.mul([neg_sech_tanh, darg])
                        }
                        "csch" => {
                            let csch_arg = self.function("csch", [arg]);
                            let coth_arg = self.function("coth", [arg]);
                            let neg_csch_coth = self.neg(self.mul([csch_arg, coth_arg]));
                            self.mul([neg_csch_coth, darg])
                        }
                        "coth" => {
                            let csch_sq = self.pow(self.function("csch", [arg]), self.integer(2));
                            let neg_csch_sq = self.neg(csch_sq);
                            self.mul([neg_csch_sq, darg])
                        }
                        "asinh" => {
                            let one = self.integer(1);
                            let arg_sq = self.pow(arg, self.integer(2));
                            let one_plus_sq = self.add([arg_sq, one]);
                            let half = self.rational(1, 2);
                            let denom = self.pow(one_plus_sq, half);
                            let deriv = self.div(self.integer(1), denom);
                            self.mul([deriv, darg])
                        }
                        "acosh" => {
                            let one = self.integer(1);
                            let arg_sq = self.pow(arg, self.integer(2));
                            let sq_minus_one = self.sub(arg_sq, one);
                            let half = self.rational(1, 2);
                            let denom = self.pow(sq_minus_one, half);
                            let deriv = self.div(self.integer(1), denom);
                            self.mul([deriv, darg])
                        }
                        "atanh" => {
                            let one = self.integer(1);
                            let arg_sq = self.pow(arg, self.integer(2));
                            let one_minus_sq = self.sub(one, arg_sq);
                            let deriv = self.div(self.integer(1), one_minus_sq);
                            self.mul([deriv, darg])
                        }
                        "abs" | "Abs" => {
                            let sign_arg = self.function("sign", [arg]);
                            self.mul([sign_arg, darg])
                        }
                        "asin" => {
                            let one = self.integer(1);
                            let arg_sq = self.pow(arg, self.integer(2));
                            let one_minus_sq = self.sub(one, arg_sq);
                            let half = self.rational(1, 2);
                            let denom = self.pow(one_minus_sq, half);
                            let deriv = self.div(self.integer(1), denom);
                            self.mul([deriv, darg])
                        }
                        "acos" => {
                            let one = self.integer(1);
                            let arg_sq = self.pow(arg, self.integer(2));
                            let one_minus_sq = self.sub(one, arg_sq);
                            let half = self.rational(1, 2);
                            let denom = self.pow(one_minus_sq, half);
                            let deriv = self.div(self.integer(-1), denom);
                            self.mul([deriv, darg])
                        }
                        "atan" => {
                            let one = self.integer(1);
                            let arg_sq = self.pow(arg, self.integer(2));
                            let one_plus_sq = self.add([one, arg_sq]);
                            let deriv = self.div(self.integer(1), one_plus_sq);
                            self.mul([deriv, darg])
                        }
                        "sqrt" => {
                            let sqrt_arg = self.function("sqrt", [arg]);
                            let two_sqrt = self.mul([self.integer(2), sqrt_arg]);
                            let deriv = self.div(self.integer(1), two_sqrt);
                            self.mul([deriv, darg])
                        }
                        "exp" => {
                            let exp_arg = self.function("exp", [arg]);
                            self.mul([exp_arg, darg])
                        }
                        "ln" | "log" => {
                            let reciprocal = self.div(self.integer(1), arg);
                            self.mul([reciprocal, darg])
                        }
                        "gamma" => {
                            let psi_arg = self.function("digamma", [arg]);
                            let gamma_arg = self.function("gamma", [arg]);
                            let prod = self.mul([gamma_arg, psi_arg]);
                            self.mul([prod, darg])
                        }
                        "erf" => {
                            let pi = self.constant(algebra_core::Constant::Pi);
                            let sqrt_pi = self.function("sqrt", [pi]);
                            let two_over_sqrt_pi = self.div(self.integer(2), sqrt_pi);
                            let neg_arg_sq = self.neg(self.pow(arg, self.integer(2)));
                            let exp_term = self.function("exp", [neg_arg_sq]);
                            let deriv = self.mul([two_over_sqrt_pi, exp_term]);
                            self.mul([deriv, darg])
                        }
                        "erfc" => {
                            let pi = self.constant(algebra_core::Constant::Pi);
                            let sqrt_pi = self.function("sqrt", [pi]);
                            let neg_two_over_sqrt_pi = self.div(self.integer(-2), sqrt_pi);
                            let neg_arg_sq = self.neg(self.pow(arg, self.integer(2)));
                            let exp_term = self.function("exp", [neg_arg_sq]);
                            let deriv = self.mul([neg_two_over_sqrt_pi, exp_term]);
                            self.mul([deriv, darg])
                        }
                        "lambert_w" | "LambertW" | "W" => {
                            let w_node = self.function("lambert_w", [arg]);
                            let one = self.integer(1);
                            let one_plus_w = self.add([one, w_node]);
                            let denom = self.mul([arg, one_plus_w]);
                            let deriv = self.div(w_node, denom);
                            self.mul([deriv, darg])
                        }
                        "si" | "Si" => {
                            let sin_arg = self.function("sin", [arg]);
                            let deriv = self.div(sin_arg, arg);
                            self.mul([deriv, darg])
                        }
                        "ci" | "Ci" => {
                            let cos_arg = self.function("cos", [arg]);
                            let deriv = self.div(cos_arg, arg);
                            self.mul([deriv, darg])
                        }
                        "ei" | "Ei" => {
                            let exp_arg = self.function("exp", [arg]);
                            let deriv = self.div(exp_arg, arg);
                            self.mul([deriv, darg])
                        }
                        "li" => {
                            let ln_arg = self.function("ln", [arg]);
                            let deriv = self.div(self.integer(1), ln_arg);
                            self.mul([deriv, darg])
                        }
                        _ => self.derivative(
                            expr,
                            self.symbols.resolve(wrt).unwrap_or_default().as_str(),
                            1,
                        ),
                    }
                } else if args.len() == 2 && fn_name == "log" {
                    let ln_u = self.function("ln", [args[0]]);
                    let ln_b = self.function("ln", [args[1]]);
                    let ratio = self.div(ln_u, ln_b);
                    self.diff(ratio, wrt)
                } else {
                    self.derivative(
                        expr,
                        self.symbols.resolve(wrt).unwrap_or_default().as_str(),
                        1,
                    )
                }
            }
            _ => self.derivative(
                expr,
                self.symbols.resolve(wrt).unwrap_or_default().as_str(),
                1,
            ),
        }
    }

    fn diff_with_steps(&self, expr: ExprId, wrt: SymbolId) -> StepByStepResult<ExprId> {
        let mut steps = Vec::new();
        let wrt_name = self.symbols.resolve(wrt).unwrap_or_else(|| "x".to_string());

        let node = self.get(expr);
        let res = match &node.kind {
            ExprKind::Number(_) => {
                let res = self.integer(0);
                steps.push(MathStep::new(
                    1,
                    "Constant Rule",
                    format!("Derivative of constant with respect to {} is 0", wrt_name),
                    expr,
                    res,
                ));
                res
            }
            ExprKind::Symbol(s) => {
                let res = if *s == wrt {
                    self.integer(1)
                } else {
                    self.integer(0)
                };
                steps.push(MathStep::new(
                    1,
                    "Variable Rule",
                    format!("Derivative of symbol w.r.t. {}", wrt_name),
                    expr,
                    res,
                ));
                res
            }
            ExprKind::Pow(_b, _e) => {
                let res = self.diff(expr, wrt);
                steps.push(MathStep::new(
                    1,
                    "Power Rule",
                    format!(
                        "Applied Power Rule d/d{}[u^n] = n * u^(n-1) * du/d{}",
                        wrt_name, wrt_name
                    ),
                    expr,
                    res,
                ));
                res
            }
            ExprKind::Add(_) => {
                let res = self.diff(expr, wrt);
                steps.push(MathStep::new(
                    1,
                    "Sum Rule",
                    format!("Applied Sum Rule: Differentiate each term with respect to {} independently", wrt_name),
                    expr,
                    res,
                ));
                res
            }
            ExprKind::Mul(_) => {
                let res = self.diff(expr, wrt);
                steps.push(MathStep::new(
                    1,
                    "Product Rule",
                    format!(
                        "Applied Product Rule: d/d{}[u * v] = u' * v + u * v'",
                        wrt_name
                    ),
                    expr,
                    res,
                ));
                res
            }
            _ => {
                let res = self.diff(expr, wrt);
                steps.push(MathStep::new(
                    1,
                    "Chain Rule",
                    format!(
                        "Applied Chain Rule differentiation with respect to {}",
                        wrt_name
                    ),
                    expr,
                    res,
                ));
                res
            }
        };

        StepByStepResult::new(res, steps)
    }

    fn forward_difference(&self, expr: ExprId, wrt: SymbolId, step: ExprId) -> ExprId {
        let wrt_name = self.symbols.resolve(wrt).unwrap_or_default();
        let x_sym = self.symbol(&wrt_name);
        let x_plus_h = self.add([x_sym, step]);

        let f_x_plus_h = self.substitute(expr, wrt, x_plus_h);
        self.sub(f_x_plus_h, expr)
    }

    fn taylor_series(&self, expr: ExprId, wrt: SymbolId, point: ExprId, order: u32) -> ExprId {
        let wrt_name = self.symbols.resolve(wrt).unwrap_or_default();
        let x_sym = self.symbol(&wrt_name);
        let zero = self.integer(0);
        let x_minus_a = if point == zero {
            x_sym
        } else {
            self.sub(x_sym, point)
        };

        // Removable singularity quotient f(x)/g(x) where g(point) == 0:
        // Expand numerator and denominator via Taylor coefficients and divide power series directly.
        if let ExprKind::Div(num, den) = self.get(expr).kind {
            let sub_den = self.substitute(den, wrt, point);
            let den_at_pt = crate::limits::LimitEngine::simplify_constants(self, sub_den);
            let den_at_pt =
                crate::simplify::Simplifier::simplify(self, den_at_pt).unwrap_or(den_at_pt);
            if den_at_pt == zero
                && let Some(q_coeffs) =
                    taylor_div_coeffs(self, num, den, wrt, point, order as usize)
            {
                return build_taylor_poly(self, &q_coeffs, x_minus_a);
            }
        }

        let coeffs = taylor_coeffs_vec(self, expr, wrt, point, order as usize);
        build_taylor_poly(self, &coeffs, x_minus_a)
    }

    fn limit(&self, expr: ExprId, wrt: SymbolId, point: ExprId) -> AlgebraResult<ExprId> {
        crate::limits::LimitEngine::limit(self, expr, wrt, point)
    }

    fn integrate(&self, expr: ExprId, wrt: SymbolId) -> AlgebraResult<ExprId> {
        crate::risch::RischIntegrator::integrate(self, expr, wrt)
    }

    fn integrate_with_steps(
        &self,
        expr: ExprId,
        wrt: SymbolId,
    ) -> AlgebraResult<StepByStepResult<ExprId>> {
        let wrt_name = self.symbols.resolve(wrt).unwrap_or_else(|| "x".to_string());
        let mut steps = Vec::new();
        let node = self.get(expr);

        let res = match &node.kind {
            ExprKind::Number(_) => {
                let res = self.integrate(expr, wrt)?;
                steps.push(MathStep::new(
                    1,
                    "Constant Rule",
                    format!("Integrate constant with respect to {wrt_name}: ∫ c d{wrt_name} = c · {wrt_name}"),
                    expr,
                    res,
                ));
                res
            }
            ExprKind::Symbol(s) if *s == wrt => {
                let res = self.integrate(expr, wrt)?;
                steps.push(MathStep::new(
                    1,
                    "Power Rule",
                    format!(
                        "Applied Power Rule with n = 1: ∫ {wrt_name} d{wrt_name} = {wrt_name}^2 / 2"
                    ),
                    expr,
                    res,
                ));
                res
            }
            ExprKind::Symbol(_) => {
                let res = self.integrate(expr, wrt)?;
                steps.push(MathStep::new(
                    1,
                    "Constant Symbol Rule",
                    format!("Treat symbol independent of {wrt_name} as constant: ∫ k d{wrt_name} = k · {wrt_name}"),
                    expr,
                    res,
                ));
                res
            }
            ExprKind::Add(terms) => {
                let mut int_terms = Vec::with_capacity(terms.len());
                for (idx, &term) in terms.iter().enumerate() {
                    let sub_int = self.integrate(term, wrt)?;
                    int_terms.push(sub_int);
                    steps.push(MathStep::new(
                        idx + 1,
                        "Sum Linearity Term",
                        format!(
                            "Integrated additive term {} of {} with respect to {wrt_name}",
                            idx + 1,
                            terms.len()
                        ),
                        term,
                        sub_int,
                    ));
                }
                let combined = self.add(int_terms);
                steps.push(MathStep::new(
                    terms.len() + 1,
                    "Linear Combination",
                    format!("Summed integrated terms with respect to {wrt_name}"),
                    expr,
                    combined,
                ));
                combined
            }
            ExprKind::Sub(lhs, rhs) => {
                let il = self.integrate(*lhs, wrt)?;
                let ir = self.integrate(*rhs, wrt)?;
                steps.push(MathStep::new(
                    1,
                    "Difference Linearity (Left)",
                    format!("Integrated minuend with respect to {wrt_name}"),
                    *lhs,
                    il,
                ));
                steps.push(MathStep::new(
                    2,
                    "Difference Linearity (Right)",
                    format!("Integrated subtrahend with respect to {wrt_name}"),
                    *rhs,
                    ir,
                ));
                let combined = self.sub(il, ir);
                steps.push(MathStep::new(
                    3,
                    "Linear Combination",
                    "Combined minuend and subtrahend antiderivatives",
                    expr,
                    combined,
                ));
                combined
            }
            ExprKind::Mul(factors) => {
                let has_const = factors.iter().any(|&f| !self.has_symbol(f, wrt));
                let res = self.integrate(expr, wrt)?;
                let (title, desc) = if has_const {
                    (
                        "Constant Multiple / Risch Product Rule",
                        format!(
                            "Factored constants independent of {wrt_name} and integrated remaining factor"
                        ),
                    )
                } else {
                    (
                        "Integration by Parts / Risch Heuristic",
                        format!("Integrated product expression with respect to {wrt_name}"),
                    )
                };
                steps.push(MathStep::new(1, title, desc, expr, res));
                res
            }
            ExprKind::Pow(_, _) => {
                let res = self.integrate(expr, wrt)?;
                steps.push(MathStep::new(
                    1,
                    "Power Rule",
                    format!(
                        "Applied Power Rule ∫ {wrt_name}^n d{wrt_name} = {wrt_name}^(n+1) / (n+1)"
                    ),
                    expr,
                    res,
                ));
                res
            }
            ExprKind::Div(_, _) => {
                let res = self.integrate(expr, wrt)?;
                steps.push(MathStep::new(
                    1,
                    "Hermite / Rothstein-Trager Reduction",
                    format!("Integrated rational function in {wrt_name} via Hermite / logarithmic reduction"),
                    expr,
                    res,
                ));
                res
            }
            ExprKind::Function { name, .. } => {
                let fn_name = self.symbols.resolve(*name).unwrap_or_default();
                let res = self.integrate(expr, wrt)?;
                steps.push(MathStep::new(
                    1,
                    "Elementary Function Table",
                    format!("Applied elementary antiderivative table rule for '{fn_name}' with respect to {wrt_name}"),
                    expr,
                    res,
                ));
                res
            }
            _ => {
                let res = self.integrate(expr, wrt)?;
                steps.push(MathStep::new(
                    1,
                    "Risch Differential Field Heuristic",
                    format!("Computed antiderivative with respect to {wrt_name}"),
                    expr,
                    res,
                ));
                res
            }
        };

        Ok(StepByStepResult::new(res, steps))
    }

    fn integrate_definite(
        &self,
        expr: ExprId,
        wrt: SymbolId,
        lower: ExprId,
        upper: ExprId,
    ) -> AlgebraResult<ExprId> {
        use crate::limits::{LimitDirection, LimitEngine};
        use crate::numeric::NumericalEval as _;
        use crate::solver::SymbolicSolver as _;
        use algebra_core::{Constant, Number};

        let ctx = crate::numeric::EvalContext::default();
        let wrt_name = self.symbols.resolve(wrt).unwrap_or_else(|| "x".to_string());

        let is_sym_inf = |id: ExprId| -> bool {
            matches!(
                &self.get(id).kind,
                ExprKind::Symbol(s)
                    if matches!(
                        self.symbols.resolve(*s).as_deref(),
                        Some("oo" | "inf" | "infinity" | "∞")
                    )
            )
        };
        let is_pos_inf_bound = |id: ExprId| -> bool {
            matches!(
                self.get(id).kind,
                ExprKind::Number(Number::Constant(Constant::Infinity))
            ) || is_sym_inf(id)
        };
        let is_neg_inf_bound = |id: ExprId| -> bool {
            matches!(
                self.get(id).kind,
                ExprKind::Number(Number::Constant(Constant::NegInfinity))
            ) || matches!(self.get(id).kind, ExprKind::Neg(inner) if is_pos_inf_bound(inner))
        };
        let is_inf_bound = |id: ExprId| -> bool { is_pos_inf_bound(id) || is_neg_inf_bound(id) };

        let mut singular_lower = false;
        let mut singular_upper = false;

        // Check for interior denominator poles and flag boundary singularities for limit evaluation
        if let (Ok(low_v), Ok(up_v)) = (
            self.evalf(lower, &ctx).map(|v| v.to_f64()),
            self.evalf(upper, &ctx).map(|v| v.to_f64()),
        ) && low_v.is_finite()
            && up_v.is_finite()
        {
            let lo = low_v.min(up_v);
            let hi = low_v.max(up_v);
            let mut dens = Vec::new();
            collect_denominators(self, expr, wrt, &mut dens);
            for den in dens {
                if let Ok(roots) = self.solveset(den, wrt) {
                    for r in roots {
                        let real_part = match self.evalf(r, &ctx) {
                            Ok(crate::numeric::BigValue::Real(rv)) => Some(rv),
                            Ok(crate::numeric::BigValue::Complex(rv, iv)) if iv.abs() < 1e-9 => {
                                Some(rv)
                            }
                            _ => None,
                        };
                        if let Some(rv) = real_part
                            && rv.is_finite()
                        {
                            if (rv - low_v).abs() < 1e-9 {
                                singular_lower = true;
                            } else if (rv - up_v).abs() < 1e-9 {
                                singular_upper = true;
                            } else if rv > lo + 1e-9 && rv < hi - 1e-9 {
                                return Err(AlgebraError::EvaluationError(format!(
                                    "Definite integral diverges: singularity at {} = {}",
                                    wrt_name, rv
                                )));
                            }
                        }
                    }
                }
            }
        }

        let f_antideriv = self.integrate(expr, wrt)?;

        let eval_bound =
            |bound: ExprId, dir: LimitDirection, force_limit: bool| -> AlgebraResult<ExprId> {
                let need_limit = force_limit || is_inf_bound(bound);
                let raw = if need_limit {
                    LimitEngine::limit_directed(self, f_antideriv, wrt, bound, dir)?
                } else {
                    let sub = self.substitute(f_antideriv, wrt, bound);
                    let simp = crate::simplify::Simplifier::simplify(self, sub).unwrap_or(sub);
                    if matches!(
                        self.get(simp).kind,
                        ExprKind::Number(Number::Constant(
                            Constant::Infinity | Constant::NegInfinity | Constant::Undefined
                        ))
                    ) {
                        LimitEngine::limit_directed(self, f_antideriv, wrt, bound, dir)?
                    } else {
                        simp
                    }
                };
                if matches!(
                    self.get(raw).kind,
                    ExprKind::Number(Number::Constant(
                        Constant::Infinity | Constant::NegInfinity | Constant::Undefined
                    ))
                ) {
                    return Err(AlgebraError::EvaluationError(
                        "Definite integral diverges at integration boundary".into(),
                    ));
                }
                Ok(raw)
            };

        let lower_dir = if is_pos_inf_bound(lower) {
            LimitDirection::PositiveInfinity
        } else if is_neg_inf_bound(lower) {
            LimitDirection::NegativeInfinity
        } else {
            LimitDirection::FromRight
        };
        let upper_dir = if is_pos_inf_bound(upper) {
            LimitDirection::PositiveInfinity
        } else if is_neg_inf_bound(upper) {
            LimitDirection::NegativeInfinity
        } else {
            LimitDirection::FromLeft
        };

        let f_b = eval_bound(upper, upper_dir, singular_upper)?;
        let f_a = eval_bound(lower, lower_dir, singular_lower)?;
        let diff = self.sub(f_b, f_a);
        let res = crate::simplify::Simplifier::simplify(self, diff).unwrap_or(diff);

        // Guard against finite non-convergent values (e.g. NaN / inf after subtraction)
        if let Ok(val) = self.evalf(res, &ctx)
            && !val.to_f64().is_finite()
        {
            return Err(AlgebraError::EvaluationError(
                "Definite integral diverges at integration boundary".into(),
            ));
        }

        Ok(res)
    }

    fn summation(
        &self,
        expr: ExprId,
        wrt: SymbolId,
        lower: ExprId,
        upper: ExprId,
    ) -> AlgebraResult<ExprId> {
        let wrt_name = self.symbols.resolve(wrt).unwrap_or_default();
        let k = self.symbol(&wrt_name);

        if expr == k {
            // sum_{k=1}^n k = n*(n+1)/2
            let one = self.integer(1);
            let two = self.integer(2);
            let n_plus_1 = self.add([upper, one]);
            let num = self.mul([upper, n_plus_1]);
            let result = self.div(num, two);

            // If lower is not 1, subtract sum_{k=1}^{lower-1} k
            if lower == one {
                Ok(result)
            } else {
                let lower_minus_1 = self.sub(lower, one);
                let lower_term = self.add([lower_minus_1, one]);
                let lower_num = self.mul([lower_minus_1, lower_term]);
                let lower_sum = self.div(lower_num, two);
                Ok(self.sub(result, lower_sum))
            }
        } else if !self.has_symbol(expr, wrt) {
            // sum_{k=a}^b C = C * (b - a + 1)
            let one = self.integer(1);
            let diff = self.sub(upper, lower);
            let count = self.add([diff, one]);
            Ok(self.mul([expr, count]))
        } else if let Some(coeffs) = polynomial_coefficients(self, expr, wrt) {
            sum_polynomial(self, &coeffs, lower, upper)
        } else if let Some((poly, base)) = polynomial_times_geometric(self, expr, wrt) {
            sum_polynomial_geometric(self, &poly, &base, lower, upper)
        } else {
            Err(AlgebraError::EvaluationError(
                "summation: only polynomial summands, and polynomial times c^k with a rational \
                 constant c, are supported"
                    .into(),
            ))
        }
    }
}

/// The exact rational value of a constant expression built from integers, rationals, negation,
/// sums, differences, products and quotients; `None` if it contains anything else.
fn constant_rational(graph: &ExprGraph, id: ExprId) -> Option<num_rational::BigRational> {
    use num_bigint::BigInt;
    use num_rational::BigRational;
    use num_traits::Zero;
    match &graph.get(id).kind {
        ExprKind::Number(Number::Integer(i)) => Some(BigRational::from_integer(BigInt::from(*i))),
        ExprKind::Number(Number::Rational(n, d)) => {
            Some(BigRational::new(BigInt::from(*n), BigInt::from(*d)))
        }
        ExprKind::Neg(x) => constant_rational(graph, *x).map(|v| -v),
        ExprKind::Add(ts) => ts.iter().try_fold(BigRational::zero(), |acc, &t| {
            Some(acc + constant_rational(graph, t)?)
        }),
        ExprKind::Sub(l, r) => Some(constant_rational(graph, *l)? - constant_rational(graph, *r)?),
        ExprKind::Mul(fs) => fs
            .iter()
            .try_fold(BigRational::from_integer(1.into()), |acc, &f| {
                Some(acc * constant_rational(graph, f)?)
            }),
        ExprKind::Div(l, r) => {
            let d = constant_rational(graph, *r)?;
            (!d.is_zero()).then(|| constant_rational(graph, *l).map(|n| n / d))?
        }
        _ => None,
    }
}

/// A rational constant `c` and a polynomial `p(k)` with rational coefficients such that
/// `expr = p(k) * c^k`, found by structure: sums of terms that each have one factor `c^k` (a
/// numeric base with exponent `var`) and a polynomial in `var` for the rest, all with the same
/// base. `None` for anything else (including `c = 1`, a plain polynomial).
fn polynomial_times_geometric(
    graph: &ExprGraph,
    expr: ExprId,
    var: SymbolId,
) -> Option<(crate::gosper::QPoly, num_rational::BigRational)> {
    use crate::gosper::QPoly;
    use num_traits::One;
    // `c^k` with a constant base and the summation variable as the exponent.
    let geometric = |id: ExprId| -> Option<num_rational::BigRational> {
        let ExprKind::Pow(base, exponent) = graph.get(id).kind.clone() else {
            return None;
        };
        let ExprKind::Symbol(s) = graph.get(exponent).kind else {
            return None;
        };
        if s != var {
            return None;
        }
        constant_rational(graph, base)
    };
    // One term: exactly one `c^k` factor, the rest a polynomial with rational coefficients.
    let term = |id: ExprId| -> Option<(QPoly, num_rational::BigRational)> {
        let (base, rest): (Option<num_rational::BigRational>, Vec<ExprId>) =
            match &graph.get(id).kind {
                ExprKind::Mul(factors) => {
                    let mut base = None;
                    let mut rest = Vec::new();
                    for &f in factors.iter() {
                        match (geometric(f), base.is_some()) {
                            (Some(c), false) => base = Some(c),
                            (Some(_), true) => return None,
                            (None, _) => rest.push(f),
                        }
                    }
                    (base, rest)
                }
                ExprKind::Neg(inner) => {
                    let (p, c) = polynomial_times_geometric(graph, *inner, var)?;
                    return Some((p.scale(&-num_rational::BigRational::one()), c));
                }
                _ => (geometric(id), Vec::new()),
            };
        let base = base?;
        let rest_expr = if rest.is_empty() {
            graph.integer(1)
        } else {
            graph.mul(rest)
        };
        let coeffs = polynomial_coefficients(graph, rest_expr, var)?;
        let rational: Option<Vec<num_rational::BigRational>> = coeffs
            .iter()
            .map(|&c| constant_rational(graph, c))
            .collect();
        Some((QPoly::new(rational?), base))
    };
    let (poly, base) = match &graph.get(expr).kind {
        ExprKind::Add(terms) => {
            let mut parts = terms.iter().map(|&t| term(t));
            let (mut poly, base) = parts.next()??;
            for part in parts {
                let (p, c) = part?;
                if c != base {
                    return None;
                }
                poly = poly.add(&p);
            }
            (poly, base)
        }
        _ => term(expr)?,
    };
    (!base.is_one()).then_some((poly, base))
}

/// `sum_{k=lower}^{upper} p(k) c^k` by Gosper's algorithm: the antidifference is `q(k) c^k` with
/// `q` a polynomial, so the sum is `q(upper + 1) c^(upper + 1) - q(lower) c^lower`. The result is
/// checked against the defining identity `c q(k + 1) - q(k) = p(k)` before it is returned.
fn sum_polynomial_geometric(
    graph: &ExprGraph,
    poly: &crate::gosper::QPoly,
    base: &num_rational::BigRational,
    lower: ExprId,
    upper: ExprId,
) -> AlgebraResult<ExprId> {
    use crate::gosper::gosper;
    use num_traits::One;
    let fail = |msg: String| AlgebraError::EvaluationError(format!("summation: {msg}"));
    if poly.is_zero() {
        return Ok(graph.integer(0));
    }
    // t(k) = p(k) c^k has ratio c p(k+1) / p(k).
    let shifted = poly.shift(&num_rational::BigRational::one());
    let found = gosper(&shifted.scale(base), poly)
        .map_err(&fail)?
        .ok_or_else(|| {
            fail("this term has no closed-form antidifference (Gosper's algorithm)".into())
        })?;
    // z(k) = R(k) t(k) = (num / den) p(k) c^k, so q = num * p / den (exact for these terms).
    let (q, rem) = found
        .numerator
        .mul(poly)
        .div_rem(&found.denominator)
        .ok_or_else(|| fail("zero denominator in the antidifference".into()))?;
    if !rem.is_zero() {
        return Err(fail(
            "the antidifference is not a polynomial multiple of c^k".into(),
        ));
    }
    // Verify c q(k+1) - q(k) = p(k).
    let check = q
        .shift(&num_rational::BigRational::one())
        .scale(base)
        .sub(&q);
    if check != *poly {
        return Err(fail("internal check of the antidifference failed".into()));
    }

    // Build q(m) c^m as an expression for m = upper + 1 and m = lower.
    let c_expr = {
        let (n, d) = (base.numer(), base.denom());
        match (i64::try_from(n.clone()).ok(), i64::try_from(d.clone()).ok()) {
            (Some(n), Some(1)) => graph.integer(n),
            (Some(n), Some(d)) => graph.number(Number::Rational(n, d)),
            _ => return Err(fail("the base is too large".into())),
        }
    };
    let coefficient = |r: &num_rational::BigRational| -> AlgebraResult<ExprId> {
        let (n, d) = (
            i64::try_from(r.numer().clone()),
            i64::try_from(r.denom().clone()),
        );
        match (n, d) {
            (Ok(n), Ok(1)) => Ok(graph.integer(n)),
            (Ok(n), Ok(d)) => Ok(graph.number(Number::Rational(n, d))),
            _ => Err(fail("a coefficient is too large".into())),
        }
    };
    let at = |m: ExprId| -> AlgebraResult<ExprId> {
        // Horner evaluation of q at the expression m.
        let mut value = graph.integer(0);
        for i in (0..=q.degree().unwrap_or(0)).rev() {
            value = graph.add([graph.mul([value, m]), coefficient(&q.coeff(i))?]);
        }
        Ok(graph.mul([value, graph.pow(c_expr, m)]))
    };
    let one = graph.integer(1);
    let top = at(graph.add([upper, one]))?;
    let bottom = at(lower)?;
    Ok(graph.sub(top, bottom))
}

/// Largest power of the summation variable summed in closed form (Faulhaber coefficients stay in `i64`).
const MAX_FAULHABER_DEGREE: usize = 12;

/// Coefficients `[c_0, c_1, ...]` of `expr` as a polynomial in `var` whose coefficients are free of
/// `var`, or `None` if `expr` is not such a polynomial (a negative or symbolic power, a function of
/// `var`, a division by something containing `var`).
fn polynomial_coefficients(graph: &ExprGraph, expr: ExprId, var: SymbolId) -> Option<Vec<ExprId>> {
    fn add(graph: &ExprGraph, a: &[ExprId], b: &[ExprId]) -> Vec<ExprId> {
        (0..a.len().max(b.len()))
            .map(|i| match (a.get(i), b.get(i)) {
                (Some(&x), Some(&y)) => graph.add([x, y]),
                (Some(&x), None) | (None, Some(&x)) => x,
                (None, None) => graph.integer(0),
            })
            .collect()
    }
    fn mul(graph: &ExprGraph, a: &[ExprId], b: &[ExprId]) -> Vec<ExprId> {
        let mut out = vec![graph.integer(0); a.len() + b.len() - 1];
        for (i, &x) in a.iter().enumerate() {
            for (j, &y) in b.iter().enumerate() {
                out[i + j] = graph.add([out[i + j], graph.mul([x, y])]);
            }
        }
        out
    }
    if !graph.has_symbol(expr, var) {
        return Some(vec![expr]);
    }
    let node = graph.get(expr);
    match &node.kind {
        ExprKind::Symbol(_) => Some(vec![graph.integer(0), graph.integer(1)]),
        ExprKind::Neg(inner) => {
            let c = polynomial_coefficients(graph, *inner, var)?;
            Some(c.into_iter().map(|x| graph.neg(x)).collect())
        }
        ExprKind::Add(terms) => {
            let mut acc = vec![graph.integer(0)];
            for &term in terms.iter() {
                acc = add(graph, &acc, &polynomial_coefficients(graph, term, var)?);
            }
            Some(acc)
        }
        ExprKind::Sub(l, r) => {
            let a = polynomial_coefficients(graph, *l, var)?;
            let b: Vec<ExprId> = polynomial_coefficients(graph, *r, var)?
                .into_iter()
                .map(|x| graph.neg(x))
                .collect();
            Some(add(graph, &a, &b))
        }
        ExprKind::Mul(factors) => {
            let mut acc = vec![graph.integer(1)];
            for &factor in factors.iter() {
                acc = mul(graph, &acc, &polynomial_coefficients(graph, factor, var)?);
            }
            Some(acc)
        }
        ExprKind::Div(num, den) if !graph.has_symbol(*den, var) => {
            let c = polynomial_coefficients(graph, *num, var)?;
            Some(c.into_iter().map(|x| graph.div(x, *den)).collect())
        }
        ExprKind::Pow(base, exponent) => {
            let ExprKind::Number(Number::Integer(m)) = graph.get(*exponent).kind else {
                return None;
            };
            let m = usize::try_from(m)
                .ok()
                .filter(|&m| m <= MAX_FAULHABER_DEGREE)?;
            let base = polynomial_coefficients(graph, *base, var)?;
            let mut acc = vec![graph.integer(1)];
            for _ in 0..m {
                acc = mul(graph, &acc, &base);
            }
            Some(acc)
        }
        _ => None,
    }
}

/// Faulhaber: `sum_{k=1}^{n} k^m` as an expression in `n`, for `m <= MAX_FAULHABER_DEGREE`.
fn power_sum(graph: &ExprGraph, m: usize, n: ExprId) -> AlgebraResult<ExprId> {
    use num_bigint::BigInt;
    use num_rational::BigRational;
    use num_traits::{One, ToPrimitive, Zero};
    if m > MAX_FAULHABER_DEGREE {
        return Err(AlgebraError::EvaluationError(format!(
            "summation: powers above {MAX_FAULHABER_DEGREE} are not supported"
        )));
    }
    let binomial = |n: usize, r: usize| -> BigInt {
        let mut acc = BigInt::one();
        for i in 0..r {
            acc = acc * BigInt::from(n - i) / BigInt::from(i + 1);
        }
        acc
    };
    // Bernoulli numbers from sum_{i=0}^{j} C(j+1, i) B_i = 0 (B_1 = -1/2), then B_1 is flipped to
    // +1/2 for the convention in S_m(n) below.
    let mut bernoulli: Vec<BigRational> = Vec::with_capacity(m + 1);
    for j in 0..=m {
        if j == 0 {
            bernoulli.push(BigRational::one());
            continue;
        }
        let mut acc = BigRational::zero();
        for (i, b) in bernoulli.iter().enumerate() {
            acc += b * BigRational::from(binomial(j + 1, i));
        }
        bernoulli.push(-acc / BigRational::from(BigInt::from(j + 1)));
    }
    if m >= 1 {
        bernoulli[1] = BigRational::new(BigInt::one(), BigInt::from(2));
    }
    // S_m(n) = 1/(m+1) * sum_{j=0}^{m} C(m+1, j) B_j n^(m+1-j)
    let overflow = || AlgebraError::EvaluationError("summation: coefficient overflow".into());
    let mut terms = Vec::new();
    for (j, b) in bernoulli.iter().enumerate() {
        let coefficient =
            b * BigRational::from(binomial(m + 1, j)) / BigRational::from(BigInt::from(m + 1));
        if coefficient.is_zero() {
            continue;
        }
        let num = coefficient.numer().to_i64().ok_or_else(overflow)?;
        let den = coefficient.denom().to_i64().ok_or_else(overflow)?;
        let power = graph.pow(n, graph.integer((m + 1 - j) as i64));
        terms.push(graph.mul([graph.rational(num, den), power]));
    }
    Ok(graph.add(terms))
}

/// `sum_{k=lower}^{upper} P(k)` for a polynomial with the given coefficients.
fn sum_polynomial(
    graph: &ExprGraph,
    coeffs: &[ExprId],
    lower: ExprId,
    upper: ExprId,
) -> AlgebraResult<ExprId> {
    let below = graph.sub(lower, graph.integer(1));
    let mut total = Vec::new();
    for (m, &c) in coeffs.iter().enumerate() {
        let span = graph.sub(power_sum(graph, m, upper)?, power_sum(graph, m, below)?);
        total.push(graph.mul([c, span]));
    }
    let sum = graph.add(total);
    Ok(Simplifier::simplify(graph, sum).unwrap_or(sum))
}

/// Fractional Calculus for non-integer order derivatives $D^\alpha f(x)$.
pub struct FractionalCalculus;

impl FractionalCalculus {
    /// Compute Caputo fractional derivative $D^\alpha f(x)$ for non-integer order $\alpha > 0$.
    pub fn caputo_derivative(
        graph: &ExprGraph,
        expr: ExprId,
        var: SymbolId,
        alpha: f64,
    ) -> AlgebraResult<ExprId> {
        let var_name = graph.symbols.resolve(var).unwrap_or_default();
        let n = alpha.ceil() as i64;
        let gamma_part = graph.function("Gamma", [graph.float((n as f64) - alpha)]);

        let mut d_expr = expr;
        for _ in 0..n {
            d_expr = graph.diff(d_expr, var);
        }

        let tau_str = format!("{}_tau", var_name);
        let tau_sym = graph.symbol(&tau_str);
        let var_sym = graph.symbol(&var_name);

        let sub_d = graph.substitute(d_expr, var, tau_sym);
        let t_minus_tau = graph.sub(var_sym, tau_sym);
        // Caputo kernel: (t - tau)^(n - alpha - 1) in the numerator, i.e. a denominator power
        // of alpha + 1 - n.
        let exp = graph.float(alpha + 1.0 - (n as f64));
        let pow_term = graph.pow(t_minus_tau, exp);

        let integrand = graph.div(sub_d, pow_term);
        let int_term = graph.integral(integrand, &tau_str, Some(graph.integer(0)), Some(var_sym));

        Ok(graph.div(int_term, gamma_part))
    }
}

/// Variational Calculus for functional variations $\delta J = 0$ and Euler-Lagrange equations.
pub struct VariationalCalculus;

impl VariationalCalculus {
    /// Compute Euler-Lagrange equation $\frac{d}{dt}\frac{\partial L}{\partial \dot{q}} - \frac{\partial L}{\partial q} = 0$.
    pub fn euler_lagrange(
        graph: &ExprGraph,
        lagrangian: ExprId,
        q: SymbolId,
        q_dot: SymbolId,
        t: SymbolId,
    ) -> ExprId {
        let dl_dq = graph.diff(lagrangian, q);
        let dl_dqdot = graph.diff(lagrangian, q_dot);
        let d_dt_dl_dqdot = graph.diff(dl_dqdot, t);

        graph.sub(d_dt_dl_dqdot, dl_dq)
    }
}

fn collect_denominators(graph: &ExprGraph, expr: ExprId, wrt: SymbolId, out: &mut Vec<ExprId>) {
    match &graph.get(expr).kind {
        ExprKind::Div(n, d) => {
            if graph.has_symbol(*d, wrt) {
                out.push(*d);
            }
            collect_denominators(graph, *n, wrt, out);
            collect_denominators(graph, *d, wrt, out);
        }
        ExprKind::Pow(b, e) => {
            if graph.has_symbol(*b, wrt)
                && (matches!(
                    graph.get(*e).kind,
                    ExprKind::Number(algebra_core::Number::Integer(k)) if k < 0
                ) || matches!(graph.get(*e).kind, ExprKind::Neg(_)))
            {
                out.push(*b);
            }
            collect_denominators(graph, *b, wrt, out);
        }
        ExprKind::Add(xs) | ExprKind::Mul(xs) => {
            for &x in xs {
                collect_denominators(graph, x, wrt, out);
            }
        }
        ExprKind::Sub(a, b) => {
            collect_denominators(graph, *a, wrt, out);
            collect_denominators(graph, *b, wrt, out);
        }
        ExprKind::Neg(a) => collect_denominators(graph, *a, wrt, out),
        ExprKind::Function { args, .. } => {
            for &a in args {
                collect_denominators(graph, a, wrt, out);
            }
        }
        _ => {}
    }
}

/// Exact rational value of a numeric literal expression, if it is one.
pub(crate) fn exact_rational(graph: &ExprGraph, e: ExprId) -> Option<num_rational::BigRational> {
    use num_bigint::BigInt;
    use num_rational::BigRational;
    match &graph.get(e).kind {
        ExprKind::Number(Number::Integer(i)) => Some(BigRational::from(BigInt::from(*i))),
        ExprKind::Number(Number::BigInteger(b)) => Some(BigRational::from(b.clone())),
        ExprKind::Number(Number::Rational(n, d)) => {
            Some(BigRational::new(BigInt::from(*n), BigInt::from(*d)))
        }
        ExprKind::Number(Number::BigRational(r)) => Some((**r).clone()),
        ExprKind::Neg(inner) => exact_rational(graph, *inner).map(|r| -r),
        _ => None,
    }
}

pub(crate) fn rational_expr(
    graph: &ExprGraph,
    r: &num_rational::BigRational,
) -> AlgebraResult<ExprId> {
    use num_traits::ToPrimitive;
    match (r.numer().to_i64(), r.denom().to_i64()) {
        (Some(n), Some(d)) => Ok(graph.rational(n, d)),
        _ => Err(AlgebraError::EvaluationError(
            "pade: coefficient too large".into(),
        )),
    }
}

/// Padé approximant `[l/m]` of `expr` about `point`: the rational function
/// `P(x - point) / Q(x - point)` with `deg P <= l`, `deg Q <= m`, `Q(0) = 1`, whose Taylor series
/// agrees with that of `expr` through order `l + m`.
///
/// The Taylor coefficients must be exact rationals. Returns an error when they are not, or when
/// the approximant does not exist (a singular coefficient system).
pub fn pade_approximant(
    graph: &ExprGraph,
    expr: ExprId,
    var: SymbolId,
    point: ExprId,
    l: usize,
    m: usize,
) -> AlgebraResult<ExprId> {
    use num_rational::BigRational;
    use num_traits::{One, Zero};
    let fail = |why: &str| AlgebraError::EvaluationError(format!("pade: {why}"));
    let raw = taylor_coeffs_vec(graph, expr, var, point, l + m);
    let c: Vec<BigRational> = raw
        .iter()
        .map(|&e| exact_rational(graph, e))
        .collect::<Option<_>>()
        .ok_or_else(|| fail("the Taylor coefficients are not all exact rationals"))?;
    let coeff = |i: isize| -> BigRational {
        usize::try_from(i)
            .ok()
            .and_then(|i| c.get(i).cloned())
            .unwrap_or_else(BigRational::zero)
    };

    // Denominator q_1..q_m from sum_{j=0}^{m} q_j c_{k-j} = 0 for k = l+1..=l+m (q_0 = 1).
    let mut q = vec![BigRational::one()];
    if m > 0 {
        let mut a: Vec<Vec<BigRational>> = (0..m)
            .map(|row| {
                let k = (l + 1 + row) as isize;
                let mut r: Vec<BigRational> = (1..=m as isize).map(|j| coeff(k - j)).collect();
                r.push(-coeff(k));
                r
            })
            .collect();
        // Gauss-Jordan elimination. A rank-deficient system still has solutions (the approximant
        // is then not unique but its value is): free unknowns are set to zero.
        let mut pivot_row = 0;
        let mut pivot_cols = Vec::new();
        for col in 0..m {
            let Some(found) = (pivot_row..m).find(|&r| !a[r][col].is_zero()) else {
                continue;
            };
            a.swap(pivot_row, found);
            let lead = a[pivot_row][col].clone();
            for v in a[pivot_row].iter_mut() {
                *v = v.clone() / lead.clone();
            }
            for r in 0..m {
                if r != pivot_row && !a[r][col].is_zero() {
                    let factor = a[r][col].clone();
                    let reference = a[pivot_row].clone();
                    for (v, pv) in a[r].iter_mut().zip(reference) {
                        *v = v.clone() - factor.clone() * pv;
                    }
                }
            }
            pivot_cols.push(col);
            pivot_row += 1;
        }
        if a[pivot_row..].iter().any(|row| !row[m].is_zero()) {
            return Err(fail("the approximant does not exist (inconsistent system)"));
        }
        let mut unknowns = vec![BigRational::zero(); m];
        for (row, &col) in pivot_cols.iter().enumerate() {
            unknowns[col] = a[row][m].clone();
        }
        q.extend(unknowns);
    }
    // Numerator p_i = sum_{j=0}^{min(i, m)} q_j c_{i-j}.
    let p: Vec<BigRational> = (0..=l)
        .map(|i| {
            (0..=i.min(m))
                .map(|j| q[j].clone() * coeff((i - j) as isize))
                .fold(BigRational::zero(), |acc, v| acc + v)
        })
        .collect();

    let wrt_name = graph.symbols.resolve(var).unwrap_or_default();
    let shifted = if point == graph.integer(0) {
        graph.symbol(&wrt_name)
    } else {
        graph.sub(graph.symbol(&wrt_name), point)
    };
    let polynomial = |coeffs: &[BigRational]| -> AlgebraResult<ExprId> {
        let mut terms = Vec::new();
        for (i, r) in coeffs.iter().enumerate() {
            if r.is_zero() {
                continue;
            }
            let value = rational_expr(graph, r)?;
            terms.push(if i == 0 {
                value
            } else {
                graph.mul([value, graph.pow(shifted, graph.integer(i as i64))])
            });
        }
        Ok(if terms.is_empty() {
            graph.integer(0)
        } else {
            graph.add(terms)
        })
    };
    let numerator = polynomial(&p)?;
    let denominator = polynomial(&q)?;
    let result = graph.div(numerator, denominator);
    Ok(Simplifier::simplify(graph, result).unwrap_or(result))
}

pub(crate) fn taylor_coeffs_vec(
    graph: &ExprGraph,
    expr: ExprId,
    wrt: SymbolId,
    point: ExprId,
    order: usize,
) -> Vec<ExprId> {
    // Fast path: exact series arithmetic, when the expression and the point allow it.
    if let Some(fast) = crate::series_fast::taylor_coeffs_fast(graph, expr, wrt, point, order) {
        return fast;
    }
    let mut coeffs = Vec::with_capacity(order + 1);
    let mut current_deriv = expr;
    let mut factorial = num_bigint::BigInt::from(1u8);
    let zero = graph.integer(0);

    // Time budget. Each order costs several times the one before when the derivatives swell, so an order that
    // took long predicts that the next one will take much longer: stop instead of running for minutes. (Debug
    // builds are about ten times slower.)
    let slowdown = if cfg!(debug_assertions) { 10.0 } else { 1.0 };
    let series_started = std::time::Instant::now();
    let mut last_order_started = series_started;
    for n in 0..=order {
        let now = std::time::Instant::now();
        let last_took = now.duration_since(last_order_started).as_secs_f64();
        last_order_started = now;
        if n > 1
            && (last_took > 0.8 * slowdown
                || series_started.elapsed().as_secs_f64() > 4.0 * slowdown)
        {
            SERIES_BUDGET_EXCEEDED.with(|f| f.set(true));
            while coeffs.len() <= order {
                coeffs.push(zero);
            }
            return coeffs;
        }
        if n > 0 {
            factorial *= n;
        }
        let sub = graph.substitute(current_deriv, wrt, point);
        let c_sub = crate::limits::LimitEngine::simplify_constants(graph, sub);
        let mut eval_deriv = crate::simplify::Simplifier::simplify(graph, c_sub).unwrap_or(c_sub);
        if crate::limits::LimitEngine::is_indeterminate(graph, eval_deriv)
            && let ExprKind::Div(num, den) = graph.get(current_deriv).kind
            && let Ok(Some(lim)) =
                crate::limits::LimitEngine::eval_lhopital(graph, num, den, wrt, point, 6)
        {
            eval_deriv = crate::simplify::Simplifier::simplify(graph, lim).unwrap_or(lim);
        }
        // A singular value (`0^0`, `ln(0)`, a division by zero) means the function is not analytic at the point:
        // there is no Taylor series, and carrying on would print nonsense.
        if has_singular_value(graph, eval_deriv) {
            SERIES_NOT_ANALYTIC.with(|f| f.set(true));
            while coeffs.len() <= order {
                coeffs.push(zero);
            }
            return coeffs;
        }
        if eval_deriv == zero {
            coeffs.push(zero);
        } else if n <= 1 {
            coeffs.push(eval_deriv);
        } else {
            let fact_expr = if let Ok(v) = i64::try_from(&factorial) {
                graph.integer(v)
            } else {
                graph.intern(algebra_core::ExprNode::new(
                    algebra_core::Domain::Integers,
                    ExprKind::Number(algebra_core::Number::BigInteger(factorial.clone())),
                ))
            };
            let raw = graph.div(eval_deriv, fact_expr);
            coeffs.push(crate::simplify::Simplifier::simplify(graph, raw).unwrap_or(raw));
        }
        if n < order {
            current_deriv = graph.diff(current_deriv, wrt);
            // Expression swell: derivatives of `x^x`, nested compositions, ... grow exponentially, and the
            // method then takes minutes. Give up, flag it for the caller, and pad with zeros.
            if distinct_nodes_exceed(graph, current_deriv, SERIES_NODE_BUDGET) {
                SERIES_BUDGET_EXCEEDED.with(|f| f.set(true));
                while coeffs.len() <= order {
                    coeffs.push(zero);
                }
                return coeffs;
            }
        }
    }
    coeffs
}

/// The largest derivative expression (in distinct nodes) the differentiation method will work with.
const SERIES_NODE_BUDGET: usize = 30_000;

thread_local! {
    /// Set by [`taylor_coeffs_vec`] when a coefficient was singular (`0^0`, `ln(0)`, `1/0`).
    pub(crate) static SERIES_NOT_ANALYTIC: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };

    /// Set by [`taylor_coeffs_vec`] when it gave up because of expression swell, so a caller that can report an
    /// error (the series command) does so instead of returning a truncated series.
    pub(crate) static SERIES_BUDGET_EXCEEDED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Whether the value contains `0^0` (or another power of zero with a non-positive exponent), `ln(0)` or a division by
/// the literal zero.
fn has_singular_value(graph: &ExprGraph, root: ExprId) -> bool {
    let zero = graph.integer(0);
    let mut seen = std::collections::HashSet::new();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        let node = graph.get(id);
        match &node.kind {
            ExprKind::Div(_, d) if *d == zero => return true,
            ExprKind::Pow(base, exp) if *base == zero => {
                let non_positive = match &graph.get(*exp).kind {
                    ExprKind::Number(n) => n.as_f64().is_some_and(|v| v <= 0.0),
                    _ => false,
                };
                if non_positive {
                    return true;
                }
            }
            ExprKind::Function { name, args }
                if args.len() == 1
                    && args[0] == zero
                    && matches!(graph.symbols.resolve(*name).as_deref(), Some("ln" | "log")) =>
            {
                return true;
            }
            _ => {}
        }
        stack.extend(node.children());
    }
    false
}

/// Whether the expression has more than `limit` distinct nodes (counting stops at the limit).
fn distinct_nodes_exceed(graph: &ExprGraph, root: ExprId, limit: usize) -> bool {
    let mut seen = std::collections::HashSet::new();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        if seen.len() > limit {
            return true;
        }
        stack.extend(graph.get(id).children());
    }
    false
}

fn taylor_div_coeffs(
    graph: &ExprGraph,
    num: ExprId,
    den: ExprId,
    wrt: SymbolId,
    point: ExprId,
    order: usize,
) -> Option<Vec<ExprId>> {
    let zero = graph.integer(0);
    let max_probe = order + 6;
    let den_c = taylor_coeffs_vec(graph, den, wrt, point, max_probe);
    let shift = den_c.iter().position(|&c| c != zero)?;
    if shift == 0 || shift > 6 {
        return None;
    }
    let num_c = taylor_coeffs_vec(graph, num, wrt, point, order + shift);
    if num_c.iter().take(shift).any(|&c| c != zero) {
        return None;
    }

    let n_slice = &num_c[shift..];
    let d_slice = &den_c[shift..];
    let d0 = *d_slice.first()?;
    if d0 == zero {
        return None;
    }

    let mut q = Vec::with_capacity(order + 1);
    for m in 0..=order {
        let mut rhs = *n_slice.get(m).unwrap_or(&zero);
        for j in 1..=m.min(d_slice.len().saturating_sub(1)) {
            if d_slice[j] != zero && q[m - j] != zero {
                let prod = graph.mul([d_slice[j], q[m - j]]);
                rhs = graph.sub(rhs, prod);
            }
        }
        let qm_raw = if d0 == graph.integer(1) {
            rhs
        } else {
            graph.div(rhs, d0)
        };
        let qm = crate::simplify::Simplifier::simplify(graph, qm_raw).unwrap_or(qm_raw);
        q.push(qm);
    }
    Some(q)
}

fn build_taylor_poly(graph: &ExprGraph, coeffs: &[ExprId], x_minus_a: ExprId) -> ExprId {
    let zero = graph.integer(0);
    let mut terms = Vec::new();
    for (n, &c) in coeffs.iter().enumerate() {
        if c == zero {
            continue;
        }
        let term = if n == 0 {
            c
        } else if n == 1 {
            graph.mul([c, x_minus_a])
        } else {
            let pow_term = graph.pow(x_minus_a, graph.integer(n as i64));
            graph.mul([c, pow_term])
        };
        let simp = crate::simplify::Simplifier::simplify(graph, term).unwrap_or(term);
        if simp != zero {
            terms.push(simp);
        }
    }
    if terms.is_empty() {
        zero
    } else {
        graph.add(terms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use algebra_core::ExprParser;

    #[test]
    fn test_integrate_with_steps_polynomial_and_elementary() {
        let graph = ExprGraph::new();
        let x = graph.symbols.get_or_intern("x");
        let parser = ExprParser::new(&graph);

        let poly = parser.parse("3*x^2 + cos(x)").expect("parse integrand");
        let stepped = graph
            .integrate_with_steps(poly, x)
            .expect("integrate_with_steps");
        assert_eq!(stepped.steps.len(), 3);
        assert_eq!(stepped.steps[0].rule_name, "Sum Linearity Term");
        assert_eq!(stepped.steps[2].rule_name, "Linear Combination");

        let direct = graph.integrate(poly, x).expect("direct integrate");
        assert_eq!(stepped.result, direct);
    }
}
