//! Scalar numbers, arbitrary-precision rationals, exact scientific notation, and mathematical constants.

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, ToPrimitive, Zero};
use std::fmt;

/// Built-in mathematical constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Constant {
    /// Archimedian constant Pi (π ≈ 3.14159...)
    Pi,
    /// Euler's number e (e ≈ 2.71828...)
    E,
    /// Imaginary unit i (i^2 = -1)
    I,
    /// Euler-Mascheroni constant γ (γ ≈ 0.57721...)
    EulerGamma,
    /// Golden ratio φ (φ ≈ 1.61803...)
    GoldenRatio,
    /// Apéry's constant ζ(3) (ζ(3) ≈ 1.20205...)
    Apery,
    /// Catalan's constant G (G ≈ 0.91596...)
    Catalan,
    /// Positive infinity (+∞)
    Infinity,
    /// Negative infinity (-∞)
    NegInfinity,
    /// Undefined / Not-a-Number mathematical result (NaN / Complex infinity)
    Undefined,
}

impl fmt::Display for Constant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pi => write!(f, "π"),
            Self::E => write!(f, "e"),
            Self::I => write!(f, "i"),
            Self::EulerGamma => write!(f, "γ"),
            Self::GoldenRatio => write!(f, "φ"),
            Self::Apery => write!(f, "ζ(3)"),
            Self::Catalan => write!(f, "G"),
            Self::Infinity => write!(f, "∞"),
            Self::NegInfinity => write!(f, "-∞"),
            Self::Undefined => write!(f, "Undefined"),
        }
    }
}

/// Greatest Common Divisor helper for 64-bit integers.
pub const fn gcd_i64(a: i64, b: i64) -> i64 {
    let mut x = a.unsigned_abs();
    let mut y = b.unsigned_abs();
    while y != 0 {
        let t = y;
        y = x % y;
        x = t;
    }
    if x > i64::MAX as u64 {
        i64::MAX
    } else {
        x as i64
    }
}

/// Scalar numbers representation used in expression DAG nodes.
///
/// **Lossless by Default**: All machine integers, rationals, and decimals decompose
/// into exact lossless types, with arbitrary-precision auto-promotion (`BigInteger`,
/// `BigRational`) when machine limits are exceeded.
/// Lossy floating-point representations (`Float`) are strictly reserved for explicit
/// numerical approximations (`evalf`, `N`), interactive sliders, or insoluble fallbacks.
#[derive(Debug, Clone, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Number {
    /// Exact 64-bit signed integer.
    Integer(i64),
    /// Arbitrary-precision promoted integer.
    BigInteger(BigInt),
    /// Exact rational fraction (numerator, denominator) in canonical reduced form.
    Rational(i64, i64),
    /// Arbitrary-precision promoted rational fraction.
    BigRational(Box<BigRational>),
    /// Exact scientific notation: mantissa * 10^exponent using lossless integers.
    Scientific { mantissa: i64, exponent: i32 },
    /// 64-bit floating point number (used ONLY for explicit lossy approximation or fallback).
    Float(u64),
    /// Mathematical constant.
    Constant(Constant),
}

impl Constant {
    /// Numerical floating-point approximation of constant.
    pub const fn approx_f64(&self) -> Option<f64> {
        match self {
            Self::Pi => Some(std::f64::consts::PI),
            Self::E => Some(std::f64::consts::E),
            Self::EulerGamma => Some(0.5772156649015329),
            Self::GoldenRatio => Some(1.618033988749895),
            Self::Apery => Some(1.202056903159594),
            Self::Catalan => Some(0.915965594177219),
            Self::Infinity => Some(f64::INFINITY),
            Self::NegInfinity => Some(f64::NEG_INFINITY),
            Self::I | Self::Undefined => None,
        }
    }
}

impl Number {
    /// Static Zero constant: `0`.
    pub const ZERO: Self = Self::Integer(0);
    /// Static Unity constant: `1`.
    pub const ONE: Self = Self::Integer(1);
    /// Static Pi constant: `π`.
    pub const PI: Self = Self::Constant(Constant::Pi);
    /// Static Euler's Number constant: `e`.
    pub const E: Self = Self::Constant(Constant::E);
    /// Static Imaginary Unit: `i`.
    pub const I: Self = Self::Constant(Constant::I);
    /// Static Positive Infinity: `+∞`.
    pub const INFINITY: Self = Self::Constant(Constant::Infinity);
    /// Static Negative Infinity: `-∞`.
    pub const NEG_INFINITY: Self = Self::Constant(Constant::NegInfinity);
    /// Static Undefined constant: `Undefined`.
    pub const UNDEFINED: Self = Self::Constant(Constant::Undefined);

