//! # `algebra_engine::mcts`
//!
//! Monte Carlo Tree Search (MCTS) Proof Guidance & Neural-Symbolic Policy Engine.
//!
//! Guides algebraic expression simplification and formal equivalence search over the
//! hash-consed DAG arena (`ExprGraph`) using:
//! - **Value Head**: Evaluates distance to canonical simplified form $\hat{V}(E) \in [0, 1]$
//!   using syntactic complexity and quantized neural AST embeddings.
//! - **Policy Head**: Computes softmax prior probability distribution $\pi(a \mid E)$
//!   over valid algebraic rewrite actions.
//! - **PUCT Tree Exploration**: Balances exploitation $Q(s, a)$ with exploration $U(s, a)$.
//! - **Fast Rollout**: Simulation phase guided by Schwartz-Zippel fingerprint similarity.
//! - **Lake Trajectory Streaming**: Telemetry and dataset logging to `spodeian-lake` for
//!   closed-loop self-supervised re-training with `spodeian-ml`.

use crate::heuristic::NeuralExpressionEmbedder;
use crate::poly::grobner::GrobnerBasis;
use crate::poly::multivariate::MonomialOrder;
use crate::simplify::AlgebraicTransformations;
use crate::simplify::detector::DomainFeatureDetector;
use algebra_core::lemma_miner::LemmaRegistry;
use algebra_core::probabilistic::ProbabilisticVerifier;
use algebra_core::{ExprGraph, ExprId, ExprKind, SymbolId};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Discrete algebraic rewrite action applicable during MCTS exploration.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MctsAction {
    /// Expand products and polynomial powers: $(a+b)(c+d) \to ac+ad+bc+bd$.
    Expand,
    /// Put rational terms over a common denominator: $a/b + c/d \to (ad+bc)/(bd)$.
    Together,
    /// Cancel common polynomial factors in rational expressions.
    Cancel,
    /// Collect terms with respect to a target variable: $a x + b x \to (a+b) x$.
    Collect(SymbolId),
    /// Evaluate exact constant sub-expressions.
    FoldConstants,
    /// Apply standard trigonometric reduction: $\sin^2(x) + \cos^2(x) \to 1$.
    TrigReduction,
    /// Reduce modulo polynomial ideal using Gröbner basis.
    GrobnerReduce,
    /// Apply an autonomous learned rewrite lemma by ID.
    ApplyLemma(String),
}

impl std::fmt::Display for MctsAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Expand => write!(f, "Expand"),
            Self::Together => write!(f, "Together"),
            Self::Cancel => write!(f, "Cancel"),
            Self::Collect(sym) => write!(f, "Collect(@{})", sym.index()),
            Self::FoldConstants => write!(f, "FoldConstants"),
            Self::TrigReduction => write!(f, "TrigReduction"),
            Self::GrobnerReduce => write!(f, "GrobnerReduce"),
            Self::ApplyLemma(id) => write!(f, "ApplyLemma({id})"),
        }
    }
}

/// Node representation in the MCTS search tree.
#[derive(Debug, Clone)]
pub struct MctsNode {
    /// Symbolic expression DAG index representing the state.
    pub expr_id: ExprId,
    /// Index of parent node in search tree (`None` for root).
    pub parent: Option<usize>,
    /// Child transitions: mapping from action to child node index.
    pub children: Vec<(MctsAction, usize)>,
    /// Number of times this node has been visited: $N(s)$.
    pub visits: u32,
    /// Cumulative backpropagated value: $W(s)$.
    pub total_value: f64,
    /// Prior probability assigned by policy head: $P(s, a)$.
    pub prior_prob: f64,
    /// Search tree depth from root.
    pub depth: usize,
    /// Cached unexpanded candidate actions.
    pub untried_actions: Vec<MctsAction>,
}

