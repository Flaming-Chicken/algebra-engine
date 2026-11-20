//! Path and matching problems on simple undirected graphs (named graphs `K4`, `C5`, ... or edge lists):
//! `is_eulerian`, `euler_path` (Hierholzer), `hamiltonian_path`, `hamiltonian_cycle` and `max_matching`.
//! The Hamiltonian and matching searches use bitmask dynamic programming and accept up to 20 vertices.

use crate::graph_commands::{Graph, parse_graph};
use std::collections::BTreeSet;

fn labels(g: &Graph, path: &[usize]) -> String {
    path.iter()
        .map(|v| g.names[*v].as_str())
        .collect::<Vec<_>>()
        .join(" - ")
}

fn connected_among_edges(g: &Graph) -> bool {
    let adj = g.adjacency();
    let Some(start) = (0..g.n()).find(|v| !adj[*v].is_empty()) else {
        return true;
    };
    let mut seen = vec![false; g.n()];
    seen[start] = true;
    let mut stack = vec![start];
    while let Some(u) = stack.pop() {
        for v in &adj[u] {
            if !seen[*v] {
                seen[*v] = true;
                stack.push(*v);
            }
        }
    }
    (0..g.n()).all(|v| adj[v].is_empty() || seen[v])
}

fn euler(g: &Graph, want_path: bool) -> Result<String, String> {
    let adj = g.adjacency();
    let odd: Vec<usize> = (0..g.n()).filter(|v| adj[*v].len() % 2 == 1).collect();
    if !connected_among_edges(g) || !(odd.is_empty() || odd.len() == 2) {
        return Ok(if want_path {
            "none".into()
        } else {
            "false".into()
        });
    }
    if !want_path {
        return Ok((odd.is_empty()).to_string());
    }
    // Hierholzer: start at an odd vertex when there is one.
    let start = odd
        .first()
        .copied()
        .or_else(|| (0..g.n()).find(|v| !adj[*v].is_empty()))
        .ok_or("the graph has no edges")?;
    let mut remaining: Vec<BTreeSet<usize>> =
        adj.iter().map(|a| a.iter().copied().collect()).collect();
    let mut stack = vec![start];
    let mut path = Vec::new();
    while let Some(&u) = stack.last() {
        if let Some(&v) = remaining[u].iter().next() {
            remaining[u].remove(&v);
            remaining[v].remove(&u);
            stack.push(v);
        } else {
            path.push(u);
            stack.pop();
        }
    }
    path.reverse();
    let kind = if odd.is_empty() { "circuit" } else { "path" };
    Ok(format!("Euler {kind}: {}", labels(g, &path)))
}

fn hamiltonian(g: &Graph, cycle: bool) -> Result<String, String> {
    let n = g.n();
    if n > 20 {
        return Err("hamiltonian search is limited to 20 vertices".into());
    }
    if n < 3 && cycle {
        return Ok("none".into());
    }
    let adj = g.adjacency();
    let mut adjacent = vec![0u32; n];
    for (u, row) in adj.iter().enumerate() {
        for v in row {
            adjacent[u] |= 1 << v;
        }
    }
    let full = (1u32 << n) - 1;
    // reach[mask] bit v: a path covering exactly `mask` that starts at `start` and ends at v.
    let starts: Vec<usize> = if cycle { vec![0] } else { (0..n).collect() };
    for start in starts {
        let mut reach = vec![0u32; 1 << n];
        reach[1 << start] = 1 << start;
        for mask in 1..=full {
            let ends = reach[mask as usize];
            if ends == 0 {
                continue;
            }
            for v in 0..n {
                if ends >> v & 1 == 1 {
                    let mut next = adjacent[v] & !mask;
                    while next != 0 {
                        let w = next.trailing_zeros() as usize;
                        reach[(mask | 1 << w) as usize] |= 1 << w;
                        next &= next - 1;
                    }
                }
            }
        }
        let ends = reach[full as usize];
        let finish =
            (0..n).find(|v| ends >> v & 1 == 1 && (!cycle || adjacent[*v] >> start & 1 == 1));
        if let Some(mut end) = finish {
            // Walk back through the masks.
            let mut path = vec![end];
            let mut mask = full;
            while mask != 1 << start {
                let before = mask & !(1 << end);
                let prev = (0..n)
                    .find(|u| reach[before as usize] >> u & 1 == 1 && adjacent[*u] >> end & 1 == 1)
                    .ok_or("internal error")?;
                path.push(prev);
                mask = before;
                end = prev;
            }
            path.reverse();
            if cycle {
                path.push(start);
            }
            return Ok(labels(g, &path));
        }
    }
    Ok("none".into())
}

