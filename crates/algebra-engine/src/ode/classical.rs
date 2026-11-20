//! Classical first- and second-order ODE methods that the `dsolve` command needs beyond the constant-coefficient and
//! first-order linear cases:
//!
//! * a **linearity check**: the command read the coefficients of `y''`, `y'` and `y` by substituting `0` and `1`,
//!   which silently turned `y' = y^2` into `y' = y` (and `y' = y (1 - y)` into `y' = 0`); the model is now checked
//!   against the equation at random points before it is trusted;
//! * **separable** equations `y' = f(x) g(y)`: `int dy / g(y) = int f(x) dx + C`, solved for `y` when possible;
//! * constant-coefficient **non-homogeneous** equations `a y'' + b y' + c y = g(x)` by variation of parameters from
//!   the homogeneous basis: $y_p = -y_1 \int y_2 g / (a W) + y_2 \int y_1 g / (a W)$.

use crate::calculus::SymbolicCalculus;
use crate::numeric::{EvalContext, NumericalEval};
use crate::simplify::Simplifier;
use algebra_core::{ExprGraph, ExprId, ExprKind, SymbolId};
use std::collections::HashMap;

fn simplified(graph: &ExprGraph, id: ExprId) -> ExprId {
    // Distribute, merge the exponentials (so `x exp(x) exp(x) - x exp(2 x)` cancels), simplify, merge again.
    let expanded = crate::simplify::AlgebraicTransformations::expand(graph, id);
    let merged = merge_exponentials(graph, expanded);
    let simplified = Simplifier::simplify(graph, merged).unwrap_or(merged);
    merge_exponentials(graph, simplified)
}

/// Rewrite products of exponentials as one exponential, `exp(u) exp(v) = exp(u + v)` and `exp(u) / exp(v) =
/// exp(u - v)` (an exponential of zero is dropped): the general simplifier leaves `exp(x) exp(-x)` as it is, which
/// hides a Wronskian of `-2` behind exponentials.
fn merge_exponentials(graph: &ExprGraph, id: ExprId) -> ExprId {
    /// Split a product-like expression into non-exponential factors and the arguments of its exponential factors
    /// (`sign = -1` inverts them, for a denominator).
    fn split(
        graph: &ExprGraph,
        e: ExprId,
        negate: bool,
        others: &mut Vec<ExprId>,
        args: &mut Vec<ExprId>,
    ) {
        let signed = |u: ExprId| if negate { graph.neg(u) } else { u };
        match graph.get(e).kind.clone() {
            ExprKind::Mul(fs) => {
                for f in fs {
                    split(graph, f, negate, others, args);
                }
            }
            ExprKind::Neg(a) => {
                others.push(graph.integer(-1));
                split(graph, a, negate, others, args);
            }
            ExprKind::Div(a, b) => {
                split(graph, a, negate, others, args);
                split(graph, b, !negate, others, args);
            }
            ExprKind::Function { name, args: fa }
                if fa.len() == 1 && graph.symbols.resolve(name).as_deref() == Some("exp") =>
            {
                args.push(signed(fa[0]));
            }
            ExprKind::Pow(base, exp) => {
                if let ExprKind::Function { name, args: fa } = graph.get(base).kind.clone()
                    && fa.len() == 1
                    && graph.symbols.resolve(name).as_deref() == Some("exp")
                {
                    args.push(signed(graph.mul([exp, fa[0]])));
                } else {
                    let merged = merge_exponentials(graph, base);
                    let p = graph.pow(merged, exp);
                    others.push(if negate {
                        graph.pow(p, graph.integer(-1))
                    } else {
                        p
                    });
                }
            }
            ExprKind::Add(_) | ExprKind::Sub(_, _) => {
                let merged = merge_exponentials(graph, e);
                others.push(if negate {
                    graph.pow(merged, graph.integer(-1))
                } else {
                    merged
                });
            }
            _ => others.push(if negate {
                graph.pow(e, graph.integer(-1))
            } else {
                e
            }),
        }
    }
    match graph.get(id).kind.clone() {
        ExprKind::Add(terms) => graph.add(
            terms
                .iter()
                .map(|&t| merge_exponentials(graph, t))
                .collect::<Vec<_>>(),
        ),
        ExprKind::Sub(a, b) => {
            graph.sub(merge_exponentials(graph, a), merge_exponentials(graph, b))
        }
        ExprKind::Mul(_) | ExprKind::Div(_, _) | ExprKind::Neg(_) | ExprKind::Pow(_, _) => {
            let (mut others, mut args) = (Vec::new(), Vec::new());
            split(graph, id, false, &mut others, &mut args);
            if !args.is_empty() {
                let sum = Simplifier::simplify(graph, graph.add(args.clone()))
                    .unwrap_or_else(|_| graph.add(args));
                if sum != graph.integer(0) {
                    others.push(graph.function("exp", [sum]));
                }
            }
            match others.len() {
                0 => graph.integer(1),
                1 => others[0],
                _ => graph.mul(others),
            }
        }
        _ => id,
    }
}

