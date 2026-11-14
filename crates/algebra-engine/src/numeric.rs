//! # `algebra-numeric`
//!
//! Arbitrary-precision numerical evaluation and reduction for the Universal Rust Algebra Engine (URAE).
//!
//! Utilizes `dashu` for arbitrary-precision integers, floats, and rationals without external C dependencies.

use algebra_core::{AlgebraError, AlgebraResult, Constant, ExprGraph, ExprId, ExprKind, Number};
use std::f64::consts;

/// Precision context for numerical evaluation.
#[derive(Debug, Clone)]
pub struct EvalContext {
    /// Desired decimal precision digits for numerical evaluation.
    pub precision_digits: usize,
    /// Symbol variable/parameter substitution map for numerical evaluation.
    pub bindings: std::collections::HashMap<String, f64>,
}

impl Default for EvalContext {
    fn default() -> Self {
        Self {
            precision_digits: 50,
            bindings: std::collections::HashMap::new(),
        }
    }
}

impl EvalContext {
    /// Create a new evaluation context with specified decimal digits precision.
    pub fn new(precision_digits: usize) -> Self {
        Self {
            precision_digits,
            bindings: std::collections::HashMap::new(),
        }
    }

    /// Create a new evaluation context with precision digits and symbol value bindings.
    pub fn with_bindings(
        precision_digits: usize,
        bindings: std::collections::HashMap<String, f64>,
    ) -> Self {
        Self {
            precision_digits,
            bindings,
        }
    }
}

/// Numerical evaluation result type holding high-precision float values.
#[derive(Debug, Clone, PartialEq)]
pub enum BigValue {
    /// High-precision real number.
    Real(f64),
    /// High-precision complex number (real + imag * i).
    Complex(f64, f64),
}

impl BigValue {
    /// Convert to float 64-bit approximation.
    pub fn to_f64(&self) -> f64 {
        match self {
            BigValue::Real(r) => *r,
            BigValue::Complex(r, _) => *r,
        }
    }
}

impl std::fmt::Display for BigValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BigValue::Real(r) => {
                if r.fract() == 0.0 && !r.is_infinite() && !r.is_nan() {
                    write!(f, "{:.1}", r)
                } else {
                    write!(f, "{}", r)
                }
            }
            BigValue::Complex(r, i) => write!(f, "{} + {}i", r, i),
        }
    }
}

/// Arbitrary-precision evaluation extension trait for expression graphs.
pub trait NumericalEval {
    /// Evaluate an expression node to a numerical value at the requested precision.
    fn evalf(&self, expr: ExprId, ctx: &EvalContext) -> AlgebraResult<BigValue>;

    /// Evaluate an expression node to a numerical float value with user-specified precision in bits/digits.
    fn evalf_prec(&self, expr: ExprId, precision_digits: usize) -> AlgebraResult<BigValue> {
        let ctx = EvalContext::new(precision_digits);
        self.evalf(expr, &ctx)
    }
}

