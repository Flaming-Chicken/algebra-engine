//! Wavefront level-synchronous parallel hypergraph evaluator.
//!
//! Partitions hyperedges into topological dependency levels and evaluates
//! independent operations in parallel via `rayon`.

use cat_core::handles::WireId;
use cat_core::hypergraph::{HyperEdge, HyperOp, OpenHypergraph};
use cat_core::scalar::ScalarAtom;
use rayon::prelude::*;
use smallvec::SmallVec;
use std::collections::HashMap;

/// Wavefront execution level containing edges whose inputs are simultaneously available.
#[derive(Debug, Clone)]
pub struct WavefrontLevel {
    pub edges: Vec<HyperEdge>,
}

/// Wavefront schedule for parallel level-by-level evaluation.
#[derive(Debug, Clone)]
pub struct WavefrontSchedule {
    pub levels: Vec<WavefrontLevel>,
}

impl WavefrontSchedule {
    /// Construct a topological level-synchronous schedule from an open hypergraph.
    pub fn build(hypergraph: &OpenHypergraph) -> Self {
        let mut wire_ready_at_level: HashMap<WireId, usize> = HashMap::new();

        // Boundary input wires are available at level 0
        for port in &hypergraph.boundary_ports {
            if port.polarity == cat_core::PortPolarity::Input {
                wire_ready_at_level.insert(port.wire, 0);
            }
        }

        let mut edge_levels: Vec<usize> = vec![0; hypergraph.edges.len()];
        let mut max_level = 0;

        for (i, edge) in hypergraph.edges.iter().enumerate() {
            let mut req_level = 0;
            for &in_w in &edge.inputs {
                if let Some(&lvl) = wire_ready_at_level.get(&in_w) {
                    req_level = req_level.max(lvl + 1);
                }
            }
            edge_levels[i] = req_level;
            max_level = max_level.max(req_level);

            for &out_w in &edge.outputs {
                wire_ready_at_level.insert(out_w, req_level);
            }
        }

        let mut levels = vec![WavefrontLevel { edges: Vec::new() }; max_level + 1];
        for (i, edge) in hypergraph.edges.iter().enumerate() {
            levels[edge_levels[i]].edges.push(edge.clone());
        }

        Self { levels }
    }

    /// Execute the schedule evaluating wires using parallel Rayon tasks at each level.
    pub fn evaluate(
        &self,
        initial_wires: HashMap<WireId, ScalarAtom>,
    ) -> HashMap<WireId, ScalarAtom> {
        let mut wire_values = initial_wires;

        for level in &self.levels {
            // Evaluate independent edges in parallel
            let level_results: Vec<(SmallVec<[WireId; 4]>, ScalarAtom)> = level
                .edges
                .par_iter()
                .map(|edge| {
                    let mut input_atoms: SmallVec<[ScalarAtom; 4]> = SmallVec::new();
                    for &in_w in &edge.inputs {
                        if let Some(val) = wire_values.get(&in_w) {
                            input_atoms.push(val.clone());
                        } else {
                            input_atoms.push(ScalarAtom::Zero);
                        }
                    }

                    let result = match &edge.op {
                        HyperOp::Nullary(sid) => ScalarAtom::Integer(sid.0 as i64),
                        HyperOp::Add => {
                            let mut acc = ScalarAtom::Zero;
                            for a in &input_atoms {
                                acc = cat_core::ScalarLattice::add(&acc, a);
                            }
                            acc
                        }
                        HyperOp::Mul => {
                            let mut acc = ScalarAtom::One;
                            for a in &input_atoms {
                                acc = cat_core::ScalarLattice::mul(&acc, a);
                            }
                            acc
                        }
                        HyperOp::Neg => {
                            if let Some(first) = input_atoms.first() {
                                cat_core::ScalarLattice::mul(&ScalarAtom::Integer(-1), first)
                            } else {
                                ScalarAtom::Zero
                            }
                        }
                        _ => ScalarAtom::Zero,
                    };

                    (edge.outputs.clone(), result)
                })
                .collect();

            // Store level outputs into wire environment
            for (outputs, val) in level_results {
                for out_w in outputs {
                    wire_values.insert(out_w, val.clone());
                }
            }
        }

        wire_values
    }
}