impl MctsNode {
    /// Create a new unexpanded MCTS node.
    pub fn new(
        expr_id: ExprId,
        parent: Option<usize>,
        prior_prob: f64,
        depth: usize,
        untried_actions: Vec<MctsAction>,
    ) -> Self {
        Self {
            expr_id,
            parent,
            children: Vec::new(),
            visits: 0,
            total_value: 0.0,
            prior_prob,
            depth,
            untried_actions,
        }
    }

    /// Compute mean action value $Q(s) = W(s) / N(s)$.
    pub fn mean_value(&self) -> f64 {
        if self.visits == 0 {
            0.0
        } else {
            self.total_value / (self.visits as f64)
        }
    }
}

/// Value Head estimating proximity to canonical simplified form $\hat{V}(E) \in [0, 1]$.
#[derive(Debug, Clone)]
pub struct MctsValueHead {
    embedder: Option<NeuralExpressionEmbedder>,
}

impl Default for MctsValueHead {
    fn default() -> Self {
        Self::new(None)
    }
}

impl MctsValueHead {
    /// Create a new value head with optional neural embedder.
    pub fn new(embedder: Option<NeuralExpressionEmbedder>) -> Self {
        Self { embedder }
    }

    /// Compute syntactic complexity (node count + operator weights) of an expression DAG.
    pub fn compute_complexity(graph: &ExprGraph, root: ExprId) -> usize {
        let mut count = 0;
        let mut stack = vec![root];
        let mut visited = std::collections::HashSet::new();

        while let Some(curr) = stack.pop() {
            if !visited.insert(curr) {
                continue;
            }
            let node = graph.get(curr);
            let weight = match &node.kind {
                ExprKind::Number(_) | ExprKind::Symbol(_) => 1,
                ExprKind::Add(terms) => 2 + terms.len(),
                ExprKind::Mul(factors) => 3 + factors.len(),
                ExprKind::Sub(_, _) => 2,
                ExprKind::Div(_, _) => 5,
                ExprKind::Pow(_, _) => 6,
                ExprKind::Neg(_) => 1,
                ExprKind::Function { args, .. } => 8 + args.len(),
                _ => 4,
            };
            count += weight;
            stack.extend(node.children());
        }

        count
    }

    /// Estimate value $\hat{V}(E) \in [0, 1]$.
    ///
    /// If `target` is provided, computes similarity/match towards target.
    /// Otherwise, computes syntactic simplicity reward $\hat{V}(E) = 1 / (1 + 0.05 \cdot C(E))$.
    pub fn evaluate(&self, graph: &ExprGraph, expr_id: ExprId, target: Option<ExprId>) -> f64 {
        if let Some(tgt) = target {
            if expr_id == tgt {
                return 1.0;
            }

            // Schwartz-Zippel fingerprint check
            let syms = ProbabilisticVerifier::extract_symbols(graph, expr_id);
            let mut env = HashMap::new();
            for (i, sym) in syms.iter().enumerate() {
                env.insert(*sym, (i as u64 + 17) * 997);
            }
            let fp_e = ProbabilisticVerifier::eval_mod_p(
                graph,
                expr_id,
                ProbabilisticVerifier::MERSENNE_31,
                &env,
            );
            let fp_t = ProbabilisticVerifier::eval_mod_p(
                graph,
                tgt,
                ProbabilisticVerifier::MERSENNE_31,
                &env,
            );
            if let (Some(fe), Some(ft)) = (fp_e, fp_t) {
                if fe == ft {
                    return 0.95;
                }
            }

            // Neural embedding cosine similarity
            if let Some(emb) = &self.embedder {
                let sim = emb.score_similarity(graph, expr_id, tgt);
                // Map [-1, 1] to [0, 1]
                return ((sim + 1.0) / 2.0).clamp(0.0, 1.0);
            }
        }

        // Simplicity-based value
        let cost = Self::compute_complexity(graph, expr_id);
        1.0 / (1.0 + 0.05 * (cost as f64))
    }
}

/// Policy Head computing prior selection probability distribution $\pi(a \mid E)$ over rewrite rules.
#[derive(Debug, Clone, Default)]
pub struct MctsPolicyHead;

