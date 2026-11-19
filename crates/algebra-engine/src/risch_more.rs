//! Further closed-form integrals, tried when the main integrator gives up:
//!
//! * polynomials times `sqrt(q)` or divided by `sqrt(q)` for a quadratic `q = a x^2 + b x + c`, by the reduction
//!   $\int x^n / \sqrt{q} = \frac{x^{n-1} \sqrt{q}}{n a} - \frac{(2n-1) b}{2 n a} \int \frac{x^{n-1}}{\sqrt{q}} -
//!   \frac{(n-1) c}{n a} \int \frac{x^{n-2}}{\sqrt{q}}$ and the two base integrals ($\ln$ for `a > 0`, $\arcsin$ for
//!   `a < 0`); $\int P \sqrt{q} = \int P q / \sqrt{q}$;
//! * a polynomial times `atan`, `asin`, `acos` or a power of `ln` by integration by parts;
//! * integer powers of `sin` and `cos` of a linear argument by the reduction formulas, and `1/sin`, `1/cos`.
//!
//! Every formula is verified by differentiation in the test suite.

use crate::poly::{UniPoly, expr_from_rational};
use algebra_core::{AlgebraError, AlgebraResult, ExprGraph, ExprId, ExprKind, SymbolId};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};

type Q = BigRational;

fn q(n: i64) -> Q {
    Q::from_integer(BigInt::from(n))
}

fn poly_of(graph: &ExprGraph, id: ExprId, wrt: SymbolId) -> Option<Vec<Q>> {
    UniPoly::from_expr(graph, id, wrt).ok().map(|p| p.coeffs)
}

fn poly_expr(graph: &ExprGraph, x: ExprId, coeffs: &[Q]) -> ExprId {
    let mut terms = Vec::new();
    for (k, c) in coeffs.iter().enumerate() {
        if c.is_zero() {
            continue;
        }
        let power = match k {
            0 => None,
            1 => Some(x),
            _ => Some(graph.pow(x, graph.integer(k as i64))),
        };
        terms.push(match power {
            None => expr_from_rational(graph, c),
            Some(p) if c.is_one() => p,
            Some(p) => graph.mul([expr_from_rational(graph, c), p]),
        });
    }
    match terms.len() {
        0 => graph.integer(0),
        1 => terms[0],
        _ => graph.add(terms),
    }
}

fn poly_mul(a: &[Q], b: &[Q]) -> Vec<Q> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut out = vec![Q::zero(); a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            out[i + j] += x * y;
        }
    }
    out
}

/// A square root of a positive rational as an expression: exact when the rational is a perfect square.
fn sqrt_of(graph: &ExprGraph, r: &Q) -> ExprId {
    let (n, d) = (r.numer(), r.denom());
    let (sn, sd) = (n.sqrt(), d.sqrt());
    if &(&sn * &sn) == n && &(&sd * &sd) == d {
        return expr_from_rational(graph, &Q::new(sn, sd));
    }
    graph.function("sqrt", [expr_from_rational(graph, r)])
}

/// `a x^2 + b x + c` with rational coefficients and `a != 0`.
struct Quadratic {
    a: Q,
    b: Q,
    c: Q,
}

impl Quadratic {
    fn from_expr(graph: &ExprGraph, id: ExprId, wrt: SymbolId) -> Option<Self> {
        let coeffs = poly_of(graph, id, wrt)?;
        if coeffs.len() != 3 || coeffs[2].is_zero() {
            return None;
        }
        Some(Self {
            a: coeffs[2].clone(),
            b: coeffs[1].clone(),
            c: coeffs[0].clone(),
        })
    }

    fn expr(&self, graph: &ExprGraph, x: ExprId) -> ExprId {
        poly_expr(graph, x, &[self.c.clone(), self.b.clone(), self.a.clone()])
    }

    fn sqrt_expr(&self, graph: &ExprGraph, x: ExprId) -> ExprId {
        graph.function("sqrt", [self.expr(graph, x)])
    }

