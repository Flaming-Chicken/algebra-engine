//! The assignment problem (minimum or maximum weight perfect matching of a bipartite graph) on a rational cost matrix:
//! `assignment([[4,1,3],[2,0,5],[3,2,2]])` and `max_assignment(...)`, by the Hungarian method with potentials in exact arithmetic.
//! A matrix with more columns than rows leaves the surplus columns unassigned.

use num_rational::BigRational;
use num_traits::Zero;

type Q = BigRational;

fn show(q: &Q) -> String {
    if q.is_integer() {
        q.numer().to_string()
    } else {
        format!("{}/{}", q.numer(), q.denom())
    }
}

fn parse_matrix(text: &str, wide_only: bool) -> Result<Vec<Vec<Q>>, String> {
    let inner = text
        .trim()
        .strip_prefix('[')
        .and_then(|r| r.strip_suffix(']'))
        .ok_or("the costs must be a matrix [[...], ...]")?;
    let rows = crate::graph_commands::split_top(inner);
    let matrix: Vec<Vec<Q>> = rows
        .iter()
        .map(|row| {
            let body = row.trim().trim_start_matches('[').trim_end_matches(']');
            body.split(',')
                .map(|cell| {
                    let t = cell.trim();
                    match t.split_once('/') {
                        Some((a, b)) => {
                            let (a, b) = (
                                a.trim().parse::<num_bigint::BigInt>().ok()?,
                                b.trim().parse::<num_bigint::BigInt>().ok()?,
                            );
                            (!b.is_zero()).then(|| Q::new(a, b))
                        }
                        None => t.parse::<num_bigint::BigInt>().ok().map(Q::from_integer),
                    }
                })
                .collect::<Option<Vec<Q>>>()
                .ok_or_else(|| format!("{row} has a cell that is not a rational number"))
        })
        .collect::<Result<_, _>>()?;
    if matrix.is_empty() || matrix.iter().any(|r| r.len() != matrix[0].len()) {
        return Err("the cost matrix must be rectangular".into());
    }
    if wide_only && matrix.len() > matrix[0].len() {
        return Err("there are more rows than columns: transpose the matrix".into());
    }
    Ok(matrix)
}

/// Hungarian algorithm for `n` rows and `m >= n` columns; returns the column chosen for every row.
fn hungarian(cost: &[Vec<Q>]) -> Vec<usize> {
    let (n, m) = (cost.len(), cost[0].len());
    let mut u = vec![Q::zero(); n + 1];
    let mut v = vec![Q::zero(); m + 1];
    let mut matched_row = vec![0usize; m + 1]; // matched_row[j] = row (1-based) assigned to column j (1-based)
    let mut way = vec![0usize; m + 1];
    for i in 1..=n {
        matched_row[0] = i;
        let mut j0 = 0;
        let mut minv: Vec<Option<Q>> = vec![None; m + 1];
        let mut used = vec![false; m + 1];
        loop {
            used[j0] = true;
            let (i0, mut delta, mut j1): (usize, Option<Q>, usize) = (matched_row[j0], None, 0);
            for j in 1..=m {
                if !used[j] {
                    let current = &cost[i0 - 1][j - 1] - &u[i0] - &v[j];
                    if minv[j].as_ref().is_none_or(|x| current < *x) {
                        minv[j] = Some(current);
                        way[j] = j0;
                    }
                    if let Some(x) = &minv[j]
                        && delta.as_ref().is_none_or(|d| x < d)
                    {
                        delta = Some(x.clone());
                        j1 = j;
                    }
                }
            }
            let delta = delta.unwrap_or_else(Q::zero);
            for j in 0..=m {
                if used[j] {
                    let row = matched_row[j];
                    u[row] += &delta;
                    v[j] -= &delta;
                } else if let Some(x) = minv[j].as_mut() {
                    *x -= &delta;
                }
            }
            j0 = j1;
            if matched_row[j0] == 0 {
                break;
            }
        }
        loop {
            let j1 = way[j0];
            matched_row[j0] = matched_row[j1];
            j0 = j1;
            if j0 == 0 {
                break;
            }
        }
    }
    let mut chosen = vec![0usize; n];
    for j in 1..=m {
        if matched_row[j] != 0 {
            chosen[matched_row[j] - 1] = j - 1;
        }
    }
    chosen
}

