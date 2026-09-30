//! # `urae-cli` (URAE Command Line Interface)
//!
//! Interactive REPL terminal application and processor for the Universal Rust Algebra Engine (URAE).
//!
//! ---
//!
//! ## Quick Start Tutorial
//!
//! ### 1. Launching the REPL
//! ```bash
//! cargo run -p urae-cli
//! ```
//! Terminal output:
//! ```text
//! Universal Rust Algebra Engine (URAE) v0.1.0
//! Type 'exit' or 'quit' to exit. Enter expressions to parse and evaluate.
//!
//! urae>
//! ```
//!
//! ---
//!
//! ## REPL Commands & Usage Guide
//!
//! - **Expression Parsing & Rendering**: `2 * x + 3` $\rightarrow$ `2 \cdot x + 3`
//! - **Permissive Equation Solving**: `solve g(x) = 3, x`, `given g(x) = 3 find x`, `find x where g(x) = 3`
//! - **Permissive Differentiation**: `diff x^3, x`, `differentiate x^3 wrt x`, `d/dx (x^3)`
//! - **Permissive Integration**: `integrate cos(x), x`, `integral of cos(x) wrt x`, `int cos(x) dx`
//! - **Esoteric Hyperoperations**: `tetration 2 3`, `knuth 2 2 3`, `ackermann 2 2`, `slog 2 16`
//! - **Tropical Semirings**: `maxplus_add 3 5`, `minplus_mul 3 5`, `log_sum_exp 3 5 0.01`
//! - **Number Theory**: `cf 1.41421356`, `is_prime 101`, `gcd 30 20`, `legendre 2 7`
//! - **Control & StatMech**: `stability 2x2 -2 1 0 -3`, `fermi_dirac 0.5 1.0 100.0`
//! - **E-Graph Simplification**: `simplify (x + 0) * 1` $\rightarrow$ `x`
//! - **Transformations**: `expand (x+1)^3`, `factor x^2-1`, `together 1/x + 1/y`, `cancel (x^2-1)/(x-1)`
//! - **Numerical Evaluation**: `eval sqrt(pi^2)` $\rightarrow$ `3.1415926535`
//! - **Variable & Parameter Inspection**: `vars`, `params`
//! - **AI Copilot Co-Reasoning**: `agent Explain how to differentiate x^2 + 3`
//! - **Formal Proof Export**: `proof x + 0` $\rightarrow$ Lean 4 & Coq proof script.

use algebra_advanced::agent::{AiConfig, AiProvider, AlgebraAgentInterface};
pub mod json_api;

use algebra_core::ExprGraph;
use algebra_core::format::LatexFormatter;
use algebra_core::parser::parse_operation;
use algebra_engine::executor::{ExecutionContext, OperationExecutor};
pub use json_api::{UraeJsonRequest, UraeJsonResponse, process_json_request, process_request};
use std::collections::HashMap;

/// Run the interactive CLI REPL session directly in the terminal.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_repl() {
    let default_level = default_logging_level();
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default_level));
    let (filter_layer, reload_handle) = tracing_subscriber::reload::Layer::new(filter);
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    let _ = tracing_subscriber::registry()
        .with(filter_layer)
        .with(tracing_subscriber::fmt::layer())
        .try_init();

    println!(
        "Universal Rust Algebra Engine (URAE) v{}",
        env!("CARGO_PKG_VERSION")
    );
    println!("Type 'exit' or 'quit' to exit. Enter expressions to parse and evaluate.\n");

    let graph = ExprGraph::new();
    let formatter = LatexFormatter;
    let mut ai_config = load_ai_config_from_env();

    use std::io::{self, BufRead, Write};
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    loop {
        print!("urae> ");
        if io::stdout().flush().is_err() {
            break;
        }

        let mut line = String::new();
        if handle.read_line(&mut line).is_err() || line.is_empty() {
            break;
        }

        let input = line.trim();
        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            println!("Goodbye!");
            break;
        }

        if input.starts_with('{') || input.starts_with('[') {
            let resp_json = process_json_request(&graph, input);
            println!("= {}\n", resp_json);
            continue;
        }

        if let Some(log_level) = input.strip_prefix("log ") {
            let level_str = log_level.trim();
            if let Ok(new_filter) = tracing_subscriber::EnvFilter::try_new(level_str) {
                let _ = reload_handle.reload(new_filter);
                println!("Logging verbosity set to '{}'\n", level_str);
            } else {
                eprintln!("Invalid log level. Usage: log <off|error|warn|info|debug|trace>\n");
            }
            continue;
        }

        match process_input_with_ai_config(&graph, &formatter, input, &mut ai_config) {
            Ok(result) => println!("= {}\n", result),
            Err(err) => eprintln!("Error: {}\n", err),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn default_logging_level() -> &'static str {
    if cfg!(test) {
        "warn"
    } else if cfg!(debug_assertions) {
        "debug"
    } else {
        "error"
    }
}

