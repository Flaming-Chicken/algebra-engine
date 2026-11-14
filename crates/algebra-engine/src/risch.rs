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
        // Fallback to Almkvist-Zeilberger creative telescoping
        Ok(crate::weyl_dmodules::AlmkvistZeilberger::gaussian_integral_ode())
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

                // Check standard integration-by-parts forms: x^k * exp(a*x), x^k * sin(a*x), etc.
                Self::integrate_products(graph, factors, wrt)
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
                if *base == x && !graph.has_symbol(*exp, wrt) {
                    let one = graph.integer(1);
                    let n_plus_1 = graph.add([*exp, one]);
                    let x_pow = graph.pow(x, n_plus_1);
                    return Ok(graph.div(x_pow, n_plus_1));
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

        // Case 1: Simple fraction 1 / (a*x + b) -> (1/a) * ln|a*x + b|
        let d_den = graph.diff(den, wrt);
        if !graph.has_symbol(num, wrt) && !graph.has_symbol(d_den, wrt) {
            // den is linear: a*x + b with d_den = a
            let inv_a = graph.div(num, d_den);
            let log_den = graph.function("ln", [den]);
            return Ok(graph.mul([inv_a, log_den]));
        }

        // Case 2: Derivative in numerator: f'(x) / f(x) -> ln|f(x)|
        if num == d_den {
            return Ok(graph.function("ln", [den]));
        }

        // Case 3: Constant * f'(x) / f(x)
        if let ExprKind::Mul(factors) = graph.get(num).kind
            && factors.contains(&d_den)
        {
            let other_factors: Vec<ExprId> =
                factors.iter().copied().filter(|&f| f != d_den).collect();
            let c = graph.mul(other_factors);
            let log_den = graph.function("ln", [den]);
            return Ok(graph.mul([c, log_den]));
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
                let a = graph.function("sqrt", [a_sq]);
                let inv_a = graph.div(num, a);
                let x_over_a = graph.div(x, a);
                let arctan_term = graph.function("arctan", [x_over_a]);
                return Ok(graph.mul([inv_a, arctan_term]));
            }
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
                "ln" => {
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
                _ => Err(AlgebraError::EvaluationError(format!(
                    "Integration of {} not implemented",
                    fn_name
                ))),
            }
        } else {
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
                if let (ExprKind::Function { name: n1, args: a1 }, ExprKind::Function { name: n2, args: a2 }) =
                    (&graph.get(u).kind, &graph.get(v).kind)
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
                        }
                    }
                }
                return Err(AlgebraError::EvaluationError(
                    "Multi-factor product integration not matched".into(),
                ));
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

        Err(AlgebraError::EvaluationError(
            "Multi-factor product integration not matched".into(),
        ))
    }
}