/// Whether `ode = c2 d2 + c1 d1 + c0 y + base` holds identically (checked at several pseudo-random points).
pub(crate) fn is_linear_model(
    graph: &ExprGraph,
    ode: ExprId,
    symbols: [SymbolId; 4],    // d2, d1, y, x
    coefficients: [ExprId; 4], // c2, c1, c0, base
) -> bool {
    let names: Vec<String> = symbols
        .iter()
        .map(|&s| graph.symbols.resolve(s).unwrap_or_default())
        .collect();
    let points: [[f64; 4]; 4] = [
        [0.7, -1.3, 0.9, 1.7],
        [-1.1, 0.4, 2.3, 0.6],
        [2.2, 1.9, -0.8, 2.9],
        [0.3, -0.6, 1.4, 0.45],
    ];
    for p in points {
        let bindings: HashMap<String, f64> = names.iter().cloned().zip(p).collect();
        let ctx = EvalContext::with_bindings(50, bindings);
        let Ok(actual) = graph.evalf(ode, &ctx).map(|v| v.to_f64()) else {
            return false;
        };
        let value = |id: ExprId| graph.evalf(id, &ctx).map(|v| v.to_f64());
        let (Ok(c2), Ok(c1), Ok(c0), Ok(b)) = (
            value(coefficients[0]),
            value(coefficients[1]),
            value(coefficients[2]),
            value(coefficients[3]),
        ) else {
            return false;
        };
        let model = c2 * p[0] + c1 * p[1] + c0 * p[2] + b;
        if !actual.is_finite()
            || !model.is_finite()
            || (actual - model).abs() > 1e-8 * (1.0 + actual.abs())
        {
            return false;
        }
    }
    true
}

/// Multiplicative factors of an expression: products are flattened, a quotient contributes the numerator's factors
/// and the denominator's as inverse powers, a negation contributes `-1`.
fn factors_of(graph: &ExprGraph, id: ExprId, out: &mut Vec<ExprId>) {
    match &graph.get(id).kind {
        ExprKind::Mul(fs) => {
            for &f in fs {
                factors_of(graph, f, out);
            }
        }
        ExprKind::Div(n, d) => {
            factors_of(graph, *n, out);
            let mut den = Vec::new();
            factors_of(graph, *d, &mut den);
            for f in den {
                out.push(graph.pow(f, graph.integer(-1)));
            }
        }
        ExprKind::Neg(a) => {
            out.push(graph.integer(-1));
            factors_of(graph, *a, out);
        }
        _ => out.push(id),
    }
}

/// The solution of a separable equation.
pub(crate) enum Separable {
    /// Explicit solutions `y = ...` (with the constant `C1`).
    Explicit(Vec<ExprId>),
    /// The implicit solution `g(y) = f(x) + C1`, when it cannot be solved for `y` in closed form.
    Implicit { g: ExprId, f: ExprId },
}

/// The derivative `y'` of a first-order equation `ode = c(x, y) y' + r(x, y)` that is linear in `y'`: `-r / c`. The
/// coefficient `c` is not assumed to be 1 (`y + (x^2 y - x) y' = 0` is `y' = -y / (x^2 y - x)`).
fn solved_for_derivative(graph: &ExprGraph, ode: ExprId, d1: SymbolId) -> Option<ExprId> {
    let coefficient = simplified(graph, graph.diff(ode, d1));
    if graph.has_symbol(coefficient, d1) || coefficient == graph.integer(0) {
        return None;
    }
    let remainder = simplified(graph, graph.substitute(ode, d1, graph.integer(0)));
    Some(simplified(
        graph,
        graph.div(graph.neg(remainder), coefficient),
    ))
}

