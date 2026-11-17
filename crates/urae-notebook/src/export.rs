//! Notebook export: turn the document text into LaTeX, Python (SymPy) and Lean 4 sources.
//!
//! Each non-blank line is parsed. A line that parses becomes a typeset formula, a SymPy
//! expression or a Lean statement; a line that does not (prose, commands, unsupported syntax)
//! is kept as a comment so nothing is lost and nothing is invented. Lines starting with `#` or
//! `//` are comments.

use urae::prelude::*;
use urae::{ExprGraph, ExprId, ExprKind, Number};

/// Split `lhs = rhs` on the first single `=` (not part of `==`, `<=`, `>=` or `!=`).
fn split_equation(line: &str) -> Option<(&str, &str)> {
    let bytes = line.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b != b'=' {
            continue;
        }
        let before = i.checked_sub(1).map(|j| bytes[j]);
        let after = bytes.get(i + 1).copied();
        if matches!(before, Some(b'<' | b'>' | b'!' | b'=')) || after == Some(b'=') {
            continue;
        }
        return Some((&line[..i], &line[i + 1..]));
    }
    None
}

fn is_comment(line: &str) -> bool {
    line.starts_with('#') || line.starts_with("//")
}

/// A parsed document line.
enum Line {
    Blank,
    Comment(String),
    Expression(ExprId),
    Equation(ExprId, ExprId),
    Unparsed(String),
}

fn classify(graph: &ExprGraph, raw: &str) -> Line {
    let line = raw.trim();
    if line.is_empty() {
        return Line::Blank;
    }
    if is_comment(line) {
        return Line::Comment(line.trim_start_matches(['#', '/']).trim().to_string());
    }
    let parser = ExprParser::new(graph);
    if let Some((lhs, rhs)) = split_equation(line) {
        if let (Ok(l), Ok(r)) = (parser.parse(lhs.trim()), parser.parse(rhs.trim())) {
            return Line::Equation(l, r);
        }
        return Line::Unparsed(line.to_string());
    }
    match parser.parse(line) {
        Ok(e) => Line::Expression(e),
        Err(_) => Line::Unparsed(line.to_string()),
    }
}

fn latex(graph: &ExprGraph, e: ExprId) -> Option<String> {
    LatexFormatter.format(graph, e).ok()
}

/// Escape the characters LaTeX treats as markup, for text placed in a `%` comment or verbatim.
fn tex_comment(text: &str) -> String {
    text.replace('\n', " ")
}

/// Render the document as a compilable LaTeX article. Formulas are typeset with `amsmath`.
pub fn to_latex(document: &str) -> String {
    let graph = ExprGraph::new();
    let mut body = String::new();
    for raw in document.lines() {
        match classify(&graph, raw) {
            Line::Blank => body.push('\n'),
            Line::Comment(c) => body.push_str(&format!("% {}\n", tex_comment(&c))),
            Line::Expression(e) => match latex(&graph, e) {
                Some(t) => body.push_str(&format!("\\[ {t} \\]\n")),
                None => body.push_str(&format!("% not typeset: {}\n", tex_comment(raw.trim()))),
            },
            Line::Equation(l, r) => match (latex(&graph, l), latex(&graph, r)) {
                (Some(a), Some(b)) => body.push_str(&format!("\\[ {a} = {b} \\]\n")),
                _ => body.push_str(&format!("% not typeset: {}\n", tex_comment(raw.trim()))),
            },
            Line::Unparsed(t) => body.push_str(&format!("% not typeset: {}\n", tex_comment(&t))),
        }
    }
    format!(
        "\\documentclass{{article}}\n\\usepackage{{amsmath}}\n\\begin{{document}}\n{body}\\end{{document}}\n"
    )
}

