//! Exact linear programming over the rationals: `maximize 3x + 2y subject to x + y <= 4, x + 3y <= 6`.
//!
//! Two-phase simplex with Bland's rule on a dense tableau of rationals. Variables are non-negative. Constraints may be
//! `<=`, `>=` or `=`. The answer is exact: the optimal value and a vertex, or "infeasible" / "unbounded".

use algebra_core::parser::ExprParser;
use algebra_core::{ExprGraph, ExprId, ExprKind, RelOp, SymbolId};
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};
use std::collections::{BTreeSet, HashMap};

type Q = BigRational;

#[derive(Debug, PartialEq)]
pub enum Outcome {
    Optimal { value: Q, point: Vec<Q> },
    Infeasible,
    Unbounded,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Sense {
    LessEqual,
    GreaterEqual,
    Equal,
}

fn q_text(r: &Q) -> String {
    if r.is_integer() {
        r.numer().to_string()
    } else {
        format!("{}/{}", r.numer(), r.denom())
    }
}

/// Maximise `c . x` subject to `rows[i] . x (sense) rhs[i]` and `x >= 0`.
pub fn maximise(c: &[Q], rows: &[Vec<Q>], senses: &[Sense], rhs: &[Q]) -> Outcome {
    let n = c.len();
    let m = rows.len();
    // Normalise to non-negative right-hand sides.
    let mut a: Vec<Vec<Q>> = rows.to_vec();
    let mut b: Vec<Q> = rhs.to_vec();
    let mut kinds: Vec<Sense> = senses.to_vec();
    for i in 0..m {
        if b[i].is_negative() {
            for v in a[i].iter_mut() {
                *v = -v.clone();
            }
            b[i] = -b[i].clone();
            kinds[i] = match kinds[i] {
                Sense::LessEqual => Sense::GreaterEqual,
                Sense::GreaterEqual => Sense::LessEqual,
                Sense::Equal => Sense::Equal,
            };
        }
    }
    // Columns: x (n), slacks / surplus, artificials.
    let slack_count = kinds.iter().filter(|k| **k != Sense::Equal).count();
    let artificial_count = kinds.iter().filter(|k| **k != Sense::LessEqual).count();
    let columns = n + slack_count + artificial_count;
    let mut tableau: Vec<Vec<Q>> = vec![vec![Q::zero(); columns + 1]; m];
    let mut basis: Vec<usize> = vec![0; m];
    let (mut slack_at, mut artificial_at) = (n, n + slack_count);
    let mut artificials: Vec<usize> = Vec::new();
    for i in 0..m {
        for j in 0..n {
            tableau[i][j] = a[i][j].clone();
        }
        tableau[i][columns] = b[i].clone();
        match kinds[i] {
            Sense::LessEqual => {
                tableau[i][slack_at] = Q::one();
                basis[i] = slack_at;
                slack_at += 1;
            }
            Sense::GreaterEqual => {
                tableau[i][slack_at] = -Q::one();
                slack_at += 1;
                tableau[i][artificial_at] = Q::one();
                basis[i] = artificial_at;
                artificials.push(artificial_at);
                artificial_at += 1;
            }
            Sense::Equal => {
                tableau[i][artificial_at] = Q::one();
                basis[i] = artificial_at;
                artificials.push(artificial_at);
                artificial_at += 1;
            }
        }
    }

    // One simplex run on `cost` (maximise); `allowed` marks the columns that may enter.
    fn run(tableau: &mut [Vec<Q>], basis: &mut [usize], cost: &[Q], allowed: &[bool]) -> bool {
        let m = tableau.len();
        let columns = cost.len();
        loop {
            // Reduced costs: cost_j - sum_i cost_basis_i * tableau_ij.
            let mut entering = None;
            for j in 0..columns {
                if !allowed[j] || basis.contains(&j) {
                    continue;
                }
                let mut reduced = cost[j].clone();
                for i in 0..m {
                    reduced -= &cost[basis[i]] * &tableau[i][j];
                }
                if reduced.is_positive() {
                    entering = Some(j);
                    break; // Bland: the lowest index.
                }
            }
            let Some(j) = entering else {
                return true;
            };
            // Ratio test with Bland's tie-break (lowest basis index).
            let mut leaving: Option<(usize, Q)> = None;
            for i in 0..m {
                if tableau[i][j].is_positive() {
                    let ratio = &tableau[i][columns] / &tableau[i][j];
                    let better = match &leaving {
                        None => true,
                        Some((li, lr)) => ratio < *lr || (ratio == *lr && basis[i] < basis[*li]),
                    };
                    if better {
                        leaving = Some((i, ratio));
                    }
                }
            }
            let Some((row, _)) = leaving else {
                return false; // unbounded
            };
            let pivot = tableau[row][j].clone();
            for v in tableau[row].iter_mut() {
                *v = &*v / &pivot;
            }
            let pivot_row = tableau[row].clone();
            for i in 0..m {
                if i != row && !tableau[i][j].is_zero() {
                    let factor = tableau[i][j].clone();
                    for (v, p) in tableau[i].iter_mut().zip(&pivot_row) {
                        *v -= &factor * p;
                    }
                }
            }
            basis[row] = j;
        }
    }

    // Phase 1: minimise the sum of the artificials (maximise its negative).
    if !artificials.is_empty() {
        let mut cost = vec![Q::zero(); columns];
        for &j in &artificials {
            cost[j] = -Q::one();
        }
        let allowed = vec![true; columns];
        run(&mut tableau, &mut basis, &cost, &allowed);
        let infeasibility: Q = (0..m)
            .filter(|&i| artificials.contains(&basis[i]))
            .fold(Q::zero(), |s, i| s + &tableau[i][columns]);
        if !infeasibility.is_zero() {
            return Outcome::Infeasible;
        }
        // Drive degenerate artificials out of the basis when possible.
        for i in 0..m {
            if artificials.contains(&basis[i]) {
                if let Some(j) = (0..n + slack_count).find(|&j| !tableau[i][j].is_zero()) {
                    let pivot = tableau[i][j].clone();
                    for v in tableau[i].iter_mut() {
                        *v = &*v / &pivot;
                    }
                    let pivot_row = tableau[i].clone();
                    for r in 0..m {
                        if r != i && !tableau[r][j].is_zero() {
                            let factor = tableau[r][j].clone();
                            for (v, p) in tableau[r].iter_mut().zip(&pivot_row) {
                                *v -= &factor * p;
                            }
                        }
                    }
                    basis[i] = j;
                }
            }
        }
    }
    // Phase 2.
    let mut cost = vec![Q::zero(); columns];
    for (j, cj) in c.iter().enumerate() {
        cost[j] = cj.clone();
    }
    let mut allowed = vec![true; columns];
    for &j in &artificials {
        allowed[j] = false;
    }
    if !run(&mut tableau, &mut basis, &cost, &allowed) {
        return Outcome::Unbounded;
    }
    let mut point = vec![Q::zero(); n];
    for i in 0..m {
        if basis[i] < n {
            point[basis[i]] = tableau[i][columns].clone();
        }
    }
    let value = c
        .iter()
        .zip(&point)
        .fold(Q::zero(), |s, (ci, xi)| s + ci * xi);
    Outcome::Optimal { value, point }
}

fn exact(graph: &ExprGraph, id: ExprId) -> Option<Q> {
    crate::numeric::try_eval_exact(graph, graph.simplify_expr(id), &HashMap::new())
        .ok()
        .flatten()
}

trait SimplifyExt {
    fn simplify_expr(&self, id: ExprId) -> ExprId;
}

impl SimplifyExt for ExprGraph {
    fn simplify_expr(&self, id: ExprId) -> ExprId {
        use crate::simplify::SymbolicSimplifier as _;
        self.simplify(id)
    }
}

/// A linear form `sum a_i x_i + constant`.
fn linear_form(graph: &ExprGraph, expr: ExprId, vars: &[SymbolId]) -> Result<(Vec<Q>, Q), String> {
    use crate::calculus::SymbolicCalculus as _;
    let mut coefficients = Vec::new();
    for &v in vars {
        let d = graph.diff(expr, v);
        coefficients.push(
            exact(graph, d).ok_or("the expression must be linear with rational coefficients")?,
        );
    }
    let mut constant = expr;
    for &v in vars {
        constant = graph.substitute(constant, v, graph.integer(0));
    }
    let constant =
        exact(graph, constant).ok_or("the expression must be linear with rational coefficients")?;
    // Linearity check at a second point.
    let mut at_one = expr;
    for &v in vars {
        at_one = graph.substitute(at_one, v, graph.integer(1));
    }
    let expected = coefficients.iter().fold(constant.clone(), |s, c| s + c);
    if exact(graph, at_one) != Some(expected) {
        return Err("the expression must be linear in the variables".into());
    }
    Ok((coefficients, constant))
}

/// Split `a, b, c` at top-level commas.
pub(crate) fn split_commas(text: &str) -> Vec<String> {
    let (mut depth, mut current, mut out) = (0i32, String::new(), Vec::new());
    for c in text.chars() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            _ => {}
        }
        if c == ',' && depth == 0 {
            out.push(std::mem::take(&mut current));
        } else {
            current.push(c);
        }
    }
    out.push(current);
    out.into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// `maximize 3x + 2y subject to x + y <= 4, x + 3y <= 6` (also `minimize`, `max`, `min`, `s.t.`, `such that`).
