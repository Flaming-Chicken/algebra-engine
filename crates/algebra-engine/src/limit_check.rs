//! A numerical sanity check for limits at a finite point.
//!
//! The symbolic limit engine can return a wrong value (`abs(x)/x` at `0+` gave `0`) or an expression with a division
//! by zero inside (`exp(1/x)` at `0+` gave `exp(1/0)`). This module samples the function at `a +- h` for shrinking `h`
//! and classifies the behaviour: a finite value, divergence to `+oo` or `-oo`, or unknown. When the classification is
//! clear and the symbolic answer contradicts it, the numerical answer replaces it (and is labelled an estimate).

use crate::numeric::{BigValue, EvalContext, NumericalEval};
use algebra_core::{Constant, ExprGraph, ExprId};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Behaviour {
    Finite(f64),
    Infinite(f64),
    Unknown,
}

fn sample(graph: &ExprGraph, expr: ExprId, var: &str, at: f64) -> Option<f64> {
    let mut bindings = HashMap::new();
    bindings.insert(var.to_string(), at);
    let ctx = EvalContext::with_bindings(50, bindings);
    match graph.evalf(expr, &ctx).ok()? {
        BigValue::Real(r) if !r.is_nan() => Some(r),
        _ => None,
    }
}

/// How `expr` behaves as `var -> a` from the side `side` (`+1.0` or `-1.0`).
pub fn classify(graph: &ExprGraph, expr: ExprId, var: &str, a: f64, side: f64) -> Behaviour {
    let scale = a.abs().max(1.0);
    let hs = [1e-2, 1e-3, 1e-4, 1e-5];
    let mut values = Vec::new();
    for h in hs {
        match sample(graph, expr, var, a + side * h * scale) {
            Some(v) => values.push(v),
            None => return Behaviour::Unknown,
        }
    }
    let n = values.len();
    let (v3, v2, v1) = (values[n - 1], values[n - 2], values[n - 3]);
    // Divergence: growing by a factor each time `h` shrinks tenfold (overflow counts), with one sign.
    let grows =
        |big: f64, small: f64| big.abs() > 5.0 * small.abs() && big.signum() == small.signum();
    if v3.is_infinite() && v2.abs() > 20.0 && v2.signum() == v3.signum() {
        return Behaviour::Infinite(v3.signum());
    }
    if v3.abs() > 100.0 && v2.abs() > 20.0 && grows(v3, v2) && grows(v2, v1) {
        return Behaviour::Infinite(v3.signum());
    }
    let near =
        |a: f64, b: f64, tolerance: f64| (a - b).abs() <= tolerance * a.abs().max(b.abs()).max(1.0);
    if near(v3, v2, 2e-4) && near(v2, v1, 2e-3) {
        return Behaviour::Finite(v3);
    }
    Behaviour::Unknown
}

fn nearly(a: f64, b: f64) -> bool {
    (a - b).abs() <= 2e-3 * a.abs().max(b.abs()).max(1.0)
}

/// The behaviour over the requested sides (`[1.0]`, `[-1.0]` or both); `None` if it is unclear. `Some(None)` means the
/// two-sided limit does not exist.
pub fn combined(
    graph: &ExprGraph,
    expr: ExprId,
    var: &str,
    a: f64,
    sides: &[f64],
) -> Option<Option<Behaviour>> {
    let classes: Vec<Behaviour> = sides
        .iter()
        .map(|&s| classify(graph, expr, var, a, s))
        .collect();
    if classes.contains(&Behaviour::Unknown) {
        return None;
    }
    if classes.len() == 1 {
        return Some(Some(classes[0]));
    }
    match (classes[0], classes[1]) {
        (Behaviour::Finite(x), Behaviour::Finite(y)) if nearly(x, y) => {
            Some(Some(Behaviour::Finite((x + y) / 2.0)))
        }
        (Behaviour::Infinite(x), Behaviour::Infinite(y)) if x == y => {
            Some(Some(Behaviour::Infinite(x)))
        }
        _ => Some(None),
    }
}

/// Whether the symbolic answer `symbolic` agrees with the numerical behaviour.
pub fn agrees(graph: &ExprGraph, symbolic: ExprId, behaviour: Option<Behaviour>) -> bool {
    let value = graph.evalf(symbolic, &EvalContext::default());
    let undefined = matches!(
        graph.get(symbolic).kind,
        algebra_core::ExprKind::Number(algebra_core::Number::Constant(Constant::Undefined))
    );
    match behaviour {
        None => undefined,
        Some(Behaviour::Finite(v)) => match value {
            Ok(BigValue::Real(s)) => s.is_finite() && (s - v).abs() <= 1e-2 * v.abs().max(1.0),
            _ => false,
        },
        Some(Behaviour::Infinite(sign)) => match value {
            Ok(BigValue::Real(s)) => s.is_infinite() && s.signum() == sign,
            _ => {
                matches!(
                    graph.get(symbolic).kind,
                    algebra_core::ExprKind::Number(algebra_core::Number::Constant(
                        Constant::Infinity | Constant::NegInfinity
                    ))
                ) && {
                    let positive = matches!(
                        graph.get(symbolic).kind,
                        algebra_core::ExprKind::Number(algebra_core::Number::Constant(
                            Constant::Infinity
                        ))
                    );
                    positive == (sign > 0.0)
                }
            }
        },
        Some(Behaviour::Unknown) => true,
    }
}

/// The expression for a numerical behaviour: a small rational when one fits, else the decimal value.
pub fn as_expression(graph: &ExprGraph, behaviour: Option<Behaviour>) -> ExprId {
    match behaviour {
        None => graph.constant(Constant::Undefined),
        Some(Behaviour::Infinite(sign)) => graph.constant(if sign > 0.0 {
            Constant::Infinity
        } else {
            Constant::NegInfinity
        }),
        Some(Behaviour::Unknown) => graph.constant(Constant::Undefined),
        Some(Behaviour::Finite(v)) => {
            for q in 1..=50i64 {
                let p = (v * q as f64).round();
                if (v - p / q as f64).abs() < 1e-5 * v.abs().max(1.0) {
                    return graph.rational(p as i64, q);
                }
            }
            graph.float(v)
        }
    }
}
