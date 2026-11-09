//! # `algebra_engine::heuristic`
//!
//! Multi-Tier Probabilistic Heuristic Search, Path Weighting & Solution Certification Engine.
//!
//! ## 4-Tier Architecture
//! 1. **Tier 1: $O(1)$ Fast Probabilistic Screening ($\epsilon \approx 0.01\%$)**:
//!    Fast Schwartz-Zippel fingerprinting over $\mathbb{F}_p$ ($p = 2^{31}-1$) to assign heuristic
//!    weights to candidate search branches in $A^*$ / E-Graph exploration queues.
//! 2. **Tier 2: Progressive Multi-Point / Multi-Prime CRT Verification ($\epsilon < 10^{-20}$)**:
//!    Promoted candidate branches undergo deep verification across multiple cryptographic
//!    primes ($\mathbb{F}_{2^{31}-1}, \mathbb{F}_{2^{61}-1}, \mathbb{F}_{2^{64}-59}$) and randomized interval bounds.
//! 3. **Tier 3: Opportunistic Deterministic Proof & Fallback**:
//!    Exact deterministic reduction (Gröbner basis, canonical polynomial forms, Risch decision tree)
//!    executed when cheaper or when candidate branches are exhausted.
//! 4. **Tier 4: Probabilistic Solution Returns with Formal Confidence Certificates**:
//!    When computational budgets (soft/hard caps) are reached, returns [`Solution::Probabilistic`]
//!    with upper bound on error probability and sample metadata.

use algebra_core::probabilistic::{ProbabilisticVerifier, Solution};
use algebra_core::{EngineConfig, ExprGraph, ExprId, ExprKind, Number};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// Heuristic search candidate with priority weight.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchCandidate {
    pub expr_id: ExprId,
    pub priority_weight: f64,
    pub depth: usize,
    pub step_description: String,
}

impl Eq for SearchCandidate {}

impl PartialOrd for SearchCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SearchCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority_weight
            .partial_cmp(&other.priority_weight)
            .unwrap_or(Ordering::Equal)
    }
}

/// Quantized Transformer Weights for Symbolic Expression Embeddings.
///
/// Ingests small, quantized INT8 or FP32 Transformer representations via the Safetensors bridge
/// for low-overhead heuristic guidance across desktop, mobile, and WASM.
#[derive(Debug, Clone)]
pub struct QuantizedTransformerWeights {
    pub vocab_size: usize,
    pub embed_dim: usize,
    pub token_embeddings: Vec<f32>,
    pub projection_weights: Vec<f32>,
    pub scale: f32,
}

