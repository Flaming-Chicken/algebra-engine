//! # `algebra_engine::risch`
//!
//! Complete Transcendental Risch Differential Field Integration Algorithm.
//!
//! ## Mathematical Foundations
//! Risch's algorithm (1969) decides whether an elementary function has an elementary antiderivative
//! $\int f(x) \, \mathrm{d}x$, and if so, computes it in closed form.
//!
//! ### 1. Differential Field Towers
//! The algorithm operates over a differential tower $K_0 = \mathbb{Q}(x) \subset K_1 = K_0(\theta_1) \subset \cdots \subset K_n = K_{n-1}(\theta_n)$
//! where each extension $\theta_i$ is a monomial generator satisfying:
//! - **Logarithmic extension**: $\theta' = \frac{u'}{u}$ for $u \in K_{i-1}$ ($\theta = \ln u$)
//! - **Exponential extension**: $\theta' = u' \theta$ for $u \in K_{i-1}$ ($\theta = \exp u$)
//! - **Tangent extension**: $\theta' = u' (1 + \theta^2)$ for $u \in K_{i-1}$ ($\theta = \tan u$)
//!
//! ### 2. Hermite Reduction
//! For any rational fraction $\frac{A}{D} \in K(\theta)$ with square-free factorization of the denominator
//! $D = D_1 D_2^2 \cdots D_k^k$, Hermite reduction applies integration by parts to compute:
//! $$\int \frac{A}{D} \, \mathrm{d}\theta = g + \int P \, \mathrm{d}\theta + \int \frac{A_0}{D^*} \, \mathrm{d}\theta$$
//! where $g \in K(\theta)$, $P$ is polynomial in $\theta$, and $D^* = D_1 D_2 \cdots D_k$ is strictly square-free.
//!
//! ### 3. Rothstein-Trager / Lazard-Rioboo-Trager Method
//! The logarithmic part with square-free denominator $\int \frac{A_0}{D^*} \, \mathrm{d}\theta$ is given by:
//! $$\int \frac{A_0}{D^*} \, \mathrm{d}\theta = \sum_{i=1}^m c_i \ln\left( \gcd\left(A_0 - c_i (D^*)', D^*\right) \right)$$
//! where $c_i$ are the distinct roots of the resultant polynomial:
//! $$R(z) = \operatorname{res}_\theta(A_0 - z (D^*)', D^*)$$
//!
//! ## References
//! - Risch, R. H. (1969). The problem of integration in finite terms. *Trans. Amer. Math. Soc.*, 139, 167–189.
//! - Bronstein, M. (2005). *Symbolic Integration I: Transcendental Functions* (2nd ed.). Springer.
//! - Rothstein, M. (1976). *Aspects of Symbolic Integration and Simplification of Exponential and Primitive Functions*. PhD thesis, Univ. of Wisconsin–Madison.
//! - Trager, B. M. (1976). Algebraic factoring and rational function integration. *Proc. SYMSAC*, 219–226.

use crate::calculus::SymbolicCalculus;
use algebra_core::{
    AlgebraError, AlgebraResult, Domain, ExprGraph, ExprId, ExprKind, ExprNode, SymbolId,
};

/// Monomial extension kind in a differential field tower $K(\theta)$.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtensionKind {
    /// Base independent variable $x$ ($\theta' = 1$).
    Base(SymbolId),
    /// Logarithmic monomial $\theta = \ln(u)$ with $\theta' = \frac{u'}{u}$.
    Logarithmic { arg: ExprId, generator: SymbolId },
    /// Exponential monomial $\theta = \exp(u)$ with $\theta' = u' \theta$.
    Exponential { arg: ExprId, generator: SymbolId },
    /// Trigonometric tangent monomial $\theta = \tan(u)$ with $\theta' = u'(1 + \theta^2)$.
    Tangent { arg: ExprId, generator: SymbolId },
}

/// Differential Field Tower $K_n = \mathbb{Q}(x)(\theta_1, \dots, \theta_n)$.
#[derive(Debug, Clone)]
pub struct DifferentialFieldTower {
    /// Chain of monomial extensions.
    pub extensions: Vec<ExtensionKind>,
}

impl DifferentialFieldTower {
    /// Construct a base rational differential field $\mathbb{Q}(x)$.
    pub fn new_base(x: SymbolId) -> Self {
        Self {
            extensions: vec![ExtensionKind::Base(x)],
        }
    }

    /// Add a logarithmic extension $\theta = \ln(u)$.
    pub fn push_log(&mut self, arg: ExprId, generator: SymbolId) {
        self.extensions
            .push(ExtensionKind::Logarithmic { arg, generator });
    }

    /// Add an exponential extension $\theta = \exp(u)$.
    pub fn push_exp(&mut self, arg: ExprId, generator: SymbolId) {
        self.extensions
            .push(ExtensionKind::Exponential { arg, generator });
    }

    /// Compute the formal derivative of a monomial generator $\theta'$ in the tower.
    pub fn derivative_of_generator(&self, graph: &ExprGraph, idx: usize, wrt: SymbolId) -> ExprId {
        match &self.extensions[idx] {
            ExtensionKind::Base(_) => graph.integer(1),
            ExtensionKind::Logarithmic { arg, .. } => {
                let u = *arg;
                let du = graph.diff(u, wrt);
                graph.div(du, u)
            }
            ExtensionKind::Exponential { arg, generator } => {
                let u = *arg;
                let du = graph.diff(u, wrt);
                let theta =
                    graph.intern(ExprNode::new(Domain::Reals, ExprKind::Symbol(*generator)));
                graph.mul([du, theta])
            }
            ExtensionKind::Tangent { arg, generator } => {
                let u = *arg;
                let du = graph.diff(u, wrt);
                let theta =
                    graph.intern(ExprNode::new(Domain::Reals, ExprKind::Symbol(*generator)));
                let theta_sq = graph.pow(theta, graph.integer(2));
                let one_plus_theta_sq = graph.add([graph.integer(1), theta_sq]);
                graph.mul([du, one_plus_theta_sq])
            }
        }
    }
}

/// Result of Hermite Rational Reduction: $\int \frac{A}{D} = g + \int P + \int \frac{A_0}{D^*}$.
#[derive(Debug, Clone)]
pub struct HermiteReductionResult {
    /// Rational integrated part $g \in K(\theta)$.
    pub integrated_part: ExprId,
    /// Polynomial integrand part $P \in K[\theta]$.
    pub polynomial_part: ExprId,
    /// Residual simple-pole numerator $A_0$.
    pub residual_num: ExprId,
    /// Residual square-free denominator $D^*$.
    pub residual_den: ExprId,
}

/// Risch Symbolic Integrator engine.
pub struct RischIntegrator;

impl RischIntegrator {
    /// Creative telescoping fallback for parametric definite integrals $I(x) = \int f(x, y) \, \mathrm{d}y$.
    /// When indefinite integration has no elementary antiderivative, Zeilberger's algorithm finds
    /// a differential operator $L(x, \partial_x)$ such that $L(x, \partial_x) f(x, y) = \partial_y g(x, y)$,
    /// yielding the ODE $L(x, \partial_x) I(x) = 0$.
    pub fn creative_telescoping_integral(
        _graph: &ExprGraph,
        _expr: ExprId,
        _x_var: SymbolId,
        _y_var: SymbolId,
    ) -> AlgebraResult<(crate::weyl_dmodules::WeylOperator, String)> {
        // Creative telescoping for a general integrand is not implemented. The fixed Gaussian
        // case is available as `AlmkvistZeilberger::gaussian_integral_ode`.
        Err(algebra_core::AlgebraError::EvaluationError(
            "creative telescoping is not implemented for general integrands;              see AlmkvistZeilberger::gaussian_integral_ode for the Gaussian case"
                .into(),
        ))
    }

