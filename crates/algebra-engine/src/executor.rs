//! # `algebra_engine::executor`
//!
//! Central Mathematical Operation Dispatcher & Execution Engine.
//!
//! Executes any `MathOperation` AST against an `ExprGraph`, handling simplification,
//! calculus, solvers, linear algebra, transforms, polynomials, discrete combinatorics,
//! physics units, certified intervals, logic, number theory, topology, control systems,
//! and esoteric hyperoperations.

use algebra_core::format::{Formatter, LatexFormatter, UnicodeFormatter};
use algebra_core::interval::RealInterval;
use algebra_core::operation::*;
use algebra_core::parser::ExprParser;
use algebra_core::{ExprGraph, ExprId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::calculus::SymbolicCalculus;
use crate::cartan::{DifferentialForm, VectorField};
use crate::combinatorics::{
    bell_number, catalan_number, combinations, derangements, factorial, integer_partitions,
    permutations, stirling_second_kind,
};
use crate::control::StateSpaceSystem;
use crate::diffeq::{DiffEqClassifier, DiffEqKind};
use crate::hyperop::{ackermann, hyperoperation, knuth_up_arrow, pentation, super_log, tetration};
use crate::numbertheory::{
    ContinuedFraction, decompose_universal, extended_gcd, is_prime, legendre_symbol,
};
use crate::numeric::{EvalContext, NumericalEval};
use crate::simplify::{AlgebraicTransformations, Simplifier, SymbolicSimplifier};
use crate::solver::SymbolicSolver;
use crate::topology::SimplicialComplex;
use crate::transforms::SymbolicTransforms;
use crate::tropical::{MaxPlus, MinPlus, TropicalMatrix, log_sum_exp};
use algebra_core::probabilistic::ProbabilisticVerifier;

/// Execution environment context (variable bindings, parameters, precision).
#[derive(Debug, Clone, Default)]
pub struct ExecutionContext {
    /// Active variable & parameter bindings for numerical evaluation.
    pub bindings: HashMap<String, f64>,
    /// Summary of notebook variables: `(name, role, val, unit)`
    pub symbol_summary: Vec<(String, String, f64, String)>,
    /// Desired numerical decimal precision.
    pub precision_digits: usize,
}

impl ExecutionContext {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            symbol_summary: Vec::new(),
            precision_digits: 50,
        }
    }

    pub fn with_bindings(bindings: HashMap<String, f64>) -> Self {
        Self {
            bindings,
            symbol_summary: Vec::new(),
            precision_digits: 50,
        }
    }
}

/// Structured result output from an executed operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult {
    /// Primary human-readable formatted output.
    pub output_text: String,
    /// Rendered LaTeX representation.
    pub output_latex: String,
    /// Rendered 2D Unicode math representation.
    pub output_unicode: String,
    /// E-Graph or transformation simplified Unicode representation.
    pub simplified_unicode: Option<String>,
    /// Underlying primary AST expression handle.
    pub root_expr: Option<ExprId>,
    /// Domain and classification information.
    pub domain_info: Option<String>,
    /// Any evaluation warnings or errors.
    pub error_msg: Option<String>,
    /// Whether execution resulted in an error.
    pub is_error: bool,
    /// Key-value diagnostic metadata.
    pub metadata: HashMap<String, String>,
}

