//! # `algebra_engine::workbench`
//!
//! Shared Computational Workbench for Interactive Systems of Equations & Differential Equations.
//!
//! Powers both native desktop GUI (`urae-notebook`) and WebAssembly UI (`urae-wasm`) endpoints with
//! unified setup, parameter alteration, symbolic + numerical solving, visualization geometry extraction
//! (Marching Squares implicit zero-contours, multi-variable ODE trajectories, 2D phase-plane quiver
//! direction fields), and local Jacobian / eigenvalue stability analysis.
//!
//! ## References
//! - Hairer, E., Nørsett, S. P., & Wanner, G. (1993). *Solving Ordinary Differential Equations I*. Springer.
//! - Lorensen, W. E., & Cline, H. E. (1987). Marching cubes / Marching squares iso-contour extraction.
//! - Strogatz, S. H. (2015). *Nonlinear Dynamics and Chaos* (2nd ed.). Westview Press.

use crate::calculus::SymbolicCalculus;
use crate::executor::{ExecutionContext, OperationExecutor};
use crate::matrix::SymbolicMatrix;
use crate::numeric::{EvalContext, NumericalEval};
use crate::ode::{NumericalOdeConfig, NumericalOdeMethod, NumericalOdeSolver};
use crate::simplify::Simplifier;
use algebra_core::format::{Formatter, LatexFormatter, UnicodeFormatter};
use algebra_core::operation::MathOperation;
use algebra_core::parser::{ExprParser, parse_operation};
use algebra_core::vm::{BytecodeProgram, BytecodeVM, VmCompiler};
use algebra_core::{ExprGraph, ExprId, ExprKind, RelOp};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single 2D line segment belonging to an implicit equation contour $f(x, y) = 0$.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ContourSegment {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

/// A 2D vector-field direction sample for phase portraits.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct QuiverArrow {
    pub x: f64,
    pub y: f64,
    pub dx: f64,
    pub dy: f64,
    pub speed: f64,
}

/// Specification of an algebraic / nonlinear equation system to solve, visualize, and analyze.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquationSystemSpec {
    pub name: String,
    pub equations: Vec<String>,
    pub variables: Vec<String>,
    pub parameters: Vec<(String, f64)>,
    pub x_range: [f64; 2],
    pub y_range: [f64; 2],
    #[serde(default = "default_grid_res")]
    pub grid_res: usize,
}

fn default_grid_res() -> usize {
    44
}

impl Default for EquationSystemSpec {
    fn default() -> Self {
        Self {
            name: "Circle-Line Intersection".to_string(),
            equations: vec!["x^2 + y^2 = r^2".to_string(), "y = m * x + c".to_string()],
            variables: vec!["x".to_string(), "y".to_string()],
            parameters: vec![("r".to_string(), 3.0), ("m".to_string(), 1.0), ("c".to_string(), 0.5)],
            x_range: [-5.0, 5.0],
            y_range: [-5.0, 5.0],
            grid_res: 48,
        }
    }
}

/// Result of solving, visualizing contours, and analyzing an [`EquationSystemSpec`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquationSystemAnalysis {
    pub success: bool,
    pub error_msg: Option<String>,
    pub solution_text: String,
    pub solution_latex: String,
    pub jacobian_unicode: String,
    pub jacobian_latex: String,
    pub rank: Option<usize>,
    pub contours: Vec<Vec<ContourSegment>>,
    pub solution_points: Vec<[f64; 2]>,
    pub residuals: Vec<f64>,
}

/// Specification of a coupled 1st-order ODE system $\frac{d\mathbf{y}}{dt} = \mathbf{f}(t, \mathbf{y}; \mathbf{p})$.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifferentialSystemSpec {
    pub name: String,
    pub state_vars: Vec<String>,
    pub rhs_expressions: Vec<String>,
    pub t_var: String,
    pub t_span: [f64; 2],
    pub y0: Vec<f64>,
    pub parameters: Vec<(String, f64)>,
    pub method: String,
    pub phase_axes: [usize; 2],
    pub phase_x_range: [f64; 2],
    pub phase_y_range: [f64; 2],
}

