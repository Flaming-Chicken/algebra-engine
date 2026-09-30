//! # JSON Request & Response Engine for URAE
//!
//! Provides structured JSON/TOML request and response payloads supporting nested LaTeX,
//! SymPy, raw mathematical formulas, CLI commands, parameter bindings, code generation,
//! formal proof trace exports, and AI copilot co-reasoning explanations.

use algebra_advanced::agent::{AiConfig, AlgebraAgentInterface};
use algebra_advanced::codegen::{CodeGenerator, TargetLanguage};
use algebra_advanced::proof::{ProofTrace, UnificationEngine};
use algebra_core::format::{Formatter, LatexFormatter, UnicodeFormatter};
use algebra_core::parser::ExprParser;
use algebra_core::{ExprGraph, ExprKind};
use algebra_engine::calculus::SymbolicCalculus;
use algebra_engine::numeric::{EvalContext, NumericalEval};
use algebra_engine::simplify::Simplifier;
use algebra_engine::solver::SymbolicSolver;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Structured request payload for nested LaTeX, commands, formulas, and parameter bindings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UraeJsonRequest {
    /// Command or formula text (e.g. "\\frac{x^2+1}{2}", "diff x^3, x", "solve x^2 - 4 = 0", "f(x) = a * x^2 + c")
    pub input: String,

    /// Optional parameter variable bindings e.g. {"a": 2.5, "c": -3.0}
    #[serde(default)]
    pub bindings: HashMap<String, f64>,

    /// Target variable for differentiation or solving (defaults to "x")
    #[serde(default)]
    pub target_variable: Option<String>,

    /// Explicit request flags
    #[serde(default)]
    pub compute_derivative: bool,
    #[serde(default)]
    pub compute_integral: bool,
    #[serde(default)]
    pub solve_equation: bool,
    #[serde(default)]
    pub simplify: bool,
    #[serde(default)]
    pub eval_numerical: bool,
    #[serde(default)]
    pub generate_code: bool,
    #[serde(default)]
    pub export_proof: bool,
    #[serde(default)]
    pub ai_explain: bool,
    #[serde(default)]
    pub embed_neural: bool,

    /// Optional AI configuration if ai_explain is requested
    #[serde(default)]
    pub ai_config: Option<AiConfig>,
}

/// Structured response payload with all mathematical outputs, formats, solutions, and code exports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UraeJsonResponse {
    pub success: bool,
    pub input_raw: String,
    pub output_latex: String,
    pub output_unicode: String,
    pub substituted_latex: Option<String>,
    pub derivative_latex: Option<String>,
    pub integral_latex: Option<String>,
    pub solutions: Option<Vec<String>>,
    pub eval_result: Option<f64>,
    pub domain_info: Option<String>,
    pub physical_unit: Option<String>,
    pub rust_code: Option<String>,
    pub python_code: Option<String>,
    pub lean4_proof: Option<String>,
    pub coq_proof: Option<String>,
    pub ai_explanation: Option<String>,
    #[serde(default)]
    pub ast_search_depth: Option<usize>,
    #[serde(default)]
    pub neural_embedding: Option<Vec<f32>>,
    pub error_msg: Option<String>,
}