fn matching(g: &Graph) -> Result<String, String> {
    let n = g.n();
    if n > 20 {
        return Err("max_matching is limited to 20 vertices".into());
    }
    let adj = g.adjacency();
    let mut best = vec![0u8; 1 << n];
    for mask in 1u32..1 << n {
        let u = mask.trailing_zeros() as usize;
        let rest = mask & (mask - 1);
        let mut value = best[rest as usize];
        for v in &adj[u] {
            if rest >> v & 1 == 1 {
                value = value.max(1 + best[(rest & !(1 << v)) as usize]);
            }
        }
        best[mask as usize] = value;
    }
    let (mut mask, mut pairs) = ((1u32 << n) - 1, Vec::new());
    while mask != 0 {
        let u = mask.trailing_zeros() as usize;
        let rest = mask & (mask - 1);
        if best[mask as usize] == best[rest as usize] {
            mask = rest;
            continue;
        }
        let v = adj[u]
            .iter()
            .copied()
            .find(|v| {
                rest >> v & 1 == 1 && best[mask as usize] == 1 + best[(rest & !(1 << v)) as usize]
            })
            .ok_or("internal error")?;
        pairs.push(format!("{}-{}", g.names[u], g.names[v]));
        mask = rest & !(1 << v);
    }
    Ok(format!("{}: {}", pairs.len(), pairs.join(", ")))
}

pub fn run(input: &str) -> Option<Result<String, String>> {
    let t = input.trim();
    let open = t.find('(')?;
    let name = t[..open].trim().to_ascii_lowercase();
    if !t.ends_with(')') {
        return None;
    }
    let command = match name.as_str() {
        "is_eulerian" | "eulerian" => "eulerian",
        "euler_path" | "euler_circuit" | "eulerian_path" | "euler_trail" => "euler_path",
        "hamiltonian_path" | "hamilton_path" => "hamiltonian_path",
        "hamiltonian_cycle" | "hamilton_cycle" | "hamiltonian_circuit" => "hamiltonian_cycle",
        "max_matching" | "maximum_matching" | "matching_number" => "matching",
        "is_isomorphic" | "isomorphic" => "isomorphic",
        "automorphisms" | "automorphism_count" | "aut" => "automorphisms",
        _ => return None,
    };
    let body = &t[open + 1..t.len() - 1];
    if command == "isomorphic" {
        let parts = crate::graph_commands::split_top(body);
        let [a, b] = parts.as_slice() else {
            return Some(Err("is_isomorphic(G, H)".into()));
        };
        return Some(parse_graph(a).and_then(|g| parse_graph(b).map(|h| isomorphic(&g, &h))));
    }
    let g = match parse_graph(body) {
        Ok(g) => g,
        Err(e) => return Some(Err(e)),
    };
    Some(match command {
        "eulerian" => euler(&g, false),
        "euler_path" => euler(&g, true),
        "hamiltonian_path" => hamiltonian(&g, false),
        "hamiltonian_cycle" => hamiltonian(&g, true),
        "automorphisms" => automorphisms(&g),
        _ => matching(&g),
    })
}

/// Adjacency bitmasks (vertices up to 64).
fn masks(g: &Graph) -> Vec<u64> {
    let mut m = vec![0u64; g.n()];
    for &(a, b) in &g.edges {
        m[a] |= 1 << b;
        m[b] |= 1 << a;
    }
    m
}

/// Count (or find the first of) the bijections from `g` to `h` that preserve adjacency, extending partial maps vertex by vertex
/// and pruning by degree and by adjacency to the vertices already placed.
fn extend(g: &[u64], h: &[u64], map: &mut Vec<usize>, used: u64, count_all: bool, found: &mut u64) {
    let k = map.len();
    if k == g.len() {
        *found += 1;
        return;
    }
    for candidate in 0..h.len() {
        if used >> candidate & 1 == 1 || g[k].count_ones() != h[candidate].count_ones() {
            continue;
        }
        let consistent = (0..k).all(|j| (g[k] >> j & 1) == (h[candidate] >> map[j] & 1));
        if !consistent {
            continue;
        }
        map.push(candidate);
        extend(g, h, map, used | 1 << candidate, count_all, found);
        map.pop();
        if !count_all && *found > 0 {
            return;
        }
    }
}

fn isomorphic(g: &Graph, h: &Graph) -> String {
    if g.n() != h.n() || g.edges.len() != h.edges.len() || g.n() > 64 {
        return "false".into();
    }
    let (a, b) = (masks(g), masks(h));
    let mut degrees_a: Vec<u32> = a.iter().map(|m| m.count_ones()).collect();
    let mut degrees_b: Vec<u32> = b.iter().map(|m| m.count_ones()).collect();
    degrees_a.sort_unstable();
    degrees_b.sort_unstable();
    if degrees_a != degrees_b {
        return "false".into();
    }
    let mut found = 0;
    extend(&a, &b, &mut Vec::new(), 0, false, &mut found);
    (found > 0).to_string()
}

fn automorphisms(g: &Graph) -> Result<String, String> {
    if g.n() > 12 {
        return Err("automorphisms is limited to 12 vertices".into());
    }
    let a = masks(g);
    let mut found = 0;
    extend(&a, &a, &mut Vec::new(), 0, true, &mut found);
    Ok(found.to_string())
}
