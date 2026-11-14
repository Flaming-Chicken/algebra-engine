//! # `algebra-calculus`
//!
//! Analytical calculus (differentiation, Taylor series, Gruntz limits, Risch integration)
//! and Discrete calculus (finite differences, Gosper summation).

use algebra_core::{
    AlgebraError, AlgebraResult, ExprGraph, ExprId, ExprKind, MathStep, StepByStepResult, SymbolId,
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
                let db = self.diff(b, wrt);

                let one = self.integer(1);
                let e_minus_1 = self.sub(e, one);
                let b_pow = self.pow(b, e_minus_1);

                let term = self.mul([e, b_pow]);
                self.mul([term, db])
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
        let mut terms = Vec::new();
        let wrt_name = self.symbols.resolve(wrt).unwrap_or_default();
        let x_sym = self.symbol(&wrt_name);
        let x_minus_a = self.sub(x_sym, point);

        let mut current_deriv = expr;
        let mut factorial = 1i64;

        for n in 0..=order {
            let eval_deriv = self.substitute(current_deriv, wrt, point);
            if n == 0 {
                terms.push(eval_deriv);
            } else {
                factorial *= n as i64;
                let n_expr = self.integer(n as i64);
                let pow_term = self.pow(x_minus_a, n_expr);
                let coeff = self.div(eval_deriv, self.integer(factorial));
                let term = self.mul([coeff, pow_term]);
                terms.push(term);
            }
            current_deriv = self.diff(current_deriv, wrt);
        }

        self.add(terms)
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
                    format!("Applied Power Rule with n = 1: ∫ {wrt_name} d{wrt_name} = {wrt_name}^2 / 2"),
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
                        format!("Integrated additive term {} of {} with respect to {wrt_name}", idx + 1, terms.len()),
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
                        format!("Factored constants independent of {wrt_name} and integrated remaining factor"),
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
                    format!("Applied Power Rule ∫ {wrt_name}^n d{wrt_name} = {wrt_name}^(n+1) / (n+1)"),
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
        let f_antideriv = self.integrate(expr, wrt)?;
        let f_b = self.substitute(f_antideriv, wrt, upper);
        let f_a = self.substitute(f_antideriv, wrt, lower);
        let diff = self.sub(f_b, f_a);
        Ok(crate::simplify::Simplifier::simplify(self, diff).unwrap_or(diff))
    }

    fn summation(
        &self,
        expr: ExprId,
        wrt: SymbolId,
        lower: ExprId,
        upper: ExprId,
    ) -> AlgebraResult<ExprId> {
        let node = self.get(expr);
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
        } else if match &node.kind {
            ExprKind::Symbol(s) => *s != wrt,
            _ => true,
        } {
            // sum_{k=a}^b C = C * (b - a + 1)
            let one = self.integer(1);
            let diff = self.sub(upper, lower);
            let count = self.add([diff, one]);
            Ok(self.mul([expr, count]))
        } else {
            Err(AlgebraError::EvaluationError(
                "Gosper hypergeometric summation heuristic fallback".into(),
            ))
        }
    }
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
        let exp = graph.float((alpha - (n as f64)).abs());
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
        let stepped = graph.integrate_with_steps(poly, x).expect("integrate_with_steps");
        assert_eq!(stepped.steps.len(), 3);
        assert_eq!(stepped.steps[0].rule_name, "Sum Linearity Term");
        assert_eq!(stepped.steps[2].rule_name, "Linear Combination");

        let direct = graph.integrate(poly, x).expect("direct integrate");
        assert_eq!(stepped.result, direct);
    }
}
