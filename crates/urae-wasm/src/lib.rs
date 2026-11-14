//! # `urae-wasm` (URAE Serverless Edge Engine for Cloudflare Workers & Pages)
//!
//! WebAssembly edge compilation targets providing near-zero cold start latency
//! execution of URAE symbolic differentiation, simplification, equation solving, and JSON API payloads.

use algebra_core::ExprGraph;
use algebra_core::format::{Formatter, LatexFormatter, UnicodeFormatter};
use algebra_core::parser::ExprParser;
use algebra_engine::calculus::SymbolicCalculus;
use algebra_engine::executor::ExecutionContext;
use algebra_engine::simplify::Simplifier;
use algebra_engine::solver::SymbolicSolver;
use algebra_engine::wolfram::WolframAlphaEngine;
use wasm_bindgen::prelude::*;

/// Natural Language Wolfram Alpha multi-pod query engine on WebAssembly edge.
/// Returns JSON-encoded `WolframAlphaResponse` containing all structured pods (Input, Result,
/// Steps, Numerical, Alternate forms, Derivatives, Integrals, Roots).
#[wasm_bindgen]
pub fn wasm_wolfram_query(query_str: &str) -> String {
    let graph = ExprGraph::new();
    let ctx = ExecutionContext::new();
    let resp = WolframAlphaEngine::query(&graph, query_str, &ctx);
    serde_json::to_string(&resp).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
}

/// Evaluates a line in the Rust-extended Pad, returning rich formatted JSON with
/// exact LaTeX, 2D Unicode, plain text, and any generated Wolfram pods.
#[wasm_bindgen]
pub fn wasm_pad_eval_line(line_str: &str) -> String {
    let graph = ExprGraph::new();
    let ctx = ExecutionContext::new();
    let trimmed = line_str.trim();
    if trimmed.is_empty() {
        return "{}".to_string();
    }
    let resp = WolframAlphaEngine::query(&graph, trimmed, &ctx);
    serde_json::to_string(&resp).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
}

/// Evaluates a multi-line Rust-extended mathematical pad session.
/// Takes raw pad text, processes each line, maintaining variable definitions and returning an array of evaluated lines.
#[wasm_bindgen]
pub fn wasm_pad_eval_session(session_text: &str) -> String {
    let graph = ExprGraph::new();
    let mut ctx = ExecutionContext::new();
    let mut results = Vec::new();

    for line in session_text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
            results.push(serde_json::json!({
                "raw": line,
                "is_comment_or_empty": true,
                "text": "",
                "latex": "",
                "unicode": "",
            }));
            continue;
        }

        // Check if line is a variable or parameter assignment, e.g., a = 5.0 or x = 2
        if let Some((lhs, rhs)) = trimmed.split_once('=') {
            let var_name = lhs.trim();
            let val_str = rhs.trim().trim_end_matches(';');
            if !var_name.contains('(') && var_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                if let Ok(val) = val_str.parse::<f64>() {
                    ctx.bindings.insert(var_name.to_string(), val);
                }
            }
        }

        let resp = WolframAlphaEngine::query(&graph, trimmed, &ctx);
        let result_pod = resp.pods.iter().find(|p| p.title == "Result");
        let error_pod = resp.pods.iter().find(|p| p.title == "Error");

        if let Some(res) = result_pod {
            results.push(serde_json::json!({
                "raw": line,
                "is_error": false,
                "text": res.text,
                "latex": res.latex,
                "unicode": res.unicode,
                "pods": resp.pods,
                "execution_time_us": resp.execution_time_us,
            }));
        } else if let Some(err) = error_pod {
            results.push(serde_json::json!({
                "raw": line,
                "is_error": true,
                "text": err.text,
                "latex": err.latex,
                "unicode": err.unicode,
                "pods": resp.pods,
                "execution_time_us": resp.execution_time_us,
            }));
        } else {
            let first = resp.pods.first();
            results.push(serde_json::json!({
                "raw": line,
                "is_error": false,
                "text": first.map(|p| p.text.as_str()).unwrap_or_default(),
                "latex": first.map(|p| p.latex.as_str()).unwrap_or_default(),
                "unicode": first.map(|p| p.unicode.as_str()).unwrap_or_default(),
                "pods": resp.pods,
                "execution_time_us": resp.execution_time_us,
            }));
        }
    }

    serde_json::to_string(&results).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
}

