//! # `algebra_engine::session`
//!
//! Centralized Execution Session and Workspace State Manager for URAE.
//!
//! Owns the `ExprGraph`, symbol bindings, declared domains, symbol roles,
//! user-defined functions, physical units, and ambient mathematical context.
//! Serves as the primary execution runtime across CLI, LSP, Jupyter Kernel,
//! Python bindings, and Notebook interfaces.

use crate::executor::{ExecutionContext, OperationExecutor, OperationResult};
use crate::numeric::{EvalContext, NumericalEval};
use algebra_core::domain::{DomainBound, MathContext};
use algebra_core::operation::{MathOperation, SymbolRoleKind};
use algebra_core::parser::{parse_domain_declaration, parse_operation};
use algebra_core::{ExprGraph, ExprId, ExprKind};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Rich metadata for a declared symbol in the active session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SymbolSessionMeta {
    pub name: String,
    pub role: SymbolRoleKind,
    pub cur_val: f64,
    pub min_val: f64,
    pub max_val: f64,
    pub domain_type: String,
    pub unit_str: Option<String>,
}

impl SymbolSessionMeta {
    pub fn new(name: impl Into<String>, val: f64, role: SymbolRoleKind) -> Self {
        let n = name.into();
        Self {
            name: n,
            role,
            cur_val: val,
            min_val: val - 10.0,
            max_val: val + 10.0,
            domain_type: "Reals".to_string(),
            unit_str: None,
        }
    }

    pub fn clamp(&self, val: f64) -> f64 {
        if !val.is_finite() {
            return self.cur_val;
        }
        val.clamp(self.min_val, self.max_val)
    }
}

/// Centralized stateful execution engine session for URAE.
#[derive(Debug)]
pub struct UraeSession {
    /// Arena expression DAG.
    pub graph: ExprGraph,
    /// Numerical symbol bindings (for evaluation & slider scrubbing).
    pub bindings: HashMap<String, f64>,
    /// Declared domain bounds per symbol.
    pub symbol_domains: HashMap<String, DomainBound>,
    /// Rich metadata for symbols (role, value, unit, domain).
    pub symbol_metadata: HashMap<String, SymbolSessionMeta>,
    /// User-defined function definitions `name -> (args, body)`.
    pub user_functions: HashMap<String, (Vec<String>, String)>,
    /// Ambient mathematical context.
    pub context: MathContext,
    /// Document line history or command execution trace.
    pub history: Vec<String>,
}

impl Default for UraeSession {
    fn default() -> Self {
        Self::new()
    }
}

impl UraeSession {
    /// Create a new clean URAE execution session.
    pub fn new() -> Self {
        Self {
            graph: ExprGraph::new(),
            bindings: HashMap::new(),
            symbol_domains: HashMap::new(),
            symbol_metadata: HashMap::new(),
            user_functions: HashMap::new(),
            context: MathContext::default(),
            history: Vec::new(),
        }
    }

    /// Clear all variable bindings, domains, functions, and reset the expression graph.
    pub fn reset(&mut self) {
        self.graph = ExprGraph::new();
        self.bindings.clear();
        self.symbol_domains.clear();
        self.symbol_metadata.clear();
        self.user_functions.clear();
        self.context = MathContext::default();
        self.history.clear();
    }

    /// Set or update a symbol's numerical binding.
    pub fn set_binding(&mut self, name: impl Into<String>, val: f64) {
        let name_str = name.into();
        self.bindings.insert(name_str.clone(), val);
        if let Some(meta) = self.symbol_metadata.get_mut(&name_str) {
            meta.cur_val = meta.clamp(val);
        } else {
            self.symbol_metadata.insert(
                name_str.clone(),
                SymbolSessionMeta::new(&name_str, val, SymbolRoleKind::Parameter),
            );
        }
    }

    /// Retrieve a symbol's active numerical binding value.
    pub fn get_binding(&self, name: &str) -> Option<f64> {
        self.bindings.get(name).copied()
    }

    /// Register a domain constraint for a symbol.
    pub fn set_domain(&mut self, bound: DomainBound) {
        let name = bound.name.clone();
        if let Some(meta) = self.symbol_metadata.get_mut(&name) {
            meta.domain_type = bound.domain_type.clone();
            if let Some(min) = bound.min_val {
                meta.min_val = min;
            }
            if let Some(max) = bound.max_val {
                meta.max_val = max;
            }
            meta.cur_val = meta.clamp(meta.cur_val);
            self.bindings.insert(name.clone(), meta.cur_val);
        }
        self.symbol_domains.insert(name, bound);
    }

