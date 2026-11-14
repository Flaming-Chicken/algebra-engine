//! # `urae-py` (URAE Python Session & Expression Wrapper)
//!
//! Rust-side Python session & expression wrapper for the Universal Rust Algebra Engine (URAE)
//! (PyO3 `#[pymodule]` C-extension bindings scheduled for Wave 2).
//!
//! Prepares symbolic differentiation, E-Graph simplification, equation solving,
//! parametric CAD machinery generation, multiphysics FEA simulation pipelines,
//! and quantum circuit state vectors for Python interop (`import urae`).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use urae::prelude::*;

/// Helper to format an expression node into `PyExprGraph`.
fn node_to_py_expr(graph: &ExprGraph, id: ExprId) -> Result<PyExprGraph, String> {
    let latex = LatexFormatter.format(graph, id).map_err(|e| e.to_string())?;
    let unicode = UnicodeFormatter
        .format(graph, id)
        .map_err(|e| e.to_string())?;
    Ok(PyExprGraph {
        raw_text: unicode.clone(),
        latex,
        unicode,
    })
}

/// Python-compatible Symbolic Expression Graph Wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PyExprGraph {
    pub raw_text: String,
    pub latex: String,
    pub unicode: String,
}

impl PyExprGraph {
    /// Create new graph from mathematical expression string.
    pub fn parse(expr_str: &str) -> Result<Self, String> {
        let graph = ExprGraph::new();
        let parser = ExprParser::new(&graph);
        match parser.parse(expr_str) {
            Ok(id) => {
                let latex = LatexFormatter.format(&graph, id).unwrap_or_default();
                let unicode = UnicodeFormatter.format(&graph, id).unwrap_or_default();
                Ok(Self {
                    raw_text: expr_str.to_string(),
                    latex,
                    unicode,
                })
            }
            Err(err) => Err(format!("Parse error: {}", err)),
        }
    }

    /// Symbolically differentiate with respect to variable string.
    pub fn diff(&self, wrt: &str) -> Result<Self, String> {
        let graph = ExprGraph::new();
        let parser = ExprParser::new(&graph);
        let expr_id = parser.parse(&self.raw_text).map_err(|e| e.to_string())?;
        let wrt_sym = graph.symbols.get_or_intern(wrt);
        let diff_id = graph.diff(expr_id, wrt_sym);
        let latex = LatexFormatter.format(&graph, diff_id).unwrap_or_default();
        let unicode = UnicodeFormatter.format(&graph, diff_id).unwrap_or_default();
        Ok(Self {
            raw_text: unicode.clone(),
            latex,
            unicode,
        })
    }

    /// Symbolically integrate using Risch algorithm with respect to variable string.
    pub fn integrate(&self, wrt: &str) -> Result<Self, String> {
        let graph = ExprGraph::new();
        let parser = ExprParser::new(&graph);
        let expr_id = parser.parse(&self.raw_text).map_err(|e| e.to_string())?;
        let wrt_sym = graph.symbols.get_or_intern(wrt);
        let int_id = graph
            .integrate(expr_id, wrt_sym)
            .map_err(|e| e.to_string())?;
        let latex = LatexFormatter.format(&graph, int_id).unwrap_or_default();
        let unicode = UnicodeFormatter.format(&graph, int_id).unwrap_or_default();
        Ok(Self {
            raw_text: unicode.clone(),
            latex,
            unicode,
        })
    }

    /// Simplify expression using E-Graph pattern saturation.
    pub fn simplify(&self) -> Result<Self, String> {
        let graph = ExprGraph::new();
        let parser = ExprParser::new(&graph);
        let expr_id = parser.parse(&self.raw_text).map_err(|e| e.to_string())?;
        let simp_id = Simplifier::simplify(&graph, expr_id).map_err(|e| e.to_string())?;
        node_to_py_expr(&graph, simp_id)
    }

