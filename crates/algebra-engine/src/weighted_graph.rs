//! Weighted and directed graph commands on edge lists `[[a, b, w], ...]` (weights are exact rationals; a missing weight
//! is 1): `dijkstra` (negative weights through Bellman-Ford), `minimum_spanning_tree`, `max_flow` with a minimum cut,
//! `topological_sort` and `strongly_connected_components`. `dijkstra` treats edges as undirected unless the last
//! argument is `directed`; `max_flow`, `topological_sort` and the components are directed.

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};
use std::collections::VecDeque;

type Q = BigRational;

struct Edges {
    names: Vec<String>,
    list: Vec<(usize, usize, Q)>,
}

fn show(q: &Q) -> String {
    if q.is_integer() {
        q.numer().to_string()
    } else {
        format!("{}/{}", q.numer(), q.denom())
    }
}

fn weight(text: &str) -> Option<Q> {
    let t = text.trim();
    match t.split_once('/') {
        Some((a, b)) => {
            let (a, b) = (
                a.trim().parse::<BigInt>().ok()?,
                b.trim().parse::<BigInt>().ok()?,
            );
            (!b.is_zero()).then(|| Q::new(a, b))
        }
        None => match t.split_once('.') {
            Some((whole, frac)) if frac.chars().all(|c| c.is_ascii_digit()) && !frac.is_empty() => {
                let digits = BigInt::from(10).pow(frac.len() as u32);
                let all: BigInt = format!("{whole}{frac}").parse().ok()?;
                Some(Q::new(all, digits))
            }
            _ => t.parse::<BigInt>().ok().map(Q::from_integer),
        },
    }
}

fn parse_edges(text: &str) -> Result<Edges, String> {
    let inner = text
        .trim()
        .strip_prefix('[')
        .and_then(|r| r.strip_suffix(']'))
        .ok_or("the graph must be an edge list [[a, b, w], ...]")?;
    let mut names: Vec<String> = Vec::new();
    let mut index = |label: &str| match names.iter().position(|n| n == label) {
        Some(i) => i,
        None => {
            names.push(label.to_string());
            names.len() - 1
        }
    };
    let mut list = Vec::new();
    for item in crate::graph_commands::split_top(inner) {
        let body = item.trim().trim_start_matches('[').trim_end_matches(']');
        let parts: Vec<&str> = body.split(',').map(str::trim).collect();
        let (a, b, w) = match parts.as_slice() {
            [a, b] => (*a, *b, Q::one()),
            [a, b, w] => (
                *a,
                *b,
                weight(w).ok_or_else(|| format!("{w} is not a weight"))?,
            ),
            _ => return Err(format!("{item} is not an edge [a, b, w]")),
        };
        let (a, b) = (index(a), index(b));
        list.push((a, b, w));
    }
    if list.is_empty() {
        return Err("the graph has no edges".into());
    }
    Ok(Edges { names, list })
}

fn vertex(g: &Edges, label: &str) -> Result<usize, String> {
    g.names
        .iter()
        .position(|n| n == label.trim())
        .ok_or_else(|| format!("{} is not a vertex", label.trim()))
}

fn directed_flag(args: &mut Vec<String>) -> bool {
    if args
        .last()
        .is_some_and(|l| l.trim().eq_ignore_ascii_case("directed"))
    {
        args.pop();
        return true;
    }
    false
}

type Paths = (Vec<Option<Q>>, Vec<Option<usize>>);