pub fn run(input: &str) -> Option<Result<String, String>> {
    let t = input.trim();
    let open = t.find('(')?;
    let name = t[..open].trim().to_ascii_lowercase();
    if !t.ends_with(')') {
        return None;
    }
    if matches!(
        name.as_str(),
        "min_cost_flow" | "mincostflow" | "minimum_cost_flow"
    ) {
        return Some(min_cost_flow(&t[open + 1..t.len() - 1]));
    }
    if matches!(
        name.as_str(),
        "transportation" | "transport" | "transportation_problem"
    ) {
        return Some(transportation(&t[open + 1..t.len() - 1]));
    }
    let maximise = match name.as_str() {
        "assignment" | "min_assignment" | "hungarian" | "linear_assignment" => false,
        "max_assignment" | "maximum_assignment" => true,
        _ => return None,
    };
    let mut cost = match parse_matrix(&t[open + 1..t.len() - 1], true) {
        Ok(m) => m,
        Err(e) => return Some(Err(e)),
    };
    let original = cost.clone();
    if maximise {
        for row in cost.iter_mut() {
            for cell in row.iter_mut() {
                *cell = -cell.clone();
            }
        }
    }
    let chosen = hungarian(&cost);
    let total = chosen
        .iter()
        .enumerate()
        .fold(Q::zero(), |acc, (i, &j)| acc + &original[i][j]);
    let pairs: Vec<String> = chosen
        .iter()
        .enumerate()
        .map(|(i, j)| format!("row {} -> column {}", i + 1, j + 1))
        .collect();
    Some(Ok(format!(
        "{} cost {}: {}",
        if maximise { "maximum" } else { "minimum" },
        show(&total),
        pairs.join(", ")
    )))
}

fn parse_list(text: &str) -> Result<Vec<Q>, String> {
    let inner = text.trim().trim_start_matches('[').trim_end_matches(']');
    // A single row of the matrix parser: `[[a, b, c]]`.
    let matrix = parse_matrix(&format!("[[{inner}]]"), false)?;
    Ok(matrix[0].clone())
}

/// `transportation(supply, demand, costs)`: ship from the sources to the sinks at minimum total cost, the shipments from source `i` to
/// sink `j` costing `costs[i][j]` per unit; total supply must cover total demand (a surplus stays at the sources).
fn transportation(body: &str) -> Result<String, String> {
    use crate::simplex_exact::{Outcome, Sense, maximise};
    let parts = crate::graph_commands::split_top(body);
    let [supply, demand, costs] = parts.as_slice() else {
        return Err("transportation(supply, demand, costs)".into());
    };
    let (supply, demand) = (parse_list(supply)?, parse_list(demand)?);
    let costs = parse_matrix(costs, false)?;
    let (m, n) = (supply.len(), demand.len());
    if costs.len() != m || costs[0].len() != n {
        return Err("costs must have one row per source and one column per sink".into());
    }
    if supply.iter().chain(&demand).any(|v| v < &Q::zero()) {
        return Err("supplies and demands must not be negative".into());
    }
    // Variables x[i][j] at index i * n + j; maximise the negated cost.
    let objective: Vec<Q> = (0..m * n).map(|k| -costs[k / n][k % n].clone()).collect();
    let mut rows = Vec::new();
    let mut senses = Vec::new();
    let mut rhs = Vec::new();
    for (i, s) in supply.iter().enumerate() {
        rows.push(
            (0..m * n)
                .map(|k| {
                    if k / n == i {
                        Q::from_integer(1.into())
                    } else {
                        Q::zero()
                    }
                })
                .collect(),
        );
        senses.push(Sense::LessEqual);
        rhs.push(s.clone());
    }
    for (j, d) in demand.iter().enumerate() {
        rows.push(
            (0..m * n)
                .map(|k| {
                    if k % n == j {
                        Q::from_integer(1.into())
                    } else {
                        Q::zero()
                    }
                })
                .collect(),
        );
        senses.push(Sense::GreaterEqual);
        rhs.push(d.clone());
    }
    match maximise(&objective, &rows, &senses, &rhs) {
        Outcome::Optimal { value, point } => {
            let shipments: Vec<String> = (0..m)
                .flat_map(|i| (0..n).map(move |j| (i, j)))
                .filter(|&(i, j)| !point[i * n + j].is_zero())
                .map(|(i, j)| {
                    format!(
                        "{} from source {} to sink {}",
                        show(&point[i * n + j]),
                        i + 1,
                        j + 1
                    )
                })
                .collect();
            Ok(format!(
                "minimum cost {}: {}",
                show(&-value),
                shipments.join(", ")
            ))
        }
        Outcome::Infeasible => Err("infeasible: the total supply does not cover the demand".into()),
        Outcome::Unbounded => Err("unbounded".into()),
    }
}

fn rational(text: &str) -> Option<Q> {
    let t = text.trim();
    match t.split_once('/') {
        Some((a, b)) => {
            let (a, b) = (
                a.trim().parse::<num_bigint::BigInt>().ok()?,
                b.trim().parse::<num_bigint::BigInt>().ok()?,
            );
            (!b.is_zero()).then(|| Q::new(a, b))
        }
        None => t.parse::<num_bigint::BigInt>().ok().map(Q::from_integer),
    }
}