/// `y' = f(x) g(y)`: the solution, if the right-hand side separates.
pub(crate) fn solve_separable(
    graph: &ExprGraph,
    ode: ExprId,
    d1: SymbolId,
    y: SymbolId,
    x: SymbolId,
) -> Option<Separable> {
    use crate::solver::SymbolicSolver;
    // ode = c(x, y) d1 + r(x, y) is solved for the derivative: h = -r / c.
    let h = solved_for_derivative(graph, ode, d1)?;
    let mut factors = Vec::new();
    factors_of(graph, h, &mut factors);
    let (mut fx, mut gy) = (Vec::new(), Vec::new());
    for f in factors {
        let (has_x, has_y) = (graph.has_symbol(f, x), graph.has_symbol(f, y));
        match (has_x, has_y) {
            (true, true) => return None,
            (false, true) => gy.push(f),
            _ => fx.push(f),
        }
    }
    if gy.is_empty() {
        return None; // y' = f(x): a plain integral, handled elsewhere
    }
    let f_expr = if fx.is_empty() {
        graph.integer(1)
    } else {
        graph.mul(fx)
    };
    let g_expr = graph.mul(gy);
    // G(y) = int dy / g(y), F(x) = int f dx.
    let big_g = graph
        .integrate(simplified(graph, graph.div(graph.integer(1), g_expr)), y)
        .ok()?;
    let big_f = graph.integrate(f_expr, x).ok()?;
    let c1 = graph.symbol("C1");
    let implicit = graph.sub(graph.sub(big_g, big_f), c1);
    match graph.solveset(implicit, y) {
        Ok(solutions)
            if !solutions.is_empty() && !solutions.iter().any(|&s| graph.has_symbol(s, y)) =>
        {
            Some(Separable::Explicit(
                solutions
                    .into_iter()
                    .map(|s| simplified(graph, s))
                    .collect(),
            ))
        }
        _ => Some(Separable::Implicit {
            g: simplified(graph, big_g),
            f: simplified(graph, big_f),
        }),
    }
}

/// A particular solution of `a y'' + b y' + c y = g(x)` from the homogeneous basis `y1`, `y2`, by variation of
/// parameters, as the full solution `C1 y1 + C2 y2 + y_p`.
pub(crate) fn variation_of_parameters(
    graph: &ExprGraph,
    a: ExprId,
    forcing: ExprId,
    y1: ExprId,
    y2: ExprId,
    x: SymbolId,
) -> Option<ExprId> {
    let dy1 = graph.diff(y1, x);
    let dy2 = graph.diff(y2, x);
    let wronskian = simplified(graph, graph.sub(graph.mul([y1, dy2]), graph.mul([dy1, y2])));
    let denominator = simplified(graph, graph.mul([a, wronskian]));
    // Simplified, then merged again: the general simplifier rewrites `exp(2 x)` as `exp(x)^2`, which the integrator
    // does not take.
    let integrand = |u: ExprId| {
        merge_exponentials(
            graph,
            simplified(graph, graph.div(graph.mul([u, forcing]), denominator)),
        )
    };
    let first = graph.integrate(integrand(y2), x).ok()?;
    let second = graph.integrate(integrand(y1), x).ok()?;
    let particular = graph.add([graph.neg(graph.mul([y1, first])), graph.mul([y2, second])]);
    let c1 = graph.symbol("C1");
    let c2 = graph.symbol("C2");
    let solution = graph.add([graph.mul([c1, y1]), graph.mul([c2, y2]), particular]);
    Some(simplified(graph, solution))
}

// ---------------------------------------------------------------------------------------------------------------
// Bernoulli equations
// ---------------------------------------------------------------------------------------------------------------