    /// `int dx / sqrt(q)`.
    fn base_integral(&self, graph: &ExprGraph, x: ExprId) -> Option<ExprId> {
        let sq = self.sqrt_expr(graph, x);
        let two_a_x_b = poly_expr(graph, x, &[self.b.clone(), &self.a * q(2)]);
        if self.a.is_positive() {
            // (1 / sqrt a) ln(2 sqrt(a) sqrt(q) + 2 a x + b)
            let root_a = sqrt_of(graph, &self.a);
            let inner = graph.add([graph.mul([graph.integer(2), root_a, sq]), two_a_x_b]);
            Some(graph.div(graph.function("ln", [inner]), root_a))
        } else {
            // -(1 / sqrt(-a)) asin((2 a x + b) / sqrt(b^2 - 4 a c))
            let discriminant = &self.b * &self.b - q(4) * &self.a * &self.c;
            if !discriminant.is_positive() {
                return None; // q is never positive
            }
            let root_neg_a = sqrt_of(graph, &-self.a.clone());
            let root_d = sqrt_of(graph, &discriminant);
            let arg = graph.div(two_a_x_b, root_d);
            Some(graph.neg(graph.div(graph.function("asin", [arg]), root_neg_a)))
        }
    }

    /// `int x^n / sqrt(q) dx`.
    fn power_over_sqrt(&self, graph: &ExprGraph, x: ExprId, n: usize) -> Option<ExprId> {
        match n {
            0 => self.base_integral(graph, x),
            1 => {
                // sqrt(q) / a - b / (2 a) int dx / sqrt(q)
                let sq = self.sqrt_expr(graph, x);
                let first = graph.div(sq, expr_from_rational(graph, &self.a));
                let second = graph.mul([
                    expr_from_rational(graph, &(&self.b / (q(2) * &self.a))),
                    self.base_integral(graph, x)?,
                ]);
                Some(graph.sub(first, second))
            }
            _ => {
                let nn = q(n as i64);
                let sq = self.sqrt_expr(graph, x);
                let lead = graph.mul([
                    expr_from_rational(graph, &(Q::one() / (&nn * &self.a))),
                    graph.pow(x, graph.integer(n as i64 - 1)),
                    sq,
                ]);
                let c1 = (q(2 * n as i64 - 1) * &self.b) / (q(2) * &nn * &self.a);
                let c2 = (q(n as i64 - 1) * &self.c) / (&nn * &self.a);
                let t1 = graph.mul([
                    expr_from_rational(graph, &c1),
                    self.power_over_sqrt(graph, x, n - 1)?,
                ]);
                let t2 = graph.mul([
                    expr_from_rational(graph, &c2),
                    self.power_over_sqrt(graph, x, n - 2)?,
                ]);
                Some(graph.sub(graph.sub(lead, t1), t2))
            }
        }
    }

    /// `int P(x) / sqrt(q) dx` for a polynomial `P`.
    fn poly_over_sqrt(&self, graph: &ExprGraph, x: ExprId, p: &[Q]) -> Option<ExprId> {
        let mut terms = Vec::new();
        for (n, c) in p.iter().enumerate() {
            if c.is_zero() {
                continue;
            }
            let t = self.power_over_sqrt(graph, x, n)?;
            terms.push(graph.mul([expr_from_rational(graph, c), t]));
        }
        Some(graph.add(terms))
    }
}

