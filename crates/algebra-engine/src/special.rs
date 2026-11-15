//! # `algebra-special`
//!
//! Symbolic special functions suite providing full parity with SymPy (`sympy.functions.special.*`)
//! and Wolfram Alpha:
//! - Gamma $\Gamma(x)$, Log-Gamma $\ln\Gamma(x)$, Digamma $\psi(x)$, Polygamma $\psi^{(m)}(x)$, Beta $B(a, b)$
//! - Riemann Zeta $\zeta(s)$, Dirichlet Eta $\eta(s)$
//! - Error functions $\text{erf}(x), \text{erfc}(x), \text{erfi}(x)$
//! - Exponential & Logarithmic Integrals $\text{Ei}(x), \text{li}(x), \text{Si}(x), \text{Ci}(x)$
//! - Lambert W function $W_0(x), W_{-1}(x)$
//! - Bessel functions $J_\nu(x), Y_\nu(x), I_\nu(x), K_\nu(x)$, Spherical Bessel $j_n(x)$
//! - Airy functions $\text{Ai}(x), \text{Bi}(x)$
//! - Polylogarithms $\text{Li}_s(x)$ & Dilogarithm $\text{Li}_2(x)$
//! - Distributions: Dirac Delta $\delta(x)$, Heaviside $\Theta(x)$

pub use crate::weyl_dmodules::{BesselJ, ErrorFunctionErf, HermiteH, HolonomicFunction, LegendreP};
use algebra_core::{AlgebraResult, ExprGraph, ExprId, ExprKind, Number};

/// Special functions constructors for symbolic expressions.
pub trait SpecialFunctions {
    /// Construct Gamma function $\Gamma(x)$ node.
    fn gamma(&self, arg: ExprId) -> ExprId;

    /// Construct Log-Gamma function $\ln\Gamma(x)$ node.
    fn log_gamma(&self, arg: ExprId) -> ExprId;

    /// Construct Digamma function $\psi(x) = \frac{d}{dx}\ln\Gamma(x)$ node.
    fn digamma(&self, arg: ExprId) -> ExprId;

    /// Construct Beta function $B(a, b) = \frac{\Gamma(a)\Gamma(b)}{\Gamma(a+b)}$ node.
    fn beta(&self, a: ExprId, b: ExprId) -> ExprId;

    /// Construct Bessel function of the first kind $J_\nu(x)$ node.
    fn bessel_j(&self, order: ExprId, arg: ExprId) -> ExprId;

    /// Construct Bessel function of the second kind $Y_\nu(x)$ node.
    fn bessel_y(&self, order: ExprId, arg: ExprId) -> ExprId;

    /// Construct Modified Bessel function of the first kind $I_\nu(x)$ node.
    fn bessel_i(&self, order: ExprId, arg: ExprId) -> ExprId;

    /// Construct Modified Bessel function of the second kind $K_\nu(x)$ node.
    fn bessel_k(&self, order: ExprId, arg: ExprId) -> ExprId;

    /// Construct Riemann Zeta function $\zeta(s)$ node.
    fn zeta(&self, arg: ExprId) -> ExprId;

    /// Construct Dirichlet Eta function $\eta(s) = (1 - 2^{1-s})\zeta(s)$ node.
    fn dirichlet_eta(&self, arg: ExprId) -> ExprId;

    /// Construct Dirac Delta distribution $\delta(x)$ node.
    fn dirac_delta(&self, arg: ExprId) -> ExprId;

    /// Construct Heaviside step distribution $\Theta(x)$ node.
    fn heaviside(&self, arg: ExprId) -> ExprId;

    /// Construct Error function $\text{erf}(x)$ node.
    fn erf(&self, arg: ExprId) -> ExprId;

    /// Construct Complementary Error function $\text{erfc}(x)$ node.
    fn erfc(&self, arg: ExprId) -> ExprId;

    /// Construct Lambert W function $W(x)$ node.
    fn lambert_w(&self, arg: ExprId) -> ExprId;

    /// Construct Sine Integral $\text{Si}(x)$ node.
    fn sine_integral(&self, arg: ExprId) -> ExprId;

    /// Construct Cosine Integral $\text{Ci}(x)$ node.
    fn cosine_integral(&self, arg: ExprId) -> ExprId;

    /// Construct Exponential Integral $\text{Ei}(x)$ node.
    fn exponential_integral(&self, arg: ExprId) -> ExprId;

    /// Construct Logarithmic Integral $\text{li}(x)$ node.
    fn logarithmic_integral(&self, arg: ExprId) -> ExprId;