impl QuantizedTransformerWeights {
    /// Ingest from a SafeTensors binary byte buffer using `spodeian-ml-utils`.
    pub fn from_safetensors_bytes(bytes: &[u8]) -> Result<Self, String> {
        // First try central spodeian_ml_utils loader backed by Candle
        if let Ok(loaded) = spodeian_ml_utils::LoadedModelWeights::from_bytes(bytes, &spodeian_ml_utils::candle_core::Device::Cpu) {
            let emb_tensor = loaded
                .get_tensor("embeddings")
                .or_else(|_| loaded.get_tensor("token_embeddings"));

            if let Ok(tensor) = emb_tensor {
                let shape = tensor.dims();
                let vocab_size = *shape.first().unwrap_or(&16);
                let embed_dim = *shape.get(1).unwrap_or(&8);
                let scale = 0.05f32;

                let token_embeddings = if tensor.dtype() == spodeian_ml_utils::candle_core::DType::U8 {
                    spodeian_ml_utils::LoadedModelWeights::dequantize_int8(tensor, scale)
                        .map_err(|e| e.to_string())?
                        .flatten_all()
                        .map_err(|e| e.to_string())?
                        .to_vec1::<f32>()
                        .map_err(|e| e.to_string())?
                } else {
                    tensor
                        .to_dtype(spodeian_ml_utils::candle_core::DType::F32)
                        .map_err(|e| e.to_string())?
                        .flatten_all()
                        .map_err(|e| e.to_string())?
                        .to_vec1::<f32>()
                        .map_err(|e| e.to_string())?
                };

                let projection_weights = if let Ok(p_tensor) = loaded.get_tensor("projection").or_else(|_| loaded.get_tensor("projection_weights")) {
                    if p_tensor.dtype() == spodeian_ml_utils::candle_core::DType::U8 {
                        spodeian_ml_utils::LoadedModelWeights::dequantize_int8(p_tensor, scale)
                            .map_err(|e| e.to_string())?
                            .flatten_all()
                            .map_err(|e| e.to_string())?
                            .to_vec1::<f32>()
                            .map_err(|e| e.to_string())?
                    } else {
                        p_tensor
                            .to_dtype(spodeian_ml_utils::candle_core::DType::F32)
                            .map_err(|e| e.to_string())?
                            .flatten_all()
                            .map_err(|e| e.to_string())?
                            .to_vec1::<f32>()
                            .map_err(|e| e.to_string())?
                    }
                } else {
                    let mut id = vec![0.0f32; embed_dim * embed_dim];
                    for i in 0..embed_dim {
                        id[i * embed_dim + i] = 1.0;
                    }
                    id
                };


                return Ok(Self {
                    vocab_size,
                    embed_dim,
                    token_embeddings,
                    projection_weights,
                    scale,
                });
            }
        }

        // Fallback for minimal synthetic buffers without Candle tensors
        if bytes.len() < 8 {
            return Err("SafeTensors buffer too short for header size".to_string());
        }

        let header_len = u64::from_le_bytes(
            bytes[0..8]
                .try_into()
                .map_err(|e| format!("Invalid header length: {e}"))?,
        ) as usize;

        if bytes.len() < 8 + header_len {
            return Err("SafeTensors buffer truncated".to_string());
        }

        let header_json = std::str::from_utf8(&bytes[8..8 + header_len])
            .map_err(|e| format!("Invalid UTF-8 in SafeTensors header: {e}"))?;

        let header: serde_json::Value = serde_json::from_str(header_json)
            .map_err(|e| format!("Invalid JSON header: {e}"))?;

        let data_start = 8 + header_len;

        let scale = header
            .get("__metadata__")
            .and_then(|m| m.get("scale"))
            .and_then(|s| s.as_str())
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.05);

        let emb_info = header
            .get("embeddings")
            .or_else(|| header.get("token_embeddings"))
            .ok_or_else(|| "Missing 'embeddings' tensor in SafeTensors header".to_string())?;

        let shape = emb_info
            .get("shape")
            .and_then(|s| s.as_array())
            .ok_or_else(|| "Missing shape for embeddings".to_string())?;

        let vocab_size = shape.first().and_then(|v| v.as_u64()).unwrap_or(16) as usize;
        let embed_dim = shape.get(1).and_then(|v| v.as_u64()).unwrap_or(8) as usize;

        let offsets = emb_info
            .get("data_offsets")
            .and_then(|o| o.as_array())
            .ok_or_else(|| "Missing data_offsets for embeddings".to_string())?;
        let s_off = offsets[0].as_u64().unwrap_or(0) as usize;
        let e_off = offsets[1].as_u64().unwrap_or(0) as usize;

        let emb_bytes = &bytes[data_start + s_off..data_start + e_off];
        let dtype_str = emb_info.get("dtype").and_then(|d| d.as_str()).unwrap_or("F32");

        let token_embeddings = if dtype_str == "I8" || dtype_str == "U8" || dtype_str == "INT8" {
            emb_bytes.iter().map(|&b| (b as i8) as f32 * scale).collect()
        } else {
            emb_bytes
                .as_chunks::<4>()
                .0
                .iter()
                .map(|chunk| f32::from_le_bytes(*chunk))
                .collect()
        };

        let proj_info = header
            .get("projection")
            .or_else(|| header.get("projection_weights"));