impl MctsPolicyHead {
    /// Generate candidate actions and their normalized prior probabilities $\pi(a \mid E)$.
    pub fn compute_policy(
        graph: &ExprGraph,
        expr_id: ExprId,
        lemma_registry: Option<&LemmaRegistry>,
    ) -> Vec<(MctsAction, f64)> {
        let features = DomainFeatureDetector::scan(graph, expr_id);
        let mut candidates = Vec::new();

        // 1. Always evaluate constant folding
        candidates.push((MctsAction::FoldConstants, 1.0f64));

        // 2. Expand: favored if ring arithmetic exists
        if features.is_commutative_ring || features.is_non_commutative_ring {
            candidates.push((MctsAction::Expand, 2.5f64));
        }

        // 3. Together & Cancel: favored if rational fractions / division exist
        let mut has_div = false;
        let mut stack = vec![expr_id];
        let mut visited = std::collections::HashSet::new();
        while let Some(curr) = stack.pop() {
            if !visited.insert(curr) {
                continue;
            }
            let node = graph.get(curr);
            if matches!(
                node.kind,
                ExprKind::Div(_, _) | ExprKind::Number(algebra_core::Number::Rational(_, _))
            ) {
                has_div = true;
                break;
            }
            stack.extend(node.children());
        }
        if has_div {
            candidates.push((MctsAction::Together, 3.0f64));
            candidates.push((MctsAction::Cancel, 3.5f64));
        }

        // 4. Collect: collect over free symbols
        let syms = ProbabilisticVerifier::extract_symbols(graph, expr_id);
        for &s in syms.iter().take(2) {
            candidates.push((MctsAction::Collect(s), 2.0f64));
        }

        // 5. Trig reduction if trigonometric functions detected
        if features.has_trigonometric {
            candidates.push((MctsAction::TrigReduction, 4.0f64));
        }

        // 6. Gröbner ideal reduction if commutative ring terms detected
        if features.is_commutative_ring {
            candidates.push((MctsAction::GrobnerReduce, 2.0f64));
        }

        // 7. Registered learned lemmas
        if let Some(reg) = lemma_registry {
            for lemma in reg.get_lemmas().iter().take(3) {
                candidates.push((MctsAction::ApplyLemma(lemma.id.clone()), 2.2f64));
            }
        }

        // Softmax normalization over action logits
        let max_logit = candidates
            .iter()
            .map(|(_, l)| *l)
            .fold(f64::NEG_INFINITY, f64::max);
        let mut exp_sum = 0.0f64;
        let mut exps = Vec::with_capacity(candidates.len());

        for (action, logit) in candidates {
            let e = (logit - max_logit).exp();
            exp_sum += e;
            exps.push((action, e));
        }

        exps.into_iter()
            .map(|(a, e)| (a, if exp_sum > 0.0 { e / exp_sum } else { 0.0 }))
            .collect()
    }
}

/// Single transition step in an MCTS proof trajectory.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProofStep {
    /// Initial expression index.
    pub from_expr: u32,
    /// Action applied.
    pub action: MctsAction,
    /// Resulting expression index.
    pub to_expr: u32,
    /// Estimated value at resulting state.
    pub value: f64,
    /// Visit count of the selected edge.
    pub visits: u32,
}

/// Outcome of an MCTS proof guidance or simplification search.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MctsSearchResult {
    /// Root expression index.
    pub root_expr: u32,
    /// Simplest / optimal expression index found.
    pub best_expr: u32,
    /// Shortest proof trajectory found.
    pub trajectory: Vec<ProofStep>,
    /// Highest reward / value attained.
    pub best_value: f64,
    /// Total MCTS simulation iterations performed.
    pub iterations: usize,
    /// Whether target expression was reached (if target specified).
    pub is_target_reached: bool,
}