    /// Compute symbolic limit $\lim_{\text{wrt} \to \text{point}} \text{self}$.
    pub fn limit(&self, wrt: &str, point: &str) -> Result<Self, String> {
        let graph = ExprGraph::new();
        let parser = ExprParser::new(&graph);
        let expr_id = parser.parse(&self.raw_text).map_err(|e| e.to_string())?;
        let pt_id = parser.parse(point).map_err(|e| e.to_string())?;
        let wrt_sym = graph.symbols.get_or_intern(wrt);
        let lim_id = graph
            .limit(expr_id, wrt_sym, pt_id)
            .map_err(|e| e.to_string())?;
        let simp_id = Simplifier::simplify(&graph, lim_id).unwrap_or(lim_id);
        node_to_py_expr(&graph, simp_id)
    }

    /// Compute Taylor series expansion around `point` up to `order`.
    pub fn series(&self, wrt: &str, point: &str, order: u32) -> Result<Self, String> {
        let graph = ExprGraph::new();
        let parser = ExprParser::new(&graph);
        let expr_id = parser.parse(&self.raw_text).map_err(|e| e.to_string())?;
        let pt_id = parser.parse(point).map_err(|e| e.to_string())?;
        let wrt_sym = graph.symbols.get_or_intern(wrt);
        let ser_id = graph.taylor_series(expr_id, wrt_sym, pt_id, order);
        let simp_id = Simplifier::simplify(&graph, ser_id).unwrap_or(ser_id);
        node_to_py_expr(&graph, simp_id)
    }

    /// Expand products and polynomial powers.
    pub fn expand(&self) -> Result<Self, String> {
        let graph = ExprGraph::new();
        let parser = ExprParser::new(&graph);
        let expr_id = parser.parse(&self.raw_text).map_err(|e| e.to_string())?;
        let expanded = AlgebraicTransformations::expand(&graph, expr_id);
        let simp_id = Simplifier::simplify(&graph, expanded).unwrap_or(expanded);
        node_to_py_expr(&graph, simp_id)
    }

    /// Factor quadratic polynomial expression in `x`.
    pub fn factor(&self) -> Result<Self, String> {
        let graph = ExprGraph::new();
        let parser = ExprParser::new(&graph);
        let expr_id = parser.parse(&self.raw_text).map_err(|e| e.to_string())?;
        let factored = AlgebraicTransformations::factor(&graph, expr_id);
        node_to_py_expr(&graph, factored)
    }

    /// Perform partial fractions decomposition with respect to `wrt`.
    pub fn partial_fractions(&self, wrt: &str) -> Result<Self, String> {
        let graph = ExprGraph::new();
        let parser = ExprParser::new(&graph);
        let expr_id = parser.parse(&self.raw_text).map_err(|e| e.to_string())?;
        let wrt_sym = graph.symbols.get_or_intern(wrt);
        let pf_id = AlgebraicTransformations::partial_fractions(&graph, expr_id, wrt_sym);
        node_to_py_expr(&graph, pf_id)
    }

    /// Compute unilateral Laplace transform $\mathcal{L}\{f(t)\}(s)$.
    pub fn laplace(&self, t_var: &str, s_var: &str) -> Result<Self, String> {
        let graph = ExprGraph::new();
        let parser = ExprParser::new(&graph);
        let expr_id = parser.parse(&self.raw_text).map_err(|e| e.to_string())?;
        let t_sym = graph.symbols.get_or_intern(t_var);
        let s_sym = graph.symbols.get_or_intern(s_var);
        let lap_id = graph
            .laplace_transform(expr_id, t_sym, s_sym)
            .map_err(|e| e.to_string())?;
        node_to_py_expr(&graph, lap_id)
    }
}

/// Standalone Functional API for Python.
pub fn py_diff(expr: &str, wrt: &str) -> Result<String, String> {
    let py_graph = PyExprGraph::parse(expr)?;
    let res = py_graph.diff(wrt)?;
    Ok(res.latex)
}