        let projection_weights = if let Some(p_info) = proj_info {
            let p_offsets = p_info
                .get("data_offsets")
                .and_then(|o| o.as_array())
                .ok_or_else(|| "Missing data_offsets for projection".to_string())?;
            let p_s = p_offsets[0].as_u64().unwrap_or(0) as usize;
            let p_e = p_offsets[1].as_u64().unwrap_or(0) as usize;
            let p_bytes = &bytes[data_start + p_s..data_start + p_e];
            let p_dtype = p_info.get("dtype").and_then(|d| d.as_str()).unwrap_or("F32");
            if p_dtype == "I8" || p_dtype == "U8" || p_dtype == "INT8" {
                p_bytes.iter().map(|&b| (b as i8) as f32 * scale).collect()
            } else {
                p_bytes
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|chunk| f32::from_le_bytes(*chunk))
                    .collect()
            }
        } else {
            let mut id = vec![0.0f32; embed_dim * embed_dim];
            for i in 0..embed_dim {
                id[i * embed_dim + i] = 1.0;
            }
            id
        };

        Ok(Self {
            vocab_size,
            embed_dim,
            token_embeddings,
            projection_weights,
            scale,
        })
    }


    /// Calibrated compact 8-dimensional quantized representation for core AST symbol tokens.
    pub fn default_quantized_weights() -> Self {
        let vocab_size = 16;
        let embed_dim = 8;
        let mut token_embeddings = vec![0.0f32; vocab_size * embed_dim];

        // Seed distinctive orthogonal representations for fundamental AST node classes
        for token_id in 0..vocab_size {
            for dim in 0..embed_dim {
                let freq = (token_id + 1) as f32 * (dim + 1) as f32 * 0.35;
                token_embeddings[token_id * embed_dim + dim] = freq.sin();
            }
        }

        let mut projection_weights = vec![0.0f32; embed_dim * embed_dim];
        for i in 0..embed_dim {
            projection_weights[i * embed_dim + i] = 1.0;
        }

        Self {
            vocab_size,
            embed_dim,
            token_embeddings,
            projection_weights,
            scale: 0.05,
        }
    }
}

/// Neural Expression Embedder for Heuristic Search Guidance.
#[derive(Debug, Clone)]
pub struct NeuralExpressionEmbedder {
    pub weights: QuantizedTransformerWeights,
    pub evaluator: spodeian_ml_utils::ast::MathAstEvaluator,
}

impl Default for NeuralExpressionEmbedder {
    fn default() -> Self {
        // Load production contrastive AST embeddings trained via spodeian-ml
        const EMBEDDED_MODEL: &[u8] = include_bytes!("../data/algebra_expression_embedder.safetensors");
        if let Ok(weights) = QuantizedTransformerWeights::from_safetensors_bytes(EMBEDDED_MODEL) {
            Self::new(weights)
        } else {
            Self::new(QuantizedTransformerWeights::default_quantized_weights())
        }
    }
}


impl NeuralExpressionEmbedder {
    pub fn new(weights: QuantizedTransformerWeights) -> Self {
        let mut evaluator = spodeian_ml_utils::ast::MathAstEvaluator::default_with_dim(
            weights.embed_dim,
            weights.vocab_size,
        );
        if weights.token_embeddings.len() >= weights.vocab_size * weights.embed_dim {
            evaluator.op_embeddings = weights.token_embeddings.clone();
        }
        Self { weights, evaluator }
    }

    /// Ingest weights from SafeTensors binary bytes.
    pub fn from_safetensors(bytes: &[u8]) -> Result<Self, String> {
        let weights = QuantizedTransformerWeights::from_safetensors_bytes(bytes)?;
        Ok(Self::new(weights))
    }

    /// Map expression node to symbolic token ID.
    pub fn node_to_token_id(node: &algebra_core::ExprNode) -> usize {
        match &node.kind {
            ExprKind::Number(_) => 1,
            ExprKind::Symbol(_) => 2,
            ExprKind::Add(_) => 3,
            ExprKind::Mul(_) => 4,
            ExprKind::Pow(_, _) => 5,
            ExprKind::Div(_, _) => 6,
            ExprKind::Sub(_, _) => 7,
            ExprKind::Neg(_) => 8,
            ExprKind::Function { .. } => 9,
            ExprKind::Derivative { .. } => 10,
            ExprKind::Integral { .. } => 11,
            ExprKind::Matrix { .. } => 12,
            ExprKind::Relational { .. } => 13,
            ExprKind::SetOperation { .. } => 14,
            ExprKind::Sum { .. } => 15,
            _ => 0,
        }
    }

