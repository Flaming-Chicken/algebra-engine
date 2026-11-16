//! # `algebra_engine::stream`
//!
//! Runtime-Agnostic Async Execution, Data-Stream Ingestion, and File Batch Processing.
//!
//! Provides composable execution pathways spanning immutable parallel stateless batches, mutable
//! stateful session streams, NDJSON request pipelines, CSV tabular expression evaluation, and
//! runtime-agnostic [`std::future::Future`] tasks:
//! - [`ComputeFuture<T>`]: Safe `std::future::Future` backed by worker threads on native targets
//!   and immediate evaluation on WebAssembly, compatible with any async executor (`block_on`, `tokio`,
//!   `futures`, etc.) with zero required async runtime dependency.
//! - [`StreamProcessor`]: Streaming `BufRead` / `Write` processors for script files, NDJSON request
//!   streams, and CSV/TSV datasets (accelerated via [`BytecodeVM`] compilation).
//! - Mutable vs. Immutable Concurrency:
//!   - Immutable &[`ExprGraph`] supports lock-free multi-threaded batch execution (`execute_batch`).
//!   - Mutable &mut [`UraeSession`] preserves sequential bindings, functions, `ans`, and assumptions.

use crate::executor::{ExecutionContext, OperationExecutor, OperationResult};
use crate::query::{QueryEngine, QueryResponse, ResultPod};
use crate::session::UraeSession;
use algebra_core::parser::{ExprParser, parse_operation};
use algebra_core::vm::{BytecodeVM, VmCompiler};
use algebra_core::ExprGraph;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::io::{self, BufRead, Write};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

// =========================================================================
// Runtime-Agnostic `Future` & Synchronous Executor Helper
// =========================================================================

struct FutureShared<T> {
    result: Option<T>,
    waker: Option<Waker>,
}

/// Runtime-agnostic future representing a background symbolic or numerical computation.
pub struct ComputeFuture<T> {
    shared: Arc<parking_lot::Mutex<FutureShared<T>>>,
}

impl<T: Send + 'static> ComputeFuture<T> {
    /// Wrap an already completed value.
    pub fn ready(val: T) -> Self {
        Self {
            shared: Arc::new(parking_lot::Mutex::new(FutureShared {
                result: Some(val),
                waker: None,
            })),
        }
    }

    /// Execute closure asynchronously on a background thread (native) or inline (wasm32),
    /// waking any registered executor [`Waker`] on completion.
    pub fn spawn<F>(work: F) -> Self
    where
        F: FnOnce() -> T + Send + 'static,
    {
        let shared = Arc::new(parking_lot::Mutex::new(FutureShared {
            result: None,
            waker: None,
        }));
        #[cfg(not(target_arch = "wasm32"))]
        {
            let shared_bg = Arc::clone(&shared);
            std::thread::spawn(move || {
                let out = work();
                let waker = {
                    let mut guard = shared_bg.lock();
                    guard.result = Some(out);
                    guard.waker.take()
                };
                if let Some(w) = waker {
                    w.wake();
                }
            });
        }
        #[cfg(target_arch = "wasm32")]
        {
            let out = work();
            shared.lock().result = Some(out);
        }
        Self { shared }
    }

    /// Drive this future to completion using a safe `std::task::Wake` park/unpark loop.
    pub fn block_on(self) -> T {
        struct ThreadWake(std::thread::Thread);
        impl std::task::Wake for ThreadWake {
            fn wake(self: Arc<Self>) {
                self.0.unpark();
            }
        }
        let waker = Waker::from(Arc::new(ThreadWake(std::thread::current())));
        let mut cx = Context::from_waker(&waker);
        let mut pinned = Box::pin(self);
        loop {
            match pinned.as_mut().poll(&mut cx) {
                Poll::Ready(val) => return val,
                Poll::Pending => std::thread::park(),
            }
        }
    }
}

impl<T> Future for ComputeFuture<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut guard = self.shared.lock();
        if let Some(val) = guard.result.take() {
            Poll::Ready(val)
        } else {
            guard.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

/// Evaluate a structured multi-pod query asynchronously against a shared [`ExprGraph`].
pub fn query_async(graph: &ExprGraph, query: &str, ctx: &ExecutionContext) -> ComputeFuture<QueryResponse> {
    let g = graph.clone();
    let q = query.to_string();
    let c = ctx.clone();
    ComputeFuture::spawn(move || QueryEngine::query(&g, &q, &c))
}

/// Execute an operation command string asynchronously against a shared [`ExprGraph`].
pub fn execute_async(
    graph: &ExprGraph,
    command: &str,
    ctx: &ExecutionContext,
) -> ComputeFuture<OperationResult> {
    let g = graph.clone();
    let cmd = command.to_string();
    let c = ctx.clone();
    ComputeFuture::spawn(move || {
        let op = parse_operation(&cmd);
        OperationExecutor::execute(&g, &op, &c)
    })
}

/// Process a batch of independent command strings asynchronously in parallel using immutable `&ExprGraph`.
pub fn execute_batch_async(
    graph: &ExprGraph,
    commands: Vec<String>,
    ctx: &ExecutionContext,
) -> ComputeFuture<Vec<OperationResult>> {
    let g = graph.clone();
    let c = ctx.clone();
    ComputeFuture::spawn(move || StreamProcessor::execute_batch(&g, &commands, &c))
}

// =========================================================================
// Streaming Request / Response Payloads & Processors
// =========================================================================

/// Execution mode selector for streaming JSON requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamMode {
    #[default]
    Execute,
    Query,
}

