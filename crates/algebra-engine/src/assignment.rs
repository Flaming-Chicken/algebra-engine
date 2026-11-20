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

fn parse_matrix(text: &str) -> Result<Vec<Vec<Q>>, String> {
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
    if matrix.len() > matrix[0].len() {
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
    let maximise = match name.as_str() {
        "assignment" | "min_assignment" | "hungarian" | "linear_assignment" => false,
        "max_assignment" | "maximum_assignment" => true,
        _ => return None,
    };
    let mut cost = match parse_matrix(&t[open + 1..t.len() - 1]) {
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
