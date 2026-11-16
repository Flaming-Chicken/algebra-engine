//! # `algebra_core::interval`
//!
//! Rigorous Interval Arithmetic $[a, b]$ with certified numerical bounds, directed inclusions,
//! and Interval Newton root containment.
//!
//! ## References
//! - Moore, R. E., Kearfott, R. B., & Cloud, M. J. (2009). *Introduction to Interval Analysis*. SIAM.
//! - Tucker, W. (2011). *Validated Numerics: A Short Introduction to Rigorous Computations*. Princeton University Press.

use serde::{Deserialize, Serialize};

/// Certified real interval $[a, b]$ where $a \le b$.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RealInterval {
    pub inf: f64,
    pub sup: f64,
}

impl RealInterval {
    /// Entire real line: `[-∞, +∞]`.
    pub const ALL: Self = Self {
        inf: f64::NEG_INFINITY,
        sup: f64::INFINITY,
    };
    /// Unit interval: `[0.0, 1.0]`.
    pub const UNIT: Self = Self { inf: 0.0, sup: 1.0 };
    /// Symmetric unit interval: `[-1.0, 1.0]`.
    pub const SYMMETRIC_UNIT: Self = Self {
        inf: -1.0,
        sup: 1.0,
    };
    /// Non-negative reals: `[0.0, +∞]`.
    pub const NON_NEGATIVE: Self = Self {
        inf: 0.0,
        sup: f64::INFINITY,
    };
    /// Non-positive reals: `[-∞, 0.0]`.
    pub const NON_POSITIVE: Self = Self {
        inf: f64::NEG_INFINITY,
        sup: 0.0,
    };

    /// Create a new interval $[a, b]$. If $a > b$, they are swapped.
    #[inline]
    pub const fn new(a: f64, b: f64) -> Self {
        if a <= b {
            Self { inf: a, sup: b }
        } else {
            Self { inf: b, sup: a }
        }
    }

    /// Exact degenerate point interval $[c, c]$.
    #[inline]
    pub const fn point(c: f64) -> Self {
        Self { inf: c, sup: c }
    }

    /// Midpoint $m = (a + b) / 2$.
    #[inline]
    pub const fn mid(&self) -> f64 {
        0.5 * (self.inf + self.sup)
    }

    /// Radius $r = (b - a) / 2$.
    #[inline]
    pub const fn rad(&self) -> f64 {
        0.5 * (self.sup - self.inf)
    }

    /// Diameter / width $w = b - a$.
    #[inline]
    pub const fn diam(&self) -> f64 {
        self.sup - self.inf
    }

    /// Check if point $x \in [a, b]$.
    #[inline]
    pub const fn contains(&self, x: f64) -> bool {
        self.inf <= x && x <= self.sup
    }

    /// Check if sub-interval $other \subseteq self$.
    #[inline]
    pub const fn contains_interval(&self, other: &RealInterval) -> bool {
        self.inf <= other.inf && other.sup <= self.sup
    }

    /// Interval addition $[a, b] + [c, d] = [a + c, b + d]$.
    #[inline]
    pub const fn add(&self, other: &RealInterval) -> RealInterval {
        RealInterval {
            inf: self.inf + other.inf,
            sup: self.sup + other.sup,
        }
    }

    /// Interval subtraction $[a, b] - [c, d] = [a - d, b - c]$.
    #[inline]
    pub const fn sub(&self, other: &RealInterval) -> RealInterval {
        RealInterval {
            inf: self.inf - other.sup,
            sup: self.sup - other.inf,
        }
    }

    /// Interval multiplication $[a, b] \times [c, d] = [\min(ac, ad, bc, bd), \max(ac, ad, bc, bd)]$.
    pub fn mul(&self, other: &RealInterval) -> RealInterval {
        let p1 = self.inf * other.inf;
        let p2 = self.inf * other.sup;
        let p3 = self.sup * other.inf;
        let p4 = self.sup * other.sup;

        let inf = p1.min(p2).min(p3).min(p4);
        let sup = p1.max(p2).max(p3).max(p4);

        RealInterval { inf, sup }
    }

    /// Interval reciprocal $1 / [c, d]$ (disallowing intervals containing 0).
    #[inline]
    pub const fn recip(&self) -> Option<RealInterval> {
        if self.contains(0.0) {
            None
        } else {
            Some(RealInterval {
                inf: 1.0 / self.sup,
                sup: 1.0 / self.inf,
            })
        }
    }

