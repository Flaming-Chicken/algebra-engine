//! Fixed-step numerical solution of `y' = f(x, y)`: `rk4`, `euler_method` and `midpoint_method` (Heun is `heun`).
//!
//! Two spellings: `rk4(f, x0, y0, x_end, n)` and `rk4(y' = f, y(x0) = y0, x_end, n)`. The answer is `y(x_end)` after `n`
//! equal steps.

use crate::numeric::{EvalContext, NumericalEval};
use crate::session::UraeSession;
use algebra_core::parser::ExprParser;
use std::collections::{BTreeSet, HashMap};

fn split_top(text: &str) -> Vec<String> {
    let (mut depth, mut cur, mut out) = (0i32, String::new(), Vec::new());
    for c in text.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            _ => {}
        }
        if c == ',' && depth == 0 {
            out.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    out.push(cur);
    out.into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn constant(session: &UraeSession, text: &str) -> Result<f64, String> {
    let id = ExprParser::new(&session.graph)
        .parse(text)
        .map_err(|e| e.to_string())?;
    let v = session
        .graph
        .evalf(id, &EvalContext::default())
        .map_err(|e| format!("{text} is not a number: {e}"))?
        .to_f64();
    if v.is_finite() {
        Ok(v)
    } else {
        Err(format!("{text} is not a finite number"))
    }
}

/// `Some(result)` when `input` is a fixed-step ODE method call.
pub fn run(session: &mut UraeSession, input: &str) -> Option<Result<String, String>> {
    let t = input.trim();
    let open = t.find('(')?;
    let name = t[..open].trim().to_ascii_lowercase();
    let method = match name.as_str() {
        "rk4" | "runge_kutta" | "rungekutta" => "rk4",
        "euler_method" | "euler" | "forward_euler" => "euler",
        "heun" | "heun_method" | "rk2" => "heun",
        "midpoint_method" => "midpoint",
        _ => return None,
    };
    if !t.ends_with(')') {
        return None;
    }
    Some(solve(
        session,
        method,
        &split_top(&t[open + 1..t.len() - 1]),
    ))
}

fn solve(session: &mut UraeSession, method: &str, args: &[String]) -> Result<String, String> {
    // Normalise to (f text, x0, y0, x_end, n, dependent name).
    let usage = format!("{method}(f, x0, y0, x_end, n) or {method}(y' = f, y(x0) = y0, x_end, n)");
    let (f_text, x0, y0, x_end, n, dependent) = if let Some(rest) = args.first().and_then(|a| {
        let (lhs, rhs) = a.split_once('=')?;
        let lhs = lhs.trim();
        let name = lhs.strip_suffix('\'')?.trim();
        (!name.is_empty()).then(|| (name.to_string(), rhs.trim().to_string()))
    }) {
        let (dep, f_text) = rest;
        if args.len() != 4 {
            return Err(usage);
        }
        let (call, value) = args[1].split_once('=').ok_or(usage.clone())?;
        let point = call
            .trim()
            .strip_prefix(dep.as_str())
            .and_then(|r| r.trim().strip_prefix('('))
            .and_then(|r| r.strip_suffix(')'))
            .ok_or(usage.clone())?;
        (
            f_text,
            constant(session, point)?,
            constant(session, value)?,
            constant(session, &args[2])?,
            constant(session, &args[3])?,
            Some(dep),
        )
    } else {
        if args.len() != 5 {
            return Err(usage);
        }
        (
            args[0].clone(),
            constant(session, &args[1])?,
            constant(session, &args[2])?,
            constant(session, &args[3])?,
            constant(session, &args[4])?,
            None,
        )
    };
    if n < 1.0 || n.fract() != 0.0 || n > 1e7 {
        return Err("the number of steps must be a positive integer".into());
    }
    let n = n as usize;
    let g = &session.graph;
    let f = ExprParser::new(g)
        .parse(&f_text)
        .map_err(|e| e.to_string())?;
    let mut symbols = BTreeSet::new();
    crate::poly::collect_expr_symbols(g, f, &mut symbols);
    let names: Vec<String> = symbols
        .iter()
        .filter_map(|s| g.symbols.resolve(*s))
        .filter(|s| !matches!(s.as_str(), "pi" | "e" | "i"))
        .collect();
    let dep = match dependent {
        Some(d) => d,
        None if names.iter().any(|s| s == "y") => "y".to_string(),
        None => names
            .iter()
            .find(|s| s.as_str() != "x" && s.as_str() != "t")
            .cloned()
            .unwrap_or_else(|| "y".to_string()),
    };
    let ind = names
        .iter()
        .find(|s| **s != dep && (s.as_str() == "x" || s.as_str() == "t"))
        .cloned()
        .or_else(|| names.iter().find(|s| **s != dep).cloned())
        .unwrap_or_else(|| "x".to_string());
    if let Some(extra) = names.iter().find(|s| **s != dep && **s != ind) {
        return Err(format!("the right-hand side has an unknown symbol {extra}"));
    }
    let rhs = |x: f64, y: f64| -> f64 {
        let ctx =
            EvalContext::with_bindings(50, HashMap::from([(ind.clone(), x), (dep.clone(), y)]));
        g.evalf(f, &ctx).map(|v| v.to_f64()).unwrap_or(f64::NAN)
    };
    let h = (x_end - x0) / n as f64;
    let (mut x, mut y) = (x0, y0);
    for k in 0..n {
        y += match method {
            "euler" => h * rhs(x, y),
            "heun" => {
                let k1 = rhs(x, y);
                let k2 = rhs(x + h, y + h * k1);
                h * (k1 + k2) / 2.0
            }
            "midpoint" => {
                let k1 = rhs(x, y);
                h * rhs(x + h / 2.0, y + h * k1 / 2.0)
            }
            _ => {
                let k1 = rhs(x, y);
                let k2 = rhs(x + h / 2.0, y + h * k1 / 2.0);
                let k3 = rhs(x + h / 2.0, y + h * k2 / 2.0);
                let k4 = rhs(x + h, y + h * k3);
                h * (k1 + 2.0 * k2 + 2.0 * k3 + k4) / 6.0
            }
        };
        x = x0 + h * (k + 1) as f64;
        if !y.is_finite() {
            return Err(format!("the solution is not finite at {ind} = {x}"));
        }
    }
    let label = match method {
        "euler" => "Euler",
        "heun" => "Heun",
        "midpoint" => "midpoint",
        _ => "RK4",
    };
    Ok(format!("{dep}({x_end}) ≈ {y}  ({label}, {n} steps)"))
}
