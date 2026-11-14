//! # `urae-py` (URAE Python Session & Expression Wrapper)
//!
//! Rust-side Python session & expression wrapper for the Universal Rust Algebra Engine (URAE)
//! (PyO3 `#[pymodule]` C-extension bindings scheduled for Wave 2).
//!
//! Prepares symbolic differentiation, E-Graph simplification, equation solving,
//! parametric CAD machinery generation, multiphysics FEA simulation pipelines,
//! and quantum circuit state vectors for Python interop (`import urae`).

use serde::{Deserialize, Serialize};
use urae::prelude::*;

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
        let latex = LatexFormatter.format(&graph, simp_id).unwrap_or_default();
        let unicode = UnicodeFormatter.format(&graph, simp_id).unwrap_or_default();
        Ok(Self {
            raw_text: unicode.clone(),
            latex,
            unicode,
        })
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
