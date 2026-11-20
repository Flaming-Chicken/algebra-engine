//! Finite graph commands on named graphs (`K4`, `C5`, `P4`) or edge lists (`[[1,2],[2,3],[3,1]]`):
//! `degree_sequence`, `is_connected`, `components`, `is_bipartite`, `is_tree`, `spanning_trees`, `chromatic_polynomial`,
//! `chromatic_number`, `shortest_path` and `diameter`. Everything is exact (integer arithmetic).

use num_bigint::BigInt;
use num_traits::{One, Zero};
use std::collections::{BTreeSet, VecDeque};

/// A simple undirected graph on vertices `0..n` with named vertices.
pub(crate) struct Graph {
    pub(crate) names: Vec<String>,
    pub(crate) edges: BTreeSet<(usize, usize)>,
}

impl Graph {
    pub(crate) fn n(&self) -> usize {
        self.names.len()
    }

    pub(crate) fn adjacency(&self) -> Vec<Vec<usize>> {
        let mut adj = vec![Vec::new(); self.n()];
        for &(a, b) in &self.edges {
            adj[a].push(b);
            adj[b].push(a);
        }
        adj
    }
}

pub(crate) fn split_top(text: &str) -> Vec<String> {
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

/// `K4`, `C5`, `P4`, `S4` (a star with 4 leaves) or an edge list.
pub(crate) fn parse_graph(text: &str) -> Result<Graph, String> {
    let t = text.trim();
    if let Some(rest) = t.strip_prefix(['K', 'C', 'P', 'S', 'k', 'c', 'p', 's'])
        && let Ok(n) = rest.parse::<usize>()
    {
        let kind = t.chars().next().unwrap_or('K').to_ascii_uppercase();
        if !(1..=64).contains(&n) {
            return Err("a named graph needs 1 to 64 vertices".into());
        }
        let count = if kind == 'S' { n + 1 } else { n };
        let mut edges = BTreeSet::new();
        match kind {
            'K' => {
                for a in 0..n {
                    for b in a + 1..n {
                        edges.insert((a, b));
                    }
                }
            }
            'C' => {
                if n < 3 {
                    return Err("a cycle needs at least 3 vertices".into());
                }
                for a in 0..n {
                    let b = (a + 1) % n;
                    edges.insert((a.min(b), a.max(b)));
                }
            }
            'P' => {
                for a in 0..n.saturating_sub(1) {
                    edges.insert((a, a + 1));
                }
            }
            _ => {
                for leaf in 1..=n {
                    edges.insert((0, leaf));
                }
            }
        }
        return Ok(Graph {
            names: (1..=count).map(|i| i.to_string()).collect(),
            edges,
        });
    }
    let inner = t
        .strip_prefix('[')
        .and_then(|r| r.strip_suffix(']'))
        .or_else(|| t.strip_prefix('{').and_then(|r| r.strip_suffix('}')))
        .ok_or_else(|| {
            format!("{t} is not a graph: use K4, C5, P4, S4 or an edge list [[1,2],[2,3]]")
        })?;
    let mut names: Vec<String> = Vec::new();
    let mut index = |label: &str| -> usize {
        match names.iter().position(|n| n == label) {
            Some(i) => i,
            None => {
                names.push(label.to_string());
                names.len() - 1
            }
        }
    };
    let mut edges = BTreeSet::new();
    for item in split_top(inner) {
        let body = item
            .trim()
            .trim_start_matches(['[', '('])
            .trim_end_matches([']', ')']);
        let ends: Vec<&str> = if body.contains('-') {
            body.split('-').map(str::trim).collect()
        } else {
            body.split(',').map(str::trim).collect()
        };
        let [a, b] = ends.as_slice() else {
            return Err(format!("{item} is not an edge"));
        };
        let (a, b) = (index(a), index(b));
        if a != b {
            edges.insert((a.min(b), a.max(b)));
        }
    }
    Ok(Graph { names, edges })
}

fn components(graph: &Graph) -> Vec<Vec<usize>> {
    let adj = graph.adjacency();
    let mut seen = vec![false; graph.n()];
    let mut out = Vec::new();
    for start in 0..graph.n() {
        if seen[start] {
            continue;
        }
        let (mut queue, mut component) = (VecDeque::from([start]), Vec::new());
        seen[start] = true;
        while let Some(v) = queue.pop_front() {
            component.push(v);
            for &w in &adj[v] {
                if !seen[w] {
                    seen[w] = true;
                    queue.push_back(w);
                }
            }
        }
        out.push(component);
    }
    out
}

fn bfs_distances(graph: &Graph, from: usize) -> (Vec<Option<usize>>, Vec<Option<usize>>) {
    let adj = graph.adjacency();
    let (mut dist, mut parent) = (vec![None; graph.n()], vec![None; graph.n()]);
    dist[from] = Some(0);
    let mut queue = VecDeque::from([from]);
    while let Some(v) = queue.pop_front() {
        for &w in &adj[v] {
            if dist[w].is_none() {
                dist[w] = Some(dist[v].unwrap_or(0) + 1);
                parent[w] = Some(v);
                queue.push_back(w);
            }
        }
    }
    (dist, parent)
}

/// Determinant of an integer matrix by the fraction-free Bareiss algorithm.
fn determinant(mut m: Vec<Vec<BigInt>>) -> BigInt {
    let n = m.len();
    if n == 0 {
        return BigInt::one();
    }
    let (mut sign, mut previous) = (1i32, BigInt::one());
    for k in 0..n - 1 {
        if m[k][k].is_zero() {
            let Some(swap) = (k + 1..n).find(|&r| !m[r][k].is_zero()) else {
                return BigInt::zero();
            };
            m.swap(k, swap);
            sign = -sign;
        }
        for i in k + 1..n {
            for j in k + 1..n {
                m[i][j] = (&m[i][j] * &m[k][k] - &m[i][k] * &m[k][j]) / &previous;
            }
        }
        previous = m[k][k].clone();
    }
    let d = m[n - 1][n - 1].clone();
    if sign < 0 { -d } else { d }
}

/// The number of spanning trees: any cofactor of the Laplacian (the matrix-tree theorem).
fn spanning_trees(graph: &Graph) -> BigInt {
    let n = graph.n();
    if n == 0 {
        return BigInt::zero();
    }
    let mut laplacian = vec![vec![BigInt::zero(); n]; n];
    for &(a, b) in &graph.edges {
        laplacian[a][a] += 1;
        laplacian[b][b] += 1;
        laplacian[a][b] -= 1;
        laplacian[b][a] -= 1;
    }
    let minor: Vec<Vec<BigInt>> = laplacian[1..].iter().map(|row| row[1..].to_vec()).collect();
    determinant(minor)
}

/// The chromatic polynomial by deletion-contraction: coefficients of `k^0, k^1, ...`.
fn chromatic(n: usize, edges: &BTreeSet<(usize, usize)>) -> Vec<BigInt> {
    let Some(&(a, b)) = edges.iter().next() else {
        let mut p = vec![BigInt::zero(); n + 1];
        p[n] = BigInt::one();
        return p;
    };
    let mut deleted = edges.clone();
    deleted.remove(&(a, b));
    // Contract b into a: relabel b as a, drop the loop, renumber vertices above b.
    let relabel = |v: usize| -> usize {
        let v = if v == b { a } else { v };
        if v > b { v - 1 } else { v }
    };
    let contracted: BTreeSet<(usize, usize)> = deleted
        .iter()
        .filter_map(|&(x, y)| {
            let (x, y) = (relabel(x), relabel(y));
            (x != y).then_some((x.min(y), x.max(y)))
        })
        .collect();
    let (without, with) = (chromatic(n, &deleted), chromatic(n - 1, &contracted));
    let mut out = vec![BigInt::zero(); without.len().max(with.len())];
    for (i, c) in without.iter().enumerate() {
        out[i] += c;
    }
    for (i, c) in with.iter().enumerate() {
        out[i] -= c;
    }
    out
}

fn polynomial_text(coefficients: &[BigInt]) -> String {
    let mut out = String::new();
    for (power, c) in coefficients.iter().enumerate().rev() {
        if c.is_zero() {
            continue;
        }
        let magnitude = if c < &BigInt::zero() {
            -c.clone()
        } else {
            c.clone()
        };
        let variable = match power {
            0 => String::new(),
            1 => "k".to_string(),
            p => format!("k^{p}"),
        };
        let body = match (power, magnitude == BigInt::one()) {
            (0, _) => magnitude.to_string(),
            (_, true) => variable,
            _ => format!("{magnitude}*{variable}"),
        };
        if out.is_empty() {
            out.push_str(&format!(
                "{}{body}",
                if c < &BigInt::zero() { "-" } else { "" }
            ));
        } else {
            out.push_str(&format!(
                " {} {body}",
                if c < &BigInt::zero() { "-" } else { "+" }
            ));
        }
    }
    if out.is_empty() { "0".into() } else { out }
}

fn evaluate_polynomial(coefficients: &[BigInt], k: i64) -> BigInt {
    coefficients
        .iter()
        .rev()
        .fold(BigInt::zero(), |acc, c| acc * k + c)
}

/// `Some(result)` when `input` is a graph command.
pub fn run(input: &str) -> Option<Result<String, String>> {
    let t = input.trim();
    let open = t.find('(')?;
    let name = t[..open].trim().to_ascii_lowercase();
    if !t.ends_with(')') {
        return None;
    }
    let command = match name.as_str() {
        "degree_sequence" | "degrees_of" => "degree_sequence",
        "is_connected" | "connected" => "is_connected",
        "components" | "connected_components" | "num_components" => "components",
        "is_bipartite" | "bipartite" => "is_bipartite",
        "is_tree" => "is_tree",
        "spanning_trees" | "count_spanning_trees" | "num_spanning_trees" => "spanning_trees",
        "chromatic_polynomial" => "chromatic_polynomial",
        "chromatic_number" => "chromatic_number",
        "shortest_path" | "graph_distance" => "shortest_path",
        "diameter" | "graph_diameter" => "diameter",
        _ => return None,
    };
    let args = split_top(&t[open + 1..t.len() - 1]);
    Some(evaluate(command, &args))
}

fn evaluate(command: &str, args: &[String]) -> Result<String, String> {
    let graph = parse_graph(args.first().ok_or("a graph is needed")?)?;
    if graph.n() == 0 {
        return Err("the graph has no vertices".into());
    }
    let comps = components(&graph);
    match command {
        "degree_sequence" => {
            let adj = graph.adjacency();
            let mut degrees: Vec<usize> = adj.iter().map(Vec::len).collect();
            degrees.sort_unstable_by(|a, b| b.cmp(a));
            Ok(format!(
                "[{}]",
                degrees
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        }
        "is_connected" => Ok((comps.len() == 1).to_string()),
        "components" => Ok(comps.len().to_string()),
        "is_tree" => Ok((comps.len() == 1 && graph.edges.len() + 1 == graph.n()).to_string()),
        "is_bipartite" => {
            let adj = graph.adjacency();
            let mut colour: Vec<Option<bool>> = vec![None; graph.n()];
            for start in 0..graph.n() {
                if colour[start].is_some() {
                    continue;
                }
                colour[start] = Some(false);
                let mut queue = VecDeque::from([start]);
                while let Some(v) = queue.pop_front() {
                    for &w in &adj[v] {
                        match colour[w] {
                            None => {
                                colour[w] = colour[v].map(|c| !c);
                                queue.push_back(w);
                            }
                            Some(c) if Some(c) == colour[v] => return Ok("false".into()),
                            _ => {}
                        }
                    }
                }
            }
            Ok("true".into())
        }
        "spanning_trees" => Ok(spanning_trees(&graph).to_string()),
        "chromatic_polynomial" | "chromatic_number" => {
            if graph.edges.len() > 24 {
                return Err(
                    "chromatic polynomials are computed for graphs with at most 24 edges".into(),
                );
            }
            let polynomial = chromatic(graph.n(), &graph.edges);
            if command == "chromatic_polynomial" {
                return Ok(format!("P(k) = {}", polynomial_text(&polynomial)));
            }
            let number = (1..=graph.n() as i64)
                .find(|&k| evaluate_polynomial(&polynomial, k) > BigInt::zero())
                .ok_or("no proper colouring found")?;
            Ok(number.to_string())
        }
        "shortest_path" => {
            let [_, from, to] = args else {
                return Err("shortest_path(graph, from, to)".into());
            };
            let find = |label: &str| {
                graph
                    .names
                    .iter()
                    .position(|n| n == label.trim())
                    .ok_or_else(|| format!("{label} is not a vertex"))
            };
            let (a, b) = (find(from)?, find(to)?);
            let (dist, parent) = bfs_distances(&graph, a);
            let Some(length) = dist[b] else {
                return Ok("no path".into());
            };
            let mut path = vec![b];
            while let Some(p) = parent[*path.last().unwrap_or(&b)] {
                path.push(p);
            }
            path.reverse();
            let labels: Vec<&str> = path.iter().map(|&v| graph.names[v].as_str()).collect();
            Ok(format!("length {length}: {}", labels.join(" - ")))
        }
        _ => {
            if comps.len() != 1 {
                return Ok("infinite (the graph is not connected)".into());
            }
            let longest = (0..graph.n())
                .filter_map(|s| bfs_distances(&graph, s).0.into_iter().flatten().max())
                .max()
                .unwrap_or(0);
            Ok(longest.to_string())
        }
    }
}