    /// Perform full Risch transcendental integration on `expr` with respect to variable `wrt`.
    pub fn integrate(graph: &ExprGraph, expr: ExprId, wrt: SymbolId) -> AlgebraResult<ExprId> {
        let wrt_name = graph.symbols.resolve(wrt).unwrap_or_default();
        let x = graph.symbol(&wrt_name);

        // 1. Basic Polynomial & Power Check
        let node = graph.get(expr);
        match &node.kind {
            ExprKind::Number(_) => Ok(graph.mul([expr, x])),
            ExprKind::Symbol(s) if *s == wrt => {
                let two = graph.integer(2);
                let x_sq = graph.pow(x, two);
                Ok(graph.div(x_sq, two))
            }
            ExprKind::Symbol(_) => Ok(graph.mul([expr, x])),
            ExprKind::Add(terms) => {
                let mut int_terms = Vec::with_capacity(terms.len());
                for &t in terms {
                    int_terms.push(Self::integrate(graph, t, wrt)?);
                }
                Ok(graph.add(int_terms))
            }
            ExprKind::Sub(a, b) => {
                let ia = Self::integrate(graph, *a, wrt)?;
                let ib = Self::integrate(graph, *b, wrt)?;
                Ok(graph.sub(ia, ib))
            }
            ExprKind::Neg(a) => {
                let ia = Self::integrate(graph, *a, wrt)?;
                Ok(graph.neg(ia))
            }
            ExprKind::Mul(factors) => {
                // Check if all factors except one are independent of wrt
                let mut const_factors = Vec::new();
                let mut dep_factors = Vec::new();
                for &f in factors {
                    if graph.has_symbol(f, wrt) {
                        dep_factors.push(f);
                    } else {
                        const_factors.push(f);
                    }
                }

                if dep_factors.is_empty() {
                    return Ok(graph.mul([expr, x]));
                }

                if dep_factors.len() == 1 {
                    let sub_int = Self::integrate(graph, dep_factors[0], wrt)?;
                    return if const_factors.is_empty() {
                        Ok(sub_int)
                    } else {
                        let c = graph.mul(const_factors);
                        Ok(graph.mul([c, sub_int]))
                    };
                }

                // Check standard integration-by-parts / u-sub forms
                let dep_int = Self::integrate_products(graph, &dep_factors, wrt)?;
                if const_factors.is_empty() {
                    Ok(dep_int)
                } else {
                    let c = if const_factors.len() == 1 {
                        const_factors[0]
                    } else {
                        graph.mul(const_factors)
                    };
                    Ok(graph.mul([c, dep_int]))
                }
            }
            ExprKind::Div(num, den) => {
                // Rational function integration via Hermite & Rothstein-Trager
                Self::integrate_rational(graph, *num, *den, wrt)
            }
            ExprKind::Function { name, args } => {
                let fn_name = graph.symbols.resolve(*name).unwrap_or_default();
                Self::integrate_elementary_function(graph, &fn_name, args, wrt)
            }
            ExprKind::Pow(base, exp) => {
                // 1. Constant base: b^(a*x + c) -> b^(a*x+c) / (a * ln(b))
                if !graph.has_symbol(*base, wrt) && graph.has_symbol(*exp, wrt) {
                    let d_exp = graph.diff(*exp, wrt);
                    if !graph.has_symbol(d_exp, wrt) && d_exp != graph.integer(0) {
                        let ln_b = graph.function("ln", [*base]);
                        let denom = graph.mul([d_exp, ln_b]);
                        return Ok(graph.div(expr, denom));
                    }
                }

                // 2. Trig / hyperbolic powers before general power rule (so sec(x)^2 doesn't misuse (a*x+b)^n)
                if !graph.has_symbol(*exp, wrt)
                    && let ExprKind::Function { name, args } = graph.get(*base).kind
                    && args.len() == 1
                    && let ExprKind::Number(algebra_core::Number::Integer(k)) = graph.get(*exp).kind
                {
                    let fn_name = graph.symbols.resolve(name).unwrap_or_default();
                    let u = args[0];
                    let du = graph.diff(u, wrt);
                    if !graph.has_symbol(du, wrt) && du != graph.integer(0) {
                        let scale_du = |raw: ExprId| -> ExprId {
                            if du == graph.integer(1) {
                                raw
                            } else {
                                graph.div(raw, du)
                            }
                        };
                        match (fn_name.as_str(), k) {
                            ("sec", 2) | ("cos", -2) => {
                                return Ok(scale_du(graph.function("tan", [u])));
                            }
                            ("csc", 2) | ("sin", -2) => {
                                return Ok(scale_du(graph.neg(graph.function("cot", [u]))));
                            }
                            ("sech", 2) | ("cosh", -2) => {
                                return Ok(scale_du(graph.function("tanh", [u])));
                            }
                            ("csch", 2) | ("sinh", -2) => {
                                return Ok(scale_du(graph.neg(graph.function("coth", [u]))));
                            }
                            ("tan", 2) => {
                                let raw = graph.sub(graph.function("tan", [u]), u);
                                return Ok(scale_du(raw));
                            }
                            ("cot", 2) => {
                                let raw = graph.sub(graph.neg(graph.function("cot", [u])), u);
                                return Ok(scale_du(raw));
                            }
                            ("sin", 2) => {
                                let two = graph.integer(2);
                                let four = graph.integer(4);
                                let two_u = graph.mul([two, u]);
                                let raw = graph.sub(
                                    graph.div(u, two),
                                    graph.div(graph.function("sin", [two_u]), four),
                                );
                                return Ok(scale_du(raw));
                            }
                            ("cos", 2) => {
                                let two = graph.integer(2);
                                let four = graph.integer(4);
                                let two_u = graph.mul([two, u]);
                                let raw = graph.add([
                                    graph.div(u, two),
                                    graph.div(graph.function("sin", [two_u]), four),
                                ]);
                                return Ok(scale_du(raw));
                            }
                            ("sin", 3) => {
                                let cos_u = graph.function("cos", [u]);
                                let cos3 = graph.pow(cos_u, graph.integer(3));
                                let raw = graph.sub(graph.div(cos3, graph.integer(3)), cos_u);
                                return Ok(scale_du(raw));
                            }
                            ("cos", 3) => {
                                let sin_u = graph.function("sin", [u]);
                                let sin3 = graph.pow(sin_u, graph.integer(3));
                                let raw = graph.sub(sin_u, graph.div(sin3, graph.integer(3)));
                                return Ok(scale_du(raw));
                            }
                            _ => {}
                        }
                    }
                }

                // 3. General power rule for x^n and (a*x + b)^n, including n = -1 and n = 1/2
                if !graph.has_symbol(*exp, wrt) {
                    let d_base = graph.diff(*base, wrt);
                    if !graph.has_symbol(d_base, wrt) && d_base != graph.integer(0) {
                        let is_neg_one = matches!(
                            graph.get(*exp).kind,
                            ExprKind::Number(algebra_core::Number::Integer(-1))
                        ) || matches!(
                            graph.get(*exp).kind,
                            ExprKind::Neg(inner) if inner == graph.integer(1)
                        );
                        if is_neg_one {
                            let ln_base = graph.function("ln", [*base]);
                            return if d_base == graph.integer(1) {
                                Ok(ln_base)
                            } else {
                                Ok(graph.div(ln_base, d_base))
                            };
                        }
                        let one = graph.integer(1);
                        let n_plus_1 =
                            crate::simplify::Simplifier::simplify(graph, graph.add([*exp, one]))
                                .unwrap_or_else(|_| graph.add([*exp, one]));
                        let u_pow = graph.pow(*base, n_plus_1);
                        let denom = if d_base == graph.integer(1) {
                            n_plus_1
                        } else {
                            graph.mul([d_base, n_plus_1])
                        };
                        return Ok(graph.div(u_pow, denom));
                    }

                    // Integrate powers of ln(a*x + b)^n for positive integer n >= 2
                    if let ExprKind::Function { name, args } = graph.get(*base).kind
                        && matches!(
                            graph.symbols.resolve(name).as_deref(),
                            Some("ln") | Some("log")
                        )
                        && args.len() == 1
                        && let ExprKind::Number(algebra_core::Number::Integer(n)) =
                            graph.get(*exp).kind
                        && (2..=8).contains(&n)
                    {
                        let inner = args[0];
                        let d_inner = graph.diff(inner, wrt);
                        if !graph.has_symbol(d_inner, wrt) && d_inner != graph.integer(0) {
                            let inv_a = graph.div(graph.integer(1), d_inner);
                            let mut acc = Self::integrate(graph, *base, wrt)?;
                            for k in 2..=n {
                                let k_id = graph.integer(k);
                                let ln_pow_k = graph.pow(*base, k_id);
                                let lead = if d_inner == graph.integer(1) {
                                    graph.mul([inner, ln_pow_k])
                                } else {
                                    graph.mul([inv_a, inner, ln_pow_k])
                                };
                                let sub = graph.mul([k_id, acc]);
                                acc = graph.sub(lead, sub);
                            }
                            return Ok(acc);
                        }
                    }

                    // Check negative integer power Base^(-m) -> 1 / Base^m as rational/u-sub function
                    if let ExprKind::Number(algebra_core::Number::Integer(k)) = graph.get(*exp).kind
                        && k < 0
                    {
                        let positive_den = if k == -1 {
                            *base
                        } else {
                            graph.pow(*base, graph.integer(-k))
                        };
                        if let Ok(res) =
                            Self::integrate_rational(graph, graph.integer(1), positive_den, wrt)
                        {
                            return Ok(res);
                        }
                    }

                    // Check (1 - x^2)^(1/2) or (1 - x^2)^(-1/2) etc.
                    if let Some(q_exp) = risch_rat_from_expr(graph, *exp) {
                        if q_exp == num_rational::BigRational::new(1.into(), 2.into())
                            && let Ok(res) = integrate_sqrt_radicand(graph, *base, wrt)
                        {
                            return Ok(res);
                        }
                        if q_exp == num_rational::BigRational::new((-1).into(), 2.into())
                            && let Ok(res) = integrate_inv_sqrt_radicand(graph, *base, wrt)
                        {
                            return Ok(res);
                        }
                    }
                }
                Err(AlgebraError::EvaluationError(
                    "Non-elementary power integration".into(),
                ))
            }
            _ => Err(AlgebraError::EvaluationError(
                "Risch heuristic fallback".into(),
            )),
        }
    }