pub fn load_ai_config_from_env() -> AiConfig {
    let mut config = AiConfig::default();

    if let Ok(enabled_str) = std::env::var("URAE_AI_ENABLED") {
        config.enabled =
            enabled_str.trim().eq_ignore_ascii_case("true") || enabled_str.trim() == "1";
    }

    if let Ok(prov_str) = std::env::var("URAE_AI_PROVIDER") {
        match prov_str.trim().to_lowercase().as_str() {
            "googleai" | "google" | "gemini" => config.provider = AiProvider::GoogleAi,
            "anthropic" | "claude" => config.provider = AiProvider::Anthropic,
            "lmstudio" | "lm-studio" => config.provider = AiProvider::LmStudio,
            "opencode" => config.provider = AiProvider::OpenCode,
            "openrouter" => config.provider = AiProvider::OpenRouter,
            _ => {}
        }
    }

    if let Ok(key_str) = std::env::var("URAE_AI_API_KEY") {
        if !key_str.trim().is_empty() {
            config.api_key = Some(key_str);
        }
    }

    if let Ok(url_str) = std::env::var("URAE_AI_URL") {
        if !url_str.trim().is_empty() {
            config.access_url = Some(url_str);
        }
    }

    if let Ok(model_str) = std::env::var("URAE_AI_MODEL") {
        if !model_str.trim().is_empty() {
            config.model_name = Some(model_str);
        }
    }

    config
}

pub fn process_input(
    graph: &ExprGraph,
    formatter: &LatexFormatter,
    input: &str,
) -> Result<String, String> {
    let mut ai_config = load_ai_config_from_env();
    process_input_with_context(graph, formatter, input, &mut ai_config, None, None)
}

pub fn process_input_with_ai_config(
    graph: &ExprGraph,
    formatter: &LatexFormatter,
    input: &str,
    ai_config: &mut AiConfig,
) -> Result<String, String> {
    process_input_with_context(graph, formatter, input, ai_config, None, None)
}

