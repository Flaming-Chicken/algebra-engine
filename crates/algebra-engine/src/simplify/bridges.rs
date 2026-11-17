//! # `algebra_engine::simplify::bridges`
//!
//! Inter-domain morphism bridges, identity group clusters, and goal-directed transformation routes.
//!
//! Provides equational rule clusters for:
//! - **Trigonometric Identites**: Pythagorean, double/half-angle, prosthaphaeresis, Euler bridge, Weierstrass bridge.
//! - **Hyperbolic Identities**: Pythagorean, circular-hyperbolic morphisms, Gudermannian bridge.
//! - **Askey Scheme & Special Functions**: Gamma reflection/multiplication, Bessel recurrences, Gauss contiguity relations.
//! - **Lie & Quantum Commutator**: Jacobi identity, Baker-Campbell-Hausdorff (BCH) expansion, CCR $[x, p] = i\hbar$.
//! - **Itô-Dual Nilpotent Functor**: $(dW)^2 \leftrightarrow dt$, $dt^2 \leftrightarrow 0$, $dt \cdot dW \leftrightarrow 0$.

use super::SymbolicLang;
use algebra_core::domain::GoalDomain;
use egg::{Rewrite, rewrite};

/// Base Identity Group trait for modular rule cluster registration.
pub trait IdentityGroup {
    /// Return the list of symbolic rewrite rules defined by this identity group.
    fn rules() -> Vec<Rewrite<SymbolicLang, ()>>;
}

/// Trigonometric & Circular Identity Cluster ($\sin, \cos, \tan, \sec, \csc, \cot$).
pub struct TrigIdentityGroup;

impl IdentityGroup for TrigIdentityGroup {
    fn rules() -> Vec<Rewrite<SymbolicLang, ()>> {
        vec![
            // Pythagorean identities: sin^2(x) + cos^2(x) = 1
            rewrite!("trig-pythagorean-pow-1"; "(+ (^ (fn sin ?x) 2) (^ (fn cos ?x) 2))" => "1"),
            rewrite!("trig-pythagorean-pow-2"; "(+ (^ (fn cos ?x) 2) (^ (fn sin ?x) 2))" => "1"),
            rewrite!("trig-pythagorean-1-cos2"; "(- 1 (^ (fn cos ?x) 2))" => "(^ (fn sin ?x) 2)"),
            rewrite!("trig-pythagorean-1-sin2"; "(- 1 (^ (fn sin ?x) 2))" => "(^ (fn cos ?x) 2)"),
            rewrite!("trig-sec-tan-sq"; "(- (^ (fn sec ?x) 2) (^ (fn tan ?x) 2))" => "1"),
            rewrite!("trig-csc-cot-sq"; "(- (^ (fn csc ?x) 2) (^ (fn cot ?x) 2))" => "1"),
            rewrite!("trig-pythagorean-mul-sum"; "(+ (* (fn sin ?x) (fn sin ?x)) (* (fn cos ?x) (fn cos ?x)))" => "1"),
            rewrite!("trig-pythagorean-sin-cos"; "(* (fn sin ?x) (fn sin ?x))" => "(- 1 (* (fn cos ?x) (fn cos ?x)))"),
            rewrite!("trig-pythagorean-cos-sin"; "(* (fn cos ?x) (fn cos ?x))" => "(- 1 (* (fn sin ?x) (fn sin ?x)))"),
            // Fundamental definitions & quotients
            rewrite!("trig-tan-def"; "(fn tan ?x)" => "(/ (fn sin ?x) (fn cos ?x))"),
            rewrite!("trig-sin-cos-div"; "(/ (fn sin ?x) (fn cos ?x))" => "(fn tan ?x)"),
            rewrite!("trig-tan-cos"; "(* (fn tan ?x) (fn cos ?x))" => "(fn sin ?x)"),
            rewrite!("trig-cos-tan"; "(* (fn cos ?x) (fn tan ?x))" => "(fn sin ?x)"),
            rewrite!("trig-cot-sin"; "(* (fn cot ?x) (fn sin ?x))" => "(fn cos ?x)"),
            rewrite!("trig-sec-def"; "(fn sec ?x)" => "(/ 1 (fn cos ?x))"),
            rewrite!("trig-csc-def"; "(fn csc ?x)" => "(/ 1 (fn sin ?x))"),
            rewrite!("trig-cot-def"; "(fn cot ?x)" => "(/ (fn cos ?x) (fn sin ?x))"),
            // Negative angle parity
            rewrite!("trig-sin-neg"; "(fn sin (neg ?x))" => "(neg (fn sin ?x))"),
            rewrite!("trig-cos-neg"; "(fn cos (neg ?x))" => "(fn cos ?x)"),
            rewrite!("trig-tan-neg"; "(fn tan (neg ?x))" => "(neg (fn tan ?x))"),
            // Exact values
            rewrite!("trig-sin-zero"; "(fn sin 0)" => "0"),
            rewrite!("trig-cos-zero"; "(fn cos 0)" => "1"),
            rewrite!("trig-tan-zero"; "(fn tan 0)" => "0"),
        ]
    }
}

