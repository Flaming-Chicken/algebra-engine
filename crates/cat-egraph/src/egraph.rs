//! Speculative E-Graph saturation with 3-tier rule scheduling.
//!
//! - Tier A: Zero-Expansion Structural Contractions.
//! - Tier B: Equivalence Explorers (algebraic identities).
//! - Tier C: Legacy Domain Heuristics (goal-directed monotonic FLOP reducers).

use crate::lattice::{EquivalenceEdge, EquivalenceKind};
use smallvec::SmallVec;
use std::collections::HashMap;

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

/// A fully extracted term: the cheapest expression an e-class represents, with the children
/// resolved recursively (an [`ENode`] only refers to child e-classes by id).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractedTerm {
    Scalar(i64),
    Var(String),
    Add(Vec<ExtractedTerm>),
    Mul(Vec<ExtractedTerm>),
    Neg(Box<ExtractedTerm>),
    Pow(Box<ExtractedTerm>, Box<ExtractedTerm>),
}

/// Categorical E-Graph data structure.
///
/// `add` hash-conses: adding a node that already exists (up to the current equivalence of its
/// children) returns the existing e-class. `union` merges classes and [`EGraph::rebuild`]
/// restores congruence (if `a = b` then `f(a) = f(b)`); `run_saturation` rebuilds after every
/// round of rewrites.
#[derive(Default)]
pub struct EGraph {
    pub classes: Vec<EClass>,
    pub union_find: Vec<u32>,
    pub edges: Vec<EquivalenceEdge>,
    memo: HashMap<ENode, u32>,
}

impl EGraph {
    /// Create empty e-graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// The node with every child replaced by its canonical e-class.
    fn canonical(&mut self, node: &ENode) -> ENode {
        match node {
            ENode::Scalar(_) | ENode::Var(_) => node.clone(),
            ENode::Add(children) => ENode::Add(children.iter().map(|&c| self.find(c)).collect()),
            ENode::Mul(children) => ENode::Mul(children.iter().map(|&c| self.find(c)).collect()),
            ENode::Neg(child) => ENode::Neg(self.find(*child)),
            ENode::Pow(base, exponent) => ENode::Pow(self.find(*base), self.find(*exponent)),
        }
    }