/// Configuration settings for the Monte Carlo Tree Search proof engine.
#[derive(Debug, Clone)]
pub struct MctsConfig {
    /// Exploration constant $c_{\text{puct}}$ in PUCT formula (default: 1.414).
    pub c_puct: f64,
    /// Maximum search iterations (default: 50).
    pub max_iterations: usize,
    /// Maximum depth in search tree (default: 10).
    pub max_depth: usize,
    /// Maximum rollout simulation steps (default: 4).
    pub rollout_horizon: usize,
}

impl Default for MctsConfig {
    fn default() -> Self {
        Self {
            c_puct: 1.414,
            max_iterations: 50,
            max_depth: 10,
            rollout_horizon: 4,
        }
    }
}

/// Monte Carlo Tree Search (MCTS) Proof Search Engine.
#[derive(Debug, Clone)]
pub struct MctsSearchEngine {
    config: MctsConfig,
    value_head: MctsValueHead,
    lemma_registry: Option<LemmaRegistry>,
}

impl MctsSearchEngine {
    /// Create a new MCTS search engine with specified configuration.
    pub fn new(
        config: MctsConfig,
        value_head: MctsValueHead,
        lemma_registry: Option<LemmaRegistry>,
    ) -> Self {
        Self {
            config,
            value_head,
            lemma_registry,
        }
    }

    /// Apply an algebraic action to transform an expression into a successor.
    pub fn apply_action(
        &self,
        graph: &ExprGraph,
        expr_id: ExprId,
        action: &MctsAction,
    ) -> Option<ExprId> {
        match action {
            MctsAction::Expand => {
                let res = AlgebraicTransformations::expand(graph, expr_id);
                if res != expr_id { Some(res) } else { None }
            }
            MctsAction::Together => {
                let res = AlgebraicTransformations::together(graph, expr_id);
                if res != expr_id { Some(res) } else { None }
            }
            MctsAction::Cancel => {
                let res = AlgebraicTransformations::cancel(graph, expr_id);
                if res != expr_id { Some(res) } else { None }
            }
            MctsAction::Collect(sym) => {
                let res = AlgebraicTransformations::collect(graph, expr_id, *sym);
                if res != expr_id { Some(res) } else { None }
            }
            MctsAction::FoldConstants => {
                // Check if all children evaluate to scalar number
                let syms = ProbabilisticVerifier::extract_symbols(graph, expr_id);
                if syms.is_empty() {
                    if let Some(val) =
                        ProbabilisticVerifier::eval_float(graph, expr_id, &HashMap::new())
                    {
                        let folded = graph.float(val);
                        if folded != expr_id {
                            return Some(folded);
                        }
                    }
                }
                None
            }
            MctsAction::TrigReduction => {
                // Apply sin^2(x) + cos^2(x) = 1 simplification if expression has terms
                let node = graph.get(expr_id);
                if let ExprKind::Add(terms) = &node.kind {
                    if terms.len() == 2 {
                        let t0 = graph.get(terms[0]);
                        let t1 = graph.get(terms[1]);
                        if let (ExprKind::Pow(b0, e0), ExprKind::Pow(b1, e1)) = (&t0.kind, &t1.kind)
                        {
                            let n_e0 = graph.get(*e0);
                            let n_e1 = graph.get(*e1);
                            if let (
                                ExprKind::Number(algebra_core::Number::Integer(2)),
                                ExprKind::Number(algebra_core::Number::Integer(2)),
                            ) = (&n_e0.kind, &n_e1.kind)
                            {
                                let nb0 = graph.get(*b0);
                                let nb1 = graph.get(*b1);
                                if let (
                                    ExprKind::Function { name: f0, args: a0 },
                                    ExprKind::Function { name: f1, args: a1 },
                                ) = (&nb0.kind, &nb1.kind)
                                {
                                    let n0 = graph.symbols.resolve(*f0).unwrap_or_default();
                                    let n1 = graph.symbols.resolve(*f1).unwrap_or_default();
                                    if ((n0 == "sin" && n1 == "cos")
                                        || (n0 == "cos" && n1 == "sin"))
                                        && a0 == a1
                                    {
                                        return Some(graph.integer(1));
                                    }
                                }
                            }
                        }
                    }
                }
                None
            }
            MctsAction::GrobnerReduce => {
                let vars: Vec<String> = ProbabilisticVerifier::extract_symbols(graph, expr_id)
                    .iter()
                    .filter_map(|&s| graph.symbols.resolve(s))
                    .collect();
                if let Ok(basis) =
                    GrobnerBasis::from_exprs(graph, &[], &vars, MonomialOrder::GrevLex)
                {
                    if let Ok(reduced) = basis.reduce_expr(graph, expr_id) {
                        if reduced != expr_id {
                            return Some(reduced);
                        }
                    }
                }
                None
            }
            MctsAction::ApplyLemma(lemma_id) => {
                if let Some(reg) = &self.lemma_registry {
                    if let Some(lemma) = reg.find_by_id(lemma_id) {
                        if expr_id == lemma.lhs {
                            return Some(lemma.rhs);
                        } else if lemma.is_bidirectional && expr_id == lemma.rhs {
                            return Some(lemma.lhs);
                        }
                    }
                }
                None
            }
        }
    }

