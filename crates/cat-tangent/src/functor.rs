//! Tangent bundle monoidal functor over open hypergraphs.
//!
//! Maps an open hypergraph f: X \to Y to its tangent diagram T(f): T(X) \to T(Y).

use cat_core::handles::WireId;
use cat_core::hypergraph::{BoundaryPort, HyperEdge, HyperOp, OpenHypergraph};
use std::collections::HashMap;

/// Functor mapping hypergraphs to their tangent bundle diagrams.
pub struct TangentFunctor;

impl TangentFunctor {
    /// Apply tangent functor T: Hypergraph \to Hypergraph.
    /// Each wire w is doubled into (primal_wire, tangent_wire).
    pub fn lift_hypergraph(diag: &OpenHypergraph) -> OpenHypergraph {
        let mut t_diag = OpenHypergraph::new();
        let mut wire_map: HashMap<WireId, (WireId, WireId)> = HashMap::new();

        // 1. Allocate primal and tangent wires
        for i in 0..diag.wire_count {
            let orig_w = WireId(i);
            let primal_w = t_diag.alloc_wire();
            let tangent_w = t_diag.alloc_wire();
            wire_map.insert(orig_w, (primal_w, tangent_w));
        }

        // 2. Map boundary ports
        for port in &diag.boundary_ports {
            if let Some(&(pw, tw)) = wire_map.get(&port.wire) {
                t_diag.boundary_ports.push(BoundaryPort {
                    wire: pw,
                    polarity: port.polarity,
                    sort: port.sort,
                });
                t_diag.boundary_ports.push(BoundaryPort {
                    wire: tw,
                    polarity: port.polarity,
                    sort: port.sort,
                });
            }
        }

        // 3. Map hyperedges: for each edge, generate primal and tangent propagation
        for edge in &diag.edges {
            match &edge.op {
                HyperOp::Add => {
                    // d(u + v) = du + dv
                    let mut primal_in = smallvec::SmallVec::new();
                    let mut tangent_in = smallvec::SmallVec::new();
                    for &in_w in &edge.inputs {
                        if let Some(&(pw, tw)) = wire_map.get(&in_w) {
                            primal_in.push(pw);
                            tangent_in.push(tw);
                        }
                    }
                    if let Some(&out_w) = edge.outputs.first() {
                        if let Some(&(pw, tw)) = wire_map.get(&out_w) {
                            t_diag.add_edge(HyperEdge::new(primal_in, smallvec::smallvec![pw], HyperOp::Add));
                            t_diag.add_edge(HyperEdge::new(tangent_in, smallvec::smallvec![tw], HyperOp::Add));
                        }
                    }
                }
                HyperOp::Mul => {
                    // d(u * v) = u dv + v du (Leibniz product rule)
                    if edge.inputs.len() == 2 && edge.outputs.len() == 1 {
                        let &(u, du) = wire_map.get(&edge.inputs[0]).unwrap();
                        let &(v, dv) = wire_map.get(&edge.inputs[1]).unwrap();
                        let &(out_p, out_t) = wire_map.get(&edge.outputs[0]).unwrap();

                        // Primal: out_p = u * v
                        t_diag.add_edge(HyperEdge::binary(u, v, out_p, HyperOp::Mul));

                        // Tangent: out_t = (u * dv) + (v * du)
                        let u_dv = t_diag.alloc_wire();
                        let v_du = t_diag.alloc_wire();
                        t_diag.add_edge(HyperEdge::binary(u, dv, u_dv, HyperOp::Mul));
                        t_diag.add_edge(HyperEdge::binary(v, du, v_du, HyperOp::Mul));
                        t_diag.add_edge(HyperEdge::binary(u_dv, v_du, out_t, HyperOp::Add));
                    }
                }
                _ => {
                    // Default passthrough
                    let mut primal_in = smallvec::SmallVec::new();
                    for &in_w in &edge.inputs {
                        if let Some(&(pw, _)) = wire_map.get(&in_w) {
                            primal_in.push(pw);
                        }
                    }
                    let mut primal_out = smallvec::SmallVec::new();
                    for &out_w in &edge.outputs {
                        if let Some(&(pw, _)) = wire_map.get(&out_w) {
                            primal_out.push(pw);
                        }
                    }
                    t_diag.add_edge(HyperEdge::new(primal_in, primal_out, edge.op.clone()));
                }
            }
        }

        t_diag.canonicalize();
        t_diag
    }
}