pub fn solve_text(graph: &ExprGraph, input: &str) -> Option<Result<String, String>> {
    let lowered = input.trim();
    let (maximise_goal, rest) = match strip_word(lowered, &["maximize", "maximise", "max"]) {
        Some(r) => (true, r),
        None => (
            false,
            strip_word(lowered, &["minimize", "minimise", "min"])?,
        ),
    };
    let rest = rest.trim_start_matches(':').trim();
    let (objective_text, constraints_text) = split_subject(rest)?;
    Some(solve(
        graph,
        maximise_goal,
        objective_text,
        &split_commas(constraints_text),
    ))
}

pub(crate) fn strip_word<'a>(input: &'a str, words: &[&str]) -> Option<&'a str> {
    for w in words {
        if input.len() > w.len()
            && input
                .get(..w.len())
                .is_some_and(|h| h.eq_ignore_ascii_case(w))
            && input[w.len()..].starts_with([' ', ':'])
        {
            return Some(&input[w.len()..]);
        }
    }
    None
}

pub(crate) fn split_subject(text: &str) -> Option<(&str, &str)> {
    let lower = text.to_ascii_lowercase();
    for marker in [" subject to ", " s.t. ", " st ", " such that ", " where "] {
        if let Some(at) = lower.find(marker) {
            return Some((&text[..at], &text[at + marker.len()..]));
        }
    }
    None
}