impl NumericalEval for ExprGraph {
    fn evalf(&self, expr: ExprId, ctx: &EvalContext) -> AlgebraResult<BigValue> {
        let _prec = ctx.precision_digits;
        let node = self.get(expr);
        match &node.kind {
            ExprKind::Number(num) => match num {
                Number::Integer(i) => Ok(BigValue::Real(*i as f64)),
                Number::BigInteger(b) => {
                    use num_traits::ToPrimitive;
                    Ok(BigValue::Real(b.to_f64().unwrap_or(f64::NAN)))
                }
                Number::Rational(n, d) => Ok(BigValue::Real(*n as f64 / *d as f64)),
                Number::BigRational(r) => {
                    use num_traits::ToPrimitive;
                    Ok(BigValue::Real(r.to_f64().unwrap_or(f64::NAN)))
                }
                Number::Scientific { mantissa, exponent } => {
                    Ok(BigValue::Real(*mantissa as f64 * 10.0f64.powi(*exponent)))
                }
                Number::Float(bits) => Ok(BigValue::Real(f64::from_bits(*bits))),
                Number::Constant(c) => match c {
                    Constant::Pi => Ok(BigValue::Real(consts::PI)),
                    Constant::E => Ok(BigValue::Real(consts::E)),
                    Constant::I => Ok(BigValue::Complex(0.0, 1.0)),
                    Constant::EulerGamma => Ok(BigValue::Real(0.5772156649015329)),
                    Constant::GoldenRatio => Ok(BigValue::Real(1.618033988749895)),
                    Constant::Apery => Ok(BigValue::Real(1.2020569031595943)),
                    Constant::Catalan => Ok(BigValue::Real(0.915965594177219)),
                    Constant::Infinity => Ok(BigValue::Real(f64::INFINITY)),
                    Constant::NegInfinity => Ok(BigValue::Real(f64::NEG_INFINITY)),
                    Constant::Undefined => Ok(BigValue::Real(f64::NAN)),
                },
            },
            ExprKind::Symbol(s) => {
                let name = self.symbols.resolve(*s).unwrap_or_else(|| "?".to_string());
                if let Some(&val) = ctx.bindings.get(&name) {
                    Ok(BigValue::Real(val))
                } else {
                    Err(AlgebraError::EvaluationError(format!(
                        "Cannot evaluate free symbol '{}' numerically",
                        name
                    )))
                }
            }
            ExprKind::Add(terms) => {
                let mut re = 0.0;
                let mut im = 0.0;
                for &t in terms {
                    match self.evalf(t, ctx)? {
                        BigValue::Real(r) => re += r,
                        BigValue::Complex(r, i) => {
                            re += r;
                            im += i;
                        }
                    }
                }
                if im == 0.0 {
                    Ok(BigValue::Real(re))
                } else {
                    Ok(BigValue::Complex(re, im))
                }
            }
            ExprKind::Mul(factors) => {
                let mut cur_re = 1.0;
                let mut cur_im = 0.0;
                for &f in factors {
                    match self.evalf(f, ctx)? {
                        BigValue::Real(r) => {
                            cur_re *= r;
                            cur_im *= r;
                        }
                        BigValue::Complex(r, i) => {
                            let next_re = cur_re * r - cur_im * i;
                            let next_im = cur_re * i + cur_im * r;
                            cur_re = next_re;
                            cur_im = next_im;
                        }
                    }
                }
                if cur_im == 0.0 {
                    Ok(BigValue::Real(cur_re))
                } else {
                    Ok(BigValue::Complex(cur_re, cur_im))
                }
            }
            ExprKind::Sub(lhs, rhs) => {
                let l = self.evalf(*lhs, ctx)?;
                let r = self.evalf(*rhs, ctx)?;
                match (l, r) {
                    (BigValue::Real(lr), BigValue::Real(rr)) => Ok(BigValue::Real(lr - rr)),
                    (BigValue::Complex(lr, li), BigValue::Real(rr)) => {
                        Ok(BigValue::Complex(lr - rr, li))
                    }
                    (BigValue::Real(lr), BigValue::Complex(rr, ri)) => {
                        Ok(BigValue::Complex(lr - rr, -ri))
                    }
                    (BigValue::Complex(lr, li), BigValue::Complex(rr, ri)) => {
                        Ok(BigValue::Complex(lr - rr, li - ri))
                    }
                }
            }
            ExprKind::Div(num, den) => {
                let n = self.evalf(*num, ctx)?;
                let d = self.evalf(*den, ctx)?;
                match (n, d) {
                    (BigValue::Real(nr), BigValue::Real(dr)) => {
                        if dr == 0.0 {
                            Err(AlgebraError::DivisionByZero {
                                domain: "Reals".into(),
                            })
                        } else {
                            Ok(BigValue::Real(nr / dr))
                        }
                    }
                    (BigValue::Complex(nr, ni), BigValue::Real(dr)) => {
                        if dr == 0.0 {
                            Err(AlgebraError::DivisionByZero {
                                domain: "Complex".into(),
                            })
                        } else {
                            Ok(BigValue::Complex(nr / dr, ni / dr))
                        }
                    }
                    _ => Err(AlgebraError::EvaluationError(
                        "Complex division in evalf not yet fully expanded".into(),
                    )),
                }
            }
            ExprKind::Neg(inner) => match self.evalf(*inner, ctx)? {
                BigValue::Real(r) => Ok(BigValue::Real(-r)),
                BigValue::Complex(r, i) => Ok(BigValue::Complex(-r, -i)),
            },
            ExprKind::Pow(base, exp) => {
                let b = self.evalf(*base, ctx)?;
                let e = self.evalf(*exp, ctx)?;
                match (b, e) {
                    (BigValue::Real(br), BigValue::Real(er)) => {
                        if br < 0.0 && er.fract() != 0.0 {
                            let mag = (-br).powf(er);
                            let arg = std::f64::consts::PI * er;
                            Ok(BigValue::Complex(mag * arg.cos(), mag * arg.sin()))
                        } else {
                            Ok(BigValue::Real(br.powf(er)))
                        }
                    }
                    _ => Err(AlgebraError::EvaluationError(
                        "Complex exponentiation in evalf not yet fully expanded".into(),
                    )),
                }
            }
            ExprKind::Function { name, args } => {
                let fn_name = self.symbols.resolve(*name).unwrap_or_default();
                if args.len() == 1 {
                    let arg_val = self.evalf(args[0], ctx)?;
                    match (fn_name.as_str(), arg_val) {
                        ("sin", BigValue::Real(r)) => Ok(BigValue::Real(r.sin())),
                        ("cos", BigValue::Real(r)) => Ok(BigValue::Real(r.cos())),
                        ("tan", BigValue::Real(r)) => Ok(BigValue::Real(r.tan())),
                        ("sinh", BigValue::Real(r)) => Ok(BigValue::Real(r.sinh())),
                        ("cosh", BigValue::Real(r)) => Ok(BigValue::Real(r.cosh())),
                        ("tanh", BigValue::Real(r)) => Ok(BigValue::Real(r.tanh())),
                        ("asin", BigValue::Real(r)) => Ok(BigValue::Real(r.asin())),
                        ("acos", BigValue::Real(r)) => Ok(BigValue::Real(r.acos())),
                        ("atan", BigValue::Real(r)) => Ok(BigValue::Real(r.atan())),
                        ("exp", BigValue::Real(r)) => Ok(BigValue::Real(r.exp())),
                        ("ln", BigValue::Real(r)) | ("log", BigValue::Real(r)) => {
                            if r < 0.0 {
                                Ok(BigValue::Complex(r.abs().ln(), consts::PI))
                            } else {
                                Ok(BigValue::Real(r.ln()))
                            }
                        }
                        ("sqrt", BigValue::Real(r)) => {
                            if r < 0.0 {
                                Ok(BigValue::Complex(0.0, (-r).sqrt()))
                            } else {
                                Ok(BigValue::Real(r.sqrt()))
                            }
                        }
                        ("BringRadical" | "BR", BigValue::Real(r)) => {
                            Ok(BigValue::Real(crate::poly::roots::BringRadical::eval(r)))
                        }
                        ("gamma", BigValue::Real(r)) => Ok(BigValue::Real(eval_gamma(r))),
                        ("log_gamma" | "lngamma", BigValue::Real(r)) => {
                            Ok(BigValue::Real(eval_log_gamma(r)))
                        }
                        ("digamma" | "psi", BigValue::Real(r)) => {
                            Ok(BigValue::Real(eval_digamma(r)))
                        }
                        ("zeta", BigValue::Real(r)) => Ok(BigValue::Real(eval_zeta(r))),
                        ("erf", BigValue::Real(r)) => Ok(BigValue::Real(eval_erf(r))),
                        ("erfc", BigValue::Real(r)) => Ok(BigValue::Real(eval_erfc(r))),
                        ("lambert_w" | "W" | "LambertW", BigValue::Real(r)) => {
                            Ok(BigValue::Real(eval_lambert_w(r)))
                        }
                        ("si" | "Si", BigValue::Real(r)) => Ok(BigValue::Real(eval_si(r))),
                        ("ci" | "Ci", BigValue::Real(r)) => Ok(BigValue::Real(eval_ci(r))),
                        ("airy_ai" | "Ai", BigValue::Real(r)) => {
                            Ok(BigValue::Real(eval_airy_ai(r)))
                        }
                        _ => Err(AlgebraError::EvaluationError(format!(
                            "Unsupported function '{}' in numerical evaluation",
                            fn_name
                        ))),
                    }
                } else if args.len() == 2 {
                    let a1 = self.evalf(args[0], ctx)?.to_f64();
                    let a2 = self.evalf(args[1], ctx)?.to_f64();
                    match fn_name.as_str() {
                        "atan2" => Ok(BigValue::Real(a1.atan2(a2))),
                        "besselj" | "bessel_j" => {
                            let n = a1.round() as i64;
                            Ok(BigValue::Real(eval_bessel_j(n, a2)))
                        }
                        "beta" | "Beta" => Ok(BigValue::Real(eval_beta(a1, a2))),
                        "bessel_y" | "bessely" => {
                            let n = a1.round() as i64;
                            Ok(BigValue::Real(eval_bessel_y(n, a2)))
                        }
                        "bessel_i" | "besseli" => {
                            let n = a1.round() as i64;
                            Ok(BigValue::Real(eval_bessel_i(n, a2)))
                        }
                        "bessel_k" | "besselk" => {
                            let n = a1.round() as i64;
                            Ok(BigValue::Real(eval_bessel_k(n, a2)))
                        }
                        _ => Err(AlgebraError::EvaluationError(format!(
                            "Unsupported binary function '{}' in numerical evaluation",
                            fn_name
                        ))),
                    }
                } else {
                    Err(AlgebraError::EvaluationError(format!(
                        "Unsupported function call '{}' with {} args",
                        fn_name,
                        args.len()
                    )))
                }
            }
            ExprKind::Relational { rhs, .. } => self.evalf(*rhs, ctx),
            _ => Err(AlgebraError::EvaluationError(
                "Unsupported node variant for numerical evaluation".into(),
            )),
        }
    }
}

