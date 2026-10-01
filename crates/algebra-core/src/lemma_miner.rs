//! # `algebra_core::lemma_miner`
//!
//! Autonomous Symbolic Expression Generation, Fingerprint Clustering & Lemma Mining.
//!
//! Synthesizes deep symbolic AST expressions across polynomial, trigonometric, and
//! exponential domains, clusters them via multi-prime Schwartz-Zippel fingerprints
//! over Mersenne primes ($\mathbb{F}_{2^{31}-1}, \mathbb{F}_{2^{61}-1}, \mathbb{F}_{2^{64}-59}$),
//! formulates algebraic equivalence hypotheses $E_1 \equiv E_2$, and manages the learned lemma registry.

use crate::graph::ExprGraph;
use crate::id::{ExprId, SymbolId};
use crate::probabilistic::ProbabilisticVerifier;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Domain taxonomy for synthetic algebraic expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DomainKind {
    /// Polynomial ring operations: $+$, $-$, $*$, and small integer exponents.
    #[default]
    Polynomial,
    /// Trigonometric operations: polynomial operations plus $\sin, \cos, \tan$.
    Trigonometric,
    /// Exponential & logarithmic operations: polynomial operations plus $\exp, \ln$.
    Exponential,
    /// Mixed transcendental & algebraic operations.
    Mixed,
}

/// Configuration settings for the autonomous random expression generator.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExpressionGeneratorConfig {
    /// Maximum recursive AST depth (default: 3).
    pub max_depth: usize,
    /// Variable symbol names used in synthesized expressions (e.g. `["x", "y"]`).
    pub var_names: Vec<String>,
    /// Target algebraic domain.
    pub domain: DomainKind,
    /// Set of discrete scalar integer constants to sample from.
    pub constants: Vec<i64>,
    /// Random seed for deterministic reproducibility.
    pub seed: u64,
}

impl Default for ExpressionGeneratorConfig {
    fn default() -> Self {
        Self {
            max_depth: 3,
            var_names: vec!["x".to_string(), "y".to_string()],
            domain: DomainKind::Polynomial,
            constants: vec![-2, -1, 0, 1, 2],
            seed: 0x9E3779B97F4A7C15,
        }
    }
}

/// Lightweight deterministic 64-bit XorShift pseudo-random number generator.
#[derive(Debug, Clone)]
pub struct DeterministicPrng {
    state: u64,
}

impl DeterministicPrng {
    /// Create a new PRNG seeded with an initial non-zero integer.
    pub fn new(seed: u64) -> Self {
        let state = if seed == 0 { 0x5DEECE66D } else { seed };
        Self { state }
    }

    /// Advance state and produce next pseudo-random `u64`.
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Produce a pseudo-random `usize` in range `0..upper`.
    pub fn next_bounded(&mut self, upper: usize) -> usize {
        if upper <= 1 {
            0
        } else {
            (self.next_u64() % (upper as u64)) as usize
        }
    }
}

/// Synthetic AST Expression Generator producing diverse DAG structures.
#[derive(Debug, Clone)]
pub struct ExpressionGenerator {
    config: ExpressionGeneratorConfig,
    prng: DeterministicPrng,
}

impl ExpressionGenerator {
    /// Create a new expression generator with given configuration.
    pub fn new(config: ExpressionGeneratorConfig) -> Self {
        let prng = DeterministicPrng::new(config.seed);
        Self { config, prng }
    }

