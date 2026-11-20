//! `critical_points(f)` and `extrema(f)`: the stationary points of a one-variable function, found by solving
//! `f'(x) = 0` exactly and classified by the sign of `f''` there.

use crate::numeric::NumericalEval;
use crate::session::UraeSession;
use algebra_core::parser::ExprParser;
use std::collections::{BTreeSet, HashMap};

/// The argument text of `name(...)` when the whole input is that call.
fn call_argument<'a>(input: &'a str, names: &[&str]) -> Option<&'a str> {
    let open = input.find('(')?;
    let head = input[..open].trim().to_ascii_lowercase();
    if !names.contains(&head.as_str()) || !input.ends_with(')') {
        return None;
    }
    Some(input[open + 1..input.len() - 1].trim())
}

/// Roots in the text `Solutions for x: [a, b, c]` (top-level commas only).
fn roots_of(text: &str) -> Option<Vec<String>> {
    let open = text.find('[')?;
    let close = text.rfind(']')?;
    let inner = &text[open + 1..close];
    let (mut depth, mut current, mut out) = (0i32, String::new(), Vec::new());
    for c in inner.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            _ => {}
        }
        if c == ',' && depth == 0 {
            out.push(std::mem::take(&mut current));
        } else {
            current.push(c);
        }
    }
    out.push(current);
    Some(
        out.into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
    )
}

/// `Some(result text)` when `input` is a `critical_points` / `extrema` call.
pub fn run(session: &mut UraeSession, input: &str) -> Option<Result<String, String>> {
    let trimmed = input.trim();
    let classify = call_argument(trimmed, &["extrema", "extremum", "local_extrema"]).is_some();
    let body = call_argument(
        trimmed,
        &[
            "critical_points",
            "stationary_points",
            "critical_point",
            "extrema",
            "extremum",
            "local_extrema",
        ],
    )?;
    let (function, variable) = {
        let graph = &session.graph;
        let parser = ExprParser::new(graph);
        let (text, named) = match body.rsplit_once(',') {
            Some((f, v)) if v.trim().chars().all(|c| c.is_alphabetic() || c == '_') => {
                (f.trim().to_string(), Some(v.trim().to_string()))
            }
            _ => (body.to_string(), None),
        };
        let id = match parser.parse(&text) {
            Ok(id) => id,
            Err(e) => return Some(Err(e.to_string())),
        };
        let variable = match named {
            Some(v) => v,
            None => {
                let mut symbols = BTreeSet::new();
                crate::poly::collect_expr_symbols(graph, id, &mut symbols);
                let names: Vec<String> = symbols
                    .iter()
                    .filter_map(|s| graph.symbols.resolve(*s))
                    .filter(|n| n != "pi" && n != "e")
                    .collect();
                match names.as_slice() {
                    [] => "x".to_string(),
                    [one] => one.clone(),
                    _ => {
                        return Some(Err(
                            "several variables: name the one, `critical_points(f, x)`".into(),
                        ));
                    }
                }
            }
        };
        (text, variable)
    };
    let solved = match session.eval(&format!(
        "solve(diff({function}, {variable}) = 0, {variable})"
    )) {
        Ok(t) => t,
        Err(e) => return Some(Err(e)),
    };
    let Some(roots) = roots_of(&solved) else {
        return Some(Ok(format!("critical points: {solved}")));
    };
    if roots.is_empty() {
        return Some(Ok(format!("no critical points of {function}")));
    }
    if !classify {
        return Some(Ok(format!(
            "critical points ({variable}): {}",
            roots.join(", ")
        )));
    }
    let graph = &session.graph;
    let parser = ExprParser::new(graph);
    let f = match parser.parse(&function) {
        Ok(f) => f,
        Err(e) => return Some(Err(e.to_string())),
    };
    let at = |x: f64| -> f64 {
        let ctx =
            crate::numeric::EvalContext::with_bindings(50, HashMap::from([(variable.clone(), x)]));
        graph.evalf(f, &ctx).map(|v| v.to_f64()).unwrap_or(f64::NAN)
    };
    let mut lines = Vec::new();
    for root in &roots {
        let r = match parser
            .parse(root)
            .ok()
            .and_then(|id| {
                graph
                    .evalf(id, &crate::numeric::EvalContext::default())
                    .ok()
            })
            .map(|v| v.to_f64())
        {
            Some(r) if r.is_finite() => r,
            _ => {
                lines.push(format!("{variable} = {root}: not classified"));
                continue;
            }
        };
        let h = 1e-3 * (1.0 + r.abs());
        let second = (at(r + h) - 2.0 * at(r) + at(r - h)) / (h * h);
        let kind = if second > 1e-4 {
            "local minimum"
        } else if second < -1e-4 {
            "local maximum"
        } else {
            "inconclusive (second derivative ~ 0)"
        };
        lines.push(format!("{variable} = {root}: {kind}, f = {}", at(r)));
    }
    Some(Ok(lines.join("; ")))
}