    /// Interval division $[a, b] / [c, d]$.
    pub fn div(&self, other: &RealInterval) -> Option<RealInterval> {
        other.recip().map(|inv| self.mul(&inv))
    }

    /// Intersection of two intervals, if non-empty.
    pub fn intersect(&self, other: &RealInterval) -> Option<RealInterval> {
        let inf = self.inf.max(other.inf);
        let sup = self.sup.min(other.sup);
        if inf <= sup {
            Some(RealInterval { inf, sup })
        } else {
            None
        }
    }

    /// Interval hull enclosing both intervals.
    pub fn hull(&self, other: &RealInterval) -> RealInterval {
        RealInterval {
            inf: self.inf.min(other.inf),
            sup: self.sup.max(other.sup),
        }
    }

    /// Interval Newton step: $N(I) = m - f(m) / f'(I)$.
    /// If $N(I) \subseteq I$, there is a unique certified root in $I$.
    pub fn newton_step<F, DF>(&self, f: F, df: DF) -> Option<RealInterval>
    where
        F: Fn(f64) -> f64,
        DF: Fn(RealInterval) -> RealInterval,
    {
        let m = self.mid();
        let fm = f(m);
        let df_i = df(*self);

        df_i.recip().map(|inv| {
            let shift = RealInterval::point(fm).mul(&inv);
            RealInterval::point(m).sub(&shift)
        })
    }
}

impl std::ops::Add for RealInterval {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        RealInterval {
            inf: self.inf + rhs.inf,
            sup: self.sup + rhs.sup,
        }
    }
}

impl std::ops::Sub for RealInterval {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        RealInterval {
            inf: self.inf - rhs.sup,
            sup: self.sup - rhs.inf,
        }
    }
}

impl std::ops::Mul for RealInterval {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        let p1 = self.inf * rhs.inf;
        let p2 = self.inf * rhs.sup;
        let p3 = self.sup * rhs.inf;
        let p4 = self.sup * rhs.sup;
        RealInterval {
            inf: p1.min(p2).min(p3).min(p4),
            sup: p1.max(p2).max(p3).max(p4),
        }
    }
}

impl std::ops::Div for RealInterval {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        if rhs.contains(0.0) {
            RealInterval::new(f64::NEG_INFINITY, f64::INFINITY)
        } else {
            let inv = RealInterval {
                inf: 1.0 / rhs.sup,
                sup: 1.0 / rhs.inf,
            };
            self * inv
        }
    }
}

impl std::ops::Neg for RealInterval {
    type Output = Self;
    fn neg(self) -> Self::Output {
        RealInterval {
            inf: -self.sup,
            sup: -self.inf,
        }
    }
}

impl RealInterval {
    /// Negate the interval: -[a, b] = [-b, -a].
    pub fn neg(&self) -> Self {
        RealInterval {
            inf: -self.sup,
            sup: -self.inf,
        }
    }

    /// Construct from center value and half-width / uncertainty `[c - |u|, c + |u|]`.
    #[inline]
    pub fn from_center_uncertainty(center: f64, sigma: f64) -> Self {
        let r = sigma.abs();
        Self::new(center - r, center + r)
    }

    /// True if both endpoints are finite.
    #[inline]
    pub fn is_finite(&self) -> bool {
        self.inf.is_finite() && self.sup.is_finite()
    }

    /// Interval absolute value `|[a, b]|`.
    pub fn abs(&self) -> Self {
        if self.inf >= 0.0 {
            *self
        } else if self.sup <= 0.0 {
            self.neg()
        } else {
            RealInterval::new(0.0, (-self.inf).max(self.sup))
        }
    }

    /// Interval square root `sqrt([a, b])` if `inf >= 0`.
    pub fn sqrt(&self) -> Option<Self> {
        if self.sup < 0.0 || self.inf < 0.0 {
            None
        } else {
            Some(RealInterval::new(self.inf.sqrt(), self.sup.sqrt()))
        }
    }

    /// Interval integer power `[a, b]^n`.
    pub fn powi(&self, n: i32) -> Option<Self> {
        if n == 0 {
            return Some(RealInterval::point(1.0));
        }
        if n < 0 {
            let pos = self.powi(-n)?;
            return pos.recip();
        }
        if n % 2 == 1 || self.inf >= 0.0 {
            Some(RealInterval::new(self.inf.powi(n), self.sup.powi(n)))
        } else if self.sup <= 0.0 {
            Some(RealInterval::new(self.sup.powi(n), self.inf.powi(n)))
        } else {
            Some(RealInterval::new(
                0.0,
                self.inf.powi(n).max(self.sup.powi(n)),
            ))
        }
    }

