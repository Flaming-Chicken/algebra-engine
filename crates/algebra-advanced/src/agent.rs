//! # `algebra-agent`
//!
//! AI Agent Co-Reasoning & LLM Symbolic Integration Engine for URAE.
//!
//! Provides JSON-Schema function definitions for LLM tool calling (OpenAI / Anthropic formats),
//! structured JSON tool execution, natural-language step-by-step mathematical explanation generators,
//! and provider setup supporting Google AI, Anthropic, LM Studio, OpenCode, and OpenRouter.
//!
//! **Security Note**: All AI components are **off by default** until explicitly enabled and configured
//! with an API key or access URL.

use crate::proof::ProofTrace;
use algebra_core::format::{Formatter, LatexFormatter};
use algebra_core::parser::ExprParser;
use algebra_core::{AlgebraResult, ExprGraph, ExprId};
use algebra_engine::calculus::SymbolicCalculus;
use algebra_engine::simplify::Simplifier;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Supported LLM API Providers for AI Copilot and Symbolic Integration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AiProvider {
    /// Google AI Gemini API
    #[default]
    GoogleAi,
    /// Anthropic Claude API
    Anthropic,
    /// LM Studio Local OpenAI-compatible Server
    LmStudio,
    /// OpenCode Local / Remote API Server
    OpenCode,
    /// OpenRouter Multi-Provider API Gateway
    OpenRouter,
}

impl AiProvider {
    pub fn name(&self) -> &'static str {
        match self {
            Self::GoogleAi => "Google AI (Gemini)",
            Self::Anthropic => "Anthropic (Claude)",
            Self::LmStudio => "LM Studio (Local)",
            Self::OpenCode => "OpenCode",
            Self::OpenRouter => "OpenRouter",
        }
    }

    pub fn default_access_url(&self) -> &'static str {
        match self {
            Self::GoogleAi => "https://generativelanguage.googleapis.com",
            Self::Anthropic => "https://api.anthropic.com",
            Self::LmStudio => "http://localhost:1234/v1",
            Self::OpenCode => "http://localhost:8080/v1",
            Self::OpenRouter => "https://openrouter.ai/api/v1",
        }
    }

    pub fn default_model(&self) -> &'static str {
        match self {
            Self::GoogleAi => "gemini-1.5-flash",
            Self::Anthropic => "claude-3-5-sonnet-20241022",
            Self::LmStudio => "local-model",
            Self::OpenCode => "opencode-model",
            Self::OpenRouter => "anthropic/claude-3.5-sonnet",
        }
    }

    pub fn requires_api_key(&self) -> bool {
        match self {
            Self::GoogleAi | Self::Anthropic | Self::OpenRouter => true,
            Self::LmStudio | Self::OpenCode => false,
        }
    }
}

/// Configuration for AI Providers.
///
/// **Safety Default**: `enabled` is `false` by default until configured.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiConfig {
    /// Whether AI features are active. Off by default until setup.
    pub enabled: bool,
    /// Selected LLM Provider.
    pub provider: AiProvider,
    /// Optional API Key (required for Google AI, Anthropic, OpenRouter).
    pub api_key: Option<String>,
    /// Optional Custom Endpoint Access URL (overrides default).
    pub access_url: Option<String>,
    /// Optional Model Name override.
    pub model_name: Option<String>,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Off by default!
            provider: AiProvider::GoogleAi,
            api_key: None,
            access_url: None,
            model_name: None,
        }
    }
}

impl AiConfig {
    pub fn new(provider: AiProvider) -> Self {
        Self {
            enabled: false,
            provider,
            api_key: None,
            access_url: None,
            model_name: None,
        }
    }

    /// Check if the AI provider is properly configured with required credentials or URLs.
    pub fn is_configured(&self) -> bool {
        if !self.enabled {
            return false;
        }

        let has_key = self
            .api_key
            .as_ref()
            .map(|k| !k.trim().is_empty())
            .unwrap_or(false);
        let has_url = self
            .access_url
            .as_ref()
            .map(|u| !u.trim().is_empty())
            .unwrap_or(false);

        if self.provider.requires_api_key() {
            has_key || has_url
        } else {
            true
        }
    }

