//! # `algebra_engine::lemma_miner`
//!
//! Autonomous Algebraic Rewrite Lemma Mining with Gröbner Basis Formal Verification.
//!
//! Orchestrates synthetic expression generation, Schwartz-Zippel fingerprint clustering,
//! Gröbner basis ideal reduction to formally prove equivalence hypotheses $E_1 \equiv E_2$,
//! and streams theorem trajectories to `spodeian-lake` for offline `spodeian-ml` self-improvement.

use crate::poly::multivariate::{MonomialOrder, MultiPoly};
use crate::simplify::AlgebraicTransformations;
use algebra_core::lemma_miner::{
    ExpressionGeneratorConfig, FormalEquivalenceProver, LearnedLemma, LemmaMiner, LemmaRegistry,
};
use algebra_core::{ExprGraph, ExprId, ExprKind};
#[cfg(not(target_arch = "wasm32"))]
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

/// A transcendental subterm replaced by a fresh polynomial variable.
struct Atom {
    name: String,
    function: String,
    argument: ExprId,
}

impl GrobnerProver {
    /// Replace every `sin`, `cos`, `sinh`, `cosh` or `exp` application in `id` by a fresh
    /// variable, recording the substitution in `atoms` (equal subterms share a variable).
    fn abstract_atoms(graph: &ExprGraph, id: ExprId, atoms: &mut Vec<Atom>) -> ExprId {
        let node = graph.get(id);
        match &node.kind {
            ExprKind::Function { name, args } if args.len() == 1 => {
                let function = graph.symbols.resolve(*name).unwrap_or_default();
                if matches!(function.as_str(), "sin" | "cos" | "sinh" | "cosh" | "exp") {
                    let argument = args[0];
                    let existing = atoms
                        .iter()
                        .find(|a| a.function == function && a.argument == argument);
                    let name = match existing {
                        Some(atom) => atom.name.clone(),
                        None => {
                            let name = format!("_atom{}", atoms.len());
                            atoms.push(Atom {
                                name: name.clone(),
                                function,
                                argument,
                            });
                            name
                        }
                    };
                    return graph.symbol(&name);
                }
                id
            }
            ExprKind::Add(items) => {
                let parts: Vec<ExprId> = items
                    .iter()
                    .map(|&i| Self::abstract_atoms(graph, i, atoms))
                    .collect();
                graph.add(parts)
            }
            ExprKind::Mul(items) => {
                let parts: Vec<ExprId> = items
                    .iter()
                    .map(|&i| Self::abstract_atoms(graph, i, atoms))
                    .collect();
                graph.mul(parts)
            }
            ExprKind::Sub(l, r) => {
                let (l, r) = (
                    Self::abstract_atoms(graph, *l, atoms),
                    Self::abstract_atoms(graph, *r, atoms),
                );
                graph.sub(l, r)
            }
            ExprKind::Neg(inner) => {
                let inner = Self::abstract_atoms(graph, *inner, atoms);
                graph.neg(inner)
            }
            ExprKind::Pow(base, exponent) => {
                let base = Self::abstract_atoms(graph, *base, atoms);
                graph.pow(base, *exponent)
            }
            _ => id,
        }
    }

    /// Whether `a + b` is zero: one is the negation of the other, or their sum simplifies to 0.
    fn are_negatives(graph: &ExprGraph, a: ExprId, b: ExprId) -> bool {
        let negates = |outer: ExprId, inner: ExprId| matches!(&graph.get(outer).kind, ExprKind::Neg(x) if *x == inner);
        if negates(a, b) || negates(b, a) {
            return true;
        }
        let sum = graph.add([a, b]);
        let simplified = crate::simplify::Simplifier::simplify(graph, sum).unwrap_or(sum);
        matches!(&graph.get(simplified).kind, ExprKind::Number(n) if n.is_zero())
    }

    /// Decide `diff = 0` modulo the side relations of the atoms in it (see step 5 above).
    fn proved_modulo_side_relations(
        &self,
        graph: &ExprGraph,
        diff: ExprId,
        vars: &[String],
    ) -> bool {
        use crate::poly::exact::{ExactGrobner, ExactPoly};
        let mut atoms = Vec::new();
        let abstracted = Self::abstract_atoms(graph, diff, &mut atoms);
        if atoms.is_empty() {
            return false;
        }

        // Relations among the atoms that actually occur.
        let mut relations: Vec<ExprId> = Vec::new();
        let square = |name: &str| graph.pow(graph.symbol(name), graph.integer(2));
        for first in &atoms {
            for second in &atoms {
                if first.argument == second.argument {
                    if first.function == "sin" && second.function == "cos" {
                        relations.push(graph.sub(
                            graph.add([square(&first.name), square(&second.name)]),
                            graph.integer(1),
                        ));
                    }
                    if first.function == "cosh" && second.function == "sinh" {
                        relations.push(graph.sub(
                            graph.sub(square(&first.name), square(&second.name)),
                            graph.integer(1),
                        ));
                    }
                }
                if first.function == "exp" && second.function == "exp" && first.name < second.name {
                    if Self::are_negatives(graph, first.argument, second.argument) {
                        relations.push(graph.sub(
                            graph.mul([graph.symbol(&first.name), graph.symbol(&second.name)]),
                            graph.integer(1),
                        ));
                    }
                }
            }
        }

        let mut variables: Vec<String> = vars.to_vec();
        variables.extend(atoms.iter().map(|a| a.name.clone()));
        let expand = |e: ExprId| AlgebraicTransformations::expand(graph, e);
        let convert = |e: ExprId| -> Option<ExactPoly> {
            let poly = MultiPoly::from_expr(graph, expand(e), &variables, self.order).ok()?;
            ExactPoly::from_multi_poly(&poly)
        };
        let Some(target) = convert(abstracted) else {
            return false;
        };
        let Some(generators): Option<Vec<ExactPoly>> =
            relations.iter().map(|&r| convert(r)).collect()
        else {
            return false;
        };
        ExactGrobner::compute(&generators).contains(&target)
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

        // 5. Modulo the side relations of the transcendental atoms that occur: sin^2 + cos^2 = 1,
        //    cosh^2 - sinh^2 = 1 and exp(a) exp(-a) = 1. Each atom becomes a fresh variable and
        //    membership of the difference in the exact (rational) Groebner ideal decides the claim.
        if self.proved_modulo_side_relations(graph, diff, &vars) {
            return true;
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
