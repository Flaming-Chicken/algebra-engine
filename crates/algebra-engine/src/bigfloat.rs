//! Arbitrary-precision evaluation of constant expressions (`N(pi, 50)`, `evalf(sqrt(2), 100)`).
//!
//! Numbers are binary fixed-point integers (`value = n / 2^p`) with guard bits; the elementary functions are computed
//! from their series with argument reduction: `exp` by halving, `ln` by the `atanh` series after reducing the mantissa
//! to `[1, 2)`, `sin` and `cos` by reduction modulo `pi/2` and the Taylor series, `atan` by the half-angle
//! identity and the Taylor series, `pi` by Machin's formula. The error of every result is below `2^-(p - 20)`,
//! far inside the guard bits that [`evaluate`] adds.

use crate::poly::rational_from_number;
use algebra_core::{AlgebraError, AlgebraResult, Constant, ExprGraph, ExprId, ExprKind};
use num_bigint::{BigInt, Sign};
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};

fn err(message: impl Into<String>) -> AlgebraError {
    AlgebraError::EvaluationError(message.into())
}

/// The fixed-point context: `p` fractional bits.
#[derive(Clone, Copy)]
struct Fixed {
    p: u32,
}

impl Fixed {
    fn one(&self) -> BigInt {
        BigInt::one() << self.p
    }

    fn scale_rational(&self, r: &BigRational) -> BigInt {
        (r.numer() << self.p) / r.denom()
    }

    fn mul(&self, a: &BigInt, b: &BigInt) -> BigInt {
        (a * b) >> self.p
    }

    fn div(&self, a: &BigInt, b: &BigInt) -> AlgebraResult<BigInt> {
        if b.is_zero() {
            return Err(err("division by zero"));
        }
        Ok((a << self.p) / b)
    }

    fn sqrt(&self, a: &BigInt) -> AlgebraResult<BigInt> {
        if a.is_negative() {
            return Err(err("the square root of a negative number is not real"));
        }
        Ok((a << self.p).sqrt())
    }

    /// `exp(x)` by halving the argument until it is small, the Taylor series, and squaring back.
    fn exp(&self, x: &BigInt) -> AlgebraResult<BigInt> {
        // exp of a large argument would need many more bits for the integer part.
        let magnitude = x.bits().saturating_sub(self.p as u64);
        if magnitude > 24 {
            return Err(err("exp: the argument is too large"));
        }
        let mut halvings = 0u32;
        let mut r = x.clone();
        let limit = self.one() >> 8;
        while r.abs() > limit {
            r >>= 1;
            halvings += 1;
        }
        let (mut term, mut sum) = (self.one(), self.one());
        for n in 1..(self.p as i64 + 20) {
            term = self.mul(&term, &r) / n;
            if term.is_zero() {
                break;
            }
            sum += &term;
        }
        for _ in 0..halvings {
            sum = self.mul(&sum, &sum);
        }
        Ok(sum)
    }

    /// `atanh(z)` for `|z| < 1/2` by its series.
    fn atanh(&self, z: &BigInt) -> BigInt {
        let z2 = self.mul(z, z);
        let (mut power, mut sum) = (z.clone(), z.clone());
        for n in 1..(self.p as i64) {
            power = self.mul(&power, &z2);
            if power.is_zero() {
                break;
            }
            sum += &power / (2 * n + 1);
        }
        sum
    }

    fn ln2(&self) -> BigInt {
        // ln 2 = 2 atanh(1/3)
        let third = self.one() / 3;
        self.atanh(&third) * 2
    }

    /// `ln(x)` for `x > 0`.
    fn ln(&self, x: &BigInt) -> AlgebraResult<BigInt> {
        if !x.is_positive() {
            return Err(err("ln: the argument must be positive"));
        }
        // x = m 2^k with m in [1, 2).
        let k = x.bits() as i64 - 1 - self.p as i64;
        let m = if k >= 0 {
            x >> (k as u32)
        } else {
            x << ((-k) as u32)
        };
        let one = self.one();
        let z = self.div(&(&m - &one), &(&m + &one))?;
        Ok(self.atanh(&z) * 2 + self.ln2() * k)
    }

    fn atan_small(&self, x: &BigInt) -> BigInt {
        // Taylor series for |x| <= 1/8.
        let x2 = self.mul(x, x);
        let (mut power, mut sum) = (x.clone(), x.clone());
        for n in 1..(self.p as i64) {
            power = -self.mul(&power, &x2);
            if power.is_zero() {
                break;
            }
            sum += &power / (2 * n + 1);
        }
        sum
    }

    fn atan(&self, x: &BigInt) -> AlgebraResult<BigInt> {
        // Halve the angle: atan x = 2 atan(x / (1 + sqrt(1 + x^2))) until the argument is small.
        let mut halvings = 0u32;
        let mut r = x.clone();
        let eighth = self.one() >> 3;
        while r.abs() > eighth {
            let sq = self.sqrt(&(self.one() + self.mul(&r, &r)))?;
            r = self.div(&r, &(self.one() + sq))?;
            halvings += 1;
        }
        Ok(self.atan_small(&r) << halvings)
    }