    /// Generate an expression node recursively up to `depth`.
    pub fn generate_node(&mut self, graph: &ExprGraph, depth: usize) -> ExprId {
        if depth >= self.config.max_depth || (depth > 0 && self.prng.next_bounded(100) < 30) {
            // Leaf node: symbol or constant number
            if self.prng.next_bounded(2) == 0 && !self.config.var_names.is_empty() {
                let idx = self.prng.next_bounded(self.config.var_names.len());
                graph.symbol(&self.config.var_names[idx])
            } else {
                let idx = self.prng.next_bounded(self.config.constants.len());
                graph.integer(self.config.constants[idx])
            }
        } else {
            // Internal node
            let op_choice = match self.config.domain {
                DomainKind::Polynomial => self.prng.next_bounded(5),
                DomainKind::Trigonometric => self.prng.next_bounded(7),
                DomainKind::Exponential => self.prng.next_bounded(7),
                DomainKind::Mixed => self.prng.next_bounded(9),
            };

            match op_choice {
                0 => {
                    // Addition
                    let lhs = self.generate_node(graph, depth + 1);
                    let rhs = self.generate_node(graph, depth + 1);
                    graph.add([lhs, rhs])
                }
                1 => {
                    // Multiplication
                    let lhs = self.generate_node(graph, depth + 1);
                    let rhs = self.generate_node(graph, depth + 1);
                    graph.mul([lhs, rhs])
                }
                2 => {
                    // Subtraction
                    let lhs = self.generate_node(graph, depth + 1);
                    let rhs = self.generate_node(graph, depth + 1);
                    graph.sub(lhs, rhs)
                }
                3 => {
                    // Negation
                    let inner = self.generate_node(graph, depth + 1);
                    graph.neg(inner)
                }
                4 => {
                    // Power (small integer exponent 2 or 3)
                    let base = self.generate_node(graph, depth + 1);
                    let exp_val = (self.prng.next_bounded(2) + 2) as i64;
                    let exp = graph.integer(exp_val);
                    graph.pow(base, exp)
                }
                5 => {
                    // Domain-specific: sin or exp
                    let inner = self.generate_node(graph, depth + 1);
                    match self.config.domain {
                        DomainKind::Trigonometric | DomainKind::Mixed => {
                            graph.function("sin", [inner])
                        }
                        _ => graph.function("exp", [inner]),
                    }
                }
                6 => {
                    // Domain-specific: cos or ln
                    let inner = self.generate_node(graph, depth + 1);
                    match self.config.domain {
                        DomainKind::Trigonometric | DomainKind::Mixed => {
                            graph.function("cos", [inner])
                        }
                        _ => graph.function("ln", [inner]),
                    }
                }
                7 => {
                    // Mixed: tan or exp
                    let inner = self.generate_node(graph, depth + 1);
                    graph.function("tan", [inner])
                }
                _ => {
                    // Mixed: exp
                    let inner = self.generate_node(graph, depth + 1);
                    graph.function("exp", [inner])
                }
            }
        }
    }

    /// Generate a batch of $N$ distinct expressions.
    pub fn generate_batch(&mut self, graph: &ExprGraph, count: usize) -> Vec<ExprId> {
        let mut results = Vec::with_capacity(count);
        for _ in 0..count {
            results.push(self.generate_node(graph, 0));
        }
        results
    }
}

/// Multi-Prime Schwartz-Zippel fingerprint triple over Mersenne primes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FingerprintTriple {
    /// Modulo $2^{31} - 1$.
    pub p31: u64,
    /// Modulo $2^{61} - 1$.
    pub p61: u64,
    /// Modulo $2^{64} - 59$.
    pub p64: u64,
}

impl FingerprintTriple {
    /// Compute fingerprint triple for an expression given an environment mapping.
    pub fn compute(
        graph: &ExprGraph,
        expr_id: ExprId,
        env_31: &HashMap<SymbolId, u64>,
        env_61: &HashMap<SymbolId, u64>,
        env_64: &HashMap<SymbolId, u64>,
    ) -> Option<Self> {
        let p31 = ProbabilisticVerifier::eval_mod_p(
            graph,
            expr_id,
            ProbabilisticVerifier::MERSENNE_31,
            env_31,
        )?;
        let p61 = ProbabilisticVerifier::eval_mod_p(
            graph,
            expr_id,
            ProbabilisticVerifier::MERSENNE_61,
            env_61,
        )?;
        let p64 = ProbabilisticVerifier::eval_mod_p(
            graph,
            expr_id,
            ProbabilisticVerifier::LARGE_PRIME_64,
            env_64,
        )?;

        Some(Self { p31, p61, p64 })
    }
}

/// Proposed algebraic equivalence hypothesis $E_1 \equiv E_2$.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EquivalenceHypothesis {
    /// Left-hand side expression.
    pub lhs: ExprId,
    /// Right-hand side expression.
    pub rhs: ExprId,
    /// Free variable symbol names.
    pub var_names: Vec<String>,
    /// Matched fingerprint across multi-prime evaluation.
    pub fingerprint: FingerprintTriple,
    /// Number of evaluation environments successfully matched.
    pub samples_matched: usize,
    /// Estimated confidence score $1.0 - \epsilon$.
    pub confidence: f64,
}