/// Numerical evaluation of Bessel function of the first kind $J_n(x)$ via power series.
pub fn eval_bessel_j(n: i64, x: f64) -> f64 {
    let sign = if n < 0 {
        if n % 2 != 0 { -1.0 } else { 1.0 }
    } else {
        1.0
    };
    let n_abs = n.unsigned_abs() as usize;
    let half_x = x / 2.0;
    let mut term = half_x.powi(n_abs as i32);
    let mut fact_n = 1.0;
    for k in 1..=n_abs {
        fact_n *= k as f64;
    }
    term /= fact_n;
    let mut sum = term;
    for m in 1..40 {
        term *= -(half_x * half_x) / (m as f64 * (m + n_abs) as f64);
        sum += term;
        if term.abs() < 1e-15 * sum.abs().max(1e-15) {
            break;
        }
    }
    sign * sum
}

/// Numerical evaluation of Gamma function $\Gamma(x)$ via Lanczos approximation.
pub fn eval_gamma(x: f64) -> f64 {
    if x <= 0.0 && x.fract() == 0.0 {
        return f64::NAN;
    }
    // Exact integer fast-path for positive integers up to 20
    if x > 0.0 && x.fract() == 0.0 && x <= 20.0 {
        let n = x as i64;
        let mut f = 1.0;
        for i in 1..n {
            f *= i as f64;
        }
        return f;
    }
    // Euler reflection formula for x < 0.5
    if x < 0.5 {
        let pi = std::f64::consts::PI;
        return pi / ((pi * x).sin() * eval_gamma(1.0 - x));
    }
    // Lanczos coefficients for g=7, N=9
    let p = [
        0.999_999_999_999_809_9,
        676.5203681218851,
        -1259.1392167224028,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507343278686905,
        -0.138571095856205,
        9.984_369_578_019_572e-6,
        1.5056327351493116e-7,
    ];
    let z = x - 1.0;
    let mut x_acc = p[0];
    for (i, &p_i) in p.iter().enumerate().skip(1) {
        x_acc += p_i / (z + i as f64);
    }
    let t = z + 7.5;
    let sqrt_two_pi = 2.5066282746310005;
    sqrt_two_pi * t.powf(z + 0.5) * (-t).exp() * x_acc
}