    /// Compute continuous semantic vector embedding for an expression AST DAG
    /// using `spodeian_ml_utils::ast::MathAstEvaluator`.
    pub fn embed_expression(&self, graph: &ExprGraph, root: ExprId) -> Vec<f32> {
        use smallvec::SmallVec;
        use spodeian_ml_utils::ast::{AstNodeKind, CanonicalAstNode};

        let mut nodes: Vec<CanonicalAstNode> = Vec::new();
        let mut id_map: ahash::AHashMap<ExprId, usize> = ahash::AHashMap::new();
        let mut queue: SmallVec<[(ExprId, usize); 64]> = SmallVec::new();
        let mut var_order: ahash::AHashMap<algebra_core::SymbolId, usize> = ahash::AHashMap::new();

        queue.push((root, 0usize));
        id_map.insert(root, 0);

        let mut head = 0usize;
        while head < queue.len() && nodes.len() < 64 {
            let (curr_id, depth) = queue[head];
            let curr_idx = head;
            head += 1;

            let node = graph.get(curr_id);
            let kind = match &node.kind {
                ExprKind::Number(_) => AstNodeKind::Num,
                ExprKind::Symbol(_) => AstNodeKind::Var,
                ExprKind::Add(_) => AstNodeKind::Add,
                ExprKind::Mul(_) => AstNodeKind::Mul,
                ExprKind::Pow(_, _) => AstNodeKind::Pow,
                ExprKind::Div(_, _) => AstNodeKind::Div,
                ExprKind::Sub(_, _) => AstNodeKind::Sub,
                ExprKind::Neg(_) => AstNodeKind::Neg,
                ExprKind::Function { .. } => AstNodeKind::Func,
                ExprKind::Derivative { .. } => AstNodeKind::Derivative,
                ExprKind::Integral { .. } => AstNodeKind::Integral,
                ExprKind::Matrix { .. } => AstNodeKind::Matrix,
                ExprKind::Relational { .. } => AstNodeKind::Relational,
                ExprKind::SetOperation { .. } => AstNodeKind::SetOp,
                ExprKind::Sum { .. } => AstNodeKind::Sum,
                _ => AstNodeKind::Pad,
            };

            let mut c_node = CanonicalAstNode::new(curr_idx, kind, depth);

            if let ExprKind::Number(num) = &node.kind {
                let val = match num {
                    Number::Integer(i) => i.to_string().parse::<f32>().unwrap_or(0.0),
                    Number::Float(f) => *f as f32,
                    _ => 0.0,
                };
                let sign = if val < 0.0 { -1.0 } else if val > 0.0 { 1.0 } else { 0.0 };
                let abs_v = val.abs();
                let exp = if abs_v > 1e-12 { abs_v.log2().floor() } else { 0.0 };
                let mant = if abs_v > 1e-12 { abs_v / 2.0f32.powf(exp) } else { 0.0 };
                c_node.num_features = [sign, mant, exp];
            } else if let ExprKind::Symbol(s) = &node.kind {
                // Pre-order alpha-equivalence anonymization
                let next_var = var_order.len();
                let v_id = *var_order.entry(s.clone()).or_insert(next_var);
                c_node.var_id = v_id;
            }

            for child in node.children() {
                let child_idx = if let Some(&existing) = id_map.get(&child) {
                    existing
                } else if queue.len() < 64 {
                    let next_idx = queue.len();
                    id_map.insert(child, next_idx);
                    queue.push((child, depth + 1));
                    next_idx
                } else {
                    continue;
                };
                c_node.children.push(child_idx);
            }

            nodes.push(c_node);
        }

        self.evaluator.evaluate_dag(&nodes, 0)
    }

    /// Compute cosine similarity between two expression embeddings in $[-1.0, 1.0]$.
    pub fn cosine_similarity(e1: &[f32], e2: &[f32]) -> f64 {
        assert_eq!(e1.len(), e2.len());
        let dot: f32 = e1.iter().zip(e2).map(|(&a, &b)| a * b).sum();
        dot as f64
    }

