//! Initial conditions for a general solution: fix the constants `C1, C2, ...` of `y(x) = ...` from `y(x0) = v`,
//! `y'(x0) = v`, ... .
//!
//! The conditions are equations in the constants. When they are linear (every linear ODE) they are solved exactly by
//! Gaussian elimination over the rationals; a single remaining nonlinear equation in one constant goes to the
//! equation solver.

use crate::calculus::SymbolicCalculus;
use crate::numeric::try_eval_exact;
use crate::simplify::SymbolicSimplifier;
use crate::solver::SymbolicSolver;
use algebra_core::parser::ExprParser;
use algebra_core::{ExprGraph, ExprId, ExprKind, SymbolId};
use num_rational::BigRational;
use num_traits::Zero;
use std::collections::{BTreeSet, HashMap};

type Q = BigRational;

/// One initial condition: the derivative order, the point and the value, as text.
#[derive(Debug, Clone)]
pub struct Condition {
    pub name: String,
    pub order: usize,
    pub point: String,
    pub value: String,
}

/// Parse `y(0) = 1`, `y'(0) = 2`, `y''(1) = -3`; `None` if the text is not an initial condition.
pub fn parse_condition(text: &str) -> Option<Condition> {
    let (lhs, rhs) = text.split_once('=')?;
    let lhs = lhs.trim();
    let open = lhs.find('(')?;
    if !lhs.ends_with(')') {
        return None;
    }
    let name = &lhs[..open];
    let primes = name.chars().rev().take_while(|&c| c == '\'').count();
    let base = &name[..name.len() - primes];
    if base.is_empty() || !base.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    let point = lhs[open + 1..lhs.len() - 1].trim();
    if point.is_empty() || rhs.trim().is_empty() {
        return None;
    }
    Some(Condition {
        name: base.to_string(),
        order: primes,
        point: point.to_string(),
        value: rhs.trim().to_string(),
    })
}

fn constants(
    graph: &ExprGraph,
    expr: ExprId,
    out: &mut BTreeSet<String>,
    others: &mut BTreeSet<String>,
) {
    match graph.get(expr).kind.clone() {
        ExprKind::Symbol(s) => {
            let name = graph.symbols.resolve(s).unwrap_or_default();
            let is_constant = name.len() > 1
                && name.starts_with('C')
                && name[1..].chars().all(|c| c.is_ascii_digit());
            if is_constant {
                out.insert(name);
            } else {
                others.insert(name);
            }
        }
        ExprKind::Add(ts) | ExprKind::Mul(ts) => {
            ts.iter().for_each(|&t| constants(graph, t, out, others))
        }
        ExprKind::Sub(a, b) | ExprKind::Div(a, b) | ExprKind::Pow(a, b) => {
            constants(graph, a, out, others);
            constants(graph, b, out, others);
        }
        ExprKind::Neg(a) => constants(graph, a, out, others),
        ExprKind::Function { args, .. } => {
            args.iter().for_each(|&a| constants(graph, a, out, others))
        }
        _ => {}
    }
}

fn exact(graph: &ExprGraph, id: ExprId) -> Option<Q> {
    let simplified = graph.simplify(id);
    try_eval_exact(graph, simplified, &HashMap::new())
        .ok()
        .flatten()
}

/// Solve a square linear system over the rationals; `None` if it is singular.
fn solve_linear(mut rows: Vec<Vec<Q>>, n: usize) -> Option<Vec<Q>> {
    for col in 0..n {
        let pivot = (col..n).find(|&r| !rows[r][col].is_zero())?;
        rows.swap(col, pivot);
        let p = rows[col][col].clone();
        for c in col..=n {
            rows[col][c] = &rows[col][c] / &p;
        }
        for r in 0..n {
            if r != col && !rows[r][col].is_zero() {
                let f = rows[r][col].clone();
                for c in col..=n {
                    let delta = &f * &rows[col][c];
                    rows[r][c] -= delta;
                }
            }
        }
    }
    Some((0..n).map(|r| rows[r][n].clone()).collect())
}

/// Substitute the initial conditions into the general solution `rhs` (a function of one variable and `C1..Cn`).
pub fn apply(graph: &ExprGraph, rhs: ExprId, conditions: &[Condition]) -> Result<ExprId, String> {
    let (mut cs, mut others) = (BTreeSet::new(), BTreeSet::new());
    constants(graph, rhs, &mut cs, &mut others);
    if cs.is_empty() {
        return Err("the solution has no arbitrary constants to fix".into());
    }
    if conditions.len() < cs.len() {
        return Err(format!(
            "{} initial condition(s) given but the solution has {} constants",
            conditions.len(),
            cs.len()
        ));
    }
    let var_name = if others.contains("x") {
        "x".to_string()
    } else {
        others
            .iter()
            .next()
            .cloned()
            .ok_or("the solution has no independent variable")?
    };
    let var: SymbolId = graph.symbols.get_or_intern(&var_name);
    let names: Vec<String> = cs.iter().cloned().collect();
    let syms: Vec<SymbolId> = names
        .iter()
        .map(|n| graph.symbols.get_or_intern(n))
        .collect();
    let parser = ExprParser::new(graph);

    // Each condition as an expression that must vanish.
    let mut equations = Vec::new();
    for condition in conditions {
        let mut derivative = rhs;
        for _ in 0..condition.order {
            derivative = graph.simplify(graph.diff(derivative, var));
        }
        let point = parser.parse(&condition.point).map_err(|e| e.to_string())?;
        let value = parser.parse(&condition.value).map_err(|e| e.to_string())?;
        let at_point = graph.substitute(derivative, var, point);
        equations.push(graph.sub(at_point, value));
    }

    // Linear in the constants?
    let n = names.len();
    let zero = graph.integer(0);
    let mut matrix: Vec<Vec<Q>> = Vec::new();
    let mut linear = true;
    for &equation in &equations {
        let mut constant = equation;
        for &s in &syms {
            constant = graph.substitute(constant, s, zero);
        }
        let mut row = Vec::with_capacity(n + 1);
        for &s in &syms {
            match exact(graph, graph.diff(equation, s)) {
                Some(c) => row.push(c),
                None => {
                    linear = false;
                    break;
                }
            }
        }
        match (linear, exact(graph, constant)) {
            (true, Some(c)) => {
                row.push(-c);
                matrix.push(row);
            }
            _ => {
                linear = false;
            }
        }
        if !linear {
            break;
        }
    }
    let mut result = rhs;
    if linear {
        let square: Vec<Vec<Q>> = matrix.into_iter().take(n).collect();
        let solution = solve_linear(square, n)
            .ok_or("the initial conditions do not determine the constants (singular system)")?;
        for (s, value) in syms.iter().zip(&solution) {
            result = graph.substitute(result, *s, crate::poly::expr_from_rational(graph, value));
        }
    } else if n == 1 {
        // Clear denominators first: 1/C1 + 1 = 0 becomes 1 + C1 = 0.
        let together = crate::simplify::AlgebraicTransformations::together(graph, equations[0]);
        let numerator = match graph.get(together).kind.clone() {
            ExprKind::Div(n, _) => n,
            _ => together,
        };
        let solutions = graph
            .solveset(numerator, syms[0])
            .map_err(|e| e.to_string())?;
        let first = solutions
            .first()
            .ok_or("no value of the constant satisfies the initial condition")?;
        result = graph.substitute(result, syms[0], *first);
    } else {
        return Err("the initial conditions are nonlinear in the constants".into());
    }
    Ok(graph.simplify(result))
}