pub fn py_integrate(expr: &str, wrt: &str) -> Result<String, String> {
    let py_graph = PyExprGraph::parse(expr)?;
    let res = py_graph.integrate(wrt)?;
    Ok(res.latex)
}

pub fn py_simplify(expr: &str) -> Result<String, String> {
    let py_graph = PyExprGraph::parse(expr)?;
    let res = py_graph.simplify()?;
    Ok(res.latex)
}

pub fn py_solve(eq_str: &str, wrt: &str) -> Result<Vec<String>, String> {
    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let eq_id = parser.parse(eq_str).map_err(|e| e.to_string())?;
    let sym = graph.symbols.get_or_intern(wrt);
    let sols = graph.solveset(eq_id, sym).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for s in sols {
        out.push(LatexFormatter.format(&graph, s).unwrap_or_default());
    }
    Ok(out)
}

/// Summary trajectory returned by [`py_ode_solve`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OdeTrajectorySummary {
    pub t: Vec<f64>,
    pub y: Vec<Vec<f64>>,
    pub method_used: String,
    pub steps_accepted: usize,
    pub steps_rejected: usize,
}

/// Compute symbolic limit $\lim_{\text{wrt} \to \text{point}} \text{expr}$ and return Unicode math text.
pub fn py_limit(expr: &str, wrt: &str, point: &str) -> Result<String, String> {
    let py_graph = PyExprGraph::parse(expr)?;
    Ok(py_graph.limit(wrt, point)?.unicode)
}

/// Compute Taylor series expansion of `expr` around `point` up to `order`.
pub fn py_series(expr: &str, wrt: &str, point: &str, order: u32) -> Result<String, String> {
    let py_graph = PyExprGraph::parse(expr)?;
    Ok(py_graph.series(wrt, point, order)?.unicode)
}

/// Factor a quadratic polynomial expression in `x`.
pub fn py_factor(expr: &str) -> Result<String, String> {
    let py_graph = PyExprGraph::parse(expr)?;
    Ok(py_graph.factor()?.unicode)
}

/// Expand products and polynomial powers in `expr`.
pub fn py_expand(expr: &str) -> Result<String, String> {
    let py_graph = PyExprGraph::parse(expr)?;
    Ok(py_graph.expand()?.unicode)
}

/// Decompose rational expression into partial fractions with respect to `wrt`.
pub fn py_partial_fractions(expr: &str, wrt: &str) -> Result<String, String> {
    let py_graph = PyExprGraph::parse(expr)?;
    Ok(py_graph.partial_fractions(wrt)?.unicode)
}

/// Compute unilateral Laplace transform $\mathcal{L}\{f(t)\}(s)$.
pub fn py_laplace(expr: &str, t_var: &str, s_var: &str) -> Result<String, String> {
    let py_graph = PyExprGraph::parse(expr)?;
    Ok(py_graph.laplace(t_var, s_var)?.unicode)
}

/// Compute reduced Gröbner basis (grevlex) for polynomial generators in `vars`.
pub fn py_grobner_basis(polys: &[&str], vars: &[&str]) -> Result<Vec<String>, String> {
    if polys.is_empty() {
        return Err("At least one generator polynomial is required".to_string());
    }
    if vars.is_empty() {
        return Err("At least one variable name is required".to_string());
    }

    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let mut generators = Vec::with_capacity(polys.len());
    for poly_str in polys {
        generators.push(parser.parse(poly_str).map_err(|e| e.to_string())?);
    }

    let var_names: Vec<String> = vars.iter().map(|v| (*v).to_string()).collect();
    let basis = GrobnerBasis::from_exprs(
        &graph,
        &generators,
        &var_names,
        MonomialOrder::GrevLex,
    )
    .map_err(|e| e.to_string())?;

    basis
        .polynomials
        .iter()
        .map(|p| {
            let expr_id = p.to_expr(&graph);
            UnicodeFormatter.format(&graph, expr_id).map_err(|e| e.to_string())
        })
        .collect()
}

