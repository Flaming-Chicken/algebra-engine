use algebra_core::ExprGraph;
use algebra_core::operation::*;
use algebra_core::parser::parse_operation;
use algebra_engine::executor::{ExecutionContext, OperationExecutor};

#[test]
fn test_executor_calculus_and_transformations() {
    let graph = ExprGraph::new();
    let ctx = ExecutionContext::new();

    // Differentiate higher order
    let op_diff2 = parse_operation("diff x^4, x, 2");
    let res_diff2 = OperationExecutor::execute(&graph, &op_diff2, &ctx);
    assert!(!res_diff2.is_error);
    assert!(res_diff2.output_text.contains('x'));

    // Simplify Expand
    let op_exp = parse_operation("expand (x + 1)^2");
    let res_exp = OperationExecutor::execute(&graph, &op_exp, &ctx);
    assert!(!res_exp.is_error);
    assert!(res_exp.output_text.contains("2") && res_exp.output_text.contains("x"));

    // Solve linear equation
    let op_solve = parse_operation("solve 2*x - 6 = 0, x");
    let res_solve = OperationExecutor::execute(&graph, &op_solve, &ctx);
    assert!(!res_solve.is_error);
    assert!(res_solve.output_text.contains("3"));
}

#[test]
fn test_executor_combinatorics_and_physics() {
    let graph = ExprGraph::new();
    let ctx = ExecutionContext::new();

    let op_fact = parse_operation("factorial 5");
    let res_fact = OperationExecutor::execute(&graph, &op_fact, &ctx);
    assert!(!res_fact.is_error);
    assert!(res_fact.output_text.contains("120"));

    let op_cat = parse_operation("catalan 4");
    let res_cat = OperationExecutor::execute(&graph, &op_cat, &ctx);
    assert!(!res_cat.is_error);
    assert!(res_cat.output_text.contains("14"));

    let op_conv = parse_operation("convert 72 [km/h] to [m/s]");
    let res_conv = OperationExecutor::execute(&graph, &op_conv, &ctx);
    assert!(!res_conv.is_error);
    assert!(
        res_conv.output_text.contains("= 20 [m/s]"),
        "{}",
        res_conv.output_text
    );
}

#[test]
fn test_executor_intervals_and_logic() {
    let graph = ExprGraph::new();
    let ctx = ExecutionContext::new();

    let op_sat = parse_operation("sat (A or B)");
    let res_sat = OperationExecutor::execute(&graph, &op_sat, &ctx);
    assert!(!res_sat.is_error);
    assert!(res_sat.output_text.contains("Satisfiable"));
}

#[test]
fn test_executor_esoteric_hyperop() {
    let graph = ExprGraph::new();
    let ctx = ExecutionContext::new();

    let op_tet = parse_operation("tetration 2 3");
    let res_tet = OperationExecutor::execute(&graph, &op_tet, &ctx);
    assert!(!res_tet.is_error);
    assert!(res_tet.output_text.contains("16"));

    let op_ack = parse_operation("ackermann 2 2");
    let res_ack = OperationExecutor::execute(&graph, &op_ack, &ctx);
    assert!(!res_ack.is_error);
    assert!(res_ack.output_text.contains("7"));
}

#[test]
fn test_executor_tropical_and_number_theory() {
    let graph = ExprGraph::new();
    let ctx = ExecutionContext::new();

    let op_mp = parse_operation("maxplus_add 3 5");
    let res_mp = OperationExecutor::execute(&graph, &op_mp, &ctx);
    assert!(!res_mp.is_error);
    assert!(res_mp.output_text.contains("5.0"));

    let op_prime = parse_operation("is_prime 101");
    let res_prime = OperationExecutor::execute(&graph, &op_prime, &ctx);
    assert!(!res_prime.is_error);
    assert!(res_prime.output_text.contains("true"));

    let op_gcd = parse_operation("gcd 30 20");
    let res_gcd = OperationExecutor::execute(&graph, &op_gcd, &ctx);
    assert!(!res_gcd.is_error);
    assert!(res_gcd.output_text.contains("10"));
}

#[test]
fn test_executor_cartan_clifford_certified_and_block_diagram() {
    let graph = ExprGraph::new();
    let ctx = ExecutionContext::new();

    // 1. Cartan Exterior Derivative
    let cartan_op = MathOperation::Cartan(CartanOpKind::ExteriorDerivative {
        form_str: "dx".to_string(),
        dim: 3,
    });
    let res_cartan = OperationExecutor::execute(&graph, &cartan_op, &ctx);
    assert!(!res_cartan.is_error);
    assert!(res_cartan.output_text.contains("Exterior Derivative"));

    // 2. Clifford Rotor Rotation
    let clifford_op = MathOperation::Clifford(CliffordOpKind::RotorSandwich {
        sig_p: 3,
        sig_q: 0,
        sig_r: 0,
        vector: vec![1.0, 0.0, 0.0],
        angle_rad: std::f64::consts::PI / 2.0,
        bivector_plane: (0, 1),
    });
    let res_clifford = OperationExecutor::execute(&graph, &clifford_op, &ctx);
    assert!(!res_clifford.is_error);
    assert!(res_clifford.output_text.contains("Clifford Rotor Rotation"));

    // 3. Certified Simplification
    let cert_op = MathOperation::Certified(CertifiedOpKind::Simplify {
        expression: "(x + 0) * 1".to_string(),
        eps: 1e-15,
    });
    let res_cert = OperationExecutor::execute(&graph, &cert_op, &ctx);
    assert!(!res_cert.is_error);
    assert!(res_cert.output_text.contains("Certified"));

    // 4. Block Diagram Feedback
    let block_op = MathOperation::BlockDiagram(BlockDiagramOpKind::Feedback {
        g_expr: "1/s".to_string(),
        h_expr: "1".to_string(),
    });
    let res_block = OperationExecutor::execute(&graph, &block_op, &ctx);
    assert!(!res_block.is_error);
    assert!(res_block.output_text.contains("Closed Loop"));
}

#[test]
fn test_executor_control_and_system() {
    let graph = ExprGraph::new();
    let mut ctx = ExecutionContext::new();
    ctx.symbol_summary.push((
        "a".to_string(),
        "Parameter".to_string(),
        2.0,
        "m".to_string(),
    ));

    let op_vars = parse_operation("vars");
    let res_vars = OperationExecutor::execute(&graph, &op_vars, &ctx);
    assert!(!res_vars.is_error);
    assert!(res_vars.output_text.contains("a (Parameter): 2.0000 [m]"));

    let op_help = parse_operation("help");
    let res_help = OperationExecutor::execute(&graph, &op_help, &ctx);
    assert!(!res_help.is_error);
    assert!(
        res_help
            .output_text
            .contains("Universal Rust Algebra Engine")
    );
}
