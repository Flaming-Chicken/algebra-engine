//! Cartesian vector calculus on symbolic expressions: `gradient`, `hessian`, `jacobian`, `divergence`, `curl`,
//! `laplacian`, and the vector products `cross`, `dot`, `norm`.
//!
//! Vectors are written as lists `[a, b, c]`; the variables are a list `[x, y, z]` (when omitted, the free symbols in
//! alphabetical order).

use crate::calculus::SymbolicCalculus;
use crate::matrix::SymbolicMatrix;
use crate::numeric_commands::{NumericOutput, list_items, parse};
use crate::poly::collect_expr_symbols;
use crate::simplify::SymbolicSimplifier;
use algebra_core::{ExprGraph, ExprId, SymbolId};
use std::collections::BTreeSet;

fn vector(graph: &ExprGraph, text: &str) -> Result<Vec<ExprId>, String> {
    let t = text.trim();
    if !(t.starts_with('[') && t.ends_with(']')) {
        return Err(format!("{t} is not a list such as [a, b, c]"));
    }
    list_items(t).iter().map(|i| parse(graph, i)).collect()
}

fn variables(
    graph: &ExprGraph,
    given: Option<&String>,
    exprs: &[ExprId],
) -> Result<Vec<SymbolId>, String> {
    if let Some(text) = given {
        let items = list_items(text);
        return Ok(items
            .iter()
            .map(|name| graph.symbols.get_or_intern(name.trim()))
            .collect());
    }
    let mut symbols = BTreeSet::new();
    for &e in exprs {
        collect_expr_symbols(graph, e, &mut symbols);
    }
    let mut named: Vec<(String, SymbolId)> = symbols
        .into_iter()
        .map(|s| (graph.symbols.resolve(s).unwrap_or_default(), s))
        .filter(|(name, _)| !matches!(name.as_str(), "pi" | "e" | "i"))
        .collect();
    named.sort();
    Ok(named.into_iter().map(|(_, s)| s).collect())
}

fn d(graph: &ExprGraph, f: ExprId, v: SymbolId) -> ExprId {
    graph.simplify(graph.diff(f, v))
}

fn matrix_output(
    graph: &ExprGraph,
    rows: usize,
    cols: usize,
    elements: Vec<ExprId>,
    note: &str,
) -> Result<NumericOutput, String> {
    let elements: Vec<ExprId> = elements.into_iter().map(|e| graph.simplify(e)).collect();
    let m = SymbolicMatrix::new(rows, cols, elements).map_err(|e| e.to_string())?;
    Ok(NumericOutput::Expression(m.to_expr_id(graph), note.into()))
}

pub fn run(graph: &ExprGraph, name: &str, args: &[String]) -> Result<NumericOutput, String> {
    let args: Vec<String> = args.iter().map(|a| a.trim().to_string()).collect();
    let first = args.first().ok_or("an argument is needed")?;
    match name {
        "gradient" => {
            let f = parse(graph, first)?;
            let vars = variables(graph, args.get(1), &[f])?;
            let elements = vars.iter().map(|&v| d(graph, f, v)).collect::<Vec<_>>();
            matrix_output(graph, 1, vars.len(), elements, "gradient")
        }
        "hessian" => {
            let f = parse(graph, first)?;
            let vars = variables(graph, args.get(1), &[f])?;
            let n = vars.len();
            let mut elements = Vec::with_capacity(n * n);
            for &a in &vars {
                let da = d(graph, f, a);
                for &b in &vars {
                    elements.push(d(graph, da, b));
                }
            }
            matrix_output(graph, n, n, elements, "Hessian")
        }
        "jacobian" => {
            let fs = vector(graph, first)?;
            let vars = variables(graph, args.get(1), &fs)?;
            let mut elements = Vec::new();
            for &f in &fs {
                for &v in &vars {
                    elements.push(d(graph, f, v));
                }
            }
            matrix_output(graph, fs.len(), vars.len(), elements, "Jacobian")
        }
        "divergence" => {
            let field = vector(graph, first)?;
            let vars = variables(graph, args.get(1), &field)?;
            if vars.len() != field.len() {
                return Err("the field and the variables need the same dimension".into());
            }
            let terms: Vec<ExprId> = field
                .iter()
                .zip(&vars)
                .map(|(&f, &v)| d(graph, f, v))
                .collect();
            Ok(NumericOutput::Expression(
                graph.simplify(graph.add(terms)),
                "divergence".into(),
            ))
        }
        "curl" => {
            let f = vector(graph, first)?;
            let vars = variables(graph, args.get(1), &f)?;
            match (f.len(), vars.len()) {
                (3, 3) => {
                    let parts = vec![
                        graph.sub(d(graph, f[2], vars[1]), d(graph, f[1], vars[2])),
                        graph.sub(d(graph, f[0], vars[2]), d(graph, f[2], vars[0])),
                        graph.sub(d(graph, f[1], vars[0]), d(graph, f[0], vars[1])),
                    ];
                    matrix_output(graph, 1, 3, parts, "curl")
                }
                (2, 2) => Ok(NumericOutput::Expression(
                    graph.simplify(graph.sub(d(graph, f[1], vars[0]), d(graph, f[0], vars[1]))),
                    "scalar curl".into(),
                )),
                _ => Err("curl needs a field and variables of dimension 3 (or 2)".into()),
            }
        }
        "laplacian" => {
            let f = parse(graph, first)?;
            let vars = variables(graph, args.get(1), &[f])?;
            let terms: Vec<ExprId> = vars.iter().map(|&v| d(graph, d(graph, f, v), v)).collect();
            Ok(NumericOutput::Expression(
                graph.simplify(graph.add(terms)),
                "Laplacian".into(),
            ))
        }
        "cross" | "dot" => {
            let a = vector(graph, first)?;
            let b = vector(graph, args.get(1).ok_or("two vectors are needed")?)?;
            if name == "dot" {
                if a.len() != b.len() {
                    return Err("the vectors need the same length".into());
                }
                let terms: Vec<ExprId> =
                    a.iter().zip(&b).map(|(&x, &y)| graph.mul([x, y])).collect();
                return Ok(NumericOutput::Expression(
                    graph.simplify(graph.add(terms)),
                    "dot product".into(),
                ));
            }
            if a.len() != 3 || b.len() != 3 {
                return Err("the cross product needs two vectors of length 3".into());
            }
            let component =
                |i: usize, j: usize| graph.sub(graph.mul([a[i], b[j]]), graph.mul([a[j], b[i]]));
            matrix_output(
                graph,
                1,
                3,
                vec![component(1, 2), component(2, 0), component(0, 1)],
                "cross product",
            )
        }
        "norm" => {
            let a = vector(graph, first)?;
            let squares: Vec<ExprId> = a.iter().map(|&x| graph.pow(x, graph.integer(2))).collect();
            let root = graph.function("sqrt", [graph.add(squares)]);
            Ok(NumericOutput::Expression(
                graph.simplify(root),
                "Euclidean norm".into(),
            ))
        }
        other => Err(format!("unknown vector calculus function {other}")),
    }
}