fn validate_square_matrix(rows: &[Vec<f64>]) -> Result<usize, String> {
    if rows.is_empty() {
        return Err("Matrix must not be empty".to_string());
    }
    let n = rows.len();
    if rows.iter().any(|r| r.len() != n) {
        return Err(format!("Matrix must be square ({n} columns expected per row)"));
    }
    if rows.iter().flatten().any(|v| !v.is_finite()) {
        return Err("Matrix entries must be finite".to_string());
    }
    Ok(n)
}

/// Compute the determinant of a real $N \times N$ matrix via Gaussian elimination with partial pivoting.
pub fn py_matrix_det(rows: &[Vec<f64>]) -> Result<f64, String> {
    let n = validate_square_matrix(rows)?;
    let mut mat = rows.to_vec();
    let mut sign = 1.0_f64;

    for k in 0..n {
        let (pivot_idx, pivot_abs) = mat[k..n]
            .iter()
            .enumerate()
            .map(|(offset, row)| (k + offset, row[k].abs()))
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap_or((k, 0.0));

        if pivot_abs < 1e-12 {
            return Ok(0.0);
        }
        if pivot_idx != k {
            mat.swap(k, pivot_idx);
            sign = -sign;
        }

        let pivot = mat[k][k];
        let (upper, lower) = mat.split_at_mut(k + 1);
        let pivot_row = &upper[k];
        for row in lower {
            let factor = row[k] / pivot;
            for (dst, &src) in row[k..n].iter_mut().zip(&pivot_row[k..n]) {
                *dst -= factor * src;
            }
        }
    }

    let mut det = sign;
    for (i, row) in mat.iter().enumerate() {
        det *= row[i];
    }
    Ok(det)
}

/// Compute the inverse of a real invertible $N \times N$ matrix via Gauss-Jordan elimination.
pub fn py_matrix_inv(rows: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, String> {
    let n = validate_square_matrix(rows)?;
    let mut aug: Vec<Vec<f64>> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let mut full = Vec::with_capacity(2 * n);
            full.extend_from_slice(row);
            full.resize(2 * n, 0.0);
            full[n + i] = 1.0;
            full
        })
        .collect();

    for k in 0..n {
        let (pivot_idx, pivot_abs) = aug[k..n]
            .iter()
            .enumerate()
            .map(|(offset, row)| (k + offset, row[k].abs()))
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap_or((k, 0.0));

        if pivot_abs < 1e-12 {
            return Err("Matrix is singular and cannot be inverted".to_string());
        }
        if pivot_idx != k {
            aug.swap(k, pivot_idx);
        }

        let pivot = aug[k][k];
        for val in &mut aug[k] {
            *val /= pivot;
        }

        let pivot_row = aug[k].clone();
        for (i, row) in aug.iter_mut().enumerate() {
            if i == k {
                continue;
            }
            let factor = row[k];
            if factor.abs() > 0.0 {
                for (dst, &src) in row.iter_mut().zip(&pivot_row) {
                    *dst -= factor * src;
                }
            }
        }
    }

    Ok(aug.into_iter().map(|row| row[n..].to_vec()).collect())
}