    /// Score semantic equivalence similarity between two symbolic expressions.
    pub fn score_similarity(&self, graph: &ExprGraph, candidate: ExprId, target: ExprId) -> f64 {
        let e1 = self.embed_expression(graph, candidate);
        let e2 = self.embed_expression(graph, target);
        Self::cosine_similarity(&e1, &e2)
    }
}

/// Pluggable heuristic evaluator guiding algebraic expression search and simplification.
pub trait SearchHeuristic: Send + Sync {
    /// Evaluate heuristic priority / fitness score for candidate expression in `graph`,
    /// optionally evaluated relative to a target expression `target_id`.
    fn evaluate(&self, graph: &ExprGraph, candidate_id: ExprId, target_id: Option<ExprId>) -> f64;

    /// Descriptive name of the search heuristic.
    fn name(&self) -> &'static str;
}

/// Syntactic complexity minimization heuristic (favors smaller node counts and lower tree depth).
#[derive(Debug, Clone, Copy, Default)]
pub struct TreeSizeHeuristic;

impl SearchHeuristic for TreeSizeHeuristic {
    fn evaluate(&self, graph: &ExprGraph, candidate_id: ExprId, _target_id: Option<ExprId>) -> f64 {
        let nodes = count_ast_nodes(graph, candidate_id) as f64;
        let depth = compute_ast_depth(graph, candidate_id) as f64;
        100.0 / (1.0 + nodes + 0.2 * depth)
    }

    fn name(&self) -> &'static str {
        "TreeSizeHeuristic"
    }
}

/// Schwartz-Zippel numerical equivalence probability heuristic.
#[derive(Debug, Clone, Copy, Default)]
pub struct SchwartzZippelHeuristic;

impl SearchHeuristic for SchwartzZippelHeuristic {
    fn evaluate(&self, graph: &ExprGraph, candidate_id: ExprId, target_id: Option<ExprId>) -> f64 {
        ProbabilisticVerifier::compute_candidate_heuristic_weight(graph, candidate_id, target_id)
    }

    fn name(&self) -> &'static str {
        "SchwartzZippelHeuristic"
    }
}

impl SearchHeuristic for NeuralExpressionEmbedder {
    fn evaluate(&self, graph: &ExprGraph, candidate_id: ExprId, target_id: Option<ExprId>) -> f64 {
        if let Some(target) = target_id {
            let sim = self.score_similarity(graph, candidate_id, target);
            (1.0 + sim).max(0.0)
        } else {
            let emb = self.embed_expression(graph, candidate_id);
            let norm_sq: f32 = emb.iter().map(|&x| x * x).sum();
            norm_sq as f64
        }
    }

    fn name(&self) -> &'static str {
        "NeuralExpressionEmbedder"
    }
}

/// Weighted composite combination of multiple heuristics.
pub struct CompositeSearchHeuristic {
    pub components: Vec<(Box<dyn SearchHeuristic>, f64)>,
}

impl Default for CompositeSearchHeuristic {
    fn default() -> Self {
        Self::default_composite()
    }
}

impl CompositeSearchHeuristic {
    pub fn new(components: Vec<(Box<dyn SearchHeuristic>, f64)>) -> Self {
        Self { components }
    }

    pub fn default_composite() -> Self {
        Self {
            components: vec![
                (Box::new(SchwartzZippelHeuristic), 1.0),
                (Box::new(TreeSizeHeuristic), 0.2),
            ],
        }
    }
}

impl SearchHeuristic for CompositeSearchHeuristic {
    fn evaluate(&self, graph: &ExprGraph, candidate_id: ExprId, target_id: Option<ExprId>) -> f64 {
        let mut total = 0.0;
        for (h, weight) in &self.components {
            total += h.evaluate(graph, candidate_id, target_id) * weight;
        }
        total
    }

    fn name(&self) -> &'static str {
        "CompositeSearchHeuristic"
    }
}

/// Multi-tier heuristic search and certification engine.
pub struct HeuristicSearchEngine {
    pub config: EngineConfig,
    pub heuristic: Box<dyn SearchHeuristic>,
    pub neural_embedder: Option<NeuralExpressionEmbedder>,
}

impl Default for HeuristicSearchEngine {
    fn default() -> Self {
        Self::new(EngineConfig::default())
    }
}