    fn pi(&self) -> AlgebraResult<BigInt> {
        // Machin: pi = 16 atan(1/5) - 4 atan(1/239)
        let a = self.atan_small(&(self.one() / 5));
        let b = self.atan_small(&(self.one() / 239));
        Ok(a * 16 - b * 4)
    }

    /// `(sin x, cos x)`.
    fn sin_cos(&self, x: &BigInt) -> AlgebraResult<(BigInt, BigInt)> {
        // Reduce modulo pi/2 (the context carries guard bits for the cancellation).
        let half_pi: BigInt = self.pi()? >> 1;
        let floor_adjust: BigInt = if x.is_negative() {
            BigInt::from(-1)
        } else {
            BigInt::zero()
        };
        let quotient: BigInt = (x / &half_pi) + floor_adjust;
        let r = x - &quotient * &half_pi;
        let r2 = self.mul(&r, &r);
        // Taylor series for sin and cos of |r| <= pi/2 (reduced again by the quadrant below).
        let (mut s_term, mut s_sum) = (r.clone(), r.clone());
        let (mut c_term, mut c_sum) = (self.one(), self.one());
        for n in 1..(self.p as i64 + 20) {
            c_term = -self.mul(&c_term, &r2) / ((2 * n - 1) * (2 * n));
            s_term = -self.mul(&s_term, &r2) / ((2 * n) * (2 * n + 1));
            if c_term.is_zero() && s_term.is_zero() {
                break;
            }
            c_sum += &c_term;
            s_sum += &s_term;
        }
        let quadrant = quotient.mod_floor_i(4);
        Ok(match quadrant {
            0 => (s_sum, c_sum),
            1 => (c_sum, -s_sum),
            2 => (-s_sum, -c_sum),
            _ => (-c_sum, s_sum),
        })
    }
}

trait ModFloorI {
    fn mod_floor_i(&self, m: i64) -> i64;
}

impl ModFloorI for BigInt {
    fn mod_floor_i(&self, m: i64) -> i64 {
        let m_big = BigInt::from(m);
        let r = ((self % &m_big) + &m_big) % &m_big;
        r.to_i64().unwrap_or(0)
    }
}

/// An exact value or a fixed-point value during evaluation.
fn eval_node(fx: &Fixed, graph: &ExprGraph, id: ExprId) -> AlgebraResult<BigInt> {
    match &graph.get(id).kind {
        ExprKind::Number(n) => match n {
            algebra_core::Number::Constant(Constant::Pi) => fx.pi(),
            algebra_core::Number::Constant(Constant::E) => fx.exp(&fx.one()),
            algebra_core::Number::Float(bits) => {
                let v = f64::from_bits(*bits);
                let r = BigRational::from_float(v).ok_or_else(|| err("a non-finite number"))?;
                Ok(fx.scale_rational(&r))
            }
            algebra_core::Number::Constant(c) => Err(err(format!(
                "the constant {c:?} has no numerical value here"
            ))),
            other => Ok(fx.scale_rational(
                &rational_from_number(other).map_err(|_| err("unsupported number"))?,
            )),
        },
        ExprKind::Symbol(s) => {
            let name = graph.symbols.resolve(*s).unwrap_or_default();
            match name.as_str() {
                "pi" | "π" => fx.pi(),
                "e" => fx.exp(&fx.one()),
                _ => Err(err(format!("the symbol {name} has no value"))),
            }
        }
        ExprKind::Add(terms) => {
            let mut sum = BigInt::zero();
            for &t in terms {
                sum += eval_node(fx, graph, t)?;
            }
            Ok(sum)
        }
        ExprKind::Sub(a, b) => Ok(eval_node(fx, graph, *a)? - eval_node(fx, graph, *b)?),
        ExprKind::Neg(a) => Ok(-eval_node(fx, graph, *a)?),
        ExprKind::Mul(factors) => {
            let mut product = fx.one();
            for &f in factors {
                product = fx.mul(&product, &eval_node(fx, graph, f)?);
            }
            Ok(product)
        }
        ExprKind::Div(a, b) => fx.div(&eval_node(fx, graph, *a)?, &eval_node(fx, graph, *b)?),
        ExprKind::Pow(base, exponent) => {
            // An integer exponent by repeated multiplication (exact for negative bases), else exp(y ln x).
            if let ExprKind::Number(n) = &graph.get(*exponent).kind
                && let Ok(q) = rational_from_number(n)
                && q.is_integer()
                && let Some(k) = q.to_integer().to_i64()
                && k.unsigned_abs() <= 100_000
            {
                let b = eval_node(fx, graph, *base)?;
                let mut result = fx.one();
                let mut square = b;
                let mut e = k.unsigned_abs();
                while e > 0 {
                    if e & 1 == 1 {
                        result = fx.mul(&result, &square);
                    }
                    square = fx.mul(&square, &square);
                    e >>= 1;
                }
                return if k < 0 {
                    fx.div(&fx.one(), &result)
                } else {
                    Ok(result)
                };
            }
            let b = eval_node(fx, graph, *base)?;
            let y = eval_node(fx, graph, *exponent)?;
            let lnb = fx.ln(&b)?;
            fx.exp(&fx.mul(&y, &lnb))
        }
        ExprKind::Function { name, args } if args.len() == 1 => {
            let x = eval_node(fx, graph, args[0])?;
            let fname = graph.symbols.resolve(*name).unwrap_or_default();
            match fname.as_str() {
                "sqrt" => fx.sqrt(&x),
                "exp" => fx.exp(&x),
                "ln" | "log" => fx.ln(&x),
                "sin" => Ok(fx.sin_cos(&x)?.0),
                "cos" => Ok(fx.sin_cos(&x)?.1),
                "tan" => {
                    let (s, c) = fx.sin_cos(&x)?;
                    fx.div(&s, &c)
                }
                "atan" => fx.atan(&x),
                "abs" => Ok(x.abs()),
                "sinh" | "cosh" | "tanh" => {
                    let (ex, enx) = (fx.exp(&x)?, fx.exp(&-x.clone())?);
                    match fname.as_str() {
                        "sinh" => Ok((&ex - &enx) >> 1),
                        "cosh" => Ok((&ex + &enx) >> 1),
                        _ => fx.div(&(&ex - &enx), &(&ex + &enx)),
                    }
                }
                "asin" | "acos" => {
                    // asin x = atan(x / sqrt(1 - x^2)); acos x = pi/2 - asin x
                    let root = fx.sqrt(&(fx.one() - fx.mul(&x, &x)))?;
                    let asin = if root.is_zero() {
                        let half_pi: BigInt = fx.pi()? >> 1;
                        if x.is_negative() { -half_pi } else { half_pi }
                    } else {
                        fx.atan(&fx.div(&x, &root)?)?
                    };
                    if fname == "asin" {
                        Ok(asin)
                    } else {
                        Ok((fx.pi()? >> 1) - asin)
                    }
                }
                other => Err(err(format!(
                    "the function {other} is not available at arbitrary precision"
                ))),
            }
        }
        _ => Err(err(
            "this expression is not a constant numerical expression",
        )),
    }
}