pub fn process_input_with_context(
    graph: &ExprGraph,
    formatter: &LatexFormatter,
    input: &str,
    ai_config: &mut AiConfig,
    bindings: Option<&HashMap<String, f64>>,
    symbol_summary: Option<&[(String, String, f64, String)]>,
) -> Result<String, String> {
    let input = input.trim();
    let _ = formatter;

    // 1. Check for AI configuration queries
    if let Some(agent_cmd) = input
        .strip_prefix("agent ")
        .or_else(|| input.strip_prefix("ask "))
    {
        let prompt = agent_cmd.trim();

        if prompt.eq_ignore_ascii_case("status") {
            return Ok(ai_config.status_summary());
        }
        if prompt.eq_ignore_ascii_case("enable") {
            ai_config.enabled = true;
            return Ok(format!(
                "AI Copilot Enabled. Status: {}",
                ai_config.status_summary()
            ));
        }
        if prompt.eq_ignore_ascii_case("disable") {
            ai_config.enabled = false;
            return Ok("AI Copilot Disabled.".to_string());
        }

        if let Some(cfg_cmd) = prompt.strip_prefix("config ") {
            let parts: Vec<&str> = cfg_cmd.splitn(2, ' ').collect();
            if parts.len() == 2 {
                match parts[0].to_lowercase().as_str() {
                    "provider" => {
                        match parts[1].to_lowercase().as_str() {
                            "googleai" | "google" | "gemini" => ai_config.provider = AiProvider::GoogleAi,
                            "anthropic" | "claude" => ai_config.provider = AiProvider::Anthropic,
                            "lmstudio" | "lm-studio" => ai_config.provider = AiProvider::LmStudio,
                            "opencode" => ai_config.provider = AiProvider::OpenCode,
                            "openrouter" => ai_config.provider = AiProvider::OpenRouter,
                            _ => return Err("Supported providers: googleai, anthropic, lmstudio, opencode, openrouter".to_string()),
                        }
                        return Ok(format!(
                            "Provider set to {}. Status: {}",
                            ai_config.provider.name(),
                            ai_config.status_summary()
                        ));
                    }
                    "key" => {
                        ai_config.api_key = Some(parts[1].to_string());
                        return Ok(format!(
                            "API key updated. Status: {}",
                            ai_config.status_summary()
                        ));
                    }
                    "url" => {
                        ai_config.access_url = Some(parts[1].to_string());
                        return Ok(format!(
                            "Access URL updated to {}. Status: {}",
                            parts[1],
                            ai_config.status_summary()
                        ));
                    }
                    "model" => {
                        ai_config.model_name = Some(parts[1].to_string());
                        return Ok(format!(
                            "Model updated to {}. Status: {}",
                            parts[1],
                            ai_config.status_summary()
                        ));
                    }
                    _ => {
                        return Err(
                            "Usage: agent config <provider|key|url|model> <value>".to_string()
                        );
                    }
                }
            }
            return Err("Usage: agent config <provider|key|url|model> <value>".to_string());
        }

        let agent = AlgebraAgentInterface::new();
        return agent.query(ai_config, prompt, graph);
    }

    // 2. Neural Expression Embedding & AST Proof-Search Depth Commands
    if let Some(expr_str) = input.strip_prefix("embed ") {
        let parser = algebra_core::parser::ExprParser::new(graph);
        let id = parser
            .parse(expr_str.trim())
            .map_err(|e| format!("Parse error: {}", e))?;
        let embedder = algebra_engine::heuristic::NeuralExpressionEmbedder::default();
        let vec = embedder.embed_expression(graph, id);
        let depth = algebra_engine::heuristic::compute_ast_depth(graph, id);
        let nodes = algebra_engine::heuristic::count_ast_nodes(graph, id);
        let preview = vec
            .iter()
            .take(6)
            .map(|v| format!("{:+.4}", v))
            .collect::<Vec<_>>()
            .join(", ");
        let hex_sample: String = vec
            .iter()
            .take(4)
            .map(|v| format!("{:02X}", (v.abs() * 255.0) as u8))
            .collect();
        let mut out = String::new();
        out.push_str(&format!(
            "Neural Expression Embedding (dim: {}, L2-norm: 1.000):\n",
            vec.len()
        ));
        out.push_str(&format!(
            "  * AST Structure : depth = {}, total nodes = {}\n",
            depth, nodes
        ));
        out.push_str(&format!("  * Vector Preview: [{preview}, ...]\n"));
        out.push_str(&format!("  * Embedding Hash: 0x{hex_sample}"));
        return Ok(out);
    }

    if let Some(args_str) = input
        .strip_prefix("similarity ")
        .or_else(|| input.strip_prefix("embed_sim "))
    {
        let parts: Vec<&str> = args_str.split(',').collect();
        if parts.len() != 2 {
            return Err("Usage: similarity <expr1>, <expr2>".to_string());
        }
        let parser = algebra_core::parser::ExprParser::new(graph);
        let id1 = parser
            .parse(parts[0].trim())
            .map_err(|e| format!("Parse error in expr1: {}", e))?;
        let id2 = parser
            .parse(parts[1].trim())
            .map_err(|e| format!("Parse error in expr2: {}", e))?;
        let embedder = algebra_engine::heuristic::NeuralExpressionEmbedder::default();
        let v1 = embedder.embed_expression(graph, id1);
        let v2 = embedder.embed_expression(graph, id2);
        let sim = algebra_engine::heuristic::NeuralExpressionEmbedder::cosine_similarity(&v1, &v2);
        let quality = if sim > 0.90 {
            "Strong Semantic Equivalence"
        } else if sim > 0.50 {
            "Moderate Structural Similarity"
        } else {
            "Distinct Expression Geometry"
        };
        let mut out = String::new();
        out.push_str("Neural Semantic Equivalence Score:\n");
        out.push_str(&format!("  * Cosine Similarity: {:.6} ({})\n", sim, quality));
        out.push_str(&format!(
            "  * Expr 1 AST Depth : {}\n",
            algebra_engine::heuristic::compute_ast_depth(graph, id1)
        ));
        out.push_str(&format!(
            "  * Expr 2 AST Depth : {}",
            algebra_engine::heuristic::compute_ast_depth(graph, id2)
        ));
        return Ok(out);
    }

    if let Some(expr_str) = input
        .strip_prefix("proof_search ")
        .or_else(|| input.strip_prefix("proof-depth "))
        .or_else(|| input.strip_prefix("proof_depth "))
    {
        let parser = algebra_core::parser::ExprParser::new(graph);
        let id = parser
            .parse(expr_str.trim())
            .map_err(|e| format!("Parse error: {}", e))?;
        let search_engine =
            algebra_engine::heuristic::HeuristicSearchEngine::with_default_neural_guidance(
                algebra_core::EngineConfig::default(),
            );
        let indicator = search_engine.analyze_ast_proof_depth(graph, id);
        let emb_preview = indicator
            .neural_embedding
            .iter()
            .take(4)
            .map(|v| format!("{:+.3}", v))
            .collect::<Vec<_>>()
            .join(", ");
        let mut out = String::new();
        out.push_str(&format!(
            "AST Proof-Search Depth Analysis for `{}`:\n",
            expr_str.trim()
        ));
        out.push_str(&format!(
            "  * AST Tree Depth        : {}\n",
            indicator.ast_depth
        ));
        out.push_str(&format!(
            "  * AST Node Count        : {}\n",
            indicator.ast_node_count
        ));
        out.push_str(&format!(
            "  * Proof Search Depth    : {} / {}\n",
            indicator.proof_search_depth, indicator.max_depth_budget
        ));
        out.push_str(&format!(
            "  * Heuristic Candidate Wt: {:.4}\n",
            indicator.candidate_weight
        ));
        out.push_str(&format!(
            "  * Verification Status   : {}\n",
            indicator.certification_status
        ));
        out.push_str(&format!(
            "  * Neural Embedding [{}d]: [{emb_preview}, ...]",
            indicator.neural_embedding.len()
        ));
        return Ok(out);
    }

    // 3. Parse text into structured MathOperation
    let op = parse_operation(input);

    // 4. Prepare execution context
    let mut exec_ctx = ExecutionContext::new();
    if let Some(b) = bindings {
        exec_ctx.bindings = b.clone();
    }
    if let Some(s) = symbol_summary {
        exec_ctx.symbol_summary = s.to_vec();
    }

    // 5. Central execution via OperationExecutor
    let result = OperationExecutor::execute(graph, &op, &exec_ctx);
    if result.is_error {
        Err(result.error_msg.unwrap_or(result.output_text))
    } else {
        Ok(result.output_text)
    }
}