    /// Create an exact integer number.
    #[inline]
    pub const fn integer(val: i64) -> Self {
        Self::Integer(val)
    }

    /// Create an exact rational fraction in canonical reduced form with gcd=1 and den > 0.
    pub const fn rational(mut num: i64, mut den: i64) -> Self {
        if den == 0 {
            return Self::Constant(Constant::Undefined);
        }
        if num == 0 {
            return Self::Integer(0);
        }
        if den < 0 && num != i64::MIN && den != i64::MIN {
            num = -num;
            den = -den;
        }
        let g = gcd_i64(num, den);
        if g > 1 {
            num /= g;
            den /= g;
        }
        if den < 0 && num != i64::MIN && den != i64::MIN {
            num = -num;
            den = -den;
        }
        if den == 1 {
            Self::Integer(num)
        } else {
            Self::Rational(num, den)
        }
    }

    /// Create an exact scientific notation number: $m \times 10^e$.
    pub fn scientific(mantissa: i64, exponent: i32) -> Self {
        if mantissa == 0 {
            return Self::Integer(0);
        }
        if exponent == 0 {
            return Self::Integer(mantissa);
        }
        if exponent > 0 && exponent <= 18 {
            if let Some(pow10) = 10i64.checked_pow(exponent as u32)
                && let Some(val) = mantissa.checked_mul(pow10)
            {
                return Self::Integer(val);
            }
        } else if (-18..0).contains(&exponent) {
            let neg_exp = (-exponent) as u32;
            if let Some(pow10) = 10i64.checked_pow(neg_exp) {
                return Self::rational(mantissa, pow10);
            }
        }
        Self::Scientific { mantissa, exponent }
    }

    /// Create a mathematical constant.
    #[inline]
    pub const fn constant(c: Constant) -> Self {
        Self::Constant(c)
    }

    /// Create an explicit lossy float number from raw IEEE-754 bit pattern.
    #[inline]
    pub const fn from_bits(bits: u64) -> Self {
        Self::Float(Self::canonical_float_bits(bits))
    }

    /// The bit pattern a float is stored under: `-0.0` is stored as `+0.0` and every NaN as one quiet NaN,
    /// so that numbers that compare equal also hash equal (the node identity of the expression arena
    /// is the bits, and `0.0` and `-0.0` used to be two different nodes). The sign of a zero is therefore
    /// not preserved inside an expression; evaluation of operations that depend on it (such as
    /// `1 / x`) uses the stored `+0.0`.
    pub const fn canonical_float_bits(bits: u64) -> u64 {
        const EXPONENT: u64 = 0x7ff0_0000_0000_0000;
        const MANTISSA: u64 = 0x000f_ffff_ffff_ffff;
        if bits << 1 == 0 {
            0
        } else if bits & EXPONENT == EXPONENT && bits & MANTISSA != 0 {
            0x7ff8_0000_0000_0000
        } else {
            bits
        }
    }

    /// Create an explicit lossy float number from `f64`.
    #[inline]
    pub const fn float(val: f64) -> Self {
        Self::Float(Self::canonical_float_bits(val.to_bits()))
    }

    /// Returns `true` if this number is exact and lossless (not an approximate Float).
    #[inline]
    pub fn is_lossless(&self) -> bool {
        !matches!(self, Self::Float(_))
    }

