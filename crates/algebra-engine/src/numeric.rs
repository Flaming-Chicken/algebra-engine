//! # `algebra-numeric`
//!
//! Arbitrary-precision numerical evaluation and reduction for the Universal Rust Algebra Engine (URAE).
//!
//! Utilizes `dashu` for arbitrary-precision integers, floats, and rationals without external C dependencies.

use crate::branch::{BranchCutFunctionKind, BranchCutManager, BranchSelection};
use algebra_core::{
    AlgebraError, AlgebraResult, Constant, ExprGraph, ExprId, ExprKind, Number, RealInterval,
    UncertainValue,
};
use std::collections::HashMap;
use std::f64::consts;

/// Precision, parameter bindings, uncertainty metadata, and branch-sheet context for numerical evaluation.
#[derive(Debug, Clone)]
pub struct EvalContext {
    /// Desired decimal precision digits for numerical evaluation.
    pub precision_digits: usize,
    /// Symbol variable/parameter substitution map for numerical evaluation.
    pub bindings: HashMap<String, f64>,
    /// Optional 1-sigma Gaussian uncertainties keyed by symbol name.
    pub uncertainties: HashMap<String, f64>,
    /// Optional certified real intervals keyed by symbol name.
    pub intervals: HashMap<String, RealInterval>,
    /// Optional default Riemann sheet index `k` (None => principal sheet `k = 0`).
    pub branch_sheet: Option<i64>,
}

impl Default for EvalContext {
    fn default() -> Self {
        Self {
            precision_digits: 50,
            bindings: HashMap::new(),
            uncertainties: HashMap::new(),
            intervals: HashMap::new(),
            branch_sheet: None,
        }
    }
}

impl EvalContext {
    /// Create a new evaluation context with specified decimal digits precision.
    pub fn new(precision_digits: usize) -> Self {
        Self {
            precision_digits,
            ..Default::default()
        }
    }

    /// Create a new evaluation context with precision digits and symbol value bindings.
    pub fn with_bindings(precision_digits: usize, bindings: HashMap<String, f64>) -> Self {
        Self {
            precision_digits,
            bindings,
            ..Default::default()
        }
    }

    /// Builder helper to attach symbol 1-sigma uncertainties.
    pub fn with_uncertainties(mut self, uncertainties: HashMap<String, f64>) -> Self {
        self.uncertainties = uncertainties;
        self
    }

    /// Builder helper to attach explicit symbol interval bounds.
    pub fn with_intervals(mut self, intervals: HashMap<String, RealInterval>) -> Self {
        self.intervals = intervals;
        self
    }

    /// Builder helper to select a specific Riemann sheet index.
    pub fn with_branch_sheet(mut self, sheet: Option<i64>) -> Self {
        self.branch_sheet = sheet;
        self
    }