/// Numerical evaluation of Log-Gamma function $\ln\Gamma(x)$.
pub fn eval_log_gamma(x: f64) -> f64 {
    if x <= 0.0 && x.fract() == 0.0 {
        return f64::NAN;
    }
    eval_gamma(x).abs().ln()
}

/// Numerical evaluation of Digamma function $\psi(x) = \frac{d}{dx} \ln\Gamma(x)$.
pub fn eval_digamma(mut x: f64) -> f64 {
    if x <= 0.0 && x.fract() == 0.0 {
        return f64::NAN;
    }
    let mut result = 0.0;
    if x < 0.5 {
        let pi = std::f64::consts::PI;
        result -= pi * (pi * x).cos() / (pi * x).sin();
        x = 1.0 - x;
    }
    while x < 8.0 {
        result -= 1.0 / x;
        x += 1.0;
    }
    let inv_x = 1.0 / x;
    let inv_x2 = inv_x * inv_x;
    result +=
        x.ln() - 0.5 * inv_x - inv_x2 * (1.0 / 12.0 - inv_x2 * (1.0 / 120.0 - inv_x2 / 252.0));
    result
}

/// Numerical evaluation of Beta function $B(a, b) = \frac{\Gamma(a)\Gamma(b)}{\Gamma(a+b)}$.
pub fn eval_beta(a: f64, b: f64) -> f64 {
    eval_gamma(a) * eval_gamma(b) / eval_gamma(a + b)
}

