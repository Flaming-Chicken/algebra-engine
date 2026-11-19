//! Numerical and statistical commands: `nsolve`, `nintegrate`, `minimize`, `maximize`, `mean`, `variance`,
//! `stddev`, `median`, `interpolate`, `fit`.
//!
//! Root finding is Newton's method with the symbolic derivative (damped, with a bisection fallback on a bracket);
//! integration is adaptive Gauss-Kronrod (G7, K15) with the usual substitutions for infinite ranges; optimisation is
//! golden-section search on a bracket or Newton on the derivative; the statistics, the interpolating polynomial and the
//! least-squares fit are computed exactly over the rationals.

use crate::calculus::SymbolicCalculus;
use crate::numeric::{EvalContext, NumericalEval};
use crate::poly::{UniPoly, collect_expr_symbols, expr_from_rational};
use algebra_core::parser::ExprParser;
use algebra_core::{ExprGraph, ExprId, ExprKind, SymbolId};
use num_rational::BigRational;
use num_traits::{One, ToPrimitive, Zero};
use std::collections::{BTreeSet, HashMap};

/// What a numerical command produces.
pub enum NumericOutput {
    /// Plain text (a number with its qualification).
    Text(String),
    /// An expression (an interpolating polynomial) with a note.
    Expression(ExprId, String),
}

type Q = BigRational;

fn split_args(args: &[String]) -> Vec<String> {
    args.iter().map(|a| a.trim().to_string()).collect()
}

fn parse(graph: &ExprGraph, text: &str) -> Result<ExprId, String> {
    ExprParser::new(graph)
        .parse(text)
        .map_err(|e| e.to_string())
}

fn number(graph: &ExprGraph, text: &str) -> Result<f64, String> {
    let id = parse(graph, text)?;
    // `oo` is handled by the callers; everything else must evaluate.
    let value = graph
        .evalf(id, &EvalContext::default())
        .map(|v| v.to_f64())
        .map_err(|e| format!("{text} is not a number: {e}"))?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!(
            "{text} is not a finite number (use oo or -oo for an infinite bound of an integral)"
        ))
    }
}

fn infinity_sign(graph: &ExprGraph, text: &str) -> Option<f64> {
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    match compact.as_str() {
        "oo" | "+oo" | "inf" | "+inf" | "infinity" | "∞" | "+∞" => return Some(1.0),
        "-oo" | "-inf" | "-infinity" | "-∞" => return Some(-1.0),
        _ => {}
    }
    let id = parse(graph, text).ok()?;
    match graph.get(id).kind {
        ExprKind::Number(algebra_core::Number::Constant(algebra_core::Constant::Infinity)) => {
            Some(1.0)
        }
        ExprKind::Number(algebra_core::Number::Constant(algebra_core::Constant::NegInfinity)) => {
            Some(-1.0)
        }
        _ => None,
    }
}

/// The function `f(x)` of one variable as a closure, and the variable's symbol.
struct Function<'g> {
    graph: &'g ExprGraph,
    expr: ExprId,
    name: String,
    symbol: SymbolId,
}

impl<'g> Function<'g> {
    fn new(graph: &'g ExprGraph, text: &str, variable: Option<&str>) -> Result<Self, String> {
        let expr = parse(graph, text)?;
        let symbol = match variable {
            Some(v) => graph.symbols.get_or_intern(v),
            None => {
                let mut symbols = BTreeSet::new();
                collect_expr_symbols(graph, expr, &mut symbols);
                symbols
                    .retain(|&s| !matches!(graph.symbols.resolve(s).as_deref(), Some("pi" | "e")));
                match symbols.len() {
                    0 => graph.symbols.get_or_intern("x"),
                    1 => *symbols.iter().next().ok_or("no variable")?,
                    _ => return Err("the expression has several variables: name the one to use, for example `nsolve(f, x, 1)`".into()),
                }
            }
        };
        let name = graph.symbols.resolve(symbol).unwrap_or_default();
        Ok(Self {
            graph,
            expr,
            name,
            symbol,
        })
    }