impl HeuristicSearchEngine {
    /// Create a new heuristic search engine with given configuration.
    pub fn new(config: EngineConfig) -> Self {
        Self {
            config,
            heuristic: Box::new(SchwartzZippelHeuristic),
            neural_embedder: None,
        }
    }

    /// Attach a custom pluggable search heuristic.
    pub fn with_heuristic(mut self, heuristic: Box<dyn SearchHeuristic>) -> Self {
        self.heuristic = heuristic;
        self
    }

    /// Create a new heuristic search engine with calibrated neural embedder active by default.
    pub fn with_default_neural_guidance(config: EngineConfig) -> Self {
        let embedder = NeuralExpressionEmbedder::default();
        Self {
            config,
            heuristic: Box::new(CompositeSearchHeuristic::new(vec![
                (Box::new(SchwartzZippelHeuristic), 1.0),
                (Box::new(TreeSizeHeuristic), 0.2),
                (Box::new(embedder.clone()), 0.5),
            ])),
            neural_embedder: Some(embedder),
        }
    }

    /// Attach a neural expression embedder for hybrid search guidance.
    pub fn with_neural_embedder(mut self, embedder: NeuralExpressionEmbedder) -> Self {
        self.neural_embedder = Some(embedder);
        self
    }

    /// Tier 1: Fast heuristic weighting of a candidate search branch.
    /// Combines active `SearchHeuristic` with neural transformer embeddings.
    pub fn weight_candidate(
        &self,
        graph: &ExprGraph,
        candidate_id: ExprId,
        target_id: Option<ExprId>,
    ) -> f64 {
        let base_weight = self.heuristic.evaluate(graph, candidate_id, target_id);

        if let (Some(embedder), Some(target)) = (&self.neural_embedder, target_id) {
            let sim = embedder.score_similarity(graph, candidate_id, target);
            base_weight * (1.0 + 0.5 * sim.max(0.0))
        } else {
            base_weight
        }
    }

