//! # Foundational Regression Test Suite for `algebra-core`
//!
//! Tests covering:
//! - AST construction and `ExprGraph` DAG interning / hash-consing deduplication.
//! - Parser round-tripping across expressions and operations.
//! - Schwartz-Zippel probabilistic identity verification and candidate heuristic weighting.
//! - Bytecode VM compilation, scalar evaluation, batch execution, and dual number automatic differentiation.

use algebra_core::format::{Formatter, LatexFormatter};
use algebra_core::parser::ExprParser;
use algebra_core::probabilistic::ProbabilisticVerifier;
use algebra_core::vm::{BytecodeVM, DualNumber, VmCompiler};
use algebra_core::{ExprGraph, ExprKind};

#[test]
fn test_expr_graph_hash_consing_deduplication() {
    let graph = ExprGraph::new();

    // Intern identical symbols
    let x1 = graph.symbol("x");
    let x2 = graph.symbol("x");
    assert_eq!(x1, x2, "Symbol interning must return identical ExprId");

    // Intern identical numbers
    let n1 = graph.integer(42);
    let n2 = graph.integer(42);
    assert_eq!(n1, n2, "Integer interning must return identical ExprId");

    // Intern compound operations
    let add1 = graph.add([x1, n1]);
    let add2 = graph.add([x2, n2]);
    assert_eq!(add1, add2, "Compound Add expression must be hash-consed to the same ExprId");

    // Intern power and multiplication
    let pow1 = graph.pow(x1, n1);
    let pow2 = graph.pow(x2, n2);
    assert_eq!(pow1, pow2, "Pow expression must be deduplicated");

    let mul1 = graph.mul([x1, pow1]);
    let mul2 = graph.mul([x2, pow2]);
    assert_eq!(mul1, mul2, "Mul expression must be deduplicated");

    // Check node retrieval
    let node = graph.get(mul1);
    match node.kind {
        ExprKind::Mul(ref factors) => {
            assert_eq!(factors.len(), 2);
            assert_eq!(factors[0], x1);
            assert_eq!(factors[1], pow1);
        }
        _ => panic!("Expected Mul node"),
    }
}

#[test]
fn test_parser_and_latex_formatter_roundtrip() {
    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let formatter = LatexFormatter;

    // Parse simple polynomial
    let input = "2 * x^2 + 3 * x - 5";
    let expr_id = parser.parse(input).expect("Failed to parse polynomial expression");

    let latex = formatter.format(&graph, expr_id).expect("Formatting failed");
    assert!(!latex.is_empty());
    assert!(latex.contains("x"));

    // Parse rational function with division
    let rational_input = "(x^2 - 1) / (x + 1)";
    let rat_id = parser.parse(rational_input).expect("Failed to parse rational expression");
    let rat_latex = formatter.format(&graph, rat_id).expect("Formatting failed");
    assert!(rat_latex.contains("frac") || rat_latex.contains("/"));
}

#[test]
fn test_schwartz_zippel_probabilistic_identity_verification() {
    let graph = ExprGraph::new();

    // Verify identity: (x - 1) * (x + 1) == x^2 - 1
    let x = graph.symbol("x");
    let one = graph.integer(1);
    let two = graph.integer(2);

    let x_minus_1 = graph.sub(x, one);
    let x_plus_1 = graph.add([x, one]);
    let lhs = graph.mul([x_minus_1, x_plus_1]);

    let x_sq = graph.pow(x, two);
    let rhs = graph.sub(x_sq, one);

    // Run probabilistic verification with target epsilon = 1e-12
    let solution = ProbabilisticVerifier::verify_equivalence(&graph, lhs, rhs, 1e-12);
    assert!(*solution.as_ref(), "Probabilistic verifier must confirm algebraic equivalence of (x-1)(x+1) and x^2-1");
    assert!(solution.error_probability() <= 1e-12);

    // Negative check: x^2 - 1 != x^2 + 1
    let not_rhs = graph.add([x_sq, one]);
    let not_solution = ProbabilisticVerifier::verify_equivalence(&graph, lhs, not_rhs, 1e-6);
    assert!(!*not_solution.as_ref(), "Probabilistic verifier must reject unequal expressions");
}

#[test]
fn test_candidate_heuristic_weight_calculation() {
    let graph = ExprGraph::new();

    let x = graph.symbol("x");
    let zero = graph.integer(0);
    let simple_expr = graph.add([x, zero]);

    let weight = ProbabilisticVerifier::compute_candidate_heuristic_weight(&graph, simple_expr, Some(x));
    assert!(weight > 0.0, "Candidate heuristic weight must be strictly positive");
}

#[test]
fn test_vm_compiler_and_scalar_eval() {
    let graph = ExprGraph::new();
    // Construct: f(x) = 2.0 * x^2 + 3.0 * x - 5.0
    let two = graph.float(2.0);
    let x = graph.symbol("x");
    let two_exp = graph.integer(2);
    let x_sq = graph.pow(x, two_exp);
    let term1 = graph.mul([two, x_sq]);

    let three = graph.float(3.0);
    let term2 = graph.mul([three, x]);

    let sum1 = graph.add([term1, term2]);
    let five = graph.float(5.0);
    let root = graph.sub(sum1, five);

    let program = VmCompiler::compile(&graph, root, "x", &[]).unwrap();
    let vm = BytecodeVM::new();

    // f(0) = -5
    assert_eq!(vm.eval_scalar(&program, 0.0, &[]).unwrap(), -5.0);
    // f(1) = 2(1) + 3(1) - 5 = 0
    assert_eq!(vm.eval_scalar(&program, 1.0, &[]).unwrap(), 0.0);
    // f(2) = 2(4) + 3(2) - 5 = 8 + 6 - 5 = 9
    assert_eq!(vm.eval_scalar(&program, 2.0, &[]).unwrap(), 9.0);
}

#[test]
fn test_vm_batch_eval() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let sin_x = graph.function("sin", [x]);

    let program = VmCompiler::compile(&graph, sin_x, "x", &[]).unwrap();
    let vm = BytecodeVM::new();

    let xs = vec![0.0, std::f64::consts::FRAC_PI_2, std::f64::consts::PI];
    let mut ys = vec![0.0; 3];
    vm.eval_batch(&program, &xs, &[], &mut ys).unwrap();

    assert!((ys[0] - 0.0).abs() < 1e-10);
    assert!((ys[1] - 1.0).abs() < 1e-10);
    assert!((ys[2] - 0.0).abs() < 1e-10);
}

#[test]
fn test_vm_dual_number_derivative() {
    let graph = ExprGraph::new();
    // f(x) = x^3 => f'(x) = 3x^2
    let x = graph.symbol("x");
    let three = graph.integer(3);
    let x_cube = graph.pow(x, three);

    let program = VmCompiler::compile(&graph, x_cube, "x", &[]).unwrap();
    let vm = BytecodeVM::new();

    let x_dual = DualNumber::variable(2.0); // x = 2 + 1*eps
    let res = vm.eval_dual(&program, x_dual, &[]).unwrap();

    assert_eq!(res.real, 8.0); // 2^3 = 8
    assert_eq!(res.dual, 12.0); // 3*(2^2) = 12
}