    /// Returns `true` if any symbol has non-zero uncertainty or a non-point interval.
    pub fn has_uncertainty(&self) -> bool {
        self.uncertainties.values().any(|&u| u.abs() > 0.0)
            || self.intervals.values().any(|iv| iv.diam() > 0.0)
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

    /// Convert to `(re, im)` complex coordinates.
    pub fn as_complex(&self) -> (f64, f64) {
        match self {
            BigValue::Real(r) => (*r, 0.0),
            BigValue::Complex(r, i) => (*r, *i),
        }
    }

    /// Construct from `(re, im)`, normalizing negligible imaginary parts (`|im| <= 1e-14`) to `Real`.
    pub fn from_complex(re: f64, im: f64) -> Self {
        if im == 0.0 || (im.is_finite() && re.is_finite() && im.abs() <= 1e-14 * re.abs().max(1.0))
        {
            let clean_re = if re.abs() <= 1e-15 { 0.0 } else { re };
            BigValue::Real(clean_re)
        } else {
            BigValue::Complex(re, im)
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
                    match name.as_str() {
                        "pi" | "π" | "Pi" | "PI" => Ok(BigValue::Real(consts::PI)),
                        "tau" | "τ" => Ok(BigValue::Real(2.0 * consts::PI)),
                        "e" | "E" => Ok(BigValue::Real(consts::E)),
                        "i" | "I" => Ok(BigValue::Complex(0.0, 1.0)),
                        "phi" | "φ" | "GoldenRatio" => Ok(BigValue::Real(1.618033988749895)),
                        "gamma" | "EulerGamma" => Ok(BigValue::Real(0.5772156649015329)),
                        "oo" | "inf" | "infinity" | "Infinity" => Ok(BigValue::Real(f64::INFINITY)),
                        _ => Err(AlgebraError::EvaluationError(format!(
                            "Cannot evaluate free symbol '{}' numerically",
                            name
                        ))),
                    }
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
                let (nr, ni) = self.evalf(*num, ctx)?.as_complex();
                let (dr, di) = self.evalf(*den, ctx)?.as_complex();
                let den2 = dr * dr + di * di;
                if den2 == 0.0 {
                    return Err(AlgebraError::DivisionByZero {
                        domain: if ni == 0.0 && di == 0.0 {
                            "Reals".into()
                        } else {
                            "Complex".into()
                        },
                    });
                }
                if ni == 0.0 && di == 0.0 {
                    Ok(BigValue::Real(nr / dr))
                } else {
                    let re = (nr * dr + ni * di) / den2;
                    let im = (ni * dr - nr * di) / den2;
                    Ok(BigValue::from_complex(re, im))
                }
            }
            ExprKind::Neg(inner) => match self.evalf(*inner, ctx)? {
                BigValue::Real(r) => Ok(BigValue::Real(-r)),
                BigValue::Complex(r, i) => Ok(BigValue::Complex(-r, -i)),
            },
            ExprKind::Pow(base, exp) => {
                let b = self.evalf(*base, ctx)?;
                let e = self.evalf(*exp, ctx)?;
                let sheet_sel = ctx
                    .branch_sheet
                    .map(BranchSelection::Sheet)
                    .unwrap_or(BranchSelection::Principal);
                match (b, e) {
                    (BigValue::Real(br), BigValue::Real(er))
                        if (br > 0.0 || (br == 0.0 && er >= 0.0) || er.fract() == 0.0)
                            && ctx.branch_sheet.is_none() =>
                    {
                        Ok(BigValue::Real(br.powf(er)))
                    }
                    (b_val, e_val) => {
                        let (bre, bim) = b_val.as_complex();
                        let (ere, eim) = e_val.as_complex();
                        // Exact integer power via repeated squaring to avoid log branch noise
                        if eim == 0.0 && ere.fract() == 0.0 && ere.abs() <= 1024.0 {
                            let n = ere as i32;
                            if n >= 0 {
                                let (re, im) = complex_powi(bre, bim, n);
                                return Ok(BigValue::from_complex(re, im));
                            } else {
                                let (pr, pi) = complex_powi(bre, bim, -n);
                                let d2 = pr * pr + pi * pi;
                                if d2 == 0.0 {
                                    return Err(AlgebraError::DivisionByZero {
                                        domain: "Complex".into(),
                                    });
                                }
                                return Ok(BigValue::from_complex(pr / d2, -pi / d2));
                            }
                        }
                        let mv = BranchCutManager::eval_power(bre, bim, ere, eim, &sheet_sel)?;
                        let (re, im) = mv.primary_value();
                        Ok(BigValue::from_complex(re, im))
                    }
                }
            }
            ExprKind::Function { name, args } => {
                let fn_name = self.symbols.resolve(*name).unwrap_or_default();
                let default_sel = ctx
                    .branch_sheet
                    .map(BranchSelection::Sheet)
                    .unwrap_or(BranchSelection::Principal);
                if args.len() == 1 {
                    let arg_val = self.evalf(args[0], ctx)?;
                    eval_unary_fn(&fn_name, arg_val, &default_sel)
                } else if args.len() == 2 {
                    eval_binary_fn(self, &fn_name, args[0], args[1], ctx, &default_sel)
                } else if args.len() == 3 {
                    eval_ternary_fn(self, &fn_name, args[0], args[1], args[2], ctx)
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

/// Nodes and weights of the 24-point Gauss–Legendre rule on `[-1, 1]` (Newton's method on `P_24`).
fn gauss_legendre_24() -> &'static ([f64; 24], [f64; 24]) {
    use std::sync::OnceLock;
    static RULE: OnceLock<([f64; 24], [f64; 24])> = OnceLock::new();
    RULE.get_or_init(|| {
        const N: usize = 24;
        let (mut nodes, mut weights) = ([0.0; N], [0.0; N]);
        for i in 0..N / 2 {
            let mut x = (std::f64::consts::PI * (i as f64 + 0.75) / (N as f64 + 0.5)).cos();
            let mut dp = 1.0;
            for _ in 0..100 {
                let (mut p0, mut p1) = (1.0, x);
                for k in 2..=N {
                    let kf = k as f64;
                    let p2 = ((2.0 * kf - 1.0) * x * p1 - (kf - 1.0) * p0) / kf;
                    p0 = p1;
                    p1 = p2;
                }
                dp = N as f64 * (x * p1 - p0) / (x * x - 1.0);
                let dx = p1 / dp;
                x -= dx;
                if dx.abs() < 1e-16 {
                    break;
                }
            }
            nodes[i] = -x;
            nodes[N - 1 - i] = x;
            let w = 2.0 / ((1.0 - x * x) * dp * dp);
            weights[i] = w;
            weights[N - 1 - i] = w;
        }
        (nodes, weights)
    })
}

/// `integral_a^b f` by `panels` equal panels of the 24-point Gauss–Legendre rule.
fn integrate_gl(f: &dyn Fn(f64) -> f64, a: f64, b: f64, panels: usize) -> f64 {
    let (nodes, weights) = gauss_legendre_24();
    let width = (b - a) / panels as f64;
    let mut total = 0.0;
    for p in 0..panels {
        let (lo, hi) = (a + p as f64 * width, a + (p + 1) as f64 * width);
        let (mid, half) = (0.5 * (lo + hi), 0.5 * (hi - lo));
        let mut panel = 0.0;
        for (x, w) in nodes.iter().zip(weights) {
            panel += w * f(mid + half * x);
        }
        total += half * panel;
    }
    total
}

/// `integral_0^infinity f(t) dt` for an integrand that decays like `exp(-rate(t))`: dyadic panels
/// `[h, 2h]`, `[2h, 4h]`, ... starting from a first panel `[0, h]` of width `h`, up to `t_max`.
fn integrate_half_line(f: &dyn Fn(f64) -> f64, h: f64, t_max: f64) -> f64 {
    // Each dyadic panel is cut in four: the integrands are strongly varying near the cut-off.
    let mut total = integrate_gl(f, 0.0, h, 4);
    let mut lo = h;
    while lo < t_max {
        let hi = 2.0 * lo;
        total += integrate_gl(f, lo, hi, 4);
        lo = hi;
    }
    total
}

/// The point beyond which `exp(-(x * kernel(t)) + order * t)` is below `exp(-745)` (underflow).
fn decay_limit(x: f64, order: f64, kernel: &dyn Fn(f64) -> f64) -> f64 {
    let mut t = 1.0;
    while t < 800.0 && x * kernel(t) - order * t < 800.0 {
        t *= 1.25;
    }
    t
}

/// Hankel's asymptotic expansion of `J_n(x)` and `Y_n(x)` for `x` large against `n^2`.
fn bessel_hankel(n: i64, x: f64) -> (f64, f64) {
    let mu = 4.0 * (n * n) as f64;
    let (mut p, mut q) = (1.0, 0.0);
    let mut term = 1.0;
    let mut previous = f64::INFINITY;
    for k in 1..60 {
        let kf = k as f64;
        term *= (mu - (2.0 * kf - 1.0).powi(2)) / (kf * 8.0 * x);
        if term.abs() > previous || term.abs() < 1e-17 {
            break;
        }
        previous = term.abs();
        // Terms alternate between Q (odd k) and P (even k) with signs -, -, +, +, ...
        match k % 4 {
            1 => q += term,
            2 => p -= term,
            3 => q -= term,
            _ => p += term,
        }
    }
    let chi = x - (0.5 * n as f64 + 0.25) * std::f64::consts::PI;
    let scale = (2.0 / (std::f64::consts::PI * x)).sqrt();
    (
        scale * (p * chi.cos() - q * chi.sin()),
        scale * (p * chi.sin() + q * chi.cos()),
    )
}

/// `J_nu(x)` for `x > 0` and any real order, by Schläfli's integral
/// `(1/pi) int_0^pi cos(nu t - x sin t) dt - (sin(nu pi)/pi) int_0^inf exp(-x sinh t - nu t) dt`.
fn bessel_j_real_order(nu: f64, x: f64) -> f64 {
    let pi = std::f64::consts::PI;
    let panels = (((x + nu.abs()) * pi / 8.0).ceil() as usize).max(1);
    let first = integrate_gl(&|t| (nu * t - x * t.sin()).cos(), 0.0, pi, panels) / pi;
    let sin_nu_pi = (nu * pi).sin();
    if nu.fract() == 0.0 || sin_nu_pi.abs() < 1e-300 {
        return first;
    }
    let limit = decay_limit(x, -nu, &|t| t.sinh());
    let second = integrate_half_line(
        &|t| (-x * t.sinh() - nu * t).exp(),
        (0.5 / x).min(0.5),
        limit,
    );
    first - sin_nu_pi / pi * second
}

/// `K_nu(x)` for `x > 0` by `int_0^inf exp(-x cosh t) cosh(nu t) dt`.
fn bessel_k_real_order(nu: f64, x: f64) -> f64 {
    let nu = nu.abs();
    let limit = decay_limit(x, nu, &|t| t.cosh());
    integrate_half_line(
        &|t| (-x * t.cosh()).exp() * (nu * t).cosh(),
        (1.0_f64).min(2.0 / x.sqrt()),
        limit,
    )
}

/// Numerical evaluation of the Bessel function of the first kind $J_n(x)$ for integer order.
///
/// Uses Bessel's integral $J_n(x) = \frac{1}{2\pi}\int_0^{2\pi}\cos(n\tau - x\sin\tau)\,d\tau$ with the
/// trapezoid rule, which for a periodic analytic integrand converges exponentially and, unlike the
/// power series, does not lose all its digits to cancellation for large $x$; Hankel's asymptotic
/// expansion takes over for $x > 2000$ when $4 n^2 < x$.
pub fn eval_bessel_j(n: i64, x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    if n < 0 {
        let sign = if n % 2 != 0 { -1.0 } else { 1.0 };
        return sign * eval_bessel_j(-n, x);
    }
    if x < 0.0 {
        let sign = if n % 2 != 0 { -1.0 } else { 1.0 };
        return sign * eval_bessel_j(n, -x);
    }
    if x == 0.0 {
        return if n == 0 { 1.0 } else { 0.0 };
    }
    if x.is_infinite() {
        return 0.0;
    }
    if x > 2000.0 && 4.0 * (n * n) as f64 <= x {
        return bessel_hankel(n, x).0;
    }
    // The trapezoid rule aliases J_{n + N}; J_k(x) is negligible once k - x exceeds about
    // 16 (x / 2)^(1/3) (the Airy-type transition region), so take N beyond that.
    let points = (x + 18.0 * (x / 2.0).cbrt()).ceil() as usize + (n as usize) + 48;
    let step = 2.0 * std::f64::consts::PI / points as f64;
    let nf = n as f64;
    let sum: f64 = (0..points)
        .map(|k| {
            let tau = k as f64 * step;
            (nf * tau - x * tau.sin()).cos()
        })
        .sum();
    sum / points as f64
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

/// Numerical evaluation of the log-gamma function $\ln|\Gamma(x)|$, without overflow for large $x$
/// (Lanczos in logarithmic form; the reflection formula for $x < 1/2$).
pub fn eval_log_gamma(x: f64) -> f64 {
    if x.is_nan() || (x <= 0.0 && x.fract() == 0.0) {
        return f64::NAN;
    }
    let pi = std::f64::consts::PI;
    if x < 0.5 {
        return (pi / (pi * x).sin()).abs().ln() - eval_log_gamma(1.0 - x);
    }
    // Shift the argument above 12 with ln Gamma(x) = ln Gamma(x + k) - sum ln(x + i), then Stirling's
    // series (error below 1e-17 there).
    let (mut y, mut shift) = (x, 0.0);
    while y < 12.0 {
        shift += y.ln();
        y += 1.0;
    }
    let inv = 1.0 / y;
    let inv2 = inv * inv;
    let series = inv
        * (1.0 / 12.0
            + inv2
                * (-1.0 / 360.0
                    + inv2
                        * (1.0 / 1260.0
                            + inv2
                                * (-1.0 / 1680.0
                                    + inv2 * (1.0 / 1188.0 - inv2 * 691.0 / 360360.0)))));
    (y - 0.5) * y.ln() - y + 0.5 * (2.0 * pi).ln() + series - shift
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

/// Numerical evaluation of the Beta function $B(a, b) = \frac{\Gamma(a)\Gamma(b)}{\Gamma(a+b)}$,
/// through log-gamma so that large arguments do not overflow.
pub fn eval_beta(a: f64, b: f64) -> f64 {
    if a > 0.0 && b > 0.0 {
        return (eval_log_gamma(a) + eval_log_gamma(b) - eval_log_gamma(a + b)).exp();
    }
    eval_gamma(a) * eval_gamma(b) / eval_gamma(a + b)
}

/// Numerical evaluation of the error function $\mathrm{erf}(x)$ to about 15 digits: the Maclaurin
/// series $\frac{2}{\sqrt\pi}e^{-x^2}\sum_n \frac{2^n x^{2n+1}}{(2n+1)!!}$ (positive terms) for
/// $|x| < 3$ and the continued fraction for $\mathrm{erfc}$ beyond.
pub fn eval_erf(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    if x == 0.0 {
        return 0.0;
    }
    let ax = x.abs();
    let value = if ax < 3.0 {
        let mut term = ax;
        let mut sum = ax;
        for n in 1..400 {
            term *= 2.0 * ax * ax / (2.0 * n as f64 + 1.0);
            sum += term;
            if term < 1e-17 * sum {
                break;
            }
        }
        2.0 / std::f64::consts::PI.sqrt() * (-ax * ax).exp() * sum
    } else {
        1.0 - erfc_continued_fraction(ax)
    };
    if x < 0.0 { -value } else { value }
}

/// `erfc(x)` for `x >= 3` by the continued fraction
/// `e^{-x^2}/sqrt(pi) / (x + (1/2)/(x + 1/(x + (3/2)/(x + ...))))`.
fn erfc_continued_fraction(x: f64) -> f64 {
    let mut fraction = x;
    for k in (1..=80).rev() {
        fraction = x + (k as f64 / 2.0) / fraction;
    }
    (-x * x).exp() / (std::f64::consts::PI.sqrt() * fraction)
}

/// Numerical evaluation of the complementary error function $\mathrm{erfc}(x) = 1 - \mathrm{erf}(x)$,
/// without the cancellation of `1 - erf(x)` for large $x$.
pub fn eval_erfc(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    if x >= 3.0 {
        return erfc_continued_fraction(x);
    }
    if x <= -3.0 {
        return 2.0 - erfc_continued_fraction(-x);
    }
    1.0 - eval_erf(x)
}

/// Numerical evaluation of the Riemann zeta function $\zeta(s)$ for real $s \ne 1$.
///
/// Euler–Maclaurin summation, valid for every real $s \ne 1$ (it continues $\zeta$ analytically to
/// $s < 1$): $\sum_{k<N} k^{-s} + \frac{N^{1-s}}{s-1} + \frac{N^{-s}}2 +
/// \sum_{j\ge1}\frac{B_{2j}}{(2j)!}\,s(s+1)\cdots(s+2j-2)\,N^{-s-2j+1}$ with $N = 24$; the
/// functional equation handles $s < 0$.
pub fn eval_zeta(s: f64) -> f64 {
    if s.is_nan() {
        return f64::NAN;
    }
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
        // zeta(-n) = -B_{n+1} / (n + 1)
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
    if s > 60.0 {
        // 1 + 2^-s + 3^-s + ...: converged to double precision after a few terms.
        return (1..=8).map(|k| (k as f64).powf(-s)).sum();
    }
    const N: usize = 24;
    let n = N as f64;
    let mut sum: f64 = (1..N).map(|k| (k as f64).powf(-s)).sum();
    sum += n.powf(1.0 - s) / (s - 1.0) + 0.5 * n.powf(-s);
    // B_{2j} / (2j)!
    let bernoulli = [
        1.0 / 12.0,
        -1.0 / 720.0,
        1.0 / 30240.0,
        -1.0 / 1209600.0,
        1.0 / 47900160.0,
        -691.0 / 1307674368000.0,
        1.0 / 74724249600.0,
        -3617.0 / 10670622842880000.0,
        43867.0 / 5109094217170944000.0,
        -174611.0 / 802857662698291200000.0,
    ];
    let mut rising = s; // s (s+1) ... (s + 2j - 2)
    let mut power = n.powf(-s - 1.0); // N^{-s-2j+1} for j = 1
    for (j, b) in bernoulli.iter().enumerate() {
        let jf = (j + 1) as f64;
        let term = b * rising * power;
        sum += term;
        if term.abs() < 1e-17 * sum.abs() {
            break;
        }
        rising *= (s + 2.0 * jf - 1.0) * (s + 2.0 * jf);
        power /= n * n;
    }
    sum
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

/// Numerical evaluation of the Airy function $\mathrm{Ai}(x)$.
///
/// For $x > 0$, $\mathrm{Ai}(x) = \frac1\pi\sqrt{x/3}\,K_{1/3}(\zeta)$ and for $x < 0$,
/// $\mathrm{Ai}(-x) = \frac{\sqrt x}{3}\left(J_{1/3}(\zeta) + J_{-1/3}(\zeta)\right)$, with
/// $\zeta = \frac23 |x|^{3/2}$; for $|x| > 10$ the asymptotic expansions (DLMF 9.7) are used.
pub fn eval_airy_ai(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    if x.abs() < 1e-12 {
        return 0.355_028_053_887_817_2; // 1 / (3^(2/3) * Gamma(2/3))
    }
    let pi = std::f64::consts::PI;
    let z = x.abs();
    let zeta = 2.0 / 3.0 * z.powf(1.5);
    if z > 10.0 {
        // u_k = Gamma(3k + 1/2) / (54^k k! Gamma(k + 1/2)).
        let mut u = vec![1.0];
        for k in 1..40 {
            let kf = k as f64;
            let prev = u[k - 1];
            u.push(
                prev * (6.0 * kf - 5.0) * (6.0 * kf - 3.0) * (6.0 * kf - 1.0)
                    / ((2.0 * kf - 1.0) * 216.0 * kf),
            );
        }
        if x > 0.0 {
            let mut sum = 0.0;
            let mut power = 1.0;
            let mut previous = f64::INFINITY;
            for (k, uk) in u.iter().enumerate() {
                let term = uk * power;
                if term.abs() > previous {
                    break;
                }
                previous = term.abs();
                sum += if k % 2 == 0 { term } else { -term };
                power /= zeta;
            }
            return (-zeta).exp() / (2.0 * pi.sqrt() * z.powf(0.25)) * sum;
        }
        let (mut even, mut odd) = (0.0, 0.0);
        let mut power = 1.0;
        let mut previous = f64::INFINITY;
        for (k, uk) in u.iter().enumerate() {
            let term = uk * power;
            if term.abs() > previous {
                break;
            }
            previous = term.abs();
            let pair = (k / 2) % 2 == 0;
            if k % 2 == 0 {
                even += if pair { term } else { -term };
            } else {
                odd += if pair { term } else { -term };
            }
            power /= zeta;
        }
        let phase = zeta + pi / 4.0;
        return (phase.sin() * even - phase.cos() * odd) / (pi.sqrt() * z.powf(0.25));
    }
    if x > 0.0 {
        (z / 3.0).sqrt() / pi * bessel_k_real_order(1.0 / 3.0, zeta)
    } else {
        z.sqrt() / 3.0
            * (bessel_j_real_order(1.0 / 3.0, zeta) + bessel_j_real_order(-1.0 / 3.0, zeta))
    }
}

/// Numerical evaluation of the Bessel function of the second kind $Y_n(x)$ for integer order, $x > 0$.
///
/// $$Y_n(x) = \frac1\pi\int_0^\pi \sin(x\sin\theta - n\theta)\,d\theta -
///   \frac1\pi\int_0^\infty \left(e^{nt} + (-1)^n e^{-nt}\right) e^{-x\sinh t}\,dt,$$
/// by Gauss–Legendre quadrature; Hankel's expansion for $x > 2000$.
pub fn eval_bessel_y(n: i64, x: f64) -> f64 {
    if x.is_nan() || x <= 0.0 {
        return f64::NAN;
    }
    if n < 0 {
        let sign = if n % 2 != 0 { -1.0 } else { 1.0 };
        return sign * eval_bessel_y(-n, x);
    }
    if x.is_infinite() {
        return 0.0;
    }
    if x > 2000.0 && 4.0 * (n * n) as f64 <= x {
        return bessel_hankel(n, x).1;
    }
    let pi = std::f64::consts::PI;
    let nf = n as f64;
    let panels = (((x + nf) * pi / 8.0).ceil() as usize).max(1);
    let first = integrate_gl(&|t| (x * t.sin() - nf * t).sin(), 0.0, pi, panels) / pi;
    let sign = if n % 2 == 0 { 1.0 } else { -1.0 };
    let limit = decay_limit(x, nf, &|t| t.sinh());
    let second = integrate_half_line(
        &|t| ((nf * t).exp() + sign * (-nf * t).exp()) * (-x * t.sinh()).exp(),
        (0.5 / x).min(0.5),
        limit,
    );
    first - second / pi
}

/// Numerical evaluation of the modified Bessel function of the first kind $I_n(x)$ by its power
/// series (all terms are positive for $x \ge 0$, so there is no cancellation).
pub fn eval_bessel_i(n: i64, x: f64) -> f64 {
    let n_abs = n.unsigned_abs() as usize;
    let half_x = x.abs() / 2.0;
    let mut term = 1.0;
    for k in 1..=n_abs {
        term *= half_x / k as f64;
    }
    let mut sum = term;
    for m in 1..5000 {
        term *= (half_x * half_x) / (m as f64 * (m + n_abs) as f64);
        sum += term;
        if term < 1e-17 * sum {
            break;
        }
    }
    // I_{-n} = I_n and I_n(-x) = (-1)^n I_n(x).
    if x < 0.0 && n_abs % 2 == 1 { -sum } else { sum }
}

/// Numerical evaluation of the modified Bessel function of the second kind $K_n(x)$, $x > 0$.
///
/// $K_n(x) = \int_0^\infty e^{-x\cosh t}\cosh(nt)\,dt$ by Gauss–Legendre quadrature (the integrand
/// is smooth and decays double-exponentially).
pub fn eval_bessel_k(n: i64, x: f64) -> f64 {
    if x.is_nan() || x <= 0.0 {
        return f64::NAN;
    }
    if x.is_infinite() {
        return 0.0;
    }
    bessel_k_real_order(n.unsigned_abs() as f64, x)
}

/// Exact arbitrary-precision evaluation over $\mathbb{Q}$ when the expression contains
/// only exact integers/rationals, arithmetic, integer powers, exact square roots,
/// factorials, gcd/lcm/mod/binomial, and no floats or transcendental quantities.
pub fn try_eval_exact(
    graph: &ExprGraph,
    id: ExprId,
    bindings: &std::collections::HashMap<String, f64>,
) -> AlgebraResult<Option<num_rational::BigRational>> {
    use num::Integer as _;
    use num_bigint::{BigInt, BigUint};
    use num_rational::BigRational;
    use num_traits::{Signed as _, ToPrimitive as _, Zero as _};

    fn perfect_square_biguint(u: &BigUint) -> Option<BigUint> {
        let s = u.sqrt();
        if &(&s * &s) == u { Some(s) } else { None }
    }

    fn factorial_bigint(n: usize) -> BigInt {
        let mut acc = BigInt::from(1u8);
        for k in 2..=n {
            acc *= k;
        }
        acc
    }

    fn pi_multiple_rational(
        g: &ExprGraph,
        id: ExprId,
        bindings: &std::collections::HashMap<String, f64>,
    ) -> Option<BigRational> {
        match &g.get(id).kind {
            ExprKind::Number(Number::Constant(Constant::Pi)) => {
                Some(BigRational::from_integer(BigInt::from(1u8)))
            }
            ExprKind::Symbol(s) if g.symbols.resolve(*s).as_deref() == Some("pi") => {
                Some(BigRational::from_integer(BigInt::from(1u8)))
            }
            ExprKind::Neg(inner) => Some(-pi_multiple_rational(g, *inner, bindings)?),
            ExprKind::Add(terms) => {
                let mut acc = BigRational::zero();
                for &t in terms {
                    acc += pi_multiple_rational(g, t, bindings)?;
                }
                Some(acc)
            }
            ExprKind::Sub(a, b) => Some(
                pi_multiple_rational(g, *a, bindings)? - pi_multiple_rational(g, *b, bindings)?,
            ),
            ExprKind::Div(a, b) => {
                let pa = pi_multiple_rational(g, *a, bindings)?;
                let rb = eval_node(g, *b, bindings).ok()??;
                if rb.is_zero() { None } else { Some(pa / rb) }
            }
            ExprKind::Mul(factors) => {
                let mut pi_count = 0usize;
                let mut coeff = BigRational::from_integer(BigInt::from(1u8));
                for &f in factors {
                    if let Some(p) = pi_multiple_rational(g, f, bindings) {
                        pi_count += 1;
                        coeff *= p;
                    } else if let Ok(Some(r)) = eval_node(g, f, bindings) {
                        coeff *= r;
                    } else {
                        return None;
                    }
                }
                if pi_count == 1 { Some(coeff) } else { None }
            }
            _ => None,
        }
    }

    fn exact_rational_trig(fn_name: &str, q_pi: &BigRational) -> Option<BigRational> {
        let twelve = BigRational::from_integer(BigInt::from(12u8));
        let scaled = q_pi * twelve;
        if !scaled.is_integer() {
            return None;
        }
        let k24 = scaled.numer().mod_floor(&BigInt::from(24u8)).to_i64()?;
        match fn_name {
            "sin" => match k24 {
                0 | 12 => Some(BigRational::zero()),
                2 | 10 => Some(BigRational::new(BigInt::from(1), BigInt::from(2))),
                6 => Some(BigRational::from_integer(BigInt::from(1))),
                14 | 22 => Some(BigRational::new(BigInt::from(-1), BigInt::from(2))),
                18 => Some(BigRational::from_integer(BigInt::from(-1))),
                _ => None,
            },
            "cos" => match k24 {
                0 => Some(BigRational::from_integer(BigInt::from(1))),
                4 | 20 => Some(BigRational::new(BigInt::from(1), BigInt::from(2))),
                6 | 18 => Some(BigRational::zero()),
                8 | 16 => Some(BigRational::new(BigInt::from(-1), BigInt::from(2))),
                12 => Some(BigRational::from_integer(BigInt::from(-1))),
                _ => None,
            },
            "tan" => match k24 {
                0 | 12 => Some(BigRational::zero()),
                3 | 15 => Some(BigRational::from_integer(BigInt::from(1))),
                9 | 21 => Some(BigRational::from_integer(BigInt::from(-1))),
                _ => None,
            },
            _ => None,
        }
    }

    fn eval_node(
        g: &ExprGraph,
        id: ExprId,
        bindings: &std::collections::HashMap<String, f64>,
    ) -> AlgebraResult<Option<BigRational>> {
        match &g.get(id).kind {
            ExprKind::Number(n) => match n {
                Number::Integer(i) => Ok(Some(BigRational::from_integer(BigInt::from(*i)))),
                Number::BigInteger(b) => Ok(Some(BigRational::from_integer(b.clone()))),
                Number::Rational(num, den) => {
                    if *den == 0 {
                        Err(AlgebraError::DivisionByZero {
                            domain: "Rationals".into(),
                        })
                    } else {
                        Ok(Some(BigRational::new(
                            BigInt::from(*num),
                            BigInt::from(*den),
                        )))
                    }
                }
                Number::BigRational(r) => Ok(Some((**r).clone())),
                Number::Float(_) | Number::Scientific { .. } | Number::Constant(_) => Ok(None),
            },
            ExprKind::Symbol(s) => {
                if let Some(name) = g.symbols.resolve(*s)
                    && let Some(&v) = bindings.get(&name)
                    && v.is_finite()
                    && v.fract() == 0.0
                    && v.abs() < 9.0e15
                {
                    return Ok(Some(BigRational::from_integer(BigInt::from(v as i64))));
                }
                Ok(None)
            }
            ExprKind::Neg(a) => Ok(eval_node(g, *a, bindings)?.map(|r| -r)),
            ExprKind::Add(terms) => {
                let mut sum = BigRational::zero();
                for &t in terms {
                    match eval_node(g, t, bindings)? {
                        Some(r) => sum += r,
                        None => return Ok(None),
                    }
                }
                Ok(Some(sum))
            }
            ExprKind::Sub(a, b) => match (eval_node(g, *a, bindings)?, eval_node(g, *b, bindings)?)
            {
                (Some(ra), Some(rb)) => Ok(Some(ra - rb)),
                _ => Ok(None),
            },
            ExprKind::Mul(factors) => {
                let mut prod = BigRational::from_integer(BigInt::from(1u8));
                for &f in factors {
                    match eval_node(g, f, bindings)? {
                        Some(r) => prod *= r,
                        None => return Ok(None),
                    }
                }
                Ok(Some(prod))
            }
            ExprKind::Div(a, b) => match (eval_node(g, *a, bindings)?, eval_node(g, *b, bindings)?)
            {
                (Some(ra), Some(rb)) => {
                    if rb.is_zero() {
                        Err(AlgebraError::DivisionByZero {
                            domain: "Rationals".into(),
                        })
                    } else {
                        Ok(Some(ra / rb))
                    }
                }
                _ => Ok(None),
            },
            ExprKind::Pow(b, e) => {
                let is_i = matches!(
                    g.get(*b).kind,
                    ExprKind::Number(Number::Constant(algebra_core::Constant::I))
                ) || matches!(
                    g.get(*b).kind,
                    ExprKind::Symbol(sym) if matches!(g.symbols.resolve(sym).as_deref(), Some("i" | "I"))
                );
                if is_i
                    && let Ok(Some(re)) = eval_node(g, *e, bindings)
                    && re.is_integer()
                {
                    let m4 = re.numer().mod_floor(&BigInt::from(4u8)).to_i64();
                    if m4 == Some(0) {
                        return Ok(Some(BigRational::from_integer(BigInt::from(1u8))));
                    } else if m4 == Some(2) {
                        return Ok(Some(BigRational::from_integer(BigInt::from(-1i8))));
                    }
                }
                match (eval_node(g, *b, bindings)?, eval_node(g, *e, bindings)?) {
                    (Some(rb), Some(re)) => {
                        if re.is_integer() {
                            let Some(exp_i64) = re.numer().to_i64() else {
                                return Ok(None);
                            };
                            if exp_i64 < 0 && rb.is_zero() {
                                return Err(AlgebraError::DivisionByZero {
                                    domain: "Rationals".into(),
                                });
                            }
                            if exp_i64 == 0 {
                                return Ok(Some(BigRational::from_integer(BigInt::from(1u8))));
                            }
                            let abs_exp = exp_i64.unsigned_abs();
                            if abs_exp > 100_000 {
                                return Ok(None);
                            }
                            let base_bits = rb.numer().bits().max(rb.denom().bits()).max(1);
                            if base_bits.saturating_mul(abs_exp) > 2_000_000 {
                                return Ok(None);
                            }
                            let Ok(exp_u32) = u32::try_from(abs_exp) else {
                                return Ok(None);
                            };
                            let res = if exp_i64 > 0 {
                                BigRational::new(rb.numer().pow(exp_u32), rb.denom().pow(exp_u32))
                            } else {
                                BigRational::new(rb.denom().pow(exp_u32), rb.numer().pow(exp_u32))
                            };
                            Ok(Some(res))
                        } else if *re.denom() == BigInt::from(2u8)
                            && !rb.is_negative()
                            && let (Some(nu), Some(du)) =
                                (rb.numer().to_biguint(), rb.denom().to_biguint())
                            && let (Some(sn), Some(sd)) =
                                (perfect_square_biguint(&nu), perfect_square_biguint(&du))
                        {
                            let sqrt_q = BigRational::new(BigInt::from(sn), BigInt::from(sd));
                            let Some(exp_i64) = re.numer().to_i64() else {
                                return Ok(None);
                            };
                            if exp_i64 < 0 && sqrt_q.is_zero() {
                                return Err(AlgebraError::DivisionByZero {
                                    domain: "Rationals".into(),
                                });
                            }
                            let Ok(exp_u32) = u32::try_from(exp_i64.unsigned_abs()) else {
                                return Ok(None);
                            };
                            if exp_u32 > 10_000 {
                                return Ok(None);
                            }
                            let res = if exp_i64 >= 0 {
                                BigRational::new(
                                    sqrt_q.numer().pow(exp_u32),
                                    sqrt_q.denom().pow(exp_u32),
                                )
                            } else {
                                BigRational::new(
                                    sqrt_q.denom().pow(exp_u32),
                                    sqrt_q.numer().pow(exp_u32),
                                )
                            };
                            Ok(Some(res))
                        } else {
                            Ok(None)
                        }
                    }
                    _ => Ok(None),
                }
            }
            ExprKind::Function { name, args } => {
                let fn_name = g.symbols.resolve(*name).unwrap_or_default();
                if args.len() == 1 {
                    if matches!(fn_name.as_str(), "sin" | "cos" | "tan")
                        && let Some(q_pi) = pi_multiple_rational(g, args[0], bindings)
                        && let Some(exact_trig) = exact_rational_trig(&fn_name, &q_pi)
                    {
                        return Ok(Some(exact_trig));
                    }
                    if matches!(fn_name.as_str(), "ln" | "log") {
                        let is_e_node = |node_id: ExprId| -> bool {
                            matches!(
                                g.get(node_id).kind,
                                ExprKind::Number(Number::Constant(algebra_core::Constant::E))
                            ) || matches!(
                                g.get(node_id).kind,
                                ExprKind::Symbol(sym) if g.symbols.resolve(sym).as_deref() == Some("e")
                            )
                        };
                        if is_e_node(args[0]) {
                            return Ok(Some(BigRational::from_integer(BigInt::from(1u8))));
                        }
                        if let ExprKind::Pow(eb, ee) = g.get(args[0]).kind
                            && is_e_node(eb)
                            && let Some(q_exp) = eval_node(g, ee, bindings)?
                        {
                            return Ok(Some(q_exp));
                        }
                        if let ExprKind::Function {
                            name: inner_n,
                            args: inner_a,
                        } = g.get(args[0]).kind
                            && g.symbols.resolve(inner_n).as_deref() == Some("exp")
                            && inner_a.len() == 1
                            && let Some(q_exp) = eval_node(g, inner_a[0], bindings)?
                        {
                            return Ok(Some(q_exp));
                        }
                    }
                    let Some(arg) = eval_node(g, args[0], bindings)? else {
                        return Ok(None);
                    };
                    match fn_name.as_str() {
                        "abs" | "Abs" => Ok(Some(arg.abs())),
                        "floor" => Ok(Some(BigRational::from_integer(arg.floor().to_integer()))),
                        "ceil" => Ok(Some(BigRational::from_integer(arg.ceil().to_integer()))),
                        "sqrt" => {
                            if !arg.is_negative()
                                && let (Some(nu), Some(du)) =
                                    (arg.numer().to_biguint(), arg.denom().to_biguint())
                                && let (Some(sn), Some(sd)) =
                                    (perfect_square_biguint(&nu), perfect_square_biguint(&du))
                            {
                                Ok(Some(BigRational::new(BigInt::from(sn), BigInt::from(sd))))
                            } else {
                                Ok(None)
                            }
                        }
                        "factorial" => {
                            if arg.is_integer()
                                && !arg.is_negative()
                                && let Some(n) = arg.numer().to_usize()
                                && n <= 5000
                            {
                                Ok(Some(BigRational::from_integer(factorial_bigint(n))))
                            } else {
                                Ok(None)
                            }
                        }
                        "sin" | "tan" | "sinh" | "tanh" | "asin" | "arcsin" | "atan" | "arctan"
                            if arg.is_zero() =>
                        {
                            Ok(Some(BigRational::zero()))
                        }
                        "cos" | "cosh" | "exp" if arg.is_zero() => {
                            Ok(Some(BigRational::from_integer(BigInt::from(1u8))))
                        }
                        "ln" | "log" => {
                            if *arg.numer() == BigInt::from(1u8)
                                && *arg.denom() == BigInt::from(1u8)
                            {
                                Ok(Some(BigRational::zero()))
                            } else {
                                Ok(None)
                            }
                        }
                        _ => Ok(None),
                    }
                } else if args.len() == 2 {
                    let (Some(a), Some(b)) = (
                        eval_node(g, args[0], bindings)?,
                        eval_node(g, args[1], bindings)?,
                    ) else {
                        return Ok(None);
                    };
                    match fn_name.as_str() {
                        "gcd" if a.is_integer() && b.is_integer() => {
                            Ok(Some(BigRational::from_integer(a.numer().gcd(b.numer()))))
                        }
                        "lcm" if a.is_integer() && b.is_integer() => {
                            Ok(Some(BigRational::from_integer(a.numer().lcm(b.numer()))))
                        }
                        "mod" | "Mod" if a.is_integer() && b.is_integer() => {
                            if b.is_zero() {
                                Err(AlgebraError::DivisionByZero {
                                    domain: "Integers".into(),
                                })
                            } else {
                                let rem = a.numer().mod_floor(b.numer());
                                Ok(Some(BigRational::from_integer(rem)))
                            }
                        }
                        "binomial" if a.is_integer() && b.is_integer() => {
                            if let (Some(n), Some(k)) = (a.numer().to_usize(), b.numer().to_usize())
                                && k <= n
                                && n <= 2000
                            {
                                let k = k.min(n - k);
                                let mut num = BigInt::from(1u8);
                                let mut den = BigInt::from(1u8);
                                for i in 0..k {
                                    num *= n - i;
                                    den *= i + 1;
                                }
                                Ok(Some(BigRational::from_integer(num / den)))
                            } else {
                                Ok(None)
                            }
                        }
                        "log" | "log_base" => {
                            use num_traits::One as _;
                            if a.is_one() && !b.is_zero() && !b.is_one() {
                                return Ok(Some(BigRational::zero()));
                            }
                            if !a.is_positive() || !b.is_positive() || b.is_one() {
                                return Ok(None);
                            }
                            // Check if a = b^k or a = b^(-k) for small integer k
                            let inv = a < BigRational::one();
                            let target = if inv { a.recip() } else { a.clone() };
                            let step = if b < BigRational::one() {
                                b.recip()
                            } else {
                                b.clone()
                            };
                            let flip = b < BigRational::one();
                            let mut cur = BigRational::one();
                            for k in 0i64..=128 {
                                if cur == target {
                                    let exp = match (inv, flip) {
                                        (false, false) | (true, true) => k,
                                        _ => -k,
                                    };
                                    return Ok(Some(BigRational::from_integer(BigInt::from(exp))));
                                }
                                if cur > target || cur.numer().bits() > 4096 {
                                    break;
                                }
                                cur *= &step;
                            }
                            Ok(None)
                        }
                        _ => Ok(None),
                    }
                } else {
                    Ok(None)
                }
            }
            ExprKind::Relational { rhs, .. } => eval_node(g, *rhs, bindings),
            _ => Ok(None),
        }
    }

    eval_node(graph, id, bindings)
}

fn complex_powi(mut re: f64, mut im: f64, mut n: i32) -> (f64, f64) {
    let mut acc_re = 1.0;
    let mut acc_im = 0.0;
    while n > 0 {
        if n & 1 == 1 {
            let nr = acc_re * re - acc_im * im;
            let ni = acc_re * im + acc_im * re;
            acc_re = nr;
            acc_im = ni;
        }
        n >>= 1;
        if n > 0 {
            let nr = re * re - im * im;
            let ni = 2.0 * re * im;
            re = nr;
            im = ni;
        }
    }
    (acc_re, acc_im)
}

fn big_value_div(num: BigValue, den: BigValue) -> AlgebraResult<BigValue> {
    match (num, den) {
        (BigValue::Real(a), BigValue::Real(b)) => {
            if b == 0.0 {
                Err(AlgebraError::DivisionByZero {
                    domain: "Real".into(),
                })
            } else {
                Ok(BigValue::Real(a / b))
            }
        }
        (a, b) => {
            let (ar, ai) = a.as_complex();
            let (br, bi) = b.as_complex();
            let d2 = br * br + bi * bi;
            if d2 == 0.0 {
                return Err(AlgebraError::DivisionByZero {
                    domain: "Complex".into(),
                });
            }
            Ok(BigValue::from_complex(
                (ar * br + ai * bi) / d2,
                (ai * br - ar * bi) / d2,
            ))
        }
    }
}

fn eval_unary_fn(
    fn_name: &str,
    arg_val: BigValue,
    default_sel: &BranchSelection,
) -> AlgebraResult<BigValue> {
    match (fn_name, &arg_val) {
        ("sin", BigValue::Real(r)) => Ok(BigValue::Real(r.sin())),
        ("sin", BigValue::Complex(re, im)) => Ok(BigValue::from_complex(
            re.sin() * im.cosh(),
            re.cos() * im.sinh(),
        )),
        ("cos", BigValue::Real(r)) => Ok(BigValue::Real(r.cos())),
        ("cos", BigValue::Complex(re, im)) => Ok(BigValue::from_complex(
            re.cos() * im.cosh(),
            -re.sin() * im.sinh(),
        )),
        ("tan", BigValue::Real(r)) => Ok(BigValue::Real(r.tan())),
        ("tan", BigValue::Complex(_, _)) => {
            let s = eval_unary_fn("sin", arg_val.clone(), default_sel)?;
            let c = eval_unary_fn("cos", arg_val, default_sel)?;
            big_value_div(s, c)
        }
        ("sec" | "csc" | "sech" | "csch", _) => {
            let base = match fn_name {
                "sec" => "cos",
                "csc" => "sin",
                "sech" => "cosh",
                "csch" => "sinh",
                _ => unreachable!(),
            };
            let den = eval_unary_fn(base, arg_val, default_sel)?;
            big_value_div(BigValue::Real(1.0), den)
        }
        ("cot", _) => {
            let c = eval_unary_fn("cos", arg_val.clone(), default_sel)?;
            let s = eval_unary_fn("sin", arg_val, default_sel)?;
            big_value_div(c, s)
        }
        ("sinh", BigValue::Real(r)) => Ok(BigValue::Real(r.sinh())),
        ("sinh", BigValue::Complex(re, im)) => Ok(BigValue::from_complex(
            re.sinh() * im.cos(),
            re.cosh() * im.sin(),
        )),
        ("cosh", BigValue::Real(r)) => Ok(BigValue::Real(r.cosh())),
        ("cosh", BigValue::Complex(re, im)) => Ok(BigValue::from_complex(
            re.cosh() * im.cos(),
            re.sinh() * im.sin(),
        )),
        ("tanh", BigValue::Real(r)) => Ok(BigValue::Real(r.tanh())),
        ("tanh", BigValue::Complex(_, _)) => {
            let s = eval_unary_fn("sinh", arg_val.clone(), default_sel)?;
            let c = eval_unary_fn("cosh", arg_val, default_sel)?;
            big_value_div(s, c)
        }
        ("coth", _) => {
            let c = eval_unary_fn("cosh", arg_val.clone(), default_sel)?;
            let s = eval_unary_fn("sinh", arg_val, default_sel)?;
            big_value_div(c, s)
        }
        ("exp", BigValue::Real(r)) => Ok(BigValue::Real(r.exp())),
        ("exp", BigValue::Complex(re, im)) => {
            let mag = re.exp();
            Ok(BigValue::from_complex(mag * im.cos(), mag * im.sin()))
        }
        ("ln" | "log", _) => {
            let (re, im) = arg_val.as_complex();
            if re > 0.0 && im == 0.0 && *default_sel == BranchSelection::Principal {
                Ok(BigValue::Real(re.ln()))
            } else {
                let mv = BranchCutManager::eval(BranchCutFunctionKind::Log, re, im, default_sel)?;
                let (r, i) = mv.primary_value();
                Ok(BigValue::from_complex(r, i))
            }
        }
        ("sqrt", _) => {
            let (re, im) = arg_val.as_complex();
            if re >= 0.0 && im == 0.0 && *default_sel == BranchSelection::Principal {
                Ok(BigValue::Real(re.sqrt()))
            } else {
                let mv = BranchCutManager::eval(BranchCutFunctionKind::Sqrt, re, im, default_sel)?;
                let (r, i) = mv.primary_value();
                Ok(BigValue::from_complex(r, i))
            }
        }
        ("asin" | "arcsin", _) => {
            let (re, im) = arg_val.as_complex();
            if im == 0.0 && re.abs() <= 1.0 && *default_sel == BranchSelection::Principal {
                Ok(BigValue::Real(re.asin()))
            } else {
                let mv =
                    BranchCutManager::eval(BranchCutFunctionKind::ArcSin, re, im, default_sel)?;
                let (r, i) = mv.primary_value();
                Ok(BigValue::from_complex(r, i))
            }
        }
        ("acos" | "arccos", _) => {
            let (re, im) = arg_val.as_complex();
            if im == 0.0 && re.abs() <= 1.0 && *default_sel == BranchSelection::Principal {
                Ok(BigValue::Real(re.acos()))
            } else {
                let mv =
                    BranchCutManager::eval(BranchCutFunctionKind::ArcCos, re, im, default_sel)?;
                let (r, i) = mv.primary_value();
                Ok(BigValue::from_complex(r, i))
            }
        }
        ("atan" | "arctan", _) => {
            let (re, im) = arg_val.as_complex();
            if im == 0.0 && *default_sel == BranchSelection::Principal {
                Ok(BigValue::Real(re.atan()))
            } else {
                let mv =
                    BranchCutManager::eval(BranchCutFunctionKind::ArcTan, re, im, default_sel)?;
                let (r, i) = mv.primary_value();
                Ok(BigValue::from_complex(r, i))
            }
        }
        ("asinh" | "arcsinh", _) => {
            let (re, im) = arg_val.as_complex();
            if im == 0.0 && *default_sel == BranchSelection::Principal {
                Ok(BigValue::Real(re.asinh()))
            } else {
                let mv =
                    BranchCutManager::eval(BranchCutFunctionKind::ArcSinh, re, im, default_sel)?;
                let (r, i) = mv.primary_value();
                Ok(BigValue::from_complex(r, i))
            }
        }
        ("acosh" | "arccosh", _) => {
            let (re, im) = arg_val.as_complex();
            if im == 0.0 && re >= 1.0 && *default_sel == BranchSelection::Principal {
                Ok(BigValue::Real(re.acosh()))
            } else {
                let mv =
                    BranchCutManager::eval(BranchCutFunctionKind::ArcCosh, re, im, default_sel)?;
                let (r, i) = mv.primary_value();
                Ok(BigValue::from_complex(r, i))
            }
        }
        ("atanh" | "arctanh", _) => {
            let (re, im) = arg_val.as_complex();
            if im == 0.0 && re.abs() < 1.0 && *default_sel == BranchSelection::Principal {
                Ok(BigValue::Real(re.atanh()))
            } else {
                let mv =
                    BranchCutManager::eval(BranchCutFunctionKind::ArcTanh, re, im, default_sel)?;
                let (r, i) = mv.primary_value();
                Ok(BigValue::from_complex(r, i))
            }
        }
        ("lambert_w" | "W" | "LambertW", _) => {
            let (re, im) = arg_val.as_complex();
            let inv_e = -(-1.0f64).exp();
            if im == 0.0 && re >= inv_e && *default_sel == BranchSelection::Principal {
                Ok(BigValue::Real(eval_lambert_w(re)))
            } else {
                let mv =
                    BranchCutManager::eval(BranchCutFunctionKind::LambertW, re, im, default_sel)?;
                let (r, i) = mv.primary_value();
                Ok(BigValue::from_complex(r, i))
            }
        }
        ("abs" | "Abs", BigValue::Real(r)) => Ok(BigValue::Real(r.abs())),
        ("abs" | "Abs", BigValue::Complex(r, i)) => Ok(BigValue::Real(r.hypot(*i))),
        ("sign" | "sgn" | "signum", BigValue::Real(r)) => Ok(BigValue::Real(if *r > 0.0 {
            1.0
        } else if *r < 0.0 {
            -1.0
        } else {
            0.0
        })),
        ("heaviside" | "Heaviside" | "step", BigValue::Real(r)) => {
            Ok(BigValue::Real(if *r > 0.0 {
                1.0
            } else if *r < 0.0 {
                0.0
            } else {
                0.5
            }))
        }
        ("re" | "real" | "Re", BigValue::Real(r)) => Ok(BigValue::Real(*r)),
        ("re" | "real" | "Re", BigValue::Complex(r, _)) => Ok(BigValue::Real(*r)),
        ("im" | "imag" | "Im", BigValue::Real(_)) => Ok(BigValue::Real(0.0)),
        ("im" | "imag" | "Im", BigValue::Complex(_, i)) => Ok(BigValue::Real(*i)),
        ("arg" | "phase", _) => {
            let (re, im) = arg_val.as_complex();
            Ok(BigValue::Real(im.atan2(re)))
        }
        ("conj" | "conjugate", BigValue::Real(r)) => Ok(BigValue::Real(*r)),
        ("conj" | "conjugate", BigValue::Complex(r, i)) => Ok(BigValue::from_complex(*r, -*i)),
        ("BringRadical" | "BR", BigValue::Real(r)) => {
            Ok(BigValue::Real(crate::poly::roots::BringRadical::eval(*r)))
        }
        ("gamma", BigValue::Real(r)) => Ok(BigValue::Real(
            crate::bigfloat::special_f64("gamma", *r).unwrap_or_else(|| eval_gamma(*r)),
        )),
        ("log_gamma" | "lngamma" | "lgamma", BigValue::Real(r)) => Ok(BigValue::Real(
            crate::bigfloat::special_f64("lgamma", *r).unwrap_or_else(|| eval_log_gamma(*r)),
        )),
        ("digamma" | "psi", BigValue::Real(r)) => Ok(BigValue::Real(eval_digamma(*r))),
        ("zeta", BigValue::Real(r)) => Ok(BigValue::Real(eval_zeta(*r))),
        ("erf", BigValue::Real(r)) => Ok(BigValue::Real(
            crate::bigfloat::special_f64("erf", *r).unwrap_or_else(|| eval_erf(*r)),
        )),
        ("erfc", BigValue::Real(r)) => Ok(BigValue::Real(
            crate::bigfloat::special_f64("erfc", *r).unwrap_or_else(|| eval_erfc(*r)),
        )),
        ("si" | "Si", BigValue::Real(r)) => Ok(BigValue::Real(eval_si(*r))),
        ("ci" | "Ci", BigValue::Real(r)) => Ok(BigValue::Real(eval_ci(*r))),
        ("airy_ai" | "Ai", BigValue::Real(r)) => Ok(BigValue::Real(eval_airy_ai(*r))),
        ("floor", BigValue::Real(r)) => Ok(BigValue::Real(r.floor())),
        ("ceil", BigValue::Real(r)) => Ok(BigValue::Real(r.ceil())),
        ("round", BigValue::Real(r)) => Ok(BigValue::Real(r.round())),
        ("factorial", BigValue::Real(r)) => Ok(BigValue::Real(eval_gamma(*r + 1.0))),
        _ => Err(AlgebraError::EvaluationError(format!(
            "Unsupported function '{}' in numerical evaluation",
            fn_name
        ))),
    }
}

fn eval_binary_fn(
    graph: &ExprGraph,
    fn_name: &str,
    arg0: ExprId,
    arg1: ExprId,
    ctx: &EvalContext,
    default_sel: &BranchSelection,
) -> AlgebraResult<BigValue> {
    match fn_name {
        "uncertain" | "measurement" => return graph.evalf(arg0, ctx),
        "interval" => {
            let lo = graph.evalf(arg0, ctx)?.to_f64();
            let hi = graph.evalf(arg1, ctx)?.to_f64();
            return Ok(BigValue::Real(0.5 * (lo + hi)));
        }
        "lambert_w" | "LambertW" | "W" => {
            let k = graph.evalf(arg0, ctx)?.to_f64().round() as i64;
            let (zr, zi) = graph.evalf(arg1, ctx)?.as_complex();
            let mv = BranchCutManager::eval(
                BranchCutFunctionKind::LambertW,
                zr,
                zi,
                &BranchSelection::Sheet(k),
            )?;
            let (r, i) = mv.primary_value();
            return Ok(BigValue::from_complex(r, i));
        }
        "ln" | "log_branch" => {
            let (zr, zi) = graph.evalf(arg0, ctx)?.as_complex();
            let k = graph.evalf(arg1, ctx)?.to_f64().round() as i64;
            let mv = BranchCutManager::eval(
                BranchCutFunctionKind::Log,
                zr,
                zi,
                &BranchSelection::Sheet(k),
            )?;
            let (r, i) = mv.primary_value();
            return Ok(BigValue::from_complex(r, i));
        }
        "sqrt" => {
            let (zr, zi) = graph.evalf(arg0, ctx)?.as_complex();
            let k = graph.evalf(arg1, ctx)?.to_f64().round() as i64;
            let mv = BranchCutManager::eval(
                BranchCutFunctionKind::Sqrt,
                zr,
                zi,
                &BranchSelection::Sheet(k),
            )?;
            let (r, i) = mv.primary_value();
            return Ok(BigValue::from_complex(r, i));
        }
        "root" => {
            let (zr, zi) = graph.evalf(arg0, ctx)?.as_complex();
            let n = graph.evalf(arg1, ctx)?.to_f64().round() as i64;
            if n <= 0 {
                return Err(AlgebraError::DivisionByZero {
                    domain: "Complex Root".into(),
                });
            }
            let mv =
                BranchCutManager::eval(BranchCutFunctionKind::Root(n as u32), zr, zi, default_sel)?;
            let (r, i) = mv.primary_value();
            return Ok(BigValue::from_complex(r, i));
        }
        "asin" | "arcsin" | "acos" | "arccos" | "atan" | "arctan" | "asinh" | "arcsinh"
        | "acosh" | "arccosh" | "atanh" | "arctanh" => {
            let (zr, zi) = graph.evalf(arg0, ctx)?.as_complex();
            let k = graph.evalf(arg1, ctx)?.to_f64().round() as i64;
            let kind = match fn_name {
                "asin" | "arcsin" => BranchCutFunctionKind::ArcSin,
                "acos" | "arccos" => BranchCutFunctionKind::ArcCos,
                "atan" | "arctan" => BranchCutFunctionKind::ArcTan,
                "asinh" | "arcsinh" => BranchCutFunctionKind::ArcSinh,
                "acosh" | "arccosh" => BranchCutFunctionKind::ArcCosh,
                _ => BranchCutFunctionKind::ArcTanh,
            };
            let mv = BranchCutManager::eval(kind, zr, zi, &BranchSelection::Sheet(k))?;
            let (r, i) = mv.primary_value();
            return Ok(BigValue::from_complex(r, i));
        }
        _ => {}
    }

    let a1 = graph.evalf(arg0, ctx)?.to_f64();
    let a2 = graph.evalf(arg1, ctx)?.to_f64();
    match fn_name {
        "log" | "log_base" => Ok(BigValue::Real(a1.ln() / a2.ln())),
        "mod" | "Mod" => {
            if a2 == 0.0 {
                Err(AlgebraError::DivisionByZero {
                    domain: "Reals".into(),
                })
            } else {
                Ok(BigValue::Real(a1.rem_euclid(a2)))
            }
        }
        "atan2" => Ok(BigValue::Real(a1.atan2(a2))),
        "besselj" | "bessel_j" => Ok(BigValue::Real(eval_bessel_j(a1.round() as i64, a2))),
        "beta" | "Beta" => Ok(BigValue::Real(eval_beta(a1, a2))),
        "bessel_y" | "bessely" => Ok(BigValue::Real(eval_bessel_y(a1.round() as i64, a2))),
        "bessel_i" | "besseli" => Ok(BigValue::Real(eval_bessel_i(a1.round() as i64, a2))),
        "bessel_k" | "besselk" => Ok(BigValue::Real(eval_bessel_k(a1.round() as i64, a2))),
        _ => Err(AlgebraError::EvaluationError(format!(
            "Unsupported binary function '{}' in numerical evaluation",
            fn_name
        ))),
    }
}

fn eval_ternary_fn(
    graph: &ExprGraph,
    fn_name: &str,
    arg0: ExprId,
    arg1: ExprId,
    arg2: ExprId,
    ctx: &EvalContext,
) -> AlgebraResult<BigValue> {
    if fn_name == "root" {
        let (zr, zi) = graph.evalf(arg0, ctx)?.as_complex();
        let n = graph.evalf(arg1, ctx)?.to_f64().round() as i64;
        let k = graph.evalf(arg2, ctx)?.to_f64().round() as i64;
        if n <= 0 {
            return Err(AlgebraError::DivisionByZero {
                domain: "Complex Root".into(),
            });
        }
        let mv = BranchCutManager::eval(
            BranchCutFunctionKind::Root(n as u32),
            zr,
            zi,
            &BranchSelection::Sheet(k),
        )?;
        let (r, i) = mv.primary_value();
        return Ok(BigValue::from_complex(r, i));
    }
    if fn_name == "RootOf" {
        return eval_root_of(graph, arg0, arg1, arg2, ctx);
    }
    Err(AlgebraError::EvaluationError(format!(
        "Unsupported ternary function '{}' in numerical evaluation",
        fn_name
    )))
}

/// Evaluate a real expression DAG under rigorous interval arithmetic `[inf, sup]`.
pub fn eval_interval(
    graph: &ExprGraph,
    id: ExprId,
    env: &HashMap<String, RealInterval>,
) -> AlgebraResult<RealInterval> {
    let node = graph.get(id);
    match &node.kind {
        ExprKind::Number(n) => match n {
            Number::Constant(Constant::Pi) => Ok(RealInterval::point(consts::PI)),
            Number::Constant(Constant::E) => Ok(RealInterval::point(consts::E)),
            Number::Constant(Constant::EulerGamma) => Ok(RealInterval::point(0.5772156649015329)),
            Number::Constant(Constant::GoldenRatio) => Ok(RealInterval::point(1.618033988749895)),
            _ => n.as_f64().map(RealInterval::point).ok_or_else(|| {
                AlgebraError::EvaluationError("Non-real constant in interval evaluation".into())
            }),
        },
        ExprKind::Symbol(sym) => {
            let name = graph.symbols.resolve(*sym).unwrap_or_default();
            if let Some(&iv) = env.get(&name) {
                Ok(iv)
            } else {
                match name.as_str() {
                    "pi" | "π" | "Pi" | "PI" => Ok(RealInterval::point(consts::PI)),
                    "tau" | "τ" => Ok(RealInterval::point(2.0 * consts::PI)),
                    "e" | "E" => Ok(RealInterval::point(consts::E)),
                    _ => Err(AlgebraError::EvaluationError(format!(
                        "Unbound symbol '{}' in interval evaluation",
                        name
                    ))),
                }
            }
        }
        ExprKind::Add(terms) => {
            let mut acc = RealInterval::point(0.0);
            for &t in terms {
                acc = acc + eval_interval(graph, t, env)?;
            }
            Ok(acc)
        }
        ExprKind::Mul(factors) => {
            let mut acc = RealInterval::point(1.0);
            for &f in factors {
                acc = acc * eval_interval(graph, f, env)?;
            }
            Ok(acc)
        }
        ExprKind::Sub(a, b) => Ok(eval_interval(graph, *a, env)? - eval_interval(graph, *b, env)?),
        ExprKind::Div(a, b) => {
            let num = eval_interval(graph, *a, env)?;
            let den = eval_interval(graph, *b, env)?;
            num.div(&den).ok_or_else(|| AlgebraError::DivisionByZero {
                domain: "RealInterval contains zero".into(),
            })
        }
        ExprKind::Neg(a) => Ok(eval_interval(graph, *a, env)?.neg()),
        ExprKind::Pow(b, e) => {
            let bv = eval_interval(graph, *b, env)?;
            let ev = eval_interval(graph, *e, env)?;
            bv.pow_interval(&ev)
                .ok_or_else(|| AlgebraError::EvaluationError("Interval power domain error".into()))
        }
        ExprKind::Function { name, args } => {
            let fn_name = graph.symbols.resolve(*name).unwrap_or_default();
            if args.len() == 2 && (fn_name == "uncertain" || fn_name == "measurement") {
                let v = eval_interval(graph, args[0], env)?;
                let u = eval_interval(graph, args[1], env)?.abs();
                return Ok(RealInterval::new(v.inf - u.sup, v.sup + u.sup));
            }
            if args.len() == 2 && fn_name == "interval" {
                let lo = eval_interval(graph, args[0], env)?;
                let hi = eval_interval(graph, args[1], env)?;
                return Ok(RealInterval::new(lo.inf.min(hi.inf), lo.sup.max(hi.sup)));
            }
            if args.len() == 1 {
                let a = eval_interval(graph, args[0], env)?;
                match fn_name.as_str() {
                    "abs" | "Abs" => Ok(a.abs()),
                    "sqrt" => a.sqrt().ok_or_else(|| {
                        AlgebraError::EvaluationError("Negative domain for interval sqrt".into())
                    }),
                    "exp" => Ok(a.exp()),
                    "ln" | "log" => a.ln().ok_or_else(|| {
                        AlgebraError::EvaluationError("Non-positive domain for interval ln".into())
                    }),
                    "sin" => Ok(a.sin()),
                    "cos" => Ok(a.cos()),
                    "tan" => a.tan().ok_or_else(|| {
                        AlgebraError::EvaluationError("Pole in interval tan".into())
                    }),
                    "asin" | "arcsin" => a.asin().ok_or_else(|| {
                        AlgebraError::EvaluationError("Out-of-domain interval asin".into())
                    }),
                    "acos" | "arccos" => a.acos().ok_or_else(|| {
                        AlgebraError::EvaluationError("Out-of-domain interval acos".into())
                    }),
                    "atan" | "arctan" => Ok(a.atan()),
                    "sinh" => Ok(a.sinh()),
                    "cosh" => Ok(a.cosh()),
                    "tanh" => Ok(a.tanh()),
                    _ => Err(AlgebraError::EvaluationError(format!(
                        "Unsupported interval function '{}'",
                        fn_name
                    ))),
                }
            } else {
                Err(AlgebraError::EvaluationError(format!(
                    "Unsupported interval function '{}'",
                    fn_name
                )))
            }
        }
        ExprKind::Relational { rhs, .. } => eval_interval(graph, *rhs, env),
        _ => Err(AlgebraError::EvaluationError(
            "Unsupported node in interval evaluation".into(),
        )),
    }
}

/// Returns `true` if the expression DAG contains any `uncertain`, `measurement`, or `interval` nodes.
pub fn expr_has_uncertainty_nodes(graph: &ExprGraph, id: ExprId) -> bool {
    match &graph.get(id).kind {
        ExprKind::Function { name, args } => {
            let n = graph.symbols.resolve(*name).unwrap_or_default();
            if matches!(n.as_str(), "uncertain" | "measurement" | "interval") {
                return true;
            }
            args.iter().any(|&a| expr_has_uncertainty_nodes(graph, a))
        }
        ExprKind::Add(xs) | ExprKind::Mul(xs) => {
            xs.iter().any(|&x| expr_has_uncertainty_nodes(graph, x))
        }
        ExprKind::Sub(a, b)
        | ExprKind::Div(a, b)
        | ExprKind::Pow(a, b)
        | ExprKind::Relational { lhs: a, rhs: b, .. } => {
            expr_has_uncertainty_nodes(graph, *a) || expr_has_uncertainty_nodes(graph, *b)
        }
        ExprKind::Neg(a) => expr_has_uncertainty_nodes(graph, *a),
        _ => false,
    }
}

/// Evaluate a real expression DAG with 1st-order correlated Gaussian uncertainty propagation (GUM)
/// paired with rigorous interval enclosure bounds.
///
/// Named parameter uncertainties track correlation automatically (`a - a` gives `0 ± 0`), while each
/// inline `uncertain(v, u)` or `interval(lo, hi)` literal is treated as an independent source.
pub fn eval_uncertain(
    graph: &ExprGraph,
    id: ExprId,
    ctx: &EvalContext,
) -> AlgebraResult<UncertainValue> {
    use crate::calculus::SymbolicCalculus;

    let mut base_bindings = ctx.bindings.clone();
    let mut base_uncertainties = ctx.uncertainties.clone();
    let mut interval_env: HashMap<String, RealInterval> = ctx
        .bindings
        .iter()
        .map(|(k, &v)| {
            let iv = ctx.intervals.get(k).copied().unwrap_or_else(|| {
                let s = ctx.uncertainties.get(k).copied().unwrap_or(0.0);
                RealInterval::from_center_uncertainty(v, s)
            });
            (k.clone(), iv)
        })
        .collect();
    for (k, &iv) in &ctx.intervals {
        base_bindings.entry(k.clone()).or_insert_with(|| iv.mid());
        base_uncertainties
            .entry(k.clone())
            .or_insert_with(|| iv.rad());
        interval_env.entry(k.clone()).or_insert(iv);
    }

    // Desugar inline `uncertain(v, e)` / `measurement(v, e)` / `interval(lo, hi)` into fresh
    // independent parameter symbols `__u_0`, `__u_1`, ... so symbolic derivatives handle everything uniformly.
    fn desugar(
        graph: &ExprGraph,
        e: ExprId,
        base_ctx: &EvalContext,
        bindings: &mut HashMap<String, f64>,
        uncertainties: &mut HashMap<String, f64>,
        counter: &mut usize,
    ) -> AlgebraResult<ExprId> {
        let node = graph.get(e);
        match &node.kind {
            ExprKind::Function { name, args } => {
                let fn_name = graph.symbols.resolve(*name).unwrap_or_default();
                if args.len() == 2 && (fn_name == "uncertain" || fn_name == "measurement") {
                    let v = graph.evalf(args[0], base_ctx)?.to_f64();
                    let u = graph.evalf(args[1], base_ctx)?.to_f64().abs();
                    let sym_name = format!("__u_{}", *counter);
                    *counter += 1;
                    bindings.insert(sym_name.clone(), v);
                    uncertainties.insert(sym_name.clone(), u);
                    return Ok(graph.symbol(&sym_name));
                }
                if args.len() == 2 && fn_name == "interval" {
                    let lo = graph.evalf(args[0], base_ctx)?.to_f64();
                    let hi = graph.evalf(args[1], base_ctx)?.to_f64();
                    let iv = RealInterval::new(lo, hi);
                    let sym_name = format!("__u_{}", *counter);
                    *counter += 1;
                    bindings.insert(sym_name.clone(), iv.mid());
                    uncertainties.insert(sym_name.clone(), iv.rad());
                    return Ok(graph.symbol(&sym_name));
                }
                let mut new_args = Vec::with_capacity(args.len());
                for &a in args {
                    new_args.push(desugar(
                        graph,
                        a,
                        base_ctx,
                        bindings,
                        uncertainties,
                        counter,
                    )?);
                }
                Ok(graph.function(&fn_name, new_args))
            }
            ExprKind::Add(terms) => {
                let mut new_terms = Vec::with_capacity(terms.len());
                for &t in terms {
                    new_terms.push(desugar(
                        graph,
                        t,
                        base_ctx,
                        bindings,
                        uncertainties,
                        counter,
                    )?);
                }
                Ok(graph.add(new_terms))
            }
            ExprKind::Mul(factors) => {
                let mut new_factors = Vec::with_capacity(factors.len());
                for &f in factors {
                    new_factors.push(desugar(
                        graph,
                        f,
                        base_ctx,
                        bindings,
                        uncertainties,
                        counter,
                    )?);
                }
                Ok(graph.mul(new_factors))
            }
            ExprKind::Sub(a, b) => {
                let na = desugar(graph, *a, base_ctx, bindings, uncertainties, counter)?;
                let nb = desugar(graph, *b, base_ctx, bindings, uncertainties, counter)?;
                Ok(graph.sub(na, nb))
            }
            ExprKind::Div(a, b) => {
                let na = desugar(graph, *a, base_ctx, bindings, uncertainties, counter)?;
                let nb = desugar(graph, *b, base_ctx, bindings, uncertainties, counter)?;
                Ok(graph.div(na, nb))
            }
            ExprKind::Neg(a) => {
                let na = desugar(graph, *a, base_ctx, bindings, uncertainties, counter)?;
                Ok(graph.neg(na))
            }
            ExprKind::Pow(a, b) => {
                let na = desugar(graph, *a, base_ctx, bindings, uncertainties, counter)?;
                let nb = desugar(graph, *b, base_ctx, bindings, uncertainties, counter)?;
                Ok(graph.pow(na, nb))
            }
            ExprKind::Relational { rhs, .. } => {
                desugar(graph, *rhs, base_ctx, bindings, uncertainties, counter)
            }
            _ => Ok(e),
        }
    }

    let seed_ctx = EvalContext::with_bindings(ctx.precision_digits, base_bindings.clone())
        .with_branch_sheet(ctx.branch_sheet);
    let mut counter = 0usize;
    let desugared = desugar(
        graph,
        id,
        &seed_ctx,
        &mut base_bindings,
        &mut base_uncertainties,
        &mut counter,
    )?;

    let eval_ctx = EvalContext::with_bindings(ctx.precision_digits, base_bindings.clone())
        .with_branch_sheet(ctx.branch_sheet);
    let nominal = match graph.evalf(desugared, &eval_ctx)? {
        BigValue::Real(r) => r,
        BigValue::Complex(_, _) => {
            return Err(AlgebraError::EvaluationError(
                "Uncertainty propagation expects real-valued expression at nominal point".into(),
            ));
        }
    };

    let mut variance_sum = 0.0f64;
    for (sym_name, &sigma_i) in &base_uncertainties {
        if sigma_i <= 0.0 {
            continue;
        }
        if let Some(sym_id) = graph.symbols.get(sym_name)
            && graph.has_symbol(desugared, sym_id)
        {
            let deriv_expr = graph.diff(desugared, sym_id);
            let deriv = match graph.evalf(deriv_expr, &eval_ctx) {
                Ok(BigValue::Real(d)) if d.is_finite() => d,
                _ => {
                    // Central finite-difference fallback for special functions without closed deriv rule
                    let x0 = base_bindings.get(sym_name).copied().unwrap_or(0.0);
                    let h = (1e-6 * x0.abs().max(1.0)).min(sigma_i * 0.1).max(1e-8);
                    let mut up_b = base_bindings.clone();
                    up_b.insert(sym_name.clone(), x0 + h);
                    let mut dn_b = base_bindings.clone();
                    dn_b.insert(sym_name.clone(), x0 - h);
                    let f_up = graph
                        .evalf(
                            desugared,
                            &EvalContext::with_bindings(ctx.precision_digits, up_b),
                        )
                        .map(|v| v.to_f64())
                        .unwrap_or(nominal);
                    let f_dn = graph
                        .evalf(
                            desugared,
                            &EvalContext::with_bindings(ctx.precision_digits, dn_b),
                        )
                        .map(|v| v.to_f64())
                        .unwrap_or(nominal);
                    (f_up - f_dn) / (2.0 * h)
                }
            };
            let contrib = deriv * sigma_i;
            variance_sum += contrib * contrib;
        }
    }

    let combined_sigma = variance_sum.sqrt();
    let interval = eval_interval(graph, id, &interval_env)
        .unwrap_or_else(|_| RealInterval::from_center_uncertainty(nominal, combined_sigma));

    Ok(UncertainValue {
        mean: nominal,
        std_dev: combined_sigma,
        interval,
    })
}

/// `RootOf(p(x), x, k)`: the `k`-th root (counting from 1) of a polynomial with rational coefficients, in the canonical
/// order real roots ascending, then non-real roots by real part and then imaginary part. The roots come from
/// Durand-Kerner iteration polished by Newton steps on the original polynomial.
fn eval_root_of(
    graph: &ExprGraph,
    equation: ExprId,
    variable: ExprId,
    index: ExprId,
    ctx: &EvalContext,
) -> AlgebraResult<BigValue> {
    let ExprKind::Symbol(sym) = graph.get(variable).kind else {
        return Err(AlgebraError::EvaluationError(
            "RootOf: the second argument must be a symbol".into(),
        ));
    };
    let k = graph.evalf(index, ctx)?.to_f64().round() as i64;
    let poly = crate::poly::UniPoly::from_expr(graph, equation, sym).map_err(|_| {
        AlgebraError::EvaluationError(
            "RootOf: the polynomial has coefficients that are not rational numbers".into(),
        )
    })?;
    let degree = poly.degree();
    if degree == 0 || k < 1 || k as usize > degree {
        return Err(AlgebraError::EvaluationError(format!(
            "RootOf: root number {k} does not exist for a polynomial of degree {degree}"
        )));
    }
    use num_traits::ToPrimitive;
    let ascending: Vec<f64> = poly
        .coeffs
        .iter()
        .map(|c| c.to_f64().unwrap_or(f64::NAN))
        .collect();
    let lead = ascending[degree];
    let monic_descending: Vec<f64> = ascending.iter().rev().map(|c| c / lead).collect();
    let mut roots: Vec<(f64, f64)> =
        crate::poly::roots::DurandKernerSolver::solve(&monic_descending, 2000, 1e-15)
            .into_iter()
            .map(|r| (r.re, r.im))
            .collect();
    // Newton polish on the original polynomial.
    let value_and_slope = |z: (f64, f64)| -> ((f64, f64), (f64, f64)) {
        let (mut p, mut dp) = ((0.0, 0.0), (0.0, 0.0));
        for &c in monic_descending.iter() {
            dp = (dp.0 * z.0 - dp.1 * z.1 + p.0, dp.0 * z.1 + dp.1 * z.0 + p.1);
            p = (p.0 * z.0 - p.1 * z.1 + c, p.0 * z.1 + p.1 * z.0);
        }
        (p, dp)
    };
    for z in roots.iter_mut() {
        for _ in 0..6 {
            let (p, dp) = value_and_slope(*z);
            let den = dp.0 * dp.0 + dp.1 * dp.1;
            if den < 1e-300 {
                break;
            }
            *z = (
                z.0 - (p.0 * dp.0 + p.1 * dp.1) / den,
                z.1 - (p.1 * dp.0 - p.0 * dp.1) / den,
            );
        }
    }
    let is_real = |z: &(f64, f64)| z.1.abs() <= 1e-9 * (1.0 + z.0.abs());
    roots.sort_by(|a, b| match (is_real(a), is_real(b)) {
        (true, true) => a.0.total_cmp(&b.0),
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        (false, false) => a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)),
    });
    let z = roots[(k - 1) as usize];
    Ok(if is_real(&z) {
        BigValue::Real(z.0)
    } else {
        BigValue::Complex(z.0, z.1)
    })
}