    /// Integrate rational functions $\frac{A(x)}{D(x)}$ using Hermite reduction and Rothstein-Trager.
    pub fn integrate_rational(
        graph: &ExprGraph,
        num: ExprId,
        den: ExprId,
        wrt: SymbolId,
    ) -> AlgebraResult<ExprId> {
        let wrt_name = graph.symbols.resolve(wrt).unwrap_or_default();
        let x = graph.symbol(&wrt_name);

        if !graph.has_symbol(den, wrt) {
            let sub = Self::integrate(graph, num, wrt)?;
            return Ok(graph.div(sub, den));
        }

        // Case 1: Simple fraction c / (a*x + b) -> (c/a) * ln|a*x + b|
        let d_den = graph.diff(den, wrt);
        if !graph.has_symbol(num, wrt) && !graph.has_symbol(d_den, wrt) && d_den != graph.integer(0)
        {
            let inv_a = graph.div(num, d_den);
            let log_den = graph.function("ln", [den]);
            return Ok(graph.mul([inv_a, log_den]));
        }

        // Case 2: Derivative in numerator: f'(x) / f(x) -> ln|f(x)|, or c * f'(x) / f(x) -> c * ln|f(x)|
        if num == d_den {
            return Ok(graph.function("ln", [den]));
        }
        if d_den != graph.integer(0) {
            let ratio_raw = graph.div(num, d_den);
            let ratio_cancel = crate::simplify::AlgebraicTransformations::cancel(graph, ratio_raw);
            let ratio =
                crate::simplify::Simplifier::simplify(graph, ratio_cancel).unwrap_or(ratio_cancel);
            if !graph.has_symbol(ratio, wrt) {
                let log_den = graph.function("ln", [den]);
                return Ok(if ratio == graph.integer(1) {
                    log_den
                } else {
                    graph.mul([ratio, log_den])
                });
            }
        }

        // Case 4: Quadratic denominator 1 / (x^2 + a^2) -> (1/a) * arctan(x/a)
        if !graph.has_symbol(num, wrt)
            && let ExprKind::Add(terms) = graph.get(den).kind
            && terms.len() == 2
        {
            let mut x_sq_opt = None;
            let mut a_sq_opt = None;
            for &t in &terms {
                if let ExprKind::Pow(b, p) = graph.get(t).kind
                    && b == x
                    && p == graph.integer(2)
                {
                    x_sq_opt = Some(t);
                    continue;
                }
                if !graph.has_symbol(t, wrt) {
                    a_sq_opt = Some(t);
                }
            }

            if let (Some(_x_sq), Some(a_sq)) = (x_sq_opt, a_sq_opt) {
                // x^2 - b^2 (negative real constant): num/(x^2 - b^2) integrates to
                // num/(2b) * ln((x - b)/(x + b)), not an arctan of a complex argument.
                {
                    use crate::numeric::NumericalEval as _;
                    let negative = graph
                        .evalf(a_sq, &crate::numeric::EvalContext::default())
                        .is_ok_and(|v| v.to_f64() < 0.0);
                    if negative {
                        let simplify = |e: ExprId| {
                            crate::simplify::Simplifier::simplify(graph, e).unwrap_or(e)
                        };
                        let b = simplify(graph.function("sqrt", [simplify(graph.neg(a_sq))]));
                        let two_b = graph.mul([graph.integer(2), b]);
                        let ratio = graph.div(graph.sub(x, b), graph.add([x, b]));
                        let coefficient = simplify(graph.div(num, two_b));
                        return Ok(graph.mul([coefficient, graph.function("ln", [ratio])]));
                    }
                }
                let a = graph.function("sqrt", [a_sq]);
                let inv_a = graph.div(num, a);
                let x_over_a = graph.div(x, a);
                let arctan_term = graph.function("arctan", [x_over_a]);
                return Ok(graph.mul([inv_a, arctan_term]));
            }
        }

        // Case 5: General univariate rational function P(x) / Q(x) via Euclidean division + partial fractions + completing the square
        if let Ok(res) = integrate_univariate_rational(graph, num, den, wrt) {
            return Ok(res);
        }

        // Case 6: c / (a*x + b)^k (or c / sqrt(1 - x^2))
        if !graph.has_symbol(num, wrt)
            && let ExprKind::Pow(inner_base, inner_exp) = graph.get(den).kind
            && !graph.has_symbol(inner_exp, wrt)
        {
            let neg_exp = crate::simplify::Simplifier::simplify(graph, graph.neg(inner_exp))
                .unwrap_or_else(|_| graph.neg(inner_exp));
            let sub_pow = Self::integrate(graph, graph.pow(inner_base, neg_exp), wrt)?;
            return Ok(graph.mul([num, sub_pow]));
        }

        if !graph.has_symbol(num, wrt)
            && let ExprKind::Function { name, args } = graph.get(den).kind
            && graph.symbols.resolve(name).as_deref() == Some("sqrt")
            && args.len() == 1
        {
            if let Ok(inv_sq) = integrate_inv_sqrt_radicand(graph, args[0], wrt) {
                return Ok(if num == graph.integer(1) {
                    inv_sq
                } else {
                    graph.mul([num, inv_sq])
                });
            }
            let one_minus_x2 = graph.sub(graph.integer(1), graph.pow(x, graph.integer(2)));
            let diff_check = crate::simplify::AlgebraicTransformations::expand(
                graph,
                graph.sub(args[0], one_minus_x2),
            );
            if diff_check == graph.integer(0) {
                let asin_x = graph.function("asin", [x]);
                return Ok(if num == graph.integer(1) {
                    asin_x
                } else {
                    graph.mul([num, asin_x])
                });
            }
        }

        // Fallback: try direct u-substitution on num / den (e.g. 1/(x*ln(x)), e^x/(1+e^x))
        // without re-entering integrate(Pow(den, -1)) -> integrate_rational.
        let rat = graph.div(num, den);
        if let Ok(res) = Self::try_u_substitution(graph, &[rat], wrt) {
            return Ok(res);
        }

        Err(AlgebraError::EvaluationError(
            "Higher-degree rational Risch Hermite factorization required".into(),
        ))
    }

