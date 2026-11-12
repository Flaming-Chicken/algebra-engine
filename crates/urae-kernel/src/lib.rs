//! # `urae-kernel` (Native Zero-Latency Jupyter Kernel Protocol for URAE)
//!
//! Exposes URAE continuous mathematics notepad and symbolic computation as a standard Jupyter Kernel.
//!
//! Produces multi-MIME rich display payloads:
//! - `text/plain`: Formatted Unicode math.
//! - `text/latex`: MathJax / KaTeX rendering ($$\dots$$).
//! - `text/html`: Embedded interactive Three.js 3D rotating CAD viewport.
//! - `application/json`: Structured symbolic AST representation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use urae::prelude::*;

/// Rich Display MIME Payload Bundle for Jupyter Frontends.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct MimeBundle {
    pub data: HashMap<String, String>,
    pub metadata: HashMap<String, String>,
}

impl MimeBundle {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_plain(mut self, text: impl Into<String>) -> Self {
        self.data.insert("text/plain".to_string(), text.into());
        self
    }

    pub fn with_latex(mut self, latex: impl Into<String>) -> Self {
        let l_str = latex.into();
        let formatted = if l_str.starts_with("$$") {
            l_str
        } else {
            format!("$${}$$", l_str)
        };
        self.data.insert("text/latex".to_string(), formatted);
        self
    }

    pub fn with_html(mut self, html: impl Into<String>) -> Self {
        self.data.insert("text/html".to_string(), html.into());
        self
    }

    pub fn with_json(mut self, json: impl Into<String>) -> Self {
        self.data
            .insert("application/json".to_string(), json.into());
        self
    }
}

/// Jupyter Kernel Execution Reply.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteReply {
    pub execution_count: usize,
    pub status: String,
    pub mime_bundle: MimeBundle,
    pub user_expressions: HashMap<String, String>,
}

/// Cell execution history record in a notebook or interactive session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CellRecord {
    pub id: usize,
    pub input_code: String,
    pub status: String,
    pub output_plain: String,
    pub output_latex: String,
}

/// Unified Notebook & CLI Session Controller.
///
/// Bridges stateful mathematics execution, reactive slider parameter scrub updates,
/// cell-by-cell history tracking, multi-MIME evaluation, and document serialization across
/// `urae-cli`, `urae-notebook`, and the Jupyter kernel runtime.
#[derive(Debug, Default)]
pub struct NotebookSessionController {
    /// Inner execution session state.
    pub session: UraeSession,
    /// Sequential cell execution count.
    pub execution_count: usize,
    /// Chronological cell history records.
    pub cell_history: Vec<CellRecord>,
    /// Undo snapshot stack of variable bindings.
    pub undo_stack: Vec<HashMap<String, f64>>,
}

impl NotebookSessionController {
    pub fn new() -> Self {
        Self {
            session: UraeSession::new(),
            execution_count: 0,
            cell_history: Vec::new(),
            undo_stack: Vec::new(),
        }
    }

    /// Reset session state and clear history.
    pub fn reset(&mut self) {
        self.session.reset();
        self.execution_count = 0;
        self.cell_history.clear();
        self.undo_stack.clear();
    }