/// `y' + P(x) y = Q(x) y^n` written as `y' = a(x) y + b(x) y^n` (`n` an integer other than 0 and 1): the solution through
/// the substitution `v = y^(1 - n)`.
pub(crate) fn solve_bernoulli_form(
    graph: &ExprGraph,
    ode: ExprId,
    d1: SymbolId,
    y: SymbolId,
    x: SymbolId,
) -> Option<ExprId> {
    let h = solved_for_derivative(graph, ode, d1)?;
    let terms: Vec<ExprId> = match &graph.get(h).kind {
        ExprKind::Add(ts) => ts.to_vec(),
        ExprKind::Sub(a, b) => vec![*a, graph.neg(*b)],
        _ => return None,
    };
    if terms.len() != 2 {
        return None;
    }
    // (x-part, power of y) of one term.
    let split = |term: ExprId| -> Option<(ExprId, i64)> {
        let mut factors = Vec::new();
        factors_of(graph, term, &mut factors);
        let (mut rest, mut power) = (Vec::new(), 0i64);
        for f in factors {
            if !graph.has_symbol(f, y) {
                rest.push(f);
            } else if let ExprKind::Symbol(s) = graph.get(f).kind
                && s == y
            {
                power += 1;
            } else if let ExprKind::Pow(base, e) = graph.get(f).kind.clone()
                && let ExprKind::Symbol(s) = graph.get(base).kind
                && s == y
                && let ExprKind::Number(n) = &graph.get(e).kind
                && let Some(v) = n.as_f64()
                && v.fract() == 0.0
            {
                power += v as i64;
            } else {
                return None;
            }
        }
        let coefficient = if rest.is_empty() {
            graph.integer(1)
        } else {
            graph.mul(rest)
        };
        Some((coefficient, power))
    };
    let (t0, t1) = (split(terms[0])?, split(terms[1])?);
    let ((a_x, _), (b_x, n)) = match (t0.1, t1.1) {
        (1, n) if n != 1 && n != 0 => (t0, (t1.0, n)),
        (n, 1) if n != 1 && n != 0 => (t1, (t0.0, n)),
        _ => return None,
    };
    let p = graph.neg(a_x);
    crate::ode::SymbolicOdeSolver::solve_bernoulli(graph, p, b_x, n, x).ok()
}

// ---------------------------------------------------------------------------------------------------------------
// Linear equations of any order: constant coefficients and Euler (equidimensional) equations
// ---------------------------------------------------------------------------------------------------------------

use crate::poly::expr_from_rational;
use crate::poly::factor::factor_rational_polynomial;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Basis {
    /// `exp(r x)` with `x^k` for repeated roots.
    Exponential,
    /// `x^r` with `ln(x)^k` for repeated roots (Euler equations).
    Euler,
}

/// The general solution `C1 y1 + ... + Cn yn` for the roots of a characteristic polynomial (ascending rational
/// coefficients), or `None` when it has an irreducible factor of degree three or more.
fn general_solution(
    graph: &ExprGraph,
    characteristic: &[BigRational],
    basis: Basis,
    x: SymbolId,
) -> Option<ExprId> {
    let xe = graph.symbol(&graph.symbols.resolve(x)?);
    let ln_x = graph.function("ln", [xe]);
    let t = match basis {
        Basis::Exponential => xe,
        Basis::Euler => ln_x,
    };
    let real = |r: ExprId| match basis {
        Basis::Exponential => graph.function("exp", [graph.mul([r, xe])]),
        Basis::Euler => graph.pow(xe, r),
    };
    let (_, factors) =
        factor_rational_polynomial(&crate::poly::UniPoly::from_coeffs(characteristic.to_vec()))
            .ok()?;
    let four = BigRational::from_integer(4.into());
    let two = BigRational::from_integer(2.into());
    let mut basis_functions: Vec<ExprId> = Vec::new();
    for (factor, multiplicity) in factors {
        let mut roots: Vec<ExprId> = Vec::new();
        let mut oscillating: Option<(ExprId, ExprId)> = None; // (alpha, beta) of alpha +- i beta
        match factor.degree() {
            1 => roots.push(expr_from_rational(graph, &-factor.coeffs[0].clone())),
            2 => {
                let (b, c) = (factor.coeffs[1].clone(), factor.coeffs[0].clone());
                let disc = &b * &b - &four * &c;
                let centre = expr_from_rational(graph, &-(&b / &two));
                if disc.is_negative() {
                    let beta_sq = -disc / &four;
                    oscillating = Some((
                        centre,
                        graph.function("sqrt", [expr_from_rational(graph, &beta_sq)]),
                    ));
                } else {
                    let half_root =
                        graph.function("sqrt", [expr_from_rational(graph, &(disc / &four))]);
                    roots.push(graph.add([centre, half_root]));
                    roots.push(graph.sub(centre, half_root));
                }
            }
            _ => return None,
        }
        for k in 0..multiplicity {
            let with_power = |e: ExprId| {
                if k == 0 {
                    e
                } else if k == 1 {
                    graph.mul([t, e])
                } else {
                    graph.mul([graph.pow(t, graph.integer(k as i64)), e])
                }
            };
            for &r in &roots {
                basis_functions.push(with_power(real(r)));
            }
            if let Some((alpha, beta)) = oscillating {
                let angle = match basis {
                    Basis::Exponential => graph.mul([beta, xe]),
                    Basis::Euler => graph.mul([beta, ln_x]),
                };
                let amplitude = if alpha == graph.integer(0) {
                    graph.integer(1)
                } else {
                    real(alpha)
                };
                basis_functions.push(with_power(
                    graph.mul([amplitude, graph.function("cos", [angle])]),
                ));
                basis_functions.push(with_power(
                    graph.mul([amplitude, graph.function("sin", [angle])]),
                ));
            }
        }
    }
    let terms: Vec<ExprId> = basis_functions
        .iter()
        .enumerate()
        .map(|(i, &f)| graph.mul([graph.symbol(&format!("C{}", i + 1)), f]))
        .collect();
    Some(simplified(graph, graph.add(terms)))
}

