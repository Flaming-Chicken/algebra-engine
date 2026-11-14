//! Layer 2: Open hypergraphs with anonymous internal wire connections.
//!
//! Erases named dummy indices into coordinate-free, gauge-invariant wire links.
//! Includes 1-WL (Weisfeiler-Lehman) canonical coloring and port ordering.

use crate::handles::{ObjId, ScalarId, WireId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

/// Boundary port polarity: whether a port receives or emits information.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PortPolarity {
    Input,
    Output,
}

/// Boundary port exposing an external connection to an open hypergraph.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BoundaryPort {
    pub wire: WireId,
    pub polarity: PortPolarity,
    pub sort: ObjId,
}

/// Primitive hyper-operations executing at vertices in an open hypergraph.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HyperOp {
    /// Constant scalar generator: () \to (w).
    Nullary(ScalarId),
    /// Identity wire passing object sort without modification: (w) \to (w).
    Identity(ObjId),
    /// Addition of parallel wires.
    Add,
    /// Multiplication of parallel wires.
    Mul,
    /// Additive negation: (w) \to (-w).
    Neg,
    /// Multiplicative inverse: (w) \to (w^{-1}).
    Inv,
    /// Categorical composition / piping: f \circ g.
    Compose,
    /// Monoidal tensor product: f \otimes g.
    TensorProduct,
    /// Direct tensor contraction fusing pairs of input/output wires.
    Contraction {
        pairs: SmallVec<[(WireId, WireId); 2]>,
    },
    /// Formal derivation / tangent functor: \partial^n / \partial x^n.
    Derivation {
        wrt: WireId,
        order: u32,
    },
    /// Lie bracket morphism [A, B] = A B - B A.
    LieBracket,
    /// Clifford / Cayley-Dickson geometric product.
    CliffordGeometricProduct,
    /// External legacy symbol or unreduced function handle.
    External(u32),
}

/// Hyper-edge connecting multi-input wires to multi-output wires through a [`HyperOp`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HyperEdge {
    pub inputs: SmallVec<[WireId; 4]>,
    pub outputs: SmallVec<[WireId; 4]>,
    pub op: HyperOp,
}

impl HyperEdge {
    /// Create a new hyper-edge.
    pub fn new(inputs: SmallVec<[WireId; 4]>, outputs: SmallVec<[WireId; 4]>, op: HyperOp) -> Self {
        Self { inputs, outputs, op }
    }

    /// Create a nullary generator edge producing a single output wire from a scalar.
    pub fn nullary(out: WireId, scalar: ScalarId) -> Self {
        Self {
            inputs: SmallVec::new(),
            outputs: smallvec::smallvec![out],
            op: HyperOp::Nullary(scalar),
        }
    }

    /// Create a binary operation edge (e.g. Add, Mul).
    pub fn binary(in1: WireId, in2: WireId, out: WireId, op: HyperOp) -> Self {
        Self {
            inputs: smallvec::smallvec![in1, in2],
            outputs: smallvec::smallvec![out],
            op,
        }
    }

    /// Create a unary operation edge (e.g. Neg, Inv).
    pub fn unary(in_wire: WireId, out: WireId, op: HyperOp) -> Self {
        Self {
            inputs: smallvec::smallvec![in_wire],
            outputs: smallvec::smallvec![out],
            op,
        }
    }
}

/// Open hypergraph diagram representing categorical string diagrams with anonymous internal wires.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct OpenHypergraph {
    /// Boundary ports ordered canonically (inputs then outputs).
    pub boundary_ports: SmallVec<[BoundaryPort; 4]>,
    /// Directed hyperedges representing operations.
    pub edges: SmallVec<[HyperEdge; 8]>,
    /// Total number of unique wires allocated in this diagram.
    pub wire_count: u16,
}

impl OpenHypergraph {
    /// Create an empty hypergraph diagram.
    pub fn new() -> Self {
        Self {
            boundary_ports: SmallVec::new(),
            edges: SmallVec::new(),
            wire_count: 0,
        }
    }

    /// Allocate a fresh anonymous wire.
    pub fn alloc_wire(&mut self) -> WireId {
        let w = WireId(self.wire_count);
        self.wire_count += 1;
        w
    }

    /// Add an edge to the hypergraph.
    pub fn add_edge(&mut self, edge: HyperEdge) {
        self.edges.push(edge);
    }

    /// Add an input boundary port.
    pub fn add_input_port(&mut self, wire: WireId, sort: ObjId) {
        self.boundary_ports.push(BoundaryPort {
            wire,
            polarity: PortPolarity::Input,
            sort,
        });
    }

    /// Add an output boundary port.
    pub fn add_output_port(&mut self, wire: WireId, sort: ObjId) {
        self.boundary_ports.push(BoundaryPort {
            wire,
            polarity: PortPolarity::Output,
            sort,
        });
    }

