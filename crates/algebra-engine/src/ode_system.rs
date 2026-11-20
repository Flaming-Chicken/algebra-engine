//! Linear systems of ordinary differential equations with constant coefficients:
//! `dsolve([x' = y, y' = -x])`, `dsolve([x' = 2*x + y, y' = x + 2*y], [x, y])`, with optional initial conditions
//! `x(0) = 1`.
//!
//! For `v' = A v` the solution is `v(t) = L^-1{(s I - A)^-1} v(0)`: the matrix exponential entries are inverse Laplace
//! transforms of rational functions, computed exactly. The general solution uses the constants `C1, C2, ...` as the
//! initial values.

use crate::session::UraeSession;
use algebra_core::parser::ExprParser;

fn split_top(text: &str) -> Vec<String> {
    let (mut depth, mut cur, mut out) = (0i32, String::new(), Vec::new());
    for c in text.chars() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
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

/// The entries of a displayed matrix `[ a, b ; c, d ]`.
fn matrix_entries(text: &str) -> Option<Vec<Vec<String>>> {
    let inner = text.trim().strip_prefix('[')?.strip_suffix(']')?;
    Some(inner.split(';').map(split_top).collect())
}

/// `Some(result)` when `input` is `dsolve([v' = ..., ...], ...)`.
pub fn run(session: &mut UraeSession, input: &str) -> Option<Result<String, String>> {
    let t = input.trim();
    let body = t.strip_prefix("dsolve(")?.strip_suffix(')')?;
    let args = split_top(body);
    let list = args.first()?.trim();
    let mut equations = split_top(list.strip_prefix('[')?.strip_suffix(']')?);
    // Initial conditions may sit inside the list: `x(0) = 1`.
    let mut inner_conditions: Vec<String> = Vec::new();
    equations.retain(|e| {
        let is_condition = e
            .split_once('=')
            .is_some_and(|(left, _)| left.trim().ends_with("(0)"));
        if is_condition {
            inner_conditions.push(e.clone());
        }
        !is_condition
    });
    if equations.len() < 2 {
        return None;
    }
    // Every equation is `v' = rhs`; the unknowns are the primed names, in order.
    let mut unknowns: Vec<String> = Vec::new();
    let mut right_sides: Vec<String> = Vec::new();
    for equation in &equations {
        let (left, right) = equation.split_once('=')?;
        let name = left.trim().strip_suffix('\'')?.trim();
        if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return None;
        }
        unknowns.push(name.to_string());
        right_sides.push(right.trim().to_string());
    }
    // Initial conditions `x(0) = 1` among the remaining arguments; a list of unknowns names the order.
    let mut initial: Vec<Option<String>> = vec![None; unknowns.len()];
    for extra in inner_conditions.iter().chain(&args[1..]) {
        if extra.trim_start().starts_with('[') {
            continue;
        }
        let (call, value) = extra.split_once('=')?;
        let call = call.trim();
        let name = call.split('(').next()?.trim();
        let at = call.split_once('(')?.1.strip_suffix(')')?.trim();
        if at != "0" {
            return Some(Err(
                "initial conditions are supported at t = 0 (x(0) = 1)".into()
            ));
        }
        let index = unknowns.iter().position(|u| u == name)?;
        initial[index] = Some(value.trim().to_string());
    }
    Some(solve(session, &unknowns, &right_sides, &initial))
}

fn solve(
    session: &mut UraeSession,
    unknowns: &[String],
    right_sides: &[String],
    initial: &[Option<String>],
) -> Result<String, String> {
    let n = unknowns.len();
    // The matrix A: a_ij = d(rhs_i) / d(v_j), which must be a number, and rhs_i = sum a_ij v_j exactly.
    let mut a: Vec<Vec<String>> = Vec::new();
    for rhs in right_sides {
        let mut row = Vec::new();
        let mut rebuilt = Vec::new();
        for v in unknowns {
            let derivative = session.eval(&format!("diff(({rhs}), {v})"))?;
            // A constant coefficient: the derivative must not mention any unknown or the variable t.
            let id = ExprParser::new(&session.graph)
                .parse(&derivative)
                .map_err(|e| e.to_string())?;
            let mut symbols = std::collections::BTreeSet::new();
            crate::poly::collect_expr_symbols(&session.graph, id, &mut symbols);
            if symbols.iter().any(|s| {
                !matches!(
                    session.graph.symbols.resolve(*s).as_deref(),
                    Some("pi" | "π" | "e" | "i")
                )
            }) {
                return Err(
                    "only linear systems with constant coefficients are supported (the coefficients must be numbers)".into(),
                );
            }
            rebuilt.push(format!("({derivative})*{v}"));
            row.push(derivative);
        }
        // Nothing may remain once the linear part is removed (no forcing terms).
        let remainder = session.eval(&format!("({rhs}) - ({})", rebuilt.join(" + ")))?;
        if remainder.trim() != "0" {
            return Err(
                "only homogeneous linear systems are supported: the right side has a term that is not linear in the unknowns"
                    .into(),
            );
        }
        a.push(row);
    }
    // (s I - A) and its inverse.
    let matrix: Vec<String> = (0..n)
        .map(|i| {
            let cells: Vec<String> = (0..n)
                .map(|j| {
                    if i == j {
                        format!("s - ({})", a[i][j])
                    } else {
                        format!("-({})", a[i][j])
                    }
                })
                .collect();
            format!("[{}]", cells.join(", "))
        })
        .collect();
    let resolvent = session.eval(&format!("inverse([{}])", matrix.join(", ")))?;
    let entries = matrix_entries(&resolvent).ok_or("could not form the resolvent matrix")?;
    if entries.len() != n || entries.iter().any(|r| r.len() != n) {
        return Err("could not form the resolvent matrix".into());
    }
    // The matrix exponential e^(A t): the inverse transform of every entry.
    let mut phi: Vec<Vec<String>> = Vec::new();
    for row in &entries {
        let mut out = Vec::new();
        for entry in row {
            out.push(if entry.trim() == "0" {
                "0".to_string()
            } else {
                crate::inverse_laplace::invert(session, entry, "s", "t")?
            });
        }
        phi.push(out);
    }
    // v_i(t) = sum_j phi_ij(t) c_j with c_j the initial value or the constant Cj.
    let constants: Vec<String> = (0..n)
        .map(|j| {
            initial[j]
                .clone()
                .map(|v| format!("({v})"))
                .unwrap_or_else(|| format!("C{}", j + 1))
        })
        .collect();
    let mut lines = Vec::new();
    for (i, name) in unknowns.iter().enumerate() {
        let terms: Vec<String> = (0..n)
            .map(|j| format!("({})*{}", phi[i][j], constants[j]))
            .collect();
        let value = session.eval(&terms.join(" + "))?;
        lines.push(format!("{name}(t) = {value}"));
    }
    Ok(lines.join(", "))
}
