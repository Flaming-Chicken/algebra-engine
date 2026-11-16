//! # `algebra_engine::query`
//!
//! Natural-Language Mathematical Query Interpretation & Multi-Pod Structured Response Engine.
//!
//! Given any query (e.g. `"integrate x * sin(x)"`, `"solve 2x^2 - 5x + 3 = 0"`, `"d/dx e^(-x^2)"`,
//! `"gamma(7/2)"`, `"zeta(4)"`, `"eigenvalues [[1, 2], [3, 4]]"`, `"limit sin(x)/x as x -> 0"`),
//! produces a structured multi-pod response combining symbolic computation, derivations, and diagnostics:
//! - **Pod 1: Input Interpretation**
//! - **Pod 2: Exact Symbolic Result**
//! - **Pod 3: Step-by-Step Pedagogical Derivation**
//! - **Pod 4: High-Precision Numerical / Uncertainty Approximation**
//! - **Pod 5: Alternate Forms (Expanded, Simplified)**
//! - **Pod 6: Derivative & Integral Companion Analysis**
//! - **Pod 7: Roots & Critical Points**
//!
//! ## Architecture References
//! - Geddes, K. O., Czapor, S. R., & Labahn, G. (1992). *Algorithms for Computer Algebra*. Springer.
//! - von zur Gathen, J., & Gerhard, J. (2013). *Modern Computer Algebra* (3rd ed.). Cambridge University Press.

use algebra_core::ExprGraph;
use algebra_core::format::{Formatter, LatexFormatter, UnicodeFormatter};
use algebra_core::operation::*;
use algebra_core::parser::parse_operation;
use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

use crate::calculus::SymbolicCalculus;
use crate::executor::{ExecutionContext, OperationExecutor};
use crate::numeric::{EvalContext, NumericalEval};
use crate::poly::collect_expr_symbols;
use crate::simplify::Simplifier;
use crate::solver::SymbolicSolver;

/// A single formatted mathematical result pod.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultPod {
    /// Pod title (e.g. "Input interpretation", "Result", "Step-by-step solution")
    pub title: String,
    /// Plaintext mathematical string
    pub text: String,
    /// LaTeX formatted math string
    pub latex: String,
    /// 2D Unicode formatted math string
    pub unicode: String,
    /// Educational sub-steps or bullet points
    pub subpods: Vec<String>,
}

impl ResultPod {
    pub fn new(
        title: impl Into<String>,
        text: impl Into<String>,
        latex: impl Into<String>,
        unicode: impl Into<String>,
    ) -> Self {
        Self {
            title: title.into(),
            text: text.into(),
            latex: latex.into(),
            unicode: unicode.into(),
            subpods: Vec::new(),
        }
    }

    pub fn with_subpods(mut self, subpods: Vec<String>) -> Self {
        self.subpods = subpods;
        self
    }
}

/// Complete multi-pod structured query response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    /// Raw input query
    pub query: String,
    /// Identified operation intent
    pub intent: String,
    /// List of generated result pods
    pub pods: Vec<ResultPod>,
    /// Execution elapsed time in microseconds
    pub execution_time_us: u64,
}

impl QueryResponse {
    /// Format all pods into human-readable terminal output.
    pub fn format_terminal(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "=== URAE Structured Query: \"{}\" ({} µs) ===\n\n",
            self.query, self.execution_time_us
        ));
        for pod in &self.pods {
            out.push_str(&format!("[ {} ]\n", pod.title));
            out.push_str(&format!("  {}\n", pod.unicode));
            for sub in &pod.subpods {
                out.push_str(&format!("    • {}\n", sub));
            }
            out.push('\n');
        }
        out
    }
}

/// Natural-language mathematical query interpreter and multi-pod response engine.
pub struct QueryEngine;

