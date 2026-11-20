//! Exact complex arithmetic on constant expressions: `(1 + 2i)(3 - i)`, `(2 + 3i)/(1 - i)`, `(1 + i)^8`,
//! `sqrt(-4)`, `abs(3 + 4i)`, `conj`, `re`, `im`. The value is a pair of exact rationals `a + b i`.

use crate::poly::rational_from_number;
use algebra_core::{Constant, ExprGraph, ExprId, ExprKind, Number};
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};

type Q = BigRational;
type C = (Q, Q);

fn is_imaginary_unit(graph: &ExprGraph, id: ExprId) -> bool {
    match &graph.get(id).kind {
        ExprKind::Number(Number::Constant(Constant::I)) => true,
        ExprKind::Symbol(s) => matches!(graph.symbols.resolve(*s).as_deref(), Some("i" | "I")),
        _ => false,
    }
}

fn mul(a: &C, b: &C) -> C {
    (&a.0 * &b.0 - &a.1 * &b.1, &a.0 * &b.1 + &a.1 * &b.0)
}

fn div(a: &C, b: &C) -> Option<C> {
    let norm = &b.0 * &b.0 + &b.1 * &b.1;
    if norm.is_zero() {
        return None;
    }
    Some((
        (&a.0 * &b.0 + &a.1 * &b.1) / &norm,
        (&a.1 * &b.0 - &a.0 * &b.1) / &norm,
    ))
}

/// The exact square root of a non-negative rational, if it is a rational.
fn rational_sqrt(q: &Q) -> Option<Q> {
    if q.is_negative() {
        return None;
    }
    let (n, d) = (q.numer().sqrt(), q.denom().sqrt());
    (&n * &n == *q.numer() && &d * &d == *q.denom()).then(|| Q::new(n, d))
}

/// Evaluate `id` as an exact complex number, or `None` if it is not one (a symbol, an irrational, ...).
pub fn evaluate(graph: &ExprGraph, id: ExprId) -> Option<C> {
    let zero = Q::zero();
    if is_imaginary_unit(graph, id) {
        return Some((zero, Q::one()));
    }
    match graph.get(id).kind.clone() {
        ExprKind::Number(n) => rational_from_number(&n).ok().map(|q| (q, zero)),
        ExprKind::Neg(a) => {
            let (re, im) = evaluate(graph, a)?;
            Some((-re, -im))
        }
        ExprKind::Add(ts) => {
            let mut total = (Q::zero(), Q::zero());
            for t in ts.iter() {
                let v = evaluate(graph, *t)?;
                total = (total.0 + v.0, total.1 + v.1);
            }
            Some(total)
        }
        ExprKind::Sub(a, b) => {
            let (x, y) = (evaluate(graph, a)?, evaluate(graph, b)?);
            Some((x.0 - y.0, x.1 - y.1))
        }
        ExprKind::Mul(fs) => {
            let mut product = (Q::one(), Q::zero());
            for f in fs.iter() {
                product = mul(&product, &evaluate(graph, *f)?);
            }
            Some(product)
        }
        ExprKind::Div(a, b) => div(&evaluate(graph, a)?, &evaluate(graph, b)?),
        ExprKind::Pow(base, exponent) => {
            let base = evaluate(graph, base)?;
            let e = evaluate(graph, exponent)?;
            // The principal square root of a negative rational with a rational root: (-4)^(1/2) = 2 i.
            if e.1.is_zero()
                && e.0 == Q::new(1.into(), 2.into())
                && base.1.is_zero()
                && base.0.is_negative()
            {
                return rational_sqrt(&-base.0.clone()).map(|r| (Q::zero(), r));
            }
            if !e.1.is_zero() || !e.0.is_integer() {
                return None;
            }
            let k = e.0.to_integer().to_i64()?;
            if k.abs() > 256 {
                return None;
            }
            let mut result = (Q::one(), Q::zero());
            for _ in 0..k.abs() {
                result = mul(&result, &base);
            }
            if k < 0 {
                div(&(Q::one(), Q::zero()), &result)
            } else {
                Some(result)
            }
        }
        ExprKind::Function { name, args } if args.len() == 1 => {
            let v = evaluate(graph, args[0])?;
            match graph.symbols.resolve(name).as_deref()? {
                "conj" | "conjugate" => Some((v.0, -v.1)),
                "re" | "real" => Some((v.0, Q::zero())),
                "im" | "imag" => Some((v.1, Q::zero())),
                "abs" => {
                    let norm = &v.0 * &v.0 + &v.1 * &v.1;
                    rational_sqrt(&norm).map(|r| (r, Q::zero()))
                }
                "sqrt" if v.1.is_zero() && v.0.is_negative() => {
                    rational_sqrt(&-v.0).map(|r| (Q::zero(), r))
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// Whether `id` mentions the imaginary unit.
pub fn mentions_i(graph: &ExprGraph, id: ExprId) -> bool {
    if is_imaginary_unit(graph, id) {
        return true;
    }
    match graph.get(id).kind.clone() {
        ExprKind::Add(ts) | ExprKind::Mul(ts) => ts.iter().any(|&t| mentions_i(graph, t)),
        ExprKind::Sub(a, b) | ExprKind::Div(a, b) | ExprKind::Pow(a, b) => {
            mentions_i(graph, a) || mentions_i(graph, b)
        }
        ExprKind::Neg(a) => mentions_i(graph, a),
        ExprKind::Function { args, .. } => args.iter().any(|&a| mentions_i(graph, a)),
        _ => false,
    }
}

fn show(q: &Q) -> String {
    if q.is_integer() {
        q.numer().to_string()
    } else {
        format!("{}/{}", q.numer(), q.denom())
    }
}

/// `a + b i` written plainly: `5 + 5i`, `-i`, `3/2 - i/2`, `7`.
pub fn format(value: &C) -> String {
    let (re, im) = value;
    let imaginary = |b: &Q| -> String {
        let magnitude = b.abs();
        if magnitude.is_one() {
            "i".to_string()
        } else if magnitude.is_integer() {
            format!("{}i", show(&magnitude))
        } else {
            format!("{}i/{}", magnitude.numer(), magnitude.denom())
        }
    };
    if im.is_zero() {
        show(re)
    } else if re.is_zero() {
        format!(
            "{}{}",
            if im.is_negative() { "-" } else { "" },
            imaginary(im)
        )
    } else {
        format!(
            "{} {} {}",
            show(re),
            if im.is_negative() { "-" } else { "+" },
            imaginary(im)
        )
    }
}

/// If `id` mentions `i` and is an exact complex number, its value written as text.
pub fn simplify_text(graph: &ExprGraph, id: ExprId) -> Option<String> {
    let value = evaluate(graph, id)?;
    // A real result of an expression with no `i` (`sqrt(4)`, `1/3`) is the ordinary path's to print.
    if value.1.is_zero() && !mentions_i(graph, id) {
        return None;
    }
    Some(format(&value))
}