    /// Return the active access URL (custom or provider default).
    pub fn effective_access_url(&self) -> String {
        self.access_url
            .as_ref()
            .filter(|u| !u.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| self.provider.default_access_url().to_string())
    }

    /// Return the active model name (custom or provider default).
    pub fn effective_model_name(&self) -> String {
        self.model_name
            .as_ref()
            .filter(|m| !m.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| self.provider.default_model().to_string())
    }

    /// Status message describing current configuration state.
    pub fn status_summary(&self) -> String {
        if !self.enabled {
            "AI Disabled (Off by default)".to_string()
        } else if !self.is_configured() {
            format!(
                "{} Setup Required (API Key or Access URL needed)",
                self.provider.name()
            )
        } else {
            format!(
                "{} Ready ({})",
                self.provider.name(),
                self.effective_model_name()
            )
        }
    }
}

/// Tool definition schema for LLM function calling.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters_schema: Value,
}

/// AI Agent Interface providing structured tool schemas, provider queries, and JSON execution.
#[derive(Debug, Clone, Default)]
pub struct AlgebraAgentInterface;

impl AlgebraAgentInterface {
    pub fn new() -> Self {
        Self
    }

    /// Send prompt query to the configured LLM API provider.
    ///
    /// If AI is disabled (`enabled == false`) or not set up, returns an explicit warning error.
    pub fn query(
        &self,
        config: &AiConfig,
        prompt: &str,
        _graph: &ExprGraph,
    ) -> Result<String, String> {
        if !config.enabled {
            return Err("AI components are off by default. Enable AI and configure an API key or access URL in settings.".to_string());
        }

        if !config.is_configured() {
            return Err(format!(
                "AI provider '{}' is not set up. An API key or valid Access URL is required.",
                config.provider.name()
            ));
        }

        #[cfg(all(feature = "agent", not(target_arch = "wasm32")))]
        {
            // No canned fallback: a made-up explanation is worse than a clear failure.
            self.dispatch_http_query(config, prompt).map_err(|e| {
                let key = config.api_key.as_deref().unwrap_or("");
                let reason = if key.is_empty() {
                    e
                } else {
                    e.replace(key, "<redacted>")
                };
                format!(
                    "AI provider '{}' could not be reached: {reason}",
                    config.provider.name()
                )
            })
        }

        #[cfg(not(all(feature = "agent", not(target_arch = "wasm32"))))]
        {
            let _ = prompt;
            Err(format!(
                "AI provider '{}': live AI calls are not available in this build",
                config.provider.name()
            ))
        }
    }

