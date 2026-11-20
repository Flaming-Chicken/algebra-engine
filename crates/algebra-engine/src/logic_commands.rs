//! Propositional-logic commands on a formula: `truth_table(p and q)`, `tautology(p or not p)`, `satisfiable(...)`,
//! `contradiction(...)`, `equivalent(F, G)` and `count_models(F)`.
//!
//! A formula is anything the SAT parser reads (`and`, `or`, `not`, `xor`, `->`, `<->`, `&`, `|`, `~`, `implies`, `iff`).

use crate::logic::SatSolver;
use std::collections::{BTreeMap, HashMap};

/// `name(args)` where the call spans the whole input: the lower-case name and the top-level arguments.
fn split_call(input: &str) -> Option<(String, Vec<String>)> {
    let t = input.trim();
    let open = t.find('(')?;
    let name = t[..open].trim().to_ascii_lowercase();
    if name.is_empty()
        || !name.chars().all(|c| c.is_alphanumeric() || c == '_')
        || !t.ends_with(')')
    {
        return None;
    }
    let mut depth = 0i32;
    for (i, c) in t[open..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 && open + i != t.len() - 1 {
                    return None;
                }
            }
            _ => {}
        }
    }
    let (mut args, mut current, mut depth) = (Vec::new(), String::new(), 0i32);
    for c in t[open + 1..t.len() - 1].chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
        if c == ',' && depth == 0 {
            args.push(std::mem::take(&mut current));
        } else {
            current.push(c);
        }
    }
    args.push(current);
    Some((
        name,
        args.into_iter().map(|a| a.trim().to_string()).collect(),
    ))
}

/// All assignments of `vars` as maps from variable id to truth value, in truth-table order (all true first).
fn assignments(vars: &[(u32, String)]) -> Vec<HashMap<u32, bool>> {
    let n = vars.len();
    (0..1usize << n)
        .map(|row| {
            vars.iter()
                .enumerate()
                .map(|(i, (id, _))| (*id, (row >> (n - 1 - i)) & 1 == 0))
                .collect()
        })
        .collect()
}

fn letter(b: bool) -> char {
    if b { 'T' } else { 'F' }
}

/// `Some(result)` when `input` is one of the logic commands.
pub fn run(input: &str) -> Option<Result<String, String>> {
    let (name, args) = split_call(input)?;
    let command = match name.as_str() {
        "truth_table" | "truthtable" => "table",
        "tautology" | "is_tautology" => "tautology",
        "satisfiable" | "is_satisfiable" | "sat" => "sat",
        "contradiction" | "is_contradiction" | "unsatisfiable" => "contradiction",
        "equivalent" | "is_equivalent" | "equivalence" => "equivalent",
        "count_models" | "model_count" | "models" => "count",
        _ => return None,
    };
    Some(evaluate(command, &args))
}

fn evaluate(command: &str, args: &[String]) -> Result<String, String> {
    let formula = if command == "equivalent" {
        let [a, b] = args else {
            return Err("equivalent(F, G) takes two formulas".into());
        };
        format!("({a}) <-> ({b})")
    } else {
        let [f] = args else {
            return Err(format!("{command} takes one formula"));
        };
        f.clone()
    };
    let (expr, vars) =
        SatSolver::parse_formula(&formula).map_err(|e| format!("not a formula: {e}"))?;
    if vars.len() > 12 {
        return Err("at most 12 propositional variables are supported".into());
    }
    let rows = assignments(&vars);
    let value = |env: &HashMap<u32, bool>| expr.eval_with(env);
    match command {
        "table" => {
            let header: Vec<&str> = vars.iter().map(|(_, n)| n.as_str()).collect();
            let mut lines = vec![format!("{} | {}", header.join(" "), args[0])];
            for env in &rows {
                let cells: Vec<String> = vars
                    .iter()
                    .map(|(id, n)| {
                        format!("{:>width$}", letter(env[id]), width = n.chars().count())
                    })
                    .collect();
                let v = value(env).ok_or("the formula has an undefined variable")?;
                lines.push(format!("{} | {}", cells.join(" "), letter(v)));
            }
            Ok(lines.join("\n"))
        }
        "count" => {
            let count = rows.iter().filter(|e| value(e) == Some(true)).count();
            Ok(format!("{count} of {} assignments satisfy it", rows.len()))
        }
        "tautology" | "equivalent" => {
            let holds = rows.iter().all(|e| value(e) == Some(true));
            Ok(format!("{holds}"))
        }
        "contradiction" => Ok(format!("{}", rows.iter().all(|e| value(e) == Some(false)))),
        _ => match SatSolver::solve_model(&expr) {
            None => Ok("unsatisfiable".into()),
            Some(model) => {
                let model: BTreeMap<u32, bool> = model;
                let shown: Vec<String> = vars
                    .iter()
                    .map(|(id, n)| format!("{n}={}", model.get(id).copied().unwrap_or(false)))
                    .collect();
                // A returned model must actually satisfy the formula.
                let env: HashMap<u32, bool> = vars
                    .iter()
                    .map(|(id, _)| (*id, model.get(id).copied().unwrap_or(false)))
                    .collect();
                if value(&env) != Some(true) {
                    return Err("internal error: the model does not satisfy the formula".into());
                }
                Ok(format!("satisfiable: {}", shown.join(", ")))
            }
        },
    }
}