    /// Select optimal child node index using PUCT formula:
    /// $$a^* = \arg\max_i \left( Q(s, a_i) + c_{\text{puct}} \cdot P(s, a_i) \cdot \frac{\sqrt{\sum_j N(s, a_j)}}{1 + N(s, a_i)} \right)$$
    pub fn select_puct_child(&self, nodes: &[MctsNode], node_idx: usize) -> Option<usize> {
        let parent = &nodes[node_idx];
        if parent.children.is_empty() {
            return None;
        }

        let total_child_visits: u32 = parent.children.iter().map(|&(_, c)| nodes[c].visits).sum();
        let sqrt_total = (total_child_visits as f64).sqrt();

        let mut best_score = f64::NEG_INFINITY;
        let mut best_child = None;

        for &(_, child_idx) in &parent.children {
            let child = &nodes[child_idx];
            let q = child.mean_value();
            let u =
                self.config.c_puct * child.prior_prob * sqrt_total / (1.0 + child.visits as f64);
            let score = q + u;

            if score > best_score {
                best_score = score;
                best_child = Some(child_idx);
            }
        }

        best_child
    }

    /// Fast simulation rollout phase up to `rollout_horizon` steps.
    pub fn fast_rollout(
        &self,
        graph: &ExprGraph,
        start_expr: ExprId,
        target: Option<ExprId>,
    ) -> f64 {
        let mut curr = start_expr;

        for _ in 0..self.config.rollout_horizon {
            if let Some(tgt) = target {
                if curr == tgt {
                    return 1.0;
                }
            }

            let policy = MctsPolicyHead::compute_policy(graph, curr, self.lemma_registry.as_ref());
            if policy.is_empty() {
                break;
            }

            // Pick highest prior probability action
            let (best_action, _) = policy
                .iter()
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap();

            if let Some(next_expr) = self.apply_action(graph, curr, best_action) {
                if next_expr == curr {
                    break;
                }
                curr = next_expr;
            } else {
                break;
            }
        }

        self.value_head.evaluate(graph, curr, target)
    }

