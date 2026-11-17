//! # `algebra_engine::simplify::dynamic`
//!
//! Dynamic rule selector that synthesizes conflict-free E-Graph rule sets from algebraic traits
//! and detected domain features.

use super::SymbolicLang;
use super::bridges::{
    AskeySchemeGroup, BridgeRouter, HyperbolicIdentityGroup, IdentityGroup,
    ItoNilpotentBridgeGroup, LieAlgebraGroup, TransformClusterGroup, TrigIdentityGroup,
};
use super::detector::{DetectedFeatures, DomainFeatureDetector};
use algebra_core::domain::GoalDomain;
use algebra_core::{ExprGraph, ExprId};
use egg::{Rewrite, rewrite};

/// Base Axiomatic Equational Rule Generator for Commutative and Non-Commutative Rings/Fields.
pub struct AxiomaticRuleGenerator;

impl AxiomaticRuleGenerator {
    /// Return the canonical equational rewrite rules for a commutative ring/field.
    pub fn commutative_ring_rules() -> Vec<Rewrite<SymbolicLang, ()>> {
        vec![
            // Additive Monoid & Group axioms
            rewrite!("comm-add-comm"; "(+ ?a ?b)" => "(+ ?b ?a)"),
            rewrite!("comm-add-assoc-1"; "(+ (+ ?a ?b) ?c)" => "(+ ?a (+ ?b ?c))"),
            rewrite!("comm-add-assoc-2"; "(+ ?a (+ ?b ?c))" => "(+ (+ ?a ?b) ?c)"),
            rewrite!("comm-add-zero"; "(+ ?a 0)" => "?a"),
            rewrite!("comm-add-neg"; "(+ ?a (neg ?a))" => "0"),
            rewrite!("comm-add-neg-2"; "(+ (neg ?a) ?a)" => "0"),
            rewrite!("comm-sub-def"; "(- ?a ?b)" => "(+ ?a (neg ?b))"),
            rewrite!("comm-sub-self"; "(- ?a ?a)" => "0"),
            rewrite!("comm-neg-neg"; "(neg (neg ?a))" => "?a"),
            rewrite!("comm-neg-zero"; "(neg 0)" => "0"),
            // Multiplicative Monoid & Group axioms
            rewrite!("comm-mul-comm"; "(* ?a ?b)" => "(* ?b ?a)"),
            rewrite!("comm-mul-assoc-1"; "(* (* ?a ?b) ?c)" => "(* ?a (* ?b ?c))"),
            rewrite!("comm-mul-assoc-2"; "(* ?a (* ?b ?c))" => "(* (* ?a ?b) ?c)"),
            rewrite!("comm-mul-one"; "(* ?a 1)" => "?a"),
            rewrite!("comm-mul-zero"; "(* ?a 0)" => "0"),
            rewrite!("comm-mul-neg-one"; "(* ?a -1)" => "(neg ?a)"),
            // Distributivity
            rewrite!("comm-distrib-1"; "(* ?a (+ ?b ?c))" => "(+ (* ?a ?b) (* ?a ?c))"),
            rewrite!("comm-distrib-2"; "(+ (* ?a ?b) (* ?a ?c))" => "(* ?a (+ ?b ?c))"),
            // Field Division
            rewrite!("comm-div-self"; "(/ ?a ?a)" => "1"),
            rewrite!("comm-div-one"; "(/ ?a 1)" => "?a"),
            rewrite!("comm-div-zero"; "(/ 0 ?a)" => "0"),
            rewrite!("comm-div-mul-cancel-1"; "(/ (* ?a ?b) ?a)" => "?b"),
            rewrite!("comm-div-mul-cancel-2"; "(/ (* ?b ?a) ?a)" => "?b"),
            rewrite!("comm-recip-recip"; "(/ 1 (/ 1 ?a))" => "?a"),
            // Exponentiation & Log/Exp inverse laws
            rewrite!("comm-pow-zero"; "(^ ?a 0)" => "1"),
            rewrite!("comm-pow-one"; "(^ ?a 1)" => "?a"),
            rewrite!("comm-pow-mul"; "(* (^ ?a ?b) (^ ?a ?c))" => "(^ ?a (+ ?b ?c))"),
            rewrite!("comm-exp-0"; "(fn exp 0)" => "1"),
            rewrite!("comm-ln-1"; "(fn ln 1)" => "0"),
            rewrite!("comm-log-1"; "(fn log 1)" => "0"),
            rewrite!("comm-exp-ln"; "(fn exp (fn ln ?x))" => "?x"),
            rewrite!("comm-exp-log"; "(fn exp (fn log ?x))" => "?x"),
        ]
    }

