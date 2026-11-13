//! # `algebra_engine::lemma_miner`
//!
//! Autonomous Algebraic Rewrite Lemma Mining with Gröbner Basis Formal Verification.
//!
//! Orchestrates synthetic expression generation, Schwartz-Zippel fingerprint clustering,
//! Gröbner basis ideal reduction to formally prove equivalence hypotheses $E_1 \equiv E_2$,
//! and streams theorem trajectories to `spodeian-lake` for offline `spodeian-ml` self-improvement.

use crate::poly::grobner::GrobnerBasis;
use crate::poly::multivariate::{MonomialOrder, MultiPoly};
use crate::simplify::AlgebraicTransformations;
use algebra_core::lemma_miner::{
    ExpressionGeneratorConfig, FormalEquivalenceProver, LearnedLemma, LemmaMiner, LemmaRegistry,
};
use algebra_core::{ExprGraph, ExprId, ExprKind};
use std::path::{Path, PathBuf};

/// Formal algebraic equivalence prover backed by polynomial expansion and Gröbner basis ideal reduction.
#[derive(Debug, Clone, Copy, Default)]
pub struct GrobnerProver {
    order: MonomialOrder,
}

impl GrobnerProver {
    /// Create a new Gröbner basis prover with given monomial ordering (default: GrevLex).
    pub fn new(order: MonomialOrder) -> Self {
        Self { order }
    }
}

impl FormalEquivalenceProver for GrobnerProver {
    fn prove_equivalence(
        &self,
        graph: &ExprGraph,
        lhs: ExprId,
        rhs: ExprId,
        var_names: &[String],
    ) -> bool {
        // 1. Syntactic identity
        if lhs == rhs {
            return true;
        }

        // 2. Direct numeric scalar equality
        let l_node = graph.get(lhs);
        let r_node = graph.get(rhs);
        if let (ExprKind::Number(nl), ExprKind::Number(nr)) = (&l_node.kind, &r_node.kind) {
            if nl == nr {
                return true;
            }
        }

        // 3. Difference expansion
        let diff = graph.sub(lhs, rhs);
        let expanded = AlgebraicTransformations::expand(graph, diff);
        let exp_node = graph.get(expanded);
        if let ExprKind::Number(n) = &exp_node.kind {
            if n.is_zero() {
                return true;
            }
        }

        // If no variables are specified, collect free symbols
        let vars: Vec<String> = if var_names.is_empty() {
            let mut v = Vec::new();
            for s in
                algebra_core::probabilistic::ProbabilisticVerifier::extract_symbols(graph, diff)
            {
                if let Some(name) = graph.symbols.resolve(s) {
                    if !v.contains(&name) {
                        v.push(name);
                    }
                }
            }
            v
        } else {
            var_names.to_vec()
        };

        // 4. Multivariate polynomial canonical form check
        if let Ok(poly) = MultiPoly::from_expr(graph, expanded, &vars, self.order) {
            if poly.is_zero() {
                return true;
            }
        }

        // 5. Check modulo algebraic relations (e.g., standard polynomial ideal reduction)
        // If difference polynomial reduces to zero in empty ideal
        if let Ok(basis) = GrobnerBasis::from_exprs(graph, &[], &vars, self.order) {
            if let Ok(is_member) = basis.contains_expr(graph, diff) {
                if is_member {
                    return true;
                }
            }
        }

        false
    }
}

/// Record representing a mined theorem candidate and its verification status.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MinedTheoremRecord {
    /// LHS expression index or string.
    pub lhs_index: u32,
    /// RHS expression index or string.
    pub rhs_index: u32,
    /// Human-readable description.
    pub description: String,
    /// Whether formal equivalence was proven.
    pub proven: bool,
    /// Verification algorithm name.
    pub proof_method: String,
    /// Number of free variables.
    pub num_vars: usize,
    /// Confidence metric (1.0 for formal proof).
    pub confidence: f64,
}

/// Autonomous Lemma Mining Pipeline coordinating generation, proof, and telemetry.
#[derive(Debug, Clone)]
pub struct AutonomousLemmaMiningPipeline {
    miner: LemmaMiner,
    prover: GrobnerProver,
}

impl AutonomousLemmaMiningPipeline {
    /// Create a new autonomous lemma mining pipeline.
    pub fn new(config: ExpressionGeneratorConfig, registry: LemmaRegistry) -> Self {
        let miner = LemmaMiner::new(config, registry);
        let prover = GrobnerProver::new(MonomialOrder::GrevLex);
        Self { miner, prover }
    }

    /// Access the underlying lemma registry.
    pub fn registry(&self) -> &LemmaRegistry {
        self.miner.registry()
    }

    /// Execute an autonomous mining iteration with Gröbner basis formal verification.
    pub fn mine_batch(
        &mut self,
        graph: &ExprGraph,
        batch_size: usize,
        num_rounds: usize,
    ) -> Vec<LearnedLemma> {
        self.miner
            .mine_lemmas(graph, batch_size, num_rounds, Some(&self.prover))
    }

    /// Stream verified lemma discovery records to a JSON/Parquet Lake dataset.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn export_records_to_lake(
        &self,
        lake_path: &Path,
        records: &[MinedTheoremRecord],
    ) -> Result<PathBuf, String> {
        use std::fs;
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let out_dir = lake_path.join("theorems").join("trajectories");
        fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;

        let file_path = out_dir.join(format!("theorems_{}.json", now));
        let json = serde_json::to_string_pretty(records).map_err(|e| e.to_string())?;
        fs::write(&file_path, json).map_err(|e| e.to_string())?;

        Ok(file_path)
    }
}