    /// Search for optimal algebraic simplification or target equivalence trajectory.
    pub fn search(
        &self,
        graph: &ExprGraph,
        root_expr: ExprId,
        target: Option<ExprId>,
    ) -> MctsSearchResult {
        let initial_policy =
            MctsPolicyHead::compute_policy(graph, root_expr, self.lemma_registry.as_ref());
        let root_actions: Vec<MctsAction> = initial_policy.into_iter().map(|(a, _)| a).collect();

        let mut nodes = vec![MctsNode::new(root_expr, None, 1.0, 0, root_actions)];

        let mut best_expr = root_expr;
        let mut best_value = self.value_head.evaluate(graph, root_expr, target);
        let mut target_reached = target == Some(root_expr);

        for iter in 0..self.config.max_iterations {
            if target_reached {
                break;
            }

            // 1. Selection
            let mut curr_idx = 0;
            while nodes[curr_idx].untried_actions.is_empty() && !nodes[curr_idx].children.is_empty()
            {
                if let Some(next_idx) = self.select_puct_child(&nodes, curr_idx) {
                    curr_idx = next_idx;
                } else {
                    break;
                }
            }

            // 2. Expansion
            let mut expanded_idx = curr_idx;
            if !nodes[curr_idx].untried_actions.is_empty()
                && nodes[curr_idx].depth < self.config.max_depth
            {
                let action = nodes[curr_idx].untried_actions.pop().unwrap();
                let parent_expr = nodes[curr_idx].expr_id;

                if let Some(next_expr) = self.apply_action(graph, parent_expr, &action) {
                    let child_policy = MctsPolicyHead::compute_policy(
                        graph,
                        next_expr,
                        self.lemma_registry.as_ref(),
                    );
                    let mut child_actions = Vec::new();
                    let mut prior_prob = 0.5;

                    for (a, p) in child_policy {
                        if a == action {
                            prior_prob = p;
                        }
                        child_actions.push(a);
                    }

                    let child_node = MctsNode::new(
                        next_expr,
                        Some(curr_idx),
                        prior_prob,
                        nodes[curr_idx].depth + 1,
                        child_actions,
                    );
                    let new_idx = nodes.len();
                    nodes.push(child_node);
                    nodes[curr_idx].children.push((action, new_idx));
                    expanded_idx = new_idx;

                    // Update global best tracking
                    let val = self.value_head.evaluate(graph, next_expr, target);
                    if val > best_value {
                        best_value = val;
                        best_expr = next_expr;
                    }
                    if target == Some(next_expr) {
                        target_reached = true;
                    }
                }
            }

            // 3. Rollout / Simulation
            let leaf_expr = nodes[expanded_idx].expr_id;
            let rollout_value = self.fast_rollout(graph, leaf_expr, target);

            // 4. Backpropagation
            let mut back_idx = Some(expanded_idx);
            while let Some(idx) = back_idx {
                nodes[idx].visits += 1;
                nodes[idx].total_value += rollout_value;
                back_idx = nodes[idx].parent;
            }

            if target_reached {
                tracing::debug!(iter = iter, "MCTS target expression reached");
                break;
            }
        }

        // Reconstruct best trajectory by following highest-visit child links
        let mut trajectory = Vec::new();
        let mut curr_idx = 0;

        while !nodes[curr_idx].children.is_empty() {
            let best_child = nodes[curr_idx]
                .children
                .iter()
                .max_by_key(|&(_, c)| nodes[*c].visits);

            if let Some(&(ref action, next_idx)) = best_child {
                let from_expr = nodes[curr_idx].expr_id.index();
                let to_expr = nodes[next_idx].expr_id.index();
                let visits = nodes[next_idx].visits;
                let value = nodes[next_idx].mean_value();

                trajectory.push(ProofStep {
                    from_expr,
                    action: action.clone(),
                    to_expr,
                    value,
                    visits,
                });

                curr_idx = next_idx;
            } else {
                break;
            }
        }

        MctsSearchResult {
            root_expr: root_expr.index(),
            best_expr: best_expr.index(),
            trajectory,
            best_value,
            iterations: self.config.max_iterations,
            is_target_reached: target_reached,
        }
    }

    /// Stream generated proof trajectories into `spodeian-lake` Parquet / JSON for MLOps re-training.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn stream_trajectories_to_lake(
        &self,
        lake_path: &Path,
        results: &[MctsSearchResult],
    ) -> Result<PathBuf, String> {
        use std::fs;
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let out_dir = lake_path.join("mcts").join("trajectories");
        fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;

        let file_path = out_dir.join(format!("mcts_proofs_{}.json", now));
        let json = serde_json::to_string_pretty(results).map_err(|e| e.to_string())?;
        fs::write(&file_path, json).map_err(|e| e.to_string())?;

        Ok(file_path)
    }
}