impl Default for DifferentialSystemSpec {
    fn default() -> Self {
        Self {
            name: "Damped Harmonic Oscillator".to_string(),
            state_vars: vec!["x".to_string(), "v".to_string()],
            rhs_expressions: vec![
                "v".to_string(),
                "-2 * zeta * omega * v - omega^2 * x".to_string(),
            ],
            t_var: "t".to_string(),
            t_span: [0.0, 15.0],
            y0: vec![2.0, 0.0],
            parameters: vec![("zeta".to_string(), 0.15), ("omega".to_string(), 2.0)],
            method: "tsit5".to_string(),
            phase_axes: [0, 1],
            phase_x_range: [-3.0, 3.0],
            phase_y_range: [-5.0, 5.0],
        }
    }
}

/// Result of solving, integrating, visualizing, and analyzing a [`DifferentialSystemSpec`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifferentialSystemAnalysis {
    pub success: bool,
    pub error_msg: Option<String>,
    pub symbolic_solution_text: Option<String>,
    pub symbolic_solution_latex: Option<String>,
    pub method_used: String,
    pub steps_accepted: usize,
    pub steps_rejected: usize,
    pub t: Vec<f64>,
    pub trajectories: Vec<Vec<f64>>,
    pub jacobian_unicode: String,
    pub jacobian_latex: String,
    pub eigenvalues_text: String,
    pub eigenvalues_latex: String,
    pub stability_label: String,
    pub quiver_field: Vec<QuiverArrow>,
    pub final_state: Vec<f64>,
    pub min_bounds: Vec<f64>,
    pub max_bounds: Vec<f64>,
}

/// Built-in presets for immediate exploration across both native and web GUI endpoints.
pub struct WorkbenchPresets;

impl WorkbenchPresets {
    pub fn equation_presets() -> Vec<EquationSystemSpec> {
        vec![
            EquationSystemSpec::default(),
            EquationSystemSpec {
                name: "Linear 2x2 Intersection".to_string(),
                equations: vec!["a * x + y = 5".to_string(), "x - y = 1".to_string()],
                variables: vec!["x".to_string(), "y".to_string()],
                parameters: vec![("a".to_string(), 2.0)],
                x_range: [-4.0, 6.0],
                y_range: [-4.0, 6.0],
                grid_res: 36,
            },
            EquationSystemSpec {
                name: "Ellipse & Hyperbola Conics".to_string(),
                equations: vec![
                    "x^2 / a^2 + y^2 / b^2 = 1".to_string(),
                    "x * y = c".to_string(),
                ],
                variables: vec!["x".to_string(), "y".to_string()],
                parameters: vec![("a".to_string(), 3.0), ("b".to_string(), 2.0), ("c".to_string(), 1.5)],
                x_range: [-4.5, 4.5],
                y_range: [-4.5, 4.5],
                grid_res: 52,
            },
            EquationSystemSpec {
                name: "Linear 3x3 Coupled Mesh".to_string(),
                equations: vec![
                    "2*x + y - z = 8".to_string(),
                    "-3*x - y + 2*z = -11".to_string(),
                    "-2*x + y + 2*z = -3".to_string(),
                ],
                variables: vec!["x".to_string(), "y".to_string(), "z".to_string()],
                parameters: Vec::new(),
                x_range: [-5.0, 5.0],
                y_range: [-5.0, 5.0],
                grid_res: 32,
            },
        ]
    }