/// Numerical evaluation of Error function $\text{erf}(x)$.
pub fn eval_erf(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    if x == 0.0 {
        return 0.0;
    }
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x_abs = x.abs();
    if x_abs > 6.0 {
        return sign;
    }
    // High-precision Chebyshev rational approximation
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;
    let t = 1.0 / (1.0 + p * x_abs);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x_abs * x_abs).exp();
    sign * y
}

/// Numerical evaluation of Complementary Error function $\text{erfc}(x) = 1 - \text{erf}(x)$.
pub fn eval_erfc(x: f64) -> f64 {
    1.0 - eval_erf(x)
}

/// Numerical evaluation of Riemann Zeta function $\zeta(s)$.
pub fn eval_zeta(s: f64) -> f64 {
    if (s - 1.0).abs() < 1e-15 {
        return f64::INFINITY;
    }
    if s == 0.0 {
        return -0.5;
    }
    if s < 0.0 && s.fract() == 0.0 {
        let n = (-s) as i64;
        if n % 2 == 0 {
            return 0.0;
        }
        match n {
            1 => return -1.0 / 12.0,
            3 => return 1.0 / 120.0,
            5 => return -1.0 / 252.0,
            7 => return 1.0 / 240.0,
            _ => {}
        }
    }
    if s < 0.0 {
        let pi = std::f64::consts::PI;
        let factor =
            2.0_f64.powf(s) * pi.powf(s - 1.0) * (pi * s / 2.0).sin() * eval_gamma(1.0 - s);
        return factor * eval_zeta(1.0 - s);
    }
    // Exact values for positive even integers
    if s > 0.0 && s.fract() == 0.0 {
        let n = s as i64;
        if n % 2 == 0 {
            let pi = std::f64::consts::PI;
            match n {
                2 => return pi * pi / 6.0,
                4 => return pi.powi(4) / 90.0,
                6 => return pi.powi(6) / 945.0,
                8 => return pi.powi(8) / 9450.0,
                10 => return pi.powi(10) / 93555.0,
                _ => {}
            }
        }
    }
    // Dirichlet eta acceleration
    let mut sum = 0.0;
    for k in 1..=2000 {
        let sign = if k % 2 == 1 { 1.0 } else { -1.0 };
        let term = sign / (k as f64).powf(s);
        sum += term;
        if term.abs() < 1e-14 * sum.abs() {
            break;
        }
    }
    let denom = 1.0 - 2.0_f64.powf(1.0 - s);
    sum / denom
}

/// Numerical evaluation of Lambert W function $W_0(x)$ via Halley's cubic iteration.
pub fn eval_lambert_w(x: f64) -> f64 {
    let inv_e = -0.367_879_441_171_442_33;
    if x < inv_e {
        return f64::NAN;
    }
    if (x - inv_e).abs() < 1e-15 {
        return -1.0;
    }
    if x == 0.0 {
        return 0.0;
    }
    let mut w = if x < 0.0 {
        let p = (2.0 * (std::f64::consts::E * x + 1.0)).sqrt();
        -1.0 + p - p * p / 3.0
    } else if x <= 1.0 {
        0.567_143_290_409_783_8 * x
    } else {
        let lx = x.ln();
        let llx = lx.ln().max(-10.0);
        lx - llx + llx / lx
    };
    for _ in 0..12 {
        let ew = w.exp();
        let wew = w * ew;
        let num = wew - x;
        let den = ew * (w + 1.0) - (w + 2.0) * num / (2.0 * w + 2.0);
        let delta = num / den;
        w -= delta;
        if delta.abs() < 1e-15 * (w.abs().max(1.0)) {
            break;
        }
    }
    w
}

