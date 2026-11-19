//! Exact values of special functions at the arguments where they are elementary:
//! `gamma(n) = (n - 1)!`, `gamma(k + 1/2)` a rational multiple of `sqrt(pi)`, `zeta(2k)` a rational multiple of
//! `pi^(2k)`, `zeta(0) = -1/2`, `zeta(-n) = -B_(n+1) / (n + 1)`, `erf(0) = 0`, `erfc(0) = 1`.

use crate::poly::{expr_from_rational, rational_from_number};
use algebra_core::{Constant, ExprGraph, ExprId, ExprKind};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, ToPrimitive, Zero};

type Q = BigRational;

fn factorial(n: u64) -> BigInt {
    (1..=n).fold(BigInt::one(), |p, k| p * k)
}

/// Bernoulli numbers `B_0 .. B_m` (recurrence).
fn bernoulli(m: usize) -> Vec<Q> {
    let mut b: Vec<Q> = vec![Q::one()];
    for n in 1..=m {
        let mut sum = Q::zero();
        let mut binomial = BigInt::one();
        for (k, bk) in b.iter().enumerate().take(n) {
            sum += Q::from_integer(binomial.clone()) * bk;
            binomial = binomial * BigInt::from((n + 1 - k) as u64) / BigInt::from((k + 1) as u64);
        }
        b.push(-sum / Q::from_integer(BigInt::from((n + 1) as u64)));
    }
    b
}

fn rational_argument(graph: &ExprGraph, id: ExprId) -> Option<Q> {
    match &graph.get(id).kind {
        ExprKind::Number(n) => rational_from_number(n).ok(),
        ExprKind::Neg(inner) => rational_argument(graph, *inner).map(|q| -q),
        _ => None,
    }
}

fn special(graph: &ExprGraph, name: &str, arg: ExprId) -> Option<ExprId> {
    let pi = graph.constant(Constant::Pi);
    let x = rational_argument(graph, arg)?;
    match name {
        "gamma" => {
            if x.is_integer() {
                let n = x.to_integer().to_i64()?;
                if (1..=200).contains(&n) {
                    return Some(expr_from_rational(
                        graph,
                        &Q::from_integer(factorial(n as u64 - 1)),
                    ));
                }
                return None;
            }
            // x = k + 1/2 (k may be negative)
            let doubled = &x * Q::from_integer(2.into());
            if !doubled.is_integer() {
                return None;
            }
            let twice = doubled.to_integer().to_i64()?; // odd
            let k = (twice - 1) / 2; // x = k + 1/2
            if k.abs() > 100 {
                return None;
            }
            let value = if k >= 0 {
                // Gamma(k + 1/2) = (2k)! / (4^k k!) sqrt(pi)
                Q::new(
                    factorial(2 * k as u64),
                    BigInt::from(4).pow(k as u32) * factorial(k as u64),
                )
            } else {
                // Gamma(1/2 - m) = (-4)^m m! / (2m)! sqrt(pi), m = -k
                let m = (-k) as u64;
                let sign = if m.is_multiple_of(2) { 1 } else { -1 };
                Q::new(
                    BigInt::from(sign) * BigInt::from(4).pow(m as u32) * factorial(m),
                    factorial(2 * m),
                )
            };
            let root = graph.function("sqrt", [pi]);
            Some(if value.is_one() {
                root
            } else {
                graph.mul([expr_from_rational(graph, &value), root])
            })
        }
        "zeta" => {
            if !x.is_integer() {
                return None;
            }
            let n = x.to_integer().to_i64()?;
            if n == 0 {
                return Some(expr_from_rational(graph, &Q::new((-1).into(), 2.into())));
            }
            if n < 0 {
                let m = (-n) as usize; // zeta(-m) = -B_(m+1) / (m+1)
                if m > 200 {
                    return None;
                }
                let b = bernoulli(m + 1);
                return Some(expr_from_rational(
                    graph,
                    &(-&b[m + 1] / Q::from_integer(((m + 1) as u64).into())),
                ));
            }
            if n >= 2 && n % 2 == 0 && n <= 200 {
                // zeta(2k) = (-1)^(k+1) B_2k (2 pi)^2k / (2 (2k)!) = (-1)^(k+1) B_2k 2^(2k-1) / (2k)! * pi^2k
                let k = (n / 2) as usize;
                let b = bernoulli(n as usize);
                let sign = if !k.is_multiple_of(2) { 1 } else { -1 };
                let value =
                    Q::from_integer(BigInt::from(sign) * BigInt::from(2).pow((n - 1) as u32))
                        * &b[n as usize]
                        / Q::from_integer(factorial(n as u64));
                let power = graph.pow(pi, graph.integer(n));
                return Some(if value.is_one() {
                    power
                } else {
                    graph.mul([expr_from_rational(graph, &value), power])
                });
            }
            None
        }
        "erf" if x.is_zero() => Some(graph.integer(0)),
        "erfc" if x.is_zero() => Some(graph.integer(1)),
        _ => None,
    }
}

/// Rewrite every special-function call in `expr` whose argument is one of the exact cases above.
pub fn evaluate(graph: &ExprGraph, expr: ExprId) -> ExprId {
    match graph.get(expr).kind.clone() {
        ExprKind::Number(_) | ExprKind::Symbol(_) => expr,
        ExprKind::Add(ts) => graph.add(ts.iter().map(|&t| evaluate(graph, t)).collect::<Vec<_>>()),
        ExprKind::Mul(fs) => graph.mul(fs.iter().map(|&t| evaluate(graph, t)).collect::<Vec<_>>()),
        ExprKind::Sub(a, b) => graph.sub(evaluate(graph, a), evaluate(graph, b)),
        ExprKind::Div(a, b) => graph.div(evaluate(graph, a), evaluate(graph, b)),
        ExprKind::Pow(a, b) => graph.pow(evaluate(graph, a), evaluate(graph, b)),
        ExprKind::Neg(a) => graph.neg(evaluate(graph, a)),
        ExprKind::Function { name, args } => {
            let new_args: Vec<ExprId> = args.iter().map(|&a| evaluate(graph, a)).collect();
            let fname = graph.symbols.resolve(name).unwrap_or_default();
            if new_args.len() == 1
                && let Some(value) = special(graph, &fname, new_args[0])
            {
                return value;
            }
            // Discrete functions of exact rational arguments (`fibonacci(20)`, `partitions(100)`, `bell(10)`).
            if let Some(values) = new_args
                .iter()
                .map(|&a| rational_argument(graph, a))
                .collect::<Option<Vec<Q>>>()
                && let Some(result) = crate::discrete::evaluate(&fname, &values)
            {
                return expr_from_rational(graph, &result);
            }
            // Named formulas of geometry and probability with constant arguments.
            // `pi` typed as a symbol and the constant pi must meet in one form for the formula to cancel.
            let pi_symbol = graph.symbols.get_or_intern("pi");
            let pi_constant = graph.constant(Constant::Pi);
            let canonical: Vec<ExprId> = new_args
                .iter()
                .map(|&a| graph.substitute(a, pi_symbol, pi_constant))
                .collect();
            if let Some(rewritten) = crate::formulary::rewrite(graph, &fname, &canonical) {
                return crate::simplify::Simplifier::simplify(graph, rewritten)
                    .unwrap_or(rewritten);
            }
            if new_args == args.to_vec() {
                expr
            } else {
                graph.function(&fname, new_args)
            }
        }
        _ => expr,
    }
}