    /// Get declared domain constraint for a symbol.
    pub fn get_domain(&self, name: &str) -> Option<&DomainBound> {
        self.symbol_domains.get(name)
    }

    /// Build an `ExecutionContext` populated with current bindings and symbol summary.
    pub fn execution_context(&self) -> ExecutionContext {
        let mut ctx = ExecutionContext::with_bindings(self.bindings.clone());
        let mut summary = Vec::new();
        for (name, meta) in &self.symbol_metadata {
            summary.push((
                name.clone(),
                format!("{:?}", meta.role),
                meta.cur_val,
                meta.unit_str.clone().unwrap_or_default(),
            ));
        }
        summary.sort_by(|a, b| a.0.cmp(&b.0));
        ctx.symbol_summary = summary;
        ctx
    }

    /// Parse and execute a single line of input against this session, updating state.
    pub fn execute_line(&mut self, input: &str) -> OperationResult {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return OperationResult::success("", "", "");
        }

        self.history.push(trimmed.to_string());

        // Check for domain declaration e.g. `a in Reals [0, 100]`
        if let Some(bound) = parse_domain_declaration(trimmed) {
            self.set_domain(bound.clone());
            let op = MathOperation::DomainRestriction(bound);
            let ctx = self.execution_context();
            return OperationExecutor::execute(&self.graph, &op, &ctx);
        }

        // Check for explicit parameter assignment `param = val`
        if let Some((name, val_str)) = trimmed.split_once('=') {
            let name = name.trim();
            let val_str = val_str.trim();
            if !name.contains(':')
                && !name.contains('(')
                && name.chars().all(|c| c.is_alphanumeric() || c == '_')
            {
                let (num_str, unit_opt) = if let Some(bracket_idx) = val_str.find('[') {
                    let u = val_str[bracket_idx..]
                        .trim_matches(|c| c == '[' || c == ']')
                        .trim();
                    (&val_str[..bracket_idx], Some(u.to_string()))
                } else {
                    (val_str, None)
                };

                if let Ok(f) = num_str.trim().parse::<f64>() {
                    self.set_binding(name, f);
                    if let Some(u) = unit_opt
                        && let Some(meta) = self.symbol_metadata.get_mut(name)
                    {
                        meta.unit_str = Some(u);
                    }
                    let meta = self.symbol_metadata.get(name).unwrap();
                    let unit_suffix = meta
                        .unit_str
                        .as_ref()
                        .map(|u| format!(" [{}]", u))
                        .unwrap_or_default();
                    let out = format!("{} = {:.4}{}", name, meta.cur_val, unit_suffix);
                    return OperationResult::success(out.clone(), out.clone(), out);
                }
            }
        }

        // Check for function definition e.g. `f(x) = x^2 + 1`
        if let Some((lhs, rhs)) = trimmed.split_once('=') {
            let lhs = lhs.trim();
            if let Some(open_p) = lhs.find('(')
                && let Some(close_p) = lhs.find(')')
                && close_p > open_p
            {
                let fn_name = lhs[..open_p].trim().to_string();
                let args: Vec<String> = lhs[open_p + 1..close_p]
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if !fn_name.is_empty() {
                    self.user_functions
                        .insert(fn_name.clone(), (args.clone(), rhs.trim().to_string()));
                    let out = format!("{}({}) = {}", fn_name, args.join(", "), rhs.trim());
                    return OperationResult::success(out.clone(), out.clone(), out);
                }
            }
        }

        // Expand user functions in expression before parsing operation (ensuring word boundaries)
        let mut expanded_input = trimmed.to_string();
        for (fn_name, (args, body)) in &self.user_functions {
            let p_start = format!("{}(", fn_name);
            let mut search_idx = 0;
            while let Some(rel_pos) = expanded_input[search_idx..].find(&p_start) {
                let pos = search_idx + rel_pos;
                let is_word_boundary = pos == 0 || {
                    let prev = expanded_input[..pos].chars().last().unwrap();
                    !prev.is_alphanumeric() && prev != '_'
                };

                if is_word_boundary
                    && let Some(close) = expanded_input[pos..].find(')')
                    && args.len() == 1
                {
                    let actual_arg = &expanded_input[pos + p_start.len()..pos + close];
                    let replaced_body = body.replace(&args[0], actual_arg);
                    let replacement = format!("({})", replaced_body);
                    expanded_input = format!(
                        "{}{}{}",
                        &expanded_input[..pos],
                        replacement,
                        &expanded_input[pos + close + 1..]
                    );
                    search_idx = pos + replacement.len();
                    continue;
                }
                search_idx = pos + p_start.len();
            }
        }