/// Process a single `UraeJsonRequest` struct against `ExprGraph`.
pub fn process_request(graph: &ExprGraph, req: &UraeJsonRequest) -> UraeJsonResponse {
    let raw_input = req.input.trim();
    let target_var_name = req.target_variable.as_deref().unwrap_or("x");
    let target_sym = graph.symbols.get_or_intern(target_var_name);

    // Extract physical unit if bracketed e.g. "g = 9.81 [m/s^2]" or "\frac{x}{2} [m]"
    let mut unit_opt = None;
    let mut text_clean = raw_input;
    if let Some(start_b) = raw_input.find('[') {
        if let Some(end_b) = raw_input.find(']') {
            if end_b > start_b {
                unit_opt = Some(raw_input[start_b + 1..end_b].to_string());
                text_clean = raw_input[..start_b].trim();
            }
        }
    }

    // Check CLI command prefixes e.g. "diff ", "integrate ", "solve ", "simplify ", "eval "
    if text_clean.starts_with("diff ")
        || text_clean.starts_with("integrate ")
        || text_clean.starts_with("solve ")
        || text_clean.starts_with("solveset ")
        || text_clean.starts_with("simplify ")
        || text_clean.starts_with("eval ")
        || text_clean.starts_with("agent ")
        || text_clean == "help"
        || text_clean == "vars"
    {
        let mut ai_cfg = req.ai_config.clone().unwrap_or_default();
        let summary_vec: Vec<_> = req
            .bindings
            .iter()
            .map(|(k, &v)| (k.clone(), "Parameter".to_string(), v, String::new()))
            .collect();

        match crate::process_input_with_context(
            graph,
            &LatexFormatter,
            text_clean,
            &mut ai_cfg,
            Some(&req.bindings),
            Some(&summary_vec),
        ) {
            Ok(cli_out) => {
                return UraeJsonResponse {
                    success: true,
                    input_raw: raw_input.to_string(),
                    output_latex: cli_out.clone(),
                    output_unicode: cli_out,
                    substituted_latex: None,
                    derivative_latex: None,
                    integral_latex: None,
                    solutions: None,
                    eval_result: None,
                    domain_info: Some("CLI Command Processed".to_string()),
                    physical_unit: unit_opt,
                    rust_code: None,
                    python_code: None,
                    lean4_proof: None,
                    coq_proof: None,
                    ai_explanation: None,
                    ast_search_depth: None,
                    neural_embedding: None,
                    error_msg: None,
                };
            }
            Err(err) => {
                return UraeJsonResponse {
                    success: false,
                    input_raw: raw_input.to_string(),
                    output_latex: String::new(),
                    output_unicode: String::new(),
                    substituted_latex: None,
                    derivative_latex: None,
                    integral_latex: None,
                    solutions: None,
                    eval_result: None,
                    domain_info: None,
                    physical_unit: unit_opt,
                    rust_code: None,
                    python_code: None,
                    lean4_proof: None,
                    coq_proof: None,
                    ai_explanation: None,
                    ast_search_depth: None,
                    neural_embedding: None,
                    error_msg: Some(err),
                };
            }
        }
    }

    // Parse expression or equation
    let parser = ExprParser::new(graph);
    let expr_id = match parser.parse(text_clean) {
        Ok(id) => id,
        Err(err) => {
            return UraeJsonResponse {
                success: false,
                input_raw: raw_input.to_string(),
                output_latex: String::new(),
                output_unicode: String::new(),
                substituted_latex: None,
                derivative_latex: None,
                integral_latex: None,
                solutions: None,
                eval_result: None,
                domain_info: None,
                physical_unit: unit_opt,
                rust_code: None,
                python_code: None,
                lean4_proof: None,
                coq_proof: None,
                ai_explanation: None,
                ast_search_depth: None,
                neural_embedding: None,
                error_msg: Some(err.to_string()),
            };
        }
    };

    let latex_out = LatexFormatter.format(graph, expr_id).unwrap_or_default();
    let unicode_out = UnicodeFormatter.format(graph, expr_id).unwrap_or_default();

    let target_expr = match &graph.get(expr_id).kind {
        ExprKind::Relational { rhs, .. } => *rhs,
        _ => expr_id,
    };

    let domain_info = Some(format!("{}", graph.get(target_expr).domain));

    // Substitution with parameter bindings
    let mut substituted_latex = None;
    if !req.bindings.is_empty() {
        let mut subbed = expr_id;
        for (param, &val) in &req.bindings {
            if let Some(sym_id) = graph.symbols.get(param) {
                let val_node = graph.float(val);
                subbed = graph.substitute(subbed, sym_id, val_node);
            }
        }
        if subbed != expr_id {
            substituted_latex = LatexFormatter.format(graph, subbed).ok();
        }
    }

    // Derivative
    let diff_id = graph.diff(target_expr, target_sym);
    let derivative_latex = LatexFormatter.format(graph, diff_id).ok();

    // Integral
    let integral_latex = if req.compute_integral {
        if let Ok(int_id) = graph.integrate(target_expr, target_sym) {
            LatexFormatter.format(graph, int_id).ok()
        } else {
            None
        }
    } else {
        None
    };

    // Solutions for equations or if solve_equation is true
    let solutions =
        if req.solve_equation || matches!(graph.get(expr_id).kind, ExprKind::Relational { .. }) {
            if let Ok(sols) = graph.solveset(expr_id, target_sym) {
                let mut sol_strs = Vec::new();
                for s in sols {
                    sol_strs.push(LatexFormatter.format(graph, s).unwrap_or_default());
                }
                Some(sol_strs)
            } else {
                None
            }
        } else {
            None
        };

    // Numerical evaluation
    let eval_result = if req.eval_numerical || !req.bindings.is_empty() {
        let eval_ctx = EvalContext::with_bindings(53, req.bindings.clone());
        graph.evalf(expr_id, &eval_ctx).ok().map(|v| v.to_f64())
    } else {
        None
    };

    // Code generation
    let (rust_code, python_code) = if req.generate_code {
        let r_code = CodeGenerator::emit_code(graph, expr_id, TargetLanguage::Rust).ok();
        let p_code = CodeGenerator::emit_code(graph, expr_id, TargetLanguage::C).ok();
        (r_code, p_code)
    } else {
        (None, None)
    };

    // Proof trace exports
    let (lean4_proof, coq_proof) = if req.export_proof {
        let mut trace = ProofTrace::new();
        let engine = UnificationEngine::new();
        if let Ok(simp_id) = Simplifier::simplify(graph, expr_id) {
            if let Ok(_subst) = engine.unify(graph, expr_id, simp_id) {
                trace.add_step("algebraic_simplification", expr_id, simp_id);
            }
        }
        (Some(trace.export_lean4()), Some(trace.export_coq()))
    } else {
        (None, None)
    };

    // AI Copilot Explanation
    let ai_explanation = if req.ai_explain {
        if let Some(ai_cfg) = &req.ai_config {
            let agent = AlgebraAgentInterface::new();
            agent
                .query(ai_cfg, &format!("Explain: {}", text_clean), graph)
                .ok()
        } else {
            None
        }
    } else {
        None
    };

    // Neural Expression Embedding & AST Proof Search Depth
    let (ast_search_depth, neural_embedding) = if req.export_proof || req.simplify || req.embed_neural {
        let engine = algebra_engine::heuristic::HeuristicSearchEngine::with_default_neural_guidance(
            algebra_core::EngineConfig::default(),
        );
        let indicator = engine.analyze_ast_proof_depth(graph, expr_id);
        (Some(indicator.proof_search_depth), Some(indicator.neural_embedding))
    } else {
        (None, None)
    };

    UraeJsonResponse {
        success: true,
        input_raw: raw_input.to_string(),
        output_latex: latex_out,
        output_unicode: unicode_out,
        substituted_latex,
        derivative_latex,
        integral_latex,
        solutions,
        eval_result,
        domain_info,
        physical_unit: unit_opt,
        rust_code,
        python_code,
        lean4_proof,
        coq_proof,
        ai_explanation,
        ast_search_depth,
        neural_embedding,
        error_msg: None,
    }
}

/// Process a JSON request string and return a formatted JSON response string.
pub fn process_json_request(graph: &ExprGraph, json_req_str: &str) -> String {
    match serde_json::from_str::<UraeJsonRequest>(json_req_str) {
        Ok(req) => {
            let resp = process_request(graph, &req);
            serde_json::to_string_pretty(&resp).unwrap_or_default()
        }
        Err(_) => {
            // Check if input is a JSON array of requests
            if let Ok(reqs) = serde_json::from_str::<Vec<UraeJsonRequest>>(json_req_str) {
                let resps: Vec<_> = reqs.iter().map(|r| process_request(graph, r)).collect();
                return serde_json::to_string_pretty(&resps).unwrap_or_default();
            }

            // Fallback: treat raw string as a single input request
            let req = UraeJsonRequest {
                input: json_req_str.to_string(),
                bindings: HashMap::new(),
                target_variable: None,
                compute_derivative: false,
                compute_integral: false,
                solve_equation: false,
                simplify: false,
                eval_numerical: false,
                generate_code: false,
                export_proof: false,
                ai_explain: false,
                embed_neural: false,
                ai_config: None,
            };
            let resp = process_request(graph, &req);
            serde_json::to_string_pretty(&resp).unwrap_or_default()
        }
    }
}