    /// Execute a code cell or line, producing an `ExecuteReply` with rich multi-MIME output.
    pub fn execute_cell(&mut self, code: &str) -> ExecuteReply {
        self.execution_count += 1;
        let trimmed = code.trim();

        // Push undo state before mutation
        self.undo_stack.push(self.session.bindings.clone());
        if self.undo_stack.len() > 50 {
            self.undo_stack.remove(0);
        }

        // CAD gear command
        if trimmed.starts_with("gear!") || trimmed.contains("InvoluteGear") {
            let gear = urae::cad::InvoluteGear::spur(1.5, 16, 6.0);
            let mesh = gear.generate_3d_mesh();
            let html = urae::html_export::StandaloneHtmlExporter::export_mesh_to_html(
                "Involute Gear (16T, m=1.5)",
                "Parametric 3D Spur Gear CAD Model",
                "r_b = r \\cos(\\alpha)",
                &mesh,
            );
            let mime = MimeBundle::new()
                .with_plain("InvoluteGear: 16 Teeth, Module 1.5, Face Width 6.0mm")
                .with_latex("\\text{InvoluteGear}(N=16, m=1.5, b=6.0)")
                .with_html(html);

            self.cell_history.push(CellRecord {
                id: self.execution_count,
                input_code: trimmed.to_string(),
                status: "ok".to_string(),
                output_plain: "InvoluteGear: 16 Teeth, Module 1.5, Face Width 6.0mm".to_string(),
                output_latex: "\\text{InvoluteGear}(N=16, m=1.5, b=6.0)".to_string(),
            });

            return ExecuteReply {
                execution_count: self.execution_count,
                status: "ok".to_string(),
                mime_bundle: mime,
                user_expressions: HashMap::new(),
            };
        }

        // Quantum bell pair command
        if trimmed.starts_with("quantum_bell") || trimmed.contains("bell_pair") {
            let bell = urae::quantum_circuit::QuantumCircuit::bell_pair();
            let qasm = bell.to_openqasm();
            let mime = MimeBundle::new()
                .with_plain(format!(
                    "Quantum Bell State: |Phi+> = (|00> + |11>)/sqrt(2)\n\n{}",
                    qasm
                ))
                .with_latex("\\frac{|00\\rangle + |11\\rangle}{\\sqrt{2}}");

            self.cell_history.push(CellRecord {
                id: self.execution_count,
                input_code: trimmed.to_string(),
                status: "ok".to_string(),
                output_plain: format!("Quantum Bell State:\n{}", qasm),
                output_latex: "\\frac{|00\\rangle + |11\\rangle}{\\sqrt{2}}".to_string(),
            });

            return ExecuteReply {
                execution_count: self.execution_count,
                status: "ok".to_string(),
                mime_bundle: mime,
                user_expressions: HashMap::new(),
            };
        }

        // Execute against persistent URAE Session
        let res = self.session.execute_line(trimmed);
        if res.is_error {
            let err_msg = res.error_msg.unwrap_or(res.output_text);
            let mime = MimeBundle::new().with_plain(format!("Error: {}", err_msg));
            self.cell_history.push(CellRecord {
                id: self.execution_count,
                input_code: trimmed.to_string(),
                status: "error".to_string(),
                output_plain: format!("Error: {}", err_msg),
                output_latex: String::new(),
            });

            ExecuteReply {
                execution_count: self.execution_count,
                status: "error".to_string(),
                mime_bundle: mime,
                user_expressions: HashMap::new(),
            }
        } else {
            let mime = MimeBundle::new()
                .with_plain(&res.output_unicode)
                .with_latex(&res.output_latex);

            self.cell_history.push(CellRecord {
                id: self.execution_count,
                input_code: trimmed.to_string(),
                status: "ok".to_string(),
                output_plain: res.output_unicode,
                output_latex: res.output_latex,
            });

            ExecuteReply {
                execution_count: self.execution_count,
                status: "ok".to_string(),
                mime_bundle: mime,
                user_expressions: HashMap::new(),
            }
        }
    }

    /// Update or scrub an interactive parameter slider value.
    pub fn set_parameter(&mut self, name: &str, value: f64) {
        self.session.set_binding(name, value);
    }

    /// Retrieve value of a symbol binding.
    pub fn get_parameter(&self, name: &str) -> Option<f64> {
        self.session.get_binding(name)
    }

    /// Revert to previous session snapshot.
    pub fn undo(&mut self) -> bool {
        if let Some(bindings) = self.undo_stack.pop() {
            self.session.bindings = bindings;
            true
        } else {
            false
        }
    }

    /// Export session history records to JSON.
    pub fn export_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.cell_history)
    }

    /// Import session cells from JSON format and replay them.
    pub fn import_and_replay_json(&mut self, json_str: &str) -> Result<usize, String> {
        let cells: Vec<CellRecord> = serde_json::from_str(json_str)
            .map_err(|e| format!("Invalid JSON session structure: {e}"))?;
        let count = cells.len();
        self.reset();
        for cell in cells {
            self.execute_cell(&cell.input_code);
        }
        Ok(count)
    }
}

/// URAE Jupyter Kernel State.
#[derive(Debug, Default)]
pub struct UraeKernel {
    pub controller: NotebookSessionController,
}

impl UraeKernel {
    pub fn new() -> Self {
        Self {
            controller: NotebookSessionController::new(),
        }
    }

    pub fn execution_count(&self) -> usize {
        self.controller.execution_count
    }

    pub fn session(&self) -> &UraeSession {
        &self.controller.session
    }

    pub fn session_mut(&mut self) -> &mut UraeSession {
        &mut self.controller.session
    }

    /// Kernel Information Spec.
    pub fn kernel_info(&self) -> HashMap<String, String> {
        let mut info = HashMap::new();
        info.insert("protocol_version".to_string(), "5.3".to_string());
        info.insert("implementation".to_string(), "urae-kernel".to_string());
        info.insert("implementation_version".to_string(), "0.1.0".to_string());
        info.insert("language_info.name".to_string(), "urae".to_string());
        info.insert("language_info.version".to_string(), "0.1.0".to_string());
        info.insert(
            "language_info.mimetype".to_string(),
            "text/x-urae".to_string(),
        );
        info.insert(
            "language_info.file_extension".to_string(),
            ".urae".to_string(),
        );
        info
    }

    /// Execute a cell code string and return multi-MIME payload.
    pub fn execute(&mut self, code: &str) -> ExecuteReply {
        self.controller.execute_cell(code)
    }
}
