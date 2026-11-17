//! # `algebra-quantum`
//!
//! Quantum operator algebras, non-commutative commutators $[A, B] = A B - B A$,
//! creation/annihilation operators ($a, a^\dagger$), Canonical Commutation Relations (CCR),
//! Dirac Bra-Ket notation $\langle \psi | A | \phi \rangle$, Gauge theories ($U(1), SU(2), SU(3)$),
//! and Dyson series S-matrix expansions.

use algebra_core::{AlgebraResult, ExprGraph, ExprId, SymbolId};
use algebra_engine::calculus::SymbolicCalculus;

/// Quantum operator representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperatorKind {
    Annihilation(SymbolId),
    Creation(SymbolId),
    Hamiltonian,
}

/// Quantum Operator with index and mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuantumOperator {
    pub kind: OperatorKind,
    pub mode: SymbolId,
}

impl QuantumOperator {
    #[inline]
    pub const fn annihilation(mode: SymbolId) -> Self {
        Self {
            kind: OperatorKind::Annihilation(mode),
            mode,
        }
    }

    #[inline]
    pub const fn creation(mode: SymbolId) -> Self {
        Self {
            kind: OperatorKind::Creation(mode),
            mode,
        }
    }

    /// Compute commutator bracket $[A, B] = A B - B A$.
    pub fn commutator(graph: &ExprGraph, a: ExprId, b: ExprId) -> ExprId {
        let ab = graph.mul([a, b]);
        let ba = graph.mul([b, a]);
        graph.sub(ab, ba)
    }

    /// Apply Canonical Commutation Relation $[a_i, a_j^\dagger] = \delta_{ij}$.
    pub fn ccr_commutator(
        &self,
        graph: &ExprGraph,
        other: &QuantumOperator,
    ) -> AlgebraResult<ExprId> {
        match (&self.kind, &other.kind) {
            (OperatorKind::Annihilation(m1), OperatorKind::Creation(m2)) if m1 == m2 => {
                // [a, a^\dagger] = 1
                Ok(graph.integer(1))
            }
            (OperatorKind::Creation(m1), OperatorKind::Annihilation(m2)) if m1 == m2 => {
                // [a^\dagger, a] = -1
                Ok(graph.integer(-1))
            }
            _ => {
                let zero = graph.integer(0);
                Ok(zero)
            }
        }
    }
}

/// Dirac Bra-Ket Notation $\langle \psi | A | \phi \rangle$.
#[derive(Debug, Clone)]
pub struct BraKet {
    pub bra: SymbolId,
    pub operator: Option<ExprId>,
    pub ket: SymbolId,
}

impl BraKet {
    pub fn inner_product(bra: SymbolId, ket: SymbolId) -> Self {
        Self {
            bra,
            operator: None,
            ket,
        }
    }

    pub fn matrix_element(bra: SymbolId, operator: ExprId, ket: SymbolId) -> Self {
        Self {
            bra,
            operator: Some(operator),
            ket,
        }
    }

    /// Format Bra-Ket as symbolic expression in ExprGraph.
    pub fn to_expr(&self, graph: &ExprGraph) -> ExprId {
        let bra_name = graph.symbols.resolve(self.bra).unwrap_or_default();
        let ket_name = graph.symbols.resolve(self.ket).unwrap_or_default();
        let bra_sym = graph.symbol(&bra_name);
        let ket_sym = graph.symbol(&ket_name);
        if let Some(op) = self.operator {
            graph.function("BraKetMatrixElement", [bra_sym, op, ket_sym])
        } else {
            graph.function("BraKetInnerProduct", [bra_sym, ket_sym])
        }
    }
}

/// Gauge Theory Field Strength Tensor $F_{\mu\nu}^a = \partial_\mu A_\nu^a - \partial_\nu A_\mu^a + g f^{abc} A_\mu^b A_\nu^c$.
pub struct GaugeTheory;

impl GaugeTheory {
    /// Compute Yang-Mills Non-Abelian Field Strength Tensor $F_{\mu\nu}^a$.
    pub fn field_strength_tensor(
        graph: &ExprGraph,
        a_mu_a: ExprId,
        a_nu_a: ExprId,
        x_mu: SymbolId,
        x_nu: SymbolId,
        coupling_g: Option<ExprId>,
        structure_constant_term: Option<ExprId>,
    ) -> ExprId {
        let d_mu_a_nu = graph.diff(a_nu_a, x_mu);
        let d_nu_a_mu = graph.diff(a_mu_a, x_nu);
        let abelian_f = graph.sub(d_mu_a_nu, d_nu_a_mu);

        if let (Some(g), Some(f_abc)) = (coupling_g, structure_constant_term) {
            let non_abelian_term = graph.mul([g, f_abc]);
            graph.add([abelian_f, non_abelian_term])
        } else {
            abelian_f
        }
    }
}

/// Dyson Series S-Matrix Perturbation Expansion $U(t, t_0) = I - i \int H_I(t') dt' + \dots$.
pub struct DysonSeries;

impl DysonSeries {
    /// First-order Dyson series term $-i \int H_I(t') dt'$ with an *indefinite* integral (the
    /// antiderivative, no lower limit). If `H_I` cannot be integrated in closed form the result
    /// keeps an unevaluated integral rather than substituting `H_I` itself.
    pub fn first_order_term(graph: &ExprGraph, interaction_h: ExprId, t: SymbolId) -> ExprId {
        let int_h = graph.integrate(interaction_h, t).unwrap_or_else(|_| {
            let name = graph.symbols.resolve(t).unwrap_or_default();
            graph.integral(interaction_h, &name, None, None)
        });
        let minus_i = graph.mul([graph.integer(-1), graph.constant(algebra_core::Constant::I)]);
        graph.mul([minus_i, int_h])
    }

    /// First-order Dyson series term $-i \int_{t_0}^{t} H_I(t') dt'$ over the interval from
    /// `t0` to `t`. Falls back to an unevaluated definite integral when no antiderivative is found.
    pub fn first_order_term_between(
        graph: &ExprGraph,
        interaction_h: ExprId,
        t: SymbolId,
        t0: ExprId,
    ) -> ExprId {
        let t_expr = graph.symbol(&graph.symbols.resolve(t).unwrap_or_default());
        let integral = match graph.integrate(interaction_h, t) {
            Ok(anti) => {
                let upper = graph.substitute(anti, t, t_expr);
                let lower = graph.substitute(anti, t, t0);
                graph.sub(upper, lower)
            }
            Err(_) => {
                let name = graph.symbols.resolve(t).unwrap_or_default();
                graph.integral(interaction_h, &name, Some(t0), Some(t_expr))
            }
        };
        let minus_i = graph.mul([graph.integer(-1), graph.constant(algebra_core::Constant::I)]);
        graph.mul([minus_i, integral])
    }
}
