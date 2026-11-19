//! # `algebra-morphism`
//!
//! Category theory functors, isomorphisms, homomorphisms, and representation theory domain bridges.

use algebra_core::numbers::Quaternion;
use algebra_core::{AlgebraResult, ExprGraph, ExprId};
use algebra_engine::matrix::SymbolicMatrix;
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};

/// Multi-Tiered Hierarchy of Mathematical Equality and Structural Equivalence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EquivalenceKind {
    /// 1. Strict Syntactic Identity ($A \equiv B$): Exact arena node ID and AST identity.
    SyntacticIdentity = 1,
    /// 2. Semantic / Algebraic Equality ($A = B$): Equivalence under ring/field algebraic axioms ($A - B = 0$).
    AlgebraicEquality = 2,
    /// 3. Isomorphism ($A \cong B$): Bijective structure-preserving domain morphism.
    Isomorphism = 3,
    /// 4. Category Equivalence / Adjunction ($\mathcal{C} \simeq \mathcal{D}$): Functorial natural isomorphism.
    CategoryEquivalence = 4,
    /// 5. Homotopy / Weak Equivalence ($A \simeq B$): Continuous path equivalence in Homotopy Type Theory ($a =_\text{HoTT} b$).
    HomotopyEquivalence = 5,
    /// 6. Elementary Equivalence ($M \equiv_L N$): Model Theory first-order sentence truth valuation equivalence.
    ElementaryEquivalence = 6,
    /// 7. Bisimulation / Observational Equivalence ($P \sim Q$): Behavioral state-transition equivalence in process algebra.
    Bisimulation = 7,
}

/// Classifier for multi-tiered mathematical equivalence.
#[derive(Debug, Clone, Default)]
pub struct EquivalenceClassifier;

impl EquivalenceClassifier {
    pub fn new() -> Self {
        Self
    }

    /// Classify the equivalence level between two expression nodes in an `ExprGraph`: `Some(SyntacticIdentity)` for
    /// the same node, `Some(AlgebraicEquality)` when their difference is zero, and `None` when they are not equal.
    ///
    /// Algebraic equality is decided by simplifying `a - b` to zero and otherwise by multi-prime Schwartz–Zippel
    /// evaluation (a mismatch at any sample point is a proof of inequality; agreement at every sample certifies
    /// equality with error probability below `1e-12`). The tiers above algebraic equality (isomorphism,
    /// category equivalence, homotopy, elementary equivalence, bisimulation) are relations between structures, not
    /// between two expressions, and are not decided here (see [`EquivalenceClassifier::check_isomorphism`] for
    /// named domains). This used to return `AlgebraicEquality` for every pair of distinct expressions.
    pub fn classify(&self, graph: &ExprGraph, a: ExprId, b: ExprId) -> Option<EquivalenceKind> {
        use algebra_core::probabilistic::{ProbabilisticVerifier, Solution};
        if a == b {
            return Some(EquivalenceKind::SyntacticIdentity);
        }
        let diff = graph.sub(a, b);
        if let Ok(simplified) = algebra_engine::simplify::Simplifier::simplify(graph, diff) {
            match graph.get(simplified).kind {
                algebra_core::ExprKind::Number(algebra_core::Number::Integer(0)) => {
                    return Some(EquivalenceKind::AlgebraicEquality);
                }
                algebra_core::ExprKind::Number(algebra_core::Number::Integer(_))
                | algebra_core::ExprKind::Number(algebra_core::Number::Rational(_, _)) => {
                    return None; // a non-zero constant difference
                }
                _ => {}
            }
        }
        match ProbabilisticVerifier::verify_equivalence(graph, a, b, 1e-12) {
            Solution::Deterministic(true) | Solution::Probabilistic { result: true, .. } => {
                Some(EquivalenceKind::AlgebraicEquality)
            }
            _ => None,
        }
    }

    /// Check if two mathematical domains are structurally isomorphic (e.g. Quaternions ≅ M_2(C)).
    pub fn check_isomorphism(&self, domain_a: &str, domain_b: &str) -> bool {
        let da = domain_a.to_lowercase();
        let db = domain_b.to_lowercase();

        if da == db {
            return true;
        }

        matches!(
            (da.as_str(), db.as_str()),
            ("quaternion" | "h", "matrix2x2complex" | "m_2(c)")
                | ("matrix2x2complex" | "m_2(c)", "quaternion" | "h")
                | ("dualnumber", "autodiff_operator")
                | ("autodiff_operator", "dualnumber")
                | ("complex" | "c", "matrix2x2real")
                | ("matrix2x2real", "complex" | "c")
        )
    }
}

/// Trait for algebraic domain morphisms and representation mappings.
pub trait DomainMorphism<From, To> {
    /// Map object from source domain to target domain.
    fn map_forward(&self, graph: &ExprGraph, input: &From) -> To;

    /// Optional inverse mapping from target domain back to source domain.
    fn map_backward(&self, graph: &ExprGraph, input: &To) -> Option<From>;
}

/// Category Theory Functor mapping objects and morphisms from Category C to Category D.
pub trait Functor<ObjC, ObjD> {
    /// Map object C to object D.
    fn map_object(&self, obj: &ObjC) -> ObjD;
}

/// Monoidal Category equipped with tensor product $A \otimes B$ and unit object $I$.
pub trait MonoidalCategory<Obj> {
    /// Compute tensor product object $A \otimes B$.
    fn tensor_product(&self, graph: &ExprGraph, a: &Obj, b: &Obj) -> AlgebraResult<Obj>;

    /// Return monoidal unit object $I$.
    fn unit_object(&self, graph: &ExprGraph) -> Obj;
}

