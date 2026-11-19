//! Truncated power series over the rationals: the fast path for Taylor expansion.
//!
//! The general method differentiates the expression `n` times and substitutes the point, which swells
//! exponentially (the order-12 series of `exp(sin(x))` took 22 seconds). When the expression is built from
//! rational constants, the variable, `+ - * /`, integer and rational powers, and `exp`, `sin`, `cos`, `tan`, `sinh`,
//! `cosh`, `tanh`, `ln` and `atan` whose argument has the right constant term (zero, or one for `ln`), the series
//! is computed directly by series arithmetic on rational coefficient vectors, in time polynomial in the order.
//! Anything else returns `None` and the caller falls back to differentiation.

use crate::poly::{expr_from_rational, rational_from_number};
use algebra_core::{ExprGraph, ExprId, ExprKind, SymbolId};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};

type Q = BigRational;
type Series = Vec<Q>;

fn q(n: i64) -> Q {
    Q::from_integer(BigInt::from(n))
}

fn constant(value: Q, len: usize) -> Series {
    let mut s = vec![Q::zero(); len];
    s[0] = value;
    s
}

fn add(a: &Series, b: &Series) -> Series {
    a.iter().zip(b).map(|(x, y)| x + y).collect()
}

fn sub(a: &Series, b: &Series) -> Series {
    a.iter().zip(b).map(|(x, y)| x - y).collect()
}

fn neg(a: &Series) -> Series {
    a.iter().map(|x| -x).collect()
}

fn scale(a: &Series, c: &Q) -> Series {
    a.iter().map(|x| x * c).collect()
}

fn mul(a: &Series, b: &Series) -> Series {
    let n = a.len();
    let mut out = vec![Q::zero(); n];
    for i in 0..n {
        if a[i].is_zero() {
            continue;
        }
        for j in 0..n - i {
            out[i + j] += &a[i] * &b[j];
        }
    }
    out
}

/// `a / b` for `b[0] != 0`.
fn div(a: &Series, b: &Series) -> Option<Series> {
    if b[0].is_zero() {
        return None;
    }
    let n = a.len();
    let mut out = vec![Q::zero(); n];
    for k in 0..n {
        let mut acc = a[k].clone();
        for j in 1..=k {
            acc -= &b[j] * &out[k - j];
        }
        out[k] = acc / &b[0];
    }
    Some(out)
}

/// The derivative (shorter by nothing: the last coefficient is lost and set to zero, which is fine because every use
/// below only reads coefficients `0..n-1` of it).
fn derivative(a: &Series) -> Series {
    let n = a.len();
    let mut out = vec![Q::zero(); n];
    for k in 1..n {
        out[k - 1] = &a[k] * q(k as i64);
    }
    out
}

/// The antiderivative with zero constant term.
fn integral(a: &Series) -> Series {
    let n = a.len();
    let mut out = vec![Q::zero(); n];
    for k in 1..n {
        out[k] = &a[k - 1] / q(k as i64);
    }
    out
}

fn exp_series(f: &Series) -> Option<Series> {
    if !f[0].is_zero() {
        return None;
    }
    // E' = f' E: e_k = (1/k) sum_{j=1..k} j f_j e_{k-j}
    let n = f.len();
    let mut e = vec![Q::zero(); n];
    e[0] = Q::one();
    for k in 1..n {
        let mut acc = Q::zero();
        for j in 1..=k {
            acc += q(j as i64) * &f[j] * &e[k - j];
        }
        e[k] = acc / q(k as i64);
    }
    Some(e)
}

/// `(sin f, cos f)` for `f[0] = 0` (or `(sinh f, cosh f)` when `hyperbolic`).
fn sin_cos_series(f: &Series, hyperbolic: bool) -> Option<(Series, Series)> {
    if !f[0].is_zero() {
        return None;
    }
    let n = f.len();
    let (mut s, mut c) = (vec![Q::zero(); n], vec![Q::zero(); n]);
    c[0] = Q::one();
    for k in 1..n {
        let (mut sa, mut ca) = (Q::zero(), Q::zero());
        for j in 1..=k {
            let jf = q(j as i64) * &f[j];
            sa += &jf * &c[k - j];
            ca += &jf * &s[k - j];
        }
        s[k] = sa / q(k as i64);
        c[k] = if hyperbolic {
            ca / q(k as i64)
        } else {
            -ca / q(k as i64)
        };
    }
    Some((s, c))
}

fn ln_series(f: &Series) -> Option<Series> {
    // f[0] = 1: ln f = int f' / f
    if !f[0].is_one() {
        return None;
    }
    let quotient = div(&derivative(f), f)?;
    Some(integral(&quotient))
}

fn atan_series(f: &Series) -> Option<Series> {
    if !f[0].is_zero() {
        return None;
    }
    let mut denominator = mul(f, f);
    denominator[0] += Q::one();
    Some(integral(&div(&derivative(f), &denominator)?))
}