/// Recognise `sqrt(q)` or `q^(1/2)` (returning the radicand) and `q^(-1/2)`.
fn radical(graph: &ExprGraph, id: ExprId) -> Option<(ExprId, bool)> {
    match &graph.get(id).kind {
        ExprKind::Function { name, args }
            if args.len() == 1 && graph.symbols.resolve(*name)?.as_str() == "sqrt" =>
        {
            Some((args[0], false))
        }
        ExprKind::Pow(base, exp) => {
            let half = Q::new(1.into(), 2.into());
            let value = match &graph.get(*exp).kind {
                ExprKind::Number(n) => crate::poly::rational_from_number(n).ok()?,
                _ => return None,
            };
            if value == half {
                Some((*base, false))
            } else if value == -half {
                Some((*base, true))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// `P(x) * sqrt(q)` and `P(x) / sqrt(q)`.
fn integrate_quadratic_radical(
    graph: &ExprGraph,
    expr: ExprId,
    wrt: SymbolId,
    x: ExprId,
) -> Option<ExprId> {
    // (polynomial, radicand, in_denominator)
    let parts: (Vec<Q>, ExprId, bool) = match &graph.get(expr).kind {
        ExprKind::Div(num, den) => {
            let (radicand, inverse) = radical(graph, *den)?;
            if inverse {
                return None;
            }
            (poly_of(graph, *num, wrt)?, radicand, true)
        }
        ExprKind::Mul(factors) => {
            let (ri, (radicand, inverse)) = factors
                .iter()
                .enumerate()
                .find_map(|(i, &f)| radical(graph, f).map(|r| (i, r)))?;
            let rest: Vec<ExprId> = factors
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != ri)
                .map(|(_, &f)| f)
                .collect();
            let p = if rest.is_empty() {
                vec![Q::one()]
            } else {
                poly_of(graph, graph.mul(rest), wrt)?
            };
            (p, radicand, inverse)
        }
        _ => {
            let (radicand, inverse) = radical(graph, expr)?;
            (vec![Q::one()], radicand, inverse)
        }
    };
    let (p, radicand, in_denominator) = parts;
    let quad = Quadratic::from_expr(graph, radicand, wrt)?;
    if in_denominator {
        quad.poly_over_sqrt(graph, x, &p)
    } else {
        // P sqrt(q) = P q / sqrt(q)
        let q_coeffs = [quad.c.clone(), quad.b.clone(), quad.a.clone()];
        quad.poly_over_sqrt(graph, x, &poly_mul(&p, &q_coeffs))
    }
}

/// `int P(x) f(x) dx` for `f` in `atan`, `asin`, `acos`, `ln^m`, by parts with `F = int P`.
fn integrate_by_parts(
    graph: &ExprGraph,
    expr: ExprId,
    wrt: SymbolId,
    x: ExprId,
) -> Option<AlgebraResult<ExprId>> {
    let ExprKind::Mul(factors) = &graph.get(expr).kind else {
        return None;
    };
    for (i, &f) in factors.iter().enumerate() {
        // The special factor: atan(x), asin(x), acos(x), ln(x) or ln(x)^m.
        let (name, power) = match &graph.get(f).kind {
            ExprKind::Function { name, args } if args.len() == 1 && args[0] == x => {
                (graph.symbols.resolve(*name)?, 1u32)
            }
            ExprKind::Pow(base, exp) => {
                let ExprKind::Function { name, args } = &graph.get(*base).kind else {
                    continue;
                };
                let m = match &graph.get(*exp).kind {
                    ExprKind::Number(n) => crate::poly::rational_from_number(n)
                        .ok()?
                        .to_integer()
                        .to_u32()?,
                    _ => continue,
                };
                if args.len() != 1 || args[0] != x || !(2..=8).contains(&m) {
                    continue;
                }
                (graph.symbols.resolve(*name)?, m)
            }
            _ => continue,
        };
        if !matches!(name.as_str(), "atan" | "asin" | "acos" | "ln") || (power > 1 && name != "ln")
        {
            continue;
        }
        let rest: Vec<ExprId> = factors
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, &g)| g)
            .collect();
        let p = if rest.is_empty() {
            vec![Q::one()]
        } else {
            let Some(p) = poly_of(graph, graph.mul(rest), wrt) else {
                continue;
            };
            p
        };
        // F = int P dx, with no constant term.
        let mut f_coeffs = vec![Q::zero()];
        for (k, c) in p.iter().enumerate() {
            f_coeffs.push(c / q(k as i64 + 1));
        }
        let big_f = poly_expr(graph, x, &f_coeffs);
        let boundary = graph.mul([big_f, f]);
        let remainder: AlgebraResult<ExprId> = match name.as_str() {
            "ln" => {
                // int F ln^(m-1) * m / x dx: F/x is a polynomial because F has no constant term.
                let f_over_x: Vec<Q> = f_coeffs[1..].to_vec();
                let ln_x = graph.function("ln", [x]);
                let lower_power = match power {
                    1 => None,
                    2 => Some(ln_x),
                    m => Some(graph.pow(ln_x, graph.integer(m as i64 - 1))),
                };
                let polynomial = poly_expr(graph, x, &f_over_x);
                let inner = match lower_power {
                    None => polynomial,
                    Some(l) => graph.mul([polynomial, l]),
                };
                crate::risch::RischIntegrator::integrate(graph, inner, wrt)
                    .map(|v| graph.mul([graph.integer(power as i64), v]))
            }
            "atan" => {
                // int F / (1 + x^2)
                let denominator = graph.add([graph.integer(1), graph.pow(x, graph.integer(2))]);
                crate::risch::RischIntegrator::integrate(graph, graph.div(big_f, denominator), wrt)
            }
            _ => {
                // asin and acos: int F / sqrt(1 - x^2), subtracted for asin and added for acos.
                let quad = Quadratic {
                    a: q(-1),
                    b: Q::zero(),
                    c: Q::one(),
                };
                quad.poly_over_sqrt(graph, x, &f_coeffs).ok_or_else(|| {
                    AlgebraError::EvaluationError("by parts: radical integral".into())
                })
            }
        };
        let remainder = match remainder {
            Ok(r) => r,
            Err(e) => return Some(Err(e)),
        };
        return Some(Ok(match name.as_str() {
            "ln" | "atan" | "asin" => graph.sub(boundary, remainder),
            _ => graph.add([boundary, remainder]), // acos: its derivative is -1 / sqrt(1 - x^2)
        }));
    }
    None
}

/// `int sin(u)^n` and `int cos(u)^n` for `u = a x + b` and integer `n >= 2`, and `1/sin(u)`, `1/cos(u)`.
fn integrate_trig_power(
    graph: &ExprGraph,
    expr: ExprId,
    wrt: SymbolId,
    x: ExprId,
) -> Option<ExprId> {
    let linear = |arg: ExprId| -> Option<Q> {
        let c = poly_of(graph, arg, wrt)?;
        (c.len() == 2 && !c[1].is_zero()).then(|| c[1].clone())
    };
    let trig = |id: ExprId| -> Option<(bool, ExprId)> {
        let ExprKind::Function { name, args } = &graph.get(id).kind else {
            return None;
        };
        if args.len() != 1 {
            return None;
        }
        match graph.symbols.resolve(*name)?.as_str() {
            "sin" => Some((true, args[0])),
            "cos" => Some((false, args[0])),
            _ => None,
        }
    };
    // 1/sin(u) = csc(u), 1/cos(u) = sec(u)
    if let ExprKind::Div(num, den) = &graph.get(expr).kind
        && *num == graph.integer(1)
        && let Some((is_sin, u)) = trig(*den)
    {
        let a = linear(u)?;
        let inv = expr_from_rational(graph, &(Q::one() / a));
        let antiderivative = if is_sin {
            // ln(tan(u / 2))
            graph.function(
                "ln",
                [graph.function("tan", [graph.div(u, graph.integer(2))])],
            )
        } else {
            // ln(sec u + tan u)
            graph.function(
                "ln",
                [graph.add([graph.function("sec", [u]), graph.function("tan", [u])])],
            )
        };
        return Some(graph.mul([inv, antiderivative]));
    }
    let ExprKind::Pow(base, exp) = &graph.get(expr).kind else {
        return None;
    };
    let (is_sin, u) = trig(*base)?;
    let a = linear(u)?;
    let n = match &graph.get(*exp).kind {
        ExprKind::Number(v) => crate::poly::rational_from_number(v).ok()?,
        _ => return None,
    };
    if !n.is_integer() || n < q(2) || n > q(40) {
        return None;
    }
    let n = n.to_integer().to_u32()?;
    // F_n(u) = int f^n du, by F_n = -/+ f^(n-1) g / n + (n-1)/n F_(n-2), F_0 = u, F_1 = -cos u or sin u.
    let sin_u = graph.function("sin", [u]);
    let cos_u = graph.function("cos", [u]);
    let (f, g) = if is_sin {
        (sin_u, cos_u)
    } else {
        (cos_u, sin_u)
    };
    let mut previous_even = u; // F_0
    let mut previous_odd = if is_sin { graph.neg(cos_u) } else { sin_u }; // F_1
    let mut current = if n % 2 == 0 {
        previous_even
    } else {
        previous_odd
    };
    let mut k = if n % 2 == 0 { 2 } else { 3 };
    while k <= n {
        let sign = if is_sin { -1 } else { 1 };
        let boundary = graph.mul([
            expr_from_rational(graph, &(q(sign) / q(k as i64))),
            graph.pow(f, graph.integer(k as i64 - 1)),
            g,
        ]);
        let carried = if k % 2 == 0 {
            previous_even
        } else {
            previous_odd
        };
        current = graph.add([
            boundary,
            graph.mul([
                expr_from_rational(graph, &(q(k as i64 - 1) / q(k as i64))),
                carried,
            ]),
        ]);
        if k % 2 == 0 {
            previous_even = current;
        } else {
            previous_odd = current;
        }
        k += 2;
    }
    let _ = x;
    Some(graph.mul([expr_from_rational(graph, &(Q::one() / a)), current]))
}

/// The fallback integrals: `None` if `expr` is not one of the families above.
pub(crate) fn fallback_integral(
    graph: &ExprGraph,
    expr: ExprId,
    wrt: SymbolId,
    x: ExprId,
) -> Option<AlgebraResult<ExprId>> {
    if let Some(v) = integrate_quadratic_radical(graph, expr, wrt, x) {
        return Some(Ok(v));
    }
    if let Some(r) = integrate_by_parts(graph, expr, wrt, x) {
        return Some(r);
    }
    integrate_trig_power(graph, expr, wrt, x).map(Ok)
}