    pub fn differential_presets() -> Vec<DifferentialSystemSpec> {
        vec![
            DifferentialSystemSpec::default(),
            DifferentialSystemSpec {
                name: "Lotka-Volterra Predator-Prey".to_string(),
                state_vars: vec!["prey".to_string(), "pred".to_string()],
                rhs_expressions: vec![
                    "alpha * prey - beta * prey * pred".to_string(),
                    "delta * prey * pred - gamma * pred".to_string(),
                ],
                t_var: "t".to_string(),
                t_span: [0.0, 20.0],
                y0: vec![10.0, 5.0],
                parameters: vec![
                    ("alpha".to_string(), 1.1),
                    ("beta".to_string(), 0.4),
                    ("delta".to_string(), 0.1),
                    ("gamma".to_string(), 0.4),
                ],
                method: "dp5".to_string(),
                phase_axes: [0, 1],
                phase_x_range: [0.2, 20.0],
                phase_y_range: [0.2, 12.0],
            },
            DifferentialSystemSpec {
                name: "Van der Pol Limit Cycle".to_string(),
                state_vars: vec!["x".to_string(), "y".to_string()],
                rhs_expressions: vec![
                    "y".to_string(),
                    "mu * (1 - x^2) * y - x".to_string(),
                ],
                t_var: "t".to_string(),
                t_span: [0.0, 25.0],
                y0: vec![0.5, 0.0],
                parameters: vec![("mu".to_string(), 1.5)],
                method: "radau5".to_string(),
                phase_axes: [0, 1],
                phase_x_range: [-3.5, 3.5],
                phase_y_range: [-4.0, 4.0],
            },
            DifferentialSystemSpec {
                name: "Lorenz Attractor (3D)".to_string(),
                state_vars: vec!["x".to_string(), "y".to_string(), "z".to_string()],
                rhs_expressions: vec![
                    "sigma * (y - x)".to_string(),
                    "x * (rho - z) - y".to_string(),
                    "x * y - beta * z".to_string(),
                ],
                t_var: "t".to_string(),
                t_span: [0.0, 20.0],
                y0: vec![1.0, 1.0, 1.0],
                parameters: vec![
                    ("sigma".to_string(), 10.0),
                    ("rho".to_string(), 28.0),
                    ("beta".to_string(), 2.6667),
                ],
                method: "tsit5".to_string(),
                phase_axes: [0, 2],
                phase_x_range: [-25.0, 25.0],
                phase_y_range: [0.0, 50.0],
            },
        ]
    }
}

/// Core execution and analysis methods powering the Systems & Differential Workbench.
pub struct WorkbenchEngine;

impl WorkbenchEngine {
    /// Solve, extract Jacobian, sample 2D implicit contours, and compute residuals for an equation system.
    pub fn analyze_equation_system(
        graph: &ExprGraph,
        spec: &EquationSystemSpec,
    ) -> EquationSystemAnalysis {
        if spec.equations.is_empty() || spec.variables.is_empty() {
            return EquationSystemAnalysis {
                success: false,
                error_msg: Some("Specify at least one equation and one variable.".to_string()),
                solution_text: String::new(),
                solution_latex: String::new(),
                jacobian_unicode: String::new(),
                jacobian_latex: String::new(),
                rank: None,
                contours: Vec::new(),
                solution_points: Vec::new(),
                residuals: Vec::new(),
            };
        }

        let mut bindings = HashMap::new();
        for (p, v) in &spec.parameters {
            bindings.insert(p.clone(), *v);
        }
        let ctx = ExecutionContext::with_bindings(bindings.clone());

        let op = MathOperation::SolveSystem {
            equations: spec.equations.clone(),
            variables: spec.variables.clone(),
        };
        let solve_res = OperationExecutor::execute(graph, &op, &ctx);

        // Parse residual expressions f_i(vars, params) = 0
        let parser = ExprParser::new(graph);
        let mut residuals_expr = Vec::with_capacity(spec.equations.len());
        for eq_str in &spec.equations {
            match parse_residual_expr(&parser, graph, eq_str) {
                Ok(id) => residuals_expr.push(id),
                Err(e) => {
                    return EquationSystemAnalysis {
                        success: false,
                        error_msg: Some(format!("Parse error in `{eq_str}`: {e}")),
                        solution_text: solve_res.output_text,
                        solution_latex: solve_res.output_latex,
                        jacobian_unicode: String::new(),
                        jacobian_latex: String::new(),
                        rank: None,
                        contours: Vec::new(),
                        solution_points: Vec::new(),
                        residuals: Vec::new(),
                    };
                }
            }
        }

        // Symbolic Jacobian J_ij = d f_i / d x_j
        let (jacobian_unicode, jacobian_latex) =
            format_symbolic_jacobian(graph, &residuals_expr, &spec.variables);

        let rank = solve_res
            .metadata
            .get("rank")
            .and_then(|s| s.parse::<usize>().ok());

        // Extract 2D implicit zero-contours via Marching Squares when plotting (x, y)
        let mut contours = Vec::new();
        if spec.variables.len() == 2 {
            let mut param_order: Vec<String> = Vec::with_capacity(1 + spec.parameters.len());
            param_order.push(spec.variables[1].clone());
            for (p, _) in &spec.parameters {
                param_order.push(p.clone());
            }
            let p_vals: Vec<f64> = spec.parameters.iter().map(|(_, v)| *v).collect();
            let vm = BytecodeVM::new();
            let res_n = spec.grid_res.clamp(16, 80);

            for &r_id in &residuals_expr {
                let mut segs = Vec::new();
                if let Ok(prog) = VmCompiler::compile(graph, r_id, &spec.variables[0], &param_order) {
                    segs = marching_squares_contours(
                        &vm,
                        &prog,
                        &p_vals,
                        spec.x_range,
                        spec.y_range,
                        res_n,
                    );
                }
                contours.push(segs);
            }
        }

        // Parse 2D solution marker coordinates and verify residuals
        let mut solution_points = Vec::new();
        let mut residuals = Vec::new();
        if spec.variables.len() == 2 && !solve_res.is_error {
            let pts = extract_2d_solution_points(
                graph,
                &solve_res.output_text,
                &spec.variables[0],
                &spec.variables[1],
                &bindings,
            );
            for pt in pts {
                let mut pt_bindings = bindings.clone();
                pt_bindings.insert(spec.variables[0].clone(), pt[0]);
                pt_bindings.insert(spec.variables[1].clone(), pt[1]);
                let eval_ctx = EvalContext::with_bindings(20, pt_bindings);
                let max_res = residuals_expr
                    .iter()
                    .filter_map(|&id| graph.evalf(id, &eval_ctx).ok())
                    .map(|v| {
                        let (re, im) = v.as_complex();
                        re.hypot(im)
                    })
                    .fold(0.0_f64, f64::max);
                solution_points.push(pt);
                residuals.push(max_res);
            }
        }

        EquationSystemAnalysis {
            success: !solve_res.is_error,
            error_msg: solve_res.error_msg,
            solution_text: solve_res.output_text,
            solution_latex: solve_res.output_latex,
            jacobian_unicode,
            jacobian_latex,
            rank,
            contours,
            solution_points,
            residuals,
        }
    }