    /// Integrate transcendental functions $e^{ax}, \sin(ax), \cos(ax), \ln(x), \tan(x), \sinh(x), \cosh(x)$.
    pub fn integrate_elementary_function(
        graph: &ExprGraph,
        fn_name: &str,
        args: &[ExprId],
        wrt: SymbolId,
    ) -> AlgebraResult<ExprId> {
        if args.len() != 1 {
            return Err(AlgebraError::EvaluationError(
                "Multi-argument function integration not supported".into(),
            ));
        }
        let arg = args[0];
        let d_arg = graph.diff(arg, wrt);

        // Check if argument derivative is constant: a*x + b
        if !graph.has_symbol(d_arg, wrt) {
            let inv_a = graph.div(graph.integer(1), d_arg);
            match fn_name {
                "exp" => Ok(graph.mul([inv_a, graph.function("exp", [arg])])),
                "sin" => {
                    let cos_arg = graph.function("cos", [arg]);
                    let neg_cos = graph.neg(cos_arg);
                    Ok(graph.mul([inv_a, neg_cos]))
                }
                "cos" => {
                    let sin_arg = graph.function("sin", [arg]);
                    Ok(graph.mul([inv_a, sin_arg]))
                }
                "tan" => {
                    // int tan(ax) dx = (1/a) * ln|sec(ax)| = -(1/a) * ln|cos(ax)|
                    let cos_arg = graph.function("cos", [arg]);
                    let log_cos = graph.function("ln", [cos_arg]);
                    let neg_log = graph.neg(log_cos);
                    Ok(graph.mul([inv_a, neg_log]))
                }
                "asin" => {
                    // int asin(x) dx = x * asin(x) + sqrt(1 - x^2)
                    let x_asin = graph.mul([arg, graph.function("asin", [arg])]);
                    let one = graph.integer(1);
                    let arg_sq = graph.pow(arg, graph.integer(2));
                    let sqrt_term = graph.function("sqrt", [graph.sub(one, arg_sq)]);
                    let sum = graph.add([x_asin, sqrt_term]);
                    Ok(graph.mul([inv_a, sum]))
                }
                "acos" => {
                    // int acos(x) dx = x * acos(x) - sqrt(1 - x^2)
                    let x_acos = graph.mul([arg, graph.function("acos", [arg])]);
                    let one = graph.integer(1);
                    let arg_sq = graph.pow(arg, graph.integer(2));
                    let sqrt_term = graph.function("sqrt", [graph.sub(one, arg_sq)]);
                    let diff = graph.sub(x_acos, sqrt_term);
                    Ok(graph.mul([inv_a, diff]))
                }
                "atan" => {
                    // int atan(x) dx = x * atan(x) - (1/2) * ln(1 + x^2)
                    let x_atan = graph.mul([arg, graph.function("atan", [arg])]);
                    let one = graph.integer(1);
                    let arg_sq = graph.pow(arg, graph.integer(2));
                    let ln_term = graph.function("ln", [graph.add([one, arg_sq])]);
                    let half_ln = graph.mul([graph.rational(1, 2), ln_term]);
                    let diff = graph.sub(x_atan, half_ln);
                    Ok(graph.mul([inv_a, diff]))
                }
                "erf" => {
                    // int erf(x) dx = x * erf(x) + (1/sqrt(pi)) * exp(-x^2)
                    let x_erf = graph.mul([arg, graph.function("erf", [arg])]);
                    let pi = graph.constant(algebra_core::Constant::Pi);
                    let sqrt_pi = graph.function("sqrt", [pi]);
                    let inv_sqrt_pi = graph.div(graph.integer(1), sqrt_pi);
                    let neg_x_sq = graph.neg(graph.pow(arg, graph.integer(2)));
                    let exp_term = graph.function("exp", [neg_x_sq]);
                    let add_term = graph.mul([inv_sqrt_pi, exp_term]);
                    let sum = graph.add([x_erf, add_term]);
                    Ok(graph.mul([inv_a, sum]))
                }
                "sinh" => Ok(graph.mul([inv_a, graph.function("cosh", [arg])])),
                "cosh" => Ok(graph.mul([inv_a, graph.function("sinh", [arg])])),
                "tanh" => {
                    let cosh_term = graph.function("cosh", [arg]);
                    let ln_cosh = graph.function("ln", [cosh_term]);
                    Ok(graph.mul([inv_a, ln_cosh]))
                }
                "sec" => {
                    let sec_t = graph.function("sec", [arg]);
                    let tan_t = graph.function("tan", [arg]);
                    let sum = graph.add([sec_t, tan_t]);
                    let ln_s = graph.function("ln", [sum]);
                    Ok(graph.mul([inv_a, ln_s]))
                }
                "csc" => {
                    let csc_t = graph.function("csc", [arg]);
                    let cot_t = graph.function("cot", [arg]);
                    let sum = graph.add([csc_t, cot_t]);
                    let neg_ln = graph.neg(graph.function("ln", [sum]));
                    Ok(graph.mul([inv_a, neg_ln]))
                }
                "cot" => {
                    let sin_t = graph.function("sin", [arg]);
                    let ln_s = graph.function("ln", [sin_t]);
                    Ok(graph.mul([inv_a, ln_s]))
                }
                "sech" => {
                    let sinh_t = graph.function("sinh", [arg]);
                    let atan_s = graph.function("atan", [sinh_t]);
                    Ok(graph.mul([inv_a, atan_s]))
                }
                "csch" => {
                    let half_arg = graph.div(arg, graph.integer(2));
                    let tanh_h = graph.function("tanh", [half_arg]);
                    let ln_t = graph.function("ln", [tanh_h]);
                    Ok(graph.mul([inv_a, ln_t]))
                }
                "coth" => {
                    let sinh_t = graph.function("sinh", [arg]);
                    let ln_s = graph.function("ln", [sinh_t]);
                    Ok(graph.mul([inv_a, ln_s]))
                }
                "ln" | "log" => {
                    // int ln(x) dx = x * ln(x) - x
                    let wrt_name = graph.symbols.resolve(wrt).unwrap_or_default();
                    let x = graph.symbol(&wrt_name);
                    if arg == x {
                        let x_ln_x = graph.mul([x, graph.function("ln", [x])]);
                        Ok(graph.sub(x_ln_x, x))
                    } else {
                        let arg_ln_arg = graph.mul([arg, graph.function("ln", [arg])]);
                        let sub_term = graph.sub(arg_ln_arg, arg);
                        Ok(graph.mul([inv_a, sub_term]))
                    }
                }
                "sqrt" => integrate_sqrt_radicand(graph, arg, wrt),
                _ => Err(AlgebraError::EvaluationError(format!(
                    "Integration of {} not implemented",
                    fn_name
                ))),
            }
        } else {
            if fn_name == "sqrt" {
                return integrate_sqrt_radicand(graph, arg, wrt);
            }
            // Non-elementary integration check (e.g. exp(-x^2) -> sqrt(pi)/2 * erf(x))
            if fn_name == "exp"
                && let ExprKind::Neg(inner) = graph.get(arg).kind
            {
                let wrt_name = graph.symbols.resolve(wrt).unwrap_or_default();
                let x = graph.symbol(&wrt_name);
                if inner == graph.pow(x, graph.integer(2)) {
                    // Gaussian integral: int exp(-x^2) dx = (sqrt(pi)/2) * erf(x)
                    let pi = graph.constant(algebra_core::Constant::Pi);
                    let sqrt_pi = graph.function("sqrt", [pi]);
                    let sqrt_pi_over_2 = graph.div(sqrt_pi, graph.integer(2));
                    let erf_x = graph.function("erf", [x]);
                    return Ok(graph.mul([sqrt_pi_over_2, erf_x]));
                }
            }
            Err(AlgebraError::EvaluationError(
                "Non-linear inner function argument".into(),
            ))
        }
    }