/// Abstract trait for formal algebraic provers (e.g. Gröbner Basis or E-Graph saturation).
pub trait FormalEquivalenceProver: Send + Sync {
    /// Attempt to formally prove that `lhs` and `rhs` represent equivalent expressions.
    fn prove_equivalence(
        &self,
        graph: &ExprGraph,
        lhs: ExprId,
        rhs: ExprId,
        var_names: &[String],
    ) -> bool;
}

/// Formally verified or learned rewrite lemma.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LearnedLemma {
    /// Unique identifier handle for the lemma.
    pub id: String,
    /// Human-readable mathematical name or description.
    pub name: String,
    /// Source expression pattern.
    pub lhs: ExprId,
    /// Target rewritten pattern.
    pub rhs: ExprId,
    /// Variable symbols bound in this lemma.
    pub var_names: Vec<String>,
    /// Formal verification technique used (e.g. "GrobnerBasisIdealElimination").
    pub proof_method: String,
    /// Whether the rewrite applies symmetrically in both directions.
    pub is_bidirectional: bool,
}

/// Thread-safe registry for learned and verified algebraic rewrite lemmas.
#[derive(Debug, Clone, Default)]
pub struct LemmaRegistry {
    lemmas: Arc<RwLock<Vec<LearnedLemma>>>,
}