    fn at(&self, x: f64) -> f64 {
        let ctx = EvalContext::with_bindings(50, HashMap::from([(self.name.clone(), x)]));
        self.graph
            .evalf(self.expr, &ctx)
            .map(|v| v.to_f64())
            .unwrap_or(f64::NAN)
    }

    fn derivative(&self) -> Function<'g> {
        Function {
            graph: self.graph,
            expr: self.graph.diff(self.expr, self.symbol),
            name: self.name.clone(),
            symbol: self.symbol,
        }
    }
}

/// Split `[f, x, a, b]`-style arguments into the function, an optional variable name and the numeric arguments.
fn function_and_numbers<'g>(
    graph: &'g ExprGraph,
    args: &[String],
) -> Result<(Function<'g>, Vec<String>), String> {
    let args = split_args(args);
    if args.is_empty() {
        return Err("a function is needed".into());
    }
    let (variable, rest): (Option<String>, &[String]) = match args.get(1) {
        Some(second)
            if second.chars().all(|c| c.is_alphabetic() || c == '_')
                && second != "oo"
                && second != "pi"
                && second != "e" =>
        {
            (Some(second.clone()), &args[2..])
        }
        _ => (None, &args[1..]),
    };
    Ok((
        Function::new(graph, &args[0], variable.as_deref())?,
        rest.to_vec(),
    ))
}

// -------------------------------------------------------------------------------------------------------------
// Root finding
// ------------------------------------------------------------------------------------------------------------

fn nsolve(graph: &ExprGraph, args: &[String]) -> Result<NumericOutput, String> {
    let (f, numbers) = function_and_numbers(graph, args)?;
    let df = f.derivative();
    let values: Vec<f64> = numbers
        .iter()
        .map(|n| number(graph, n))
        .collect::<Result<_, _>>()?;
    let root = match values.as_slice() {
        [start] => newton(&f, &df, *start)?,
        [a, b] => bracketed_root(&f, &df, *a, *b)?,
        _ => {
            return Err(
                "nsolve(f, start) or nsolve(f, a, b) (a bracket with a sign change)".into(),
            );
        }
    };
    let residual = f.at(root);
    Ok(NumericOutput::Text(format!(
        "{} ≈ {}  (residual {:e})",
        f.name,
        root,
        residual.abs()
    )))
}

fn newton(f: &Function, df: &Function, start: f64) -> Result<f64, String> {
    let mut x = start;
    for _ in 0..200 {
        let (fx, dfx) = (f.at(x), df.at(x));
        if !fx.is_finite() || !dfx.is_finite() {
            return Err(format!("the function is not defined near {x}"));
        }
        if fx == 0.0 {
            return Ok(x);
        }
        if dfx == 0.0 {
            return Err(format!(
                "the derivative vanishes at {x}: choose another starting point"
            ));
        }
        let mut step = fx / dfx;
        // Damping: halve the step until |f| decreases.
        let mut accepted = false;
        for _ in 0..30 {
            let next = x - step;
            let value = f.at(next);
            if value.is_finite() && value.abs() < fx.abs() * (1.0 + 1e-12) {
                x = next;
                accepted = true;
                break;
            }
            step /= 2.0;
        }
        if !accepted {
            return Err(
                "Newton's method did not converge: try a bracket, `nsolve(f, a, b)`".into(),
            );
        }
        if step.abs() <= 1e-15 * (1.0 + x.abs()) && f.at(x).abs() < 1e-9 {
            return Ok(x);
        }
    }
    if f.at(x).abs() < 1e-9 {
        Ok(x)
    } else {
        Err("no convergence after 200 iterations".into())
    }
}