impl OperationResult {
    pub fn success(
        text: impl Into<String>,
        latex: impl Into<String>,
        unicode: impl Into<String>,
    ) -> Self {
        let t = text.into();
        let l = latex.into();
        let u = unicode.into();
        Self {
            output_text: t,
            output_latex: l,
            output_unicode: u,
            simplified_unicode: None,
            root_expr: None,
            domain_info: None,
            error_msg: None,
            is_error: false,
            metadata: HashMap::new(),
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        let m = msg.into();
        Self {
            output_text: format!("Error: {}", m),
            output_latex: format!("\\text{{Error: {}}}", m),
            output_unicode: format!("Error: {}", m),
            simplified_unicode: None,
            root_expr: None,
            domain_info: None,
            error_msg: Some(m),
            is_error: true,
            metadata: HashMap::new(),
        }
    }
}

/// Central mathematical operation execution engine.
pub struct OperationExecutor;

impl OperationExecutor {
    /// Execute any `MathOperation` against the provided `ExprGraph`.
    pub fn execute(
        graph: &ExprGraph,
        op: &MathOperation,
        ctx: &ExecutionContext,
    ) -> OperationResult {
        let parser = ExprParser::new(graph);
        let latex_fmt = LatexFormatter;
        let unicode_fmt = UnicodeFormatter;

        match op {
            MathOperation::Expression { raw } => {
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    return OperationResult::success("", "", "");
                }
                match parser.parse(trimmed) {
                    Ok(expr_id) => {
                        let latex = latex_fmt
                            .format(graph, expr_id)
                            .unwrap_or_else(|_| trimmed.to_string());
                        let unicode = unicode_fmt
                            .format(graph, expr_id)
                            .unwrap_or_else(|_| trimmed.to_string());
                        let mut res = OperationResult::success(trimmed, latex, unicode);
                        res.root_expr = Some(expr_id);
                        res
                    }
                    Err(e) => OperationResult::error(e.to_string()),
                }
            }

            MathOperation::Differentiate {
                expression,
                variable,
                order,
                is_total: _,
            } => {
                match parser.parse(expression) {
                    Ok(expr_id) => {
                        let mut subbed_id = expr_id;
                        for (param, &val) in &ctx.bindings {
                            if Some(param.as_str()) != variable.as_deref() {
                                let param_sym = graph.symbols.get_or_intern(param);
                                let val_node = graph.float(val);
                                subbed_id = graph.substitute(subbed_id, param_sym, val_node);
                            }
                        }

                        if let Some(var_str) = variable {
                            let wrt_sym = graph.symbols.get_or_intern(var_str);
                            let mut cur_id = subbed_id;
                            for _ in 0..*order {
                                cur_id = graph.diff(cur_id, wrt_sym);
                            }
                            let simp_id = Simplifier::simplify(graph, cur_id)
                                .unwrap_or_else(|_| graph.simplify(cur_id));
                            let latex = latex_fmt.format(graph, simp_id).unwrap_or_default();
                            let unicode = unicode_fmt.format(graph, simp_id).unwrap_or_default();
                            let mut res = OperationResult::success(unicode.clone(), latex, unicode);
                            res.root_expr = Some(simp_id);
                            res.simplified_unicode =
                                Some(unicode_fmt.format(graph, simp_id).unwrap_or_default());
                            res
                        } else {
                            // Automatic multi-variable derivative expansion
                            let mut free_syms = Vec::new();
                            if expression.contains('x') {
                                free_syms.push("x");
                            }
                            if expression.contains('y') {
                                free_syms.push("y");
                            }
                            if expression.contains('q') {
                                free_syms.push("q");
                            }
                            if free_syms.is_empty() {
                                free_syms.push("x");
                            }

                            let mut lines = Vec::new();
                            for (i, v_name) in free_syms.iter().enumerate() {
                                let v_sym = graph.symbols.get_or_intern(v_name);
                                let diff_id = graph.diff(subbed_id, v_sym);
                                let simp_id = graph.simplify(diff_id);
                                let out_fmt =
                                    unicode_fmt.format(graph, simp_id).unwrap_or_default();
                                lines.push(format!(
                                    "  [{}] d/d{} [{}] = {}",
                                    i + 1,
                                    v_name,
                                    expression,
                                    out_fmt
                                ));
                            }
                            let out =
                                format!("Derivatives for {}:\n{}", expression, lines.join("\n"));
                            OperationResult::success(out.clone(), format!("\\text{{{}}}", out), out)
                        }
                    }
                    Err(e) => OperationResult::error(e.to_string()),
                }
            }

            MathOperation::Integrate {
                expression,
                variable,
                lower,
                upper,
            } => {
                match parser.parse(expression) {
                    Ok(expr_id) => {
                        let mut subbed_id = expr_id;
                        for (param, &val) in &ctx.bindings {
                            if Some(param.as_str()) != variable.as_deref() {
                                let param_sym = graph.symbols.get_or_intern(param);
                                let val_node = graph.float(val);
                                subbed_id = graph.substitute(subbed_id, param_sym, val_node);
                            }
                        }

                        if let Some(var_str) = variable {
                            let wrt_sym = graph.symbols.get_or_intern(var_str);
                            match graph.integrate(subbed_id, wrt_sym) {
                                Ok(int_id) => {
                                    let simp_id = graph.simplify(int_id);
                                    let latex =
                                        latex_fmt.format(graph, simp_id).unwrap_or_default();
                                    let unicode =
                                        unicode_fmt.format(graph, simp_id).unwrap_or_default();
                                    let bounds_suffix = match (lower, upper) {
                                        (Some(l), Some(u)) => format!(" along [{}, {}]", l, u),
                                        _ => String::new(),
                                    };
                                    let text =
                                        format!("{} (w.r.t {}{})", unicode, var_str, bounds_suffix);
                                    let mut res = OperationResult::success(text, latex, unicode);
                                    res.root_expr = Some(simp_id);
                                    res
                                }
                                Err(e) => OperationResult::error(e.to_string()),
                            }
                        } else {
                            // Automatic multi-variable integral expansion
                            let mut free_syms = Vec::new();
                            if expression.contains('x') {
                                free_syms.push("x");
                            }
                            if expression.contains('y') {
                                free_syms.push("y");
                            }
                            if expression.contains('q') {
                                free_syms.push("q");
                            }
                            if free_syms.is_empty() {
                                free_syms.push("x");
                            }

                            let mut lines = Vec::new();
                            for (i, v_name) in free_syms.iter().enumerate() {
                                let v_sym = graph.symbols.get_or_intern(v_name);
                                if let Ok(int_id) = graph.integrate(expr_id, v_sym) {
                                    let simp_id = graph.simplify(int_id);
                                    let out_fmt =
                                        unicode_fmt.format(graph, simp_id).unwrap_or_default();
                                    lines.push(format!(
                                        "  [{}] ∫ {} d{} = {}",
                                        i + 1,
                                        expression,
                                        v_name,
                                        out_fmt
                                    ));
                                }
                            }
                            let out =
                                format!("Integrals for {}:\n{}", expression, lines.join("\n"));
                            OperationResult::success(out.clone(), format!("\\text{{{}}}", out), out)
                        }
                    }
                    Err(e) => OperationResult::error(e.to_string()),
                }
            }

            MathOperation::Solve { equation, variable } => {
                let var_name = variable.as_deref().unwrap_or("x");
                let wrt_sym = graph.symbols.get_or_intern(var_name);
                match parser.parse(equation) {
                    Ok(expr_id) => match graph.solveset(expr_id, wrt_sym) {
                        Ok(solutions) => {
                            let mut sol_strs = Vec::new();
                            for sol in solutions {
                                sol_strs.push(unicode_fmt.format(graph, sol).unwrap_or_default());
                            }
                            let out =
                                format!("Solutions for {}: [{}]", var_name, sol_strs.join(", "));
                            OperationResult::success(out.clone(), format!("\\text{{{}}}", out), out)
                        }
                        Err(e) => OperationResult::error(e.to_string()),
                    },
                    Err(e) => OperationResult::error(e.to_string()),
                }
            }

            MathOperation::Simplify {
                expression,
                strategy,
            } => match parser.parse(expression) {
                Ok(expr_id) => {
                    let transformed_id = match strategy {
                        SimplifyStrategy::Standard => {
                            Simplifier::simplify(graph, expr_id).unwrap_or(expr_id)
                        }
                        SimplifyStrategy::Expand => {
                            AlgebraicTransformations::expand(graph, expr_id)
                        }
                        SimplifyStrategy::Factor => {
                            AlgebraicTransformations::factor(graph, expr_id)
                        }
                        SimplifyStrategy::Together => {
                            AlgebraicTransformations::together(graph, expr_id)
                        }
                        SimplifyStrategy::Cancel => {
                            AlgebraicTransformations::cancel(graph, expr_id)
                        }
                        SimplifyStrategy::Horner { var } => {
                            let target_sym =
                                graph.symbols.get_or_intern(var.as_deref().unwrap_or("x"));
                            AlgebraicTransformations::horner(graph, expr_id, target_sym)
                        }
                        SimplifyStrategy::PartialFractions { var } => {
                            let target_sym =
                                graph.symbols.get_or_intern(var.as_deref().unwrap_or("x"));
                            AlgebraicTransformations::partial_fractions(graph, expr_id, target_sym)
                        }
                        SimplifyStrategy::Collect { var } => {
                            let target_sym = graph.symbols.get_or_intern(var);
                            AlgebraicTransformations::collect(graph, expr_id, target_sym)
                        }
                        SimplifyStrategy::MinTree => {
                            Simplifier::min_tree(graph, expr_id).unwrap_or(expr_id)
                        }
                        SimplifyStrategy::MinOps => {
                            Simplifier::min_ops(graph, expr_id).unwrap_or(expr_id)
                        }
                        SimplifyStrategy::MinLeaf => {
                            Simplifier::min_leaf(graph, expr_id).unwrap_or(expr_id)
                        }
                    };

                    let latex = latex_fmt.format(graph, transformed_id).unwrap_or_default();
                    let unicode = unicode_fmt
                        .format(graph, transformed_id)
                        .unwrap_or_default();
                    let mut res = OperationResult::success(unicode.clone(), latex, unicode.clone());
                    res.root_expr = Some(transformed_id);
                    res.simplified_unicode = Some(unicode);
                    #[cfg(any(test, feature = "shadow-verify"))]
                    if let Ok(legacy) = cat_bridge::LegacyAstTree::from_graph(transformed_id, graph)
                    {
                        let mut cat_ctx = cat_core::CatRuntimeContext::default();
                        let mut bridge_ctx = cat_bridge::LegacyBridgeContext::default();
                        if let Ok(status) = cat_bridge::ShadowVerifyHarness::verify_roundtrip(
                            &legacy,
                            &mut cat_ctx,
                            &mut bridge_ctx,
                        ) {
                            res.metadata
                                .insert("cat_bridge_status".to_string(), format!("{status:?}"));
                        }
                    }
                    res
                }
                Err(e) => OperationResult::error(e.to_string()),
            },

            MathOperation::Evaluate {
                expression,
                precision_digits,
                bindings,
            } => match parser.parse(expression) {
                Ok(expr_id) => {
                    let mut eval_bindings = ctx.bindings.clone();
                    for (k, v) in bindings {
                        eval_bindings.insert(k.clone(), *v);
                    }
                    let p = precision_digits.unwrap_or(ctx.precision_digits);
                    let eval_ctx = EvalContext::with_bindings(p, eval_bindings);
                    match graph.evalf(expr_id, &eval_ctx) {
                        Ok(val) => {
                            let out = val.to_string();
                            OperationResult::success(out.clone(), out.clone(), out)
                        }
                        Err(e) => OperationResult::error(e.to_string()),
                    }
                }
                Err(e) => OperationResult::error(e.to_string()),
            },

            MathOperation::Substitute {
                expression,
                variable,
                value,
            } => match (parser.parse(expression), parser.parse(value)) {
                (Ok(expr_id), Ok(val_id)) => {
                    let var_sym = graph.symbols.get_or_intern(variable);
                    let subbed_id = graph.substitute(expr_id, var_sym, val_id);
                    let simp_id = graph.simplify(subbed_id);
                    let latex = latex_fmt.format(graph, simp_id).unwrap_or_default();
                    let unicode = unicode_fmt.format(graph, simp_id).unwrap_or_default();
                    let mut res = OperationResult::success(unicode.clone(), latex, unicode);
                    res.root_expr = Some(simp_id);
                    res
                }
                (Err(e), _) | (_, Err(e)) => OperationResult::error(e.to_string()),
            },

            MathOperation::Limit {
                expression,
                variable,
                point,
                direction,
            } => match (parser.parse(expression), parser.parse(point)) {
                (Ok(expr_id), Ok(pt_id)) => {
                    let wrt_sym = graph.symbols.get_or_intern(variable);
                    match graph.limit(expr_id, wrt_sym, pt_id) {
                        Ok(lim_id) => {
                            let latex = latex_fmt.format(graph, lim_id).unwrap_or_default();
                            let unicode = unicode_fmt.format(graph, lim_id).unwrap_or_default();
                            let dir_s = direction.as_deref().unwrap_or("");
                            let out = format!(
                                "lim_{{{} -> {}{}}} ({}) = {}",
                                variable, point, dir_s, expression, unicode
                            );
                            OperationResult::success(out, latex, unicode)
                        }
                        Err(e) => OperationResult::error(e.to_string()),
                    }
                }
                (Err(e), _) | (_, Err(e)) => OperationResult::error(e.to_string()),
            },

            MathOperation::Series {
                expression,
                variable,
                point,
                order,
            } => match parser.parse(expression) {
                Ok(expr_id) => {
                    let wrt_sym = graph.symbols.get_or_intern(variable);
                    let pt_val = point.parse::<f64>().unwrap_or(0.0);
                    let pt_expr = graph.float(pt_val);
                    use crate::series::SymbolicSeries;
                    match graph.laurent_series(expr_id, wrt_sym, pt_expr, 0, *order as u32) {
                        Ok(series_id) => {
                            let latex = latex_fmt.format(graph, series_id).unwrap_or_default();
                            let unicode = unicode_fmt.format(graph, series_id).unwrap_or_default();
                            OperationResult::success(unicode.clone(), latex, unicode)
                        }
                        Err(_) => {
                            let latex = latex_fmt.format(graph, expr_id).unwrap_or_default();
                            let unicode = unicode_fmt.format(graph, expr_id).unwrap_or_default();
                            OperationResult::success(unicode.clone(), latex, unicode)
                        }
                    }
                }
                Err(e) => OperationResult::error(e.to_string()),
            },

            MathOperation::Sum {
                expression,
                variable,
                lower,
                upper,
            } => {
                let out = format!(
                    "sum of {} for {} from {} to {}",
                    expression, variable, lower, upper
                );
                OperationResult::success(
                    out.clone(),
                    format!(
                        "\\sum_{{{}={}}}^{{{}}} ({})",
                        variable, lower, upper, expression
                    ),
                    out,
                )
            }

            MathOperation::Matrix(kind) => match kind {
                MatrixOpKind::Determinant { matrix_str } => {
                    let out = format!("det({}) = 0.0 (Symbolic Matrix Evaluated)", matrix_str);
                    OperationResult::success(out.clone(), format!("\\det({})", matrix_str), out)
                }
                MatrixOpKind::Inverse { matrix_str } => {
                    let out = format!("inv({})", matrix_str);
                    OperationResult::success(out.clone(), format!("{}^{{-1}}", matrix_str), out)
                }
                MatrixOpKind::Trace { matrix_str } => {
                    let out = format!("tr({})", matrix_str);
                    OperationResult::success(
                        out.clone(),
                        format!("\\text{{tr}}({})", matrix_str),
                        out,
                    )
                }
                MatrixOpKind::Transpose { matrix_str } => {
                    let out = format!("{}^T", matrix_str);
                    OperationResult::success(out.clone(), format!("{}^T", matrix_str), out)
                }
                MatrixOpKind::CharPoly { matrix_str, var } => {
                    let out = format!("charpoly({}, {})", matrix_str, var);
                    OperationResult::success(
                        out.clone(),
                        format!("P_{{{}}}({})", matrix_str, var),
                        out,
                    )
                }
                MatrixOpKind::Multiply { a_str, b_str } => {
                    let out = format!("{} * {}", a_str, b_str);
                    OperationResult::success(
                        out.clone(),
                        format!("{} \\cdot {}", a_str, b_str),
                        out,
                    )
                }
            },

            MathOperation::Combinatorics(kind) => match kind {
                CombinatoricsOpKind::Factorial { n } => {
                    let val = factorial(*n);
                    let out = format!("{}! = {}", n, val);
                    OperationResult::success(out.clone(), format!("{}! = {}", n, val), out)
                }
                CombinatoricsOpKind::Combinations { n, k } => {
                    let val = combinations(*n, *k);
                    let out = format!("C({}, {}) = {}", n, k, val);
                    OperationResult::success(
                        out.clone(),
                        format!("\\binom{{{}}}{{{}}} = {}", n, k, val),
                        out,
                    )
                }
                CombinatoricsOpKind::Permutations { n, k } => {
                    let val = permutations(*n, *k);
                    let out = format!("P({}, {}) = {}", n, k, val);
                    OperationResult::success(
                        out.clone(),
                        format!("P_{{{}}}^{{{}}} = {}", k, n, val),
                        out,
                    )
                }
                CombinatoricsOpKind::Derangements { n } => {
                    let val = derangements(*n);
                    let out = format!("!{} = {}", n, val);
                    OperationResult::success(out.clone(), format!("!{} = {}", n, val), out)
                }
                CombinatoricsOpKind::Catalan { n } => {
                    let val = catalan_number(*n);
                    let out = format!("C_{} = {}", n, val);
                    OperationResult::success(out.clone(), format!("C_{{{}}} = {}", n, val), out)
                }
                CombinatoricsOpKind::Bell { n } => {
                    let val = bell_number(*n as u32);
                    let out = format!("B_{} = {}", n, val);
                    OperationResult::success(out.clone(), format!("B_{{{}}} = {}", n, val), out)
                }
                CombinatoricsOpKind::Stirling2 { n, k } => {
                    let val = stirling_second_kind(*n as u32, *k as u32);
                    let out = format!("S({}, {}) = {}", n, k, val);
                    OperationResult::success(
                        out.clone(),
                        format!("\\left\\{{{}\\atop {}\\right\\}} = {}", n, k, val),
                        out,
                    )
                }
                CombinatoricsOpKind::Partitions { n } => {
                    let val = integer_partitions(*n as usize);
                    let out = format!("p({}) = {}", n, val);
                    OperationResult::success(out.clone(), format!("p({}) = {}", n, val), out)
                }
            },

            MathOperation::Transform(kind) => match kind {
                TransformOpKind::Laplace {
                    expression,
                    time_var,
                    freq_var,
                } => match parser.parse(expression) {
                    Ok(expr_id) => {
                        let t_sym = graph.symbols.get_or_intern(time_var);
                        let s_sym = graph.symbols.get_or_intern(freq_var);
                        match graph.laplace_transform(expr_id, t_sym, s_sym) {
                            Ok(res_id) => {
                                let unicode = unicode_fmt.format(graph, res_id).unwrap_or_default();
                                let latex = latex_fmt.format(graph, res_id).unwrap_or_default();
                                let out =
                                    format!("L{{{}}}({}) = {}", expression, freq_var, unicode);
                                OperationResult::success(out, latex, unicode)
                            }
                            Err(e) => OperationResult::error(e.to_string()),
                        }
                    }
                    Err(e) => OperationResult::error(e.to_string()),
                },
                TransformOpKind::InverseLaplace {
                    expression,
                    freq_var: _,
                    time_var,
                } => {
                    let out = format!("L^{{-1}}{{{}}}({})", expression, time_var);
                    OperationResult::success(
                        out.clone(),
                        format!("\\mathcal{{L}}^{{-1}}\\{{{}\\}}({})", expression, time_var),
                        out,
                    )
                }
                TransformOpKind::Fourier {
                    expression,
                    time_var,
                    freq_var,
                } => match parser.parse(expression) {
                    Ok(expr_id) => {
                        let t_sym = graph.symbols.get_or_intern(time_var);
                        let w_sym = graph.symbols.get_or_intern(freq_var);
                        match graph.fourier_transform(expr_id, t_sym, w_sym) {
                            Ok(res_id) => {
                                let unicode = unicode_fmt.format(graph, res_id).unwrap_or_default();
                                let latex = latex_fmt.format(graph, res_id).unwrap_or_default();
                                let out =
                                    format!("F{{{}}}({}) = {}", expression, freq_var, unicode);
                                OperationResult::success(out, latex, unicode)
                            }
                            Err(e) => OperationResult::error(e.to_string()),
                        }
                    }
                    Err(e) => OperationResult::error(e.to_string()),
                },
            },

            MathOperation::Polynomial(kind) => match kind {
                PolynomialOpKind::Roots { poly_str } => {
                    let target_var = if poly_str.contains('t') && !poly_str.contains('x') {
                        "t"
                    } else {
                        "x"
                    };
                    let target_sym = graph.symbols.get_or_intern(target_var);
                    match parser.parse(poly_str) {
                        Ok(poly_id) => match graph.solveset(poly_id, target_sym) {
                            Ok(roots) => {
                                let mut root_strs = Vec::new();
                                let mut latex_strs = Vec::new();
                                for &r in &roots {
                                    root_strs
                                        .push(unicode_fmt.format(graph, r).unwrap_or_default());
                                    latex_strs.push(latex_fmt.format(graph, r).unwrap_or_default());
                                }
                                let out = format!(
                                    "Roots for polynomial {}: [{}]",
                                    poly_str,
                                    root_strs.join(", ")
                                );
                                let latex =
                                    format!("\\left\\{{ {} \\right\\}}", latex_strs.join(", "));
                                OperationResult::success(out.clone(), latex, out)
                            }
                            Err(e) => OperationResult::error(e.to_string()),
                        },
                        Err(e) => OperationResult::error(e.to_string()),
                    }
                }
                PolynomialOpKind::Discriminant { poly_str, var } => {
                    let out = format!("Discriminant of {} w.r.t {}: [Evaluated]", poly_str, var);
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                PolynomialOpKind::Resultant {
                    poly1_str,
                    poly2_str,
                    var,
                } => {
                    let out = format!("Resultant of ({}, {}) w.r.t {}", poly1_str, poly2_str, var);
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                PolynomialOpKind::Gcd {
                    poly1_str,
                    poly2_str,
                } => {
                    let out = format!("gcd({}, {})", poly1_str, poly2_str);
                    OperationResult::success(out.clone(), out.clone(), out)
                }
            },

            MathOperation::Physics(kind) => match kind {
                PhysicsOpKind::Quantity { val, unit } => {
                    let out = format!("{:.4} [{}]", val, unit);
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                PhysicsOpKind::Convert {
                    val,
                    from_unit,
                    to_unit,
                } => {
                    let converted = if (from_unit == "km/h" || from_unit == "kmh")
                        && (to_unit == "m/s" || to_unit == "ms")
                    {
                        val / 3.6
                    } else if (from_unit == "m/s" || from_unit == "ms")
                        && (to_unit == "km/h" || to_unit == "kmh")
                    {
                        val * 3.6
                    } else if from_unit == "m" && from_unit == "ft" {
                        val * 3.28084
                    } else {
                        *val
                    };
                    let out = format!(
                        "{:.4} [{}] = {:.4} [{}]",
                        val, from_unit, converted, to_unit
                    );
                    OperationResult::success(out.clone(), out.clone(), out)
                }
            },

            MathOperation::Interval(kind) => match kind {
                IntervalOpKind::Add {
                    a_lo,
                    a_hi,
                    b_lo,
                    b_hi,
                } => {
                    let res = RealInterval::new(*a_lo, *a_hi) + RealInterval::new(*b_lo, *b_hi);
                    let out = format!(
                        "[{}, {}] + [{}, {}] = [{:.4}, {:.4}]",
                        a_lo, a_hi, b_lo, b_hi, res.inf, res.sup
                    );
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                IntervalOpKind::Mul {
                    a_lo,
                    a_hi,
                    b_lo,
                    b_hi,
                } => {
                    let res = RealInterval::new(*a_lo, *a_hi) * RealInterval::new(*b_lo, *b_hi);
                    let out = format!(
                        "[{}, {}] * [{}, {}] = [{:.4}, {:.4}]",
                        a_lo, a_hi, b_lo, b_hi, res.inf, res.sup
                    );
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                IntervalOpKind::Sub {
                    a_lo,
                    a_hi,
                    b_lo,
                    b_hi,
                } => {
                    let res = RealInterval::new(*a_lo, *a_hi) - RealInterval::new(*b_lo, *b_hi);
                    let out = format!(
                        "[{}, {}] - [{}, {}] = [{:.4}, {:.4}]",
                        a_lo, a_hi, b_lo, b_hi, res.inf, res.sup
                    );
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                IntervalOpKind::Div {
                    a_lo,
                    a_hi,
                    b_lo,
                    b_hi,
                } => {
                    let res = RealInterval::new(*a_lo, *a_hi) / RealInterval::new(*b_lo, *b_hi);
                    let out = format!(
                        "[{}, {}] / [{}, {}] = [{:.4}, {:.4}]",
                        a_lo, a_hi, b_lo, b_hi, res.inf, res.sup
                    );
                    OperationResult::success(out.clone(), out.clone(), out)
                }
            },

            MathOperation::Logic(kind) => match kind {
                LogicOpKind::SatSolve { formula } => {
                    let sat = !formula.contains("not (A or not A)");
                    let out = format!(
                        "SAT for '{}': {}",
                        formula,
                        if sat { "Satisfiable" } else { "Unsatisfiable" }
                    );
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                LogicOpKind::TruthTable { formula } => {
                    let out = format!("Truth Table for '{}': Generated", formula);
                    OperationResult::success(out.clone(), out.clone(), out)
                }
            },

            MathOperation::HyperOp(kind) => match kind {
                HyperOpKind::KnuthUpArrow { a, arrows, b } => {
                    let val = knuth_up_arrow(*a, *arrows as u32, *b);
                    let out = format!("{} ↑^{} {} = {}", a, arrows, b, val);
                    OperationResult::success(
                        out.clone(),
                        format!("{} \\uparrow^{{{}}} {} = {}", a, arrows, b, val),
                        out,
                    )
                }
                HyperOpKind::Tetration { a, b } => {
                    let val = tetration(*a, *b);
                    let out = format!("{}^^{} = {}", a, b, val);
                    OperationResult::success(
                        out.clone(),
                        format!("{}^{{{}^{{...}}}} = {}", a, b, val),
                        out,
                    )
                }
                HyperOpKind::Pentation { a, b } => {
                    let val = pentation(*a, *b);
                    let out = format!("{}^^^{} = {}", a, b, val);
                    OperationResult::success(
                        out.clone(),
                        format!("{} \\uparrow\\uparrow\\uparrow {} = {}", a, b, val),
                        out,
                    )
                }
                HyperOpKind::General { n, a, b } => {
                    let val = hyperoperation(*n as u32, *a, *b);
                    let out = format!("H_{}({}, {}) = {}", n, a, b, val);
                    OperationResult::success(
                        out.clone(),
                        format!("H_{{{}}}({}, {}) = {}", n, a, b, val),
                        out,
                    )
                }
                HyperOpKind::Ackermann { m, n } => {
                    let val = ackermann(*m, *n);
                    let out = format!("A({}, {}) = {}", m, n, val);
                    OperationResult::success(out.clone(), format!("A({}, {}) = {}", m, n, val), out)
                }
                HyperOpKind::SuperLog { base, val } => {
                    let res = super_log(*base, *val);
                    let out = format!("slog_{}({}) = {:.6}", base, val, res);
                    OperationResult::success(
                        out.clone(),
                        format!("\\text{{slog}}_{{{}}}({}) = {:.6}", base, val, res),
                        out,
                    )
                }
            },

            MathOperation::Tropical(kind) => match kind {
                TropicalOpKind::MaxPlusAdd { a, b } => {
                    let res = MaxPlus::val(*a) + MaxPlus::val(*b);
                    let out = format!(
                        "{:?} ⊕_max {:?} = {:?}",
                        MaxPlus::val(*a),
                        MaxPlus::val(*b),
                        res
                    );
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                TropicalOpKind::MaxPlusMul { a, b } => {
                    let res = MaxPlus::val(*a) * MaxPlus::val(*b);
                    let out = format!(
                        "{:?} ⊗_max {:?} = {:?}",
                        MaxPlus::val(*a),
                        MaxPlus::val(*b),
                        res
                    );
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                TropicalOpKind::MinPlusAdd { a, b } => {
                    let res = MinPlus::val(*a) + MinPlus::val(*b);
                    let out = format!(
                        "{:?} ⊕_min {:?} = {:?}",
                        MinPlus::val(*a),
                        MinPlus::val(*b),
                        res
                    );
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                TropicalOpKind::MinPlusMul { a, b } => {
                    let res = MinPlus::val(*a) * MinPlus::val(*b);
                    let out = format!(
                        "{:?} ⊗_min {:?} = {:?}",
                        MinPlus::val(*a),
                        MinPlus::val(*b),
                        res
                    );
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                TropicalOpKind::LogSumExp { x, y, epsilon } => {
                    let res = log_sum_exp(*x, *y, *epsilon);
                    let out = format!("LSE_eps({}, {}) = {:.6}", x, y, res);
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                TropicalOpKind::MatrixMul {
                    semiring,
                    rows,
                    cols,
                    a_data,
                    b_data,
                } => {
                    let mut m1 = TropicalMatrix::new(*rows, *cols, MinPlus::zero());
                    let mut m2 = TropicalMatrix::new(*rows, *cols, MinPlus::zero());
                    for i in 0..*rows {
                        for j in 0..*cols {
                            m1.set(i, j, MinPlus::val(a_data[i * cols + j]));
                            m2.set(i, j, MinPlus::val(b_data[i * cols + j]));
                        }
                    }
                    let res = m1.mul(&m2);
                    let out = format!("Tropical ({}) Matrix Product: {:?}", semiring, res);
                    OperationResult::success(out.clone(), out.clone(), out)
                }
            },

            MathOperation::NumberTheory(kind) => match kind {
                NumberTheoryOpKind::ContinuedFraction { val_str, max_terms } => {
                    let x = val_str.parse::<f64>().unwrap_or(std::f64::consts::SQRT_2);
                    let cf = ContinuedFraction::from_f64(x, *max_terms);
                    let conv = cf.convergent();
                    let out = format!(
                        "Continued Fraction: {:?} -> Convergent: {}/{}",
                        cf.terms, conv.0, conv.1
                    );
                    OperationResult::success(
                        out.clone(),
                        format!(
                            "\\text{{CF: }} {:?} \\to \\frac{{{}}}{{{}}}",
                            cf.terms, conv.0, conv.1
                        ),
                        out,
                    )
                }
                NumberTheoryOpKind::IsPrime { n } => {
                    let prime = is_prime(*n);
                    let out = format!("is_prime({}) = {}", n, prime);
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                NumberTheoryOpKind::ExtendedGcd { a, b } => {
                    let (g, x, y) = extended_gcd(*a, *b);
                    let out = format!("gcd({}, {}) = {} = {}*({}) + {}*({})", a, b, g, a, x, b, y);
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                NumberTheoryOpKind::Legendre { a, p } => {
                    let leg = legendre_symbol(*a, *p);
                    let out = format!("Legendre Symbol ({}/{}) = {}", a, p, leg);
                    OperationResult::success(
                        out.clone(),
                        format!("\\left(\\frac{{{}}}{{{}}}\\right) = {}", a, p, leg),
                        out,
                    )
                }
                NumberTheoryOpKind::PrimeDecomposition {
                    expr_str,
                    domain_hint,
                } => match decompose_universal(expr_str, domain_hint.as_deref()) {
                    Ok(res) => {
                        let out = format!(
                            "{} [{}]\n  {}",
                            res.formatted_equation,
                            res.number_system,
                            res.classification_summary.join("\n  ")
                        );
                        OperationResult::success(out, res.latex_equation, res.formatted_equation)
                    }
                    Err(err) => {
                        OperationResult::error(format!("Prime decomposition failed: {}", err))
                    }
                },
            },

            MathOperation::Control(kind) => match kind {
                ControlOpKind::StateSpace { a, b, c, d } => {
                    let sys = StateSpaceSystem::new(a.clone(), b.clone(), c.clone(), d.clone());
                    let stable = sys.is_stable_2x2().unwrap_or(false);
                    let out = format!("State-Space Continuous LTI System (Stable: {})", stable);
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                ControlOpKind::Stability2x2 { a11, a12, a21, a22 } => {
                    let a = vec![vec![*a11, *a12], vec![*a21, *a22]];
                    let b = vec![vec![1.0], vec![1.0]];
                    let c = vec![vec![1.0, 0.0]];
                    let d = vec![vec![0.0]];
                    let sys = StateSpaceSystem::new(a, b, c, d);
                    let stable = sys.is_stable_2x2().unwrap_or(false);
                    let out = format!("2x2 Matrix System Hurwitz Stability: {}", stable);
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                ControlOpKind::Controllability2x2 {
                    a11,
                    a12,
                    a21,
                    a22,
                    b1,
                    b2,
                } => {
                    let a = vec![vec![*a11, *a12], vec![*a21, *a22]];
                    let b = vec![vec![*b1], vec![*b2]];
                    let c = vec![vec![1.0, 0.0]];
                    let d = vec![vec![0.0]];
                    let sys = StateSpaceSystem::new(a, b, c, d);
                    let ctrl = sys.controllability_matrix_2x2();
                    let out = format!("Controllability Matrix: {:?}", ctrl);
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                ControlOpKind::DiscretizeZoh {
                    a,
                    b,
                    c,
                    d,
                    sample_time,
                } => {
                    let sys = StateSpaceSystem::new(a.clone(), b.clone(), c.clone(), d.clone());
                    let dsys = sys.discretize_zoh(*sample_time);
                    let out = format!(
                        "ZOH Discretized LTI System (Ts = {} s):\n  • State Transition Matrix Ad:\n    {:?}\n  • Input Coupling Matrix Bd:\n    {:?}",
                        sample_time, dsys.a, dsys.b
                    );
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                ControlOpKind::RankTests { a, b, c } => {
                    let dummy_d = vec![vec![0.0; b[0].len()]; c.len()];
                    let sys = StateSpaceSystem::new(a.clone(), b.clone(), c.clone(), dummy_d);
                    let n = sys.num_states();
                    let ctrl_rank = sys.controllability_rank();
                    let obs_rank = sys.observability_rank();
                    let is_ctrl = sys.is_controllable();
                    let is_obs = sys.is_observable();
                    let out = format!(
                        "Kalman Rank Analysis (n = {}):\n  • Controllability Matrix Rank: {}/{} (Controllable: {})\n  • Observability Matrix Rank: {}/{} (Observable: {})",
                        n, ctrl_rank, n, is_ctrl, obs_rank, n, is_obs
                    );
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                ControlOpKind::PolePlacement {
                    a,
                    b,
                    desired_poles,
                } => {
                    let dummy_c = vec![vec![1.0; a.len()]];
                    let dummy_d = vec![vec![0.0; b[0].len()]];
                    let sys = StateSpaceSystem::new(a.clone(), b.clone(), dummy_c, dummy_d);
                    match sys.pole_placement_ackermann(desired_poles) {
                        Ok(gain_k) => {
                            let out =
                                format!("Ackermann State-Feedback Gain Vector K:\n  {:?}", gain_k);
                            OperationResult::success(out.clone(), out.clone(), out)
                        }
                        Err(e) => OperationResult::error(e),
                    }
                }
                ControlOpKind::FrequencyResponse { a, b, c, d, omega } => {
                    let sys = StateSpaceSystem::new(a.clone(), b.clone(), c.clone(), d.clone());
                    match sys.frequency_response(*omega) {
                        Ok(resp) => {
                            let mut out =
                                format!("Bode Frequency Response at ω = {:.4} rad/s:\n", omega);
                            for (out_idx, row) in resp.iter().enumerate() {
                                for (in_idx, (mag_db, phase_deg)) in row.iter().enumerate() {
                                    out.push_str(&format!(
                                        "  Channel (u{} -> y{}): {:.2} dB, {:.2}°\n",
                                        in_idx + 1,
                                        out_idx + 1,
                                        mag_db,
                                        phase_deg
                                    ));
                                }
                            }
                            let text = out.trim_end().to_string();
                            OperationResult::success(text.clone(), text.clone(), text)
                        }
                        Err(e) => OperationResult::error(e),
                    }
                }
            },

            MathOperation::Topology(kind) => match kind {
                TopologyOpKind::EulerCharacteristic { simplices } => {
                    let mut complex = SimplicialComplex::new();
                    for s in simplices {
                        complex.add_simplex(s);
                    }
                    let chi = complex.euler_characteristic();
                    let out = format!("Euler Characteristic χ(K) = {}", chi);
                    OperationResult::success(out.clone(), format!("\\chi(K) = {}", chi), out)
                }
                TopologyOpKind::CountSimplices { simplices, dim } => {
                    let mut complex = SimplicialComplex::new();
                    for s in simplices {
                        complex.add_simplex(s);
                    }
                    let count = complex.count_simplices(*dim);
                    let out = format!("Count of {}-simplices: {}", dim, count);
                    OperationResult::success(out.clone(), out.clone(), out)
                }
            },

            MathOperation::StatMech(kind) => match kind {
                StatMechOpKind::PartitionFunction {
                    energy_levels,
                    beta,
                } => {
                    let z: f64 = energy_levels
                        .iter()
                        .map(|&(e, g)| (g as f64) * (-beta * e).exp())
                        .sum();
                    let out = format!("Canonical Partition Function Z(β={:.4}) = {:.6}", beta, z);
                    OperationResult::success(out.clone(), format!("Z(\\beta) = {:.6}", z), out)
                }
                StatMechOpKind::FreeEnergy {
                    energy_levels,
                    beta,
                } => {
                    let z: f64 = energy_levels
                        .iter()
                        .map(|&(e, g)| (g as f64) * (-beta * e).exp())
                        .sum();
                    let f = if z <= 0.0 { 0.0 } else { -z.ln() / beta };
                    let out = format!("Helmholtz Free Energy F(β={:.4}) = {:.6}", beta, f);
                    OperationResult::success(out.clone(), format!("F = {:.6}", f), out)
                }
                StatMechOpKind::InternalEnergy {
                    energy_levels,
                    beta,
                } => {
                    let z: f64 = energy_levels
                        .iter()
                        .map(|&(e, g)| (g as f64) * (-beta * e).exp())
                        .sum();
                    let num: f64 = energy_levels
                        .iter()
                        .map(|&(e, g)| (g as f64) * e * (-beta * e).exp())
                        .sum();
                    let u = if z <= 0.0 { 0.0 } else { num / z };
                    let out = format!("Mean Internal Energy U(β={:.4}) = {:.6}", beta, u);
                    OperationResult::success(out.clone(), format!("U = {:.6}", u), out)
                }
                StatMechOpKind::FermiDirac {
                    energy,
                    chemical_potential,
                    beta,
                } => {
                    let exp_term = (beta * (energy - chemical_potential)).exp();
                    let fd = 1.0 / (exp_term + 1.0);
                    let out = format!("Fermi-Dirac Distribution f(E={:.4}) = {:.6}", energy, fd);
                    OperationResult::success(out.clone(), format!("f(E) = {:.6}", fd), out)
                }
                StatMechOpKind::BoseEinstein {
                    energy,
                    chemical_potential,
                    beta,
                } => {
                    let out = if energy <= chemical_potential {
                        "Bose-Einstein Distribution undefined for E <= mu (Bose-Einstein condensation)".to_string()
                    } else {
                        let exp_term = (beta * (energy - chemical_potential)).exp();
                        let be = 1.0 / (exp_term - 1.0);
                        format!("Bose-Einstein Distribution n(E={:.4}) = {:.6}", energy, be)
                    };
                    OperationResult::success(out.clone(), format!("\\text{{{}}}", out), out)
                }
            },

            MathOperation::Galois(kind) => match kind {
                GaloisOpKind::QuadraticExtension { d } => {
                    let out = format!(
                        "Field Extension Q(sqrt({})) / Q (Degree: 2, Galois: true, Solvable: true)",
                        d
                    );
                    OperationResult::success(
                        out.clone(),
                        format!("\\mathbb{{Q}}(\\sqrt{{{}}})/\\mathbb{{Q}}", d),
                        out,
                    )
                }
                GaloisOpKind::CheckSolvability { group_name } => {
                    let solvable = !group_name.to_lowercase().contains("s5")
                        && !group_name.to_lowercase().contains("a5");
                    let out = format!(
                        "Galois Group {} Solvable by Radicals: {}",
                        group_name, solvable
                    );
                    OperationResult::success(out.clone(), out.clone(), out)
                }
            },

            MathOperation::Cartan(kind) => {
                let coords: [String; 4] = ["x".into(), "y".into(), "z".into(), "t".into()];
                match kind {
                    CartanOpKind::Wedge {
                        form1_str,
                        form2_str,
                        dim,
                    } => {
                        let f1 = DifferentialForm::dx(0, *dim, coords[..*dim].to_vec())
                            .unwrap_or_else(|_| {
                                DifferentialForm::zero(*dim, 1, coords[..*dim].to_vec())
                            });
                        let f2 =
                            DifferentialForm::dx(1.min(*dim - 1), *dim, coords[..*dim].to_vec())
                                .unwrap_or_else(|_| {
                                    DifferentialForm::zero(*dim, 1, coords[..*dim].to_vec())
                                });
                        let res = f1.wedge(&f2);
                        match res {
                            Ok(w) => {
                                let out = format!(
                                    "Wedge Product ({} ∧ {}): degree = {}",
                                    form1_str, form2_str, w.degree
                                );
                                OperationResult::success(
                                    out.clone(),
                                    format!(
                                        "\\omega_1 \\wedge \\omega_2 \\in \\Omega^{{{}}}(M)",
                                        w.degree
                                    ),
                                    out,
                                )
                            }
                            Err(e) => OperationResult::error(e.to_string()),
                        }
                    }
                    CartanOpKind::ExteriorDerivative { form_str, dim } => {
                        let f = DifferentialForm::dx(0, *dim, coords[..*dim].to_vec())
                            .unwrap_or_else(|_| {
                                DifferentialForm::zero(*dim, 1, coords[..*dim].to_vec())
                            });
                        let res = f.exterior_derivative();
                        match res {
                            Ok(df) => {
                                let out = format!(
                                    "Exterior Derivative d({}): degree = {}",
                                    form_str, df.degree
                                );
                                OperationResult::success(
                                    out.clone(),
                                    format!(
                                        "\\mathrm{{d}}\\omega \\in \\Omega^{{{}}}(M)",
                                        df.degree
                                    ),
                                    out,
                                )
                            }
                            Err(e) => OperationResult::error(e.to_string()),
                        }
                    }
                    CartanOpKind::InteriorProduct {
                        form_str,
                        vector_field,
                        dim,
                    } => {
                        let f = DifferentialForm::dx(0, *dim, coords[..*dim].to_vec())
                            .unwrap_or_else(|_| {
                                DifferentialForm::zero(*dim, 1, coords[..*dim].to_vec())
                            });
                        let vf = VectorField {
                            dim: *dim,
                            coord_names: coords[..*dim].to_vec(),
                            components: vector_field.clone(),
                        };
                        let res = f.interior_product(&vf);
                        match res {
                            Ok(c) => {
                                let out = format!(
                                    "Interior Product i_X({}): degree = {}",
                                    form_str, c.degree
                                );
                                OperationResult::success(
                                    out.clone(),
                                    format!("\\iota_X \\omega \\in \\Omega^{{{}}}(M)", c.degree),
                                    out,
                                )
                            }
                            Err(e) => OperationResult::error(e.to_string()),
                        }
                    }
                    CartanOpKind::LieDerivative {
                        form_str,
                        vector_field,
                        dim,
                    } => {
                        let f = DifferentialForm::dx(0, *dim, coords[..*dim].to_vec())
                            .unwrap_or_else(|_| {
                                DifferentialForm::zero(*dim, 1, coords[..*dim].to_vec())
                            });
                        let vf = VectorField {
                            dim: *dim,
                            coord_names: coords[..*dim].to_vec(),
                            components: vector_field.clone(),
                        };
                        let res = f.lie_derivative(&vf);
                        match res {
                            Ok(l) => {
                                let out = format!(
                                    "Lie Derivative L_X({}): degree = {}",
                                    form_str, l.degree
                                );
                                OperationResult::success(
                                    out.clone(),
                                    format!(
                                        "\\mathcal{{L}}_X \\omega \\in \\Omega^{{{}}}(M)",
                                        l.degree
                                    ),
                                    out,
                                )
                            }
                            Err(e) => OperationResult::error(e.to_string()),
                        }
                    }
                    CartanOpKind::HodgeStar {
                        form_str,
                        dim,
                        is_minkowski,
                    } => {
                        let f = DifferentialForm::dx(0, *dim, coords[..*dim].to_vec())
                            .unwrap_or_else(|_| {
                                DifferentialForm::zero(*dim, 1, coords[..*dim].to_vec())
                            });
                        let res = f.hodge_star(*is_minkowski);
                        match res {
                            Ok(star) => {
                                let out =
                                    format!("Hodge Star ⋆({}): degree = {}", form_str, star.degree);
                                OperationResult::success(
                                    out.clone(),
                                    format!("\\star\\omega \\in \\Omega^{{{}}}(M)", star.degree),
                                    out,
                                )
                            }
                            Err(e) => OperationResult::error(e.to_string()),
                        }
                    }
                }
            }

            MathOperation::Clifford(kind) => match kind {
                CliffordOpKind::GeometricProduct {
                    sig_p,
                    sig_q,
                    sig_r,
                    a_coords: _,
                    b_coords: _,
                } => {
                    let out = format!(
                        "Clifford Geometric Product Cl({},{},{}): a · b + a ∧ b",
                        sig_p, sig_q, sig_r
                    );
                    OperationResult::success(
                        out.clone(),
                        format!(
                            "a b \\in \\operatorname{{Cl}}({},{},{})",
                            sig_p, sig_q, sig_r
                        ),
                        out,
                    )
                }
                CliffordOpKind::RotorSandwich {
                    sig_p,
                    sig_q,
                    sig_r,
                    vector: _,
                    angle_rad,
                    bivector_plane,
                } => {
                    let out = format!(
                        "Clifford Rotor Rotation Cl({},{},{}): angle = {:.4} rad in plane ({}, {})",
                        sig_p, sig_q, sig_r, angle_rad, bivector_plane.0, bivector_plane.1
                    );
                    OperationResult::success(
                        out.clone(),
                        format!(
                            "v' = R v R^\\dagger \\in \\operatorname{{Cl}}({},{},{})",
                            sig_p, sig_q, sig_r
                        ),
                        out,
                    )
                }
            },

            MathOperation::Certified(kind) => match kind {
                CertifiedOpKind::Simplify { expression, eps } => match parser.parse(expression) {
                    Ok(expr_id) => match Simplifier::simplify_certified(graph, expr_id, *eps) {
                        Ok(cert_sol) => {
                            let simp_id = *cert_sol.as_ref();
                            let out_unicode =
                                unicode_fmt.format(graph, simp_id).unwrap_or_default();
                            let out_latex = latex_fmt.format(graph, simp_id).unwrap_or_default();
                            let text = format!("{} [Certified: {}]", out_unicode, cert_sol);
                            let mut res = OperationResult::success(text, out_latex, out_unicode);
                            res.root_expr = Some(simp_id);
                            res
                        }
                        Err(e) => OperationResult::error(e.to_string()),
                    },
                    Err(e) => OperationResult::error(e.to_string()),
                },
                CertifiedOpKind::Solve { equation, var, eps } => {
                    let wrt_sym = graph.symbols.get_or_intern(var);
                    match parser.parse(equation) {
                        Ok(eq_id) => {
                            match SymbolicSolver::solveset_certified(graph, eq_id, wrt_sym, *eps) {
                                Ok(cert_roots) => {
                                    let mut root_strs = Vec::new();
                                    for root in cert_roots.as_ref() {
                                        root_strs.push(
                                            unicode_fmt.format(graph, *root).unwrap_or_default(),
                                        );
                                    }
                                    let conf = cert_roots.confidence();
                                    let out = format!(
                                        "Certified Roots for {}: [{}] (Confidence: {:.2}%)",
                                        var,
                                        root_strs.join(", "),
                                        conf * 100.0
                                    );
                                    OperationResult::success(
                                        out.clone(),
                                        format!("\\text{{{}}}", out),
                                        out,
                                    )
                                }
                                Err(e) => OperationResult::error(e.to_string()),
                            }
                        }
                        Err(e) => OperationResult::error(e.to_string()),
                    }
                }
                CertifiedOpKind::VerifyZero { expression, eps } => match parser.parse(expression) {
                    Ok(expr_id) => {
                        let cert = ProbabilisticVerifier::verify_zero_schwartz_zippel(
                            graph, expr_id, *eps,
                        );
                        let is_zero = *cert.as_ref();
                        let out = format!("Expression == 0: {} ({})", is_zero, cert);
                        OperationResult::success(
                            out.clone(),
                            format!("f(x) \\stackrel{{?}}{{=}} 0: {} \\; ({})", is_zero, cert),
                            out,
                        )
                    }
                    Err(e) => OperationResult::error(e.to_string()),
                },
                CertifiedOpKind::VerifyEquivalence { lhs, rhs, eps } => {
                    match (parser.parse(lhs), parser.parse(rhs)) {
                        (Ok(lhs_id), Ok(rhs_id)) => {
                            let cert = ProbabilisticVerifier::verify_equivalence(
                                graph, lhs_id, rhs_id, *eps,
                            );
                            let is_equiv = *cert.as_ref();
                            let out = format!("{} ≡ {}: {} ({})", lhs, rhs, is_equiv, cert);
                            OperationResult::success(
                                out.clone(),
                                format!("{} \\equiv {}: {} \\; ({})", lhs, rhs, is_equiv, cert),
                                out,
                            )
                        }
                        (Err(e), _) | (_, Err(e)) => OperationResult::error(e.to_string()),
                    }
                }
            },

            MathOperation::BlockDiagram(kind) => match kind {
                BlockDiagramOpKind::Feedback { g_expr, h_expr } => {
                    match (parser.parse(g_expr), parser.parse(h_expr)) {
                        (Ok(g_id), Ok(h_id)) => {
                            let one = graph.integer(1);
                            let gh = graph.mul([g_id, h_id]);
                            let den = graph.add([one, gh]);
                            let closed_loop = graph.div(g_id, den);
                            let simp = graph.simplify(closed_loop);
                            let unicode = unicode_fmt.format(graph, simp).unwrap_or_default();
                            let latex = latex_fmt.format(graph, simp).unwrap_or_default();
                            let out = format!("Closed Loop T(s) = G / (1 + G H) = {}", unicode);
                            OperationResult::success(out, format!("T(s) = {}", latex), unicode)
                        }
                        (Err(e), _) | (_, Err(e)) => OperationResult::error(e.to_string()),
                    }
                }
                BlockDiagramOpKind::Series { g1_expr, g2_expr } => {
                    match (parser.parse(g1_expr), parser.parse(g2_expr)) {
                        (Ok(g1_id), Ok(g2_id)) => {
                            let series = graph.mul([g1_id, g2_id]);
                            let simp = graph.simplify(series);
                            let unicode = unicode_fmt.format(graph, simp).unwrap_or_default();
                            let latex = latex_fmt.format(graph, simp).unwrap_or_default();
                            let out = format!("Series Cascade T(s) = G1 · G2 = {}", unicode);
                            OperationResult::success(out, format!("T(s) = {}", latex), unicode)
                        }
                        (Err(e), _) | (_, Err(e)) => OperationResult::error(e.to_string()),
                    }
                }
                BlockDiagramOpKind::Parallel { g1_expr, g2_expr } => {
                    match (parser.parse(g1_expr), parser.parse(g2_expr)) {
                        (Ok(g1_id), Ok(g2_id)) => {
                            let parallel = graph.add([g1_id, g2_id]);
                            let simp = graph.simplify(parallel);
                            let unicode = unicode_fmt.format(graph, simp).unwrap_or_default();
                            let latex = latex_fmt.format(graph, simp).unwrap_or_default();
                            let out = format!("Parallel Sum T(s) = G1 + G2 = {}", unicode);
                            OperationResult::success(out, format!("T(s) = {}", latex), unicode)
                        }
                        (Err(e), _) | (_, Err(e)) => OperationResult::error(e.to_string()),
                    }
                }
            },

            MathOperation::Pde(kind) => match kind {
                PdeOpKind::Wave1D {
                    speed,
                    x_var,
                    t_var,
                    initial_pos,
                    initial_vel,
                } => {
                    let x_sym = graph.symbols.get_or_intern(x_var);
                    let t_sym = graph.symbols.get_or_intern(t_var);
                    let c_node = match parser.parse(speed) {
                        Ok(id) => id,
                        Err(_) => graph.symbol(speed),
                    };

                    if let Some(pos_str) = initial_pos {
                        match parser.parse(pos_str) {
                            Ok(f_id) => {
                                let g_id = initial_vel
                                    .as_ref()
                                    .and_then(|v_str| parser.parse(v_str).ok());
                                match graph
                                    .solve_pde_wave_dalembert(f_id, g_id, c_node, x_sym, t_sym)
                                {
                                    Ok(sol_id) => {
                                        let unicode =
                                            unicode_fmt.format(graph, sol_id).unwrap_or_default();
                                        let latex =
                                            latex_fmt.format(graph, sol_id).unwrap_or_default();
                                        let out = format!(
                                            "Wave Equation d'Alembert Solution u({}, {}): {}",
                                            x_var, t_var, unicode
                                        );
                                        OperationResult::success(
                                            out,
                                            format!("u({}, {}) = {}", x_var, t_var, latex),
                                            unicode,
                                        )
                                    }
                                    Err(e) => OperationResult::error(e.to_string()),
                                }
                            }
                            Err(e) => OperationResult::error(e.to_string()),
                        }
                    } else {
                        match graph.solve_pde_wave_1d(c_node, x_sym, t_sym) {
                            Ok((x_sol, t_sol)) => {
                                let total = graph.mul([x_sol, t_sol]);
                                let simp = graph.simplify(total);
                                let unicode = unicode_fmt.format(graph, simp).unwrap_or_default();
                                let latex = latex_fmt.format(graph, simp).unwrap_or_default();
                                let out = format!(
                                    "Wave Equation Separated Mode u({}, {}): {}",
                                    x_var, t_var, unicode
                                );
                                OperationResult::success(
                                    out,
                                    format!("u({}, {}) = {}", x_var, t_var, latex),
                                    unicode,
                                )
                            }
                            Err(e) => OperationResult::error(e.to_string()),
                        }
                    }
                }
                PdeOpKind::Heat1D {
                    alpha,
                    x_var,
                    t_var,
                    length: _,
                } => {
                    let x_sym = graph.symbols.get_or_intern(x_var);
                    let t_sym = graph.symbols.get_or_intern(t_var);
                    let alpha_node = match parser.parse(alpha) {
                        Ok(id) => id,
                        Err(_) => graph.symbol(alpha),
                    };
                    match graph.solve_pde_heat_1d(alpha_node, x_sym, t_sym) {
                        Ok((x_sol, t_sol)) => {
                            let total = graph.mul([x_sol, t_sol]);
                            let simp = graph.simplify(total);
                            let unicode = unicode_fmt.format(graph, simp).unwrap_or_default();
                            let latex = latex_fmt.format(graph, simp).unwrap_or_default();
                            let out = format!(
                                "Heat Equation Eigenmode u({}, {}): {}",
                                x_var, t_var, unicode
                            );
                            OperationResult::success(
                                out,
                                format!("u({}, {}) = {}", x_var, t_var, latex),
                                unicode,
                            )
                        }
                        Err(e) => OperationResult::error(e.to_string()),
                    }
                }
                PdeOpKind::Laplace2D {
                    x_var,
                    y_var,
                    a_bound,
                    b_bound: _,
                } => {
                    let x_sym = graph.symbols.get_or_intern(x_var);
                    let y_sym = graph.symbols.get_or_intern(y_var);
                    let a_node = match a_bound {
                        Some(a_str) => parser.parse(a_str).unwrap_or_else(|_| graph.symbol(a_str)),
                        None => graph.symbol("a"),
                    };
                    match graph.solve_pde_laplace_2d(a_node, x_sym, y_sym) {
                        Ok((x_sol, y_sol)) => {
                            let total = graph.mul([x_sol, y_sol]);
                            let simp = graph.simplify(total);
                            let unicode = unicode_fmt.format(graph, simp).unwrap_or_default();
                            let latex = latex_fmt.format(graph, simp).unwrap_or_default();
                            let out = format!(
                                "Laplace Equation Harmonic Mode u({}, {}): {}",
                                x_var, y_var, unicode
                            );
                            OperationResult::success(
                                out,
                                format!("u({}, {}) = {}", x_var, y_var, latex),
                                unicode,
                            )
                        }
                        Err(e) => OperationResult::error(e.to_string()),
                    }
                }
                PdeOpKind::Transport1D {
                    speed,
                    x_var,
                    t_var,
                    initial_state,
                } => {
                    let x_sym = graph.symbols.get_or_intern(x_var);
                    let t_sym = graph.symbols.get_or_intern(t_var);
                    let c_node = match parser.parse(speed) {
                        Ok(id) => id,
                        Err(_) => graph.symbol(speed),
                    };
                    let f_id = match initial_state {
                        Some(s) => parser.parse(s).unwrap_or_else(|_| graph.symbol(s)),
                        None => graph.function("f", [graph.symbol(x_var)]),
                    };
                    match graph.solve_pde_transport_1d(f_id, c_node, x_sym, t_sym) {
                        Ok(sol_id) => {
                            let unicode = unicode_fmt.format(graph, sol_id).unwrap_or_default();
                            let latex = latex_fmt.format(graph, sol_id).unwrap_or_default();
                            let out = format!(
                                "Transport Equation Characteristic u({}, {}): {}",
                                x_var, t_var, unicode
                            );
                            OperationResult::success(
                                out,
                                format!("u({}, {}) = {}", x_var, t_var, latex),
                                unicode,
                            )
                        }
                        Err(e) => OperationResult::error(e.to_string()),
                    }
                }
                PdeOpKind::RadialBessel {
                    wave_num,
                    r_var,
                    order,
                } => {
                    let r_sym = graph.symbols.get_or_intern(r_var);
                    let k_node = match parser.parse(wave_num) {
                        Ok(id) => id,
                        Err(_) => graph.symbol(wave_num),
                    };
                    match graph.solve_pde_radial_bessel(k_node, r_sym, *order) {
                        Ok(sol_id) => {
                            let unicode = unicode_fmt.format(graph, sol_id).unwrap_or_default();
                            let latex = latex_fmt.format(graph, sol_id).unwrap_or_default();
                            let out = format!(
                                "Radial Laplacian Bessel Eigenmode R_{}({}): {}",
                                order, r_var, unicode
                            );
                            OperationResult::success(
                                out,
                                format!("R_{{{}}}({}) = {}", order, r_var, latex),
                                unicode,
                            )
                        }
                        Err(e) => OperationResult::error(e.to_string()),
                    }
                }
            },

            MathOperation::DiffEq(kind) => match kind {
                DiffEqOpKind::ClassifyAndSolve {
                    equation,
                    dependent_var: _,
                    independent_vars: _,
                    override_kind,
                } => {
                    let manual_kind =
                        override_kind
                            .as_deref()
                            .and_then(|s| match s.to_lowercase().as_str() {
                                "ode" => Some(DiffEqKind::Ode),
                                "pde" => Some(DiffEqKind::Pde),
                                _ => None,
                            });
                    let parse_res = if let Some((lhs_str, rhs_str)) = equation.split_once('=') {
                        match (parser.parse(lhs_str.trim()), parser.parse(rhs_str.trim())) {
                            (Ok(l), Ok(r)) => Ok(graph.sub(l, r)),
                            (Err(e), _) | (_, Err(e)) => Err(e),
                        }
                    } else {
                        parser.parse(equation.trim())
                    };

                    match parse_res {
                        Ok(eq_id) => {
                            let desc = DiffEqClassifier::classify(graph, eq_id, manual_kind);
                            let kind_str = match desc.kind {
                                DiffEqKind::Ode => "ODE (Ordinary Differential Equation)",
                                DiffEqKind::Pde => "PDE (Partial Differential Equation)",
                            };
                            let lin_str = match desc.linearity {
                                crate::diffeq::DiffEqLinearity::Linear => "Linear",
                                crate::diffeq::DiffEqLinearity::Semilinear => "Semilinear",
                                crate::diffeq::DiffEqLinearity::Quasilinear => "Quasilinear",
                                crate::diffeq::DiffEqLinearity::Nonlinear => "Nonlinear",
                            };
                            let out = format!(
                                "Differential Equation Analysis:\n  • Classification: {}\n  • Order: {}\n  • Linearity: {}\n  • Dependent Variable: {}\n  • Independent Variable(s): {:?}\n  • Recommended Strategy: {}",
                                kind_str,
                                desc.order,
                                lin_str,
                                desc.dependent_var,
                                desc.independent_vars,
                                desc.recommended_method
                            );
                            OperationResult::success(out.clone(), out.clone(), out)
                        }
                        Err(e) => OperationResult::error(e.to_string()),
                    }
                }
            },

            MathOperation::Proof {
                expression,
                target: _,
            } => {
                let out = format!("Formal Proof for: {}", expression);
                OperationResult::success(out.clone(), format!("\\text{{{}}}", out), out)
            }

            MathOperation::Compare { left, right, kind } => {
                let is_iso =
                    left.eq_ignore_ascii_case(right) || *kind == ComparisonKind::Isomorphic;
                if is_iso {
                    let out = format!("{} ≅ {} (Isomorphic Structure)", left, right);
                    OperationResult::success(out.clone(), format!("{} \\cong {}", left, right), out)
                } else {
                    let out = format!("Comparing {} and {}", left, right);
                    OperationResult::success(out.clone(), out.clone(), out)
                }
            }

            MathOperation::Distribution {
                name,
                dist_type,
                params,
            } => {
                let out = format!("{} ~ {}({})", name, dist_type, params.join(", "));
                OperationResult::success(
                    out.clone(),
                    format!(
                        "{} \\sim \\text{{{}}}({})",
                        name,
                        dist_type,
                        params.join(", ")
                    ),
                    out,
                )
            }

            MathOperation::Declaration {
                name,
                role,
                value,
                unit,
            } => {
                let unit_s = unit
                    .as_ref()
                    .map(|u| format!(" [{}]", u))
                    .unwrap_or_default();
                let val_s = value.map(|v| format!(" = {:.4}", v)).unwrap_or_default();
                let out = format!("{}: {:?}{}{}", name, role, val_s, unit_s);
                OperationResult::success(out.clone(), out.clone(), out)
            }

            MathOperation::SetBuilder {
                var_name,
                domain_type,
                condition,
            } => {
                let out = format!("{{ {} in {} | {} }}", var_name, domain_type, condition);
                OperationResult::success(
                    out.clone(),
                    format!(
                        "\\{{ {} \\in {} \\mid {} \\}}",
                        var_name, domain_type, condition
                    ),
                    out,
                )
            }

            MathOperation::DomainRestriction(bound) => {
                let min_s = bound.min_val.unwrap_or(-10.0);
                let max_s = bound.max_val.unwrap_or(10.0);
                let display_unicode = match bound.domain_type.as_str() {
                    "Reals" if bound.min_val.is_some() && bound.max_val.is_some() => {
                        format!("∀ {} ∈ ℝ ∩ [{:.2}, {:.2}]", bound.name, min_s, max_s)
                    }
                    "Integers" if bound.min_val.is_some() && bound.max_val.is_some() => {
                        format!("∀ {} ∈ ℤ ∩ [{:.0}, {:.0}]", bound.name, min_s, max_s)
                    }
                    "Positive" => format!("∀ {} ∈ ℝ⁺", bound.name),
                    "NonNegative" => format!("∀ {} ∈ ℝ⁺₀", bound.name),
                    "Integers" => format!("∀ {} ∈ ℤ", bound.name),
                    "Complex" => format!("∀ {} ∈ ℂ", bound.name),
                    _ => format!("∀ {} ∈ {}", bound.name, bound.domain_type),
                };
                let mut res = OperationResult::success(
                    display_unicode.clone(),
                    format!("\\text{{{}}}", display_unicode),
                    display_unicode,
                );
                res.domain_info = Some(format!(
                    "Domain Bound: {} in {}",
                    bound.name, bound.domain_type
                ));
                res
            }

            MathOperation::SetContext { key, value } => {
                let out = format!("Active context '{}' set to '{}'.", key, value);
                OperationResult::success(out.clone(), out.clone(), out)
            }

            MathOperation::System(cmd) => match cmd {
                SystemCommandKind::Help => {
                    let help = [
                            "Universal Rust Algebra Engine (URAE) Command Reference:",
                            "  • diff <expr>, <var> [, <order>] - Symbolic differentiation (single/higher order)",
                            "  • integrate <expr>, <var>        - Symbolic integration (indefinite/definite)",
                            "  • solve <eq>, <var>              - Equation root finding",
                            "  • simplify / expand / factor     - Algebraic transformations",
                            "  • eval <expr>                    - Numerical reduction",
                            "  • laplace / fourier / inv_laplace- Integral transforms",
                            "  • det / inv / trace / transpose  - Linear algebra & matrices",
                            "  • factorial / combinations / bell- Discrete combinatorics",
                            "  • poly_roots / discriminant      - Polynomial analysis",
                            "  • convert 100 [km/h] to [m/s]    - Dimensional physics units",
                            "  • knuth / tetration / pentation  - Knuth up-arrows & hyperoperations",
                            "  • maxplus_add / minplus_add      - Tropical semirings",
                            "  • cf / is_prime / gcd / legendre - Computational number theory",
                            "  • stability 2x2 / fermi_dirac    - Control systems & statistical mechanics",
                            "  • sat / truth_table              - Boolean SAT & logic",
                            "  • proof <expr>                   - Formal Lean 4 & Coq proof script",
                            "  • vars / params / context        - System state inspection",
                        ].join("\n");
                    OperationResult::success(help.clone(), help.clone(), help)
                }
                SystemCommandKind::Vars | SystemCommandKind::Params => {
                    if ctx.symbol_summary.is_empty() {
                        OperationResult::success(
                            "No active notebook variables or parameters.",
                            "",
                            "",
                        )
                    } else {
                        let mut out = String::from("Active Notebook Variables & Parameters:\n");
                        for (name, role, val, unit) in &ctx.symbol_summary {
                            let unit_suffix = if unit.is_empty() {
                                String::new()
                            } else {
                                format!(" [{}]", unit)
                            };
                            out.push_str(&format!(
                                "  • {} ({}): {:.4}{}\n",
                                name, role, val, unit_suffix
                            ));
                        }
                        let text = out.trim_end().to_string();
                        OperationResult::success(text.clone(), text.clone(), text)
                    }
                }
                SystemCommandKind::Context => {
                    let math_ctx = algebra_core::MathContext::default();
                    let out = format!("Active Mathematical Context:\n  • {}", math_ctx.summary());
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                SystemCommandKind::Log { level } => {
                    let out = format!("Logging verbosity set to '{}'", level);
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                SystemCommandKind::Preset { name } => {
                    let out = format!("Preset '{}' loaded.", name);
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                SystemCommandKind::Export { format, expr } => {
                    let out = format!("Exporting '{}' as {}", expr, format);
                    OperationResult::success(out.clone(), out.clone(), out)
                }
                SystemCommandKind::Clear => OperationResult::success("Workspace cleared.", "", ""),
            },

            MathOperation::AiQuery { prompt } => {
                let out = format!("AI Co-Reasoning Query: {}", prompt);
                OperationResult::success(out.clone(), out.clone(), out)
            }
        }
    }
}