fn rational_of(value: f64) -> Option<BigRational> {
    if !value.is_finite() {
        return None;
    }
    for den in 1..=1000i64 {
        let num = (value * den as f64).round();
        if (num / den as f64 - value).abs() < 1e-9 {
            return Some(BigRational::new((num as i64).into(), den.into()));
        }
    }
    None
}

/// A homogeneous linear equation of any order with constant coefficients, or an Euler equation
/// `sum a_k x^k y^(k) = 0`, solved exactly from the factorisation of its characteristic polynomial.
///
/// `derivatives[k - 1]` is the symbol standing for `y^(k)`. Returns `None` if the equation is not of one of these
/// forms (or is not linear, or the characteristic polynomial has an irreducible factor of degree three or more).
pub(crate) fn solve_linear_homogeneous(
    graph: &ExprGraph,
    ode: ExprId,
    derivatives: &[SymbolId],
    y: SymbolId,
    x: SymbolId,
) -> Option<ExprId> {
    let order = derivatives.len();
    let zero = graph.integer(0);
    let x_name = graph.symbols.resolve(x)?;
    let value_at = |expr: ExprId, xv: f64| -> Option<f64> {
        let ctx = EvalContext::with_bindings(50, HashMap::from([(x_name.clone(), xv)]));
        graph.evalf(expr, &ctx).ok().map(|v| v.to_f64())
    };
    let substitute_all = |one_at: Option<usize>| -> ExprId {
        let mut e = ode;
        for (k, &d) in derivatives.iter().enumerate() {
            let value = if one_at == Some(k + 1) {
                graph.integer(1)
            } else {
                zero
            };
            e = graph.substitute(e, d, value);
        }
        let y_value = if one_at == Some(0) {
            graph.integer(1)
        } else {
            zero
        };
        graph.substitute(e, y, y_value)
    };
    let base = substitute_all(None);
    if value_at(base, 0.7)?.abs() > 1e-12 || value_at(base, 1.9)?.abs() > 1e-12 {
        return None; // not homogeneous
    }
    let coefficient: Vec<ExprId> = (0..=order)
        .map(|k| simplified(graph, graph.sub(substitute_all(Some(k)), base)))
        .collect();
    // The linear model must reproduce the equation.
    let mut symbols: Vec<SymbolId> = vec![y];
    symbols.extend(derivatives);
    for point in [[0.7f64, 1.3f64], [-0.9, 0.6], [1.7, 2.2]] {
        let mut bindings: HashMap<String, f64> = HashMap::new();
        bindings.insert(x_name.clone(), point[1] + 0.1);
        let mut values = Vec::new();
        for (i, &s) in symbols.iter().enumerate() {
            let v = point[0] + 0.37 * i as f64;
            bindings.insert(graph.symbols.resolve(s)?, v);
            values.push(v);
        }
        let ctx = EvalContext::with_bindings(50, bindings);
        let actual = graph.evalf(ode, &ctx).ok()?.to_f64();
        let mut model = 0.0;
        for (i, v) in values.iter().enumerate() {
            model += graph.evalf(coefficient[i], &ctx).ok()?.to_f64() * v;
        }
        if (actual - model).abs() > 1e-8 * (1.0 + actual.abs()) {
            return None;
        }
    }
    // Constant coefficients?
    let constant: Option<Vec<BigRational>> = coefficient
        .iter()
        .map(|&c| {
            let (a, b) = (value_at(c, 0.7)?, value_at(c, 1.9)?);
            if (a - b).abs() <= 1e-9 * (1.0 + a.abs()) {
                rational_of(a)
            } else {
                None
            }
        })
        .collect();
    if let Some(c) = constant
        && !c[order].is_zero()
    {
        return general_solution(graph, &c, Basis::Exponential, x);
    }
    // Euler: c_k(x) = a_k x^k with constant a_k.
    let mut a: Vec<BigRational> = Vec::new();
    for (k, &c) in coefficient.iter().enumerate() {
        let (p1, p2) = (0.7f64, 1.9f64);
        let (v1, v2) = (
            value_at(c, p1)? / p1.powi(k as i32),
            value_at(c, p2)? / p2.powi(k as i32),
        );
        if (v1 - v2).abs() > 1e-9 * (1.0 + v1.abs()) {
            return None;
        }
        a.push(rational_of(v1)?);
    }
    if a[order].is_zero() {
        return None;
    }
    // Indicial polynomial: sum a_k r (r - 1) ... (r - k + 1), ascending.
    let mut falling: Vec<BigRational> = vec![BigRational::one()];
    let mut indicial = vec![BigRational::zero(); order + 1];
    for (k, a_k) in a.iter().enumerate() {
        for (i, c) in falling.iter().enumerate() {
            indicial[i] += a_k * c;
        }
        let mut next = vec![BigRational::zero(); falling.len() + 1];
        for (i, c) in falling.iter().enumerate() {
            next[i + 1] += c;
            next[i] -= c * BigRational::from_integer((k as i64).into());
        }
        falling = next;
    }
    general_solution(graph, &indicial, Basis::Euler, x)
}