fn bracketed_root(f: &Function, df: &Function, a: f64, b: f64) -> Result<f64, String> {
    let (mut lo, mut hi) = (a.min(b), a.max(b));
    let (mut flo, fhi) = (f.at(lo), f.at(hi));
    if flo == 0.0 {
        return Ok(lo);
    }
    if fhi == 0.0 {
        return Ok(hi);
    }
    if flo.is_nan() || fhi.is_nan() || flo * fhi >= 0.0 {
        return Err(format!(
            "f({lo}) and f({hi}) have the same sign: the bracket does not contain a root"
        ));
    }
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        let fm = f.at(mid);
        if fm == 0.0 || (hi - lo) < 1e-14 * (1.0 + mid.abs()) {
            lo = mid;
            hi = mid;
            break;
        }
        if flo * fm < 0.0 {
            hi = mid;
        } else {
            lo = mid;
            flo = fm;
        }
    }
    let root = 0.5 * (lo + hi);
    // Polish with Newton if it stays inside the bracket.
    Ok(newton(f, df, root)
        .ok()
        .filter(|r| *r >= a.min(b) && *r <= a.max(b))
        .unwrap_or(root))
}

// -------------------------------------------------------------------------------------------------------------
// Integration
// ------------------------------------------------------------------------------------------------------------

const GK_NODES: [f64; 8] = [
    0.991_455_371_120_812_6,
    0.949_107_912_342_758_5,
    0.864_864_423_359_769_1,
    0.741_531_185_599_394_4,
    0.586_087_235_467_691_1,
    0.405_845_151_377_397_2,
    0.207_784_955_007_898_47,
    0.0,
];
const GK_WEIGHTS_K: [f64; 8] = [
    0.022_935_322_010_529_225,
    0.063_092_092_629_978_55,
    0.104_790_010_322_250_18,
    0.140_653_259_715_525_92,
    0.169_004_726_639_267_9,
    0.190_350_578_064_785_4,
    0.204_432_940_075_298_9,
    0.209_482_141_084_727_82,
];
const GAUSS_WEIGHTS_7: [f64; 4] = [
    0.129_484_966_168_869_7,
    0.279_705_391_489_276_7,
    0.381_830_050_505_118_9,
    0.417_959_183_673_469_4,
];

/// One Gauss-Kronrod (7, 15) panel: the integral and an error estimate.
fn gauss_kronrod(f: &dyn Fn(f64) -> f64, a: f64, b: f64) -> (f64, f64) {
    let (centre, half) = (0.5 * (a + b), 0.5 * (b - a));
    let mut kronrod = GK_WEIGHTS_K[7] * f(centre);
    let mut gauss = GAUSS_WEIGHTS_7[3] * f(centre);
    for i in 0..7 {
        let dx = half * GK_NODES[i];
        let pair = f(centre - dx) + f(centre + dx);
        kronrod += GK_WEIGHTS_K[i] * pair;
        if i % 2 == 1 {
            gauss += GAUSS_WEIGHTS_7[i / 2] * pair;
        }
    }
    (kronrod * half, ((kronrod - gauss) * half).abs())
}

fn adaptive(
    f: &dyn Fn(f64) -> f64,
    a: f64,
    b: f64,
    tolerance: f64,
    depth: u32,
    total_error: &mut f64,
) -> f64 {
    let (value, error) = gauss_kronrod(f, a, b);
    // A non-finite panel (a singularity hit, an overflow) cannot be refined: stop at once instead of splitting forever.
    if !value.is_finite() || !error.is_finite() {
        return f64::NAN;
    }
    if error <= tolerance || depth >= 40 {
        *total_error += error;
        return value;
    }
    let mid = 0.5 * (a + b);
    adaptive(f, a, mid, tolerance / 2.0, depth + 1, total_error)
        + adaptive(f, mid, b, tolerance / 2.0, depth + 1, total_error)
}