    /// Add an e-node to the graph, returning its e-class ID. A node equal to an existing one
    /// (after canonicalising its children) returns that node's class instead of making a new one.
    pub fn add(&mut self, node: ENode) -> u32 {
        let node = self.canonical(&node);
        if let Some(&existing) = self.memo.get(&node) {
            return self.find(existing);
        }
        let id = self.classes.len() as u32;
        self.classes.push(EClass {
            id,
            nodes: vec![node.clone()],
        });
        self.union_find.push(id);
        self.memo.insert(node, id);
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
            for r in rules
                .iter()
                .filter(|r| r.tier == RuleTier::TierAContractions)
            {
                iteration_rewrites += (r.apply)(self);
            }
            // Execute Tier B explorers
            for r in rules.iter().filter(|r| r.tier == RuleTier::TierBExplorers) {
                iteration_rewrites += (r.apply)(self);
            }
            total_rewrites += iteration_rewrites;
            self.rebuild();
            if iteration_rewrites == 0 {
                break;
            }
        }
        total_rewrites
    }

    /// Restore the hash-cons and congruence invariants after unions: every node's children are
    /// replaced by their canonical classes, duplicate nodes inside a class are dropped, and two
    /// classes that contain the same canonical node are merged (repeating until stable).
    pub fn rebuild(&mut self) {
        loop {
            let mut merges: Vec<(u32, u32)> = Vec::new();
            self.memo.clear();
            let roots: Vec<u32> = (0..self.classes.len() as u32)
                .filter(|&i| self.union_find[i as usize] == i)
                .collect();
            for root in roots {
                let nodes = std::mem::take(&mut self.classes[root as usize].nodes);
                let mut canonical: Vec<ENode> = Vec::with_capacity(nodes.len());
                for node in &nodes {
                    let node = self.canonical(node);
                    if canonical.contains(&node) {
                        continue;
                    }
                    let existing = self.memo.get(&node).copied();
                    match existing {
                        Some(other) if self.find(other) != root => merges.push((root, other)),
                        _ => {
                            self.memo.insert(node.clone(), root);
                        }
                    }
                    canonical.push(node);
                }
                self.classes[root as usize].nodes = canonical;
            }
            if merges.is_empty() {
                return;
            }
            for (a, b) in merges {
                self.union(
                    a,
                    b,
                    EquivalenceKind::strong_exact(true),
                    "congruence closure",
                );
            }
        }
    }

    /// Own cost of one operator, counting the operation itself and not its operands.
    fn node_cost(node: &ENode) -> usize {
        match node {
            ENode::Scalar(_) => 0,
            ENode::Var(_) => 1,
            ENode::Neg(_) => 1,
            ENode::Add(children) => 1 + children.len(),
            ENode::Mul(children) => 2 + children.len() * 2,
            ENode::Pow(_, _) => 5,
        }
    }

    /// Cheapest total cost and node for every class, found by iterating to a fixed point: a
    /// class's cost is its best node's own cost plus the cost of each child's cheapest form.
    /// Classes in a cycle with no finite way out have no entry.
    fn best_per_class(&mut self) -> HashMap<u32, (usize, ENode)> {
        self.rebuild();
        let roots: Vec<u32> = (0..self.classes.len() as u32)
            .filter(|&i| self.union_find[i as usize] == i)
            .collect();
        let mut best: HashMap<u32, (usize, ENode)> = HashMap::new();
        for _ in 0..=self.classes.len() {
            let mut changed = false;
            for &root in &roots {
                let nodes = self.classes[root as usize].nodes.clone();
                for node in nodes {
                    let children: Vec<u32> = match &node {
                        ENode::Scalar(_) | ENode::Var(_) => Vec::new(),
                        ENode::Add(c) | ENode::Mul(c) => c.iter().map(|&x| self.find(x)).collect(),
                        ENode::Neg(c) => vec![self.find(*c)],
                        ENode::Pow(a, b) => vec![self.find(*a), self.find(*b)],
                    };
                    let mut total = Self::node_cost(&node);
                    let mut finite = true;
                    for child in &children {
                        match best.get(child) {
                            Some((cost, _)) => total += cost,
                            None => {
                                finite = false;
                                break;
                            }
                        }
                    }
                    if finite && best.get(&root).is_none_or(|(cost, _)| total < *cost) {
                        best.insert(root, (total, node));
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        best
    }

    /// FLOP-optimal node for the given e-class together with the *total* cost of the cheapest
    /// expression rooted there (the node's own cost plus the cheapest form of each operand).
    /// The returned node's children are e-class ids; use [`EGraph::extract_term`] for the whole
    /// tree. A class with no finite expression (only cycles) returns its first node and
    /// `usize::MAX`.
    pub fn extract_min_cost(&mut self, root_class: u32) -> (ENode, usize) {
        let root = self.find(root_class);
        let best = self.best_per_class();
        match best.get(&root) {
            Some((cost, node)) => (node.clone(), *cost),
            None => (self.classes[root as usize].nodes[0].clone(), usize::MAX),
        }
    }

    /// The cheapest expression the e-class represents, as a tree, with its total cost.
    pub fn extract_term(&mut self, root_class: u32) -> Option<(ExtractedTerm, usize)> {
        let root = self.find(root_class);
        let best = self.best_per_class();
        let (cost, _) = best.get(&root)?;
        let cost = *cost;
        let term = self.build_term(root, &best)?;
        Some((term, cost))
    }

    fn build_term(
        &mut self,
        class: u32,
        best: &HashMap<u32, (usize, ENode)>,
    ) -> Option<ExtractedTerm> {
        let class = self.find(class);
        let (_, node) = best.get(&class)?.clone();
        Some(match node {
            ENode::Scalar(v) => ExtractedTerm::Scalar(v),
            ENode::Var(name) => ExtractedTerm::Var(name),
            ENode::Neg(child) => ExtractedTerm::Neg(Box::new(self.build_term(child, best)?)),
            ENode::Pow(base, exponent) => ExtractedTerm::Pow(
                Box::new(self.build_term(base, best)?),
                Box::new(self.build_term(exponent, best)?),
            ),
            ENode::Add(children) => ExtractedTerm::Add(
                children
                    .iter()
                    .map(|&c| self.build_term(c, best))
                    .collect::<Option<_>>()?,
            ),
            ENode::Mul(children) => ExtractedTerm::Mul(
                children
                    .iter()
                    .map(|&c| self.build_term(c, best))
                    .collect::<Option<_>>()?,
            ),
        })
    }
}