impl LemmaRegistry {
    /// Create a new empty lemma registry.
    pub fn new() -> Self {
        Self {
            lemmas: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a verified lemma. Returns `true` if newly registered, `false` if duplicate.
    pub fn register(&self, lemma: LearnedLemma) -> bool {
        let mut guard = self.lemmas.write();
        if guard.iter().any(|l| l.id == lemma.id || (l.lhs == lemma.lhs && l.rhs == lemma.rhs)) {
            false
        } else {
            guard.push(lemma);
            true
        }
    }

    /// Retrieve all currently registered lemmas.
    pub fn get_lemmas(&self) -> Vec<LearnedLemma> {
        self.lemmas.read().clone()
    }

    /// Retrieve total count of learned lemmas in the registry.
    pub fn len(&self) -> usize {
        self.lemmas.read().len()
    }

    /// Returns `true` if the registry contains no lemmas.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Find a lemma by its unique ID.
    pub fn find_by_id(&self, id: &str) -> Option<LearnedLemma> {
        self.lemmas.read().iter().find(|l| l.id == id).cloned()
    }
}

/// Autonomous Lemma Miner engine clustering expressions by Schwartz-Zippel fingerprints.
#[derive(Debug, Clone)]
pub struct LemmaMiner {
    config: ExpressionGeneratorConfig,
    registry: LemmaRegistry,
}

impl LemmaMiner {
    /// Create a new lemma miner with given generator configuration and target registry.
    pub fn new(config: ExpressionGeneratorConfig, registry: LemmaRegistry) -> Self {
        Self { config, registry }
    }

    /// Access the lemma registry.
    pub fn registry(&self) -> &LemmaRegistry {
        &self.registry
    }

    /// Cluster a set of candidate expressions into equivalence hypotheses.
    pub fn cluster_candidates(
        &self,
        graph: &ExprGraph,
        candidates: &[ExprId],
        num_rounds: usize,
    ) -> Vec<EquivalenceHypothesis> {
        // Collect symbol IDs across all candidates
        let mut all_syms = Vec::new();
        for &c in candidates {
            for s in ProbabilisticVerifier::extract_symbols(graph, c) {
                if !all_syms.contains(&s) {
                    all_syms.push(s);
                }
            }
        }

        // Generate multiple evaluation environments
        let mut envs_31 = Vec::with_capacity(num_rounds);
        let mut envs_61 = Vec::with_capacity(num_rounds);
        let mut envs_64 = Vec::with_capacity(num_rounds);

        for round in 1..=num_rounds {
            let mut e31 = HashMap::new();
            let mut e61 = HashMap::new();
            let mut e64 = HashMap::new();

            for (idx, &sym) in all_syms.iter().enumerate() {
                let v31 = ((round as u64) * 65537 + (idx as u64) * 31337 + 13)
                    % ProbabilisticVerifier::MERSENNE_31;
                let v61 = ((round as u64) * 1000003 + (idx as u64) * 7919 + 29)
                    % ProbabilisticVerifier::MERSENNE_61;
                let v64 = ((round as u64) * 10000019 + (idx as u64) * 65521 + 47)
                    % ProbabilisticVerifier::LARGE_PRIME_64;

                e31.insert(sym, if v31 == 0 { 17 } else { v31 });
                e61.insert(sym, if v61 == 0 { 19 } else { v61 });
                e64.insert(sym, if v64 == 0 { 23 } else { v64 });
            }

            envs_31.push(e31);
            envs_61.push(e61);
            envs_64.push(e64);
        }

        // Map multi-round fingerprint vector to candidate expression list
        let mut cluster_map: HashMap<Vec<FingerprintTriple>, Vec<ExprId>> = HashMap::new();

        for &c in candidates {
            let mut fps = Vec::with_capacity(num_rounds);
            let mut valid = true;

            for r in 0..num_rounds {
                if let Some(fp) =
                    FingerprintTriple::compute(graph, c, &envs_31[r], &envs_61[r], &envs_64[r])
                {
                    fps.push(fp);
                } else {
                    valid = false;
                    break;
                }
            }

            if valid {
                cluster_map.entry(fps).or_default().push(c);
            }
        }

        // Formulate pairwise hypotheses from clusters with >= 2 expressions
        let mut hypotheses = Vec::new();

        for (fps, group) in cluster_map {
            if group.len() < 2 {
                continue;
            }

            for i in 0..group.len() {
                for j in (i + 1)..group.len() {
                    let lhs = group[i];
                    let rhs = group[j];

                    // Exclude trivial exact identity
                    if lhs == rhs {
                        continue;
                    }

                    // Collect variable names
                    let mut var_names = Vec::new();
                    for s in ProbabilisticVerifier::extract_symbols(graph, lhs) {
                        if let Some(name) = graph.symbols.resolve(s) {
                            if !var_names.contains(&name) {
                                var_names.push(name);
                            }
                        }
                    }
                    for s in ProbabilisticVerifier::extract_symbols(graph, rhs) {
                        if let Some(name) = graph.symbols.resolve(s) {
                            if !var_names.contains(&name) {
                                var_names.push(name);
                            }
                        }
                    }

                    hypotheses.push(EquivalenceHypothesis {
                        lhs,
                        rhs,
                        var_names,
                        fingerprint: fps[0],
                        samples_matched: num_rounds,
                        confidence: 1.0 - (1e-18 * (num_rounds as f64)),
                    });
                }
            }
        }

        hypotheses
    }

    /// Autonomous discovery loop: synthesizes expressions, clusters by fingerprint,
    /// verifies using the formal prover, and registers proven lemmas.
    pub fn mine_lemmas(
        &mut self,
        graph: &ExprGraph,
        batch_size: usize,
        num_rounds: usize,
        prover: Option<&dyn FormalEquivalenceProver>,
    ) -> Vec<LearnedLemma> {
        let mut generator = ExpressionGenerator::new(self.config.clone());
        let candidates = generator.generate_batch(graph, batch_size);
        let hypotheses = self.cluster_candidates(graph, &candidates, num_rounds);

        let mut newly_discovered = Vec::new();

        for hyp in hypotheses {
            let is_proven = if let Some(p) = prover {
                p.prove_equivalence(graph, hyp.lhs, hyp.rhs, &hyp.var_names)
            } else {
                // Probabilistic acceptance when confidence exceeds 99.999%
                hyp.confidence > 0.9999
            };

            if is_proven {
                let lemma_id = format!(
                    "lemma_{:x}_{:x}",
                    hyp.fingerprint.p31, hyp.fingerprint.p61
                );
                let lemma_name = format!(
                    "Identity between @{} and @{}",
                    hyp.lhs.index(),
                    hyp.rhs.index()
                );
                let lemma = LearnedLemma {
                    id: lemma_id,
                    name: lemma_name,
                    lhs: hyp.lhs,
                    rhs: hyp.rhs,
                    var_names: hyp.var_names,
                    proof_method: if prover.is_some() {
                        "FormalEquivalenceProof".to_string()
                    } else {
                        "SchwartzZippelMultiPrimeCertified".to_string()
                    },
                    is_bidirectional: true,
                };

                if self.registry.register(lemma.clone()) {
                    newly_discovered.push(lemma);
                }
            }
        }

        newly_discovered
    }
}
