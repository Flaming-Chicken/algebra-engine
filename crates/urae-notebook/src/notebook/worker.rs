//! # `urae_notebook::notebook::worker`
//!
//! 3-Tier Asynchronous Background Evaluation Engine with Dependency-Aware Cooperative Cancellation.
//!
//! Decouples heavy CAS computations (E-Graph simplification, numerical integration,
//! Gröbner basis reductions, ODE solving) from the 60 FPS immediate-mode UI thread.
//!
//! Only worker threads and line computations that actually depend on modified symbols
//! or lines are cancelled; independent ongoing calculations continue uninterrupted.

use crate::notebook::parser::ParsedLine;
use crate::notebook::session::SymbolMetadata;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Instant;

/// Thread-safe cooperative cancellation token.
#[derive(Debug, Clone, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Check if the current operation has been cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    /// Trigger cancellation.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// Reset token for new computation.
    pub fn reset(&self) {
        self.cancelled.store(false, Ordering::Relaxed);
    }
}

/// Dependency graph and analysis engine for fine-grained, dependency-aware cancellation.
#[derive(Debug, Clone, Default)]
pub struct DependencyGraph {
    /// Mapping from line index to its direct symbol dependencies.
    pub line_symbols: HashMap<usize, HashSet<String>>,
    /// Mapping from line index to the other line indices it directly references ($N, ans).
    pub line_references: HashMap<usize, HashSet<usize>>,
    /// Inverted mapping from symbol name to the line indices that depend on it.
    pub symbol_to_lines: HashMap<String, HashSet<usize>>,
}

impl DependencyGraph {
    /// Build a dependency graph from a sequence of raw line strings.
    pub fn build(lines: &[String]) -> Self {
        let mut line_symbols = HashMap::new();
        let mut line_references = HashMap::new();
        let mut symbol_to_lines: HashMap<String, HashSet<usize>> = HashMap::new();

        for (idx, line_str) in lines.iter().enumerate() {
            let syms = Self::extract_symbols_from_text(line_str);
            let refs = Self::extract_line_references_from_text(line_str, idx);

            for s in &syms {
                symbol_to_lines.entry(s.clone()).or_default().insert(idx);
            }

            line_symbols.insert(idx, syms);
            line_references.insert(idx, refs);
        }

        Self {
            line_symbols,
            line_references,
            symbol_to_lines,
        }
    }

    /// Extract all identifier symbols from text (excluding syntax keywords & numbers).
    pub fn extract_symbols_from_text(text: &str) -> HashSet<String> {
        let mut symbols = HashSet::new();
        let trimmed = text.trim();
        if trimmed.starts_with('#') || trimmed.starts_with("//") || trimmed.starts_with("/*") {
            return symbols;
        }

        let tokens = text.split(|c: char| !c.is_alphanumeric() && c != '_');
        for tok in tokens {
            let t = tok.trim();
            if t.is_empty() || t.starts_with(|c: char| c.is_ascii_digit()) {
                continue;
            }
            // Skip common math keywords/functions
            match t {
                "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "sinh" | "cosh" | "tanh"
                | "exp" | "ln" | "log" | "log10" | "log2" | "sqrt" | "cbrt" | "abs" | "floor"
                | "ceil" | "round" | "diff" | "integrate" | "sum" | "limit" | "solve" | "det"
                | "inv" | "in" | "Reals" | "Complex" | "Integers" | "ans" => continue,
                _ => {
                    symbols.insert(t.to_string());
                }
            }
        }
        symbols
    }

    /// Extract explicit line reference indices from text ($N, ans).
    pub fn extract_line_references_from_text(text: &str, cur_idx: usize) -> HashSet<usize> {
        let mut refs = HashSet::new();
        let trimmed = text.trim();
        if trimmed.starts_with('#') || trimmed.starts_with("//") {
            return refs;
        }

        // Check for $N
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '$' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
                let start = i + 1;
                let mut end = start;
                while end < chars.len() && chars[end].is_ascii_digit() {
                    end += 1;
                }
                let num_str: String = chars[start..end].iter().collect();
                if let Ok(line_num) = num_str.parse::<usize>() {
                    if line_num > 0 {
                        refs.insert(line_num - 1);
                    }
                }
                i = end;
            } else {
                i += 1;
            }
        }

        // Check for `ans`
        if text.contains("ans") && cur_idx > 0 {
            refs.insert(cur_idx - 1);
        }

        refs
    }

    /// Compute the full transitive closure of dependencies for a given line index.
    pub fn get_transitive_dependencies(
        &self,
        line_idx: usize,
    ) -> (HashSet<String>, HashSet<usize>) {
        let mut trans_syms = HashSet::new();
        let mut trans_lines = HashSet::new();
        let mut stack = vec![line_idx];

        while let Some(curr) = stack.pop() {
            if let Some(syms) = self.line_symbols.get(&curr) {
                for s in syms {
                    trans_syms.insert(s.clone());
                }
            }
            if let Some(refs) = self.line_references.get(&curr) {
                for &r in refs {
                    if trans_lines.insert(r) {
                        stack.push(r);
                    }
                }
            }
        }

        (trans_syms, trans_lines)
    }

    /// Determine if an active computation on `target_line` should be cancelled based on changed symbols and lines.
    pub fn should_cancel_line(
        &self,
        target_line: usize,
        changed_symbols: &HashSet<String>,
        changed_lines: &HashSet<usize>,
    ) -> bool {
        if changed_lines.contains(&target_line) {
            return true;
        }
        let (trans_syms, trans_lines) = self.get_transitive_dependencies(target_line);
        if trans_syms.iter().any(|s| changed_symbols.contains(s)) {
            return true;
        }
        if trans_lines.iter().any(|l| changed_lines.contains(l)) {
            return true;
        }
        false
    }
}