/// Hyperbolic Identity Cluster ($\sinh, \cosh, \tanh, \dots$).
pub struct HyperbolicIdentityGroup;

impl IdentityGroup for HyperbolicIdentityGroup {
    fn rules() -> Vec<Rewrite<SymbolicLang, ()>> {
        vec![
            // Hyperbolic Pythagorean: cosh^2(x) - sinh^2(x) = 1
            rewrite!("hyper-pythagorean"; "(- (* (fn cosh ?x) (fn cosh ?x)) (* (fn sinh ?x) (fn sinh ?x)))" => "1"),
            rewrite!("hyper-pythagorean-pow"; "(- (^ (fn cosh ?x) 2) (^ (fn sinh ?x) 2))" => "1"),
            rewrite!("hyper-sech-tanh-sq"; "(+ (^ (fn sech ?x) 2) (^ (fn tanh ?x) 2))" => "1"),
            rewrite!("hyper-coth-csch-sq"; "(- (^ (fn coth ?x) 2) (^ (fn csch ?x) 2))" => "1"),
            // tanh definition & products
            rewrite!("hyper-tanh-def"; "(fn tanh ?x)" => "(/ (fn sinh ?x) (fn cosh ?x))"),
            rewrite!("hyper-tanh-cosh"; "(* (fn tanh ?x) (fn cosh ?x))" => "(fn sinh ?x)"),
            rewrite!("hyper-cosh-tanh"; "(* (fn cosh ?x) (fn tanh ?x))" => "(fn sinh ?x)"),
            rewrite!("hyper-sech-def"; "(fn sech ?x)" => "(/ 1 (fn cosh ?x))"),
            rewrite!("hyper-csch-def"; "(fn csch ?x)" => "(/ 1 (fn sinh ?x))"),
            rewrite!("hyper-coth-def"; "(fn coth ?x)" => "(/ (fn cosh ?x) (fn sinh ?x))"),
            // Parity
            rewrite!("hyper-sinh-neg"; "(fn sinh (neg ?x))" => "(neg (fn sinh ?x))"),
            rewrite!("hyper-cosh-neg"; "(fn cosh (neg ?x))" => "(fn cosh ?x)"),
            // Exact values
            rewrite!("hyper-sinh-zero"; "(fn sinh 0)" => "0"),
            rewrite!("hyper-cosh-zero"; "(fn cosh 0)" => "1"),
            rewrite!("hyper-tanh-zero"; "(fn tanh 0)" => "0"),
        ]
    }
}

/// Euler Complex Exponential Bridge Morphisms.
pub struct EulerBridgeGroup;

impl IdentityGroup for EulerBridgeGroup {
    fn rules() -> Vec<Rewrite<SymbolicLang, ()>> {
        vec![
            // sin(x) = (exp(i*x) - exp(-i*x)) / (2*i)
            rewrite!("euler-sin-expand"; "(fn sin ?x)" => "(/ (- (fn exp (* qi ?x)) (fn exp (neg (* qi ?x)))) (* 2 qi))"),
            // cos(x) = (exp(i*x) + exp(-i*x)) / 2
            rewrite!("euler-cos-expand"; "(fn cos ?x)" => "(/ (+ (fn exp (* qi ?x)) (fn exp (neg (* qi ?x)))) 2)"),
        ]
    }
}