    /// Solve, numerically integrate, compute linearized Jacobian eigenvalues & phase quiver field
    /// for a coupled 1st-order ODE system.
    pub fn analyze_differential_system(
        graph: &ExprGraph,
        spec: &DifferentialSystemSpec,
    ) -> DifferentialSystemAnalysis {
        let dim = spec.state_vars.len();
        if dim == 0 || dim != spec.rhs_expressions.len() || dim != spec.y0.len() {
            return empty_de_error(
                dim,
                "State variables, RHS expressions, and initial conditions must have equal non-zero length.".into(),
            );
        }
        if !spec.t_span[0].is_finite()
            || !spec.t_span[1].is_finite()
            || (spec.t_span[1] - spec.t_span[0]).abs() < 1e-12
        {
            return empty_de_error(dim, "Invalid integration time span [t0, t1].".into());
        }

        let parser = ExprParser::new(graph);
        let mut rhs_ids = Vec::with_capacity(dim);
        for (idx, expr_str) in spec.rhs_expressions.iter().enumerate() {
            match parser.parse(expr_str) {
                Ok(id) => rhs_ids.push(id),
                Err(e) => {
                    return empty_de_error(
                        dim,
                        format!("Parse error in {}' equation `{expr_str}`: {e}", spec.state_vars[idx]),
                    );
                }
            }
        }

        // Optional closed-form symbolic ODE solution when 1D linear ODE `y' = f(t, y)`
        let (mut sym_text, mut sym_latex) = (None, None);
        if dim == 1 {
            let mut ctx_bindings = HashMap::new();
            for (p, v) in &spec.parameters {
                ctx_bindings.insert(p.clone(), *v);
            }
            let ctx = ExecutionContext::with_bindings(ctx_bindings);
            let ode_op = parse_operation(&format!(
                "solve_ode {}' = {}, {}, {}",
                spec.state_vars[0], spec.rhs_expressions[0], spec.state_vars[0], spec.t_var
            ));
            let res = OperationExecutor::execute(graph, &ode_op, &ctx);
            if !res.is_error {
                sym_text = Some(res.output_text);
                sym_latex = Some(res.output_latex);
            }
        }

        // Compile RHS equations into contiguous BytecodeVM programs:
        // `var_name = t_var`, `param_names = [y_0, ..., y_{d-1}, param_0, ...]`
        let mut param_names: Vec<String> = Vec::with_capacity(dim + spec.parameters.len());
        param_names.extend(spec.state_vars.iter().cloned());
        for (p, _) in &spec.parameters {
            if !spec.state_vars.contains(p) && p != &spec.t_var {
                param_names.push(p.clone());
            }
        }
        let param_vals: Vec<f64> = param_names[dim..]
            .iter()
            .filter_map(|pname| spec.parameters.iter().find(|(k, _)| k == pname).map(|(_, v)| *v))
            .collect();

        let mut programs: Vec<BytecodeProgram> = Vec::with_capacity(dim);
        for (i, &id) in rhs_ids.iter().enumerate() {
            match VmCompiler::compile(graph, id, &spec.t_var, &param_names) {
                Ok(prog) => programs.push(prog),
                Err(e) => {
                    return empty_de_error(
                        dim,
                        format!("Bytecode compilation failed for {}': {e}", spec.state_vars[i]),
                    );
                }
            }
        }

        let selected = match spec.method.trim().to_ascii_lowercase().as_str() {
            "rk4" => NumericalOdeMethod::Rk4,
            "dp5" | "dopri5" | "dormandprince54" => NumericalOdeMethod::DormandPrince54,
            "tsit5" | "tsitouras54" => NumericalOdeMethod::Tsitouras54,
            "radau5" | "radau" => NumericalOdeMethod::RadauIIA5,
            "gl4" | "gauss_legendre4" => NumericalOdeMethod::GaussLegendre4,
            "meta" => NumericalOdeMethod::SelfOptimizingMeta,
            _ => NumericalOdeMethod::Auto,
        };

        let cfg = NumericalOdeConfig {
            method: selected,
            rtol: 1e-6,
            atol: 1e-8,
            max_step: ((spec.t_span[1] - spec.t_span[0]).abs() / 25.0).max(0.02),
            max_steps: 25_000,
            ..Default::default()
        };

        let vm = BytecodeVM::new();
        let traj = match NumericalOdeSolver::solve(
            |t_cur, state| {
                let mut call_params = Vec::with_capacity(dim + param_vals.len());
                call_params.extend_from_slice(state);
                call_params.extend_from_slice(&param_vals);
                programs
                    .iter()
                    .map(|p| {
                        let dy = vm.eval_scalar(p, t_cur, &call_params).unwrap_or(0.0);
                        if dy.is_finite() { dy } else { 0.0 }
                    })
                    .collect()
            },
            (spec.t_span[0], spec.t_span[1]),
            &spec.y0,
            &cfg,
        ) {
            Ok(tr) => tr,
            Err(e) => return empty_de_error(dim, format!("ODE solver error: {e}")),
        };

        let mut trajectories = vec![Vec::with_capacity(traj.t.len()); dim];
        let mut min_bounds = vec![f64::INFINITY; dim];
        let mut max_bounds = vec![f64::NEG_INFINITY; dim];
        for step_y in &traj.y {
            for (d, &val) in step_y.iter().enumerate() {
                trajectories[d].push(val);
                if val.is_finite() {
                    if val < min_bounds[d] {
                        min_bounds[d] = val;
                    }
                    if val > max_bounds[d] {
                        max_bounds[d] = val;
                    }
                }
            }
        }
        let final_state = traj.y.last().cloned().unwrap_or_else(|| spec.y0.clone());

        // Symbolic Jacobian and linearized local stability at initial point y0
        let (jacobian_unicode, jacobian_latex) =
            format_symbolic_jacobian(graph, &rhs_ids, &spec.state_vars);
        let (eigenvalues_text, eigenvalues_latex, stability_label) =
            analyze_linearized_stability(graph, &rhs_ids, spec);

        // 2D phase portrait quiver field along `phase_axes`
        let mut quiver_field = Vec::new();
        if dim >= 2 {
            let ax_x = spec.phase_axes[0].min(dim - 1);
            let ax_y = spec.phase_axes[1].min(dim - 1);
            let grid_n = 13usize;
            let [xmin, xmax] = spec.phase_x_range;
            let [ymin, ymax] = spec.phase_y_range;
            let base_t = spec.t_span[0];
            let mut base_state = spec.y0.clone();

            for ix in 0..grid_n {
                let gx = xmin + (xmax - xmin) * (ix as f64) / ((grid_n - 1) as f64);
                for iy in 0..grid_n {
                    let gy = ymin + (ymax - ymin) * (iy as f64) / ((grid_n - 1) as f64);
                    base_state[ax_x] = gx;
                    base_state[ax_y] = gy;
                    let mut call_params = Vec::with_capacity(dim + param_vals.len());
                    call_params.extend_from_slice(&base_state);
                    call_params.extend_from_slice(&param_vals);
                    let dx = vm
                        .eval_scalar(&programs[ax_x], base_t, &call_params)
                        .unwrap_or(0.0);
                    let dy = vm
                        .eval_scalar(&programs[ax_y], base_t, &call_params)
                        .unwrap_or(0.0);
                    let speed = dx.hypot(dy);
                    if speed.is_finite() && speed > 1e-12 {
                        quiver_field.push(QuiverArrow {
                            x: gx,
                            y: gy,
                            dx: dx / speed,
                            dy: dy / speed,
                            speed,
                        });
                    }
                }
            }
        }

        DifferentialSystemAnalysis {
            success: true,
            error_msg: None,
            symbolic_solution_text: sym_text,
            symbolic_solution_latex: sym_latex,
            method_used: traj.method_used,
            steps_accepted: traj.steps_accepted,
            steps_rejected: traj.steps_rejected,
            t: traj.t,
            trajectories,
            jacobian_unicode,
            jacobian_latex,
            eigenvalues_text,
            eigenvalues_latex,
            stability_label,
            quiver_field,
            final_state,
            min_bounds,
            max_bounds,
        }
    }
}

