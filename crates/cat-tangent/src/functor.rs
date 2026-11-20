//! Tangent bundle monoidal functor over open hypergraphs.
//!
//! Maps an open hypergraph f: X \to Y to its tangent diagram T(f): T(X) \to T(Y).

use cat_core::handles::{ScalarId, WireId};
use cat_core::hypergraph::{BoundaryPort, HyperEdge, HyperOp, OpenHypergraph};
use smallvec::SmallVec;
use std::collections::HashMap;
use std::fmt;

/// Why a diagram cannot be lifted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TangentError {
    /// The operation has no tangent rule here. Dropping its derivative silently (the old behaviour left the
    /// tangent wire unconnected) would be a wrong answer, so the lift refuses.
    UnsupportedOperation(String),
    /// An edge refers to a wire the diagram does not have, or has the wrong number of inputs or outputs.
    Malformed(String),
}

impl fmt::Display for TangentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedOperation(op) => write!(f, "no tangent rule for the operation {op}"),
            Self::Malformed(why) => write!(f, "malformed hypergraph: {why}"),
        }
    }
}

impl std::error::Error for TangentError {}

/// Functor mapping hypergraphs to their tangent bundle diagrams.
pub struct TangentFunctor;

impl TangentFunctor {
    /// Apply tangent functor T: Hypergraph \to Hypergraph.
    /// Each wire w is doubled into (primal_wire, tangent_wire): wire `i` becomes the wires `2i` and `2i + 1`, and
    /// the lifted diagram may have extra wires after them for intermediate products.
    ///
    /// Tangent rules (forward-mode differentiation): `Add` sums the tangents; `Mul` and
    /// `CliffordGeometricProduct` use the Leibniz rule for any number of factors, keeping the order of the
    /// factors (so the non-commutative product is right); `Neg` negates the tangent; `Inv` gives
    /// `-(u^-1)^2 du`; `LieBracket` is bilinear, `d[A, B] = [dA, B] + [A, dB]`; a constant has tangent zero and the
    /// identity has the tangent identity. Any other operation is an error, not a silently missing derivative.
    pub fn lift_hypergraph(diag: &OpenHypergraph) -> Result<OpenHypergraph, TangentError> {
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

        let lookup = |w: &WireId| {
            wire_map
                .get(w)
                .copied()
                .ok_or_else(|| TangentError::Malformed(format!("edge uses the unknown wire {w:?}")))
        };

        // 3. Map hyperedges: for each edge, generate primal and tangent propagation
        for edge in &diag.edges {
            let inputs: Vec<(WireId, WireId)> =
                edge.inputs.iter().map(&lookup).collect::<Result<_, _>>()?;
            let outputs: Vec<(WireId, WireId)> =
                edge.outputs.iter().map(&lookup).collect::<Result<_, _>>()?;
            let primal_in: SmallVec<[WireId; 4]> = inputs.iter().map(|(p, _)| *p).collect();
            let tangent_in: SmallVec<[WireId; 4]> = inputs.iter().map(|(_, t)| *t).collect();
            let single_output = || {
                if outputs.len() == 1 {
                    Ok(outputs[0])
                } else {
                    Err(TangentError::Malformed(format!(
                        "{:?} needs exactly one output, has {}",
                        edge.op,
                        outputs.len()
                    )))
                }
            };
            // The primal computation is the same edge on the primal wires.
            let primal_out: SmallVec<[WireId; 4]> = outputs.iter().map(|(p, _)| *p).collect();
            t_diag.add_edge(HyperEdge::new(
                primal_in.clone(),
                primal_out,
                edge.op.clone(),
            ));

            match &edge.op {
                HyperOp::Add => {
                    let (_, out_t) = single_output()?;
                    t_diag.add_edge(HyperEdge::new(
                        tangent_in,
                        smallvec::smallvec![out_t],
                        HyperOp::Add,
                    ));
                }
                HyperOp::Mul | HyperOp::CliffordGeometricProduct => {
                    // d(u_1 ... u_n) = sum_i u_1 ... du_i ... u_n, the factors kept in order.
                    let (_, out_t) = single_output()?;
                    let n = inputs.len();
                    if n == 0 {
                        return Err(TangentError::Malformed(
                            "a product needs at least one factor".into(),
                        ));
                    }
                    if n == 1 {
                        t_diag.add_edge(HyperEdge::new(
                            tangent_in,
                            smallvec::smallvec![out_t],
                            HyperOp::Add,
                        ));
                        continue;
                    }
                    let mut terms: SmallVec<[WireId; 4]> = SmallVec::new();
                    for i in 0..n {
                        let factors: SmallVec<[WireId; 4]> = (0..n)
                            .map(|j| if j == i { inputs[j].1 } else { inputs[j].0 })
                            .collect();
                        let term = t_diag.alloc_wire();
                        t_diag.add_edge(HyperEdge::new(
                            factors,
                            smallvec::smallvec![term],
                            edge.op.clone(),
                        ));
                        terms.push(term);
                    }
                    t_diag.add_edge(HyperEdge::new(
                        terms,
                        smallvec::smallvec![out_t],
                        HyperOp::Add,
                    ));
                }
                HyperOp::Neg => {
                    let (_, out_t) = single_output()?;
                    t_diag.add_edge(HyperEdge::new(
                        tangent_in,
                        smallvec::smallvec![out_t],
                        HyperOp::Neg,
                    ));
                }
                HyperOp::Inv => {
                    // d(u^-1) = -(u^-1)^2 du, with u^-1 the primal output.
                    let (out_p, out_t) = single_output()?;
                    let (square, scaled) = (t_diag.alloc_wire(), t_diag.alloc_wire());
                    t_diag.add_edge(HyperEdge::binary(out_p, out_p, square, HyperOp::Mul));
                    t_diag.add_edge(HyperEdge::binary(square, inputs[0].1, scaled, HyperOp::Mul));
                    t_diag.add_edge(HyperEdge::new(
                        smallvec::smallvec![scaled],
                        smallvec::smallvec![out_t],
                        HyperOp::Neg,
                    ));
                }
                HyperOp::LieBracket => {
                    if inputs.len() != 2 {
                        return Err(TangentError::Malformed(
                            "the Lie bracket takes two inputs".into(),
                        ));
                    }
                    let (_, out_t) = single_output()?;
                    let (left, right) = (t_diag.alloc_wire(), t_diag.alloc_wire());
                    t_diag.add_edge(HyperEdge::binary(
                        inputs[0].1,
                        inputs[1].0,
                        left,
                        HyperOp::LieBracket,
                    ));
                    t_diag.add_edge(HyperEdge::binary(
                        inputs[0].0,
                        inputs[1].1,
                        right,
                        HyperOp::LieBracket,
                    ));
                    t_diag.add_edge(HyperEdge::binary(left, right, out_t, HyperOp::Add));
                }
                HyperOp::Identity(_) => {
                    let (_, out_t) = single_output()?;
                    t_diag.add_edge(HyperEdge::new(
                        tangent_in,
                        smallvec::smallvec![out_t],
                        edge.op.clone(),
                    ));
                }
                HyperOp::Nullary(_) => {
                    // A constant does not vary: its tangent is the zero scalar (ScalarId(0) is zero).
                    let (_, out_t) = single_output()?;
                    t_diag.add_edge(HyperEdge::new(
                        SmallVec::new(),
                        smallvec::smallvec![out_t],
                        HyperOp::Nullary(ScalarId(0)),
                    ));
                }
                other => {
                    return Err(TangentError::UnsupportedOperation(format!("{other:?}")));
                }
            }
        }

        t_diag.canonicalize();
        Ok(t_diag)
    }
}