    /// Canonicalize the open hypergraph using 1-WL (Weisfeiler-Lehman color refinement)
    /// to eliminate isomorphic permutations of internal wire IDs and edge ordering.
    pub fn canonicalize(&mut self) {
        if self.edges.is_empty() && self.boundary_ports.is_empty() {
            return;
        }

        // 1. Assign initial colors to wires based on boundary port appearance and incident edges
        let mut wire_colors: Vec<u64> = vec![0; self.wire_count as usize];
        for (idx, port) in self.boundary_ports.iter().enumerate() {
            let p_color = match port.polarity {
                PortPolarity::Input => 0x1000_0000,
                PortPolarity::Output => 0x2000_0000,
            } ^ ((idx as u64) << 16)
                ^ (port.sort.0 as u64);
            if (port.wire.0 as usize) < wire_colors.len() {
                wire_colors[port.wire.0 as usize] ^= p_color;
            }
        }

        // 2. Weisfeiler-Lehman 1-round refinement across incident hyperedges
        for edge in &self.edges {
            let op_hash = match &edge.op {
                HyperOp::Nullary(s) => 0xAA00 ^ (s.0 as u64),
                HyperOp::Identity(o) => 0xBB00 ^ (o.0 as u64),
                HyperOp::Add => 0xCC01,
                HyperOp::Mul => 0xCC02,
                HyperOp::Neg => 0xCC03,
                HyperOp::Inv => 0xCC04,
                HyperOp::Compose => 0xDD01,
                HyperOp::TensorProduct => 0xDD02,
                HyperOp::Contraction { .. } => 0xEE01,
                HyperOp::Derivation { order, .. } => 0xFF00 ^ (*order as u64),
                HyperOp::LieBracket => 0x1100,
                HyperOp::CliffordGeometricProduct => 0x2200,
                HyperOp::External(id) => 0x3300 ^ (*id as u64),
            };

            for &in_w in &edge.inputs {
                if (in_w.0 as usize) < wire_colors.len() {
                    wire_colors[in_w.0 as usize] = wire_colors[in_w.0 as usize]
                        .rotate_left(5)
                        ^ op_hash;
                }
            }
            for &out_w in &edge.outputs {
                if (out_w.0 as usize) < wire_colors.len() {
                    wire_colors[out_w.0 as usize] = wire_colors[out_w.0 as usize]
                        .rotate_left(7)
                        ^ (op_hash << 1);
                }
            }
        }

        // 3. Compute deterministic permutation mapping old WireId -> new canonical WireId
        let mut sorted_wires: Vec<(usize, u64)> =
            wire_colors.into_iter().enumerate().collect();
        sorted_wires.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));

        let mut remap: Vec<WireId> = vec![WireId(0); self.wire_count as usize];
        for (new_id, (old_id, _)) in sorted_wires.into_iter().enumerate() {
            remap[old_id] = WireId(new_id as u16);
        }

        // 4. Rewrite boundary ports
        for port in &mut self.boundary_ports {
            if (port.wire.0 as usize) < remap.len() {
                port.wire = remap[port.wire.0 as usize];
            }
        }

        // 5. Rewrite edges and sort them canonically
        for edge in &mut self.edges {
            for in_w in &mut edge.inputs {
                if (in_w.0 as usize) < remap.len() {
                    *in_w = remap[in_w.0 as usize];
                }
            }
            for out_w in &mut edge.outputs {
                if (out_w.0 as usize) < remap.len() {
                    *out_w = remap[out_w.0 as usize];
                }
            }
        }

        // Sort edges by output wires then input wires
        self.edges.sort_by(|a, b| {
            a.outputs
                .cmp(&b.outputs)
                .then_with(|| a.inputs.cmp(&b.inputs))
        });
    }

    /// Monoidal horizontal tensor product of two open hypergraphs: A \otimes B.
    pub fn tensor(&self, other: &Self) -> Self {
        let mut result = self.clone();
        let offset = result.wire_count;

        for port in &other.boundary_ports {
            result.boundary_ports.push(BoundaryPort {
                wire: WireId(port.wire.0 + offset),
                polarity: port.polarity,
                sort: port.sort,
            });
        }

        for edge in &other.edges {
            let mut new_inputs = SmallVec::new();
            for &w in &edge.inputs {
                new_inputs.push(WireId(w.0 + offset));
            }
            let mut new_outputs = SmallVec::new();
            for &w in &edge.outputs {
                new_outputs.push(WireId(w.0 + offset));
            }
            result.edges.push(HyperEdge {
                inputs: new_inputs,
                outputs: new_outputs,
                op: edge.op.clone(),
            });
        }

        result.wire_count += other.wire_count;
        result
    }
}