/// `exp(u)` with logarithms taken out: `exp(a ln v)` is `v^a`, `exp(ln v)` is `v`, and a sum in the exponent becomes a
/// product, so that an integrating factor such as `exp(2 ln x)` is `x^2` (which the integrator can use) rather than an
/// opaque exponential.
pub(crate) fn exp_clean(graph: &ExprGraph, exponent: ExprId) -> ExprId {
    let terms: Vec<ExprId> = match &graph.get(exponent).kind {
        ExprKind::Add(ts) => ts.to_vec(),
        _ => vec![exponent],
    };
    let is_ln = |id: ExprId| -> Option<ExprId> {
        if let ExprKind::Function { name, args } = &graph.get(id).kind
            && args.len() == 1
            && matches!(graph.symbols.resolve(*name).as_deref(), Some("ln" | "log"))
        {
            Some(args[0])
        } else {
            None
        }
    };
    let factors: Vec<ExprId> = terms
        .iter()
        .map(|&t| {
            if let Some(v) = is_ln(t) {
                return v;
            }
            if let ExprKind::Neg(inner) = &graph.get(t).kind
                && let Some(v) = is_ln(*inner)
            {
                return graph.pow(v, graph.integer(-1));
            }
            if let ExprKind::Mul(fs) = &graph.get(t).kind {
                let logs: Vec<usize> = (0..fs.len()).filter(|&i| is_ln(fs[i]).is_some()).collect();
                if logs.len() == 1 {
                    let i = logs[0];
                    let rest: Vec<ExprId> = fs
                        .iter()
                        .enumerate()
                        .filter(|(j, _)| *j != i)
                        .map(|(_, &f)| f)
                        .collect();
                    let v = is_ln(fs[i]).unwrap_or(fs[i]);
                    let power = if rest.is_empty() {
                        graph.integer(1)
                    } else {
                        graph.mul(rest)
                    };
                    return graph.pow(v, power);
                }
            }
            graph.function("exp", [t])
        })
        .collect();
    match factors.len() {
        1 => factors[0],
        _ => graph.mul(factors),
    }
}