pub fn get_help_text() -> String {
    let mut help = String::new();
    help.push_str("Universal Rust Algebra Engine (URAE) REPL Commands:\n");
    help.push_str("  <expr>                     Parse and render mathematical expression\n");
    help.push_str(
        "  diff <expr>, <var>          Symbolic differentiation with respect to variable\n",
    );
    help.push_str("  integrate <expr>, <var>     Symbolic integration with respect to variable\n");
    help.push_str("  solve <eq>, <var>           Solve algebraic equation for target variable\n");
    help.push_str("  simplify <expr>            E-Graph pattern simplification\n");
    help.push_str("  expand / factor / together  Algebraic transformations\n");
    help.push_str(
        "  eval <expr>                Numerical reduction (uses notebook parameter values)\n",
    );
    help.push_str("  embed <expr>               Compute neural AST vector embedding\n");
    help.push_str("  similarity <e1>, <e2>      Compute cosine similarity of two expression embeddings\n");
    help.push_str("  proof_search <expr>        AST proof-search depth indicators and certification\n");
    help.push_str("  tetration / knuth / slog   Knuth up-arrows & hyperoperations\n");
    help.push_str("  maxplus_add / minplus_mul  Tropical semirings\n");
    help.push_str("  cf / is_prime / gcd        Computational number theory\n");
    help.push_str("  stability 2x2              Control system stability\n");
    help.push_str("  context / mathcontext       Inspect ambient mathematical context\n");
    help.push_str("  set <algebra|calculus|logic> <val>  Configure ambient mathematical system\n");
    help.push_str("  vars / params              List active notebook variables and parameters\n");
    help.push_str(
        "  agent <prompt> / ask        Ask AI Agent Copilot (Requires setup, off by default)\n",
    );
    help.push_str("  agent status / config      Configure AI copilot provider, key, and model\n");
    help.push_str("  preset <name>              Load preset math templates (quadratic, oscillator, calculus)\n");
    help.push_str(
        "  export <tex|py|lean>       Export session to LaTeX, Python SymPy, or Lean 4\n",
    );
    help.push_str("  log <off|error|warn|info|debug|trace>  Adjust live logging verbosity level\n");
    help.push_str("  proof <expr>               Export formal Lean 4 and Coq proof trace\n");
    help.push_str("  help                       Display this command guide\n");
    help.push_str("  exit / quit                Exit REPL session\n");
    help
}