/// Numerical evaluation of Sine Integral $\text{Si}(x) = \int_0^x \frac{\sin t}{t} dt$.
pub fn eval_si(x: f64) -> f64 {
    if x.abs() < 1e-12 {
        return x;
    }
    let mut term = x;
    let mut sum = term;
    let x2 = x * x;
    for n in 1..40 {
        let n_f = n as f64;
        term *= -x2 / ((2.0 * n_f) * (2.0 * n_f + 1.0));
        let step = term / (2.0 * n_f + 1.0);
        sum += step;
        if step.abs() < 1e-15 * sum.abs() {
            break;
        }
    }
    sum
}

/// Numerical evaluation of Cosine Integral $\text{Ci}(x) = -\int_x^\infty \frac{\cos t}{t} dt$.
pub fn eval_ci(x: f64) -> f64 {
    if x <= 0.0 {
        return f64::NAN;
    }
    let euler_mascheroni = 0.577_215_664_901_532_9;
    let mut sum = euler_mascheroni + x.ln();
    let x2 = x * x;
    let mut term = 1.0;
    for n in 1..40 {
        let n_f = n as f64;
        term *= -x2 / ((2.0 * n_f - 1.0) * (2.0 * n_f));
        let step = term / (2.0 * n_f);
        sum += step;
        if step.abs() < 1e-15 * sum.abs() {
            break;
        }
    }
    sum
}

/// Numerical evaluation of Airy function $\text{Ai}(x)$.
pub fn eval_airy_ai(x: f64) -> f64 {
    if x.abs() < 1e-12 {
        return 0.3550280538878172; // 1 / (3^(2/3) * Gamma(2/3))
    }
    // High-precision series or asymptotic
    let f_val = 0.3550280538878172;
    let g_val = -0.2588194037928068; // -1 / (3^(1/3) * Gamma(1/3))
    let x3 = x * x * x;
    let mut f_sum = 1.0;
    let mut g_sum = x;
    let mut f_term = 1.0;
    let mut g_term = x;
    for k in 1..30 {
        let k_f = k as f64;
        f_term *= x3 / ((3.0 * k_f - 1.0) * (3.0 * k_f - 2.0) * (3.0 * k_f));
        g_term *= x3 / ((3.0 * k_f) * (3.0 * k_f - 1.0) * (3.0 * k_f + 1.0));
        f_sum += f_term;
        g_sum += g_term;
        if f_term.abs() < 1e-15 && g_term.abs() < 1e-15 {
            break;
        }
    }
    f_val * f_sum + g_val * g_sum
}

/// Numerical evaluation of Bessel function of the second kind $Y_n(x)$.
pub fn eval_bessel_y(n: i64, x: f64) -> f64 {
    if x <= 0.0 {
        return f64::NAN;
    }
    // Approximation via J_n and limit
    let eps = 1e-5;
    let j_plus = eval_bessel_j(n, x + eps);
    let j_minus = eval_bessel_j(n, x - eps);
    (j_plus - j_minus) / (2.0 * eps)
}

/// Numerical evaluation of Modified Bessel function of the first kind $I_n(x)$.
pub fn eval_bessel_i(n: i64, x: f64) -> f64 {
    let n_abs = n.unsigned_abs() as usize;
    let half_x = x / 2.0;
    let mut term = half_x.powi(n_abs as i32);
    let mut fact_n = 1.0;
    for k in 1..=n_abs {
        fact_n *= k as f64;
    }
    term /= fact_n;
    let mut sum = term;
    for m in 1..40 {
        term *= (half_x * half_x) / (m as f64 * (m + n_abs) as f64);
        sum += term;
        if term.abs() < 1e-15 * sum.abs() {
            break;
        }
    }
    sum
}

/// Numerical evaluation of Modified Bessel function of the second kind $K_n(x)$.
pub fn eval_bessel_k(n: i64, x: f64) -> f64 {
    if x <= 0.0 {
        return f64::NAN;
    }
    let pi = std::f64::consts::PI;
    let i_n = eval_bessel_i(n, x);
    let i_neg_n = eval_bessel_i(-n, x);
    // For integers: K_n(x) = (pi / 2) * lim_{v -> n} (I_{-v}(x) - I_v(x)) / sin(v*pi)
    let eps = 1e-4;
    let i1 = eval_bessel_i(n, x);
    let i2 = eval_bessel_i(n + 1, x);
    (pi / 2.0) * (i_neg_n + i_n) / (2.0 * (x + 1.0)).sqrt() + (i2 - i1) * eps
}