fn empty_de_error(dim: usize, msg: String) -> DifferentialSystemAnalysis {
    DifferentialSystemAnalysis {
        success: false,
        error_msg: Some(msg),
        symbolic_solution_text: None,
        symbolic_solution_latex: None,
        method_used: String::new(),
        steps_accepted: 0,
        steps_rejected: 0,
        t: Vec::new(),
        trajectories: vec![Vec::new(); dim],
        jacobian_unicode: String::new(),
        jacobian_latex: String::new(),
        eigenvalues_text: String::new(),
        eigenvalues_latex: String::new(),
        stability_label: "Unavailable".to_string(),
        quiver_field: Vec::new(),
        final_state: Vec::new(),
        min_bounds: Vec::new(),
        max_bounds: Vec::new(),
    }
}

fn parse_residual_expr(parser: &ExprParser<'_>, graph: &ExprGraph, raw: &str) -> Result<ExprId, String> {
    let parsed = parser.parse(raw).map_err(|e| e.to_string())?;
    Ok(match &graph.get(parsed).kind {
        ExprKind::Relational { op: RelOp::Equal, lhs, rhs } => graph.sub(*lhs, *rhs),
        _ => parsed,
    })
}

fn format_symbolic_jacobian(
    graph: &ExprGraph,
    exprs: &[ExprId],
    vars: &[String],
) -> (String, String) {
    let m = exprs.len();
    let n = vars.len();
    let var_syms: Vec<_> = vars.iter().map(|v| graph.symbols.get_or_intern(v)).collect();
    let mut elems = Vec::with_capacity(m * n);
    for &f_i in exprs {
        for &v_sym in &var_syms {
            let d = graph.diff(f_i, v_sym);
            elems.push(Simplifier::simplify(graph, d).unwrap_or(d));
        }
    }
    if let Ok(mat) = SymbolicMatrix::new(m, n, elems) {
        let id = mat.to_expr_id(graph);
        (
            UnicodeFormatter.format(graph, id).unwrap_or_default(),
            LatexFormatter.format(graph, id).unwrap_or_default(),
        )
    } else {
        (String::new(), String::new())
    }
}