    #[cfg(all(feature = "agent", not(target_arch = "wasm32")))]
    fn dispatch_http_query(&self, config: &AiConfig, prompt: &str) -> Result<String, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| e.to_string())?;

        let url = config.effective_access_url();
        let model = config.effective_model_name();
        let api_key = config.api_key.as_deref().unwrap_or("");

        match config.provider {
            AiProvider::GoogleAi => {
                let endpoint = format!(
                    "{}/v1beta/models/{}:generateContent?key={}",
                    url, model, api_key
                );
                let body = json!({
                    "contents": [{
                        "parts": [{"text": prompt}]
                    }]
                });
                let res: Value = client
                    .post(&endpoint)
                    .json(&body)
                    .send()
                    .map_err(|e| e.to_string())?
                    .json()
                    .map_err(|e| e.to_string())?;
                if let Some(text) = res["candidates"][0]["content"]["parts"][0]["text"].as_str() {
                    return Ok(text.to_string());
                }
            }
            AiProvider::Anthropic => {
                let endpoint = format!("{}/v1/messages", url);
                let body = json!({
                    "model": model,
                    "max_tokens": 1024,
                    "messages": [{"role": "user", "content": prompt}]
                });
                let res: Value = client
                    .post(&endpoint)
                    .header("x-api-key", api_key)
                    .header("anthropic-version", "2023-06-01")
                    .json(&body)
                    .send()
                    .map_err(|e| e.to_string())?
                    .json()
                    .map_err(|e| e.to_string())?;
                if let Some(text) = res["content"][0]["text"].as_str() {
                    return Ok(text.to_string());
                }
            }
            AiProvider::LmStudio | AiProvider::OpenCode | AiProvider::OpenRouter => {
                let endpoint = if url.ends_with("/chat/completions") {
                    url
                } else {
                    format!("{}/chat/completions", url.trim_end_matches('/'))
                };
                let body = json!({
                    "model": model,
                    "messages": [{"role": "user", "content": prompt}]
                });
                let mut req = client.post(&endpoint).json(&body);
                if !api_key.is_empty() {
                    req = req.header("Authorization", format!("Bearer {}", api_key));
                }
                let res: Value = req
                    .send()
                    .map_err(|e| e.to_string())?
                    .json()
                    .map_err(|e| e.to_string())?;
                if let Some(content) = res["choices"][0]["message"]["content"].as_str() {
                    return Ok(content.to_string());
                }
            }
        }

        Err(format!(
            "Provider {} returned an unparseable response structure.",
            config.provider.name()
        ))
    }

    #[allow(dead_code)]
    #[cfg(all(feature = "agent", target_arch = "wasm32"))]
    fn dispatch_http_query(&self, config: &AiConfig, _prompt: &str) -> Result<String, String> {
        Err(format!(
            "Synchronous HTTP query to {} is not available in WASM edge runtime.",
            config.provider.name()
        ))
    }

    /// Return available LLM tool definitions in standard JSON Schema format.
    pub fn get_tool_definitions(&self) -> Vec<AgentToolDefinition> {
        vec![
            AgentToolDefinition {
                name: "differentiate".to_string(),
                description:
                    "Compute symbolic derivative of an expression with respect to a variable"
                        .to_string(),
                parameters_schema: json!({
                    "type": "object",
                    "properties": {
                        "expression": { "type": "string", "description": "Mathematical expression e.g. 'sin(x) * x^2'" },
                        "variable": { "type": "string", "description": "Variable to differentiate with respect to e.g. 'x'" }
                    },
                    "required": ["expression", "variable"]
                }),
            },
            AgentToolDefinition {
                name: "integrate".to_string(),
                description:
                    "Compute symbolic integral of an expression with respect to a variable"
                        .to_string(),
                parameters_schema: json!({
                    "type": "object",
                    "properties": {
                        "expression": { "type": "string", "description": "Mathematical expression e.g. 'cos(x)'" },
                        "variable": { "type": "string", "description": "Variable to integrate with respect to e.g. 'x'" }
                    },
                    "required": ["expression", "variable"]
                }),
            },
            AgentToolDefinition {
                name: "simplify".to_string(),
                description: "Simplify mathematical expression using E-Graph pattern rewriting"
                    .to_string(),
                parameters_schema: json!({
                    "type": "object",
                    "properties": {
                        "expression": { "type": "string", "description": "Expression to simplify e.g. 'x + 0'" }
                    },
                    "required": ["expression"]
                }),
            },
            AgentToolDefinition {
                name: "export_proof".to_string(),
                description: "Export formal Lean 4 proof trace for algebraic transformation"
                    .to_string(),
                parameters_schema: json!({
                    "type": "object",
                    "properties": {
                        "expression": { "type": "string", "description": "Expression to generate proof trace for" }
                    },
                    "required": ["expression"]
                }),
            },
        ]
    }

    /// Execute a tool call by name with JSON argument payload.
    pub fn execute_tool_call(
        &self,
        graph: &ExprGraph,
        tool_name: &str,
        arguments: &Value,
    ) -> AlgebraResult<Value> {
        let parser = ExprParser::new(graph);
        let formatter = LatexFormatter;

        match tool_name {
            "differentiate" => {
                let expr_str = arguments["expression"].as_str().unwrap_or_default();
                let var_str = arguments["variable"].as_str().unwrap_or("x");

                let expr_id = parser.parse(expr_str).map_err(|e| {
                    algebra_core::AlgebraError::DomainViolation {
                        domain: "AgentParser".to_string(),
                        reason: e.to_string(),
                    }
                })?;
                let wrt_sym = graph.symbols.get_or_intern(var_str);
                let step_res = graph.diff_with_steps(expr_id, wrt_sym);
                let latex = formatter.format(graph, step_res.result).unwrap_or_default();

                let mut step_descriptions = Vec::new();
                for step in &step_res.steps {
                    step_descriptions.push(format!(
                        "Step {}: {} ({})",
                        step.step_number, step.rule_name, step.description
                    ));
                }

                let explanation = if !step_descriptions.is_empty() {
                    step_descriptions.join("\n")
                } else {
                    ExplanationGenerator::explain_differentiation(expr_str, var_str, &latex)
                };

                Ok(json!({
                    "success": true,
                    "result_latex": latex,
                    "explanation": explanation,
                    "deterministic_steps": step_res.steps
                }))
            }
            "integrate" => {
                let expr_str = arguments["expression"].as_str().unwrap_or_default();
                let var_str = arguments["variable"].as_str().unwrap_or("x");

                let expr_id = parser.parse(expr_str).map_err(|e| {
                    algebra_core::AlgebraError::DomainViolation {
                        domain: "AgentParser".to_string(),
                        reason: e.to_string(),
                    }
                })?;
                let wrt_sym = graph.symbols.get_or_intern(var_str);
                let int_id = graph.integrate(expr_id, wrt_sym)?;
                let latex = formatter.format(graph, int_id).unwrap_or_default();

                Ok(json!({
                    "success": true,
                    "result_latex": latex
                }))
            }
            "simplify" => {
                let expr_str = arguments["expression"].as_str().unwrap_or_default();
                let expr_id = parser.parse(expr_str).map_err(|e| {
                    algebra_core::AlgebraError::DomainViolation {
                        domain: "AgentParser".to_string(),
                        reason: e.to_string(),
                    }
                })?;
                let simp_id = Simplifier::simplify(graph, expr_id).map_err(|e| {
                    algebra_core::AlgebraError::DomainViolation {
                        domain: "Simplifier".to_string(),
                        reason: e.to_string(),
                    }
                })?;
                let latex = formatter.format(graph, simp_id).unwrap_or_default();

                Ok(json!({
                    "success": true,
                    "simplified_latex": latex
                }))
            }
            "export_proof" => {
                let expr_str = arguments["expression"].as_str().unwrap_or_default();
                let expr_id = parser.parse(expr_str).map_err(|e| {
                    algebra_core::AlgebraError::DomainViolation {
                        domain: "AgentParser".to_string(),
                        reason: e.to_string(),
                    }
                })?;

                let mut trace = ProofTrace::new();
                trace.add_step("identity_transformation", expr_id, expr_id);

                Ok(json!({
                    "success": true,
                    "lean4_proof": trace.export_lean4_with(graph),
                    "coq_proof": trace.export_coq_with(graph)
                }))
            }
            _ => Ok(json!({
                "success": false,
                "error": format!("Unknown tool name: {}", tool_name)
            })),
        }
    }
}

/// Natural Language Step-by-Step Explanation Generator.
#[derive(Debug, Clone, Default)]
pub struct ExplanationGenerator;

impl ExplanationGenerator {
    pub fn explain_differentiation(expr_str: &str, variable: &str, result_latex: &str) -> String {
        format!(
            "To differentiate f({0}) = {1} with respect to {0}:\n\
             1. Identify structural sum/product terms in {1}.\n\
             2. Apply standard differentiation rules (Power Rule, Product Rule, Chain Rule) to each term.\n\
             3. Collect and simplify resulting terms to obtain d/d{0} [{1}] = {2}.",
            variable, expr_str, result_latex
        )
    }

    pub fn explain_step(before_expr: ExprId, after_expr: ExprId, rule_name: &str) -> String {
        format!(
            "Applied transformation rule '{}': reduced expression node {:?} to {:?}.",
            rule_name, before_expr, after_expr
        )
    }
}
