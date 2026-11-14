//! Speculative E-Graph saturation with 3-tier rule scheduling.
//!
//! - Tier A: Zero-Expansion Structural Contractions.
//! - Tier B: Equivalence Explorers (algebraic identities).
//! - Tier C: Legacy Domain Heuristics (goal-directed monotonic FLOP reducers).

use crate::lattice::{EquivalenceEdge, EquivalenceKind};
use smallvec::SmallVec;

/// Rule schedule tier.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RuleTier {
    /// Zero-expansion rules that strictly reduce wire count or node complexity.
    TierAContractions,
    /// Equivalence explorers (associativity, commutativity, Lie-Clifford bridges).
    TierBExplorers,
    /// Goal-directed heuristics that monotonic cost decrease.
    TierCHeuristics,
}

/// Abstract rewrite rule in the e-graph.
pub struct RewriteRule {
    pub name: &'static str,
    pub tier: RuleTier,
    pub apply: Box<dyn Fn(&mut EGraph) -> usize + Send + Sync>,
}

/// E-node representing an algebraic operator in an e-class.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ENode {
    Scalar(i64),
    Var(String),
    Add(SmallVec<[u32; 2]>),
    Mul(SmallVec<[u32; 2]>),
    Neg(u32),
    Pow(u32, u32),
}

/// E-class grouping equivalent e-nodes.
#[derive(Debug, Clone, Default)]
pub struct EClass {
    pub id: u32,
    pub nodes: Vec<ENode>,
}

/// Categorical E-Graph data structure.
#[derive(Default)]
pub struct EGraph {
    pub classes: Vec<EClass>,
    pub union_find: Vec<u32>,
    pub edges: Vec<EquivalenceEdge>,
}

impl EGraph {
    /// Create empty e-graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new e-node to the graph, returning its e-class ID.
    pub fn add(&mut self, node: ENode) -> u32 {
        let id = self.classes.len() as u32;
        self.classes.push(EClass {
            id,
            nodes: vec![node],
        });
        self.union_find.push(id);
        id
    }

    /// Find canonical root e-class with path compression.
    pub fn find(&mut self, mut id: u32) -> u32 {
        let mut root = id;
        while self.union_find[root as usize] != root {
            root = self.union_find[root as usize];
        }
        while id != root {
            let next = self.union_find[id as usize];
            self.union_find[id as usize] = root;
            id = next;
        }
        root
    }

    /// Union two e-classes with strong or weak equivalence classification.
    pub fn union(&mut self, id1: u32, id2: u32, kind: EquivalenceKind, desc: &str) -> u32 {
        let root1 = self.find(id1);
        let root2 = self.find(id2);
        if root1 != root2 {
            self.union_find[root2 as usize] = root1;
            let mut nodes2 = std::mem::take(&mut self.classes[root2 as usize].nodes);
            self.classes[root1 as usize].nodes.append(&mut nodes2);
            self.edges.push(EquivalenceEdge {
                from_class: root1,
                to_class: root2,
                kind,
                description: desc.to_string(),
            });
        }
        root1
    }

    /// Run saturation pass using scheduled rule tiers.
    pub fn run_saturation(&mut self, rules: &[RewriteRule], max_iterations: usize) -> usize {
        let mut total_rewrites = 0;
        for _ in 0..max_iterations {
            let mut iteration_rewrites = 0;
            // Execute Tier A contractions first
            for r in rules.iter().filter(|r| r.tier == RuleTier::TierAContractions) {
                iteration_rewrites += (r.apply)(self);
            }
            // Execute Tier B explorers
            for r in rules.iter().filter(|r| r.tier == RuleTier::TierBExplorers) {
                iteration_rewrites += (r.apply)(self);
            }
            total_rewrites += iteration_rewrites;
            if iteration_rewrites == 0 {
                break;
            }
        }
        total_rewrites
    }

    /// Extract FLOP-optimal AST node for given e-class.
    pub fn extract_min_cost(&mut self, root_class: u32) -> (ENode, usize) {
        let root = self.find(root_class);
        let nodes = &self.classes[root as usize].nodes;

        let mut best_node = nodes[0].clone();
        let mut min_cost = usize::MAX;

        for node in nodes {
            let cost = match node {
                ENode::Scalar(_) => 0,
                ENode::Var(_) => 1,
                ENode::Neg(_) => 1,
                ENode::Add(children) => 1 + children.len(),
                ENode::Mul(children) => 2 + children.len() * 2,
                ENode::Pow(_, _) => 5,
            };
            if cost < min_cost {
                min_cost = cost;
                best_node = node.clone();
            }
        }

        (best_node, min_cost)
    }
}