/// Integrate an ODE system $\frac{d\mathbf{y}}{dt} = \mathbf{f}(t, \mathbf{y})$ specified symbolically.
///
/// Supported `method` values: `"auto"`, `"dopri5"`, `"tsit5"`, `"radau5"`, `"meta"`, `"rk4"`, `"verlet"`, `"gl4"`.
pub fn py_ode_solve(
    rhs_exprs: &[&str],
    state_vars: &[&str],
    t_var: &str,
    t_span: (f64, f64),
    y0: &[f64],
    method: &str,
) -> Result<OdeTrajectorySummary, String> {
    if state_vars.is_empty() {
        return Err("At least one state variable is required".to_string());
    }
    if rhs_exprs.len() != state_vars.len() {
        return Err(format!(
            "rhs_exprs length ({}) must equal state_vars length ({})",
            rhs_exprs.len(),
            state_vars.len()
        ));
    }
    if y0.len() != state_vars.len() {
        return Err(format!(
            "y0 length ({}) must equal state_vars length ({})",
            y0.len(),
            state_vars.len()
        ));
    }
    if !t_span.0.is_finite()
        || !t_span.1.is_finite()
        || (t_span.1 - t_span.0).abs() <= f64::EPSILON
    {
        return Err("t_span must be finite with t0 != tend".to_string());
    }
    if y0.iter().any(|v| !v.is_finite()) {
        return Err("Initial state y0 entries must be finite".to_string());
    }

    let ode_method = match method.trim().to_ascii_lowercase().as_str() {
        "auto" => NumericalOdeMethod::Auto,
        "dopri5" | "dormand_prince54" => NumericalOdeMethod::DormandPrince54,
        "tsit5" | "tsitouras54" => NumericalOdeMethod::Tsitouras54,
        "radau5" | "radau_iia5" => NumericalOdeMethod::RadauIIA5,
        "meta" | "self_optimizing_meta" => NumericalOdeMethod::SelfOptimizingMeta,
        "rk4" => NumericalOdeMethod::Rk4,
        "verlet" | "velocity_verlet" => NumericalOdeMethod::VelocityVerlet,
        "gl4" | "gauss_legendre4" => NumericalOdeMethod::GaussLegendre4,
        other => return Err(format!("Unsupported ODE method: {other}")),
    };

    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let mut rhs_ids = Vec::with_capacity(rhs_exprs.len());
    for expr_str in rhs_exprs {
        rhs_ids.push(parser.parse(expr_str).map_err(|e| e.to_string())?);
    }

    let eval_rhs = |t: f64, y: &[f64]| -> Result<Vec<f64>, String> {
        let mut bindings = HashMap::with_capacity(state_vars.len() + 1);
        bindings.insert(t_var.to_string(), t);
        for (name, &val) in state_vars.iter().zip(y.iter()) {
            bindings.insert((*name).to_string(), val);
        }
        let ctx = EvalContext::with_bindings(50, bindings);
        rhs_ids
            .iter()
            .map(|&id| {
                graph
                    .evalf(id, &ctx)
                    .map(|val| val.to_f64())
                    .map_err(|e| e.to_string())
            })
            .collect()
    };

    // Validate initial RHS evaluation before launching solver.
    let initial_eval = eval_rhs(t_span.0, y0)?;
    if initial_eval.iter().any(|v| !v.is_finite()) {
        return Err("Initial RHS evaluation produced a non-finite value".to_string());
    }

    let config = NumericalOdeConfig {
        method: ode_method,
        ..Default::default()
    };

    let traj = NumericalOdeSolver::solve(
        |t, y| eval_rhs(t, y).unwrap_or_else(|_| vec![f64::NAN; state_vars.len()]),
        t_span,
        y0,
        &config,
    )
    .map_err(|e| e.to_string())?;

    Ok(OdeTrajectorySummary {
        t: traj.t,
        y: traj.y,
        method_used: traj.method_used,
        steps_accepted: traj.steps_accepted,
        steps_rejected: traj.steps_rejected,
    })
}

/// Python-compatible Involute Gear Generator Wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PyInvoluteGear {
    pub teeth: usize,
    pub module: f64,
    pub face_width: f64,
    pub vertex_count: usize,
    pub triangle_count: usize,
}

impl PyInvoluteGear {
    pub fn spur(module: f64, teeth: usize, face_width: f64) -> Self {
        let gear = urae::cad::InvoluteGear::spur(module, teeth, face_width);
        let mesh = gear.generate_3d_mesh();
        Self {
            teeth,
            module,
            face_width,
            vertex_count: mesh.vertices.len(),
            triangle_count: mesh.triangles.len(),
        }
    }

