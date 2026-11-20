//! Constrained extrema by Lagrange multipliers: `maximize x*y subject to x + y = 10`.
//!
//! Used when the linear-programming solver declines (the objective or a constraint is not linear). Only equality
//! constraints are supported. The stationarity system `grad f = sum lambda_k grad g_k`, `g_k = 0` goes to the system
//! solver; each solution is then classified numerically by projecting small random perturbations back onto the
//! constraint set and comparing the objective there (a local maximum, a local minimum, or a saddle).

use crate::numeric::{EvalContext, NumericalEval};
use crate::session::UraeSession;
use crate::simplex_exact::{split_commas, split_subject, strip_word};
use algebra_core::ExprId;
use algebra_core::parser::ExprParser;
use std::collections::{BTreeSet, HashMap};

fn solve_dense(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    for col in 0..n {
        let pivot = (col..n).max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))?;
        if a[pivot][col].abs() < 1e-14 {
            return None;
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        for r in col + 1..n {
            let f = a[r][col] / a[col][col];
            for c in col..n {
                a[r][c] -= f * a[col][c];
            }
            b[r] -= f * b[col];
        }
    }
    let mut x = vec![0.0; n];
    for r in (0..n).rev() {
        let s: f64 = (r + 1..n).map(|c| a[r][c] * x[c]).sum();
        x[r] = (b[r] - s) / a[r][r];
    }
    Some(x)
}