pub fn solve(
    graph: &ExprGraph,
    maximise_goal: bool,
    objective: &str,
    constraints: &[String],
) -> Result<String, String> {
    let parser = ExprParser::new(graph);
    let f = parser.parse(objective.trim()).map_err(|e| e.to_string())?;
    let mut parsed = Vec::new();
    for text in constraints {
        let id = parser.parse(text).map_err(|e| e.to_string())?;
        let ExprKind::Relational { op, lhs, rhs } = graph.get(id).kind.clone() else {
            return Err(format!("`{text}` is not a constraint such as `x + y <= 4`"));
        };
        parsed.push((op, lhs, rhs));
    }
    // Variables: all symbols, alphabetically.
    let mut symbols = BTreeSet::new();
    crate::poly::collect_expr_symbols(graph, f, &mut symbols);
    for (_, l, r) in &parsed {
        crate::poly::collect_expr_symbols(graph, *l, &mut symbols);
        crate::poly::collect_expr_symbols(graph, *r, &mut symbols);
    }
    let mut named: Vec<(String, SymbolId)> = symbols
        .into_iter()
        .map(|s| (graph.symbols.resolve(s).unwrap_or_default(), s))
        .collect();
    named.sort();
    let vars: Vec<SymbolId> = named.iter().map(|(_, s)| *s).collect();
    let names: Vec<String> = named.iter().map(|(n, _)| n.clone()).collect();
    let (objective_coeffs, objective_constant) = linear_form(graph, f, &vars)?;
    let mut rows = Vec::new();
    let mut senses = Vec::new();
    let mut rhs = Vec::new();
    for (op, l, r) in parsed {
        let (coeffs, constant) = linear_form(graph, graph.sub(l, r), &vars)?;
        // coeffs . x + constant (op) 0  =>  coeffs . x (op) -constant
        rows.push(coeffs);
        rhs.push(-constant);
        senses.push(match op {
            RelOp::LessEqual | RelOp::LessThan => Sense::LessEqual,
            RelOp::GreaterEqual | RelOp::GreaterThan => Sense::GreaterEqual,
            RelOp::Equal => Sense::Equal,
            RelOp::NotEqual => return Err("`!=` is not a linear constraint".into()),
        });
    }
    let signed: Vec<Q> = objective_coeffs
        .iter()
        .map(|c| if maximise_goal { c.clone() } else { -c.clone() })
        .collect();
    Ok(match maximise(&signed, &rows, &senses, &rhs) {
        Outcome::Infeasible => {
            "infeasible: no point satisfies all the constraints (with all variables >= 0)"
                .to_string()
        }
        Outcome::Unbounded => format!(
            "unbounded: the objective can be made arbitrarily {}",
            if maximise_goal { "large" } else { "small" }
        ),
        Outcome::Optimal { value, point } => {
            let optimum = if maximise_goal { value } else { -value } + objective_constant;
            let at: Vec<String> = names
                .iter()
                .zip(&point)
                .map(|(n, v)| format!("{n} = {}", q_text(v)))
                .collect();
            format!(
                "{} value = {} at {}",
                if maximise_goal { "maximum" } else { "minimum" },
                q_text(&optimum),
                at.join(", ")
            )
        }
    })
}