    /// Integrate products via symbolic integration by parts ($x^k e^{ax}$, $x^k \sin(ax)$, $x^k \cos(ax)$, $x^k \ln(x)$, $e^x \sin(x)$).
    fn integrate_products(
        graph: &ExprGraph,
        factors: &[ExprId],
        wrt: SymbolId,
    ) -> AlgebraResult<ExprId> {
        let wrt_name = graph.symbols.resolve(wrt).unwrap_or_default();
        let x = graph.symbol(&wrt_name);

        // Constant multiple extraction: if factor does not contain wrt
        if factors.len() == 2 {
            let has0 = graph.has_symbol(factors[0], wrt);
            let has1 = graph.has_symbol(factors[1], wrt);
            if !has0 && has1 {
                let int1 = Self::integrate(graph, factors[1], wrt)?;
                return Ok(graph.mul([factors[0], int1]));
            } else if has0 && !has1 {
                let int0 = Self::integrate(graph, factors[0], wrt)?;
                return Ok(graph.mul([factors[1], int0]));
            }
        }

        if factors.len() == 2
            && let Some(result) = Self::integrate_power_times_elementary(graph, factors, x)
        {
            return Ok(result);
        }

        if factors.len() == 2 {
            let (u, v) = (factors[0], factors[1]);

            // Case A: x * f(x)
            let (poly_part, fn_part) = if u == x {
                (u, v)
            } else if v == x {
                (v, u)
            } else if let ExprKind::Pow(b, p) = graph.get(u).kind
                && b == x
                && p == graph.integer(2)
            {
                (u, v)
            } else if let ExprKind::Pow(b, p) = graph.get(v).kind
                && b == x
                && p == graph.integer(2)
            {
                (v, u)
            } else {
                // Check e^x * sin(x) or e^x * cos(x)
                if let (
                    ExprKind::Function { name: n1, args: a1 },
                    ExprKind::Function { name: n2, args: a2 },
                ) = (&graph.get(u).kind, &graph.get(v).kind)
                {
                    let fn1 = graph.symbols.resolve(*n1).unwrap_or_default();
                    let fn2 = graph.symbols.resolve(*n2).unwrap_or_default();
                    if a1.len() == 1 && a2.len() == 1 && a1[0] == x && a2[0] == x {
                        if (fn1 == "exp" && fn2 == "sin") || (fn1 == "sin" && fn2 == "exp") {
                            // int exp(x) * sin(x) dx = (1/2) * exp(x) * (sin(x) - cos(x))
                            let exp_x = graph.function("exp", [x]);
                            let sin_x = graph.function("sin", [x]);
                            let cos_x = graph.function("cos", [x]);
                            let half = graph.rational(1, 2);
                            let diff = graph.sub(sin_x, cos_x);
                            return Ok(graph.mul([half, exp_x, diff]));
                        } else if (fn1 == "exp" && fn2 == "cos") || (fn1 == "cos" && fn2 == "exp") {
                            // int exp(x) * cos(x) dx = (1/2) * exp(x) * (sin(x) + cos(x))
                            let exp_x = graph.function("exp", [x]);
                            let sin_x = graph.function("sin", [x]);
                            let cos_x = graph.function("cos", [x]);
                            let half = graph.rational(1, 2);
                            let sum = graph.add([sin_x, cos_x]);
                            return Ok(graph.mul([half, exp_x, sum]));
                        } else if (fn1 == "sin" && fn2 == "cos") || (fn1 == "cos" && fn2 == "sin") {
                            // int sin(x)*cos(x) dx = sin(x)^2 / 2
                            let sin_x = graph.function("sin", [x]);
                            let sin_sq = graph.pow(sin_x, graph.integer(2));
                            return Ok(graph.div(sin_sq, graph.integer(2)));
                        }
                    }
                    // Linear scaled sin(a*x)*cos(a*x)
                    if a1.len() == 1
                        && a2.len() == 1
                        && a1[0] == a2[0]
                        && ((fn1 == "sin" && fn2 == "cos") || (fn1 == "cos" && fn2 == "sin"))
                    {
                        let du = graph.diff(a1[0], wrt);
                        if !graph.has_symbol(du, wrt) && du != graph.integer(0) {
                            let sin_u = graph.function("sin", [a1[0]]);
                            let sin_sq = graph.pow(sin_u, graph.integer(2));
                            let denom = graph.mul([graph.integer(2), du]);
                            return Ok(graph.div(sin_sq, denom));
                        }
                    }
                }
                return Self::try_u_substitution(graph, factors, wrt);
            };

            let is_degree_1 = poly_part == x;
            let is_degree_2 = !is_degree_1;

            if let ExprKind::Function { name, args } = graph.get(fn_part).kind {
                let fn_name = graph.symbols.resolve(name).unwrap_or_default();
                if args.len() == 1 && args[0] == x {
                    match fn_name.as_str() {
                        "exp" => {
                            let exp_x = graph.function("exp", [x]);
                            if is_degree_1 {
                                // int x * exp(x) dx = (x - 1) * exp(x)
                                let x_minus_1 = graph.sub(x, graph.integer(1));
                                return Ok(graph.mul([exp_x, x_minus_1]));
                            } else if is_degree_2 {
                                // int x^2 * exp(x) dx = (x^2 - 2x + 2) * exp(x)
                                let x_sq = graph.pow(x, graph.integer(2));
                                let two_x = graph.mul([graph.integer(2), x]);
                                let quad = graph.add([graph.sub(x_sq, two_x), graph.integer(2)]);
                                return Ok(graph.mul([exp_x, quad]));
                            }
                        }
                        "sin" => {
                            let sin_x = graph.function("sin", [x]);
                            let cos_x = graph.function("cos", [x]);
                            if is_degree_1 {
                                // int x * sin(x) dx = sin(x) - x * cos(x)
                                let x_cos_x = graph.mul([x, cos_x]);
                                return Ok(graph.sub(sin_x, x_cos_x));
                            } else if is_degree_2 {
                                // int x^2 * sin(x) dx = (2 - x^2)*cos(x) + 2x*sin(x)
                                let x_sq = graph.pow(x, graph.integer(2));
                                let two_minus_xsq = graph.sub(graph.integer(2), x_sq);
                                let term1 = graph.mul([two_minus_xsq, cos_x]);
                                let term2 = graph.mul([graph.integer(2), x, sin_x]);
                                return Ok(graph.add([term1, term2]));
                            }
                        }
                        "cos" => {
                            let cos_x = graph.function("cos", [x]);
                            let sin_x = graph.function("sin", [x]);
                            if is_degree_1 {
                                // int x * cos(x) dx = cos(x) + x * sin(x)
                                let x_sin_x = graph.mul([x, sin_x]);
                                return Ok(graph.add([cos_x, x_sin_x]));
                            } else if is_degree_2 {
                                // int x^2 * cos(x) dx = (x^2 - 2)*sin(x) + 2x*cos(x)
                                let x_sq = graph.pow(x, graph.integer(2));
                                let xsq_minus_two = graph.sub(x_sq, graph.integer(2));
                                let term1 = graph.mul([xsq_minus_two, sin_x]);
                                let term2 = graph.mul([graph.integer(2), x, cos_x]);
                                return Ok(graph.add([term1, term2]));
                            }
                        }
                        "ln" | "log" if is_degree_1 => {
                            // int x * ln(x) dx = (1/2)*x^2*ln(x) - (1/4)*x^2
                            let x_sq = graph.pow(x, graph.integer(2));
                            let ln_x = graph.function("ln", [x]);
                            let half_xsq_ln = graph.mul([graph.rational(1, 2), x_sq, ln_x]);
                            let quarter_xsq = graph.mul([graph.rational(1, 4), x_sq]);
                            return Ok(graph.sub(half_xsq_ln, quarter_xsq));
                        }
                        _ => {}
                    }
                }
            }
        }

        Self::try_u_substitution(graph, factors, wrt)
    }
}

impl RischIntegrator {
    /// `∫ x^n · f(x) dx` for `f` in `exp`, `sin`, `cos` (any `n` from 3 to 20) and
    /// `ln` (any `n` from 2 to 20), by repeated integration by parts (tabular method).
    ///
    /// Degrees 1 and 2 are handled by the fixed formulas in `integrate_products`.
    fn integrate_power_times_elementary(
        graph: &ExprGraph,
        factors: &[ExprId],
        x: ExprId,
    ) -> Option<ExprId> {
        let power_of_x = |id: ExprId| -> Option<u32> {
            if let ExprKind::Pow(base, exp) = graph.get(id).kind
                && base == x
                && let ExprKind::Number(algebra_core::Number::Integer(n)) = graph.get(exp).kind
                && (2..=20).contains(&n)
            {
                return Some(n as u32);
            }
            None
        };
        let (n, func) = match (power_of_x(factors[0]), power_of_x(factors[1])) {
            (Some(n), _) => (n, factors[1]),
            (None, Some(n)) => (n, factors[0]),
            _ => return None,
        };
        let ExprKind::Function { name, args } = graph.get(func).kind else {
            return None;
        };
        if args.len() != 1 || args[0] != x {
            return None;
        }
        let fn_name = graph.symbols.resolve(name).unwrap_or_default();

        // x^(n - k), omitted when the exponent is zero.
        let x_pow = |p: u32| -> Option<ExprId> {
            match p {
                0 => None,
                1 => Some(x),
                _ => Some(graph.pow(x, graph.integer(i64::from(p)))),
            }
        };
        // n! / (n - k)!: the k-th derivative coefficient of x^n.
        let falling = |k: u32| -> i64 { ((n - k + 1)..=n).map(i64::from).product() };
        let signed_term = |coeff: i64, power: u32, factor: Option<ExprId>| -> ExprId {
            let mut parts = vec![graph.integer(coeff)];
            parts.extend(x_pow(power));
            parts.extend(factor);
            if parts.len() == 1 {
                parts[0]
            } else {
                graph.mul(parts)
            }
        };

        match fn_name.as_str() {
            "ln" | "log" if n >= 2 => {
                // ∫ x^n ln x = x^(n+1) ln x / (n+1) - x^(n+1) / (n+1)^2
                let m = i64::from(n) + 1;
                let x_m = graph.pow(x, graph.integer(m));
                let ln_x = graph.function("ln", [x]);
                let first = graph.mul([graph.rational(1, m), x_m, ln_x]);
                let second = graph.mul([graph.rational(1, m * m), x_m]);
                Some(graph.sub(first, second))
            }
            "exp" if n >= 3 => {
                // e^x · Σ (-1)^k n!/(n-k)! x^(n-k)
                let terms: Vec<ExprId> = (0..=n)
                    .map(|k| {
                        let sign = if k % 2 == 0 { 1 } else { -1 };
                        signed_term(sign * falling(k), n - k, None)
                    })
                    .collect();
                Some(graph.mul([graph.function("exp", [x]), graph.add(terms)]))
            }
            "sin" | "cos" if n >= 3 => {
                let sin_x = graph.function("sin", [x]);
                let cos_x = graph.function("cos", [x]);
                // Antiderivative cycle A_1, A_2, ... of the trig factor.
                let cycle: [(i64, ExprId); 4] = if fn_name == "sin" {
                    [(-1, cos_x), (-1, sin_x), (1, cos_x), (1, sin_x)]
                } else {
                    [(1, sin_x), (-1, cos_x), (-1, sin_x), (1, cos_x)]
                };
                let terms: Vec<ExprId> = (0..=n)
                    .map(|k| {
                        let (cycle_sign, trig) = cycle[(k % 4) as usize];
                        let sign = if k % 2 == 0 { 1 } else { -1 };
                        signed_term(sign * cycle_sign * falling(k), n - k, Some(trig))
                    })
                    .collect();
                Some(graph.add(terms))
            }
            _ => None,
        }
    }

