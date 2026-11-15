//! # `urae-wasm` (URAE Serverless Edge Engine for Cloudflare Workers & Pages)
//!
//! WebAssembly edge compilation targets providing near-zero cold start latency
//! execution of URAE symbolic differentiation, simplification, equation solving, and JSON API payloads.

use algebra_core::ExprGraph;
use algebra_core::format::{Formatter, LatexFormatter, UnicodeFormatter};
use algebra_core::parser::ExprParser;
use algebra_core::vm::{BytecodeVM, VmCompiler};
use algebra_engine::calculus::SymbolicCalculus;
use algebra_engine::executor::ExecutionContext;
use algebra_engine::ode::{NumericalOdeConfig, NumericalOdeMethod, NumericalOdeSolver};
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
            if !var_name.contains('(') && var_name.chars().all(|c| c.is_alphanumeric() || c == '_')
            {
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

/// Sample a 1D expression `y = f(var)` across `[x_min, x_max]` using BytecodeVM,
/// returning a JSON array of finite `[x, y]` coordinate pairs for inline web notebook plotting.
#[wasm_bindgen]
pub fn wasm_plot_sample_1d(
    expr_str: &str,
    var: &str,
    x_min: f64,
    x_max: f64,
    samples: usize,
) -> String {
    if !x_min.is_finite() || !x_max.is_finite() {
        return serde_json::json!({ "error": "x_min and x_max must be finite" }).to_string();
    }
    if samples == 0 {
        return "[]".to_string();
    }

    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let expr_id = match parser.parse(expr_str) {
        Ok(id) => id,
        Err(err) => return serde_json::json!({ "error": err.to_string() }).to_string(),
    };
    let program = match VmCompiler::compile(&graph, expr_id, var, &[]) {
        Ok(prog) => prog,
        Err(err) => return serde_json::json!({ "error": err.to_string() }).to_string(),
    };

    let vm = BytecodeVM::new();
    let mut points: Vec<[f64; 2]> = Vec::with_capacity(samples);
    let denom = if samples > 1 {
        (samples - 1) as f64
    } else {
        1.0
    };
    for i in 0..samples {
        let x = x_min + (x_max - x_min) * (i as f64) / denom;
        if let Ok(y) = vm.eval_scalar(&program, x, &[])
            && y.is_finite()
        {
            points.push([x, y]);
        }
    }

    serde_json::to_string(&points).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
}

/// Numerically solve a scalar 1st-order ODE `dy/dt = rhs(t, y)` on WebAssembly edge,
/// returning JSON with method metadata and `[t, y]` trajectory samples.
#[wasm_bindgen]
pub fn wasm_ode_solve_json(
    rhs_str: &str,
    t_var: &str,
    y_var: &str,
    t0: f64,
    t1: f64,
    y0: f64,
    method: &str,
) -> String {
    if !t0.is_finite() || !t1.is_finite() || !y0.is_finite() {
        return serde_json::json!({ "error": "t0, t1, and y0 must be finite" }).to_string();
    }

    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let rhs_id = match parser.parse(rhs_str) {
        Ok(id) => id,
        Err(err) => return serde_json::json!({ "error": err.to_string() }).to_string(),
    };

    let t_param = t_var.trim().to_string();
    let program = match VmCompiler::compile(&graph, rhs_id, y_var.trim(), &[t_param]) {
        Ok(prog) => prog,
        Err(err) => return serde_json::json!({ "error": err.to_string() }).to_string(),
    };

    let selected = match method.trim().to_ascii_lowercase().as_str() {
        "rk4" => NumericalOdeMethod::Rk4,
        "dopri5" | "dormand_prince" | "dormandprince54" => NumericalOdeMethod::DormandPrince54,
        "tsit5" | "tsitouras54" => NumericalOdeMethod::Tsitouras54,
        "radau" | "radau5" | "radauiia5" => NumericalOdeMethod::RadauIIA5,
        "meta" | "self_optimizing_meta" => NumericalOdeMethod::SelfOptimizingMeta,
        _ => NumericalOdeMethod::Auto,
    };

    let cfg = NumericalOdeConfig {
        method: selected,
        rtol: 1e-5,
        atol: 1e-7,
        ..Default::default()
    };

    let vm = BytecodeVM::new();
    let traj = match NumericalOdeSolver::solve(
        move |t, state| {
            let dy = vm.eval_scalar(&program, state[0], &[t]).unwrap_or(f64::NAN);
            vec![dy]
        },
        (t0, t1),
        &[y0],
        &cfg,
    ) {
        Ok(res) => res,
        Err(err) => return serde_json::json!({ "error": err.to_string() }).to_string(),
    };

    let points: Vec<[f64; 2]> = traj
        .t
        .iter()
        .zip(traj.y.iter())
        .map(|(&ti, yi)| [ti, yi[0]])
        .collect();
    let t_final = traj.t.last().copied().unwrap_or(t0);
    let y_final = traj.y.last().and_then(|v| v.first().copied()).unwrap_or(y0);

    serde_json::json!({
        "method": traj.method_used,
        "steps": traj.steps_accepted,
        "rejected": traj.steps_rejected,
        "t_final": t_final,
        "y_final": y_final,
        "points": points,
    })
    .to_string()
}

/// Web entry point for running URAE Notebook on a WebAssembly canvas (Cloudflare Pages / Workers).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn start_notebook_web(canvas_id: &str) -> Result<(), JsValue> {
    urae_notebook::start_web(canvas_id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_plot_sample_1d() {
        let json_str = wasm_plot_sample_1d("x^2 - 1", "x", -1.0, 1.0, 5);
        let pts: Vec<[f64; 2]> = serde_json::from_str(&json_str).expect("valid JSON points");
        assert_eq!(pts.len(), 5);
        assert!((pts[0][0] - (-1.0)).abs() < 1e-12);
        assert!(pts[0][1].abs() < 1e-12);
        assert!((pts[2][1] - (-1.0)).abs() < 1e-12);
        assert!(pts[4][1].abs() < 1e-12);
    }

    #[test]
    fn test_wasm_ode_solve_json_tsit5_and_meta() {
        for method in ["tsit5", "meta"] {
            let out = wasm_ode_solve_json("-y", "t", "y", 0.0, 1.0, 1.0, method);
            let parsed: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
            assert!(parsed.get("error").is_none(), "unexpected error: {parsed}");
            let y_final = parsed["y_final"].as_f64().expect("y_final");
            let expected = (-1.0_f64).exp();
            assert!(
                (y_final - expected).abs() < 1e-2,
                "method {method}: got {y_final}, expected {expected}"
            );
            assert!(parsed["steps"].as_u64().unwrap_or(0) > 0);
        }
    }
}