/// `min_cost_flow(arcs, balances)`: arcs `[from, to, capacity, cost]`, balances `[node, b]` with `b > 0` for a source of `b` units
/// and `b < 0` for a sink; the cheapest flow meeting every balance within the arc capacities (an exact linear program).
fn min_cost_flow(body: &str) -> Result<String, String> {
    use crate::simplex_exact::{Outcome, Sense, maximise};
    let parts = crate::graph_commands::split_top(body);
    let [arc_text, balance_text] = parts.as_slice() else {
        return Err("min_cost_flow(arcs, balances): arcs [[u, v, capacity, cost], ...], balances [[node, b], ...]".into());
    };
    let strip = |t: &str| {
        let t = t.trim();
        t.strip_prefix('[')
            .and_then(|r| r.strip_suffix(']'))
            .unwrap_or(t)
            .to_string()
    };
    let mut names: Vec<String> = Vec::new();
    let mut node = |label: &str| match names.iter().position(|n| n == label.trim()) {
        Some(i) => i,
        None => {
            names.push(label.trim().to_string());
            names.len() - 1
        }
    };
    let mut arcs: Vec<(usize, usize, Q, Q)> = Vec::new();
    for item in crate::graph_commands::split_top(&strip(arc_text)) {
        let owned = strip(&item);
        let cells: Vec<&str> = owned.split(',').map(str::trim).collect();
        let [u, v, cap, cost] = cells.as_slice() else {
            return Err(format!("{item} is not an arc [from, to, capacity, cost]"));
        };
        let (cap, cost) = (
            rational(cap).ok_or("capacities must be numbers")?,
            rational(cost).ok_or("costs must be numbers")?,
        );
        if cap < Q::zero() {
            return Err("capacities must not be negative".into());
        }
        arcs.push((node(u), node(v), cap, cost));
    }
    let mut balances: Vec<(usize, Q)> = Vec::new();
    for item in crate::graph_commands::split_top(&strip(balance_text)) {
        let owned = strip(&item);
        let cells: Vec<&str> = owned.split(',').map(str::trim).collect();
        let [label, amount] = cells.as_slice() else {
            return Err(format!("{item} is not a balance [node, amount]"));
        };
        balances.push((
            node(label),
            rational(amount).ok_or("balances must be numbers")?,
        ));
    }
    let n = names.len();
    let m = arcs.len();
    let total: Q = balances.iter().fold(Q::zero(), |a, (_, b)| a + b);
    if !total.is_zero() {
        return Err("the balances must add up to zero (supplies equal demands)".into());
    }
    let one = Q::from_integer(1.into());
    let mut rows: Vec<Vec<Q>> = Vec::new();
    let mut senses = Vec::new();
    let mut rhs = Vec::new();
    for v in 0..n {
        // outflow - inflow = balance of v
        rows.push(
            arcs.iter()
                .map(|(a, b, _, _)| {
                    if *a == v && *b == v {
                        Q::zero()
                    } else if *a == v {
                        one.clone()
                    } else if *b == v {
                        -one.clone()
                    } else {
                        Q::zero()
                    }
                })
                .collect(),
        );
        senses.push(Sense::Equal);
        rhs.push(
            balances
                .iter()
                .filter(|(x, _)| *x == v)
                .fold(Q::zero(), |a, (_, b)| a + b),
        );
    }
    for (k, (_, _, cap, _)) in arcs.iter().enumerate() {
        rows.push(
            (0..m)
                .map(|j| if j == k { one.clone() } else { Q::zero() })
                .collect(),
        );
        senses.push(Sense::LessEqual);
        rhs.push(cap.clone());
    }
    let objective: Vec<Q> = arcs.iter().map(|(_, _, _, c)| -c.clone()).collect();
    match maximise(&objective, &rows, &senses, &rhs) {
        Outcome::Optimal { value, point } => {
            let flows: Vec<String> = arcs
                .iter()
                .enumerate()
                .filter(|(k, _)| !point[*k].is_zero())
                .map(|(k, (a, b, _, _))| {
                    format!("{} on {}->{}", show(&point[k]), names[*a], names[*b])
                })
                .collect();
            Ok(format!(
                "minimum cost {}: {}",
                show(&-value),
                if flows.is_empty() {
                    "no flow".to_string()
                } else {
                    flows.join(", ")
                }
            ))
        }
        Outcome::Infeasible => {
            Err("infeasible: the capacities cannot carry the required flow".into())
        }
        Outcome::Unbounded => {
            Err("unbounded: a negative-cost cycle with unlimited capacity".into())
        }
    }
}