    pub fn to_obj(&self) -> String {
        let gear = urae::cad::InvoluteGear::spur(self.module, self.teeth, self.face_width);
        let mesh = gear.generate_3d_mesh();
        urae::cad::ShapeExporter::export_obj(&mesh)
    }

    pub fn to_stl(&self) -> String {
        let gear = urae::cad::InvoluteGear::spur(self.module, self.teeth, self.face_width);
        let mesh = gear.generate_3d_mesh();
        urae::cad::ShapeExporter::export_stl_ascii(&mesh, "involute_gear")
    }
}

/// Python-compatible Quantum Circuit Wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PyQuantumCircuit {
    pub num_qubits: usize,
    pub gate_count: usize,
    pub probabilities: Vec<f64>,
}

impl PyQuantumCircuit {
    pub fn bell_pair() -> Self {
        let bell = urae::quantum_circuit::QuantumCircuit::bell_pair();
        let probs = bell.probabilities();
        Self {
            num_qubits: 2,
            gate_count: bell.gates.len(),
            probabilities: probs,
        }
    }

    pub fn ghz_state(n: usize) -> Self {
        let ghz = urae::quantum_circuit::QuantumCircuit::ghz_state(n);
        let probs = ghz.probabilities();
        Self {
            num_qubits: n,
            gate_count: ghz.gates.len(),
            probabilities: probs,
        }
    }

    pub fn to_openqasm(&self) -> String {
        let ghz = urae::quantum_circuit::QuantumCircuit::ghz_state(self.num_qubits);
        ghz.to_openqasm()
    }
}

/// Python-compatible persistent Session runtime.
#[derive(Debug, Default)]
pub struct PySession {
    pub inner: UraeSession,
}

impl PySession {
    pub fn new() -> Self {
        Self {
            inner: UraeSession::new(),
        }
    }

    pub fn execute(&mut self, input: &str) -> Result<String, String> {
        let res = self.inner.execute_line(input);
        if res.is_error {
            Err(res.error_msg.unwrap_or(res.output_text))
        } else {
            Ok(res.output_unicode)
        }
    }

    pub fn execute_latex(&mut self, input: &str) -> Result<String, String> {
        let res = self.inner.execute_line(input);
        if res.is_error {
            Err(res.error_msg.unwrap_or(res.output_text))
        } else {
            Ok(res.output_latex)
        }
    }

    pub fn set_variable(&mut self, name: &str, val: f64) {
        self.inner.set_binding(name, val);
    }

    pub fn get_variable(&self, name: &str) -> Option<f64> {
        self.inner.get_binding(name)
    }