        // Substitute numerical bindings for constant parameters in calculus or general expressions
        if !self.bindings.is_empty() {
            let parser = algebra_core::parser::ExprParser::new(&self.graph);
            if let Ok(expr_id) = parser.parse(&expanded_input) {
                let syms = self.extract_symbols(expr_id);
                // If all symbols are bound, evaluate directly
                if !syms.is_empty() && syms.iter().all(|s| self.bindings.contains_key(s)) {
                    let mut eval_ctx = EvalContext::default();
                    for (k, v) in &self.bindings {
                        eval_ctx.bindings.insert(k.clone(), *v);
                    }
                    if let Ok(num) = self.graph.evalf(expr_id, &eval_ctx) {
                        let val_f64 = num.to_f64();
                        let val_str = if val_f64.fract() == 0.0 && val_f64.abs() < 1e12 {
                            format!("{}", val_f64 as i64)
                        } else {
                            format!("{:.4}", val_f64)
                        };
                        return OperationResult::success(val_str.clone(), val_str.clone(), val_str);
                    }
                }
            }
        }

        let op = parse_operation(&expanded_input);
        self.execute_op(&op)
    }

    /// Execute a parsed `MathOperation` against this session.
    pub fn execute_op(&mut self, op: &MathOperation) -> OperationResult {
        // Intercept state mutations
        match op {
            MathOperation::Declaration {
                name,
                role,
                value,
                unit,
            } => {
                let val = value.unwrap_or(1.0);
                let mut meta = SymbolSessionMeta::new(name, val, *role);
                meta.unit_str = unit.clone();
                if *role == SymbolRoleKind::Parameter {
                    self.bindings.insert(name.clone(), val);
                }
                self.symbol_metadata.insert(name.clone(), meta);
            }
            MathOperation::DomainRestriction(bound) => {
                self.set_domain(bound.clone());
            }
            MathOperation::SetBuilder {
                var_name,
                domain_type,
                condition: _,
            } => {
                let bound = DomainBound {
                    name: var_name.clone(),
                    domain_type: domain_type.clone(),
                    min_val: None,
                    max_val: None,
                    inclusive_min: true,
                    inclusive_max: true,
                };
                self.set_domain(bound);
            }
            MathOperation::SetContext { key, value } => match key.to_lowercase().as_str() {
                "algebra" => match value.to_lowercase().as_str() {
                    "reals" | "real" | "r" => {
                        self.context.algebra_domain = algebra_core::Domain::Reals
                    }
                    "complex" | "c" => self.context.algebra_domain = algebra_core::Domain::Complex,
                    "quaternion" | "quaternions" | "h" => {
                        self.context.algebra_domain =
                            algebra_core::Domain::Hypercomplex { dimension: 4 }
                    }
                    _ => {}
                },
                "calculus" => self.context.calculus_system = value.clone(),
                "logic" => self.context.logic_system = value.clone(),
                _ => {}
            },
            _ => {}
        }

        let ctx = self.execution_context();
        OperationExecutor::execute(&self.graph, op, &ctx)
    }

    /// Extract all unique variable symbol names contained in an expression tree.
    pub fn extract_symbols(&self, root: ExprId) -> Vec<String> {
        let mut symbols = HashSet::new();
        let mut stack = vec![root];

        while let Some(curr) = stack.pop() {
            let node = self.graph.get(curr);
            match &node.kind {
                ExprKind::Symbol(sym_id) => {
                    if let Some(name) = self.graph.symbols.resolve(*sym_id) {
                        symbols.insert(name);
                    }
                }
                ExprKind::Function { name: _, args } => {
                    for &arg in args {
                        stack.push(arg);
                    }
                }
                _ => {
                    for child in node.children() {
                        stack.push(child);
                    }
                }
            }
        }

        let mut list: Vec<_> = symbols.into_iter().collect();
        list.sort();
        list
    }
}
