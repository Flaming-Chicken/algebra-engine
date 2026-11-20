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
use crate::simplify::SymbolicSimplifier;
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

pub(crate) fn parse(graph: &ExprGraph, text: &str) -> Result<ExprId, String> {
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

pub(crate) fn list_items(text: &str) -> Vec<String> {
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
        "percentile" | "quantile" => {
            return percentile_command(graph, name, args);
        }
        "quartiles" => {
            let mut sorted = values.clone();
            sorted.sort();
            let at = |p: i64| interpolate_sorted(&sorted, &Q::new(p.into(), 100.into()));
            format!(
                "Q1 = {}, Q2 (median) = {}, Q3 = {}, IQR = {}",
                q_text(&at(25)),
                q_text(&at(50)),
                q_text(&at(75)),
                q_text(&(at(75) - at(25)))
            )
        }
        "iqr" => {
            let mut sorted = values.clone();
            sorted.sort();
            q_text(
                &(interpolate_sorted(&sorted, &Q::new(3.into(), 4.into()))
                    - interpolate_sorted(&sorted, &Q::new(1.into(), 4.into()))),
            )
        }
        "skewness" | "kurtosis" => {
            let m = mean_of(&values);
            let count = Q::from_integer(n.into());
            let moment = |power: u32| {
                values.iter().fold(Q::zero(), |s, v| {
                    let d = v - &m;
                    let mut p = Q::one();
                    for _ in 0..power {
                        p *= &d;
                    }
                    s + p
                }) / &count
            };
            let m2 = moment(2);
            if m2.is_zero() {
                return Err(format!("the {name} is undefined for constant data"));
            }
            if name == "skewness" {
                // g1 = m3 / m2^(3/2): rational when m3^2 / m2^3 is a rational square.
                let m3 = moment(3);
                let squared = (&m3 * &m3) / (&m2 * &m2 * &m2);
                let value = decimal(&m3) / decimal(&m2).powf(1.5);
                let (sn, sd) = (squared.numer().sqrt(), squared.denom().sqrt());
                if &sn * &sn == *squared.numer() && &sd * &sd == *squared.denom() {
                    let r = Q::new(sn, sd);
                    with_decimal(&if m3 < Q::zero() { -r } else { r })
                } else {
                    format!("{value}")
                }
            } else {
                // Excess kurtosis m4 / m2^2 - 3 (rational).
                with_decimal(&(moment(4) / (&m2 * &m2) - Q::from_integer(3.into())))
            }
        }
        "zscore" => {
            if n < 2 {
                return Err("z-scores need at least two values".into());
            }
            let m = mean_of(&values);
            let variance = sum_squares(&values) / Q::from_integer(n.into());
            let deviation = decimal(&variance).sqrt();
            let scores: Vec<String> = values
                .iter()
                .map(|v| {
                    let z = decimal(&(v - &m)) / deviation;
                    format!("{z:.6}")
                })
                .collect();
            format!("[{}] (population standard deviation)", scores.join(", "))
        }
        "range" => {
            q_text(&(values.iter().max().ok_or("empty")? - values.iter().min().ok_or("empty")?))
        }
        "mode" => {
            let mut counts: Vec<(Q, usize)> = Vec::new();
            for v in &values {
                match counts.iter_mut().find(|(x, _)| x == v) {
                    Some(entry) => entry.1 += 1,
                    None => counts.push((v.clone(), 1)),
                }
            }
            let best = counts.iter().map(|(_, c)| *c).max().ok_or("empty")?;
            let modes: Vec<String> = counts
                .iter()
                .filter(|(_, c)| *c == best)
                .map(|(v, _)| q_text(v))
                .collect();
            if best == 1 && counts.len() > 1 {
                "no value repeats (every value is a mode)".to_string()
            } else {
                modes.join(", ")
            }
        }
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

/// `product(f, k, a, b)`: the exact finite product, or a closed form for a symbolic upper limit.
fn product(graph: &ExprGraph, args: &[String]) -> Result<NumericOutput, String> {
    let args = split_args(args);
    let [f_text, var, lower, upper] = args.as_slice() else {
        return Err("product(f, k, a, b), for example product(k, k, 1, 5)".into());
    };
    let f = parse(graph, f_text)?;
    let k = graph.symbols.get_or_intern(var);
    let k_expr = graph.symbol(var);
    let a = exact(graph, lower)
        .ok()
        .filter(|q| q.is_integer())
        .ok_or("the lower limit must be an integer")?;
    let a_int = a
        .to_integer()
        .to_i64()
        .ok_or("the lower limit is too large")?;
    if let Ok(b) = exact(graph, upper)
        && b.is_integer()
    {
        let b_int = b
            .to_integer()
            .to_i64()
            .ok_or("the upper limit is too large")?;
        if b_int < a_int {
            return Ok(NumericOutput::Expression(
                graph.integer(1),
                "empty product".into(),
            ));
        }
        if b_int - a_int > 5000 {
            return Err("too many factors for an exact product (limit 5000)".into());
        }
        let factors: Vec<ExprId> = (a_int..=b_int)
            .map(|v| graph.substitute(f, k, graph.integer(v)))
            .collect();
        let result = graph.simplify(graph.mul(factors));
        return Ok(NumericOutput::Expression(
            result,
            "exact finite product".into(),
        ));
    }
    // Symbolic upper limit n.
    let n = parse(graph, upper)?;
    let count = graph.add([n, graph.integer(1 - a_int)]);
    let closed = if !graph.has_symbol(f, k) {
        Some(graph.pow(f, count))
    } else if f == k_expr {
        let top = graph.function("factorial", [n]);
        Some(if a_int <= 1 {
            top
        } else {
            let below = (1..a_int).fold(1i64, |p, v| p.saturating_mul(v));
            graph.div(top, graph.integer(below))
        })
    } else if let ExprKind::Pow(base, exponent) = graph.get(f).kind.clone()
        && exponent == k_expr
        && !graph.has_symbol(base, k)
    {
        // base^(a + ... + n) = base^((n (n + 1) - a (a - 1)) / 2)
        let triangular = graph.div(
            graph.add([
                graph.mul([n, graph.add([n, graph.integer(1)])]),
                graph.integer(-(a_int * (a_int - 1))),
            ]),
            graph.integer(2),
        );
        Some(graph.pow(base, triangular))
    } else {
        None
    };
    match closed {
        Some(id) => Ok(NumericOutput::Expression(
            graph.simplify(id),
            "closed form".into(),
        )),
        None => Err("no closed form for this product with a symbolic upper limit".into()),
    }
}

/// Z-transform of a sequence that is a sum of terms `c n^m a^n` (also `exp(b n)`, `cos` and `sin` of a multiple of
/// `n` through their exponential forms are not covered): `Z{a^n} = z/(z - a)` and `Z{n x[n]} = -z d/dz Z{x[n]}`.
fn ztransform(graph: &ExprGraph, args: &[String]) -> Result<NumericOutput, String> {
    let args = split_args(args);
    let f = parse(graph, args.first().ok_or("ztransform(x[n], n, z)")?)?;
    let n_name = args.get(1).map(String::as_str).unwrap_or("n");
    let z_name = args.get(2).map(String::as_str).unwrap_or("z");
    let (n, z) = (
        graph.symbols.get_or_intern(n_name),
        graph.symbols.get_or_intern(z_name),
    );
    let (n_expr, z_expr) = (graph.symbol(n_name), graph.symbol(z_name));
    let expanded = crate::simplify::AlgebraicTransformations::expand(graph, f);
    let terms: Vec<ExprId> = match graph.get(expanded).kind.clone() {
        ExprKind::Add(ts) => ts.to_vec(),
        _ => vec![expanded],
    };
    let mut total = Vec::new();
    for term in terms {
        let factors: Vec<ExprId> = match graph.get(term).kind.clone() {
            ExprKind::Mul(fs) => fs.to_vec(),
            ExprKind::Neg(inner) => vec![graph.integer(-1), inner],
            _ => vec![term],
        };
        let (mut coefficient, mut base, mut power) = (Vec::new(), Vec::new(), 0i64);
        for factor in factors {
            if !graph.has_symbol(factor, n) {
                coefficient.push(factor);
                continue;
            }
            match graph.get(factor).kind.clone() {
                ExprKind::Symbol(_) if factor == n_expr => power += 1,
                ExprKind::Pow(b, e) if b == n_expr && !graph.has_symbol(e, n) => {
                    let k = crate::numeric::try_eval_exact(graph, e, &HashMap::new())
                        .ok()
                        .flatten()
                        .filter(|q| q.is_integer() && *q >= Q::zero())
                        .and_then(|q| q.to_integer().to_i64())
                        .ok_or("only non-negative integer powers of n are supported")?;
                    power += k;
                }
                ExprKind::Pow(b, e) if !graph.has_symbol(b, n) => {
                    // b^(c n)
                    let c = graph.simplify(graph.diff(e, n));
                    if graph.has_symbol(c, n)
                        || graph.simplify(graph.sub(e, graph.mul([c, n_expr]))) != graph.integer(0)
                    {
                        return Err("the exponent must be a multiple of n".into());
                    }
                    base.push(graph.pow(b, c));
                }
                ExprKind::Function { name, args }
                    if graph.symbols.resolve(name).as_deref() == Some("exp") && args.len() == 1 =>
                {
                    let c = graph.simplify(graph.diff(args[0], n));
                    if graph.has_symbol(c, n)
                        || graph.simplify(graph.sub(args[0], graph.mul([c, n_expr])))
                            != graph.integer(0)
                    {
                        return Err("the exponent must be a multiple of n".into());
                    }
                    base.push(graph.function("exp", [c]));
                }
                _ => {
                    return Err(
                        "ztransform handles sums of terms c n^m a^n (and exp(b n)); this sequence has another form"
                            .into(),
                    );
                }
            }
        }
        // Z{a^n} = z / (z - a); apply -z d/dz `power` times.
        let a = if base.is_empty() {
            graph.integer(1)
        } else {
            graph.mul(base)
        };
        let mut image = graph.div(z_expr, graph.sub(z_expr, a));
        for _ in 0..power {
            image = graph.simplify(graph.mul([graph.integer(-1), z_expr, graph.diff(image, z)]));
        }
        coefficient.push(image);
        total.push(graph.mul(coefficient));
    }
    let result = graph.simplify(graph.add(total));
    Ok(NumericOutput::Expression(
        result,
        "unilateral Z-transform".into(),
    ))
}

/// `matpow(M, n)`: an integer power of a square matrix (negative powers through the inverse).
fn matrix_power(graph: &ExprGraph, args: &[String]) -> Result<NumericOutput, String> {
    use crate::matrix::SymbolicMatrix;
    let args = split_args(args);
    let [matrix, power] = args.as_slice() else {
        return Err("matpow(M, n), for example matpow([[1, 1], [1, 0]], 10)".into());
    };
    let base = SymbolicMatrix::from_expr_str(graph, matrix).map_err(|e| e.to_string())?;
    if base.rows != base.cols {
        return Err("only a square matrix has a power".into());
    }
    let n = exact(graph, power)
        .ok()
        .filter(|q| q.is_integer())
        .and_then(|q| q.to_integer().to_i64())
        .ok_or("the exponent must be an integer")?;
    if n.abs() > 100_000 {
        return Err("the exponent is too large".into());
    }
    let simplify_all = |m: &SymbolicMatrix| -> Result<SymbolicMatrix, String> {
        let elems: Vec<ExprId> = m.elements.iter().map(|&e| graph.simplify(e)).collect();
        SymbolicMatrix::new(m.rows, m.cols, elems).map_err(|e| e.to_string())
    };
    let mut factor = if n < 0 {
        simplify_all(&base.inverse(graph).map_err(|e| e.to_string())?)?
    } else {
        base
    };
    let mut result = SymbolicMatrix::identity(graph, factor.rows);
    let mut e = n.unsigned_abs();
    while e > 0 {
        if e & 1 == 1 {
            result = simplify_all(&result.multiply(graph, &factor).map_err(|e| e.to_string())?)?;
        }
        e >>= 1;
        if e > 0 {
            factor = simplify_all(&factor.multiply(graph, &factor).map_err(|e| e.to_string())?)?;
        }
    }
    Ok(NumericOutput::Expression(
        result.to_expr_id(graph),
        format!("matrix power {n}"),
    ))
}

/// `lu(M)`: the LU decomposition `M = L U` as two matrices.
fn lu(graph: &ExprGraph, args: &[String]) -> Result<NumericOutput, String> {
    use crate::matrix::SymbolicMatrix;
    use algebra_core::format::{Formatter, UnicodeFormatter};
    let args = split_args(args);
    let matrix = args.first().ok_or("lu(M)")?;
    let m = SymbolicMatrix::from_expr_str(graph, matrix).map_err(|e| e.to_string())?;
    let (l, u) = m.lu_decomposition(graph).map_err(|e| e.to_string())?;
    let show = |m: &SymbolicMatrix| -> Result<String, String> {
        let elems: Vec<ExprId> = m.elements.iter().map(|&e| graph.simplify(e)).collect();
        let simplified = SymbolicMatrix::new(m.rows, m.cols, elems).map_err(|e| e.to_string())?;
        UnicodeFormatter
            .format(graph, simplified.to_expr_id(graph))
            .map_err(|e| e.to_string())
    };
    Ok(NumericOutput::Text(format!(
        "L = {}, U = {}",
        show(&l)?,
        show(&u)?
    )))
}

/// `trigsimp(e)` and `expand_trig(e)`.
fn trigonometric(graph: &ExprGraph, name: &str, args: &[String]) -> Result<NumericOutput, String> {
    let args = split_args(args);
    let text = args.first().ok_or("an expression is needed")?;
    let expr = parse(graph, text)?;
    let result = if name == "trigsimp" {
        crate::simplify::trig_expand::trigsimp(graph, expr)
    } else {
        graph.simplify(crate::simplify::AlgebraicTransformations::expand(
            graph,
            crate::simplify::trig_expand::expand_trig(graph, expr),
        ))
    };
    Ok(NumericOutput::Expression(result, name.replace('_', " ")))
}

/// `covariance` and `correlation` of paired data: a list of pairs, or two lists.
fn paired(graph: &ExprGraph, name: &str, args: &[String]) -> Result<NumericOutput, String> {
    let args = split_args(args);
    let pairs: Vec<(Q, Q)> = if args.len() == 2 {
        let (xs, ys) = (data(graph, &args[..1])?, data(graph, &args[1..])?);
        if xs.len() != ys.len() {
            return Err("the two lists need the same length".into());
        }
        xs.into_iter().zip(ys).collect()
    } else {
        points(
            graph,
            args.first().ok_or("a list of (x, y) pairs is needed")?,
        )?
    };
    let n = pairs.len();
    if n < 2 {
        return Err("at least two pairs are needed".into());
    }
    let count = Q::from_integer((n as i64).into());
    let mean_x = pairs.iter().fold(Q::zero(), |s, (x, _)| s + x) / &count;
    let mean_y = pairs.iter().fold(Q::zero(), |s, (_, y)| s + y) / &count;
    let sxy = pairs
        .iter()
        .fold(Q::zero(), |s, (x, y)| s + (x - &mean_x) * (y - &mean_y));
    let sxx = pairs
        .iter()
        .fold(Q::zero(), |s, (x, _)| s + (x - &mean_x) * (x - &mean_x));
    let syy = pairs
        .iter()
        .fold(Q::zero(), |s, (_, y)| s + (y - &mean_y) * (y - &mean_y));
    let show = |r: &Q| {
        if r.is_integer() {
            q_text(r)
        } else {
            format!("{} ≈ {}", q_text(r), decimal(r))
        }
    };
    if name == "covariance" {
        let cov = sxy / Q::from_integer(((n - 1) as i64).into());
        return Ok(NumericOutput::Text(format!("covariance = {}", show(&cov))));
    }
    if sxx.is_zero() || syy.is_zero() {
        return Err("the correlation is undefined when one variable is constant".into());
    }
    // r = sxy / sqrt(sxx syy): exact when sxy^2 / (sxx syy) is the square of a rational.
    let squared = (&sxy * &sxy) / (&sxx * &syy);
    let (sn, sd) = (squared.numer().sqrt(), squared.denom().sqrt());
    let text = if &sn * &sn == *squared.numer() && &sd * &sd == *squared.denom() {
        let r = Q::new(sn, sd);
        show(&if sxy < Q::zero() { -r } else { r })
    } else {
        let value = decimal(&sxy) / (decimal(&sxx) * decimal(&syy)).sqrt();
        format!("sign · √({}) ≈ {value}", q_text(&squared))
            .replace("sign · ", if sxy < Q::zero() { "-" } else { "" })
    };
    Ok(NumericOutput::Text(format!("correlation = {text}")))
}

/// Text-argument integer commands: radix conversion, listings, `pell(d)` and `diophantine(a x + b y = c)`.
fn integer_text_commands(
    graph: &ExprGraph,
    name: &str,
    args: &[String],
) -> Result<NumericOutput, String> {
    use num_bigint::BigInt;
    let args = split_args(args);
    let first = args.first().ok_or("an argument is needed")?;
    let text = match name {
        "dec2bin" | "bin2dec" | "dec2hex" | "hex" | "hex2dec" | "dec2oct" | "oct" | "oct2dec" => {
            crate::intlist::radix(name, first)?
        }
        "subsets" | "powerset" | "power_set" | "permutations_list" => {
            let labels: Vec<String> = list_items(first)
                .iter()
                .map(|i| {
                    exact(graph, i)
                        .map(|q| q_text(&q))
                        .unwrap_or_else(|_| i.trim().to_string())
                })
                .collect();
            crate::intlist::listing_text(name, &labels)?
        }
        "cycle_type" | "next_permutation" => {
            let items: Vec<Q> = list_items(first)
                .iter()
                .map(|i| exact(graph, i))
                .collect::<Result<_, _>>()?;
            crate::intlist::listing(name, &items)?
        }
        "pell" => {
            let d = exact(graph, first)?;
            if !d.is_integer() {
                return Err("pell needs an integer d".into());
            }
            let (x, y) = crate::intlist::pell(&d.to_integer())?;
            format!("pell({d}): x^2 - {d} y^2 = 1 has the fundamental solution (x, y) = ({x}, {y})")
        }
        "diophantine" => {
            let equation = parse(graph, first)?;
            let (lhs, rhs) = match &graph.get(equation).kind {
                ExprKind::Relational { lhs, rhs, .. } => (*lhs, *rhs),
                _ => return Err("an equation `a x + b y = c` is needed".into()),
            };
            let difference = graph.simplify(graph.sub(lhs, rhs));
            let mut symbols = BTreeSet::new();
            collect_expr_symbols(graph, difference, &mut symbols);
            let mut named: Vec<(String, SymbolId)> = symbols
                .into_iter()
                .map(|s| (graph.symbols.resolve(s).unwrap_or_default(), s))
                .collect();
            named.sort();
            let [(xn, xs), (yn, ys)] = named.as_slice() else {
                return Err("exactly two unknowns are needed".into());
            };
            let coefficient = |s: SymbolId| -> Result<BigInt, String> {
                let d = graph.simplify(graph.diff(difference, s));
                let q = crate::numeric::try_eval_exact(graph, d, &HashMap::new())
                    .map_err(|e| e.to_string())?
                    .ok_or("the equation must be linear with integer coefficients")?;
                if q.is_integer() {
                    Ok(q.to_integer())
                } else {
                    Err("the coefficients must be integers".to_string())
                }
            };
            let (a, b) = (coefficient(*xs)?, coefficient(*ys)?);
            let zero = graph.integer(0);
            let constant = graph.substitute(graph.substitute(difference, *xs, zero), *ys, zero);
            let c = -crate::numeric::try_eval_exact(graph, constant, &HashMap::new())
                .map_err(|e| e.to_string())?
                .filter(|q| q.is_integer())
                .ok_or("the constant must be an integer")?
                .to_integer();
            crate::intlist::linear_diophantine(&a, &b, &c, xn, yn)?
        }
        other => return Err(format!("unknown command {other}")),
    };
    Ok(NumericOutput::Text(text))
}

/// The `p`-th quantile (`0 <= p <= 1`) of sorted data by linear interpolation between order statistics (the usual
/// "type 7" definition).
fn interpolate_sorted(sorted: &[Q], p: &Q) -> Q {
    let n = sorted.len();
    if n == 1 {
        return sorted[0].clone();
    }
    let position = p * Q::from_integer(((n - 1) as i64).into());
    let lower = position
        .floor()
        .to_integer()
        .to_usize()
        .unwrap_or(0)
        .min(n - 1);
    let upper = (lower + 1).min(n - 1);
    let fraction = &position - Q::from_integer((lower as i64).into());
    &sorted[lower] + (&sorted[upper] - &sorted[lower]) * fraction
}

/// `percentile [data], p` (`p` in 0..100) and `quantile [data], q` (`q` in 0..1).
fn percentile_command(
    graph: &ExprGraph,
    name: &str,
    args: &[String],
) -> Result<NumericOutput, String> {
    let args = split_args(args);
    let [data_text, level] = args.as_slice() else {
        return Err(format!("{name} [data], p"));
    };
    let mut sorted: Vec<Q> = list_items(data_text)
        .iter()
        .map(|i| exact(graph, i))
        .collect::<Result<_, _>>()?;
    sorted.sort();
    let level = exact(graph, level)?;
    let q = if name == "percentile" {
        level.clone() / Q::from_integer(100.into())
    } else {
        level.clone()
    };
    if q < Q::zero() || q > Q::one() {
        return Err(
            "the level must be between 0 and 100 for a percentile (0 and 1 for a quantile)".into(),
        );
    }
    let value = interpolate_sorted(&sorted, &q);
    let shown = if value.is_integer() {
        q_text(&value)
    } else {
        format!("{} ≈ {}", q_text(&value), decimal(&value))
    };
    Ok(NumericOutput::Text(format!(
        "{name} {} = {shown}",
        q_text(&level)
    )))
}

/// `next_term 1, 4, 9, 16` and `find_pattern`: the rule of a sequence and its next terms.
fn sequence_rule_command(graph: &ExprGraph, args: &[String]) -> Result<NumericOutput, String> {
    let items: Vec<String> = if args.len() == 1 {
        list_items(&args[0])
    } else {
        split_args(args)
    };
    let terms: Vec<Q> = items
        .iter()
        .map(|i| exact(graph, i))
        .collect::<Result<_, _>>()?;
    crate::sequence_rule::describe(graph, &terms, 5).map(NumericOutput::Text)
}

/// Run a numerical command by name.
pub fn run(graph: &ExprGraph, name: &str, args: &[String]) -> Result<NumericOutput, String> {
    match name {
        "gradient" | "hessian" | "jacobian" | "divergence" | "curl" | "laplacian" | "cross"
        | "dot" | "norm" => crate::vectorcalc::run(graph, name, args),
        "product" => product(graph, args),
        "next_term" | "find_pattern" | "nth_term" | "sequence_rule" | "guess_sequence" => {
            sequence_rule_command(graph, args)
        }
        "percentile" | "quantile" => percentile_command(graph, name, args),
        "dec2bin" | "bin2dec" | "dec2hex" | "hex" | "hex2dec" | "dec2oct" | "oct" | "oct2dec"
        | "subsets" | "powerset" | "power_set" | "permutations_list" | "cycle_type"
        | "next_permutation" | "pell" | "diophantine" => integer_text_commands(graph, name, args),
        "covariance" | "correlation" => paired(graph, name, args),
        "trigsimp" | "expand_trig" => trigonometric(graph, name, args),
        "matpow" => matrix_power(graph, args),
        "lu" => lu(graph, args),
        "ztransform" => ztransform(graph, args),
        "nsolve" | "findroot" => nsolve(graph, args),
        "nintegrate" | "nint" => nintegrate(graph, args),
        "minimize" => minimize(graph, args, false),
        "maximize" => minimize(graph, args, true),
        "interpolate" | "lagrange" => interpolate(graph, args),
        "fit" | "polyfit" => fit(graph, args),
        "mean" | "average" | "median" | "variance" | "var" | "pvariance" | "stddev" | "std"
        | "sd" | "pstddev" | "min" | "max" | "mode" | "range" | "quartiles" | "iqr"
        | "skewness" | "kurtosis" | "zscore" => statistics(graph, name, args),
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
        "product" | "prod" => "product",
        "next_term" | "nth_term" | "find_pattern" | "sequence_rule" | "guess_sequence" => {
            "next_term"
        }
        "percentile" => "percentile",
        "quantile" => "quantile",
        "quartiles" => "quartiles",
        "iqr" => "iqr",
        "skewness" => "skewness",
        "kurtosis" => "kurtosis",
        "zscore" => "zscore",
        "dec2bin" => "dec2bin",
        "bin2dec" => "bin2dec",
        "dec2hex" | "hex" => "dec2hex",
        "hex2dec" => "hex2dec",
        "dec2oct" | "oct" => "dec2oct",
        "oct2dec" => "oct2dec",
        "subsets" | "powerset" | "power_set" => "subsets",
        "permutations_list" | "all_permutations" => "permutations_list",
        "cycle_type" => "cycle_type",
        "next_permutation" => "next_permutation",
        "pell" => "pell",
        "diophantine" => "diophantine",
        "mode" => "mode",
        "range" => "range",
        "covariance" | "cov" => "covariance",
        "correlation" | "corr" => "correlation",
        "gradient" | "grad" => "gradient",
        "hessian" => "hessian",
        "jacobian" => "jacobian",
        "divergence" => "divergence",
        "curl" => "curl",
        "laplacian" => "laplacian",
        "cross" | "cross_product" => "cross",
        "dot" | "dot_product" => "dot",
        "norm" => "norm",
        "trigsimp" | "trig_simplify" => "trigsimp",
        "expand_trig" | "trigexpand" => "expand_trig",
        "matpow" | "matrix_power" | "mpow" => "matpow",
        "lu" | "lu_decomposition" => "lu",
        "ztransform" | "z_transform" | "ztrans" => "ztransform",
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