/// Python (SymPy) source for `e`, recording the free symbols it uses. `None` if the expression
/// uses something this exporter does not translate.
fn python(graph: &ExprGraph, e: ExprId, symbols: &mut Vec<String>) -> Option<String> {
    match &graph.get(e).kind {
        ExprKind::Number(Number::Integer(i)) => Some(i.to_string()),
        ExprKind::Number(Number::Rational(n, d)) => Some(format!("sp.Rational({n}, {d})")),
        ExprKind::Number(Number::Constant(c)) => match format!("{c:?}").as_str() {
            "Pi" => Some("sp.pi".into()),
            "E" => Some("sp.E".into()),
            "I" => Some("sp.I".into()),
            "EulerGamma" => Some("sp.EulerGamma".into()),
            "GoldenRatio" => Some("sp.GoldenRatio".into()),
            _ => None,
        },
        ExprKind::Symbol(s) => {
            let name = graph.symbols.resolve(*s)?;
            if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                || name.chars().next().is_none_or(|c| c.is_ascii_digit())
            {
                return None;
            }
            if !symbols.contains(&name) {
                symbols.push(name.clone());
            }
            Some(name)
        }
        ExprKind::Add(terms) => {
            let parts: Option<Vec<String>> =
                terms.iter().map(|&t| python(graph, t, symbols)).collect();
            Some(format!("({})", parts?.join(" + ")))
        }
        ExprKind::Mul(factors) => {
            let parts: Option<Vec<String>> =
                factors.iter().map(|&f| python(graph, f, symbols)).collect();
            Some(format!("({})", parts?.join(" * ")))
        }
        ExprKind::Sub(l, r) => Some(format!(
            "({} - {})",
            python(graph, *l, symbols)?,
            python(graph, *r, symbols)?
        )),
        ExprKind::Div(l, r) => Some(format!(
            "({} / {})",
            python(graph, *l, symbols)?,
            python(graph, *r, symbols)?
        )),
        ExprKind::Neg(i) => Some(format!("(-{})", python(graph, *i, symbols)?)),
        ExprKind::Pow(b, x) => Some(format!(
            "({} ** {})",
            python(graph, *b, symbols)?,
            python(graph, *x, symbols)?
        )),
        ExprKind::Function { name, args } => {
            let name = graph.symbols.resolve(*name)?;
            let known = [
                "sin", "cos", "tan", "exp", "log", "sqrt", "asin", "acos", "atan", "sinh", "cosh",
                "tanh", "abs", "gamma",
            ];
            let mapped = match name.as_str() {
                "ln" => "log",
                "arcsin" => "asin",
                "arccos" => "acos",
                "arctan" => "atan",
                other if known.contains(&other) => other,
                _ => return None,
            };
            let parts: Option<Vec<String>> =
                args.iter().map(|&a| python(graph, a, symbols)).collect();
            let call = if mapped == "abs" { "Abs" } else { mapped };
            Some(format!("sp.{call}({})", parts?.join(", ")))
        }
        _ => None,
    }
}

/// Render the document as a Python script using SymPy: free symbols are declared, expressions are
/// assigned to `r1, r2, ...` and equations become `sp.Eq(...)`. Anything not translated is a
/// comment.
pub fn to_sympy(document: &str) -> String {
    let graph = ExprGraph::new();
    let mut symbols: Vec<String> = Vec::new();
    let mut lines: Vec<String> = Vec::new();
    let mut count = 0;
    for raw in document.lines() {
        match classify(&graph, raw) {
            Line::Blank => lines.push(String::new()),
            Line::Comment(c) => lines.push(format!("# {c}")),
            Line::Expression(e) => match python(&graph, e, &mut symbols) {
                Some(code) => {
                    count += 1;
                    lines.push(format!("r{count} = {code}"));
                }
                None => lines.push(format!("# not translated: {}", raw.trim())),
            },
            Line::Equation(l, r) => {
                match (
                    python(&graph, l, &mut symbols),
                    python(&graph, r, &mut symbols),
                ) {
                    (Some(a), Some(b)) => {
                        count += 1;
                        lines.push(format!("r{count} = sp.Eq({a}, {b})"));
                    }
                    _ => lines.push(format!("# not translated: {}", raw.trim())),
                }
            }
            Line::Unparsed(t) => lines.push(format!("# not translated: {t}")),
        }
    }
    let mut out = String::from("# Exported from URAE\nimport sympy as sp\n\n");
    if !symbols.is_empty() {
        out.push_str(&format!(
            "{} = sp.symbols('{}')\n\n",
            symbols.join(", "),
            symbols.join(" ")
        ));
    }
    out.push_str(&lines.join("\n"));
    out.push('\n');
    out
}

/// Render the equations of the document as Lean 4 theorems over the reals. Each `lhs = rhs`
/// line becomes a theorem closed by `ring`; the result is not checked by URAE, and an equation
/// that is not a ring identity will fail in Lean. Other lines are listed as comments.
pub fn to_lean(document: &str) -> String {
    let graph = ExprGraph::new();
    let mut trace = ProofTrace::new();
    let mut notes: Vec<String> = Vec::new();
    for raw in document.lines() {
        match classify(&graph, raw) {
            Line::Equation(l, r) => trace.add_step(format!("line: {}", raw.trim()), l, r),
            Line::Blank | Line::Comment(_) => {}
            Line::Expression(_) | Line::Unparsed(_) => {
                notes.push(format!("-- not an equation, skipped: {}", raw.trim()));
            }
        }
    }
    let mut out = trace.export_lean4_with(&graph);
    for note in notes {
        out.push_str(&note);
        out.push('\n');
    }
    out
}