/// Single-source shortest paths by Bellman-Ford: `(distance, predecessor)`; `Err` on a reachable negative cycle.
fn bellman_ford(n: usize, arcs: &[(usize, usize, Q)], from: usize) -> Result<Paths, String> {
    let mut dist: Vec<Option<Q>> = vec![None; n];
    let mut parent = vec![None; n];
    dist[from] = Some(Q::zero());
    for round in 0..=n {
        let mut changed = false;
        for (a, b, w) in arcs {
            if let Some(da) = dist[*a].clone() {
                let candidate = da + w;
                if dist[*b].as_ref().is_none_or(|d| candidate < *d) {
                    dist[*b] = Some(candidate);
                    parent[*b] = Some(*a);
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
        if round == n {
            return Err("a negative cycle is reachable: shortest paths are undefined".into());
        }
    }
    Ok((dist, parent))
}

fn shortest(g: &Edges, args: &[String], directed: bool) -> Result<String, String> {
    let [_, from, to] = args else {
        return Err("dijkstra(edges, from, to[, directed])".into());
    };
    let (a, b) = (vertex(g, from)?, vertex(g, to)?);
    let mut arcs = g.list.clone();
    if !directed {
        if g.list.iter().any(|(_, _, w)| w.is_negative()) {
            return Err("a negative weight on an undirected edge is a negative cycle".into());
        }
        arcs.extend(g.list.iter().map(|(x, y, w)| (*y, *x, w.clone())));
    }
    let (dist, parent) = bellman_ford(g.names.len(), &arcs, a)?;
    let Some(length) = dist[b].clone() else {
        return Ok("no path".into());
    };
    let mut path = vec![b];
    while let Some(p) = parent[*path.last().unwrap_or(&b)] {
        path.push(p);
        if path.len() > g.names.len() {
            break;
        }
    }
    path.reverse();
    let labels: Vec<&str> = path.iter().map(|&v| g.names[v].as_str()).collect();
    Ok(format!("length {}: {}", show(&length), labels.join(" - ")))
}

fn find(root: &mut [usize], x: usize) -> usize {
    let mut r = x;
    while root[r] != r {
        r = root[r];
    }
    root[x] = r;
    r
}

fn spanning_tree(g: &Edges) -> Result<String, String> {
    let n = g.names.len();
    let mut sorted = g.list.clone();
    sorted.sort_by(|x, y| x.2.cmp(&y.2));
    let mut root: Vec<usize> = (0..n).collect();
    let (mut total, mut chosen) = (Q::zero(), Vec::new());
    for (a, b, w) in sorted {
        let (ra, rb) = (find(&mut root, a), find(&mut root, b));
        if ra != rb {
            root[ra] = rb;
            total += &w;
            chosen.push(format!("{}-{} ({})", g.names[a], g.names[b], show(&w)));
        }
    }
    if chosen.len() + 1 != n {
        return Err("the graph is not connected: there is no spanning tree".into());
    }
    Ok(format!("weight {}: {}", show(&total), chosen.join(", ")))
}

fn max_flow(g: &Edges, args: &[String]) -> Result<String, String> {
    let [_, source, sink] = args else {
        return Err("max_flow(edges, source, sink)".into());
    };
    let (s, t) = (vertex(g, source)?, vertex(g, sink)?);
    if s == t {
        return Err("the source and the sink must differ".into());
    }
    let n = g.names.len();
    let mut capacity = vec![vec![Q::zero(); n]; n];
    for (a, b, w) in &g.list {
        if w.is_negative() {
            return Err("capacities must be non-negative".into());
        }
        capacity[*a][*b] += w;
    }
    let original = capacity.clone();
    let mut flow = Q::zero();
    loop {
        let mut parent: Vec<Option<usize>> = vec![None; n];
        parent[s] = Some(s);
        let mut queue = VecDeque::from([s]);
        while let Some(u) = queue.pop_front() {
            for v in 0..n {
                if parent[v].is_none() && capacity[u][v].is_positive() {
                    parent[v] = Some(u);
                    queue.push_back(v);
                }
            }
        }
        if parent[t].is_none() {
            // The vertices still reachable from the source form the source side of a minimum cut.
            let side: Vec<usize> = (0..n).filter(|v| parent[*v].is_some()).collect();
            let cut: Vec<String> = side
                .iter()
                .flat_map(|&u| {
                    (0..n)
                        .filter(|v| !side.contains(v) && original[u][*v].is_positive())
                        .map(move |v| (u, v))
                        .collect::<Vec<_>>()
                })
                .map(|(u, v)| format!("{}->{}", g.names[u], g.names[v]))
                .collect();
            return Ok(format!(
                "max flow {}, minimum cut: {}",
                show(&flow),
                cut.join(", ")
            ));
        }
        let mut bottleneck: Option<Q> = None;
        let mut v = t;
        while v != s {
            let u = parent[v].unwrap_or(s);
            bottleneck = Some(match bottleneck {
                Some(b) if b < capacity[u][v] => b,
                _ => capacity[u][v].clone(),
            });
            v = u;
        }
        let bottleneck = bottleneck.unwrap_or_else(Q::zero);
        let mut v = t;
        while v != s {
            let u = parent[v].unwrap_or(s);
            capacity[u][v] -= &bottleneck;
            capacity[v][u] += &bottleneck;
            v = u;
        }
        flow += bottleneck;
    }
}

fn adjacency(g: &Edges) -> Vec<Vec<usize>> {
    let mut adj = vec![Vec::new(); g.names.len()];
    for (a, b, _) in &g.list {
        adj[*a].push(*b);
    }
    adj
}

fn topological(g: &Edges) -> Result<String, String> {
    let adj = adjacency(g);
    let n = g.names.len();
    let mut indegree = vec![0usize; n];
    for row in &adj {
        for v in row {
            indegree[*v] += 1;
        }
    }
    let mut ready: VecDeque<usize> = (0..n).filter(|v| indegree[*v] == 0).collect();
    let mut order = Vec::new();
    while let Some(u) = ready.pop_front() {
        order.push(u);
        for v in &adj[u] {
            indegree[*v] -= 1;
            if indegree[*v] == 0 {
                ready.push_back(*v);
            }
        }
    }
    if order.len() != n {
        return Err("the graph has a directed cycle: no topological order".into());
    }
    Ok(order
        .iter()
        .map(|v| g.names[*v].as_str())
        .collect::<Vec<_>>()
        .join(", "))
}

fn strong_components(g: &Edges) -> String {
    let n = g.names.len();
    let adj = adjacency(g);
    let reach = |from: usize| -> Vec<bool> {
        let mut seen = vec![false; n];
        seen[from] = true;
        let mut stack = vec![from];
        while let Some(u) = stack.pop() {
            for v in &adj[u] {
                if !seen[*v] {
                    seen[*v] = true;
                    stack.push(*v);
                }
            }
        }
        seen
    };
    let reachable: Vec<Vec<bool>> = (0..n).map(reach).collect();
    let mut assigned = vec![false; n];
    let mut parts = Vec::new();
    for v in 0..n {
        if assigned[v] {
            continue;
        }
        let members: Vec<usize> = (0..n)
            .filter(|u| reachable[v][*u] && reachable[*u][v])
            .collect();
        for u in &members {
            assigned[*u] = true;
        }
        let labels: Vec<&str> = members.iter().map(|u| g.names[*u].as_str()).collect();
        parts.push(format!("{{{}}}", labels.join(", ")));
    }
    format!(
        "{} strongly connected components: {}",
        parts.len(),
        parts.join(", ")
    )
}

pub fn run(input: &str) -> Option<Result<String, String>> {
    let t = input.trim();
    let open = t.find('(')?;
    let name = t[..open].trim().to_ascii_lowercase();
    if !t.ends_with(')') {
        return None;
    }
    let command = match name.as_str() {
        "dijkstra" | "weighted_shortest_path" | "shortest_weighted_path" | "bellman_ford" => {
            "shortest"
        }
        "minimum_spanning_tree" | "mst" | "kruskal_mst" | "min_spanning_tree" => "mst",
        "max_flow" | "maximum_flow" | "min_cut" => "flow",
        "topological_sort" | "topological_order" | "toposort" => "topological",
        "strongly_connected_components" | "scc" | "strong_components" => "scc",
        _ => return None,
    };
    let mut args = crate::graph_commands::split_top(&t[open + 1..t.len() - 1]);
    let directed = directed_flag(&mut args);
    let g = match parse_edges(args.first()?) {
        Ok(g) => g,
        Err(e) => return Some(Err(e)),
    };
    Some(match command {
        "shortest" => shortest(&g, &args, directed),
        "mst" => spanning_tree(&g),
        "flow" => max_flow(&g, &args),
        "topological" => topological(&g),
        _ => Ok(strong_components(&g)),
    })
}