/// Single JSON request record in an NDJSON stream or batch file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamRequest {
    #[serde(default)]
    pub id: Option<String>,
    pub input: String,
    #[serde(default)]
    pub bindings: HashMap<String, f64>,
    #[serde(default)]
    pub uncertainties: HashMap<String, f64>,
    #[serde(default)]
    pub mode: StreamMode,
}

/// Single JSON response record emitted to an output stream or file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamResponse {
    #[serde(default)]
    pub id: Option<String>,
    pub success: bool,
    pub output_text: String,
    pub output_latex: String,
    pub output_unicode: String,
    #[serde(default)]
    pub error_msg: Option<String>,
    #[serde(default)]
    pub pods: Option<Vec<ResultPod>>,
}

/// Summary statistics returned after streaming script, NDJSON, or CSV processing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BatchSummary {
    pub processed: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub skipped: usize,
}

/// Data-stream and file batch processor supporting immutable parallel & mutable stateful pipelines.
pub struct StreamProcessor;

impl StreamProcessor {
    /// Execute a single [`StreamRequest`] against an immutable shared [`ExprGraph`].
    pub fn handle_request(graph: &ExprGraph, req: &StreamRequest) -> StreamResponse {
        let mut ctx = ExecutionContext::with_bindings(req.bindings.clone());
        ctx.uncertainties = req.uncertainties.clone();
        match req.mode {
            StreamMode::Execute => {
                let op = parse_operation(&req.input);
                let res = OperationExecutor::execute(graph, &op, &ctx);
                StreamResponse {
                    id: req.id.clone(),
                    success: !res.is_error,
                    output_text: res.output_text,
                    output_latex: res.output_latex,
                    output_unicode: res.output_unicode,
                    error_msg: res.error_msg,
                    pods: None,
                }
            }
            StreamMode::Query => {
                let qres = QueryEngine::query(graph, &req.input, &ctx);
                let res_pod = qres
                    .pods
                    .iter()
                    .find(|p| p.title == "Result" || p.title == "Error");
                let is_err = qres.pods.iter().any(|p| p.title == "Error");
                let text = res_pod.map(|p| p.text.clone()).unwrap_or_default();
                let latex = res_pod.map(|p| p.latex.clone()).unwrap_or_default();
                let unicode = res_pod.map(|p| p.unicode.clone()).unwrap_or_default();
                StreamResponse {
                    id: req.id.clone(),
                    success: !is_err,
                    output_text: text.clone(),
                    output_latex: latex,
                    output_unicode: unicode,
                    error_msg: if is_err { Some(text) } else { None },
                    pods: Some(qres.pods),
                }
            }
        }
    }