fn nintegrate(graph: &ExprGraph, args: &[String]) -> Result<NumericOutput, String> {
    let (f, bounds) = function_and_numbers(graph, args)?;
    if bounds.len() != 2 {
        return Err("nintegrate(f, x, a, b): two bounds are needed".into());
    }
    let (lower_inf, upper_inf) = (
        infinity_sign(graph, &bounds[0]),
        infinity_sign(graph, &bounds[1]),
    );
    let g = |x: f64| f.at(x);
    let mut error = 0.0;
    let tolerance = 1e-13;
    let value = match (lower_inf, upper_inf) {
        (None, None) => {
            let (a, b) = (number(graph, &bounds[0])?, number(graph, &bounds[1])?);
            adaptive(&g, a, b, tolerance, 0, &mut error)
        }
        (None, Some(sign)) if sign > 0.0 => {
            // x = a + t / (1 - t), t in [0, 1)
            let a = number(graph, &bounds[0])?;
            let h = |t: f64| {
                let (x, w) = (a + t / (1.0 - t), 1.0 / ((1.0 - t) * (1.0 - t)));
                g(x) * w
            };
            adaptive(&h, 0.0, 1.0, tolerance, 0, &mut error)
        }
        (Some(sign), None) if sign < 0.0 => {
            // x = b - t / (1 - t)
            let b = number(graph, &bounds[1])?;
            let h = |t: f64| {
                let (x, w) = (b - t / (1.0 - t), 1.0 / ((1.0 - t) * (1.0 - t)));
                g(x) * w
            };
            adaptive(&h, 0.0, 1.0, tolerance, 0, &mut error)
        }
        (Some(l), Some(u)) if l < 0.0 && u > 0.0 => {
            // x = t / (1 - t^2), t in (-1, 1)
            let h = |t: f64| {
                let d = 1.0 - t * t;
                g(t / d) * (1.0 + t * t) / (d * d)
            };
            adaptive(&h, -1.0, 1.0, tolerance, 0, &mut error)
        }
        _ => {
            return Err(
                "these infinite bounds are not supported (use a, oo or -oo, b or -oo, oo)".into(),
            );
        }
    };
    if !value.is_finite() {
        return Err("the integral did not converge to a finite value (a singularity, or a divergent integral)".into());
    }
    Ok(NumericOutput::Text(format!(
        "{} ≈ {}  (estimated error {:e})",
        "integral",
        value,
        error.max(1e-16 * value.abs())
    )))
}

// -------------------------------------------------------------------------------------------------------------
// Optimisation
// ------------------------------------------------------------------------------------------------------------

fn minimize(graph: &ExprGraph, args: &[String], maximise: bool) -> Result<NumericOutput, String> {
    let (f, numbers) = function_and_numbers(graph, args)?;
    let sign = if maximise { -1.0 } else { 1.0 };
    let values: Vec<f64> = numbers
        .iter()
        .map(|n| number(graph, n))
        .collect::<Result<_, _>>()?;
    let x = match values.as_slice() {
        [a, b] => {
            // Golden-section search on [a, b], then a Newton polish on f' if it stays inside.
            let (mut lo, mut hi) = (a.min(*b), a.max(*b));
            let phi = (5f64.sqrt() - 1.0) / 2.0;
            let (mut c, mut d) = (hi - phi * (hi - lo), lo + phi * (hi - lo));
            let (mut fc, mut fd) = (sign * f.at(c), sign * f.at(d));
            for _ in 0..300 {
                if (hi - lo).abs() < 1e-13 * (1.0 + lo.abs()) {
                    break;
                }
                if fc < fd {
                    hi = d;
                    d = c;
                    fd = fc;
                    c = hi - phi * (hi - lo);
                    fc = sign * f.at(c);
                } else {
                    lo = c;
                    c = d;
                    fc = fd;
                    d = lo + phi * (hi - lo);
                    fd = sign * f.at(d);
                }
            }
            let x = 0.5 * (lo + hi);
            let df = f.derivative();
            newton(&df, &df.derivative(), x)
                .ok()
                .filter(|r| (r - x).abs() < 1e-5 && *r >= a.min(*b) && *r <= a.max(*b))
                .unwrap_or(x)
        }
        [start] => {
            let df = f.derivative();
            let ddf = df.derivative();
            let x = newton(&df, &ddf, *start)
                .map_err(|e| format!("could not find a critical point: {e}"))?;
            if sign * ddf.at(x) <= 0.0 {
                return Err(format!(
                    "the critical point x = {x} is a {}, not a {}",
                    if maximise { "minimum" } else { "maximum" },
                    if maximise { "maximum" } else { "minimum" }
                ));
            }
            x
        }
        _ => return Err("minimize(f, start) or minimize(f, a, b)".into()),
    };
    Ok(NumericOutput::Text(format!(
        "{} at {} ≈ {}, value ≈ {}",
        if maximise { "maximum" } else { "minimum" },
        f.name,
        x,
        f.at(x)
    )))
}