    pub fn reset(&mut self) {
        self.inner.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval_at(expr: &str, var: &str, val: f64) -> f64 {
        let graph = ExprGraph::new();
        let parser = ExprParser::new(&graph);
        let id = parser.parse(expr).expect("parse result expression");
        let mut bindings = HashMap::new();
        bindings.insert(var.to_string(), val);
        let ctx = EvalContext::with_bindings(50, bindings);
        graph.evalf(id, &ctx).expect("evaluate result").to_f64()
    }

    #[test]
    fn test_py_calculus_algebra_and_transforms() {
        let d = py_diff("x^3", "x").expect("py_diff");
        assert!(!d.is_empty());

        let i = py_integrate("x^2", "x").expect("py_integrate");
        assert!(!i.is_empty());

        let s = py_simplify("x + 0").expect("py_simplify");
        assert_eq!(s, "x");

        let sols = py_solve("x - 4 = 0", "x").expect("py_solve");
        assert!(!sols.is_empty());

        let lim = py_limit("sin(x) / x", "x", "0").expect("py_limit");
        let lim_val = eval_at(&lim, "x", 0.0);
        assert!((lim_val - 1.0).abs() < 1e-9, "unexpected limit: {lim}");

        let ser = py_series("exp(x)", "x", "0", 3).expect("py_series");
        let approx = eval_at(&ser, "x", 0.2);
        let expected = 1.0 + 0.2 + 0.04 / 2.0 + 0.008 / 6.0;
        assert!((approx - expected).abs() < 1e-6, "series={ser}");

        let expanded = py_expand("(x + 1)^2").expect("py_expand");
        let val_expanded = eval_at(&expanded, "x", 3.0);
        assert!((val_expanded - 16.0).abs() < 1e-9, "expanded={expanded}");

        let factored = py_factor("x^2 - 5*x + 6").expect("py_factor");
        assert!(
            factored.contains("-3") && factored.contains("-2") && factored.contains('·'),
            "factored={factored}"
        );

        let pf = py_partial_fractions("1 / (x^2 - 1)", "x").expect("py_partial_fractions");
        assert!(
            (eval_at(&pf, "x", 3.0) - 0.125).abs() < 1e-9,
            "partial_fractions={pf}"
        );

        let lap = py_laplace("sin(t)", "t", "s").expect("py_laplace");
        let lap_at_2 = eval_at(&lap, "s", 2.0);
        assert!((lap_at_2 - 0.2).abs() < 1e-9, "laplace={lap}");

        let gb = py_grobner_basis(&["x^2 - 1", "x*y - 1"], &["x", "y"]).expect("py_grobner_basis");
        assert!(!gb.is_empty());
    }

    #[test]
    fn test_py_matrix_det_and_inv() {
        let mat = vec![vec![4.0, 7.0], vec![2.0, 6.0]];
        let det = py_matrix_det(&mat).expect("determinant");
        assert!((det - 10.0).abs() < 1e-12);

        let inv = py_matrix_inv(&mat).expect("inverse");
        assert!((inv[0][0] - 0.6).abs() < 1e-12);
        assert!((inv[0][1] - -0.7).abs() < 1e-12);
        assert!((inv[1][0] - -0.2).abs() < 1e-12);
        assert!((inv[1][1] - 0.4).abs() < 1e-12);

        let singular = vec![vec![1.0, 2.0], vec![2.0, 4.0]];
        assert!((py_matrix_det(&singular).expect("singular det")).abs() < 1e-12);
        assert!(py_matrix_inv(&singular).is_err());
        assert!(py_matrix_det(&[vec![1.0, 2.0]]).is_err());
    }

    #[test]
    fn test_py_ode_solve_tsit5_and_meta() {
        let traj_tsit5 = py_ode_solve(&["-y"], &["y"], "t", (0.0, 1.0), &[1.0], "tsit5")
            .expect("tsit5 ode solve");
        assert!(traj_tsit5.method_used.contains("Tsitouras"));
        let final_tsit5 = traj_tsit5.y.last().expect("non-empty tsit5 trajectory")[0];
        let exact = (-1.0_f64).exp();
        assert!((final_tsit5 - exact).abs() < 1e-5, "tsit5 final={final_tsit5}");

        let traj_meta = py_ode_solve(&["-y"], &["y"], "t", (0.0, 1.0), &[1.0], "meta")
            .expect("meta ode solve");
        assert!(traj_meta.method_used.contains("SelfOptimizingMeta"));
        let final_meta = traj_meta.y.last().expect("non-empty meta trajectory")[0];
        assert!((final_meta - exact).abs() < 1e-2, "meta final={final_meta}");

        assert!(py_ode_solve(&["-y"], &["y"], "t", (0.0, 1.0), &[1.0], "unknown").is_err());
    }

    #[test]
    fn test_py_session_execution() {
        let mut session = PySession::new();
        session.set_variable("k", 5.0);
        assert_eq!(session.get_variable("k"), Some(5.0));
        let out = session.execute("2 + 3").expect("session execute");
        assert!(out.contains('5'));
        session.reset();
        assert_eq!(session.get_variable("k"), None);
    }
}