    /// Return the canonical rewrite rules for a non-commutative ring (e.g. Quaternions, Matrices, Operators).
    pub fn non_commutative_ring_rules() -> Vec<Rewrite<SymbolicLang, ()>> {
        vec![
            // Additive Group axioms (Addition is still commutative)
            rewrite!("nc-add-comm"; "(+ ?a ?b)" => "(+ ?b ?a)"),
            rewrite!("nc-add-assoc"; "(+ (+ ?a ?b) ?c)" => "(+ ?a (+ ?b ?c))"),
            rewrite!("nc-add-zero"; "(+ ?a 0)" => "?a"),
            rewrite!("nc-add-neg"; "(+ ?a (neg ?a))" => "0"),
            rewrite!("nc-sub-def"; "(- ?a ?b)" => "(+ ?a (neg ?b))"),
            // Multiplicative Monoid axioms (NO general a*b = b*a)
            rewrite!("nc-mul-assoc"; "(* (* ?a ?b) ?c)" => "(* ?a (* ?b ?c))"),
            rewrite!("nc-mul-one-r"; "(* ?a 1)" => "?a"),
            rewrite!("nc-mul-one-l"; "(* 1 ?a)" => "?a"),
            rewrite!("nc-mul-zero-r"; "(* ?a 0)" => "0"),
            rewrite!("nc-mul-zero-l"; "(* 0 ?a)" => "0"),
            // Left and Right Distributivity
            rewrite!("nc-distrib-left"; "(* ?a (+ ?b ?c))" => "(+ (* ?a ?b) (* ?a ?c))"),
            rewrite!("nc-distrib-right"; "(* (+ ?a ?b) ?c)" => "(+ (* ?a ?c) (* ?b ?c))"),
            // Matrix Identity Identities
            rewrite!("nc-mat-id-mul-r"; "(* ?a I)" => "?a"),
            rewrite!("nc-mat-id-mul-l"; "(* I ?a)" => "?a"),
            // Quaternion Hamilton Identities: i^2 = j^2 = k^2 = -1, ij = k, ji = -k
            rewrite!("quat-i-pow2"; "(^ qi 2)" => "-1"),
            rewrite!("quat-j-pow2"; "(^ qj 2)" => "-1"),
            rewrite!("quat-k-pow2"; "(^ qk 2)" => "-1"),
            rewrite!("quat-i-sq"; "(* qi qi)" => "-1"),
            rewrite!("quat-j-sq"; "(* qj qj)" => "-1"),
            rewrite!("quat-k-sq"; "(* qk qk)" => "-1"),
            rewrite!("quat-ij"; "(* qi qj)" => "qk"),
            rewrite!("quat-ji"; "(* qj qi)" => "(neg qk)"),
            rewrite!("quat-jk"; "(* qj qk)" => "qi"),
            rewrite!("quat-kj"; "(* qk qj)" => "(neg qi)"),
            rewrite!("quat-ki"; "(* qk qi)" => "qj"),
            rewrite!("quat-ik"; "(* qi qk)" => "(neg qj)"),
        ]
    }
}

/// Dynamic Rule Selector that analyzes expressions and builds conflict-free E-Graph rule sets.
pub struct DynamicRuleSelector;

impl DynamicRuleSelector {
    /// Select the minimal, conflict-free, domain-gated rule set for the given expression and goal.
    pub fn select_rules(
        graph: &ExprGraph,
        root: ExprId,
        goal: GoalDomain,
    ) -> (Vec<Rewrite<SymbolicLang, ()>>, DetectedFeatures) {
        let features = DomainFeatureDetector::scan(graph, root);
        let mut rules = Vec::new();

        // 1. Base Algebraic Ring / Field Axioms
        if features.is_non_commutative_ring {
            rules.extend(AxiomaticRuleGenerator::non_commutative_ring_rules());
        } else {
            rules.extend(AxiomaticRuleGenerator::commutative_ring_rules());
            if !expr_has_imaginary(graph, root) {
                rules.push(rewrite!("comm-ln-exp"; "(fn ln (fn exp ?x))" => "?x"));
                rules.push(rewrite!("comm-log-exp"; "(fn log (fn exp ?x))" => "?x"));
            }
        }

        // 2. Trigonometric Identity Cluster
        if features.has_trigonometric
            || goal == GoalDomain::RationalWeierstrass
            || goal == GoalDomain::ComplexExponential
        {
            rules.extend(TrigIdentityGroup::rules());
        }

        // 3. Hyperbolic Identity Cluster
        if features.has_hyperbolic || goal == GoalDomain::Hyperbolic {
            rules.extend(HyperbolicIdentityGroup::rules());
        }

        // 4. Stochastic Itô Calculus Cluster (Gated to prevent dx^2 = 0 collapse)
        if features.has_stochastic_terms || goal == GoalDomain::NilpotentDual {
            rules.extend(ItoNilpotentBridgeGroup::rules());
        }

        // 5. Special Functions and Askey Scheme Cluster
        if features.has_special_functions {
            rules.extend(AskeySchemeGroup::rules());
        }

        // 6. Lie Algebra and Quantum Commutator Cluster
        if features.has_commutator_brackets {
            rules.extend(LieAlgebraGroup::rules());
        }

        // 7. Operational Transform Cluster
        if features.has_transforms || goal == GoalDomain::FrequencyOperational {
            rules.extend(TransformClusterGroup::rules());
        }

        // 8. Goal-Directed Bridge Morphisms
        if goal != GoalDomain::ConserveOriginal {
            rules.extend(BridgeRouter::select_goal_bridges(goal));
        }

        (rules, features)
    }
}

fn expr_has_imaginary(graph: &ExprGraph, id: ExprId) -> bool {
    use algebra_core::{Constant, ExprKind, Number};
    match &graph.get(id).kind {
        ExprKind::Number(Number::Constant(Constant::I)) => true,
        ExprKind::Symbol(s) => matches!(graph.symbols.resolve(*s).as_deref(), Some("i" | "I")),
        ExprKind::Neg(a) => expr_has_imaginary(graph, *a),
        ExprKind::Add(xs) | ExprKind::Mul(xs) => xs.iter().any(|&x| expr_has_imaginary(graph, x)),
        ExprKind::Sub(a, b) | ExprKind::Div(a, b) | ExprKind::Pow(a, b) => {
            expr_has_imaginary(graph, *a) || expr_has_imaginary(graph, *b)
        }
        ExprKind::Function { args, .. } => args.iter().any(|&a| expr_has_imaginary(graph, a)),
        _ => false,
    }
}