    /// Execute independent commands using immutable &[`ExprGraph`] (parallel on native targets).
    pub fn execute_batch(
        graph: &ExprGraph,
        commands: &[String],
        ctx: &ExecutionContext,
    ) -> Vec<OperationResult> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            commands
                .par_iter()
                .map(|cmd| {
                    let op = parse_operation(cmd);
                    OperationExecutor::execute(graph, &op, ctx)
                })
                .collect()
        }
        #[cfg(target_arch = "wasm32")]
        {
            commands
                .iter()
                .map(|cmd| {
                    let op = parse_operation(cmd);
                    OperationExecutor::execute(graph, &op, ctx)
                })
                .collect()
        }
    }

    /// Process a stateful multi-line URAE script stream via [`BufRead`] -> [`Write`],
    /// mutating `&mut UraeSession` sequentially and skipping blank or `#` / `//` comment lines.
    pub fn process_script_stream<R: BufRead, W: Write>(
        session: &mut UraeSession,
        reader: R,
        mut writer: W,
    ) -> io::Result<BatchSummary> {
        let mut summary = BatchSummary::default();
        for line_res in reader.lines() {
            let line = line_res?;
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
                summary.skipped += 1;
                continue;
            }
            summary.processed += 1;
            let res = session.execute_line(trimmed);
            if res.is_error {
                summary.failed += 1;
                writeln!(writer, "ERR | {} => {}", trimmed, res.error_msg.unwrap_or(res.output_text))?;
            } else {
                summary.succeeded += 1;
                writeln!(writer, "OK  | {} => {}", trimmed, res.output_text)?;
            }
        }
        Ok(summary)
    }

    /// Process a streaming NDJSON request feed (`StreamRequest` per line) -> NDJSON `StreamResponse`.
    pub fn process_ndjson_stream<R: BufRead, W: Write>(
        graph: &ExprGraph,
        reader: R,
        mut writer: W,
    ) -> io::Result<BatchSummary> {
        let mut summary = BatchSummary::default();
        for line_res in reader.lines() {
            let line = line_res?;
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
                summary.skipped += 1;
                continue;
            }
            summary.processed += 1;
            let resp = match serde_json::from_str::<StreamRequest>(trimmed) {
                Ok(req) => {
                    let r = Self::handle_request(graph, &req);
                    if r.success {
                        summary.succeeded += 1;
                    } else {
                        summary.failed += 1;
                    }
                    r
                }
                Err(e) => {
                    summary.failed += 1;
                    StreamResponse {
                        id: None,
                        success: false,
                        output_text: String::new(),
                        output_latex: String::new(),
                        output_unicode: String::new(),
                        error_msg: Some(format!("Malformed JSON request: {e}")),
                        pods: None,
                    }
                }
            };
            let json_line = serde_json::to_string(&resp)
                .map_err(|e| io::Error::other(format!("JSON serialization failed: {e}")))?;
            writeln!(writer, "{json_line}")?;
        }
        Ok(summary)
    }

    /// Stream a CSV dataset through [`BytecodeVM`], evaluating `expr_str` for every row using header
    /// column names as expression variables and appending `output_col`.
    pub fn eval_csv_stream<R: BufRead, W: Write>(
        graph: &ExprGraph,
        expr_str: &str,
        output_col: &str,
        reader: R,
        mut writer: W,
    ) -> io::Result<BatchSummary> {
        let mut lines = reader.lines();
        let Some(header_line_res) = lines.next() else {
            return Ok(BatchSummary::default());
        };
        let header_line = header_line_res?;
        let headers: Vec<String> = header_line
            .split(',')
            .map(|h| h.trim().trim_matches('"').to_string())
            .collect();
        if headers.is_empty() || headers.iter().all(|h| h.is_empty()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CSV header row is empty",
            ));
        }

        let expr_id = ExprParser::new(graph)
            .parse(expr_str)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
        let primary_var = &headers[0];
        let param_vars: Vec<String> = headers[1..].to_vec();
        let program = VmCompiler::compile(graph, expr_id, primary_var, &param_vars)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
        let vm = BytecodeVM::new();

        writeln!(writer, "{},{}", header_line.trim_end(), output_col)?;

        let mut summary = BatchSummary::default();
        for row_res in lines {
            let row = row_res?;
            let trimmed = row.trim();
            if trimmed.is_empty() {
                summary.skipped += 1;
                continue;
            }
            summary.processed += 1;
            let fields: Vec<&str> = trimmed.split(',').map(|s| s.trim().trim_matches('"')).collect();
            let mut vals = Vec::with_capacity(headers.len());
            let mut parse_ok = fields.len() >= headers.len();
            if parse_ok {
                for cell in fields.iter().take(headers.len()) {
                    match cell.parse::<f64>() {
                        Ok(v) => vals.push(v),
                        Err(_) => {
                            parse_ok = false;
                            break;
                        }
                    }
                }
            }
            if !parse_ok {
                summary.failed += 1;
                writeln!(writer, "{},NaN", trimmed)?;
                continue;
            }
            match vm.eval_scalar(&program, vals[0], &vals[1..]) {
                Ok(out_val) => {
                    summary.succeeded += 1;
                    writeln!(writer, "{},{}", trimmed, out_val)?;
                }
                Err(_) => {
                    summary.failed += 1;
                    writeln!(writer, "{},NaN", trimmed)?;
                }
            }
        }
        Ok(summary)
    }

    /// Execute a script file sequentially into a mutable [`UraeSession`], writing results to `out_path`.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn process_script_file(
        session: &mut UraeSession,
        in_path: impl AsRef<Path>,
        out_path: impl AsRef<Path>,
    ) -> io::Result<BatchSummary> {
        let reader = io::BufReader::new(std::fs::File::open(in_path)?);
        let writer = io::BufWriter::new(std::fs::File::create(out_path)?);
        Self::process_script_stream(session, reader, writer)
    }

    /// Process an NDJSON file of [`StreamRequest`] records and write [`StreamResponse`] lines.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn process_ndjson_file(
        graph: &ExprGraph,
        in_path: impl AsRef<Path>,
        out_path: impl AsRef<Path>,
    ) -> io::Result<BatchSummary> {
        let reader = io::BufReader::new(std::fs::File::open(in_path)?);
        let writer = io::BufWriter::new(std::fs::File::create(out_path)?);
        Self::process_ndjson_stream(graph, reader, writer)
    }

    /// Evaluate a symbolic formula across a CSV file and write augmented CSV output.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn eval_csv_file(
        graph: &ExprGraph,
        expr_str: &str,
        output_col: &str,
        in_path: impl AsRef<Path>,
        out_path: impl AsRef<Path>,
    ) -> io::Result<BatchSummary> {
        let reader = io::BufReader::new(std::fs::File::open(in_path)?);
        let writer = io::BufWriter::new(std::fs::File::create(out_path)?);
        Self::eval_csv_stream(graph, expr_str, output_col, reader, writer)
    }
}
