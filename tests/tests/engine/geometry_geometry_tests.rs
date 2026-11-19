use algebra_core::ExprGraph;
use algebra_engine::geometry::ResultantBuilder;

#[test]
fn test_quadratic_discriminant() {
    use algebra_engine::numeric::{EvalContext, NumericalEval};
    use std::collections::HashMap;

    let graph = ExprGraph::new();
    let a = graph.symbol("a");
    let b = graph.symbol("b");
    let c = graph.symbol("c");

    // Delta = b^2 - 4ac -> for a=2, b=5, c=2 => 25 - 16 = 9
    let disc = ResultantBuilder::quadratic_discriminant(&graph, a, b, c);

    let mut bindings = HashMap::new();
    bindings.insert("a".to_string(), 2.0);
    bindings.insert("b".to_string(), 5.0);
    bindings.insert("c".to_string(), 2.0);
    let ctx = EvalContext::with_bindings(53, bindings);
    let val = graph.evalf(disc, &ctx).unwrap().to_f64();
    assert!((val - 9.0).abs() < 1e-12, "expected 9.0, got {val}");
}

#[test]
fn test_sylvester_matrix_construction() {
    let graph = ExprGraph::new();
    let a = graph.symbol("a");
    let b = graph.symbol("b");
    let c = graph.symbol("c");
    let d = graph.symbol("d");
    let e = graph.symbol("e");

    let mat = ResultantBuilder::sylvester_matrix_2_1(&graph, [a, b, c], [d, e]).unwrap();

    assert_eq!(mat.rows, 3);
    assert_eq!(mat.cols, 3);
    assert_eq!(mat.get(0, 0), a);
    assert_eq!(mat.get(0, 1), b);
    assert_eq!(mat.get(0, 2), c);
    assert_eq!(mat.get(1, 0), d);
    assert_eq!(mat.get(1, 1), e);
}

#[test]
fn test_arbitrary_curvilinear_spherical_laplacian() {
    use algebra_engine::geometry::{ArbitraryCurvilinearSystem, CoordinateTransform};
    use algebra_engine::numeric::{EvalContext, NumericalEval};
    use std::collections::HashMap;

    let graph = ExprGraph::new();
    let sys = ArbitraryCurvilinearSystem::spherical_3d(&graph);

    let r_sym = graph.symbols.get_or_intern("r");
    let theta_sym = graph.symbols.get_or_intern("theta");
    let phi_sym = graph.symbols.get_or_intern("phi");

    let r = graph.symbol("r");
    let scalar_field = graph.pow(r, graph.integer(2));

    // In 3D spherical coordinates, Laplacian(r^2) = 6 identically
    let lap =
        CoordinateTransform::laplacian(&graph, &sys, scalar_field, &[r_sym, theta_sym, phi_sym]);

    let mut bindings = HashMap::new();
    bindings.insert("r".to_string(), 2.5);
    bindings.insert("theta".to_string(), 0.7);
    bindings.insert("phi".to_string(), 1.1);
    let ctx = EvalContext::with_bindings(53, bindings);
    let lap_val = graph.evalf(lap, &ctx).unwrap().to_f64();
    assert!(
        (lap_val - 6.0).abs() < 1e-10,
        "expected Laplacian(r^2)=6, got {lap_val}"
    );
}

#[test]
fn test_csg_boolean_operations() {
    use algebra_engine::geometry::{Circle2D, CsgNode, Point2D};

    let c1 = Circle2D::new(0.0, 0.0, 5.0);
    let c2 = Circle2D::new(3.0, 0.0, 5.0);

    let p = Point2D { x: 1.0, y: 0.0 };

    let csg_intersection =
        CsgNode::Intersection(Box::new(CsgNode::Circle(c1)), Box::new(CsgNode::Circle(c2)));

    assert!(csg_intersection.contains(&p));
}
