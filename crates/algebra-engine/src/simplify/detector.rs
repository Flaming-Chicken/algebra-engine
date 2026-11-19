//! # `algebra_engine::simplify::detector`
//!
//! Context-aware AST scanner and domain feature detector for symbolic expressions.
//!
//! Identifies mathematical signatures (e.g. Wiener processes, quaternions, wedge products,
//! differential forms, commutator brackets, trigonometric/hyperbolic functions) to drive
//! conflict-free dynamic rule synthesis.

use algebra_core::{ExprGraph, ExprId, ExprKind};

/// Detected mathematical features present in an expression DAG.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DetectedFeatures {
    /// Expression involves basic ring/field arithmetic.
    pub is_commutative_ring: bool,
    /// Expression contains non-commutative entities (quaternions, matrices, operators).
    pub is_non_commutative_ring: bool,
    /// Expression involves tropical (max-plus/min-plus) operators.
    pub is_tropical: bool,
    /// Expression involves stochastic differentials ($dW_t, W_t$) requiring Itô calculus.
    pub has_stochastic_terms: bool,
    /// Expression involves discrete / umbral operators (shift $E$, difference $\Delta$, falling factorials).
    pub has_umbral_terms: bool,
    /// Expression involves fractional calculus operators (Caputo, Riemann-Liouville).
    pub has_fractional_terms: bool,
    /// Expression contains exterior products ($\wedge$) or Clifford multivector blades.
    pub has_wedge_or_clifford: bool,
    /// Expression contains Lie brackets / quantum commutator brackets ($[A, B]$).
    pub has_commutator_brackets: bool,
    /// Expression contains trigonometric functions ($\sin, \cos, \tan, \dots$).
    pub has_trigonometric: bool,
    /// Expression contains hyperbolic functions ($\sinh, \cosh, \tanh, \dots$).
    pub has_hyperbolic: bool,
    /// Expression contains special functions (Bessel, Gamma, Zeta, Hypergeometric).
    pub has_special_functions: bool,
    /// Expression contains integral transform symbols ($\mathcal{L}, \mathcal{F}, \dots$).
    pub has_transforms: bool,
}

/// AST Scanner that inspects an expression DAG to determine active mathematical domains.
pub struct DomainFeatureDetector;

impl DomainFeatureDetector {
    /// Scan the expression DAG rooted at `root` and extract all mathematical domain features.
    pub fn scan(graph: &ExprGraph, root: ExprId) -> DetectedFeatures {
        let mut features = DetectedFeatures {
            is_commutative_ring: true,
            ..Default::default()
        };

        Self::traverse(graph, root, &mut features);

        if features.is_non_commutative_ring {
            features.is_commutative_ring = false;
        }

        features
    }

    fn traverse(graph: &ExprGraph, expr_id: ExprId, features: &mut DetectedFeatures) {
        let node = graph.get(expr_id);

        match &node.kind {
            ExprKind::Symbol(sym_id) => {
                if let Some(name) = graph.symbols.resolve(*sym_id) {
                    let n = name.as_str();
                    if n == "dW"
                        || n == "dW_t"
                        || n == "W_t"
                        || n.starts_with("dW")
                        || n.starts_with("W_")
                    {
                        features.has_stochastic_terms = true;
                    }
                    if n == "i" || n == "qi" || n == "qj" || n == "qk" || n == "I" {
                        features.is_non_commutative_ring = true;
                    }
                    if n == "Delta" || n == "nabla" || n.starts_with("shift_") {
                        features.has_umbral_terms = true;
                    }
                }
            }
            ExprKind::Function { name, args } => {
                if let Some(f_name_str) = graph.symbols.resolve(*name) {
                    let f_name = f_name_str.as_str();
                    match f_name {
                        "sin" | "cos" | "tan" | "sec" | "csc" | "cot" | "arcsin" | "arccos"
                        | "arctan" => {
                            features.has_trigonometric = true;
                        }
                        "sinh" | "cosh" | "tanh" | "sech" | "csch" | "coth" | "arsinh"
                        | "arcosh" | "artanh" | "gd" => {
                            features.has_hyperbolic = true;
                        }
                        "bessel_j" | "bessel_y" | "bessel_i" | "bessel_k" | "gamma" | "beta"
                        | "zeta" | "hypergeometric" | "erf" | "airy_ai" | "airy_bi" => {
                            features.has_special_functions = true;
                        }
                        "laplace" | "fourier" | "inv_laplace" | "mellin" => {
                            features.has_transforms = true;
                        }
                        "commutator" | "lie_bracket" => {
                            features.has_commutator_brackets = true;
                            features.is_non_commutative_ring = true;
                        }
                        "wedge" | "clifford_mul" => {
                            features.has_wedge_or_clifford = true;
                            features.is_non_commutative_ring = true;
                        }
                        "ito_diff" | "wiener" => {
                            features.has_stochastic_terms = true;
                        }
                        "caputo" | "riemann_liouville" => {
                            features.has_fractional_terms = true;
                        }
                        _ => {}
                    }
                }
                for arg in args.iter() {
                    Self::traverse(graph, *arg, features);
                }
            }
            ExprKind::Add(args) | ExprKind::Mul(args) => {
                for arg in args.iter() {
                    Self::traverse(graph, *arg, features);
                }
            }
            ExprKind::Sub(lhs, rhs) | ExprKind::Div(lhs, rhs) | ExprKind::Pow(lhs, rhs) => {
                Self::traverse(graph, *lhs, features);
                Self::traverse(graph, *rhs, features);
            }
            ExprKind::Neg(arg) => {
                Self::traverse(graph, *arg, features);
            }
            ExprKind::Matrix { elements, .. } => {
                features.is_non_commutative_ring = true;
                for elem in elements.iter() {
                    Self::traverse(graph, *elem, features);
                }
            }
            ExprKind::TensorContraction {
                tensor_a, tensor_b, ..
            } => {
                features.is_non_commutative_ring = true;
                Self::traverse(graph, *tensor_a, features);
                Self::traverse(graph, *tensor_b, features);
            }
            _ => {}
        }
    }
}