fn format_complex_pair(re: f64, im: f64) -> String {
    let clean = |x: f64| if x.abs() < 1e-10 { 0.0 } else { x };
    let r = clean(re);
    let i = clean(im);
    if i == 0.0 {
        format!("{r:.4}")
    } else {
        let sign = if i >= 0.0 { "+" } else { "-" };
        format!("{r:.4} {sign} {:.4}i", i.abs())
    }
}

fn analyze_linearized_stability(
    graph: &ExprGraph,
    rhs_ids: &[ExprId],
    spec: &DifferentialSystemSpec,
) -> (String, String, String) {
    let dim = spec.state_vars.len();
    let mut bindings = HashMap::new();
    bindings.insert(spec.t_var.clone(), spec.t_span[0]);
    for (v, &val) in spec.state_vars.iter().zip(spec.y0.iter()) {
        bindings.insert(v.clone(), val);
    }
    for (p, v) in &spec.parameters {
        bindings.insert(p.clone(), *v);
    }
    let eval_ctx = EvalContext::with_bindings(20, bindings);

    let mut j_num = vec![0.0_f64; dim * dim];
    for (r, &f_r) in rhs_ids.iter().enumerate() {
        for (c, v_name) in spec.state_vars.iter().enumerate() {
            let v_sym = graph.symbols.get_or_intern(v_name);
            let d = graph.diff(f_r, v_sym);
            let simp = Simplifier::simplify(graph, d).unwrap_or(d);
            if let Ok(val) = graph.evalf(simp, &eval_ctx) {
                j_num[r * dim + c] = val.to_f64();
            }
        }
    }

    if dim == 1 {
        let lam = j_num[0];
        let label = if lam < -1e-9 {
            "Locally Stable (damping < 0)"
        } else if lam > 1e-9 {
            "Locally Unstable (growth > 0)"
        } else {
            "Marginal Equilibrium"
        };
        let s = format!("[{:.4}]", lam);
        return (s.clone(), format!("\\left\\{{ {:.4} \\right\\}}", lam), label.to_string());
    }

    if dim == 2 {
        let tr = j_num[0] + j_num[3];
        let det = j_num[0] * j_num[3] - j_num[1] * j_num[2];
        let disc = tr * tr - 4.0 * det;
        let (l1_str, l2_str) = if disc >= 0.0 {
            let sd = disc.sqrt();
            (
                format_complex_pair(0.5 * (tr + sd), 0.0),
                format_complex_pair(0.5 * (tr - sd), 0.0),
            )
        } else {
            let sd = (-disc).sqrt();
            (
                format_complex_pair(0.5 * tr, 0.5 * sd),
                format_complex_pair(0.5 * tr, -0.5 * sd),
            )
        };
        let label = if det < -1e-9 {
            "Saddle Point (Unstable)"
        } else if disc < 0.0 {
            if tr < -1e-9 {
                "Stable Spiral Focus"
            } else if tr > 1e-9 {
                "Unstable Spiral Focus"
            } else {
                "Center / Conservative Orbit"
            }
        } else if tr < -1e-9 {
            "Stable Node"
        } else if tr > 1e-9 {
            "Unstable Node"
        } else {
            "Degenerate / Non-hyperbolic"
        };
        let text = format!("[{}, {}]", l1_str, l2_str);
        let latex = format!("\\left\\{{ {},\\; {} \\right\\}}", l1_str, l2_str);
        return (text, latex, label.to_string());
    }

    // 3x3 / 4x4 fallback using SymbolicMatrix eigenvalues
    if dim <= 4 {
        let elems: Vec<ExprId> = j_num.iter().map(|&v| graph.float(v)).collect();
        if let Ok(mat) = SymbolicMatrix::new(dim, dim, elems)
            && let Ok(evs) = mat.eigenvalues(graph)
        {
            let mut u = Vec::new();
            let mut l = Vec::new();
            let mut max_re = f64::NEG_INFINITY;
            for ev in evs {
                let s = Simplifier::simplify(graph, ev).unwrap_or(ev);
                if let Ok(val) = graph.evalf(s, &EvalContext::default()) {
                    let (re, _) = val.as_complex();
                    if re > max_re {
                        max_re = re;
                    }
                }
                u.push(UnicodeFormatter.format(graph, s).unwrap_or_default());
                l.push(LatexFormatter.format(graph, s).unwrap_or_default());
            }
            let label = if max_re < -1e-9 {
                "Locally Stable (all Re(λ) < 0)"
            } else if max_re.is_finite() && max_re > 1e-9 {
                "Unstable Manifold Present (max Re(λ) > 0)"
            } else {
                "Linearized Spectrum Computed"
            };
            return (
                format!("[{}]", u.join(", ")),
                format!("\\left\\{{ {} \\right\\}}", l.join(",\\; ")),
                label.to_string(),
            );
        }
    }

    (
        "[]".to_string(),
        "\\left[ \\right]".to_string(),
        "High-dimensional Jacobian computed".to_string(),
    )
}