/// `f^p` for a rational exponent `p`, when `f[0] > 0` makes `f[0]^p` rational (checked) or `p` is an integer.
fn pow_series(f: &Series, p: &Q) -> Option<Series> {
    let n = f.len();
    if p.is_integer() {
        let e = p.to_integer().to_i64()?;
        if e.unsigned_abs() > 4096 {
            return None;
        }
        let mut base = f.clone();
        if e < 0 {
            base = div(&constant(Q::one(), n), f)?;
        }
        let mut result = constant(Q::one(), n);
        for _ in 0..e.unsigned_abs() {
            result = mul(&result, &base);
        }
        return Some(result);
    }
    // Fractional power: f = c (1 + h), f^p = c^p (1+h)^p with c^p rational.
    let c = f[0].clone();
    if !c.is_positive() {
        return None;
    }
    let cp = rational_power(&c, p)?;
    let h: Series = scale(f, &(Q::one() / &c));
    // P = (1+h)^p: P' (1+h) = p h' P, so (k) P_k = sum_{j=1..k} (p j - (k - j)) h_j P_{k-j}
    let mut pk = vec![Q::zero(); n];
    pk[0] = Q::one();
    for k in 1..n {
        let mut acc = Q::zero();
        for j in 1..=k {
            acc += (p * q(j as i64) - q((k - j) as i64)) * &h[j] * &pk[k - j];
        }
        pk[k] = acc / q(k as i64);
    }
    Some(scale(&pk, &cp))
}

/// `c^p` when it is rational (`c` a rational, `p = a/b`): the b-th root of `c^a` must be exact.
fn rational_power(c: &Q, p: &Q) -> Option<Q> {
    let (a, b) = (p.numer().to_i32()?, p.denom().to_u32()?);
    let num = c.numer().clone();
    let den = c.denom().clone();
    let root = |n: &BigInt| -> Option<BigInt> {
        let r = n.nth_root(b);
        (r.pow(b) == *n).then_some(r)
    };
    let (rn, rd) = (root(&num)?, root(&den)?);
    let base = Q::new(rn, rd);
    let magnitude = a.unsigned_abs();
    let mut out = Q::one();
    for _ in 0..magnitude {
        out *= &base;
    }
    Some(if a < 0 { Q::one() / out } else { out })
}

fn series_of(
    graph: &ExprGraph,
    id: ExprId,
    wrt: SymbolId,
    point: &Q,
    len: usize,
) -> Option<Series> {
    match &graph.get(id).kind {
        ExprKind::Number(n) => Some(constant(rational_from_number(n).ok()?, len)),
        ExprKind::Symbol(s) if *s == wrt => {
            let mut v = vec![Q::zero(); len];
            v[0] = point.clone();
            if len > 1 {
                v[1] = Q::one();
            }
            Some(v)
        }
        ExprKind::Symbol(_) => None,
        ExprKind::Add(terms) => {
            let mut acc = constant(Q::zero(), len);
            for &t in terms {
                acc = add(&acc, &series_of(graph, t, wrt, point, len)?);
            }
            Some(acc)
        }
        ExprKind::Sub(a, b) => Some(sub(
            &series_of(graph, *a, wrt, point, len)?,
            &series_of(graph, *b, wrt, point, len)?,
        )),
        ExprKind::Neg(a) => Some(neg(&series_of(graph, *a, wrt, point, len)?)),
        ExprKind::Mul(factors) => {
            let mut acc = constant(Q::one(), len);
            for &f in factors {
                acc = mul(&acc, &series_of(graph, f, wrt, point, len)?);
            }
            Some(acc)
        }
        ExprKind::Div(a, b) => div(
            &series_of(graph, *a, wrt, point, len)?,
            &series_of(graph, *b, wrt, point, len)?,
        ),
        ExprKind::Pow(base, exp) => {
            let ExprKind::Number(n) = &graph.get(*exp).kind else {
                return None;
            };
            let p = rational_from_number(n).ok()?;
            pow_series(&series_of(graph, *base, wrt, point, len)?, &p)
        }
        ExprKind::Function { name, args } if args.len() == 1 => {
            let f = series_of(graph, args[0], wrt, point, len)?;
            match graph.symbols.resolve(*name)?.as_str() {
                "exp" => exp_series(&f),
                "sin" => sin_cos_series(&f, false).map(|(s, _)| s),
                "cos" => sin_cos_series(&f, false).map(|(_, c)| c),
                "tan" => {
                    let (s, c) = sin_cos_series(&f, false)?;
                    div(&s, &c)
                }
                "sinh" => sin_cos_series(&f, true).map(|(s, _)| s),
                "cosh" => sin_cos_series(&f, true).map(|(_, c)| c),
                "tanh" => {
                    let (s, c) = sin_cos_series(&f, true)?;
                    div(&s, &c)
                }
                "ln" | "log" => ln_series(&f),
                "atan" => atan_series(&f),
                "sqrt" => pow_series(&f, &Q::new(1.into(), 2.into())),
                _ => None,
            }
        }
        _ => None,
    }
}

/// The Taylor coefficients `c_0..c_order` of `expr` around the rational `point`, as expressions, if the fast path
/// applies.
pub(crate) fn taylor_coeffs_fast(
    graph: &ExprGraph,
    expr: ExprId,
    wrt: SymbolId,
    point: ExprId,
    order: usize,
) -> Option<Vec<ExprId>> {
    let point_q = match &graph.get(point).kind {
        ExprKind::Number(n) => rational_from_number(n).ok()?,
        _ => return None,
    };
    if order > 400 {
        return None;
    }
    let series = series_of(graph, expr, wrt, &point_q, order + 1)?;
    Some(
        series
            .iter()
            .map(|c| expr_from_rational(graph, c))
            .collect(),
    )
}
