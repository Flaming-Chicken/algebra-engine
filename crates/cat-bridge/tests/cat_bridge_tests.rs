use algebra_core::graph::ExprGraph;
use algebra_core::number::{Constant, Number};
use cat_bridge::*;
use cat_core::CatRuntimeContext;

#[test]
fn test_roundtrip_integers_and_rationals() {
    let mut ctx = CatRuntimeContext::new();
    let mut bridge = LegacyBridgeContext::new();

    // Integer
    let int_ast = LegacyAstTree::Number(Number::Integer(42));
    let status = ShadowVerifyHarness::verify_roundtrip(&int_ast, &mut ctx, &mut bridge)
        .expect("Roundtrip verify should succeed");
    assert_eq!(status, MigrationVerifyStatus::ExactMatch);

    // Rational
    let rat_ast = LegacyAstTree::Number(Number::rational(355, 113));
    let status = ShadowVerifyHarness::verify_roundtrip(&rat_ast, &mut ctx, &mut bridge)
        .expect("Roundtrip verify should succeed");
    assert_eq!(status, MigrationVerifyStatus::ExactMatch);
}

#[test]
fn test_complex_types_lifting_and_lowering() {
    let mut ctx = CatRuntimeContext::new();
    let mut bridge = LegacyBridgeContext::new();

    // Complex: 3 + 4i represented as Add([3, Mul([4, I])])
    let complex_ast = LegacyAstTree::Add(vec![
        LegacyAstTree::Number(Number::Integer(3)),
        LegacyAstTree::Mul(vec![
            LegacyAstTree::Number(Number::Integer(4)),
            LegacyAstTree::Number(Number::Constant(Constant::I)),
        ]),
    ]);

    // Lift to CatTerm: factors into Layer 1 GaussianInt spine
    let cat_term = complex_ast
        .lift_to_cat(&mut ctx, &mut bridge)
        .expect("Lift complex to cat should succeed");

    let scalar = ctx.get_scalar(cat_term.spine.scalar).unwrap();
    assert_eq!(scalar, cat_core::ScalarAtom::gaussian_int(3, 4));

    // Lower back to LegacyAstTree
    let lowered = LegacyAstTree::lower_from_cat(&cat_term, &ctx, &bridge)
        .expect("Lower complex from cat should succeed");
    assert_eq!(lowered, complex_ast, "Lowered complex number must match original legacy AST structure");

    // Exact Gaussian Rational: 1/2 + 3/4 i
    let complex_rat_ast = LegacyAstTree::Add(vec![
        LegacyAstTree::Number(Number::rational(1, 2)),
        LegacyAstTree::Mul(vec![
            LegacyAstTree::Number(Number::rational(3, 4)),
            LegacyAstTree::Number(Number::Constant(Constant::I)),
        ]),
    ]);
    let cat_rat = complex_rat_ast
        .lift_to_cat(&mut ctx, &mut bridge)
        .expect("Lift rational complex to cat should succeed");
    let lowered_rat = LegacyAstTree::lower_from_cat(&cat_rat, &ctx, &bridge)
        .expect("Lower rational complex from cat should succeed");
    assert_eq!(lowered_rat, complex_rat_ast);
}

#[test]
fn test_tensor_contraction_dummy_index_erasure() {
    let mut ctx = CatRuntimeContext::new();
    let mut bridge = LegacyBridgeContext::new();

    // Tensor Contraction with named indices (mu, nu) -> Einstein contraction
    let tensor_contraction = LegacyAstTree::TensorContraction {
        tensor_a: Box::new(LegacyAstTree::Symbol("R_mu_nu".into())),
        tensor_b: Box::new(LegacyAstTree::Symbol("g_inv_mu_nu".into())),
        contracted_indices: vec![("mu".into(), "mu".into()), ("nu".into(), "nu".into())],
    };

    let cat_term = tensor_contraction
        .lift_to_cat(&mut ctx, &mut bridge)
        .expect("Lift tensor contraction should succeed");

    // Verify named dummy indices were erased and logged
    assert!(
        !bridge.telemetry_audit.is_empty(),
        "Bridge must log dummy index erasure telemetry"
    );

    let diag = ctx.get_diagram(cat_term.diagram).unwrap();
    assert_eq!(diag.edges.len(), 3, "Diag contains tensor_a, tensor_b, and Contraction edges");
    let contraction_edge = diag
        .edges
        .iter()
        .find(|e| matches!(&e.op, cat_core::HyperOp::Contraction { .. }))
        .expect("Expected Contraction HyperOp");

    match &contraction_edge.op {
        cat_core::HyperOp::Contraction { pairs } => {
            assert_eq!(pairs.len(), 2, "Must have 2 internal anonymous wire pairs contracted");
        }
        _ => unreachable!(),
    }

    // Lowering synthesizes canonical dummy index names (_w0, _w1, ...)
    let lowered = LegacyAstTree::lower_from_cat(&cat_term, &ctx, &bridge)
        .expect("Lower tensor contraction should succeed");

    match lowered {
        LegacyAstTree::TensorContraction { contracted_indices, .. } => {
            assert_eq!(contracted_indices.len(), 2);
            assert_eq!(contracted_indices[0], ("_w0".into(), "_w1".into()));
            assert_eq!(contracted_indices[1], ("_w2".into(), "_w3".into()));
        }
        _ => panic!("Expected lowered TensorContraction"),
    }
}

#[test]
fn test_expr_graph_roundtrip_integration() {
    let graph = ExprGraph::new();
    let mut ctx = CatRuntimeContext::new();
    let mut bridge = LegacyBridgeContext::new();

    // Construct expression in ExprGraph: (x * 2)
    let sym_x = graph.symbol("x");
    let num_2 = graph.number(Number::Integer(2));
    let expr_id = graph.mul(vec![num_2, sym_x]);

    // Convert ExprGraph -> LegacyAstTree -> CatTerm -> LegacyAstTree -> ExprGraph
    let tree = LegacyAstTree::from_graph(expr_id, &graph).expect("From graph should succeed");
    let cat_term = tree.lift_to_cat(&mut ctx, &mut bridge).expect("Lift should succeed");
    let lowered_tree = LegacyAstTree::lower_from_cat(&cat_term, &ctx, &bridge).expect("Lower should succeed");

    let dest_graph = ExprGraph::new();
    let restored_id = lowered_tree.to_graph(&dest_graph);

    // Verify restored expression behaves identically
    let orig_node = graph.get(expr_id);
    let rest_node = dest_graph.get(restored_id);
    assert_eq!(orig_node.domain, rest_node.domain);
}
