//! # `urae-kernel` (URAE In-Memory Jupyter Notebook Session & MIME Controller)
//!
//! In-memory Jupyter notebook cell/session & MIME-bundle execution controller for URAE
//! (ZeroMQ wire-protocol kernel scheduled for Wave 2).
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

    /// Dispatch a Jupyter Messaging Protocol v5.3 message and return the shell reply plus IOPub events.
    pub fn dispatch_jupyter(&mut self, msg: &JupyterMessage) -> JupyterDispatchOutput {
        self.controller.handle_jupyter_message(msg)
    }
}

/// Jupyter Messaging Protocol v5.3 message header.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JupyterHeader {
    pub msg_id: String,
    pub session: String,
    pub username: String,
    pub date: String,
    pub msg_type: String,
    pub version: String,
}

impl JupyterHeader {
    /// Construct a paired reply header for a parent request header.
    pub fn reply_to(parent: &Self, msg_type: impl Into<String>, msg_id: impl Into<String>) -> Self {
        Self {
            msg_id: msg_id.into(),
            session: parent.session.clone(),
            username: parent.username.clone(),
            date: parent.date.clone(),
            msg_type: msg_type.into(),
            version: "5.3".to_string(),
        }
    }
}

/// Jupyter Messaging Protocol v5.3 wire message envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JupyterMessage {
    pub header: JupyterHeader,
    pub parent_header: Option<JupyterHeader>,
    pub metadata: serde_json::Value,
    pub content: serde_json::Value,
}

impl JupyterMessage {
    /// Create a message referencing `parent` as its `parent_header`.
    pub fn with_parent(
        parent: &JupyterHeader,
        msg_type: impl Into<String>,
        msg_id: impl Into<String>,
        content: serde_json::Value,
    ) -> Self {
        Self {
            header: JupyterHeader::reply_to(parent, msg_type, msg_id),
            parent_header: Some(parent.clone()),
            metadata: serde_json::json!({}),
            content,
        }
    }
}

/// Combined shell-channel reply and ordered IOPub broadcasts produced for one wire request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JupyterDispatchOutput {
    pub shell_reply: JupyterMessage,
    pub iopub_messages: Vec<JupyterMessage>,
}

const BUILTIN_COMPLETIONS: &[&str] = &[
    "diff",
    "integrate",
    "simplify",
    "solve",
    "limit",
    "taylor",
    "laplace",
    "expand",
    "factor",
    "partial_fractions",
    "gear!",
    "quantum_bell",
    "sin",
    "cos",
    "tan",
    "exp",
    "ln",
    "sqrt",
];

fn delimiters_balanced(code: &str) -> bool {
    let mut stack = Vec::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for ch in code.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if (in_single || in_double) && ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '\'' && !in_double {
            in_single = !in_single;
            continue;
        }
        if ch == '"' && !in_single {
            in_double = !in_double;
            continue;
        }
        if in_single || in_double {
            continue;
        }
        match ch {
            '(' | '[' | '{' => stack.push(ch),
            ')' | ']' | '}' => {
                let expected = match ch {
                    ')' => '(',
                    ']' => '[',
                    _ => '{',
                };
                if stack.pop() != Some(expected) {
                    return false;
                }
            }
            _ => {}
        }
    }

    !in_single && !in_double && stack.is_empty()
}