/// Evaluate a constant expression to `digits` significant decimal digits, correctly rounded in the last place except in
/// the rare case of a result within the guard bits of a rounding boundary.
pub fn evaluate(graph: &ExprGraph, expr: ExprId, digits: usize) -> AlgebraResult<String> {
    if digits == 0 || digits > 100_000 {
        return Err(err("the number of digits must be between 1 and 100000"));
    }
    // 3.33 bits per digit plus guard bits for the accumulated rounding of long series and cancellation.
    let bits =
        (digits as f64 * 3.3219281).ceil() as u32 + 96 + (digits as f64).log2().ceil() as u32 * 4;
    let fx = Fixed { p: bits };
    let value = eval_node(&fx, graph, expr)?;
    Ok(to_decimal(&value, bits, digits))
}

/// `value / 2^bits` as a decimal string with `digits` significant digits.
fn to_decimal(value: &BigInt, bits: u32, digits: usize) -> String {
    if value.is_zero() {
        return "0".to_string();
    }
    let negative = value.sign() == Sign::Minus;
    let magnitude = value.abs();
    // Number of digits before the decimal point (can be negative for values below 1).
    let integer_digits = {
        let integer_part = &magnitude >> bits;
        if integer_part.is_zero() {
            // 10^-k <= value < 10^-(k-1)
            let mut k = 0i64;
            let mut scaled = &magnitude * 10;
            let one = BigInt::one() << bits;
            while scaled < one {
                scaled *= 10;
                k += 1;
            }
            -k
        } else {
            integer_part.to_string().len() as i64
        }
    };
    // value * 10^(digits - integer_digits) rounded to an integer has exactly `digits` digits.
    let shift = digits as i64 - integer_digits;
    let numerator = if shift >= 0 {
        &magnitude * BigInt::from(10).pow(shift as u32)
    } else {
        magnitude.clone()
    };
    let denominator = if shift >= 0 {
        BigInt::one() << bits
    } else {
        (BigInt::one() << bits) * BigInt::from(10).pow((-shift) as u32)
    };
    let scaled: BigInt = (&numerator * 2 + &denominator) / (&denominator * 2); // round half up
    let mut text = scaled.to_string();
    let mut point = integer_digits;
    if text.len() > digits {
        // rounding carried into a new digit (9.99.. -> 10.0..)
        text.truncate(digits);
        point += 1;
    }
    let body = if point <= 0 {
        format!("0.{}{}", "0".repeat((-point) as usize), text)
    } else if point as usize >= text.len() {
        format!("{}{}", text, "0".repeat(point as usize - text.len()))
    } else {
        format!("{}.{}", &text[..point as usize], &text[point as usize..])
    };
    if negative { format!("-{body}") } else { body }
}