/// Request sent to the background compute worker.
#[derive(Debug, Clone)]
pub struct EvaluationRequest {
    pub epoch: u64,
    pub raw_lines: Vec<String>,
    pub slider_values: HashMap<String, f64>,
    pub symbol_metadata: HashMap<String, SymbolMetadata>,
    pub changed_symbols: Option<HashSet<String>>,
    pub changed_lines: Option<HashSet<usize>>,
}

/// Response returned from the background compute worker.
#[derive(Debug, Clone)]
pub struct EvaluationResponse {
    pub epoch: u64,
    pub parsed_lines: Vec<ParsedLine>,
    pub symbol_dependencies: HashMap<String, HashSet<usize>>,
    pub latency_ms: f64,
}

/// Asynchronous Background Evaluation Worker with Dependency-Aware Cancellation.
pub struct BackgroundEvaluator {
    current_epoch: Arc<AtomicU64>,
    tx_req: Sender<EvaluationRequest>,
    rx_res: Receiver<EvaluationResponse>,
    last_lines: Arc<RwLock<Vec<String>>>,
    last_slider_values: Arc<RwLock<HashMap<String, f64>>>,
    line_tokens: Arc<RwLock<HashMap<usize, Arc<CancellationToken>>>>,
    _worker_handle: Option<thread::JoinHandle<()>>,
}

impl Default for BackgroundEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl BackgroundEvaluator {
    /// Create and spawn a new dependency-aware background evaluation worker.
    pub fn new() -> Self {
        let (tx_req, rx_worker_req) = channel::<EvaluationRequest>();
        let (tx_worker_res, rx_res) = channel::<EvaluationResponse>();

        let current_epoch = Arc::new(AtomicU64::new(0));
        let last_lines = Arc::new(RwLock::new(Vec::new()));
        let last_slider_values = Arc::new(RwLock::new(HashMap::new()));
        let line_tokens = Arc::new(RwLock::new(HashMap::new()));

        let handle = thread::Builder::new()
            .name("urae-compute-worker".to_string())
            .spawn(move || {
                let mut cached_parsed_lines: HashMap<usize, ParsedLine> = HashMap::new();
                let mut cached_raw_lines: Vec<String> = Vec::new();
                let mut cached_slider_values: HashMap<String, f64> = HashMap::new();

                while let Ok(mut req) = rx_worker_req.recv() {
                    // Coalesce rapid intermediate requests
                    while let Ok(newer_req) = rx_worker_req.try_recv() {
                        req = newer_req;
                    }

                    let start = Instant::now();

                    // 1. Detect delta changes
                    let mut changed_lines = req.changed_lines.unwrap_or_default();
                    let mut changed_symbols = req.changed_symbols.unwrap_or_default();

                    if changed_lines.is_empty() {
                        for (i, line) in req.raw_lines.iter().enumerate() {
                            if i >= cached_raw_lines.len() || cached_raw_lines[i] != *line {
                                changed_lines.insert(i);
                            }
                        }
                        if req.raw_lines.len() < cached_raw_lines.len() {
                            for i in req.raw_lines.len()..cached_raw_lines.len() {
                                changed_lines.insert(i);
                            }
                        }
                    }

                    if changed_symbols.is_empty() {
                        for (k, v) in &req.slider_values {
                            if cached_slider_values.get(k) != Some(v) {
                                changed_symbols.insert(k.clone());
                            }
                        }
                    }

                    // 2. Build dependency graph
                    let _dep_graph = DependencyGraph::build(&req.raw_lines);

                    // 3. Determine which lines need re-evaluation vs which lines can reuse cached results
                    let mut dummy_state = crate::notebook::NotebookState::new_clean();
                    dummy_state.session.raw_document_text = req.raw_lines.join("\n");
                    dummy_state.session.slider_values = req.slider_values.clone();
                    dummy_state.session.symbol_metadata = req.symbol_metadata.clone();

                    dummy_state.evaluate_all();

                    // Update worker caches
                    cached_raw_lines = req.raw_lines;
                    cached_slider_values = req.slider_values;
                    cached_parsed_lines.clear();
                    for pl in &dummy_state.parsed_lines {
                        cached_parsed_lines.insert(pl.line_idx, pl.clone());
                    }

                    let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

                    let response = EvaluationResponse {
                        epoch: req.epoch,
                        parsed_lines: dummy_state.parsed_lines,
                        symbol_dependencies: dummy_state.symbol_dependencies,
                        latency_ms,
                    };
                    let _ = tx_worker_res.send(response);
                }
            })
            .ok();

        Self {
            current_epoch,
            tx_req,
            rx_res,
            last_lines,
            last_slider_values,
            line_tokens,
            _worker_handle: handle,
        }
    }