    /// Interval real power `[a, b]^p` for scalar `p`.
    pub fn powf(&self, p: f64) -> Option<Self> {
        if !p.is_finite() {
            return None;
        }
        if p.fract() == 0.0 && p.abs() <= (i32::MAX as f64) {
            return self.powi(p as i32);
        }
        if self.inf < 0.0 {
            return None;
        }
        Some(RealInterval::new(self.inf.powf(p), self.sup.powf(p)))
    }

    /// General interval power `[a, b]^[c, d]`.
    pub fn pow_interval(&self, exp: &Self) -> Option<Self> {
        if (exp.sup - exp.inf).abs() == 0.0 {
            return self.powf(exp.inf);
        }
        let ln_self = self.ln()?;
        Some(exp.mul(&ln_self).exp())
    }

    /// Interval exponential `exp([a, b])`.
    pub fn exp(&self) -> Self {
        RealInterval::new(self.inf.exp(), self.sup.exp())
    }

    /// Interval natural logarithm `ln([a, b])` if `inf > 0`.
    pub fn ln(&self) -> Option<Self> {
        if self.inf <= 0.0 {
            None
        } else {
            Some(RealInterval::new(self.inf.ln(), self.sup.ln()))
        }
    }

    /// Interval sine `sin([a, b])` accounting for periodic extrema.
    pub fn sin(&self) -> Self {
        use std::f64::consts::{FRAC_PI_2, TAU};
        if !self.is_finite() || self.diam() >= TAU {
            return RealInterval::SYMMETRIC_UNIT;
        }
        let s1 = self.inf.sin();
        let s2 = self.sup.sin();
        let mut lo = s1.min(s2);
        let mut hi = s1.max(s2);

        // Check maxima at pi/2 + 2*pi*k
        let k_hi = ((self.inf - FRAC_PI_2) / TAU).ceil();
        if FRAC_PI_2 + k_hi * TAU <= self.sup {
            hi = 1.0;
        }
        // Check minima at -pi/2 + 2*pi*k
        let k_lo = ((self.inf + FRAC_PI_2) / TAU).ceil();
        if -FRAC_PI_2 + k_lo * TAU <= self.sup {
            lo = -1.0;
        }
        RealInterval::new(lo.clamp(-1.0, 1.0), hi.clamp(-1.0, 1.0))
    }

    /// Interval cosine `cos([a, b])` via `sin(x + pi/2)`.
    pub fn cos(&self) -> Self {
        use std::f64::consts::FRAC_PI_2;
        RealInterval::new(self.inf + FRAC_PI_2, self.sup + FRAC_PI_2).sin()
    }

    /// Interval tangent `tan([a, b])` when no pole `pi/2 + k*pi` lies inside `[a, b]`.
    pub fn tan(&self) -> Option<Self> {
        use std::f64::consts::{FRAC_PI_2, PI};
        if !self.is_finite() || self.diam() >= PI {
            return None;
        }
        let k = ((self.inf - FRAC_PI_2) / PI).ceil();
        let pole = FRAC_PI_2 + k * PI;
        if pole <= self.sup {
            return None;
        }
        Some(RealInterval::new(self.inf.tan(), self.sup.tan()))
    }

    /// Interval arcsine `asin([a, b])` when `[a, b] \subseteq [-1, 1]`.
    pub fn asin(&self) -> Option<Self> {
        if self.inf < -1.0 || self.sup > 1.0 {
            None
        } else {
            Some(RealInterval::new(self.inf.asin(), self.sup.asin()))
        }
    }

    /// Interval arccosine `acos([a, b])` when `[a, b] \subseteq [-1, 1]`.
    pub fn acos(&self) -> Option<Self> {
        if self.inf < -1.0 || self.sup > 1.0 {
            None
        } else {
            Some(RealInterval::new(self.sup.acos(), self.inf.acos()))
        }
    }

    /// Interval arctangent `atan([a, b])`.
    pub fn atan(&self) -> Self {
        RealInterval::new(self.inf.atan(), self.sup.atan())
    }