fn marching_squares_contours(
    vm: &BytecodeVM,
    prog: &BytecodeProgram,
    p_vals: &[f64],
    x_range: [f64; 2],
    y_range: [f64; 2],
    n: usize,
) -> Vec<ContourSegment> {
    let mut vals = vec![0.0_f64; (n + 1) * (n + 1)];
    let dx = (x_range[1] - x_range[0]) / (n as f64);
    let dy = (y_range[1] - y_range[0]) / (n as f64);
    let mut params = Vec::with_capacity(1 + p_vals.len());
    params.push(0.0);
    params.extend_from_slice(p_vals);

    for iy in 0..=n {
        let y = y_range[0] + dy * (iy as f64);
        params[0] = y;
        for ix in 0..=n {
            let x = x_range[0] + dx * (ix as f64);
            let v = vm.eval_scalar(prog, x, &params).unwrap_or(f64::NAN);
            vals[iy * (n + 1) + ix] = v;
        }
    }

    let interp = |a: f64, b: f64, p0: [f64; 2], p1: [f64; 2]| -> [f64; 2] {
        let denom = b - a;
        let t = if denom.abs() < 1e-14 {
            0.5
        } else {
            (-a / denom).clamp(0.0, 1.0)
        };
        [p0[0] + t * (p1[0] - p0[0]), p0[1] + t * (p1[1] - p0[1])]
    };

    let mut segs = Vec::new();
    for iy in 0..n {
        let y0 = y_range[0] + dy * (iy as f64);
        let y1 = y0 + dy;
        for ix in 0..n {
            let x0 = x_range[0] + dx * (ix as f64);
            let x1 = x0 + dx;
            let v00 = vals[iy * (n + 1) + ix];
            let v10 = vals[iy * (n + 1) + ix + 1];
            let v11 = vals[(iy + 1) * (n + 1) + ix + 1];
            let v01 = vals[(iy + 1) * (n + 1) + ix];
            if !v00.is_finite() || !v10.is_finite() || !v11.is_finite() || !v01.is_finite() {
                continue;
            }
            let pts_edges: &[(f64, [f64; 2], f64, [f64; 2])] = &[
                (v00, [x0, y0], v10, [x1, y0]), // bottom
                (v10, [x1, y0], v11, [x1, y1]), // right
                (v01, [x0, y1], v11, [x1, y1]), // top
                (v00, [x0, y0], v01, [x0, y1]), // left
            ];
            let mut crossings = Vec::with_capacity(4);
            for &(a, pa, b, pb) in pts_edges {
                if (a <= 0.0 && b > 0.0) || (a > 0.0 && b <= 0.0) {
                    crossings.push(interp(a, b, pa, pb));
                }
            }
            if crossings.len() == 2 {
                segs.push(ContourSegment {
                    x0: crossings[0][0],
                    y0: crossings[0][1],
                    x1: crossings[1][0],
                    y1: crossings[1][1],
                });
            } else if crossings.len() == 4 {
                segs.push(ContourSegment {
                    x0: crossings[0][0],
                    y0: crossings[0][1],
                    x1: crossings[1][0],
                    y1: crossings[1][1],
                });
                segs.push(ContourSegment {
                    x0: crossings[2][0],
                    y0: crossings[2][1],
                    x1: crossings[3][0],
                    y1: crossings[3][1],
                });
            }
        }
    }
    segs
}