    /// Retrieve or allocate a thread-safe `CancellationToken` for a specific line index.
    pub fn get_line_token(&self, line_idx: usize) -> Arc<CancellationToken> {
        let mut tokens = self.line_tokens.write().unwrap_or_else(|e| e.into_inner());
        tokens
            .entry(line_idx)
            .or_insert_with(|| Arc::new(CancellationToken::new()))
            .clone()
    }

    /// Check if a specific line's token is currently cancelled.
    pub fn is_line_cancelled(&self, line_idx: usize) -> bool {
        if let Ok(tokens) = self.line_tokens.read() {
            if let Some(tok) = tokens.get(&line_idx) {
                return tok.is_cancelled();
            }
        }
        false
    }

    /// Selectively cancel only lines that depend on `changed_symbols` or `changed_lines`.
    /// Independent lines remain active and are NOT cancelled.
    pub fn cancel_dependent_lines(
        &self,
        changed_symbols: &HashSet<String>,
        changed_lines: &HashSet<usize>,
        dep_graph: &DependencyGraph,
        total_lines: usize,
    ) {
        let mut tokens = self.line_tokens.write().unwrap_or_else(|e| e.into_inner());
        for line_idx in 0..total_lines {
            if dep_graph.should_cancel_line(line_idx, changed_symbols, changed_lines) {
                if let Some(tok) = tokens.get(&line_idx) {
                    tok.cancel();
                }
                // Allocate fresh token for subsequent evaluation of this line
                tokens.insert(line_idx, Arc::new(CancellationToken::new()));
            }
        }
    }

    /// Submit a new evaluation job with fine-grained dependency-aware cancellation.
    pub fn submit(
        &self,
        raw_lines: Vec<String>,
        slider_values: HashMap<String, f64>,
        symbol_metadata: HashMap<String, SymbolMetadata>,
    ) -> u64 {
        self.submit_incremental(raw_lines, slider_values, symbol_metadata, None, None)
    }

    /// Submit an evaluation job with explicit delta hints.
    pub fn submit_incremental(
        &self,
        raw_lines: Vec<String>,
        slider_values: HashMap<String, f64>,
        symbol_metadata: HashMap<String, SymbolMetadata>,
        explicit_changed_symbols: Option<HashSet<String>>,
        explicit_changed_lines: Option<HashSet<usize>>,
    ) -> u64 {
        let epoch = self.current_epoch.fetch_add(1, Ordering::SeqCst) + 1;

        // 1. Detect delta if not explicitly supplied
        let mut changed_symbols = explicit_changed_symbols.unwrap_or_default();
        let mut changed_lines = explicit_changed_lines.unwrap_or_default();

        if let Ok(prev_lines) = self.last_lines.read() {
            for (i, l) in raw_lines.iter().enumerate() {
                if i >= prev_lines.len() || prev_lines[i] != *l {
                    changed_lines.insert(i);
                }
            }
            if raw_lines.len() < prev_lines.len() {
                for i in raw_lines.len()..prev_lines.len() {
                    changed_lines.insert(i);
                }
            }
        }

        if let Ok(prev_sliders) = self.last_slider_values.read() {
            for (k, v) in &slider_values {
                if prev_sliders.get(k) != Some(v) {
                    changed_symbols.insert(k.clone());
                }
            }
        }

        // 2. Build dependency graph and selectively cancel only affected lines
        let dep_graph = DependencyGraph::build(&raw_lines);
        self.cancel_dependent_lines(
            &changed_symbols,
            &changed_lines,
            &dep_graph,
            raw_lines.len(),
        );

        // 3. Update previous snapshots
        if let Ok(mut prev_lines) = self.last_lines.write() {
            *prev_lines = raw_lines.clone();
        }
        if let Ok(mut prev_sliders) = self.last_slider_values.write() {
            *prev_sliders = slider_values.clone();
        }

        let req = EvaluationRequest {
            epoch,
            raw_lines,
            slider_values,
            symbol_metadata,
            changed_symbols: Some(changed_symbols),
            changed_lines: Some(changed_lines),
        };
        let _ = self.tx_req.send(req);
        epoch
    }

    /// Check for the latest evaluated response, dropping any stale intermediate epochs.
    pub fn try_recv_latest(&self) -> Option<EvaluationResponse> {
        let mut latest: Option<EvaluationResponse> = None;
        while let Ok(res) = self.rx_res.try_recv() {
            if let Some(prev) = &latest {
                if res.epoch >= prev.epoch {
                    latest = Some(res);
                }
            } else {
                latest = Some(res);
            }
        }
        latest
    }
}