/// Representation Isomorphism: Quaternions ℍ ≅ M_2(ℂ).
///
/// Maps Quaternion q = w + xi + yj + zk to 2x2 complex matrix:
/// [ w + x*i    y + z*i ]
/// [ -y + z*i   w - x*i ]
pub struct QuaternionToMatrix2x2;

impl QuaternionToMatrix2x2 {
    /// Map Quaternion to Symbolic Matrix in `ExprGraph`.
    pub fn to_matrix(&self, graph: &ExprGraph, q: &Quaternion) -> AlgebraResult<SymbolicMatrix> {
        let w_num = q.w.numer().to_i64().unwrap_or(0);
        let w_den = q.w.denom().to_i64().unwrap_or(1);
        let w = graph.rational(w_num, w_den);

        let x_num = q.x.numer().to_i64().unwrap_or(0);
        let x_den = q.x.denom().to_i64().unwrap_or(1);
        let x = graph.rational(x_num, x_den);

        let y_num = q.y.numer().to_i64().unwrap_or(0);
        let y_den = q.y.denom().to_i64().unwrap_or(1);
        let y = graph.rational(y_num, y_den);

        let z_num = q.z.numer().to_i64().unwrap_or(0);
        let z_den = q.z.denom().to_i64().unwrap_or(1);
        let z = graph.rational(z_num, z_den);

        let i_const = graph.constant(algebra_core::Constant::I);

        // w + x*i
        let xi = graph.mul([x, i_const]);
        let m00 = graph.add([w, xi]);

        // y + z*i
        let zi = graph.mul([z, i_const]);
        let m01 = graph.add([y, zi]);

        // -y + z*i
        let neg_y = graph.neg(y);
        let m10 = graph.add([neg_y, zi]);

        // w - x*i
        let m11 = graph.sub(w, xi);

        SymbolicMatrix::new(2, 2, vec![m00, m01, m10, m11])
    }
}

/// Structure Matrix Basis Generators and Representation metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructureMatrixInfo {
    pub dimension: usize,
    pub basis_generators: Vec<String>,
    pub scalar_diagonal_mapping: String,
}

/// Universal Structure Matrix Promotion Engine.
pub struct StructureMatrixRepresentation;

impl StructureMatrixRepresentation {
    /// Return structure matrix representation metadata for a domain.
    pub fn get_structure_info(domain: &algebra_core::Domain) -> Option<StructureMatrixInfo> {
        match domain {
            algebra_core::Domain::Complex => Some(StructureMatrixInfo {
                dimension: 2,
                basis_generators: vec!["e1 = 1".into(), "e2 = i".into()],
                scalar_diagonal_mapping: "r * I_2 = [[r, 0], [0, r]]".into(),
            }),
            algebra_core::Domain::Hypercomplex { dimension: 4 } => Some(StructureMatrixInfo {
                dimension: 4,
                basis_generators: vec![
                    "e1 = 1".into(),
                    "e2 = i".into(),
                    "e3 = j".into(),
                    "e4 = k".into(),
                ],
                scalar_diagonal_mapping: "r * I_4 = diag(r, r, r, r)".into(),
            }),
            algebra_core::Domain::Hypercomplex { dimension } => Some(StructureMatrixInfo {
                dimension: *dimension as usize,
                basis_generators: (0..*dimension).map(|idx| format!("e{}", idx + 1)).collect(),
                scalar_diagonal_mapping: format!("r * I_{}", dimension),
            }),
            algebra_core::Domain::Matrix { rows, cols, .. } => Some(StructureMatrixInfo {
                dimension: *rows,
                basis_generators: (0..(*rows * *cols))
                    .map(|idx| format!("E_{}", idx + 1))
                    .collect(),
                scalar_diagonal_mapping: format!("r * I_{}", rows),
            }),
            _ => None,
        }
    }

    /// Promote a scalar or lower-algebra element to a target algebra representable via a structure matrix.
    /// Implicitly logs high-level domain/algebra promotion assumptions and basis generators.
    pub fn promote(
        from_domain: &algebra_core::Domain,
        to_domain: &algebra_core::Domain,
    ) -> algebra_core::Domain {
        let promoted = algebra_core::Domain::promote(from_domain, to_domain);

        if let Some(info) = Self::get_structure_info(&promoted) {
            tracing::warn!(
                target: "urae::structure_matrix_promotion",
                from = %from_domain,
                to = %to_domain,
                promoted = %promoted,
                matrix_dim = info.dimension,
                generators = ?info.basis_generators,
                mapping = %info.scalar_diagonal_mapping,
                "STRUCTURE MATRIX PROMOTION ASSUMPTION: Promoted element from domain [{}] into algebra [{}] via structure matrix diagonal (r * I_{}) using generators {:?}",
                from_domain,
                promoted,
                info.dimension,
                info.basis_generators
            );
        } else {
            tracing::debug!(
                target: "urae::domain_promotion",
                from = %from_domain,
                to = %to_domain,
                promoted = %promoted,
                "Domain promotion applied from [{}] to [{}] -> [{}]",
                from_domain,
                to_domain,
                promoted
            );
        }

        promoted
    }
}

/// Simplification Functor mapping expression objects in Category $\mathbf{Expr}$
/// to simplified normal forms via rewrite rules and e-graphs, preserving algebraic equivalence.
pub struct SimplificationFunctor<'a> {
    pub graph: &'a ExprGraph,
}

impl<'a> SimplificationFunctor<'a> {
    pub fn new(graph: &'a ExprGraph) -> Self {
        Self { graph }
    }
}

impl<'a> Functor<ExprId, ExprId> for SimplificationFunctor<'a> {
    fn map_object(&self, obj: &ExprId) -> ExprId {
        algebra_engine::simplify::Simplifier::simplify(self.graph, *obj).unwrap_or(*obj)
    }
}