fn extract_2d_solution_points(
    graph: &ExprGraph,
    sol_text: &str,
    v0: &str,
    v1: &str,
    bindings: &HashMap<String, f64>,
) -> Vec<[f64; 2]> {
    let parser = ExprParser::new(graph);
    let eval_ctx = EvalContext::with_bindings(20, bindings.clone());
    let mut pts = Vec::new();
    let mut search = sol_text;

    while let Some(open) = search.find('{') {
        let rest = &search[open + 1..];
        let Some(close) = rest.find('}') else { break };
        let inner = &rest[..close];
        let mut x_val = None;
        let mut y_val = None;
        for part in inner.split(',') {
            if let Some((lhs, rhs)) = part.split_once('=') {
                let name = lhs.trim();
                let val_str = rhs.trim();
                if let Ok(eid) = parser.parse(val_str)
                    && let Ok(bv) = graph.evalf(eid, &eval_ctx)
                {
                    let (re, im) = bv.as_complex();
                    if im.abs() < 1e-8 && re.is_finite() {
                        if name == v0 {
                            x_val = Some(re);
                        } else if name == v1 {
                            y_val = Some(re);
                        }
                    }
                }
            }
        }
        if let (Some(x), Some(y)) = (x_val, y_val) {
            pts.push([x, y]);
        }
        search = &rest[close + 1..];
    }
    pts
}