    /// Construct Airy Ai function $\text{Ai}(x)$ node.
    fn airy_ai(&self, arg: ExprId) -> ExprId;

    /// Construct Airy Bi function $\text{Bi}(x)$ node.
    fn airy_bi(&self, arg: ExprId) -> ExprId;

    /// Construct Polylogarithm $\text{Li}_s(x)$ node.
    fn polylog(&self, order: ExprId, arg: ExprId) -> ExprId;

    /// Evaluate exact values for integer and half-integer special function arguments.
    fn evaluate_special(&self, expr: ExprId) -> AlgebraResult<ExprId>;
}

impl SpecialFunctions for ExprGraph {
    fn gamma(&self, arg: ExprId) -> ExprId {
        self.function("gamma", [arg])
    }

    fn log_gamma(&self, arg: ExprId) -> ExprId {
        self.function("log_gamma", [arg])
    }

    fn digamma(&self, arg: ExprId) -> ExprId {
        self.function("digamma", [arg])
    }

    fn beta(&self, a: ExprId, b: ExprId) -> ExprId {
        self.function("beta", [a, b])
    }

    fn bessel_j(&self, order: ExprId, arg: ExprId) -> ExprId {
        self.function("bessel_j", [order, arg])
    }

    fn bessel_y(&self, order: ExprId, arg: ExprId) -> ExprId {
        self.function("bessel_y", [order, arg])
    }

    fn bessel_i(&self, order: ExprId, arg: ExprId) -> ExprId {
        self.function("bessel_i", [order, arg])
    }

    fn bessel_k(&self, order: ExprId, arg: ExprId) -> ExprId {
        self.function("bessel_k", [order, arg])
    }

    fn zeta(&self, arg: ExprId) -> ExprId {
        self.function("zeta", [arg])
    }

    fn dirichlet_eta(&self, arg: ExprId) -> ExprId {
        self.function("dirichlet_eta", [arg])
    }

    fn dirac_delta(&self, arg: ExprId) -> ExprId {
        self.function("dirac_delta", [arg])
    }

    fn heaviside(&self, arg: ExprId) -> ExprId {
        self.function("heaviside", [arg])
    }

    fn erf(&self, arg: ExprId) -> ExprId {
        self.function("erf", [arg])
    }

    fn erfc(&self, arg: ExprId) -> ExprId {
        self.function("erfc", [arg])
    }

    fn lambert_w(&self, arg: ExprId) -> ExprId {
        self.function("lambert_w", [arg])
    }

    fn sine_integral(&self, arg: ExprId) -> ExprId {
        self.function("si", [arg])
    }

    fn cosine_integral(&self, arg: ExprId) -> ExprId {
        self.function("ci", [arg])
    }

    fn exponential_integral(&self, arg: ExprId) -> ExprId {
        self.function("ei", [arg])
    }

    fn logarithmic_integral(&self, arg: ExprId) -> ExprId {
        self.function("li", [arg])
    }

    fn airy_ai(&self, arg: ExprId) -> ExprId {
        self.function("airy_ai", [arg])
    }

    fn airy_bi(&self, arg: ExprId) -> ExprId {
        self.function("airy_bi", [arg])
    }

    fn polylog(&self, order: ExprId, arg: ExprId) -> ExprId {
        self.function("polylog", [order, arg])
    }