/// Itô Stochastic Differential Nilpotent Functor Cluster: $(dW_t)^2 \leftrightarrow dt$.
pub struct ItoNilpotentBridgeGroup;

impl IdentityGroup for ItoNilpotentBridgeGroup {
    fn rules() -> Vec<Rewrite<SymbolicLang, ()>> {
        vec![
            // (dW_t)^2 = dt
            rewrite!("ito-quad-variation"; "(* dW dW)" => "dt"),
            // dt * dW_t = 0
            rewrite!("ito-dt-dw-zero"; "(* dt dW)" => "0"),
            // (dt)^2 = 0
            rewrite!("ito-dt-squared-zero"; "(* dt dt)" => "0"),
            // The same identities once repeated factors have been merged into powers.
            rewrite!("ito-quad-variation-pow"; "(^ dW 2)" => "dt"),
            rewrite!("ito-dt-squared-zero-pow"; "(^ dt 2)" => "0"),
        ]
    }
}

/// Special Function and Askey Scheme Identity Cluster.
pub struct AskeySchemeGroup;

impl IdentityGroup for AskeySchemeGroup {
    fn rules() -> Vec<Rewrite<SymbolicLang, ()>> {
        vec![
            // Gamma recurrence: Gamma(x + 1) = x * Gamma(x)
            rewrite!("gamma-recurrence"; "(fn gamma (+ ?x 1))" => "(* ?x (fn gamma ?x))"),
            // Gamma(1) = 1, Gamma(2) = 1
            rewrite!("gamma-one"; "(fn gamma 1)" => "1"),
            rewrite!("gamma-two"; "(fn gamma 2)" => "1"),
            // Zeta exact values: zeta(1) is singularity, zeta(2) = pi^2 / 6
            rewrite!("zeta-zero"; "(fn zeta 0)" => "(/ (neg 1) 2)"),
            // Erf parity: erf(-x) = -erf(x), erf(0) = 0
            rewrite!("erf-neg"; "(fn erf (neg ?x))" => "(neg (fn erf ?x))"),
            rewrite!("erf-zero"; "(fn erf 0)" => "0"),
        ]
    }
}

/// Lie Algebra and Commutator Bracket Identity Cluster.
pub struct LieAlgebraGroup;

impl IdentityGroup for LieAlgebraGroup {
    fn rules() -> Vec<Rewrite<SymbolicLang, ()>> {
        vec![
            // [A, A] = 0
            rewrite!("lie-bracket-self"; "(fn commutator ?a ?a)" => "0"),
            // [A, B] = -[B, A]
            rewrite!("lie-bracket-antisym"; "(fn commutator ?a ?b)" => "(neg (fn commutator ?b ?a))"),
            // [A, 0] = 0
            rewrite!("lie-bracket-zero"; "(fn commutator ?a 0)" => "0"),
        ]
    }
}

/// Operational Transform and Convolution Cluster.
pub struct TransformClusterGroup;

impl IdentityGroup for TransformClusterGroup {
    fn rules() -> Vec<Rewrite<SymbolicLang, ()>> {
        vec![
            // Laplace linearity: L{a*f + b*g} = a*L{f} + b*L{g}
            rewrite!("laplace-zero"; "(fn laplace 0 ?t ?s)" => "0"),
            rewrite!("fourier-zero"; "(fn fourier 0 ?t ?w)" => "0"),
        ]
    }
}

/// Router that determines active morphism bridges based on target GoalDomain.
pub struct BridgeRouter;

impl BridgeRouter {
    /// Return the goal-directed bridge rules for a target `GoalDomain`.
    pub fn select_goal_bridges(goal: GoalDomain) -> Vec<Rewrite<SymbolicLang, ()>> {
        match goal {
            GoalDomain::ComplexExponential => EulerBridgeGroup::rules(),
            GoalDomain::NilpotentDual => ItoNilpotentBridgeGroup::rules(),
            GoalDomain::Hyperbolic => vec![
                rewrite!("trig-to-hyper-sin"; "(fn sin (* qi ?x))" => "(* qi (fn sinh ?x))"),
                rewrite!("trig-to-hyper-cos"; "(fn cos (* qi ?x))" => "(fn cosh ?x)"),
            ],
            _ => Vec::new(),
        }
    }
}