/// Process a JSON request string on Cloudflare Workers / Pages edge and return structured JSON response.
#[wasm_bindgen]
pub fn wasm_process_json(json_req_str: &str) -> String {
    let graph = ExprGraph::new();
    urae_cli::process_json_request(&graph, json_req_str)
}

/// Execute any natural language query or mathematical operation on Cloudflare Workers / Pages edge.
#[wasm_bindgen]
pub fn wasm_execute_command(command_str: &str) -> String {
    let graph = ExprGraph::new();
    let op = algebra_core::parser::parse_operation(command_str);
    let ctx = algebra_engine::executor::ExecutionContext::new();
    let result = algebra_engine::executor::OperationExecutor::execute(&graph, &op, &ctx);
    result.output_text
}

/// Differentiate mathematical or LaTeX expression on edge.
#[wasm_bindgen]
pub fn wasm_differentiate(expr_str: &str, wrt_str: &str) -> String {
    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let wrt_sym = graph.symbols.get_or_intern(wrt_str);

    match parser.parse(expr_str) {
        Ok(expr_id) => {
            let diff_id = graph.diff(expr_id, wrt_sym);
            LatexFormatter.format(&graph, diff_id).unwrap_or_default()
        }
        Err(err) => format!("Error: {}", err),
    }
}

/// Integrate mathematical or LaTeX expression on edge.
#[wasm_bindgen]
pub fn wasm_integrate(expr_str: &str, wrt_str: &str) -> String {
    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let wrt_sym = graph.symbols.get_or_intern(wrt_str);

    match parser.parse(expr_str) {
        Ok(expr_id) => match graph.integrate(expr_id, wrt_sym) {
            Ok(int_id) => LatexFormatter.format(&graph, int_id).unwrap_or_default(),
            Err(err) => format!("Integration error: {}", err),
        },
        Err(err) => format!("Error: {}", err),
    }
}

/// Solve equation on edge.
#[wasm_bindgen]
pub fn wasm_solve(eq_str: &str, wrt_str: &str) -> String {
    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let target_sym = graph.symbols.get_or_intern(wrt_str);

    match parser.parse(eq_str) {
        Ok(eq_id) => match graph.solveset(eq_id, target_sym) {
            Ok(sols) => {
                let mut sol_strs = Vec::new();
                for s in sols {
                    sol_strs.push(LatexFormatter.format(&graph, s).unwrap_or_default());
                }
                format!("Solutions for {}: [{}]", wrt_str, sol_strs.join(", "))
            }
            Err(err) => format!("Solve error: {}", err),
        },
        Err(err) => format!("Error: {}", err),
    }
}

/// Simplify expression using E-Graph pattern saturation on edge.
#[wasm_bindgen]
pub fn wasm_simplify(expr_str: &str) -> String {
    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);

    match parser.parse(expr_str) {
        Ok(expr_id) => match Simplifier::simplify(&graph, expr_id) {
            Ok(simp_id) => LatexFormatter.format(&graph, simp_id).unwrap_or_default(),
            Err(err) => format!("Simplify error: {}", err),
        },
        Err(err) => format!("Error: {}", err),
    }
}

/// Convert input to LaTeX output string.
#[wasm_bindgen]
pub fn wasm_format_latex(input_str: &str) -> String {
    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    match parser.parse(input_str) {
        Ok(expr_id) => LatexFormatter.format(&graph, expr_id).unwrap_or_default(),
        Err(err) => format!("Error: {}", err),
    }
}

/// Convert input to 2D/1D Unicode output string.
#[wasm_bindgen]
pub fn wasm_format_unicode(input_str: &str) -> String {
    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    match parser.parse(input_str) {
        Ok(expr_id) => UnicodeFormatter.format(&graph, expr_id).unwrap_or_default(),
        Err(err) => format!("Error: {}", err),
    }
}

/// Web entry point for running URAE Notebook on a WebAssembly canvas (Cloudflare Pages / Workers).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn start_notebook_web(canvas_id: &str) -> Result<(), JsValue> {
    urae_notebook::start_web(canvas_id).await
}