// -------------------------------------------------------------------------------------------------------------
// Statistics, interpolation, least squares (exact)
// ------------------------------------------------------------------------------------------------------------

fn exact(graph: &ExprGraph, text: &str) -> Result<Q, String> {
    let id = parse(graph, text.trim())?;
    crate::numeric::try_eval_exact(graph, id, &HashMap::new())
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("{text} is not an exact rational number"))
}

fn list_items(text: &str) -> Vec<String> {
    let trimmed = text.trim();
    // Remove one pair of outer brackets (not every leading bracket: `[[0, 0], [1, 1]]` is a list of lists).
    let t = if trimmed.starts_with('[') && trimmed.ends_with(']') && trimmed.len() >= 2 {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };
    let mut items = Vec::new();
    let (mut depth, mut current) = (0i32, String::new());
    for c in t.chars() {
        match c {
            '[' | '(' => {
                depth += 1;
                current.push(c);
            }
            ']' | ')' => {
                depth -= 1;
                current.push(c);
            }
            ',' if depth == 0 => {
                items.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(c),
        }
    }
    if !current.trim().is_empty() {
        items.push(current.trim().to_string());
    }
    items
}

fn data(graph: &ExprGraph, args: &[String]) -> Result<Vec<Q>, String> {
    let items: Vec<String> = if args.len() == 1 {
        list_items(&args[0])
    } else {
        split_args(args)
    };
    if items.is_empty() {
        return Err("a list of numbers is needed, for example `mean [1, 2, 3]`".into());
    }
    items.iter().map(|i| exact(graph, i)).collect()
}

fn q_text(r: &Q) -> String {
    if r.is_integer() {
        r.numer().to_string()
    } else {
        format!("{}/{}", r.numer(), r.denom())
    }
}

fn decimal(r: &Q) -> f64 {
    r.to_f64().unwrap_or(f64::NAN)
}

fn mean_of(values: &[Q]) -> Q {
    values.iter().fold(Q::zero(), |s, v| s + v) / Q::from_integer((values.len() as i64).into())
}

fn sum_squares(values: &[Q]) -> Q {
    let m = mean_of(values);
    values
        .iter()
        .fold(Q::zero(), |s, v| s + (v - &m) * (v - &m))
}

fn statistics(graph: &ExprGraph, name: &str, args: &[String]) -> Result<NumericOutput, String> {
    let values = data(graph, args)?;
    let n = values.len() as i64;
    let with_decimal = |r: &Q| {
        if r.is_integer() {
            q_text(r)
        } else {
            format!("{} ≈ {}", q_text(r), decimal(r))
        }
    };
    let sqrt_text = |r: &Q| -> String {
        let root = decimal(r).sqrt();
        let (num, den) = (r.numer().clone(), r.denom().clone());
        let (sn, sd) = (num.sqrt(), den.sqrt());
        if (&sn * &sn) == num && (&sd * &sd) == den {
            q_text(&Q::new(sn, sd))
        } else {
            format!("√({}) ≈ {}", q_text(r), root)
        }
    };
    let text = match name {
        "mean" | "average" => with_decimal(&mean_of(&values)),
        "median" => {
            let mut sorted = values.clone();
            sorted.sort();
            let mid = sorted.len() / 2;
            if sorted.len() % 2 == 1 {
                q_text(&sorted[mid])
            } else {
                with_decimal(&((&sorted[mid - 1] + &sorted[mid]) / Q::from_integer(2.into())))
            }
        }
        "variance" | "var" => {
            if n < 2 {
                return Err("the sample variance needs at least two values".into());
            }
            with_decimal(&(sum_squares(&values) / Q::from_integer((n - 1).into())))
        }
        "pvariance" => with_decimal(&(sum_squares(&values) / Q::from_integer(n.into()))),
        "stddev" | "std" | "sd" => {
            if n < 2 {
                return Err("the sample standard deviation needs at least two values".into());
            }
            sqrt_text(&(sum_squares(&values) / Q::from_integer((n - 1).into())))
        }
        "pstddev" => sqrt_text(&(sum_squares(&values) / Q::from_integer(n.into()))),
        "min" => q_text(values.iter().min().ok_or("empty")?),
        "max" => q_text(values.iter().max().ok_or("empty")?),
        other => return Err(format!("unknown statistic {other}")),
    };
    Ok(NumericOutput::Text(format!("{name} = {text}")))
}

fn points(graph: &ExprGraph, arg: &str) -> Result<Vec<(Q, Q)>, String> {
    list_items(arg)
        .iter()
        .map(|item| {
            let pair = list_items(item);
            if pair.len() != 2 {
                return Err(format!("{item} is not an (x, y) pair"));
            }
            Ok((exact(graph, &pair[0])?, exact(graph, &pair[1])?))
        })
        .collect()
}

fn polynomial_expression(graph: &ExprGraph, coeffs: &[Q]) -> ExprId {
    let x = graph.symbol("x");
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

/// The polynomial of lowest degree through the points (Lagrange form, exact).
fn interpolate(graph: &ExprGraph, args: &[String]) -> Result<NumericOutput, String> {
    let pts = points(graph, args.first().ok_or("a list of points is needed")?)?;
    if pts.len() < 2 {
        return Err("at least two points are needed".into());
    }
    for (i, (xi, _)) in pts.iter().enumerate() {
        if pts[..i].iter().any(|(xj, _)| xj == xi) {
            return Err(format!("two points have the same x = {}", q_text(xi)));
        }
    }
    let mut total = UniPoly::zero();
    for (i, (xi, yi)) in pts.iter().enumerate() {
        let mut basis = UniPoly::constant(Q::one());
        for (j, (xj, _)) in pts.iter().enumerate() {
            if i != j {
                let factor =
                    UniPoly::from_coeffs(vec![-xj.clone() / (xi - xj), Q::one() / (xi - xj)]);
                basis = basis.mul(&factor);
            }
        }
        total = total.add(&basis.mul(&UniPoly::constant(yi.clone())));
    }
    let expr = polynomial_expression(graph, &total.coeffs);
    Ok(NumericOutput::Expression(
        expr,
        format!(
            "the polynomial of degree {} through the {} points",
            total.degree(),
            pts.len()
        ),
    ))
}

/// Least-squares polynomial fit of the given degree (default 1), from the exact normal equations.
fn fit(graph: &ExprGraph, args: &[String]) -> Result<NumericOutput, String> {
    let pts = points(graph, args.first().ok_or("a list of points is needed")?)?;
    let degree: usize = match args.get(1) {
        Some(d) => d
            .trim()
            .parse()
            .map_err(|_| format!("{d} is not a degree"))?,
        None => 1,
    };
    if pts.len() <= degree {
        return Err(format!(
            "a degree {degree} fit needs more than {degree} points"
        ));
    }
    let n = degree + 1;
    // Normal equations: sum_j (sum_k x^(i+j)) c_j = sum_k y x^i.
    let power = |x: &Q, e: usize| (0..e).fold(Q::one(), |p, _| p * x);
    let mut matrix = vec![vec![Q::zero(); n + 1]; n];
    for i in 0..n {
        for j in 0..n {
            matrix[i][j] = pts.iter().fold(Q::zero(), |s, (x, _)| s + power(x, i + j));
        }
        matrix[i][n] = pts.iter().fold(Q::zero(), |s, (x, y)| s + y * power(x, i));
    }
    // Gauss-Jordan over the rationals.
    for col in 0..n {
        let pivot = (col..n)
            .find(|&r| !matrix[r][col].is_zero())
            .ok_or("the points do not determine this fit (singular normal equations)")?;
        matrix.swap(col, pivot);
        let p = matrix[col][col].clone();
        for c in col..=n {
            matrix[col][c] = &matrix[col][c] / &p;
        }
        for r in 0..n {
            if r != col && !matrix[r][col].is_zero() {
                let f = matrix[r][col].clone();
                for c in col..=n {
                    let delta = &f * &matrix[col][c];
                    matrix[r][c] -= delta;
                }
            }
        }
    }
    let coeffs: Vec<Q> = (0..n).map(|i| matrix[i][n].clone()).collect();
    let expr = polynomial_expression(graph, &coeffs);
    Ok(NumericOutput::Expression(
        expr,
        format!(
            "least-squares fit of degree {degree} to {} points",
            pts.len()
        ),
    ))
}

/// Run a numerical command by name.
pub fn run(graph: &ExprGraph, name: &str, args: &[String]) -> Result<NumericOutput, String> {
    match name {
        "nsolve" | "findroot" => nsolve(graph, args),
        "nintegrate" | "nint" => nintegrate(graph, args),
        "minimize" => minimize(graph, args, false),
        "maximize" => minimize(graph, args, true),
        "interpolate" | "lagrange" => interpolate(graph, args),
        "fit" | "polyfit" => fit(graph, args),
        "mean" | "average" | "median" | "variance" | "var" | "pvariance" | "stddev" | "std"
        | "sd" | "pstddev" | "min" | "max" => statistics(graph, name, args),
        "divmod" | "floordiv" | "digits" | "egcd" | "crt" | "continued_fraction"
        | "convergents" => {
            let mut parsed: Vec<Vec<Q>> = Vec::new();
            for arg in args {
                let text = arg.trim();
                if text.starts_with('[') {
                    parsed.push(
                        list_items(text)
                            .iter()
                            .map(|i| exact(graph, i))
                            .collect::<Result<_, _>>()?,
                    );
                } else {
                    parsed.push(vec![exact(graph, text)?]);
                }
            }
            let scalar = |i: usize| -> Result<Q, String> {
                parsed
                    .get(i)
                    .and_then(|v| v.first().cloned())
                    .ok_or_else(|| format!("{name} needs more arguments"))
            };
            crate::intlist::run(name, &parsed, scalar).map(NumericOutput::Text)
        }
        other => Err(format!("unknown numerical command {other}")),
    }
}

/// The canonical name of a numerical command spelling.
pub fn canonical_name(name: &str) -> Option<&'static str> {
    Some(match name.to_ascii_lowercase().as_str() {
        "nsolve" | "findroot" | "find_root" => "nsolve",
        "nintegrate" | "nint" | "numeric_integrate" => "nintegrate",
        "minimize" | "argmin" => "minimize",
        "maximize" | "argmax" => "maximize",
        "interpolate" | "lagrange" | "interp" => "interpolate",
        "fit" | "polyfit" | "linfit" => "fit",
        "mean" | "average" | "avg" => "mean",
        "median" => "median",
        "variance" | "var" => "variance",
        "pvariance" => "pvariance",
        "stddev" | "std" | "sd" => "stddev",
        "pstddev" => "pstddev",
        "divmod" => "divmod",
        "floordiv" => "floordiv",
        "digits" => "digits",
        "egcd" | "xgcd" => "egcd",
        "crt" | "chinese_remainder" => "crt",
        "continued_fraction" | "cfrac" | "contfrac" => "continued_fraction",
        "convergents" => "convergents",
        _ => return None,
    })
}