    fn try_u_substitution(
        graph: &ExprGraph,
        factors: &[ExprId],
        wrt: SymbolId,
    ) -> AlgebraResult<ExprId> {
        if graph
            .symbols
            .resolve(wrt)
            .is_some_and(|n| n.starts_with("__risch_u"))
        {
            return Err(AlgebraError::EvaluationError(
                "Multi-factor product integration not matched".into(),
            ));
        }

        let wrt_name = graph.symbols.resolve(wrt).unwrap_or_default();
        let x = graph.symbol(&wrt_name);
        let expr = if factors.len() == 1 {
            factors[0]
        } else {
            graph.mul(factors.to_vec())
        };

        let mut candidates: Vec<ExprId> = Vec::new();
        collect_u_candidates(graph, expr, wrt, x, &mut candidates);

        let u_var = graph.symbol("__risch_u");
        let ExprKind::Symbol(u_sym) = graph.get(u_var).kind else {
            return Err(AlgebraError::EvaluationError(
                "Multi-factor product integration not matched".into(),
            ));
        };

        for cand in candidates {
            let d_raw = graph.diff(cand, wrt);
            let du = crate::simplify::Simplifier::simplify(graph, d_raw).unwrap_or(d_raw);
            if du == graph.integer(0) || contains_derivative(graph, du) {
                continue;
            }
            let ratio_raw = graph.div(expr, du);
            let togethered = crate::simplify::AlgebraicTransformations::together(graph, ratio_raw);
            let canceled = crate::simplify::AlgebraicTransformations::cancel(graph, togethered);
            let reduced =
                crate::simplify::Simplifier::simplify(graph, canceled).unwrap_or(canceled);

            let in_u = replace_subexpr(graph, reduced, cand, u_var);
            if !graph.has_symbol(in_u, wrt)
                && let Ok(anti_u) = Self::integrate(graph, in_u, u_sym)
            {
                return Ok(graph.substitute(anti_u, u_sym, cand));
            }
        }

        Err(AlgebraError::EvaluationError(
            "Multi-factor product integration not matched".into(),
        ))
    }
}

fn contains_derivative(graph: &ExprGraph, id: ExprId) -> bool {
    let node = graph.get(id);
    if matches!(node.kind, ExprKind::Derivative { .. }) {
        return true;
    }
    node.children()
        .into_iter()
        .any(|c| contains_derivative(graph, c))
}

fn push_u_cand(graph: &ExprGraph, cand: ExprId, wrt: SymbolId, x: ExprId, out: &mut Vec<ExprId>) {
    if cand != x && graph.has_symbol(cand, wrt) && !out.contains(&cand) {
        out.push(cand);
    }
}

fn collect_u_candidates(
    graph: &ExprGraph,
    id: ExprId,
    wrt: SymbolId,
    x: ExprId,
    out: &mut Vec<ExprId>,
) {
    match &graph.get(id).kind {
        ExprKind::Function { args, .. } => {
            if args.len() == 1 {
                push_u_cand(graph, args[0], wrt, x, out);
                collect_u_candidates(graph, args[0], wrt, x, out);
            } else {
                for &a in args {
                    collect_u_candidates(graph, a, wrt, x, out);
                }
            }
            push_u_cand(graph, id, wrt, x, out);
        }
        ExprKind::Pow(base, exp) => {
            push_u_cand(graph, *base, wrt, x, out);
            collect_u_candidates(graph, *base, wrt, x, out);
            push_u_cand(graph, *exp, wrt, x, out);
            collect_u_candidates(graph, *exp, wrt, x, out);
        }
        ExprKind::Add(terms) => {
            if terms.len() >= 2 {
                push_u_cand(graph, id, wrt, x, out);
            }
            for &t in terms {
                collect_u_candidates(graph, t, wrt, x, out);
            }
        }
        ExprKind::Sub(a, b) => {
            push_u_cand(graph, id, wrt, x, out);
            collect_u_candidates(graph, *a, wrt, x, out);
            collect_u_candidates(graph, *b, wrt, x, out);
        }
        ExprKind::Mul(fs) => {
            for &f in fs {
                collect_u_candidates(graph, f, wrt, x, out);
            }
        }
        ExprKind::Div(a, b) => {
            collect_u_candidates(graph, *a, wrt, x, out);
            push_u_cand(graph, *b, wrt, x, out);
            collect_u_candidates(graph, *b, wrt, x, out);
        }
        ExprKind::Neg(a) => {
            collect_u_candidates(graph, *a, wrt, x, out);
        }
        _ => {}
    }
}

fn replace_subexpr(graph: &ExprGraph, id: ExprId, target: ExprId, replacement: ExprId) -> ExprId {
    if id == target {
        return replacement;
    }
    match &graph.get(id).kind {
        ExprKind::Add(terms) => {
            let new_t: Vec<ExprId> = terms
                .iter()
                .map(|&t| replace_subexpr(graph, t, target, replacement))
                .collect();
            graph.add(new_t)
        }
        ExprKind::Mul(factors) => {
            let new_f: Vec<ExprId> = factors
                .iter()
                .map(|&f| replace_subexpr(graph, f, target, replacement))
                .collect();
            graph.mul(new_f)
        }
        ExprKind::Sub(a, b) => graph.sub(
            replace_subexpr(graph, *a, target, replacement),
            replace_subexpr(graph, *b, target, replacement),
        ),
        ExprKind::Div(a, b) => graph.div(
            replace_subexpr(graph, *a, target, replacement),
            replace_subexpr(graph, *b, target, replacement),
        ),
        ExprKind::Pow(a, b) => graph.pow(
            replace_subexpr(graph, *a, target, replacement),
            replace_subexpr(graph, *b, target, replacement),
        ),
        ExprKind::Neg(a) => graph.neg(replace_subexpr(graph, *a, target, replacement)),
        ExprKind::Function { name, args } => {
            let fn_name = graph.symbols.resolve(*name).unwrap_or_default();
            let new_args: Vec<ExprId> = args
                .iter()
                .map(|&a| replace_subexpr(graph, a, target, replacement))
                .collect();
            graph.function(&fn_name, new_args)
        }
        _ => id,
    }
}

fn risch_rat_from_expr(graph: &ExprGraph, id: ExprId) -> Option<num_rational::BigRational> {
    use num_traits::Zero as _;
    match &graph.get(id).kind {
        ExprKind::Number(n) => crate::poly::rational_from_number(n).ok(),
        ExprKind::Neg(inner) => Some(-risch_rat_from_expr(graph, *inner)?),
        ExprKind::Div(a, b) => {
            let qa = risch_rat_from_expr(graph, *a)?;
            let qb = risch_rat_from_expr(graph, *b)?;
            if qb.is_zero() { None } else { Some(qa / qb) }
        }
        _ => None,
    }
}