    /// Parallel batch candidate weighting across multiple rewrite options using Rayon.
    pub fn weight_candidates_parallel(
        &self,
        graph: &ExprGraph,
        candidates: &[ExprId],
        target_id: Option<ExprId>,
    ) -> Vec<f64> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use rayon::prelude::*;
            candidates
                .par_iter()
                .map(|&c| self.weight_candidate(graph, c, target_id))
                .collect()
        }
        #[cfg(target_arch = "wasm32")]
        {
            candidates
                .iter()
                .map(|&c| self.weight_candidate(graph, c, target_id))
                .collect()
        }
    }

    /// Tier 2: Progressive multi-prime CRT verification down to target epsilon (e.g. $10^{-20}$).
    pub fn verify_candidate(
        &self,
        graph: &ExprGraph,
        candidate_id: ExprId,
        target_id: ExprId,
        target_epsilon: f64,
    ) -> Solution<bool> {
        ProbabilisticVerifier::verify_equivalence(graph, candidate_id, target_id, target_epsilon)
    }

    /// Parallel multi-prime CRT verification using Rayon on desktop platforms.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn verify_candidate_parallel(
        &self,
        graph: &ExprGraph,
        candidate_id: ExprId,
        target_id: ExprId,
        target_epsilon: f64,
    ) -> Solution<bool> {
        ProbabilisticVerifier::verify_equivalence_parallel(graph, candidate_id, target_id, target_epsilon)
    }

    /// Simplify an expression with opportunistic deterministic proof and probabilistic fallback.
    pub fn simplify_with_certification(
        &self,
        graph: &ExprGraph,
        expr_id: ExprId,
        target_epsilon: f64,
    ) -> Solution<ExprId> {
        let budget = &self.config.budget;
        let mut queue = BinaryHeap::new();
        let mut visited = std::collections::HashSet::new();

        let initial_weight = self.weight_candidate(graph, expr_id, None);
        queue.push(SearchCandidate {
            expr_id,
            priority_weight: initial_weight,
            depth: 0,
            step_description: "Initial expression".to_string(),
        });
        visited.insert(expr_id);

        let mut best_candidate = expr_id;
        let mut best_score = initial_weight;
        let mut iterations = 0;
        let (soft_cap, hard_cap) = budget.egraph_nodes;

        while let Some(current) = queue.pop() {
            iterations += 1;

            // Check hard cap
            if iterations >= hard_cap {
                let verification =
                    self.verify_candidate(graph, best_candidate, expr_id, target_epsilon);
                return Solution::Probabilistic {
                    result: best_candidate,
                    error_probability_upper_bound: verification
                        .error_probability()
                        .max(target_epsilon),
                    sample_count: verification.sample_count(),
                    method: "Heuristic Search (Hard Cap Rollback)",
                };
            }

            // Generate neighbor rewrites
            let neighbors = self.expand_rewrites(graph, current.expr_id);
            for n in neighbors {
                if visited.insert(n) {
                    let weight = self.weight_candidate(graph, n, Some(expr_id));
                    if weight > best_score {
                        best_score = weight;
                        best_candidate = n;
                    }

                    // Soft cap: stop queuing deeper expansions if soft cap reached
                    if iterations < soft_cap {
                        queue.push(SearchCandidate {
                            expr_id: n,
                            priority_weight: weight,
                            depth: current.depth + 1,
                            step_description: "Rewrite step".to_string(),
                        });
                    }
                }
            }

            // Tier 3: If candidate is significantly simpler and verified with high confidence, test deterministic identity
            if best_candidate != expr_id {
                let diff = graph.sub(expr_id, best_candidate);
                let diff_node = graph.get(diff);
                if let ExprKind::Number(Number::Integer(0)) = diff_node.kind {
                    return Solution::Deterministic(best_candidate);
                }
            }
        }

        // Return best candidate with certification
        let verif = self.verify_candidate(graph, best_candidate, expr_id, target_epsilon);
        if verif.is_deterministic() && *verif.as_ref() {
            Solution::Deterministic(best_candidate)
        } else {
            Solution::Probabilistic {
                result: best_candidate,
                error_probability_upper_bound: verif.error_probability().max(target_epsilon),
                sample_count: verif.sample_count(),
                method: "Multi-Tier Probabilistic Heuristics (Tier 2 CRT)",
            }
        }
    }

    /// Expand elementary algebraic rewrites (constant folding, zero removals, distribution).
    fn expand_rewrites(&self, graph: &ExprGraph, id: ExprId) -> Vec<ExprId> {
        let node = graph.get(id);
        let mut out = Vec::new();

        match &node.kind {
            ExprKind::Add(terms) => {
                // Remove zero terms
                let non_zero: smallvec::SmallVec<[ExprId; 4]> = terms
                    .iter()
                    .copied()
                    .filter(|&t| {
                        let tn = graph.get(t);
                        !matches!(tn.kind, ExprKind::Number(Number::Integer(0)))
                    })
                    .collect();

                if non_zero.len() < terms.len() {
                    if non_zero.is_empty() {
                        out.push(graph.integer(0));
                    } else if non_zero.len() == 1 {
                        out.push(non_zero[0]);
                    } else {
                        out.push(graph.add(non_zero));
                    }
                }
            }
            ExprKind::Mul(factors) => {
                // If any factor is 0, entire product is 0
                for &f in factors {
                    let fn_node = graph.get(f);
                    if matches!(fn_node.kind, ExprKind::Number(Number::Integer(0))) {
                        out.push(graph.integer(0));
                        return out;
                    }
                }
                // Remove 1 factors
                let non_one: smallvec::SmallVec<[ExprId; 4]> = factors
                    .iter()
                    .copied()
                    .filter(|&f| {
                        let fn_node = graph.get(f);
                        !matches!(fn_node.kind, ExprKind::Number(Number::Integer(1)))
                    })
                    .collect();

                if non_one.len() < factors.len() {
                    if non_one.is_empty() {
                        out.push(graph.integer(1));
                    } else if non_one.len() == 1 {
                        out.push(non_one[0]);
                    } else {
                        out.push(graph.mul(non_one));
                    }
                }
            }
            ExprKind::Sub(l, r) if l == r => {
                out.push(graph.integer(0));
            }
            ExprKind::Div(l, r) if l == r => {
                out.push(graph.integer(1));
            }
            _ => {}
        }

        out
    }

    /// Compute rich AST metrics, neural expression embedding, and proof-search depth indicators.
    pub fn analyze_ast_proof_depth(&self, graph: &ExprGraph, root: ExprId) -> AstProofDepthIndicator {
        let ast_depth = compute_ast_depth(graph, root);
        let ast_node_count = count_ast_nodes(graph, root);
        let embedder = self.neural_embedder.as_ref().cloned().unwrap_or_default();
        let neural_embedding = embedder.embed_expression(graph, root);
        let candidate_weight = self.weight_candidate(graph, root, None);

        // Run bounded opportunistic proof search to observe depth reached
        let budget = &self.config.budget;
        let max_depth_budget = 16usize;
        let mut queue = BinaryHeap::new();
        let mut visited = std::collections::HashSet::new();
        queue.push(SearchCandidate {
            expr_id: root,
            priority_weight: candidate_weight,
            depth: 0,
            step_description: "Root".to_string(),
        });
        visited.insert(root);

        let mut max_depth_reached = 0usize;
        let mut is_deterministic = false;
        let mut best_candidate = root;
        let (soft_cap, hard_cap) = budget.egraph_nodes;
        let mut iterations = 0usize;

        while let Some(current) = queue.pop() {
            iterations += 1;
            if current.depth > max_depth_reached {
                max_depth_reached = current.depth;
            }
            if iterations >= hard_cap.min(64) {
                break;
            }
            let neighbors = self.expand_rewrites(graph, current.expr_id);
            for n in neighbors {
                if visited.insert(n) {
                    let weight = self.weight_candidate(graph, n, Some(root));
                    if weight > candidate_weight {
                        best_candidate = n;
                    }
                    if current.depth < max_depth_budget && iterations < soft_cap.min(32) {
                        queue.push(SearchCandidate {
                            expr_id: n,
                            priority_weight: weight,
                            depth: current.depth + 1,
                            step_description: "Rewrite".to_string(),
                        });
                    }
                }
            }
            if best_candidate != root {
                let diff = graph.sub(root, best_candidate);
                let diff_node = graph.get(diff);
                if let ExprKind::Number(Number::Integer(0)) = diff_node.kind {
                    is_deterministic = true;
                    break;
                }
            }
        }

        let certification_status = if is_deterministic {
            "Deterministic (Identity Certified)".to_string()
        } else {
            let verif = self.verify_candidate(graph, best_candidate, root, 1e-10);
            if verif.is_deterministic() && *verif.as_ref() {
                is_deterministic = true;
                "Deterministic Proof Verified".to_string()
            } else {
                format!(
                    "Probabilistic (p_err <= {:.2e}, {} samples)",
                    verif.error_probability(),
                    verif.sample_count()
                )
            }
        };

        AstProofDepthIndicator {
            ast_depth,
            ast_node_count,
            proof_search_depth: max_depth_reached,
            max_depth_budget,
            is_deterministic,
            candidate_weight,
            certification_status,
            neural_embedding,
        }
    }
}

/// Diagnostic indicators and metrics for an expression AST and its heuristic proof search.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AstProofDepthIndicator {
    pub ast_depth: usize,
    pub ast_node_count: usize,
    pub proof_search_depth: usize,
    pub max_depth_budget: usize,
    pub is_deterministic: bool,
    pub candidate_weight: f64,
    pub certification_status: String,
    pub neural_embedding: Vec<f32>,
}

/// Compute maximum tree depth of an expression AST.
pub fn compute_ast_depth(graph: &ExprGraph, root: ExprId) -> usize {
    let node = graph.get(root);
    let children = node.children();
    if children.is_empty() {
        1
    } else {
        1 + children
            .into_iter()
            .map(|c| compute_ast_depth(graph, c))
            .max()
            .unwrap_or(0)
    }
}

/// Compute total node count of an expression AST DAG.
pub fn count_ast_nodes(graph: &ExprGraph, root: ExprId) -> usize {
    let mut visited = std::collections::HashSet::new();
    let mut stack = vec![root];
    let mut count = 0;
    while let Some(curr) = stack.pop() {
        if visited.insert(curr) {
            count += 1;
            let node = graph.get(curr);
            for child in node.children() {
                stack.push(child);
            }
        }
    }
    count
}