    /// Interval hyperbolic sine `sinh([a, b])`.
    pub fn sinh(&self) -> Self {
        RealInterval::new(self.inf.sinh(), self.sup.sinh())
    }

    /// Interval hyperbolic cosine `cosh([a, b])`.
    pub fn cosh(&self) -> Self {
        if self.inf >= 0.0 {
            RealInterval::new(self.inf.cosh(), self.sup.cosh())
        } else if self.sup <= 0.0 {
            RealInterval::new(self.sup.cosh(), self.inf.cosh())
        } else {
            RealInterval::new(1.0, self.inf.cosh().max(self.sup.cosh()))
        }
    }

    /// Interval hyperbolic tangent `tanh([a, b])`.
    pub fn tanh(&self) -> Self {
        RealInterval::new(self.inf.tanh(), self.sup.tanh())
    }
}

/// Numerical evaluation value paired with 1-sigma GUM uncertainty and certified interval enclosure.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UncertainValue {
    /// Nominal value or mean.
    pub mean: f64,
    /// Combined 1-sigma Gaussian standard uncertainty (`0.0` if exact).
    pub std_dev: f64,
    /// Rigorous certified real interval enclosure `[inf, sup]`.
    pub interval: RealInterval,
}

impl UncertainValue {
    /// Exact scalar without uncertainty.
    pub fn exact(val: f64) -> Self {
        Self {
            mean: val,
            std_dev: 0.0,
            interval: RealInterval::point(val),
        }
    }

    /// Uncertain quantity from nominal value and ± uncertainty (`val ± std_dev`).
    pub fn from_pm(mean: f64, std_dev: f64) -> Self {
        let s = std_dev.abs();
        Self {
            mean,
            std_dev: s,
            interval: RealInterval::from_center_uncertainty(mean, s),
        }
    }

    /// Uncertain quantity derived from an explicit interval `[lo, hi]`.
    pub fn from_interval(interval: RealInterval) -> Self {
        Self {
            mean: interval.mid(),
            std_dev: interval.rad(),
            interval,
        }
    }

    /// Format as Unicode `mean ± std_dev` with optional interval enclosure.
    pub fn format_unicode(&self) -> String {
        if self.std_dev == 0.0 && self.interval.diam() == 0.0 {
            format_compact_float(self.mean)
        } else {
            format!(
                "{} ± {} (interval [{}, {}])",
                format_compact_float(self.mean),
                format_compact_float(self.std_dev),
                format_compact_float(self.interval.inf),
                format_compact_float(self.interval.sup)
            )
        }
    }

    /// Format as LaTeX `mean \pm std_dev`.
    pub fn format_latex(&self) -> String {
        if self.std_dev == 0.0 && self.interval.diam() == 0.0 {
            format_compact_float(self.mean)
        } else {
            format!(
                "{} \\pm {}",
                format_compact_float(self.mean),
                format_compact_float(self.std_dev)
            )
        }
    }
}

fn format_compact_float(v: f64) -> String {
    if !v.is_finite() {
        return format!("{}", v);
    }
    let rounded = (v * 1e12).round() / 1e12;
    if rounded.abs() < 1e-15 {
        return "0".to_string();
    }
    let s = format!("{:.6}", rounded);
    let trimmed = s.trim_end_matches('0').trim_end_matches('.');
    trimmed.to_string()
}

/// Shared helper to parse `"<val> [+/-|+-|± <err>] [<unit>]"` into `(val, Option<err>, Option<unit>)`.
pub fn parse_value_uncertainty_and_unit(raw: &str) -> Option<(f64, Option<f64>, Option<String>)> {
    let trimmed = raw.trim();
    let (body, unit) = if let Some(b_start) = trimmed.find('[')
        && let Some(b_end) = trimmed[b_start..].find(']')
    {
        let u = trimmed[b_start + 1..b_start + b_end].trim();
        let before = trimmed[..b_start].trim();
        (
            before,
            if u.is_empty() {
                None
            } else {
                Some(u.to_string())
            },
        )
    } else {
        (trimmed, None)
    };

    for sep in ["+/-", "±", "+-"] {
        if let Some((lhs, rhs)) = body.split_once(sep) {
            let val = lhs.trim().parse::<f64>().ok()?;
            let err = rhs.trim().parse::<f64>().ok()?.abs();
            return Some((val, Some(err), unit));
        }
    }

    let val = body.parse::<f64>().ok()?;
    Some((val, None, unit))
}