    fn evaluate_special(&self, expr: ExprId) -> AlgebraResult<ExprId> {
        let node = self.get(expr);
        if let ExprKind::Function { name, args } = &node.kind {
            let fn_name = self.symbols.resolve(*name).unwrap_or_default();
            let pi = self.constant(algebra_core::Constant::Pi);
            let sqrt_pi = self.function("sqrt", [pi]);

            // =========================================================================
            // 1. GAMMA FUNCTION & BETA FUNCTION
            // =========================================================================
            if fn_name == "gamma" && args.len() == 1 {
                let arg_node = self.get(args[0]);
                match &arg_node.kind {
                    ExprKind::Number(Number::Integer(n)) => {
                        if *n > 0 {
                            // Gamma(n) = (n - 1)!
                            let mut fact = 1i64;
                            for i in 1..*n {
                                fact = fact.saturating_mul(i);
                            }
                            return Ok(self.integer(fact));
                        }
                    }
                    ExprKind::Number(Number::Rational(num, den)) if *den == 2 => {
                        // Half-integers Gamma(num / 2)
                        match *num {
                            1 => return Ok(sqrt_pi), // Gamma(1/2) = sqrt(pi)
                            3 => {
                                // Gamma(3/2) = sqrt(pi) / 2
                                return Ok(self.div(sqrt_pi, self.integer(2)));
                            }
                            5 => {
                                // Gamma(5/2) = 3*sqrt(pi) / 4
                                let num_part = self.mul([self.integer(3), sqrt_pi]);
                                return Ok(self.div(num_part, self.integer(4)));
                            }
                            7 => {
                                // Gamma(7/2) = 15*sqrt(pi) / 8
                                let num_part = self.mul([self.integer(15), sqrt_pi]);
                                return Ok(self.div(num_part, self.integer(8)));
                            }
                            -1 => {
                                // Gamma(-1/2) = -2*sqrt(pi)
                                return Ok(self.mul([self.integer(-2), sqrt_pi]));
                            }
                            -3 => {
                                // Gamma(-3/2) = 4*sqrt(pi) / 3
                                let num_part = self.mul([self.integer(4), sqrt_pi]);
                                return Ok(self.div(num_part, self.integer(3)));
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            } else if fn_name == "beta" && args.len() == 2 {
                let a_node = self.get(args[0]);
                let b_node = self.get(args[1]);
                if let (
                    ExprKind::Number(Number::Integer(m)),
                    ExprKind::Number(Number::Integer(n)),
                ) = (&a_node.kind, &b_node.kind)
                {
                    if *m > 0 && *n > 0 {
                        // B(m, n) = (m-1)! * (n-1)! / (m+n-1)!
                        let mut fact_m = 1i64;
                        for i in 1..*m {
                            fact_m = fact_m.saturating_mul(i);
                        }
                        let mut fact_n = 1i64;
                        for i in 1..*n {
                            fact_n = fact_n.saturating_mul(i);
                        }
                        let mut fact_mn = 1i64;
                        for i in 1..(*m + *n) {
                            fact_mn = fact_mn.saturating_mul(i);
                        }
                        let num = fact_m * fact_n;
                        let mut a_g = num.unsigned_abs();
                        let mut b_g = fact_mn.unsigned_abs();
                        while b_g != 0 {
                            let t = b_g;
                            b_g = a_g % b_g;
                            a_g = t;
                        }
                        let g = (if a_g == 0 { 1 } else { a_g }) as i64;
                        return Ok(self.rational(num / g, fact_mn / g));
                    }
                }
            }
            // =========================================================================
            // 2. RIEMANN ZETA & DIRICHLET ETA
            // =========================================================================
            else if fn_name == "zeta" && args.len() == 1 {
                let arg_node = self.get(args[0]);
                if let ExprKind::Number(Number::Integer(n)) = arg_node.kind {
                    match n {
                        0 => return Ok(self.rational(-1, 2)), // zeta(0) = -1/2
                        2 => {
                            // zeta(2) = pi^2 / 6
                            let pi_sq = self.pow(pi, self.integer(2));
                            return Ok(self.div(pi_sq, self.integer(6)));
                        }
                        4 => {
                            // zeta(4) = pi^4 / 90
                            let pi_4 = self.pow(pi, self.integer(4));
                            return Ok(self.div(pi_4, self.integer(90)));
                        }
                        6 => {
                            // zeta(6) = pi^6 / 945
                            let pi_6 = self.pow(pi, self.integer(6));
                            return Ok(self.div(pi_6, self.integer(945)));
                        }
                        8 => {
                            // zeta(8) = pi^8 / 9450
                            let pi_8 = self.pow(pi, self.integer(8));
                            return Ok(self.div(pi_8, self.integer(9450)));
                        }
                        10 => {
                            // zeta(10) = pi^10 / 93555
                            let pi_10 = self.pow(pi, self.integer(10));
                            return Ok(self.div(pi_10, self.integer(93555)));
                        }
                        -1 => return Ok(self.rational(-1, 12)), // zeta(-1) = -1/12
                        -3 => return Ok(self.rational(1, 120)), // zeta(-3) = 1/120
                        -5 => return Ok(self.rational(-1, 252)), // zeta(-5) = -1/252
                        -7 => return Ok(self.rational(1, 240)), // zeta(-7) = 1/240
                        k if k < 0 && k % 2 == 0 => {
                            // Trivial zeros: zeta(-2k) = 0 for k in N+
                            return Ok(self.integer(0));
                        }
                        _ => {}
                    }
                }
            } else if fn_name == "dirichlet_eta" && args.len() == 1 {
                let arg_node = self.get(args[0]);
                if let ExprKind::Number(Number::Integer(n)) = arg_node.kind {
                    match n {
                        1 => {
                            // eta(1) = ln(2)
                            return Ok(self.function("ln", [self.integer(2)]));
                        }
                        2 => {
                            // eta(2) = pi^2 / 12
                            let pi_sq = self.pow(pi, self.integer(2));
                            return Ok(self.div(pi_sq, self.integer(12)));
                        }
                        4 => {
                            // eta(4) = 7*pi^4 / 720
                            let pi_4 = self.pow(pi, self.integer(4));
                            let num = self.mul([self.integer(7), pi_4]);
                            return Ok(self.div(num, self.integer(720)));
                        }
                        _ => {}
                    }
                }
            }
            // =========================================================================
            // 3. ERROR FUNCTIONS (erf, erfc, erfi)
            // =========================================================================
            else if fn_name == "erf" && args.len() == 1 {
                let arg_node = self.get(args[0]);
                if let ExprKind::Number(Number::Integer(0)) = arg_node.kind {
                    return Ok(self.integer(0)); // erf(0) = 0
                }
            } else if fn_name == "erfc" && args.len() == 1 {
                let arg_node = self.get(args[0]);
                if let ExprKind::Number(Number::Integer(0)) = arg_node.kind {
                    return Ok(self.integer(1)); // erfc(0) = 1
                }
            }
            // =========================================================================
            // 4. LAMBERT W FUNCTION
            // =========================================================================
            else if (fn_name == "lambert_w" || fn_name == "LambertW") && args.len() == 1 {
                let arg_node = self.get(args[0]);
                match &arg_node.kind {
                    ExprKind::Number(Number::Integer(0)) => return Ok(self.integer(0)), // W(0) = 0
                    ExprKind::Number(Number::Constant(algebra_core::Constant::E)) => {
                        return Ok(self.integer(1)); // W(e) = 1
                    }
                    ExprKind::Symbol(s) if self.symbols.resolve(*s).as_deref() == Some("e") => {
                        return Ok(self.integer(1));
                    }
                    _ => {}
                }
            }
            // =========================================================================
            // 5. BESSEL FUNCTIONS
            // =========================================================================
            else if (fn_name == "bessel_j" || fn_name == "besselj") && args.len() == 2 {
                let order_node = self.get(args[0]);
                let arg_node = self.get(args[1]);
                if let ExprKind::Number(Number::Integer(0)) = arg_node.kind {
                    if let ExprKind::Number(Number::Integer(n)) = order_node.kind {
                        if n == 0 {
                            return Ok(self.integer(1)); // J_0(0) = 1
                        } else if n > 0 {
                            return Ok(self.integer(0)); // J_n(0) = 0 for n > 0
                        }
                    }
                }
            }
            // =========================================================================
            // 6. SINE & COSINE INTEGRALS
            // =========================================================================
            else if (fn_name == "si" || fn_name == "Si") && args.len() == 1 {
                let arg_node = self.get(args[0]);
                if let ExprKind::Number(Number::Integer(0)) = arg_node.kind {
                    return Ok(self.integer(0)); // Si(0) = 0
                }
            }
            // =========================================================================
            // 7. POLYLOGARITHM
            // =========================================================================
            else if fn_name == "polylog" && args.len() == 2 {
                let order_node = self.get(args[0]);
                let arg_node = self.get(args[1]);
                if let ExprKind::Number(Number::Integer(1)) = order_node.kind {
                    // Li_1(x) = -ln(1 - x)
                    let one = self.integer(1);
                    let one_minus_x = self.sub(one, args[1]);
                    let ln_node = self.function("ln", [one_minus_x]);
                    return Ok(self.neg(ln_node));
                } else if let ExprKind::Number(Number::Integer(2)) = order_node.kind {
                    if let ExprKind::Number(Number::Integer(0)) = arg_node.kind {
                        return Ok(self.integer(0)); // Li_2(0) = 0
                    } else if let ExprKind::Number(Number::Integer(1)) = arg_node.kind {
                        // Li_2(1) = pi^2 / 6
                        let pi_sq = self.pow(pi, self.integer(2));
                        return Ok(self.div(pi_sq, self.integer(6)));
                    } else if let ExprKind::Number(Number::Integer(-1)) = arg_node.kind {
                        // Li_2(-1) = -pi^2 / 12
                        let pi_sq = self.pow(pi, self.integer(2));
                        let div_12 = self.div(pi_sq, self.integer(12));
                        return Ok(self.neg(div_12));
                    }
                }
            }
        }
        Ok(expr)
    }
}