    /// Decompose a decimal or scientific string losslessly into an exact Number.
    pub fn from_decimal_str(s: &str) -> Option<Self> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return None;
        }

        // 1. Check scientific notation e.g. "1.5e-3", "2E6"
        if let Some(e_idx) = trimmed.find(['e', 'E']) {
            let base_str = &trimmed[..e_idx];
            let exp_str = &trimmed[e_idx + 1..];
            let exp_val: i32 = exp_str.parse().ok()?;

            if let Some(dot_idx) = base_str.find('.') {
                let int_part = &base_str[..dot_idx];
                let frac_part = &base_str[dot_idx + 1..];
                let frac_len = frac_part.len() as i32;
                let combined = format!("{}{}", int_part, frac_part);
                let mantissa: i64 = combined.parse().ok()?;
                let final_exp = exp_val - frac_len;
                return Some(Self::scientific(mantissa, final_exp));
            } else {
                let mantissa: i64 = base_str.parse().ok()?;
                return Some(Self::scientific(mantissa, exp_val));
            }
        }

        // 2. Standard decimal e.g. "0.5", "1.25", "-3.1415"
        if let Some(dot_idx) = trimmed.find('.') {
            let int_part = &trimmed[..dot_idx];
            let frac_part = &trimmed[dot_idx + 1..];
            let frac_len = frac_part.len() as u32;

            if frac_len == 0 {
                let int_val: i64 = int_part.parse().ok()?;
                return Some(Self::Integer(int_val));
            }

            if frac_len <= 18 {
                let combined = format!("{}{}", int_part, frac_part);
                if let Ok(num) = combined.parse::<i64>() {
                    let den: i64 = 10i64.pow(frac_len);
                    return Some(Self::rational(num, den));
                }
            }

            // Arbitrary-precision decimal fraction
            let combined = format!("{}{}", int_part, frac_part);
            let big_num = combined.parse::<BigInt>().ok()?;
            let big_den = num_traits::pow(BigInt::from(10), frac_len as usize);
            let r = BigRational::new(big_num, big_den);
            return Some(Self::BigRational(Box::new(r)));
        }

        if let Ok(i) = trimmed.parse::<i64>() {
            return Some(Self::Integer(i));
        }
        if let Ok(bi) = trimmed.parse::<BigInt>() {
            return Some(Self::BigInteger(bi));
        }

        None
    }

    /// Decompose an `f64` into a lossless representation using continued fractions / decimal expansion.
    pub fn from_f64_lossless(val: f64) -> Self {
        if val.is_nan() {
            return Self::Constant(Constant::Undefined);
        }
        if val == f64::INFINITY {
            return Self::Constant(Constant::Infinity);
        }
        if val == f64::NEG_INFINITY {
            return Self::Constant(Constant::NegInfinity);
        }
        if val == 0.0 {
            return Self::Integer(0);
        }
        if val.fract() == 0.0 && val >= i64::MIN as f64 && val <= i64::MAX as f64 {
            return Self::Integer(val as i64);
        }

        // Continued fraction convergent search up to 1e-12 tolerance
        let sign = if val < 0.0 { -1.0 } else { 1.0 };
        let abs_val = val.abs();
        let mut x = abs_val;
        let mut h_prev2 = 0i64;
        let mut h_prev1 = 1i64;
        let mut k_prev2 = 1i64;
        let mut k_prev1 = 0i64;

        for _ in 0..20 {
            let a = x.floor() as i64;
            let h = match a.checked_mul(h_prev1).and_then(|t| t.checked_add(h_prev2)) {
                Some(v) => v,
                None => break,
            };
            let k = match a.checked_mul(k_prev1).and_then(|t| t.checked_add(k_prev2)) {
                Some(v) => v,
                None => break,
            };

            let frac_approx = h as f64 / k as f64;
            if (frac_approx - abs_val).abs() < 1e-12 {
                let num = if sign < 0.0 { -h } else { h };
                return Self::rational(num, k);
            }

            h_prev2 = h_prev1;
            h_prev1 = h;
            k_prev2 = k_prev1;
            k_prev1 = k;

            let remainder = x - (a as f64);
            if remainder.abs() < 1e-12 {
                let num = if sign < 0.0 { -h } else { h };
                return Self::rational(num, k);
            }
            x = 1.0 / remainder;
        }

        // Decimal string fallback
        let s = format!("{}", val);
        if let Some(dec) = Self::from_decimal_str(&s) {
            return dec;
        }

        Self::float(val)
    }

    /// Decompose an `f32` into a lossless representation.
    pub fn from_f32_lossless(val: f32) -> Self {
        Self::from_f64_lossless(val as f64)
    }

    /// Extract `f64` float value if numerical evaluation or approximation is requested.
    #[inline]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Float(bits) => Some(f64::from_bits(*bits)),
            Self::Integer(i) => Some(*i as f64),
            Self::BigInteger(b) => b.to_f64(),
            Self::Rational(num, den) => Some(*num as f64 / *den as f64),
            Self::BigRational(r) => r.to_f64(),
            Self::Scientific { mantissa, exponent } => {
                Some(*mantissa as f64 * 10.0f64.powi(*exponent))
            }
            Self::Constant(c) => c.approx_f64(),
        }
    }

    /// Returns `true` if number represents mathematical zero.
    #[inline]
    pub fn is_zero(&self) -> bool {
        match self {
            Self::Integer(i) => *i == 0,
            Self::BigInteger(b) => b.is_zero(),
            Self::Rational(num, _) => *num == 0,
            Self::BigRational(r) => r.is_zero(),
            Self::Scientific { mantissa, .. } => *mantissa == 0,
            Self::Float(bits) => f64::from_bits(*bits) == 0.0,
            _ => false,
        }
    }

    /// Returns `true` if number represents mathematical unity (1).
    #[inline]
    pub fn is_one(&self) -> bool {
        match self {
            Self::Integer(i) => *i == 1,
            Self::BigInteger(b) => b.is_one(),
            Self::Rational(num, den) => *num == *den && *den != 0,
            Self::BigRational(r) => r.is_one(),
            Self::Scientific { mantissa, exponent } => *mantissa == 1 && *exponent == 0,
            Self::Float(bits) => f64::from_bits(*bits) == 1.0,
            _ => false,
        }
    }

    /// Lossless addition with auto-promotion to `BigInteger` / `BigRational` on overflow.
    pub fn add_lossless(&self, rhs: &Self) -> Option<Self> {
        match (self, rhs) {
            (Self::Integer(a), Self::Integer(b)) => {
                if let Some(c) = a.checked_add(*b) {
                    Some(Self::Integer(c))
                } else {
                    Some(Self::BigInteger(BigInt::from(*a) + BigInt::from(*b)))
                }
            }
            (Self::Rational(n1, d1), Self::Rational(n2, d2)) => {
                if let (Some(t1), Some(t2), Some(d)) = (
                    n1.checked_mul(*d2),
                    n2.checked_mul(*d1),
                    d1.checked_mul(*d2),
                ) && let Some(num) = t1.checked_add(t2)
                {
                    return Some(Self::rational(num, d));
                }
                let r1 = BigRational::new(BigInt::from(*n1), BigInt::from(*d1));
                let r2 = BigRational::new(BigInt::from(*n2), BigInt::from(*d2));
                Some(Self::BigRational(Box::new(r1 + r2)))
            }
            (Self::Integer(a), Self::Rational(n, d)) | (Self::Rational(n, d), Self::Integer(a)) => {
                if let Some(ad) = a.checked_mul(*d)
                    && let Some(num) = ad.checked_add(*n)
                {
                    return Some(Self::rational(num, *d));
                }
                let r1 = BigRational::from(BigInt::from(*a));
                let r2 = BigRational::new(BigInt::from(*n), BigInt::from(*d));
                Some(Self::BigRational(Box::new(r1 + r2)))
            }
            (Self::BigInteger(a), Self::BigInteger(b)) => Some(Self::BigInteger(a + b)),
            (Self::BigInteger(a), Self::Integer(b)) | (Self::Integer(b), Self::BigInteger(a)) => {
                Some(Self::BigInteger(a + BigInt::from(*b)))
            }
            (Self::BigRational(a), Self::BigRational(b)) => {
                Some(Self::BigRational(Box::new((**a).clone() + (**b).clone())))
            }
            _ => None,
        }
    }

    /// Lossless multiplication with auto-promotion.
    pub fn mul_lossless(&self, rhs: &Self) -> Option<Self> {
        match (self, rhs) {
            (Self::Integer(a), Self::Integer(b)) => {
                if let Some(c) = a.checked_mul(*b) {
                    Some(Self::Integer(c))
                } else {
                    Some(Self::BigInteger(BigInt::from(*a) * BigInt::from(*b)))
                }
            }
            (Self::Rational(n1, d1), Self::Rational(n2, d2)) => {
                if let (Some(num), Some(den)) = (n1.checked_mul(*n2), d1.checked_mul(*d2)) {
                    Some(Self::rational(num, den))
                } else {
                    let r1 = BigRational::new(BigInt::from(*n1), BigInt::from(*d1));
                    let r2 = BigRational::new(BigInt::from(*n2), BigInt::from(*d2));
                    Some(Self::BigRational(Box::new(r1 * r2)))
                }
            }
            (Self::Integer(a), Self::Rational(n, d)) | (Self::Rational(n, d), Self::Integer(a)) => {
                if let Some(num) = a.checked_mul(*n) {
                    Some(Self::rational(num, *d))
                } else {
                    let r = BigRational::new(BigInt::from(*a * *n), BigInt::from(*d));
                    Some(Self::BigRational(Box::new(r)))
                }
            }
            (Self::BigInteger(a), Self::BigInteger(b)) => Some(Self::BigInteger(a * b)),
            (Self::BigInteger(a), Self::Integer(b)) | (Self::Integer(b), Self::BigInteger(a)) => {
                Some(Self::BigInteger(a * BigInt::from(*b)))
            }
            (Self::BigRational(a), Self::BigRational(b)) => {
                Some(Self::BigRational(Box::new((**a).clone() * (**b).clone())))
            }
            _ => None,
        }
    }
}

impl Eq for Number {}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integer(i) => write!(f, "{}", i),
            Self::BigInteger(b) => write!(f, "{}", b),
            Self::Rational(num, den) => write!(f, "{}/{}", num, den),
            Self::BigRational(r) => write!(f, "{}", r),
            Self::Scientific { mantissa, exponent } => {
                if *exponent == 0 {
                    write!(f, "{}", mantissa)
                } else {
                    write!(f, "{}e{}", mantissa, exponent)
                }
            }
            Self::Float(bits) => write!(f, "{}", f64::from_bits(*bits)),
            Self::Constant(c) => write!(f, "{}", c),
        }
    }
}