impl NotebookSessionController {
    /// Handle an incoming Jupyter Messaging Protocol v5.3 request and emit
    /// both the shell reply and ordered IOPub status/display broadcasts.
    pub fn handle_jupyter_message(&mut self, msg: &JupyterMessage) -> JupyterDispatchOutput {
        let parent = &msg.header;
        let base_id = &parent.msg_id;
        let busy = JupyterMessage::with_parent(
            parent,
            "status",
            format!("{base_id}:busy"),
            serde_json::json!({ "execution_state": "busy" }),
        );
        let idle = JupyterMessage::with_parent(
            parent,
            "status",
            format!("{base_id}:idle"),
            serde_json::json!({ "execution_state": "idle" }),
        );

        match parent.msg_type.as_str() {
            "kernel_info_request" => {
                let shell_reply = JupyterMessage::with_parent(
                    parent,
                    "kernel_info_reply",
                    format!("{base_id}:reply"),
                    serde_json::json!({
                        "status": "ok",
                        "protocol_version": "5.3",
                        "implementation": "urae-kernel",
                        "implementation_version": env!("CARGO_PKG_VERSION"),
                        "language_info": {
                            "name": "urae",
                            "version": env!("CARGO_PKG_VERSION"),
                            "mimetype": "text/x-urae",
                            "file_extension": ".urae"
                        },
                        "banner": "URAE Native Symbolic & Numerical Jupyter Kernel"
                    }),
                );
                JupyterDispatchOutput {
                    shell_reply,
                    iopub_messages: vec![busy, idle],
                }
            }
            "execute_request" => {
                let code = msg
                    .content
                    .get("code")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let silent = msg
                    .content
                    .get("silent")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let reply = self.execute_cell(code);
                let mut iopub = vec![busy];

                if !silent {
                    iopub.push(JupyterMessage::with_parent(
                        parent,
                        "execute_input",
                        format!("{base_id}:input"),
                        serde_json::json!({
                            "code": code,
                            "execution_count": reply.execution_count,
                        }),
                    ));
                    if reply.status == "error" {
                        let evalue = reply
                            .mime_bundle
                            .data
                            .get("text/plain")
                            .cloned()
                            .unwrap_or_else(|| "Execution error".to_string());
                        iopub.push(JupyterMessage::with_parent(
                            parent,
                            "error",
                            format!("{base_id}:error"),
                            serde_json::json!({
                                "ename": "UraeExecutionError",
                                "evalue": evalue,
                                "traceback": [evalue],
                            }),
                        ));
                    } else {
                        iopub.push(JupyterMessage::with_parent(
                            parent,
                            "execute_result",
                            format!("{base_id}:result"),
                            serde_json::json!({
                                "execution_count": reply.execution_count,
                                "data": reply.mime_bundle.data,
                                "metadata": reply.mime_bundle.metadata,
                            }),
                        ));
                    }
                }
                iopub.push(idle);

                let shell_content = if reply.status == "error" {
                    let evalue = reply
                        .mime_bundle
                        .data
                        .get("text/plain")
                        .cloned()
                        .unwrap_or_else(|| "Execution error".to_string());
                    serde_json::json!({
                        "status": "error",
                        "execution_count": reply.execution_count,
                        "ename": "UraeExecutionError",
                        "evalue": evalue,
                        "traceback": [evalue],
                    })
                } else {
                    serde_json::json!({
                        "status": "ok",
                        "execution_count": reply.execution_count,
                        "user_expressions": reply.user_expressions,
                    })
                };

                let shell_reply = JupyterMessage::with_parent(
                    parent,
                    "execute_reply",
                    format!("{base_id}:reply"),
                    shell_content,
                );
                JupyterDispatchOutput {
                    shell_reply,
                    iopub_messages: iopub,
                }
            }
            "is_complete_request" => {
                let code = msg
                    .content
                    .get("code")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let status = if delimiters_balanced(code) {
                    "complete"
                } else {
                    "incomplete"
                };
                let shell_reply = JupyterMessage::with_parent(
                    parent,
                    "is_complete_reply",
                    format!("{base_id}:reply"),
                    serde_json::json!({
                        "status": status,
                        "indent": if status == "incomplete" { "    " } else { "" },
                    }),
                );
                JupyterDispatchOutput {
                    shell_reply,
                    iopub_messages: vec![busy, idle],
                }
            }
            "complete_request" => {
                let code = msg
                    .content
                    .get("code")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let cursor_pos = msg
                    .content
                    .get("cursor_pos")
                    .and_then(|v| v.as_u64())
                    .map(|v| (v as usize).min(code.len()))
                    .unwrap_or(code.len());

                let prefix_slice = code.get(..cursor_pos).unwrap_or("");
                let token_start = prefix_slice
                    .rfind(|c: char| !c.is_alphanumeric() && c != '_' && c != '!')
                    .map_or(0, |idx| idx + 1);
                let prefix = &prefix_slice[token_start..];

                let mut matches: Vec<String> = self
                    .session
                    .bindings
                    .keys()
                    .filter(|name| name.starts_with(prefix))
                    .cloned()
                    .collect();
                for builtin in BUILTIN_COMPLETIONS {
                    if builtin.starts_with(prefix) && !matches.iter().any(|m| m == builtin) {
                        matches.push((*builtin).to_string());
                    }
                }
                matches.sort();

                let shell_reply = JupyterMessage::with_parent(
                    parent,
                    "complete_reply",
                    format!("{base_id}:reply"),
                    serde_json::json!({
                        "status": "ok",
                        "matches": matches,
                        "cursor_start": token_start,
                        "cursor_end": cursor_pos,
                        "metadata": {},
                    }),
                );
                JupyterDispatchOutput {
                    shell_reply,
                    iopub_messages: vec![busy, idle],
                }
            }
            "history_request" => {
                let n = msg
                    .content
                    .get("n")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);
                let slice: &[CellRecord] = match n {
                    Some(limit) if limit < self.cell_history.len() => {
                        &self.cell_history[self.cell_history.len() - limit..]
                    }
                    _ => &self.cell_history,
                };
                let history: Vec<(usize, usize, String)> = slice
                    .iter()
                    .map(|cell| (0, cell.id, cell.input_code.clone()))
                    .collect();

                let shell_reply = JupyterMessage::with_parent(
                    parent,
                    "history_reply",
                    format!("{base_id}:reply"),
                    serde_json::json!({
                        "status": "ok",
                        "history": history,
                    }),
                );
                JupyterDispatchOutput {
                    shell_reply,
                    iopub_messages: vec![busy, idle],
                }
            }
            "shutdown_request" => {
                let restart = msg
                    .content
                    .get("restart")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if restart {
                    self.reset();
                }
                let shell_reply = JupyterMessage::with_parent(
                    parent,
                    "shutdown_reply",
                    format!("{base_id}:reply"),
                    serde_json::json!({
                        "status": "ok",
                        "restart": restart,
                    }),
                );
                JupyterDispatchOutput {
                    shell_reply,
                    iopub_messages: vec![busy, idle],
                }
            }
            other => {
                let evalue = format!("Unsupported Jupyter message type: {other}");
                let shell_reply = JupyterMessage::with_parent(
                    parent,
                    format!("{}_reply", other.trim_end_matches("_request")),
                    format!("{base_id}:reply"),
                    serde_json::json!({
                        "status": "error",
                        "ename": "UnsupportedMessageType",
                        "evalue": evalue,
                        "traceback": [evalue],
                    }),
                );
                JupyterDispatchOutput {
                    shell_reply,
                    iopub_messages: vec![busy, idle],
                }
            }
        }
    }

    /// Generate a standard Jupyter `kernel.json` kernelspec for `urae-kernel`.
    pub fn kernelspec_json(binary_path: &str) -> String {
        serde_json::json!({
            "argv": [binary_path, "-f", "{connection_file}"],
            "display_name": "URAE (Rust CAS & Multiphysics)",
            "language": "urae",
            "metadata": {
                "debugger": false
            }
        })
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(msg_id: &str, msg_type: &str, content: serde_json::Value) -> JupyterMessage {
        JupyterMessage {
            header: JupyterHeader {
                msg_id: msg_id.to_string(),
                session: "test-session".to_string(),
                username: "tester".to_string(),
                date: "2026-11-14T00:00:00Z".to_string(),
                msg_type: msg_type.to_string(),
                version: "5.3".to_string(),
            },
            parent_header: None,
            metadata: serde_json::json!({}),
            content,
        }
    }

    #[test]
    fn test_jupyter_kernel_info_and_kernelspec() {
        let mut controller = NotebookSessionController::new();
        let out =
            controller.handle_jupyter_message(&req("m1", "kernel_info_request", serde_json::json!({})));
        assert_eq!(out.shell_reply.header.msg_type, "kernel_info_reply");
        assert_eq!(out.shell_reply.content["status"], "ok");
        assert_eq!(out.shell_reply.content["protocol_version"], "5.3");
        assert_eq!(out.shell_reply.content["language_info"]["name"], "urae");
        assert_eq!(out.iopub_messages.len(), 2);
        assert_eq!(out.iopub_messages[0].content["execution_state"], "busy");
        assert_eq!(out.iopub_messages[1].content["execution_state"], "idle");

        let spec: serde_json::Value =
            serde_json::from_str(&NotebookSessionController::kernelspec_json("/usr/bin/urae-kernel"))
                .expect("valid kernelspec json");
        assert_eq!(spec["language"], "urae");
        assert_eq!(spec["argv"][0], "/usr/bin/urae-kernel");
        assert_eq!(spec["argv"][2], "{connection_file}");
    }

    #[test]
    fn test_jupyter_execute_math_cad_and_error() {
        let mut controller = NotebookSessionController::new();

        let math = controller.handle_jupyter_message(&req(
            "e1",
            "execute_request",
            serde_json::json!({ "code": "2 + 3", "silent": false }),
        ));
        assert_eq!(math.shell_reply.header.msg_type, "execute_reply");
        assert_eq!(math.shell_reply.content["status"], "ok");
        assert_eq!(math.shell_reply.content["execution_count"], 1);
        assert_eq!(math.iopub_messages[1].header.msg_type, "execute_input");
        assert_eq!(math.iopub_messages[2].header.msg_type, "execute_result");
        assert!(math.iopub_messages[2].content["data"]["text/plain"].is_string());

        let cad = controller.handle_jupyter_message(&req(
            "e2",
            "execute_request",
            serde_json::json!({ "code": "gear!", "silent": false }),
        ));
        assert_eq!(cad.shell_reply.content["status"], "ok");
        let html = cad.iopub_messages[2].content["data"]["text/html"]
            .as_str()
            .expect("cad execute_result includes text/html");
        assert!(html.contains("<html") || html.contains("Involute Gear"));

        let err = controller.handle_jupyter_message(&req(
            "e3",
            "execute_request",
            serde_json::json!({ "code": "1 + )", "silent": false }),
        ));
        assert_eq!(err.shell_reply.content["status"], "error");
        assert_eq!(err.iopub_messages[2].header.msg_type, "error");
    }

    #[test]
    fn test_jupyter_completion_history_and_shutdown() {
        let mut controller = NotebookSessionController::new();
        controller.set_parameter("alpha_gain", 2.5);
        controller.execute_cell("1 + 1");

        let complete_ok = controller.handle_jupyter_message(&req(
            "c1",
            "is_complete_request",
            serde_json::json!({ "code": "sin(x)" }),
        ));
        assert_eq!(complete_ok.shell_reply.content["status"], "complete");

        let incomplete = controller.handle_jupyter_message(&req(
            "c2",
            "is_complete_request",
            serde_json::json!({ "code": "sin((x + 1)" }),
        ));
        assert_eq!(incomplete.shell_reply.content["status"], "incomplete");

        let comp = controller.handle_jupyter_message(&req(
            "c3",
            "complete_request",
            serde_json::json!({ "code": "al", "cursor_pos": 2 }),
        ));
        let matches = comp.shell_reply.content["matches"]
            .as_array()
            .expect("matches array");
        assert!(matches.iter().any(|m| m == "alpha_gain"));

        let diff_comp = controller.handle_jupyter_message(&req(
            "c4",
            "complete_request",
            serde_json::json!({ "code": "dif", "cursor_pos": 3 }),
        ));
        assert!(
            diff_comp.shell_reply.content["matches"]
                .as_array()
                .expect("matches")
                .iter()
                .any(|m| m == "diff")
        );

        let hist = controller.handle_jupyter_message(&req(
            "h1",
            "history_request",
            serde_json::json!({ "n": 10 }),
        ));
        assert_eq!(hist.shell_reply.content["history"][0][2], "1 + 1");

        let shut = controller.handle_jupyter_message(&req(
            "s1",
            "shutdown_request",
            serde_json::json!({ "restart": true }),
        ));
        assert_eq!(shut.shell_reply.content["restart"], true);
        assert_eq!(controller.execution_count, 0);
    }
}
