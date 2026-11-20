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
use algebra_core::{
    ExprGraph, ExprId, ExprKind, Formatter, RealInterval, parse_value_uncertainty_and_unit,
};
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
    #[serde(default)]
    pub uncertainty: Option<f64>,
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
            uncertainty: None,
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
    /// 1-sigma parameter uncertainties (from `a = 5.0 +/- 0.1` etc.).
    pub uncertainties: HashMap<String, f64>,
    /// Certified parameter interval bounds.
    pub intervals: HashMap<String, RealInterval>,
    /// Optional active Riemann sheet index `k`.
    pub branch_sheet: Option<i64>,
    /// Declared domain bounds per symbol.
    pub symbol_domains: HashMap<String, DomainBound>,
    /// Rich metadata for symbols (role, value, unit, domain).
    pub symbol_metadata: HashMap<String, SymbolSessionMeta>,
    /// User-defined function definitions `name -> (args, body)`.
    pub user_functions: HashMap<String, (Vec<String>, String)>,
    /// Symbolic variable aliases `name -> rhs_expr_str` (e.g. `p = (x + 1)^2`).
    pub user_aliases: HashMap<String, String>,
    /// Most recent successfully computed AST node (used for `ans`).
    pub last_result_expr: Option<ExprId>,
    /// Most recent parseable mathematical result string (used for `ans`).
    pub last_result_text: Option<String>,
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
            uncertainties: HashMap::new(),
            intervals: HashMap::new(),
            branch_sheet: None,
            symbol_domains: HashMap::new(),
            symbol_metadata: HashMap::new(),
            user_functions: HashMap::new(),
            user_aliases: HashMap::new(),
            last_result_expr: None,
            last_result_text: None,
            context: MathContext::default(),
            history: Vec::new(),
        }
    }

    /// Clear all variable bindings, domains, functions, and reset the expression graph.
    pub fn reset(&mut self) {
        self.graph = ExprGraph::new();
        self.bindings.clear();
        self.uncertainties.clear();
        self.intervals.clear();
        self.branch_sheet = None;
        self.symbol_domains.clear();
        self.symbol_metadata.clear();
        self.user_functions.clear();
        self.user_aliases.clear();
        self.last_result_expr = None;
        self.last_result_text = None;
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

    /// Set or update a symbol's numerical binding along with optional 1-sigma uncertainty.
    pub fn set_uncertain_binding(
        &mut self,
        name: impl Into<String>,
        val: f64,
        uncertainty: f64,
        unit: Option<String>,
    ) {
        let name_str = name.into();
        let sigma = uncertainty.abs();
        self.bindings.insert(name_str.clone(), val);
        self.uncertainties.insert(name_str.clone(), sigma);
        self.intervals.insert(
            name_str.clone(),
            RealInterval::from_center_uncertainty(val, sigma),
        );
        let entry = self
            .symbol_metadata
            .entry(name_str.clone())
            .or_insert_with(|| SymbolSessionMeta::new(&name_str, val, SymbolRoleKind::Parameter));
        entry.cur_val = entry.clamp(val);
        entry.uncertainty = Some(sigma);
        if let Some(u) = unit {
            entry.unit_str = Some(u);
        }
    }

    /// Build an `ExecutionContext` populated with current bindings, uncertainties, assumptions, and symbol summary.
    pub fn execution_context(&self) -> ExecutionContext {
        let mut ctx = ExecutionContext::with_bindings(self.bindings.clone());
        ctx.uncertainties = self.uncertainties.clone();
        ctx.intervals = self.intervals.clone();
        ctx.branch_sheet = self.branch_sheet;
        for (name, bound) in &self.symbol_domains {
            let sym = self.graph.symbols.get_or_intern(name);
            match bound.domain_type.as_str() {
                "Positive" => ctx
                    .assumptions
                    .assume(sym, crate::logic::Predicate::Positive),
                "NonNegative" | "Naturals" => {
                    ctx.assumptions
                        .assume(sym, crate::logic::Predicate::NonNegative);
                    if bound.domain_type == "Naturals" {
                        ctx.assumptions
                            .assume(sym, crate::logic::Predicate::Integer);
                    }
                }
                "Negative" => ctx
                    .assumptions
                    .assume(sym, crate::logic::Predicate::Negative),
                "NonPositive" => {
                    ctx.assumptions.assume(sym, crate::logic::Predicate::Real);
                    ctx.assumptions.set_bounds(
                        sym,
                        crate::logic::IntervalBound {
                            min_value: None,
                            min_inclusive: false,
                            max_value: Some(0.0),
                            max_inclusive: true,
                        },
                    );
                }
                "NonZero" => ctx
                    .assumptions
                    .assume(sym, crate::logic::Predicate::NonZero),
                "Integers" => ctx
                    .assumptions
                    .assume(sym, crate::logic::Predicate::Integer),
                "EvenIntegers" => ctx.assumptions.assume(sym, crate::logic::Predicate::Even),
                "OddIntegers" => ctx.assumptions.assume(sym, crate::logic::Predicate::Odd),
                "Prime" => ctx.assumptions.assume(sym, crate::logic::Predicate::Prime),
                "Reals" => {
                    ctx.assumptions.assume(sym, crate::logic::Predicate::Real);
                    if let Some(min) = bound.min_val {
                        if min > 0.0 {
                            ctx.assumptions
                                .assume(sym, crate::logic::Predicate::Positive);
                        } else if min >= 0.0 {
                            ctx.assumptions
                                .assume(sym, crate::logic::Predicate::NonNegative);
                        }
                    }
                    if let Some(max) = bound.max_val
                        && max < 0.0
                    {
                        ctx.assumptions
                            .assume(sym, crate::logic::Predicate::Negative);
                    }
                }
                _ => {}
            }
            if bound.min_val.is_some() || bound.max_val.is_some() {
                ctx.assumptions.set_bounds(
                    sym,
                    crate::logic::IntervalBound {
                        min_value: bound.min_val,
                        min_inclusive: bound.inclusive_min,
                        max_value: bound.max_val,
                        max_inclusive: bound.inclusive_max,
                    },
                );
            }
        }
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

    /// Register an assumption or domain bound (e.g. `"x > 0"`, `"n in Integers"`, `"x: positive"`).
    pub fn assume(&mut self, decl: &str) -> OperationResult {
        let trimmed = decl.trim();
        let line =
            if trimmed.to_ascii_lowercase().starts_with("assume ") || trimmed.contains(" in ") {
                trimmed.to_string()
            } else {
                format!("assume {}", trimmed)
            };
        self.execute_line(&line)
    }

    /// Evaluate any URAE expression or command and return its Unicode result string.
    pub fn eval(&mut self, input: &str) -> Result<String, String> {
        let res = self.execute_line(input);
        if res.is_error {
            Err(res.error_msg.unwrap_or(res.output_text))
        } else {
            Ok(res.output_unicode)
        }
    }

    /// Evaluate any URAE expression or command and return its LaTeX result string.
    pub fn eval_latex(&mut self, input: &str) -> Result<String, String> {
        let res = self.execute_line(input);
        if res.is_error {
            Err(res.error_msg.unwrap_or(res.output_text))
        } else {
            Ok(res.output_latex)
        }
    }

    /// Differentiate an expression with respect to `var` under current session state.
    pub fn diff(&mut self, expr: &str, var: &str) -> Result<String, String> {
        let cmd = format!("diff {}, {}", expr, var);
        self.eval(&cmd)
    }

    /// Integrate an expression with respect to `var` under current session state.
    pub fn integrate(&mut self, expr: &str, var: &str) -> Result<String, String> {
        let cmd = format!("integrate {}, {}", expr, var);
        self.eval(&cmd)
    }

    /// Solve an equation for `var` under current session assumptions and bindings.
    pub fn solve(&mut self, eq: &str, var: &str) -> Result<String, String> {
        let cmd = format!("solve {}, {}", eq, var);
        self.eval(&cmd)
    }

    /// Expand session user aliases, `ans` references, and user-defined functions in `input`.
    pub fn expand_definitions(&self, input: &str) -> String {
        let mut cur = input.to_string();
        for _ in 0..8 {
            let before = cur.clone();
            if !self.user_aliases.is_empty() {
                cur = subst_identifiers(&cur, &self.user_aliases);
            }
            if let Some(prev) = &self.last_result_text {
                let mut ans_map = HashMap::new();
                let wrapped = if prev.contains(' ')
                    || prev.contains('+')
                    || prev.contains('-')
                    || prev.contains('*')
                    || prev.contains('/')
                    || prev.contains('^')
                {
                    format!("({})", prev)
                } else {
                    prev.clone()
                };
                ans_map.insert("ans".to_string(), wrapped);
                cur = subst_identifiers(&cur, &ans_map);
            }
            for (fn_name, (args, body)) in &self.user_functions {
                cur = expand_fn_calls(&cur, fn_name, args, body);
            }
            if cur == before {
                break;
            }
        }
        cur
    }

    /// Parse and execute a single line of input against this session, updating state.
    pub fn execute_line(&mut self, input: &str) -> OperationResult {
        let mut result = self.execute_line_inner(input);
        // `2 x`, `a x^2 + b x + c`: a parse failure is retried with a space between two factors read as a product.
        if result.is_error
            && result
                .error_msg
                .as_deref()
                .is_some_and(|m| m.contains("Parse failed"))
            && let Some(starred) = implicit_product_spaces(input.trim())
        {
            let retry = self.execute_line_inner(&starred);
            if !retry.is_error {
                result = retry;
            }
        }
        if !result.is_error {
            self.append_float_for_decimal_input(input.trim(), &mut result);
        }
        result
    }

    /// A constant typed with a decimal literal (`sin(0.5)`, `ln(2.0)`, `normal_cdf(1.96)`) asks for a floating-point
    /// answer: when the exact answer is not already a plain number, append its value (`sin(1/2) ≈ 0.4794...`).
    fn append_float_for_decimal_input(&self, input: &str, result: &mut OperationResult) {
        let bytes = input.as_bytes();
        let has_decimal = bytes
            .windows(3)
            .any(|w| w[0].is_ascii_digit() && w[1] == b'.' && w[2].is_ascii_digit());
        if !has_decimal || input.contains('=') || input.contains('[') || input.contains('{') {
            return;
        }
        let shown = result.output_unicode.trim();
        let plain_number = !shown.is_empty()
            && shown
                .chars()
                .all(|c| c.is_ascii_digit() || matches!(c, '.' | '/' | '-' | ' ' | 'e' | '+'));
        if plain_number || shown.contains('≈') {
            return;
        }
        let parser = algebra_core::parser::ExprParser::new(&self.graph);
        // `sin(10.5°)` does not parse as typed: use the rewritten form the engine actually evaluated.
        let rewritten = crate::input_text::normalise(input).unwrap_or_else(|| input.to_string());
        let Ok(id) = parser.parse(&rewritten) else {
            return;
        };
        let mut symbols = std::collections::BTreeSet::new();
        crate::poly::collect_expr_symbols(&self.graph, id, &mut symbols);
        if symbols
            .iter()
            .filter_map(|s| self.graph.symbols.resolve(*s))
            .any(|n| !matches!(n.as_str(), "pi" | "π" | "e" | "i"))
        {
            return;
        }
        let Ok(value) = self.graph.evalf(id, &EvalContext::default()) else {
            return;
        };
        let (re, im) = value.as_complex();
        if !re.is_finite() || im.abs() > 1e-12 * (1.0 + re.abs()) {
            return;
        }
        let tail = format!(" ≈ {re}");
        result.output_text.push_str(&tail);
        result.output_unicode.push_str(&tail);
        result.output_latex.push_str(&format!(r" \approx {re}"));
    }

    fn execute_line_inner(&mut self, input: &str) -> OperationResult {
        let trimmed = input.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
            return OperationResult::success("", "", "");
        }

        self.history.push(trimmed.to_string());

        // `spanning_trees(K4)`, `chromatic_polynomial(C5)`, `shortest_path(G, a, b)`: finite graphs.
        if let Some(outcome) = crate::graph_commands::run(trimmed) {
            return match outcome {
                Ok(text) => OperationResult::success(text.clone(), text.clone(), text),
                Err(e) => OperationResult::error(e),
            };
        }

        // `truth_table(F)`, `tautology(F)`, `satisfiable(F)`, `equivalent(F, G)`: propositional logic.
        if let Some(outcome) = crate::logic_commands::run(trimmed) {
            return match outcome {
                Ok(text) => OperationResult::success(text.clone(), text.clone(), text),
                Err(e) => OperationResult::error(e),
            };
        }

        // `arg`, `modulus`, `polar`, `atan2`, `convolve`, `dft`, `fft`.
        if let Some(outcome) = crate::complex_funcs::run(self, trimmed) {
            return match outcome {
                Ok(text) => OperationResult::success(text.clone(), text.clone(), text),
                Err(e) => OperationResult::error(e),
            };
        }

        // `rk4`, `euler_method`: fixed-step numerical ODE solutions.
        if let Some(outcome) = crate::ode_numeric::run(self, trimmed) {
            return match outcome {
                Ok(text) => OperationResult::success(text.clone(), text.clone(), text),
                Err(e) => OperationResult::error(e),
            };
        }

        // `critical_points(f)` / `extrema(f)`: stationary points and their kind.
        if let Some(outcome) = crate::stationary::run(self, trimmed) {
            return match outcome {
                Ok(text) => OperationResult::success(text.clone(), text.clone(), text),
                Err(e) => OperationResult::error(e),
            };
        }

        // `maximize 3x + 2y subject to x + y <= 4, ...`: exact linear programming.
        if let Some(outcome) = crate::simplex_exact::solve_text(&self.graph, trimmed) {
            // Not a linear program (a nonlinear objective or constraint): try Lagrange multipliers.
            let nonlinear = matches!(&outcome, Err(e) if e.contains("linear"));
            if nonlinear && let Some(lagrange) = crate::constrained::run(self, trimmed) {
                return match lagrange {
                    Ok(text) => OperationResult::success(text.clone(), text.clone(), text),
                    Err(e) => OperationResult::error(e),
                };
            }
            return match outcome {
                Ok(text) => OperationResult::success(text.clone(), text.clone(), text),
                Err(e) => OperationResult::error(e),
            };
        }

        // LaTeX (an integral written with a backslash int, a backslash frac derivative) and plain English (`what is 2 plus 3`) are rewritten first.
        if let Some(plain) = crate::input_text::normalise(trimmed)
            && plain != trimmed
        {
            return self.execute_line(&plain);
        }

        // Commands inside the arguments of another command (`factor(expand((x+1)^3 - 1))`,
        // `integrate(simplify(...), x)`) are evaluated first and replaced by their results.
        let resolved = self.resolve_nested_commands(trimmed);
        let trimmed = resolved.as_deref().unwrap_or(trimmed);

        // Matrix arithmetic typed as plain input (`A + B`, `2*A`, `A * v`, `det(inverse(A))`, `identity(3)`).
        if (trimmed.contains('[')
            || crate::matexpr::is_geometry_call(trimmed)
            || ["identity(", "eye(", "zeros(", "ones("]
                .iter()
                .any(|f| trimmed.starts_with(f)))
            && let Some(outcome) = crate::matexpr::evaluate(&self.graph, trimmed)
        {
            return match outcome {
                Ok(text) => OperationResult::success(text.clone(), text.clone(), text),
                Err(e) => OperationResult::error(e),
            };
        }

        // A finite set expression (`2 in {1, 2}`, `{1,2} union {3}`) before the `x in Domain` declaration syntax.
        if trimmed.contains('{')
            && let Some(text) = crate::set_expr::evaluate(trimmed)
        {
            return OperationResult::success(text.clone(), text.clone(), text);
        }

        // Check for domain declaration e.g. `a in Reals [0, 100]`
        if let Some(bound) = parse_domain_declaration(trimmed) {
            self.set_domain(bound.clone());
            let op = MathOperation::DomainRestriction(bound);
            let ctx = self.execution_context();
            return OperationExecutor::execute(&self.graph, &op, &ctx);
        }

        // Check for assignment or function definition (`name = rhs` or `f(x, y) = body`),
        // carefully avoiding relational operators (`==`, `<=`, `>=`, `!=`).
        if !trimmed.contains("==")
            && !trimmed.contains("<=")
            && !trimmed.contains(">=")
            && !trimmed.contains("!=")
            && let Some((lhs_raw, rhs_raw)) = trimmed.split_once('=')
        {
            let lhs = lhs_raw.trim();
            let rhs_trim = rhs_raw.trim();

            // Function definition `f(x, y) = body`
            if let Some(open_p) = lhs.find('(')
                && lhs.ends_with(')')
            {
                let fn_name = lhs[..open_p].trim();
                let inner = &lhs[open_p + 1..lhs.len() - 1];
                let args: Vec<String> = inner
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_'))
                    .collect();
                if !fn_name.is_empty()
                    && fn_name.chars().all(|c| c.is_alphanumeric() || c == '_')
                    && !args.is_empty()
                {
                    self.user_functions
                        .insert(fn_name.to_string(), (args.clone(), rhs_trim.to_string()));
                    let out = format!("{}({}) = {}", fn_name, args.join(", "), rhs_trim);
                    return OperationResult::success(out.clone(), out.clone(), out);
                }
            }

            // Simple variable assignment `name = ...`
            if !lhs.contains(':')
                && !lhs.contains('(')
                && lhs != "ans"
                && lhs
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_alphabetic() || c == '_')
                && lhs.chars().all(|c| c.is_alphanumeric() || c == '_')
            {
                let val_norm = rhs_trim.replace("\\pm", "±");
                if let Some((f, err_opt, unit_opt)) = parse_value_uncertainty_and_unit(&val_norm) {
                    self.user_aliases.remove(lhs);
                    if let Some(sigma) = err_opt {
                        self.set_uncertain_binding(lhs, f, sigma, unit_opt);
                        let unit_suffix = self
                            .symbol_metadata
                            .get(lhs)
                            .and_then(|m| m.unit_str.as_ref())
                            .map(|u| format!(" [{}]", u))
                            .unwrap_or_default();
                        let out = format!("{} = {} ± {}{}", lhs, f, sigma, unit_suffix);
                        return OperationResult::success(out.clone(), out.clone(), out);
                    }
                    self.set_binding(lhs, f);
                    if let Some(u) = unit_opt
                        && let Some(meta) = self.symbol_metadata.get_mut(lhs)
                    {
                        meta.unit_str = Some(u);
                    }
                    let cur_val = self
                        .symbol_metadata
                        .get(lhs)
                        .map(|m| m.cur_val)
                        .unwrap_or(f);
                    let unit_suffix = self
                        .symbol_metadata
                        .get(lhs)
                        .and_then(|m| m.unit_str.as_ref())
                        .map(|u| format!(" [{}]", u))
                        .unwrap_or_default();
                    let out = format!("{} = {}{}", lhs, format_clean_float(cur_val), unit_suffix);
                    return OperationResult::success(out.clone(), out.clone(), out);
                }

                // Computed expression or symbolic alias assignment (e.g. `area = pi * r^2`, `p = (x + 1)^2`)
                let rhs_expanded = self.expand_definitions(rhs_trim);
                let parser = algebra_core::parser::ExprParser::new(&self.graph);
                if let Ok(rhs_id) = parser.parse(&rhs_expanded) {
                    let syms = self.extract_symbols(rhs_id);
                    let has_unc_syms = syms.iter().any(|s| {
                        self.uncertainties.get(s).copied().unwrap_or(0.0) > 0.0
                            || self.intervals.contains_key(s)
                    });
                    let has_unc_nodes =
                        crate::numeric::expr_has_uncertainty_nodes(&self.graph, rhs_id);
                    let all_bound = syms.iter().all(|s| {
                        self.bindings.contains_key(s)
                            || matches!(s.as_str(), "pi" | "π" | "e" | "i" | "I")
                    }) && !syms.iter().any(|s| s == lhs);

                    if all_bound && (has_unc_syms || has_unc_nodes) {
                        let eval_ctx = EvalContext::with_bindings(50, self.bindings.clone())
                            .with_uncertainties(self.uncertainties.clone())
                            .with_intervals(self.intervals.clone())
                            .with_branch_sheet(self.branch_sheet);
                        if let Ok(u) =
                            crate::numeric::eval_uncertain(&self.graph, rhs_id, &eval_ctx)
                        {
                            self.user_aliases.remove(lhs);
                            self.set_uncertain_binding(lhs, u.mean, u.std_dev, None);
                            let out = format!("{} = {}", lhs, u.format_unicode());
                            let latex = format!("{} = {}", lhs, u.format_latex());
                            return OperationResult::success(out.clone(), latex, out);
                        }
                    } else if all_bound {
                        use num_traits::ToPrimitive as _;
                        if let Ok(Some(q)) =
                            crate::numeric::try_eval_exact(&self.graph, rhs_id, &self.bindings)
                        {
                            let val_str = if q.is_integer() {
                                q.numer().to_string()
                            } else {
                                format!("{}/{}", q.numer(), q.denom())
                            };
                            if let Some(f) = q.to_f64()
                                && f.is_finite()
                            {
                                self.set_binding(lhs, f);
                            }
                            if !q.is_integer() || q.numer().bits() > 53 {
                                self.user_aliases
                                    .insert(lhs.to_string(), format!("({})", val_str));
                            } else {
                                self.user_aliases.remove(lhs);
                            }
                            self.last_result_text = Some(val_str.clone());
                            let out = format!("{} = {}", lhs, val_str);
                            let latex = if q.is_integer() {
                                out.clone()
                            } else {
                                format!("{} = \\frac{{{}}}{{{}}}", lhs, q.numer(), q.denom())
                            };
                            return OperationResult::success(out.clone(), latex, out);
                        }
                        let eval_ctx = EvalContext::with_bindings(50, self.bindings.clone())
                            .with_branch_sheet(self.branch_sheet);
                        if let Ok(num) = self.graph.evalf(rhs_id, &eval_ctx)
                            && let crate::numeric::BigValue::Real(val_f64) = num
                            && val_f64.is_finite()
                        {
                            self.user_aliases.remove(lhs);
                            self.set_binding(lhs, val_f64);
                            let val_str = format_clean_float(val_f64);
                            self.last_result_text = Some(val_str.clone());
                            let out = format!("{} = {}", lhs, val_str);
                            return OperationResult::success(out.clone(), out.clone(), out);
                        }
                    } else if !syms.iter().any(|s| s == lhs) {
                        // Free symbolic variables on RHS -> store as reusable symbolic alias
                        self.bindings.remove(lhs);
                        self.user_aliases
                            .insert(lhs.to_string(), rhs_expanded.clone());
                        let u_fmt = algebra_core::format::UnicodeFormatter
                            .format(&self.graph, rhs_id)
                            .unwrap_or_else(|_| rhs_trim.to_string());
                        let l_fmt = algebra_core::format::LatexFormatter
                            .format(&self.graph, rhs_id)
                            .unwrap_or_else(|_| rhs_trim.to_string());
                        self.last_result_expr = Some(rhs_id);
                        self.last_result_text = Some(u_fmt.clone());
                        let mut res = OperationResult::success(
                            format!("{} = {}", lhs, u_fmt),
                            format!("{} = {}", lhs, l_fmt),
                            format!("{} = {}", lhs, u_fmt),
                        );
                        res.root_expr = Some(rhs_id);
                        return res;
                    }
                }
            }
        }

        // `dsolve(y' = 2*y, y(0) = 1)`: a general solution with initial conditions.
        if let Some(result) = self.try_rsolve(trimmed) {
            return result;
        }
        if let Some(result) = self.try_dsolve_with_conditions(trimmed) {
            return result;
        }

        // Expand user functions, symbolic aliases, and `ans` before parsing/evaluation
        let expanded_input = self.expand_definitions(trimmed);
        let parsed_op = parse_operation(&expanded_input);

        // Evaluate pure expressions (constants, exact rationals, or all-bound expressions/uncertainties)
        // A bare matrix literal is displayed by the executor as a grid, not simplified as a scalar expression.
        let bare_matrix = crate::executor::is_single_bracket_group(&expanded_input);
        // A finite set expression typed as plain input (`{1,2,3} union {3,4}`, `2 in {1,2}`).
        if matches!(parsed_op, MathOperation::Expression { .. })
            && !bare_matrix
            && let Some(text) = crate::set_expr::evaluate(&expanded_input)
        {
            return OperationResult::success(text.clone(), text.clone(), text);
        }
        // A propositional formula typed as plain input (`a and not a`, `p implies q`): classify it.
        if matches!(parsed_op, MathOperation::Expression { .. })
            && !bare_matrix
            && algebra_core::parser::ExprParser::new(&self.graph)
                .parse(&expanded_input)
                .is_err()
            && let Some(result) = Self::classify_formula(&expanded_input)
        {
            return result;
        }
        if matches!(parsed_op, MathOperation::Expression { .. }) && !bare_matrix {
            let parser = algebra_core::parser::ExprParser::new(&self.graph);
            if let Ok(expr_id) = parser.parse(&expanded_input) {
                // `0/0`, `1/0`, `oo - oo`, `0 * oo`: undefined, whatever the simplifier's rules would say.
                if crate::indeterminate::is_undefined(&self.graph, expr_id) {
                    let text = "Undefined (division by zero or an indeterminate form)".to_string();
                    return OperationResult::success(text.clone(), text.clone(), text);
                }
                let syms = self.extract_symbols(expr_id);
                let all_bound = syms.iter().all(|s| {
                    self.bindings.contains_key(s)
                        || matches!(s.as_str(), "pi" | "π" | "e" | "i" | "I")
                });
                let has_user_bound = syms.iter().any(|s| self.bindings.contains_key(s));
                let has_unc_syms = syms.iter().any(|s| {
                    self.uncertainties.get(s).copied().unwrap_or(0.0) > 0.0
                        || self.intervals.contains_key(s)
                });
                let has_unc_nodes =
                    crate::numeric::expr_has_uncertainty_nodes(&self.graph, expr_id);
                if (all_bound && (has_unc_syms || has_unc_nodes))
                    || (syms.is_empty() && has_unc_nodes)
                {
                    let eval_ctx = EvalContext::with_bindings(50, self.bindings.clone())
                        .with_uncertainties(self.uncertainties.clone())
                        .with_intervals(self.intervals.clone())
                        .with_branch_sheet(self.branch_sheet);
                    if let Ok(u) = crate::numeric::eval_uncertain(&self.graph, expr_id, &eval_ctx) {
                        self.last_result_text = Some(format_clean_float(u.mean));
                        let out = u.format_unicode();
                        let latex = u.format_latex();
                        return OperationResult::success(out.clone(), latex, out);
                    }
                } else if all_bound || syms.is_empty() {
                    if !has_user_bound
                        && crate::radical::has_radicals(&self.graph, expr_id)
                        && let Some(r) = crate::radical::evaluate(&self.graph, expr_id)
                    {
                        let text = r.format();
                        self.last_result_text = Some(text.clone());
                        return OperationResult::success(text.clone(), text.clone(), text);
                    }
                    // A distribution function of numbers (`chi2_cdf(3.2, 4)`, `t_quantile(0.975, 10)`): its value.
                    if let algebra_core::ExprKind::Function { name, .. } =
                        self.graph.get(expr_id).kind.clone()
                        && let Some(fname) = self.graph.symbols.resolve(name)
                        && crate::distributions_f64::is_distribution_function(&fname)
                        && let Ok(value) = self.graph.evalf(expr_id, &EvalContext::default())
                    {
                        let text = format_clean_float(value.to_f64());
                        self.last_result_text = Some(text.clone());
                        return OperationResult::success(text.clone(), text.clone(), text);
                    }
                    if let Some(text) = crate::complex_exact::simplify_text(&self.graph, expr_id) {
                        self.last_result_text = Some(text.clone());
                        return OperationResult::success(text.clone(), text.clone(), text);
                    }
                    let exec_ctx = self.execution_context();
                    let simp_id = crate::simplify::Simplifier::simplify_with_assumptions(
                        &self.graph,
                        expr_id,
                        &exec_ctx.assumptions,
                    )
                    .unwrap_or(expr_id);
                    if let Ok(Some(q)) =
                        crate::numeric::try_eval_exact(&self.graph, simp_id, &self.bindings)
                    {
                        let out = if q.is_integer() {
                            q.numer().to_string()
                        } else {
                            format!("{}/{}", q.numer(), q.denom())
                        };
                        let latex = if q.is_integer() {
                            out.clone()
                        } else {
                            format!("\\frac{{{}}}{{{}}}", q.numer(), q.denom())
                        };
                        self.last_result_expr = Some(simp_id);
                        self.last_result_text = Some(out.clone());
                        let mut res = OperationResult::success(out.clone(), latex, out);
                        res.root_expr = Some(simp_id);
                        return res;
                    }
                    if has_user_bound {
                        let eval_ctx = EvalContext::with_bindings(50, self.bindings.clone())
                            .with_branch_sheet(self.branch_sheet);
                        if let Ok(num) = self.graph.evalf(simp_id, &eval_ctx) {
                            let val_f64 = num.to_f64();
                            let val_str = format_clean_float(val_f64);
                            self.last_result_text = Some(val_str.clone());
                            return OperationResult::success(
                                val_str.clone(),
                                val_str.clone(),
                                val_str,
                            );
                        }
                    } else {
                        let u_out = algebra_core::format::UnicodeFormatter
                            .format(&self.graph, simp_id)
                            .unwrap_or_else(|_| expanded_input.clone());
                        let l_out = algebra_core::format::LatexFormatter
                            .format(&self.graph, simp_id)
                            .unwrap_or_else(|_| expanded_input.clone());
                        self.last_result_expr = Some(simp_id);
                        self.last_result_text = Some(u_out.clone());
                        let mut res = OperationResult::success(u_out.clone(), l_out, u_out);
                        res.root_expr = Some(simp_id);
                        return res;
                    }
                }
            }
        }

        // A plain expression with symbols (`x^2*x^3`, `x^5/x^2`, `sqrt(x)*sqrt(x)`): show the simplified form when it
        // is strictly shorter than the expression as typed, so that obvious reductions are not left undone.
        if matches!(parsed_op, MathOperation::Expression { .. })
            && !bare_matrix
            && let Some(result) = self.shorter_form(&expanded_input)
        {
            return result;
        }
        self.execute_op(&parsed_op)
    }

    /// Replace each inner `expand(...)`, `simplify(...)`, `factor(...)`, ... call that sits inside another call by its
    /// result (as parseable text). `None` if there is nothing to replace.
    fn resolve_nested_commands(&mut self, input: &str) -> Option<String> {
        const INNER: [&str; 12] = [
            "expand",
            "simplify",
            "cancel",
            "together",
            "factor",
            "diff",
            "apart",
            "trigsimp",
            "expand_trig",
            "collect",
            "sum",
            "product",
        ];
        let chars: Vec<char> = input.chars().collect();
        let mut replaced = false;
        let mut result = String::new();
        let mut i = 0;
        let mut depth = 0i32;
        while i < chars.len() {
            let c = chars[i];
            if matches!(c, '(' | '[' | '{') {
                depth += 1;
            } else if matches!(c, ')' | ']' | '}') {
                depth -= 1;
            }
            // A call inside another call, or one that is only part of an arithmetic expression (`sum(...) + 1`).
            let in_arithmetic = i > 0
                || chars
                    .iter()
                    .any(|&ch| matches!(ch, '+' | '-' | '*' | '/' | '^'));
            if (depth >= 1 || in_arithmetic)
                && (c.is_ascii_alphabetic() || c == '_')
                && (i == 0 || !(chars[i - 1].is_alphanumeric() || chars[i - 1] == '_'))
            {
                let start = i;
                let mut j = i;
                while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                    j += 1;
                }
                let name: String = chars[start..j].iter().collect();
                if j < chars.len() && chars[j] == '(' && INNER.contains(&name.as_str()) {
                    // The matching bracket.
                    let mut k = j;
                    let mut d = 0i32;
                    while k < chars.len() {
                        match chars[k] {
                            '(' => d += 1,
                            ')' => {
                                d -= 1;
                                if d == 0 {
                                    break;
                                }
                            }
                            _ => {}
                        }
                        k += 1;
                    }
                    // The whole input is one call: it is the command itself, not a nested one.
                    if k < chars.len() && !(start == 0 && k + 1 == chars.len()) {
                        let inner: String = chars[start..=k].iter().collect();
                        let outcome = self.execute_line(&inner);
                        if !outcome.is_error
                            && !outcome.output_text.contains("w.r.t")
                            && !outcome.output_text.is_empty()
                            && outcome.output_text != inner
                        {
                            let text = outcome
                                .output_text
                                .replace('·', "*")
                                .replace('√', "sqrt")
                                .replace('π', "pi");
                            result.push_str(&format!("({text})"));
                            replaced = true;
                            i = k + 1;
                            // The brackets of the replaced call were counted by the scan above only up to `(`.
                            continue;
                        }
                    }
                }
            }
            result.push(c);
            i += 1;
        }
        replaced.then_some(result)
    }

    /// `a and not a` is a contradiction, `a or not a` a tautology; otherwise a satisfying assignment.
    fn classify_formula(input: &str) -> Option<OperationResult> {
        use crate::logic::{BoolExpr, SatSolver};
        let lowered = format!(" {} ", input.to_ascii_lowercase());
        let keywords = [
            " and ",
            " or ",
            " not ",
            " xor ",
            " implies ",
            " iff ",
            "->",
            "=>",
            "<->",
            "<=>",
            "&",
            "|",
            "~",
            "!",
            "¬",
            "∧",
            "∨",
        ];
        if !keywords.iter().any(|k| lowered.contains(k)) {
            return None;
        }
        let (expr, vars) = SatSolver::parse_formula(input).ok()?;
        if vars
            .iter()
            .any(|(_, n)| !n.chars().next().is_some_and(|c| c.is_alphabetic()))
        {
            return None;
        }
        let describe = |model: &std::collections::HashMap<u32, bool>| -> String {
            vars.iter()
                .map(|(id, name)| format!("{name}={}", model.get(id).copied().unwrap_or(false)))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let satisfying = SatSolver::solve_model(&expr);
        let refuting = SatSolver::solve_model(&BoolExpr::Not(Box::new(expr)));
        let text = match (satisfying, refuting) {
            (None, _) => format!(
                "{}: contradiction (false for every assignment)",
                input.trim()
            ),
            (Some(_), None) => format!("{}: tautology (true for every assignment)", input.trim()),
            (Some(yes), Some(no)) => format!(
                "{}: contingent; true for {}, false for {}",
                input.trim(),
                describe(&yes.into_iter().collect()),
                describe(&no.into_iter().collect())
            ),
        };
        Some(OperationResult::success(text.clone(), text.clone(), text))
    }

    /// The simplified form of `input` if it is shorter than the formatted input; `None` otherwise.
    fn shorter_form(&mut self, input: &str) -> Option<OperationResult> {
        use algebra_core::format::{Formatter, LatexFormatter, UnicodeFormatter};
        let parser = algebra_core::parser::ExprParser::new(&self.graph);
        let expr_id = parser.parse(input).ok()?;
        // Relations are not simplified here.
        if matches!(
            self.graph.get(expr_id).kind,
            algebra_core::ExprKind::Relational { .. }
        ) {
            return None;
        }
        let exec_ctx = self.execution_context();
        let simplified = crate::simplify::Simplifier::simplify_with_assumptions(
            &self.graph,
            expr_id,
            &exec_ctx.assumptions,
        )
        .ok()?;
        let original = UnicodeFormatter.format(&self.graph, expr_id).ok()?;
        let reduced = UnicodeFormatter.format(&self.graph, simplified).ok()?;
        if reduced.chars().count() > original.chars().count() {
            return None;
        }
        let latex = LatexFormatter
            .format(&self.graph, simplified)
            .unwrap_or_default();
        self.last_result_expr = Some(simplified);
        self.last_result_text = Some(reduced.clone());
        let mut res = OperationResult::success(reduced.clone(), latex, reduced);
        res.root_expr = Some(simplified);
        Some(res)
    }

    /// `rsolve a(n+2) = a(n+1) + a(n), a(0) = 0, a(1) = 1`: the closed form of a linear recurrence.
    fn try_rsolve(&mut self, input: &str) -> Option<OperationResult> {
        let body = [
            "rsolve",
            "solve_recurrence",
            "recurrence",
            "recurrence_solve",
        ]
        .iter()
        .find_map(|name| {
            input
                .strip_prefix(&format!("{name}("))
                .and_then(|b| b.strip_suffix(')'))
                .or_else(|| input.strip_prefix(&format!("{name} ")))
        })?;
        let mut parts = Vec::new();
        let (mut depth, mut current) = (0i32, String::new());
        for c in body.chars() {
            match c {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                _ => {}
            }
            if c == ',' && depth == 0 {
                parts.push(std::mem::take(&mut current));
            } else {
                current.push(c);
            }
        }
        parts.push(current);
        let parser = algebra_core::parser::ExprParser::new(&self.graph);
        let mut initial = Vec::new();
        let mut equation = None;
        for part in &parts {
            match crate::ode_ic::parse_condition(part) {
                // `a(0) = 1` is an initial value; `a(n+2) = ...` (a non-numeric index) is the recurrence itself.
                Some(condition)
                    if condition.order == 0
                        && condition
                            .point
                            .trim()
                            .trim_start_matches('-')
                            .chars()
                            .all(|c| c.is_ascii_digit())
                        && !condition.point.trim().is_empty() =>
                {
                    let index = condition.point.trim().parse::<i64>().ok().or_else(|| {
                        parser
                            .parse(&condition.point)
                            .ok()
                            .and_then(|id| {
                                crate::numeric::try_eval_exact(
                                    &self.graph,
                                    id,
                                    &std::collections::HashMap::new(),
                                )
                                .ok()
                                .flatten()
                            })
                            .filter(|q| q.is_integer())
                            .and_then(|q| num_traits::ToPrimitive::to_i64(&q.to_integer()))
                    });
                    match index {
                        Some(i) => initial.push((i, condition.value)),
                        None => {
                            return Some(OperationResult::error(
                                "the index of an initial value must be an integer",
                            ));
                        }
                    }
                }
                _ => {
                    if equation.is_none() {
                        equation = Some(part.trim().to_string());
                    }
                }
            }
        }
        let equation = equation?;
        Some(
            match crate::recurrence::solve(&self.graph, &equation, &initial) {
                Ok((name, text)) => {
                    let out = format!("{name}(n) = {text}");
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                Err(e) => OperationResult::error(e),
            },
        )
    }

    /// `dsolve <equation>, y(x0) = v, y'(x0) = w, ...`: solve, then fix the constants from the conditions.
    fn try_dsolve_with_conditions(&mut self, input: &str) -> Option<OperationResult> {
        let body = input
            .strip_prefix("dsolve(")
            .and_then(|b| b.strip_suffix(')'))
            .or_else(|| input.strip_prefix("dsolve "))?;
        let mut parts = Vec::new();
        let (mut depth, mut current) = (0i32, String::new());
        for c in body.chars() {
            match c {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                _ => {}
            }
            if c == ',' && depth == 0 {
                parts.push(std::mem::take(&mut current));
            } else {
                current.push(c);
            }
        }
        parts.push(current);
        let conditions: Vec<crate::ode_ic::Condition> = parts
            .iter()
            .filter_map(|p| crate::ode_ic::parse_condition(p))
            .collect();
        if conditions.is_empty() {
            return None;
        }
        let equation = parts
            .iter()
            .find(|p| crate::ode_ic::parse_condition(p).is_none())?
            .trim()
            .to_string();
        let general = self.execute_line(&format!("dsolve {equation}"));
        if general.is_error {
            return Some(general);
        }
        let Some(rhs) = general.root_expr else {
            return Some(general);
        };
        Some(match crate::ode_ic::apply(&self.graph, rhs, &conditions) {
            Ok(solution) => {
                use algebra_core::format::{Formatter, LatexFormatter, UnicodeFormatter};
                let unicode = UnicodeFormatter
                    .format(&self.graph, solution)
                    .unwrap_or_default();
                let latex = LatexFormatter
                    .format(&self.graph, solution)
                    .unwrap_or_default();
                let out = format!("{}(x) = {unicode}", conditions[0].name);
                let mut res = OperationResult::success(out.clone(), latex, out);
                res.root_expr = Some(solution);
                res
            }
            Err(e) => OperationResult::error(e),
        })
    }

    /// Execute a parsed `MathOperation` against this session.
    pub fn execute_op(&mut self, op: &MathOperation) -> OperationResult {
        // Intercept state mutations
        match op {
            MathOperation::System(algebra_core::operation::SystemCommandKind::Clear) => {
                self.reset();
            }
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
            MathOperation::UncertainDeclaration {
                name,
                role,
                value,
                uncertainty,
                unit,
            } => {
                self.set_uncertain_binding(name, *value, *uncertainty, unit.clone());
                if let Some(meta) = self.symbol_metadata.get_mut(name) {
                    meta.role = *role;
                }
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
        let res = OperationExecutor::execute(&self.graph, op, &ctx);
        if !res.is_error {
            if let Some(root) = res.root_expr {
                self.last_result_expr = Some(root);
                if let Ok(formatted) =
                    algebra_core::format::UnicodeFormatter.format(&self.graph, root)
                    && !formatted.is_empty()
                {
                    self.last_result_text = Some(formatted);
                }
            } else if matches!(
                op,
                MathOperation::Evaluate { .. } | MathOperation::Simplify { .. }
            ) && !res.output_unicode.is_empty()
            {
                self.last_result_text = Some(res.output_unicode.clone());
            }
        }
        res
    }

    /// Execute a natural-language mathematical query, returning a structured multi-pod response.
    pub fn structured_query(&self, query: &str) -> crate::query::QueryResponse {
        let ctx = self.execution_context();
        crate::query::QueryEngine::query(&self.graph, query, &ctx)
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

fn format_clean_float(val: f64) -> String {
    if !val.is_finite() {
        return val.to_string();
    }
    if (val - val.round()).abs() < 1e-12 && val.abs() < 1e14 {
        let i = val.round() as i64;
        format!("{}", i)
    } else {
        let s = format!("{:.10}", val);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

fn is_ident_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn subst_identifiers(expr: &str, map: &HashMap<String, String>) -> String {
    let mut out = String::with_capacity(expr.len());
    let chars: Vec<(usize, char)> = expr.char_indices().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let (byte_idx, ch) = chars[i];
        if ch.is_alphabetic() || ch == '_' {
            let mut end = i + 1;
            while end < chars.len() && is_ident_char(chars[end].1) {
                end += 1;
            }
            let end_byte = if end < chars.len() {
                chars[end].0
            } else {
                expr.len()
            };
            let ident = &expr[byte_idx..end_byte];
            // Do not substitute if immediately invoked as a function `ident(...)`
            let mut look = end;
            while look < chars.len() && chars[look].1.is_whitespace() {
                look += 1;
            }
            let is_call = look < chars.len() && chars[look].1 == '(';
            if !is_call && let Some(rep) = map.get(ident) {
                out.push_str(rep);
                i = end;
                continue;
            }
            out.push_str(ident);
            i = end;
        } else {
            out.push(ch);
            i += 1;
        }
    }
    out
}

fn expand_fn_calls(input: &str, fn_name: &str, params: &[String], body: &str) -> String {
    if fn_name.is_empty() || params.is_empty() {
        return input.to_string();
    }
    let pattern = format!("{}(", fn_name);
    let mut result = String::with_capacity(input.len());
    let mut cursor = 0usize;

    while let Some(rel) = input[cursor..].find(&pattern) {
        let pos = cursor + rel;
        let is_word_boundary = pos == 0
            || input[..pos]
                .chars()
                .next_back()
                .is_none_or(|c| !is_ident_char(c));

        if !is_word_boundary {
            result.push_str(&input[cursor..pos + pattern.len()]);
            cursor = pos + pattern.len();
            continue;
        }

        let arg_start = pos + pattern.len();
        let mut depth_paren = 1usize;
        let mut depth_brack = 0usize;
        let mut cur_arg_start = arg_start;
        let mut actual_args = Vec::new();
        let mut close_byte = None;

        for (offset, ch) in input[arg_start..].char_indices() {
            let abs = arg_start + offset;
            match ch {
                '(' => depth_paren += 1,
                ')' => {
                    depth_paren -= 1;
                    if depth_paren == 0 && depth_brack == 0 {
                        actual_args.push(input[cur_arg_start..abs].trim().to_string());
                        close_byte = Some(abs);
                        break;
                    }
                }
                '[' => depth_brack += 1,
                ']' => depth_brack = depth_brack.saturating_sub(1),
                ',' if depth_paren == 1 && depth_brack == 0 => {
                    actual_args.push(input[cur_arg_start..abs].trim().to_string());
                    cur_arg_start = abs + 1;
                }
                _ => {}
            }
        }

        if let Some(end_paren) = close_byte
            && actual_args.len() == params.len()
        {
            let mut param_map = HashMap::new();
            for (p, a) in params.iter().zip(actual_args.iter()) {
                let wrapped = if a.is_empty() {
                    a.clone()
                } else {
                    format!("({})", a)
                };
                param_map.insert(p.clone(), wrapped);
            }
            let substituted = subst_identifiers(body, &param_map);
            result.push_str(&input[cursor..pos]);
            result.push('(');
            result.push_str(&substituted);
            result.push(')');
            cursor = end_paren + 1;
        } else {
            result.push_str(&input[cursor..arg_start]);
            cursor = arg_start;
        }
    }

    result.push_str(&input[cursor..]);
    result
}

/// `input` with a `*` in place of each space that separates two factors (`a x^2 + b x` is `a*x^2 + b*x`), or `None`
/// when there is no such space. English connecting words are never treated as factors.
fn implicit_product_spaces(input: &str) -> Option<String> {
    const WORDS: [&str; 22] = [
        "to",
        "in",
        "as",
        "of",
        "and",
        "or",
        "mod",
        "union",
        "intersect",
        "intersection",
        "minus",
        "subject",
        "such",
        "that",
        "where",
        "if",
        "for",
        "with",
        "at",
        "by",
        "from",
        "is",
    ];
    // A bare function name (`sin x cos x`) would be read as a symbol: leave such input alone.
    const FUNCTIONS: [&str; 24] = [
        "sin",
        "cos",
        "tan",
        "sec",
        "csc",
        "cot",
        "asin",
        "acos",
        "atan",
        "sinh",
        "cosh",
        "tanh",
        "ln",
        "log",
        "exp",
        "sqrt",
        "abs",
        "floor",
        "ceil",
        "gamma",
        "erf",
        "det",
        "inverse",
        "transpose",
    ];
    let chars: Vec<char> = input.chars().collect();
    let mut k = 0;
    while k < chars.len() {
        if chars[k].is_alphabetic() {
            let end = chars[k..]
                .iter()
                .position(|c| !c.is_alphabetic())
                .map_or(chars.len(), |p| k + p);
            let word: String = chars[k..end].iter().collect();
            let next = chars[end..].iter().find(|c| **c != ' ');
            if FUNCTIONS.contains(&word.as_str()) && next != Some(&'(') {
                return None;
            }
            // A multi-letter word that is not a call (`5 xor 3`) is not a product of single-letter symbols.
            if word.chars().count() >= 2
                && next != Some(&'(')
                && !matches!(word.as_str(), "pi" | "oo" | "inf")
            {
                return None;
            }
            k = end;
        } else {
            k += 1;
        }
    }
    let word_before = |end: usize| -> String {
        let start = chars[..end]
            .iter()
            .rposition(|c| !c.is_alphabetic())
            .map_or(0, |p| p + 1);
        chars[start..end].iter().collect()
    };
    let word_after = |start: usize| -> String {
        chars[start..]
            .iter()
            .take_while(|c| c.is_alphabetic())
            .collect()
    };
    let (mut out, mut i, mut changed) = (String::new(), 0usize, false);
    while i < chars.len() {
        if chars[i] == ' ' {
            let mut j = i;
            while j < chars.len() && chars[j] == ' ' {
                j += 1;
            }
            if i > 0 && j < chars.len() {
                let (p, n) = (chars[i - 1], chars[j]);
                let left = p.is_alphanumeric() || p == ')';
                let right = n.is_alphanumeric() || n == '(';
                if left
                    && right
                    && !(p.is_ascii_digit() && n.is_ascii_digit())
                    && !WORDS.contains(&word_before(i).to_ascii_lowercase().as_str())
                    && !WORDS.contains(&word_after(j).to_ascii_lowercase().as_str())
                {
                    out.push('*');
                    changed = true;
                    i = j;
                    continue;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    changed.then_some(out)
}