/// A number as a small rational when it is one to about nine digits, otherwise a short decimal.
fn number_text(x: f64) -> String {
    for q in 1..=360i64 {
        let p = (x * q as f64).round();
        if (x - p / q as f64).abs() < 1e-7 {
            let p = p as i64;
            let (mut a, mut b) = (p.abs(), q);
            while b != 0 {
                (a, b) = (b, a % b);
            }
            let g = a.max(1);
            return if q / g == 1 {
                format!("{}", p / g)
            } else {
                format!("{}/{}", p / g, q / g)
            };
        }
    }
    let s = format!("{x:.6}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// `Some(result)` when `input` is a maximize / minimize request with equality constraints.
pub fn run(session: &mut UraeSession, input: &str) -> Option<Result<String, String>> {
    let t = input.trim();
    let (maximise, rest) = match strip_word(t, &["maximize", "maximise", "max"]) {
        Some(r) => (true, r),
        None => (false, strip_word(t, &["minimize", "minimise", "min"])?),
    };
    let rest = rest.trim_start_matches(':').trim();
    let (objective, constraints) = split_subject(rest)?;
    Some(solve(
        session,
        maximise,
        objective.trim(),
        &split_commas(constraints),
    ))
}

fn solve(
    session: &mut UraeSession,
    maximise: bool,
    objective: &str,
    constraints: &[String],
) -> Result<String, String> {
    if constraints.is_empty() {
        return Err("at least one constraint is needed".into());
    }
    let mut equalities: Vec<String> = Vec::new();
    for c in constraints {
        if c.contains('<') || c.contains('>') || c.contains('≤') || c.contains('≥') {
            return Err(
                "a nonlinear objective supports equality constraints only (inequalities need a linear program)"
                    .into(),
            );
        }
        let (l, r) = c
            .split_once('=')
            .ok_or_else(|| format!("{c} is not an equation"))?;
        equalities.push(format!("({}) - ({})", l.trim(), r.trim()));
    }
    // Variables: the symbols of the objective and the constraints.
    let (f, g_ids, names) = {
        let g = &session.graph;
        let parser = ExprParser::new(g);
        let f = parser.parse(objective).map_err(|e| e.to_string())?;
        let mut gs = Vec::new();
        for e in &equalities {
            gs.push(parser.parse(e).map_err(|e| e.to_string())?);
        }
        let mut symbols = BTreeSet::new();
        crate::poly::collect_expr_symbols(g, f, &mut symbols);
        for &id in &gs {
            crate::poly::collect_expr_symbols(g, id, &mut symbols);
        }
        let names: Vec<String> = symbols
            .iter()
            .filter_map(|s| g.symbols.resolve(*s))
            .filter(|n| !matches!(n.as_str(), "pi" | "e" | "i"))
            .collect();
        (f, gs, names)
    };
    let (n, m) = (names.len(), equalities.len());
    if n == 0 || m > n {
        return Err("the constraints do not leave a variable to optimise over".into());
    }
    let lambdas: Vec<String> = (1..=m).map(|k| format!("lam{k}")).collect();
    let mut system = Vec::new();
    for v in &names {
        let mut terms = vec![format!("diff({objective}, {v})")];
        for (k, e) in equalities.iter().enumerate() {
            terms.push(format!("- {}*diff({e}, {v})", lambdas[k]));
        }
        system.push(terms.join(" "));
    }
    system.extend(equalities.iter().cloned());
    let unknowns: Vec<String> = names.iter().chain(lambdas.iter()).cloned().collect();
    let solved = session.eval(&format!(
        "solve([{}], [{}])",
        system.join(", "),
        unknowns.join(", ")
    ))?;
    // Each solution is `{x = 5, y = 5, lam1 = 5}`.
    let mut points: Vec<Vec<f64>> = Vec::new();
    for block in solved.split('{').skip(1) {
        let body = block.split('}').next().unwrap_or("");
        let mut values: HashMap<String, f64> = HashMap::new();
        for piece in body.split(", ") {
            if let Some((name, value)) = piece.split_once(" = ") {
                let id = ExprParser::new(&session.graph)
                    .parse(value.trim())
                    .map_err(|e| e.to_string())?;
                let v = session
                    .graph
                    .evalf(id, &EvalContext::default())
                    .map_err(|e| e.to_string())?;
                let (re, im) = v.as_complex();
                if im.abs() < 1e-9 && re.is_finite() {
                    values.insert(name.trim().to_string(), re);
                }
            }
        }
        if names.iter().all(|n| values.contains_key(n)) {
            points.push(names.iter().map(|n| values[n]).collect());
        }
    }
    if points.is_empty() {
        return Err(format!("no real stationary points found ({solved})"));
    }
    let graph = &session.graph;
    let eval = |expr: ExprId, x: &[f64]| -> f64 {
        let bindings = names.iter().cloned().zip(x.iter().copied()).collect();
        graph
            .evalf(expr, &EvalContext::with_bindings(50, bindings))
            .map(|v| v.to_f64())
            .unwrap_or(f64::NAN)
    };
    // Gauss-Newton projection onto g = 0.
    let project = |start: &[f64]| -> Option<Vec<f64>> {
        let mut x = start.to_vec();
        for _ in 0..30 {
            let gv: Vec<f64> = g_ids.iter().map(|&g| eval(g, &x)).collect();
            if gv.iter().all(|v| v.abs() < 1e-13) {
                return Some(x);
            }
            let jac: Vec<Vec<f64>> = g_ids
                .iter()
                .map(|&g| {
                    (0..n)
                        .map(|j| {
                            let h = 1e-7 * (1.0 + x[j].abs());
                            let (mut p, mut q) = (x.clone(), x.clone());
                            p[j] += h;
                            q[j] -= h;
                            (eval(g, &p) - eval(g, &q)) / (2.0 * h)
                        })
                        .collect()
                })
                .collect();
            let gram: Vec<Vec<f64>> = (0..m)
                .map(|a| {
                    (0..m)
                        .map(|b| (0..n).map(|j| jac[a][j] * jac[b][j]).sum())
                        .collect()
                })
                .collect();
            let y = solve_dense(gram, gv)?;
            for j in 0..n {
                x[j] -= (0..m).map(|a| jac[a][j] * y[a]).sum::<f64>();
            }
        }
        None
    };
    let mut state = 0x2545_F491_4F6C_DD1Du64;
    let mut random = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state >> 11) as f64 / (1u64 << 53) as f64 - 0.5
    };
    let mut classified: Vec<(f64, Vec<f64>, &'static str)> = Vec::new();
    for raw in &points {
        // The solver's digits are limited: put the point exactly on the constraints first.
        let polished = project(raw).unwrap_or_else(|| raw.clone());
        let p = &polished;
        let f0 = eval(f, p);
        if !f0.is_finite() {
            continue;
        }
        let (mut above, mut below) = (false, false);
        for _ in 0..60 {
            let start: Vec<f64> = p
                .iter()
                .map(|v| v + 1e-3 * (1.0 + v.abs()) * random())
                .collect();
            if let Some(q) = project(&start) {
                let d = eval(f, &q) - f0;
                if d > 1e-12 * (1.0 + f0.abs()) {
                    above = true;
                } else if d < -1e-12 * (1.0 + f0.abs()) {
                    below = true;
                }
            }
        }
        let kind = match (above, below) {
            (false, true) => "local maximum",
            (true, false) => "local minimum",
            (false, false) => "isolated point",
            (true, true) => "saddle",
        };
        classified.push((f0, p.clone(), kind));
    }
    let wanted = if maximise {
        "local maximum"
    } else {
        "local minimum"
    };
    let mut candidates: Vec<&(f64, Vec<f64>, &str)> = classified
        .iter()
        .filter(|c| c.2 == wanted || c.2 == "isolated point")
        .collect();
    if candidates.is_empty() {
        let others: Vec<String> = classified
            .iter()
            .map(|c| format!("{} ({})", number_text(c.0), c.2))
            .collect();
        return Err(format!(
            "no {wanted} among the stationary points: {}",
            others.join(", ")
        ));
    }
    candidates.sort_by(|a, b| {
        if maximise {
            b.0.total_cmp(&a.0)
        } else {
            a.0.total_cmp(&b.0)
        }
    });
    let best = candidates[0].0;
    let at: Vec<String> = candidates
        .iter()
        .filter(|c| (c.0 - best).abs() < 1e-9 * (1.0 + best.abs()))
        .map(|c| {
            names
                .iter()
                .zip(&c.1)
                .map(|(n, v)| format!("{n} = {}", number_text(*v)))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .collect();
    Ok(format!(
        "{wanted} value = {} at {}",
        number_text(best),
        at.join("; ")
    ))
}
