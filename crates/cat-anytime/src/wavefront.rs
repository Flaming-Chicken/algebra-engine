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

/// The outputs of one evaluated edge and the value they take.
type EdgeOutcome = Result<(SmallVec<[WireId; 4]>, ScalarAtom), WavefrontError>;

/// Failure to schedule or evaluate a hypergraph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WavefrontError {
    /// The edges depend on each other cyclically, so no level assignment exists.
    Cycle,
    /// An edge reads a wire that is neither a boundary input nor produced by any edge, and no
    /// initial value was supplied for it.
    MissingWire(WireId),
    /// The operation has no scalar evaluation here.
    Unsupported(String),
    /// A scalar operation failed (for example inverting zero).
    Arithmetic(String),
}

impl std::fmt::Display for WavefrontError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cycle => write!(f, "the hypergraph has a dependency cycle"),
            Self::MissingWire(w) => write!(f, "wire {w:?} has no value"),
            Self::Unsupported(op) => write!(f, "operation {op} has no scalar evaluation"),
            Self::Arithmetic(why) => write!(f, "arithmetic error: {why}"),
        }
    }
}

impl std::error::Error for WavefrontError {}

/// Multiplicative inverse of an exact or floating scalar.
fn invert(a: &ScalarAtom) -> Result<ScalarAtom, WavefrontError> {
    let zero = || WavefrontError::Arithmetic("division by zero".into());
    let rational = |n: i64, d: i64| -> ScalarAtom {
        // Keep the sign in the numerator and collapse to an integer when possible.
        let (n, d) = if d < 0 { (-n, -d) } else { (n, d) };
        if d == 1 {
            ScalarAtom::Integer(n)
        } else {
            ScalarAtom::Rational(n, d)
        }
    };
    match a {
        ScalarAtom::Zero => Err(zero()),
        ScalarAtom::One => Ok(ScalarAtom::One),
        ScalarAtom::Integer(0) | ScalarAtom::Rational(0, _) => Err(zero()),
        ScalarAtom::Integer(n) => Ok(rational(1, *n)),
        ScalarAtom::Rational(n, d) => Ok(rational(*d, *n)),
        ScalarAtom::Float(x) if *x == 0.0 => Err(zero()),
        ScalarAtom::Float(x) => Ok(ScalarAtom::Float(1.0 / x)),
        other => Err(WavefrontError::Unsupported(format!("inverse of {other:?}"))),
    }
}

impl WavefrontSchedule {
    /// Construct a level-synchronous schedule from an open hypergraph.
    ///
    /// An edge's level is one more than the deepest level among the edges that produce its
    /// inputs, so the schedule is correct whatever order the edges are listed in. Wires with no
    /// producer (boundary inputs or externally supplied) are available at level 0. A cyclic
    /// dependency is an error.
    pub fn build(hypergraph: &OpenHypergraph) -> Result<Self, WavefrontError> {
        let edges = &hypergraph.edges;
        // Which edge produces each wire.
        let mut producer: HashMap<WireId, usize> = HashMap::new();
        for (i, edge) in edges.iter().enumerate() {
            for &out in &edge.outputs {
                producer.insert(out, i);
            }
        }

        // Kahn's algorithm over edge-to-edge dependencies.
        let mut indegree = vec![0usize; edges.len()];
        let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); edges.len()];
        for (i, edge) in edges.iter().enumerate() {
            let mut sources: Vec<usize> = edge
                .inputs
                .iter()
                .filter_map(|w| producer.get(w).copied())
                .collect();
            sources.sort_unstable();
            sources.dedup();
            for source in sources {
                if source == i {
                    return Err(WavefrontError::Cycle);
                }
                indegree[i] += 1;
                dependents[source].push(i);
            }
        }
        let mut level = vec![0usize; edges.len()];
        let mut queue: Vec<usize> = (0..edges.len()).filter(|&i| indegree[i] == 0).collect();
        let mut visited = 0;
        while let Some(i) = queue.pop() {
            visited += 1;
            for &next in &dependents[i] {
                level[next] = level[next].max(level[i] + 1);
                indegree[next] -= 1;
                if indegree[next] == 0 {
                    queue.push(next);
                }
            }
        }
        if visited != edges.len() {
            return Err(WavefrontError::Cycle);
        }

        let max_level = level.iter().copied().max().unwrap_or(0);
        let mut levels = vec![WavefrontLevel { edges: Vec::new() }; max_level + 1];
        for (i, edge) in edges.iter().enumerate() {
            levels[level[i]].edges.push(edge.clone());
        }
        Ok(Self { levels })
    }

    /// Execute the schedule, evaluating the edges of each level in parallel with Rayon.
    ///
    /// Supported operations: `Add`, `Mul`, `Neg`, `Inv`, `Identity`. `Nullary(id)` yields the
    /// integer `id` (the generator's index; supply real constants as `initial_wires`). A wire
    /// read before it has a value, an unsupported operation, or a failed scalar operation is an
    /// error: nothing is silently replaced by zero.
    pub fn evaluate(
        &self,
        initial_wires: HashMap<WireId, ScalarAtom>,
    ) -> Result<HashMap<WireId, ScalarAtom>, WavefrontError> {
        let mut wire_values = initial_wires;

        for level in &self.levels {
            let level_results: Vec<EdgeOutcome> = level
                .edges
                .par_iter()
                .map(|edge| {
                    let mut input_atoms: SmallVec<[ScalarAtom; 4]> = SmallVec::new();
                    for &in_w in &edge.inputs {
                        match wire_values.get(&in_w) {
                            Some(val) => input_atoms.push(val.clone()),
                            None => return Err(WavefrontError::MissingWire(in_w)),
                        }
                    }
                    let result = match &edge.op {
                        HyperOp::Nullary(sid) => ScalarAtom::Integer(sid.0 as i64),
                        HyperOp::Identity(_) => input_atoms
                            .first()
                            .cloned()
                            .ok_or(WavefrontError::MissingWire(WireId(0)))?,
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
                        HyperOp::Neg => match input_atoms.first() {
                            Some(first) => {
                                cat_core::ScalarLattice::mul(&ScalarAtom::Integer(-1), first)
                            }
                            None => {
                                return Err(WavefrontError::Arithmetic(
                                    "negation of nothing".into(),
                                ));
                            }
                        },
                        HyperOp::Inv => match input_atoms.first() {
                            Some(first) => invert(first)?,
                            None => {
                                return Err(WavefrontError::Arithmetic(
                                    "inverse of nothing".into(),
                                ));
                            }
                        },
                        other => {
                            return Err(WavefrontError::Unsupported(format!("{other:?}")));
                        }
                    };
                    Ok((edge.outputs.clone(), result))
                })
                .collect();

            for outcome in level_results {
                let (outputs, val) = outcome?;
                for out_w in outputs {
                    wire_values.insert(out_w, val.clone());
                }
            }
        }

        Ok(wire_values)
    }
}