impl QueryEngine {
    /// Process any natural-language math query and return a structured multi-pod response.
    pub fn query(
        graph: &ExprGraph,
        query_str: &str,
        ctx: &ExecutionContext,
    ) -> QueryResponse {
        #[cfg(not(target_arch = "wasm32"))]
        let start_time = Instant::now();
        let query_trimmed = query_str.trim();
        let preprocessed = Self::preprocess_natural_query(query_trimmed);

        // 1. Parse operation
        let op = parse_operation(&preprocessed);
        let latex_fmt = LatexFormatter;
        let unicode_fmt = UnicodeFormatter;
        let mut pods = Vec::new();

        let intent_name = match &op {
            MathOperation::Differentiate { .. } => "Differentiation",
            MathOperation::Integrate { .. } => "Integration",
            MathOperation::Solve { .. } => "Equation Solving",
            MathOperation::SolveSystem { .. } => "System of Equations Solving",
            MathOperation::Simplify { .. } => "Simplification",
            MathOperation::Evaluate { .. } => "Numerical Evaluation",
            MathOperation::Limit { .. } => "Limit",
            MathOperation::Series { .. } => "Series Expansion",
            MathOperation::Sum { .. } => "Summation",
            MathOperation::Matrix(_) => "Linear Algebra",
            _ => "Expression Analysis",
        };

        // 2. Generate Input Interpretation Pod
        let (input_text, input_latex, input_unicode) =
            Self::format_input_interpretation(graph, &op, query_trimmed);
        pods.push(ResultPod::new(
            "Input interpretation",
            input_text,
            input_latex,
            input_unicode,
        ));

        // 3. Execute Primary Operation
        let exec_res = OperationExecutor::execute(graph, &op, ctx);

        // 4. Generate Result Pod
        if exec_res.is_error {
            pods.push(ResultPod::new(
                "Error",
                exec_res.output_text.clone(),
                exec_res.output_latex.clone(),
                exec_res.output_unicode.clone(),
            ));
        } else {
            pods.push(ResultPod::new(
                "Result",
                exec_res.output_text.clone(),
                exec_res.output_latex.clone(),
                exec_res.output_unicode.clone(),
            ));
        }

        // 5. Generate Step-by-Step Solution Pod (if applicable)
        if let Some(step_pod) = Self::generate_step_by_step_pod(graph, &op) {
            pods.push(step_pod);
        }

        // 6. Generate High-Precision Numerical Approximation Pod (if symbolic)
        if let Some(root_id) = exec_res.root_expr {
            let eval_ctx = EvalContext::with_bindings(ctx.precision_digits, ctx.bindings.clone())
                .with_uncertainties(ctx.uncertainties.clone())
                .with_intervals(ctx.intervals.clone())
                .with_branch_sheet(ctx.branch_sheet);
            if eval_ctx.has_uncertainty() || crate::numeric::expr_has_uncertainty_nodes(graph, root_id) {
                if let Ok(u) = crate::numeric::eval_uncertain(graph, root_id, &eval_ctx) {
                    let u_unicode = u.format_unicode();
                    let u_latex = u.format_latex();
                    pods.push(ResultPod::new(
                        "Numerical approximation",
                        u_unicode.clone(),
                        u_latex,
                        u_unicode,
                    ));
                }
            } else if let Ok(big_val) = graph.evalf(root_id, &eval_ctx) {
                let f_val = big_val.to_f64();
                if f_val.is_finite() {
                    let num_str = format!("{:.15}", f_val);
                    pods.push(ResultPod::new(
                        "Numerical approximation",
                        num_str.clone(),
                        num_str.clone(),
                        num_str,
                    ));
                }
            }
        }

        // 7. Alternate Forms Pod (Expanded, Simplified)
        if let Some(root_id) = exec_res.root_expr {
            let mut sub_forms = Vec::new();
            let mut latex_forms = Vec::new();
            let exp_id = crate::simplify::AlgebraicTransformations::expand(graph, root_id);
            if exp_id != root_id {
                let exp_str = unicode_fmt.format(graph, exp_id).unwrap_or_default();
                let exp_tex = latex_fmt.format(graph, exp_id).unwrap_or_default();
                sub_forms.push(format!("Expanded form: {}", exp_str));
                latex_forms.push(format!("\\text{{Expanded form: }} {}", exp_tex));
            }
            if let Ok(simp_id) = Simplifier::simplify(graph, root_id)
                && simp_id != root_id
            {
                let simp_str = unicode_fmt.format(graph, simp_id).unwrap_or_default();
                let simp_tex = latex_fmt.format(graph, simp_id).unwrap_or_default();
                sub_forms.push(format!("Simplified form: {}", simp_str));
                latex_forms.push(format!("\\text{{Simplified form: }} {}", simp_tex));
            }
            if !sub_forms.is_empty() {
                pods.push(
                    ResultPod::new(
                        "Alternate forms",
                        sub_forms.join("\n"),
                        latex_forms.join(", \\quad "),
                        sub_forms.join(" | "),
                    )
                    .with_subpods(sub_forms),
                );
            }
        }

        // 8. Companion Differential & Integral Analysis Pod (when analyzing an expression)
        if let MathOperation::Expression { raw } = &op {
            let parser = algebra_core::parser::ExprParser::new(graph);
            if let Ok(expr_id) = parser.parse(raw) {
                let mut sym_ids = std::collections::BTreeSet::new();
                collect_expr_symbols(graph, expr_id, &mut sym_ids);
                let mut free_vars: Vec<String> = sym_ids
                    .into_iter()
                    .filter_map(|s| graph.symbols.resolve(s))
                    .filter(|name| {
                        !matches!(
                            name.as_str(),
                            "pi" | "π" | "e" | "i" | "I" | "tau" | "τ" | "oo" | "inf" | "Inf" | "infinity" | "∞" | "nan" | "NaN"
                        ) && !ctx.bindings.contains_key(name)
                    })
                    .collect();
                if let Some(x_pos) = free_vars.iter().position(|v| v == "x") {
                    let x_var = free_vars.remove(x_pos);
                    free_vars.insert(0, x_var);
                }
                if let Some(first_v) = free_vars.first() {
                    let v_sym = graph.symbols.get_or_intern(first_v);
                    let mut comp_sub = Vec::new();
                    let mut comp_latex = Vec::new();

                    // Derivative
                    let diff_id = graph.diff(expr_id, v_sym);
                    let simp_diff = Simplifier::simplify(graph, diff_id).unwrap_or(diff_id);
                    let diff_str = unicode_fmt.format(graph, simp_diff).unwrap_or_default();
                    let diff_tex = latex_fmt.format(graph, simp_diff).unwrap_or_default();
                    comp_sub.push(format!("d/d{}: {}", first_v, diff_str));
                    comp_latex.push(format!("\\frac{{d}}{{d{}}} = {}", first_v, diff_tex));

                    // Indefinite Integral
                    if let Ok(int_id) = graph.integrate(expr_id, v_sym) {
                        let simp_int = Simplifier::simplify(graph, int_id).unwrap_or(int_id);
                        let int_str = unicode_fmt.format(graph, simp_int).unwrap_or_default();
                        let int_tex = latex_fmt.format(graph, simp_int).unwrap_or_default();
                        comp_sub.push(format!("∫ ... d{}: {} + C", first_v, int_str));
                        comp_latex.push(format!("\\int d{} = {} + C", first_v, int_tex));
                    }

                    if !comp_sub.is_empty() {
                        pods.push(
                            ResultPod::new(
                                "Calculus companion",
                                comp_sub.join("\n"),
                                comp_latex.join(", \\quad "),
                                comp_sub.join("  |  "),
                            )
                            .with_subpods(comp_sub),
                        );
                    }

                    // Roots / Zeros
                    if let Ok(roots) = graph.solveset(expr_id, v_sym)
                        && !roots.is_empty()
                    {
                        let root_strs: Vec<String> = roots
                            .iter()
                            .map(|&r| unicode_fmt.format(graph, r).unwrap_or_default())
                            .collect();
                        let root_tex: Vec<String> = roots
                            .iter()
                            .map(|&r| latex_fmt.format(graph, r).unwrap_or_default())
                            .collect();
                        let roots_display = format!("{} = {}", first_v, root_strs.join(", "));
                        pods.push(ResultPod::new(
                            "Roots / Zeros",
                            roots_display.clone(),
                            format!("{} \\in \\left\\{{ {} \\right\\}}", first_v, root_tex.join(", ")),
                            roots_display,
                        ));
                    }
                }
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        let elapsed = start_time.elapsed().as_micros() as u64;
        #[cfg(target_arch = "wasm32")]
        let elapsed = 0u64;

        QueryResponse {
            query: query_trimmed.to_string(),
            intent: intent_name.to_string(),
            pods,
            execution_time_us: elapsed,
        }
    }

    fn format_input_interpretation(
        _graph: &ExprGraph,
        op: &MathOperation,
        raw: &str,
    ) -> (String, String, String) {
        match op {
            MathOperation::Differentiate {
                expression,
                variable,
                order,
                ..
            } => {
                let v = variable.as_deref().unwrap_or("x");
                let text = format!("d^{}/d{}^{} [{}]", order, v, order, expression);
                let latex = format!(
                    "\\frac{{d^{}}}{{d{}^{}}} \\left[ {} \\right]",
                    order, v, order, expression
                );
                let unicode = format!("d^{}/d{}^{} [{}]", order, v, order, expression);
                (text, latex, unicode)
            }
            MathOperation::Integrate {
                expression,
                variable,
                lower,
                upper,
            } => {
                let v = variable.as_deref().unwrap_or("x");
                if let (Some(l), Some(u)) = (lower, upper) {
                    let text = format!("integral from {} to {} of {} d{}", l, u, expression, v);
                    let latex = format!("\\int_{{{}}}^{{{}}} {} \\, d{}", l, u, expression, v);
                    let unicode = format!("∫_{{{}}}^{{{}}} {} d{}", l, u, expression, v);
                    (text, latex, unicode)
                } else {
                    let text = format!("integral of {} d{}", expression, v);
                    let latex = format!("\\int {} \\, d{}", expression, v);
                    let unicode = format!("∫ {} d{}", expression, v);
                    (text, latex, unicode)
                }
            }
            MathOperation::Solve { equation, variable } => {
                let v = variable.as_deref().unwrap_or("x");
                let text = format!("solve {} for {}", equation, v);
                let latex = format!("\\text{{solve }} {} \\text{{ for }} {}", equation, v);
                let unicode = format!("solve {} for {}", equation, v);
                (text, latex, unicode)
            }
            MathOperation::SolveSystem {
                equations,
                variables,
            } => {
                let eqs = equations.join(", ");
                let vars = if variables.is_empty() {
                    "variables".to_string()
                } else {
                    variables.join(", ")
                };
                let text = format!("solve system [{}] for [{}]", eqs, vars);
                let latex = format!(
                    "\\text{{solve }} \\left\\{{ {} \\right\\}} \\text{{ for }} \\left({})\\right.",
                    eqs, vars
                );
                (text.clone(), latex, text)
            }
            MathOperation::Limit {
                expression,
                variable,
                point,
                ..
            } => {
                let text = format!("limit of {} as {} -> {}", expression, variable, point);
                let latex = format!("\\lim_{{{} \\to {}}} {}", variable, point, expression);
                let unicode = format!("lim_{{{} -> {}}} {}", variable, point, expression);
                (text, latex, unicode)
            }
            MathOperation::Series {
                expression,
                variable,
                point,
                order,
            } => {
                let text = format!(
                    "series expansion of {} around {} = {} to order {}",
                    expression, variable, point, order
                );
                let latex = format!(
                    "\\text{{series }}[{}, \\{{{}\\}}, {} = {}, \\text{{order }} {}]",
                    expression, variable, variable, point, order
                );
                let unicode = format!(
                    "Taylor series of {} around {} = {} to order {}",
                    expression, variable, point, order
                );
                (text, latex, unicode)
            }
            _ => (
                raw.to_string(),
                format!("\\text{{{}}}", raw),
                raw.to_string(),
            ),
        }
    }

    fn generate_step_by_step_pod(graph: &ExprGraph, op: &MathOperation) -> Option<ResultPod> {
        let unicode_fmt = UnicodeFormatter;
        let latex_fmt = LatexFormatter;
        match op {
            MathOperation::Differentiate {
                expression,
                variable,
                ..
            } => {
                let parser = algebra_core::parser::ExprParser::new(graph);
                let expr_id = parser.parse(expression).ok()?;
                let var_str = variable.as_deref().unwrap_or("x");
                let wrt_sym = graph.symbols.get_or_intern(var_str);
                let step_res = graph.diff_with_steps(expr_id, wrt_sym);
                if !step_res.steps.is_empty() {
                    let mut lines = Vec::new();
                    let mut tex_lines = Vec::new();
                    for s in &step_res.steps {
                        let to_str = unicode_fmt.format(graph, s.after_expr).unwrap_or_default();
                        let to_tex = latex_fmt.format(graph, s.after_expr).unwrap_or_default();
                        lines.push(format!("{}: {} -> {}", s.rule_name, s.description, to_str));
                        tex_lines.push(format!("\\text{{{}: }} {}", s.rule_name, to_tex));
                    }
                    return Some(
                        ResultPod::new(
                            "Step-by-step solution",
                            lines.join("\n"),
                            format!("\\begin{{aligned}} {} \\end{{aligned}}", tex_lines.join(" \\\\ ")),
                            lines.join("\n"),
                        )
                        .with_subpods(lines),
                    );
                }
            }
            MathOperation::Integrate {
                expression,
                variable,
                lower: None,
                upper: None,
            } => {
                let parser = algebra_core::parser::ExprParser::new(graph);
                let expr_id = parser.parse(expression).ok()?;
                let var_str = variable.as_deref().unwrap_or("x");
                let wrt_sym = graph.symbols.get_or_intern(var_str);
                if let Ok(step_res) = graph.integrate_with_steps(expr_id, wrt_sym)
                    && !step_res.steps.is_empty()
                {
                    let mut lines = Vec::new();
                    let mut tex_lines = Vec::new();
                    for s in &step_res.steps {
                        let to_str = unicode_fmt.format(graph, s.after_expr).unwrap_or_default();
                        let to_tex = latex_fmt.format(graph, s.after_expr).unwrap_or_default();
                        lines.push(format!("{}: {} -> {}", s.rule_name, s.description, to_str));
                        tex_lines.push(format!("\\text{{{}: }} {}", s.rule_name, to_tex));
                    }
                    return Some(
                        ResultPod::new(
                            "Step-by-step solution",
                            lines.join("\n"),
                            format!("\\begin{{aligned}} {} \\end{{aligned}}", tex_lines.join(" \\\\ ")),
                            lines.join("\n"),
                        )
                        .with_subpods(lines),
                    );
                }
            }
            MathOperation::Solve { equation, variable } => {
                let parser = algebra_core::parser::ExprParser::new(graph);
                let eq_id = parser.parse(equation).ok()?;
                let var_str = variable.as_deref().unwrap_or("x");
                let wrt_sym = graph.symbols.get_or_intern(var_str);
                if let Ok(step_res) = graph.solveset_with_steps(eq_id, wrt_sym)
                    && !step_res.steps.is_empty()
                {
                    let mut lines = Vec::new();
                    let mut tex_lines = Vec::new();
                    for s in &step_res.steps {
                        let to_str = unicode_fmt.format(graph, s.after_expr).unwrap_or_default();
                        let to_tex = latex_fmt.format(graph, s.after_expr).unwrap_or_default();
                        lines.push(format!("{}: {} -> {}", s.rule_name, s.description, to_str));
                        tex_lines.push(format!("\\text{{{}: }} {}", s.rule_name, to_tex));
                    }
                    return Some(
                        ResultPod::new(
                            "Step-by-step solution",
                            lines.join("\n"),
                            format!("\\begin{{aligned}} {} \\end{{aligned}}", tex_lines.join(" \\\\ ")),
                            lines.join("\n"),
                        )
                        .with_subpods(lines),
                    );
                }
            }
            _ => {}
        }
        None
    }

    /// Preprocess natural user queries into standard mathematical syntax recognized by `parse_operation`.
    pub fn preprocess_natural_query(input: &str) -> String {
        let mut s = input.trim().to_string();

        // 1. General d/d<var> derivative syntax: d/d<var>(f) or d/d<var> f -> diff f, <var>
        if let Some(after_dd) = s.strip_prefix("d/d") {
            let var_end = after_dd
                .find(|c: char| !c.is_alphanumeric() && c != '_')
                .unwrap_or(after_dd.len());
            let var_name = &after_dd[..var_end];
            if !var_name.is_empty() {
                let rest = after_dd[var_end..].trim();
                let inner = rest
                    .strip_prefix('(')
                    .and_then(|r| r.strip_suffix(')'))
                    .unwrap_or(rest)
                    .trim();
                if !inner.is_empty() {
                    s = format!("diff {}, {}", inner, var_name);
                }
            }
        }

        // 2. Unicode integral: ∫ ... dx -> integrate ...
        if s.starts_with('∫') {
            s = s.trim_start_matches('∫').trim().to_string();
            if let Some(stripped) = s.strip_suffix("dx") {
                s = format!("integrate {}", stripped.trim());
            } else if let Some(stripped) = s.strip_suffix("dt") {
                s = format!("integrate {}, t", stripped.trim());
            } else if let Some(stripped) = s.strip_suffix("du") {
                s = format!("integrate {}, u", stripped.trim());
            } else if let Some(stripped) = s.strip_suffix("dy") {
                s = format!("integrate {}, y", stripped.trim());
            } else {
                s = format!("integrate {}", s);
            }
        }

        // 3. Normalize e^(...) -> exp(...) with nested term normalization
        while let Some(idx) = s.find("e^(") {
            let after = &s[idx + 3..];
            if let Some(close_idx) = after.find(')') {
                let inner = &after[..close_idx];
                let mut inner_norm = inner.to_string();
                for num in ["2", "3", "4", "5", "6", "7", "8", "9"] {
                    for var in ["x", "t", "y", "z"] {
                        inner_norm = inner_norm
                            .replace(&format!("{}{}", num, var), &format!("{}*{}", num, var));
                    }
                }
                let rep = format!("exp({})", inner_norm);
                let full = &s[idx..idx + 4 + close_idx];
                s = s.replace(full, &rep);
                continue;
            }
            break;
        }

        // 4. Normalize e^x, e^t, e^y, e^z -> exp(x), exp(t), etc.
        s = s
            .replace("e^x", "exp(x)")
            .replace("e^t", "exp(t)")
            .replace("e^y", "exp(y)")
            .replace("e^z", "exp(z)");

        // 5. Implicit coefficient multiplication: 3exp(x) -> 3*exp(x)
        for num in ["2", "3", "4", "5", "6", "7", "8", "9"] {
            for fn_name in [
                "exp", "sin", "cos", "tan", "ln", "log", "erf", "zeta", "gamma",
            ] {
                let target = format!("{}{}(", num, fn_name);
                let rep = format!("{}*{}(", num, fn_name);
                s = s.replace(&target, &rep);
            }
        }

        // 6. Strip trailing 'dx' in integrals: 'integrate x * sin(x) dx' -> 'integrate x * sin(x)'
        if s.starts_with("integrate ") && s.ends_with(" dx") {
            s = s[..s.len() - 3].trim().to_string();
        }

        s
    }
}