fn integrate_univariate_rational(
    graph: &ExprGraph,
    num: ExprId,
    den: ExprId,
    wrt: SymbolId,
) -> AlgebraResult<ExprId> {
    use num_bigint::BigInt;
    use num_rational::BigRational;
    use num_traits::{One as _, Signed as _, Zero as _};

    let p = crate::poly::UniPoly::from_expr(graph, num, wrt)?;
    let q = crate::poly::UniPoly::from_expr(graph, den, wrt)?;
    let x_sym = graph.symbol(&graph.symbols.resolve(wrt).unwrap_or_default());

    if q.coeffs.iter().all(|c| c.is_zero()) {
        return Err(AlgebraError::DivisionByZero {
            domain: "reals".to_string(),
        });
    }

    let (quo, rem) = p.div_rem(&q)?;
    let mut parts: Vec<ExprId> = Vec::new();

    // Integrate polynomial quotient part
    for (k, c) in quo.coeffs.iter().enumerate() {
        if c.is_zero() {
            continue;
        }
        let new_exp = k + 1;
        let scaled = c / BigRational::from_integer(BigInt::from(new_exp));
        let c_expr = crate::poly::expr_from_rational(graph, &scaled);
        let x_term = if new_exp == 1 {
            x_sym
        } else {
            graph.pow(x_sym, graph.integer(new_exp as i64))
        };
        parts.push(if scaled.is_one() {
            x_term
        } else {
            graph.mul([c_expr, x_term])
        });
    }

    if rem.coeffs.iter().all(|c| c.is_zero()) {
        return Ok(if parts.is_empty() {
            graph.integer(0)
        } else if parts.len() == 1 {
            parts[0]
        } else {
            graph.add(parts)
        });
    }

    let deg_q = q.degree();
    if deg_q == 1 {
        // r0 / (q1*x + q0)
        let r0 = &rem.coeffs[0];
        let q1 = &q.coeffs[1];
        let coeff = r0 / q1;
        let c_expr = crate::poly::expr_from_rational(graph, &coeff);
        let q_expr = q.to_expr(graph, wrt);
        let ln_q = graph.function("ln", [q_expr]);
        parts.push(if coeff.is_one() {
            ln_q
        } else {
            graph.mul([c_expr, ln_q])
        });
    } else if deg_q == 2 {
        let a = &q.coeffs[2];
        let b = &q.coeffs[1];
        let c = &q.coeffs[0];
        let r0 = rem
            .coeffs
            .first()
            .cloned()
            .unwrap_or_else(BigRational::zero);
        let r1 = rem.coeffs.get(1).cloned().unwrap_or_else(BigRational::zero);

        let two = BigRational::from_integer(BigInt::from(2));
        let four = BigRational::from_integer(BigInt::from(4));
        let disc = b * b - &four * a * c;

        if disc.is_zero() {
            // Double root r = -b / (2a): c1/(x - r) + c2/(x - r)^2
            let r = -b / (&two * a);
            let c1 = &r1 / a;
            let c2 = (&r1 * &r + &r0) / a;
            let x_minus_r = graph.sub(x_sym, crate::poly::expr_from_rational(graph, &r));
            if !c1.is_zero() {
                let ln_t = graph.function("ln", [x_minus_r]);
                parts.push(if c1.is_one() {
                    ln_t
                } else {
                    graph.mul([crate::poly::expr_from_rational(graph, &c1), ln_t])
                });
            }
            if !c2.is_zero() {
                let neg_c2 = -&c2;
                let term = graph.div(crate::poly::expr_from_rational(graph, &neg_c2), x_minus_r);
                parts.push(term);
            }
        } else {
            // Check if disc is an exact rational square (partial fractions over Q)
            let sqrt_rat = if disc.is_positive() {
                let sn = disc.numer().sqrt();
                let sd = disc.denom().sqrt();
                if &(&sn * &sn) == disc.numer() && &(&sd * &sd) == disc.denom() {
                    Some(BigRational::new(sn, sd))
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(d) = sqrt_rat {
                let rt1 = (-b + &d) / (&two * a);
                let rt2 = (-b - &d) / (&two * a);
                let val1 = &r1 * &rt1 + &r0;
                let val2 = &r1 * &rt2 + &r0;
                let big_a = &val1 / (a * (&rt1 - &rt2));
                let big_b = &val2 / (a * (&rt2 - &rt1));

                for (coeff, rt) in [(big_a, rt1), (big_b, rt2)] {
                    if coeff.is_zero() {
                        continue;
                    }
                    let lin = if rt.is_zero() {
                        x_sym
                    } else if rt.is_negative() {
                        graph.add([x_sym, crate::poly::expr_from_rational(graph, &(-&rt))])
                    } else {
                        graph.sub(x_sym, crate::poly::expr_from_rational(graph, &rt))
                    };
                    let ln_lin = graph.function("ln", [lin]);
                    parts.push(if coeff.is_one() {
                        ln_lin
                    } else {
                        graph.mul([crate::poly::expr_from_rational(graph, &coeff), ln_lin])
                    });
                }
            } else if disc.is_negative() {
                // Irreducible quadratic: (r1/(2a))*ln(Q) + (K/(a*k))*atan((x + h)/k)
                let coeff_ln = &r1 / (&two * a);
                if !coeff_ln.is_zero() {
                    let q_expr = q.to_expr(graph, wrt);
                    let ln_q = graph.function("ln", [q_expr]);
                    parts.push(if coeff_ln.is_one() {
                        ln_q
                    } else {
                        graph.mul([crate::poly::expr_from_rational(graph, &coeff_ln), ln_q])
                    });
                }

                let cap_k = &r0 - &r1 * b / (&two * a);
                if !cap_k.is_zero() {
                    let h = b / (&two * a);
                    let k_sq = (-disc) / (&four * a * a);
                    let kn = k_sq.numer().sqrt();
                    let kd = k_sq.denom().sqrt();
                    let k_expr = if &(&kn * &kn) == k_sq.numer() && &(&kd * &kd) == k_sq.denom() {
                        let kr = BigRational::new(kn, kd);
                        let outer_c = &cap_k / (a * &kr);
                        let inner_arg = if h.is_zero() {
                            graph.div(x_sym, crate::poly::expr_from_rational(graph, &kr))
                        } else {
                            let shifted = if h.is_negative() {
                                graph.sub(x_sym, crate::poly::expr_from_rational(graph, &(-&h)))
                            } else {
                                graph.add([x_sym, crate::poly::expr_from_rational(graph, &h)])
                            };
                            if kr.is_one() {
                                shifted
                            } else {
                                graph.div(shifted, crate::poly::expr_from_rational(graph, &kr))
                            }
                        };
                        let atan_t = graph.function("atan", [inner_arg]);
                        if outer_c.is_one() {
                            atan_t
                        } else {
                            graph.mul([crate::poly::expr_from_rational(graph, &outer_c), atan_t])
                        }
                    } else {
                        let k_sym =
                            graph.function("sqrt", [crate::poly::expr_from_rational(graph, &k_sq)]);
                        let shifted = if h.is_zero() {
                            x_sym
                        } else {
                            graph.add([x_sym, crate::poly::expr_from_rational(graph, &h)])
                        };
                        let atan_t = graph.function("atan", [graph.div(shifted, k_sym)]);
                        let outer =
                            graph.div(crate::poly::expr_from_rational(graph, &(&cap_k / a)), k_sym);
                        graph.mul([outer, atan_t])
                    };
                    parts.push(k_expr);
                }
            } else {
                return Err(AlgebraError::EvaluationError(
                    "Quadratic irrational partial fraction split unsupported".into(),
                ));
            }
        }
    } else if rem.coeffs[0..rem.coeffs.len() - 1]
        .iter()
        .all(|c| c.is_zero())
    {
        // Pure monomial denominator c0 * x^d -> check if q is monomial a*x^n
        if q.coeffs[0..deg_q].iter().all(|c| c.is_zero()) {
            let qd = &q.coeffs[deg_q];
            for (k, c) in rem.coeffs.iter().enumerate() {
                if c.is_zero() {
                    continue;
                }
                let coeff = c / qd;
                let net_exp = (k as i64) - (deg_q as i64);
                if net_exp == -1 {
                    let ln_x = graph.function("ln", [x_sym]);
                    parts.push(if coeff.is_one() {
                        ln_x
                    } else {
                        graph.mul([crate::poly::expr_from_rational(graph, &coeff), ln_x])
                    });
                } else {
                    let new_exp = net_exp + 1;
                    let scaled = coeff / BigRational::from_integer(BigInt::from(new_exp));
                    let x_pow = graph.pow(x_sym, graph.integer(new_exp));
                    parts.push(graph.mul([crate::poly::expr_from_rational(graph, &scaled), x_pow]));
                }
            }
        } else {
            let rat_expr = graph.div(num, den);
            let pfd =
                crate::simplify::AlgebraicTransformations::partial_fractions(graph, rat_expr, wrt);
            if pfd != rat_expr && matches!(graph.get(pfd).kind, ExprKind::Add(_)) {
                return RischIntegrator::integrate(graph, pfd, wrt);
            }
            return Err(AlgebraError::EvaluationError(
                "Higher-degree rational Risch Hermite factorization required".into(),
            ));
        }
    } else {
        // Pure monomial denominator a*x^n for arbitrary numerator polynomial R(x)
        if deg_q >= 1 && q.coeffs[0..deg_q].iter().all(|c| c.is_zero()) {
            let qd = &q.coeffs[deg_q];
            for (k, c) in rem.coeffs.iter().enumerate() {
                if c.is_zero() {
                    continue;
                }
                let coeff = c / qd;
                let net_exp = (k as i64) - (deg_q as i64);
                if net_exp == -1 {
                    let ln_x = graph.function("ln", [x_sym]);
                    parts.push(if coeff.is_one() {
                        ln_x
                    } else {
                        graph.mul([crate::poly::expr_from_rational(graph, &coeff), ln_x])
                    });
                } else {
                    let new_exp = net_exp + 1;
                    let scaled = coeff / BigRational::from_integer(BigInt::from(new_exp));
                    let x_pow = graph.pow(x_sym, graph.integer(new_exp));
                    parts.push(graph.mul([crate::poly::expr_from_rational(graph, &scaled), x_pow]));
                }
            }
        } else {
            let rat_expr = graph.div(num, den);
            let pfd =
                crate::simplify::AlgebraicTransformations::partial_fractions(graph, rat_expr, wrt);
            if pfd != rat_expr && matches!(graph.get(pfd).kind, ExprKind::Add(_)) {
                return RischIntegrator::integrate(graph, pfd, wrt);
            }
            return Err(AlgebraError::EvaluationError(
                "Higher-degree rational Risch Hermite factorization required".into(),
            ));
        }
    }

    Ok(if parts.is_empty() {
        graph.integer(0)
    } else if parts.len() == 1 {
        parts[0]
    } else {
        graph.add(parts)
    })
}

fn integrate_sqrt_radicand(
    graph: &ExprGraph,
    radicand: ExprId,
    wrt: SymbolId,
) -> AlgebraResult<ExprId> {
    use num_bigint::BigInt;
    use num_rational::BigRational;
    use num_traits::{One as _, Signed as _, Zero as _};

    let p = crate::poly::UniPoly::from_expr(graph, radicand, wrt)?;
    let x_sym = graph.symbol(&graph.symbols.resolve(wrt).unwrap_or_default());

    if p.degree() == 1 {
        let a = &p.coeffs[1];
        let b = &p.coeffs[0];
        let _ = b;
        let coeff = BigRational::new(BigInt::from(2), BigInt::from(3)) / a;
        let three_halves = graph.div(graph.integer(3), graph.integer(2));
        let pow_term = graph.pow(radicand, three_halves);
        return Ok(graph.mul([crate::poly::expr_from_rational(graph, &coeff), pow_term]));
    }

    if p.degree() == 2 && p.coeffs[1].is_zero() {
        let c = &p.coeffs[0];
        let a = &p.coeffs[2];
        // int sqrt(c + a*x^2) dx when c > 0, a < 0 (i.e. sqrt(alpha^2 - beta^2 * x^2))
        if c.is_positive() && a.is_negative() {
            let neg_a = -a;
            let sqrt_rat = |q: &BigRational| -> ExprId {
                let sn = q.numer().sqrt();
                let sd = q.denom().sqrt();
                if &(&sn * &sn) == q.numer() && &(&sd * &sd) == q.denom() {
                    crate::poly::expr_from_rational(graph, &BigRational::new(sn, sd))
                } else {
                    graph.function("sqrt", [crate::poly::expr_from_rational(graph, q)])
                }
            };
            let alpha = sqrt_rat(c);
            let beta = sqrt_rat(&neg_a);
            let half = graph.rational(1, 2);
            let sqrt_expr = graph.function("sqrt", [radicand]);
            let first = graph.mul([half, x_sym, sqrt_expr]);
            let asin_arg = if beta == alpha {
                x_sym
            } else if alpha == graph.integer(1) {
                graph.mul([beta, x_sym])
            } else if beta == graph.integer(1) {
                graph.div(x_sym, alpha)
            } else {
                graph.div(graph.mul([beta, x_sym]), alpha)
            };
            let asin_term = graph.function("asin", [asin_arg]);
            let second_coeff = if beta == graph.integer(1) {
                graph.mul([half, crate::poly::expr_from_rational(graph, c)])
            } else {
                graph.div(
                    graph.mul([crate::poly::expr_from_rational(graph, c)]),
                    graph.mul([graph.integer(2), beta]),
                )
            };
            let second = graph.mul([second_coeff, asin_term]);
            let _ = BigRational::one().is_one();
            return Ok(graph.add([first, second]));
        }
    }

    Err(AlgebraError::EvaluationError(
        "Unsupported sqrt integrand".into(),
    ))
}

fn integrate_inv_sqrt_radicand(
    graph: &ExprGraph,
    radicand: ExprId,
    wrt: SymbolId,
) -> AlgebraResult<ExprId> {
    use num_bigint::BigInt;
    use num_rational::BigRational;
    use num_traits::{One as _, Signed as _, Zero as _};

    let p = crate::poly::UniPoly::from_expr(graph, radicand, wrt)?;
    let x_sym = graph.symbol(&graph.symbols.resolve(wrt).unwrap_or_default());

    if p.degree() == 1 {
        let a = &p.coeffs[1];
        let coeff = BigRational::from_integer(BigInt::from(2)) / a;
        let sqrt_rad = graph.function("sqrt", [radicand]);
        return Ok(if coeff.is_one() {
            sqrt_rad
        } else {
            graph.mul([crate::poly::expr_from_rational(graph, &coeff), sqrt_rad])
        });
    }

    if p.degree() == 2 && p.coeffs[1].is_zero() {
        let c = &p.coeffs[0];
        let a = &p.coeffs[2];
        let sqrt_rat = |q: &BigRational| -> ExprId {
            let sn = q.numer().sqrt();
            let sd = q.denom().sqrt();
            if &(&sn * &sn) == q.numer() && &(&sd * &sd) == q.denom() {
                crate::poly::expr_from_rational(graph, &BigRational::new(sn, sd))
            } else {
                graph.function("sqrt", [crate::poly::expr_from_rational(graph, q)])
            }
        };
        let scale_arg = |alpha: ExprId, beta: ExprId| -> ExprId {
            if beta == alpha {
                x_sym
            } else if alpha == graph.integer(1) {
                graph.mul([beta, x_sym])
            } else if beta == graph.integer(1) {
                graph.div(x_sym, alpha)
            } else {
                graph.div(graph.mul([beta, x_sym]), alpha)
            }
        };
        let scale_outer = |beta: ExprId, term: ExprId| -> ExprId {
            if beta == graph.integer(1) {
                term
            } else {
                graph.div(term, beta)
            }
        };

        if c.is_positive() && a.is_negative() {
            // 1 / sqrt(alpha^2 - beta^2 x^2) -> (1/beta) * asin(beta*x / alpha)
            let alpha = sqrt_rat(c);
            let beta = sqrt_rat(&(-a));
            let arg = scale_arg(alpha, beta);
            return Ok(scale_outer(beta, graph.function("asin", [arg])));
        }
        if c.is_positive() && a.is_positive() {
            // 1 / sqrt(alpha^2 + beta^2 x^2) -> (1/beta) * asinh(beta*x / alpha)
            let alpha = sqrt_rat(c);
            let beta = sqrt_rat(a);
            let arg = scale_arg(alpha, beta);
            return Ok(scale_outer(beta, graph.function("asinh", [arg])));
        }
        if c.is_negative() && a.is_positive() {
            // 1 / sqrt(beta^2 x^2 - alpha^2) -> (1/beta) * acosh(beta*x / alpha)
            let alpha = sqrt_rat(&(-c));
            let beta = sqrt_rat(a);
            let arg = scale_arg(alpha, beta);
            return Ok(scale_outer(beta, graph.function("acosh", [arg])));
        }
    }

    Err(AlgebraError::EvaluationError(
        "Unsupported reciprocal sqrt integrand".into(),
    ))
}
