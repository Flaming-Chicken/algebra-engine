//! Integration Tests for Phase 12: Cylindrical Algebraic Decomposition (CAD),
//! Real Quantifier Elimination (QE), and Parametric Geometric Constraint Solving.

use algebra_engine::cad::{
    CadCellType, CadEngine, CadPolynomial, GeometricConstraint, GeometricConstraintSolver,
    GeometricEntity, Quantifier, QuantifierEliminator, RelationalOp, Sign,
};
use algebra_engine::{cad, quantifier_elim};

#[test]
fn test_cad_polynomial_evaluation_and_derivative() {
    let vars = vec!["x".to_string(), "y".to_string()];
    let mut p = CadPolynomial::zero(vars.clone());

    // p(x, y) = 3*x^2*y + 2*x - 5
    let exp1 = vec![2, 1]; // x^2 * y
    let exp2 = vec![1, 0]; // x
    let exp3 = vec![0, 0]; // 1

    p.terms.insert(exp1, 3.0);
    p.terms.insert(exp2, 2.0);
    p.terms.insert(exp3, -5.0);

    // Evaluation at (2, 3): 3*(4)*(3) + 2*(2) - 5 = 36 + 4 - 5 = 35
    assert_eq!(p.evaluate(&[2.0, 3.0]), 35.0);

    // Derivative wrt x (var_idx 0): dp/dx = 6*x*y + 2
    let dp_dx = p.derivative(0);
    assert_eq!(dp_dx.evaluate(&[2.0, 3.0]), 6.0 * 2.0 * 3.0 + 2.0); // 38.0

    // Derivative wrt y (var_idx 1): dp/dy = 3*x^2
    let dp_dy = p.derivative(1);
    assert_eq!(dp_dy.evaluate(&[2.0, 3.0]), 3.0 * 4.0); // 12.0
}

#[test]
fn test_cad_1d_decomposition_univariate() {
    let vars = vec!["x".to_string()];

    // f(x) = x^2 - 4 (roots at -2, 2)
    let mut f = CadPolynomial::zero(vars.clone());
    f.terms.insert(vec![2], 1.0);
    f.terms.insert(vec![0], -4.0);

    let cad_tree = CadEngine::decompose(&[f], &vars, 1e-10).unwrap();

    // 1D decomposition of x^2 - 4 should produce:
    // Sector (-inf, -2), Section {-2}, Sector (-2, 2), Section {2}, Sector (2, +inf) -> 5 cells
    assert_eq!(cad_tree.children.len(), 5);

    assert_eq!(cad_tree.children[0].cell_type, CadCellType::Sector);
    assert_eq!(cad_tree.children[1].cell_type, CadCellType::Section);
    assert_eq!(cad_tree.children[2].cell_type, CadCellType::Sector);
    assert_eq!(cad_tree.children[3].cell_type, CadCellType::Section);
    assert_eq!(cad_tree.children[4].cell_type, CadCellType::Sector);

    // Signs of x^2 - 4 across the 5 cells:
    // Sector (-inf, -2): Positive (+1)
    // Section {-2}: Zero (0)
    // Sector (-2, 2): Negative (-1)
    // Section {2}: Zero (0)
    // Sector (2, +inf): Positive (+1)
    assert_eq!(cad_tree.children[0].signs[0], Sign::Positive);
    assert_eq!(cad_tree.children[1].signs[0], Sign::Zero);
    assert_eq!(cad_tree.children[2].signs[0], Sign::Negative);
    assert_eq!(cad_tree.children[3].signs[0], Sign::Zero);
    assert_eq!(cad_tree.children[4].signs[0], Sign::Positive);
}

#[test]
fn test_cad_2d_decomposition_circle() {
    let vars = vec!["x".to_string(), "y".to_string()];

    // Unit circle: x^2 + y^2 - 1 = 0
    let mut circle = CadPolynomial::zero(vars.clone());
    circle.terms.insert(vec![2, 0], 1.0); // x^2
    circle.terms.insert(vec![0, 2], 1.0); // y^2
    circle.terms.insert(vec![0, 0], -1.0); // -1

    let cad_2d = CadEngine::decompose(&[circle], &vars, 1e-8).unwrap();
    // Exactly 13 cells: fibres of 1, 3, 5, 3 and 1 cells over x < -1, -1, (-1, 1), 1 and x > 1.
    assert_eq!(cad_2d.count_leaf_cells(), 13);
}

#[test]
fn test_quantifier_elimination_existential_and_universal() {
    let vars = vec!["x".to_string()];

    // f(x) = x^2 - 4
    let mut f = CadPolynomial::zero(vars.clone());
    f.terms.insert(vec![2], 1.0);
    f.terms.insert(vec![0], -4.0);

    // 1. Exists x: x^2 - 4 == 0 -> TRUE (roots exist at -2, 2)
    let q_exists = vec![(Quantifier::Exists, "x".to_string())];
    let cond_eq = vec![(0, RelationalOp::Equal)];
    let res_exists =
        QuantifierEliminator::eliminate(&q_exists, &[f.clone()], &cond_eq, &vars).unwrap();
    assert!(res_exists);

    // 2. ForAll x: x^2 - 4 == 0 -> FALSE
    let q_forall = vec![(Quantifier::ForAll, "x".to_string())];
    let res_forall = QuantifierEliminator::eliminate(&q_forall, &[f], &cond_eq, &vars).unwrap();
    assert!(!res_forall);

    // 3. ForAll x: x^2 + 1 > 0 -> TRUE (always positive over R)
    let mut g = CadPolynomial::zero(vars.clone());
    g.terms.insert(vec![2], 1.0);
    g.terms.insert(vec![0], 1.0);

    let cond_gt = vec![(0, RelationalOp::GreaterThan)];
    let res_pos = QuantifierEliminator::eliminate(&q_forall, &[g], &cond_gt, &vars).unwrap();
    assert!(res_pos);
}

#[test]
fn test_parametric_geometric_constraint_solver_2d() {
    let mut solver = GeometricConstraintSolver::new();

    // Entity 1: Fixed origin Point P1 (0, 0)
    solver.add_entity(
        "P1",
        GeometricEntity::Point2D {
            name: "P1".to_string(),
            x: Some(0.0),
            y: Some(0.0),
        },
    );

    // Entity 2: Free Point P2 (?, ?)
    solver.add_entity(
        "P2",
        GeometricEntity::Point2D {
            name: "P2".to_string(),
            x: None,
            y: None,
        },
    );

    // Entity 3: Free Point P3 (?, ?)
    solver.add_entity(
        "P3",
        GeometricEntity::Point2D {
            name: "P3".to_string(),
            x: None,
            y: None,
        },
    );

    // Constraints:
    // Distance(P1, P2) == 10.0
    // Coincident(P2, P3)
    solver.add_constraint(GeometricConstraint::Distance(
        "P1".to_string(),
        "P2".to_string(),
        10.0,
    ));
    solver.add_constraint(GeometricConstraint::Coincident(
        "P2".to_string(),
        "P3".to_string(),
    ));

    let solution = solver.solve().unwrap();

    assert_eq!(solution.get("P1").unwrap(), &vec![0.0, 0.0]);
    assert_eq!(solution.get("P2").unwrap(), &vec![10.0, 0.0]);
    assert_eq!(solution.get("P3").unwrap(), &vec![10.0, 0.0]);
}

#[test]
fn test_dsl_macros_cad_and_quantifier_elim() {
    let vars = vec!["x".to_string()];
    let mut f = CadPolynomial::zero(vars.clone());
    f.terms.insert(vec![2], 1.0);
    f.terms.insert(vec![0], -9.0); // x^2 - 9

    // cad! macro: x^2 - 9 produces 5 cells (-inf..-3, -3, -3..3, 3, 3..+inf)
    let cad_res = cad!(&[f.clone()], &vars).unwrap();
    assert_eq!(cad_res.children.len(), 5);
    assert_eq!(cad_res.children[0].signs[0], Sign::Positive);
    assert_eq!(cad_res.children[1].signs[0], Sign::Zero);
    assert_eq!(cad_res.children[2].signs[0], Sign::Negative);
    assert_eq!(cad_res.children[3].signs[0], Sign::Zero);
    assert_eq!(cad_res.children[4].signs[0], Sign::Positive);

    // Unsupported constraints (Angle/Perpendicular) currently leave free points unsolved (pinned audit finding)
    let mut partial_solver = GeometricConstraintSolver::new();
    partial_solver.add_entity(
        "P1",
        GeometricEntity::Point2D {
            name: "P1".to_string(),
            x: Some(0.0),
            y: Some(0.0),
        },
    );
    partial_solver.add_entity(
        "P2",
        GeometricEntity::Point2D {
            name: "P2".to_string(),
            x: None,
            y: None,
        },
    );
    partial_solver.add_constraint(GeometricConstraint::Angle(
        "P1".to_string(),
        "P2".to_string(),
        45.0,
    ));
    // An angle is a constraint between two lines; between two points it used to be ignored silently and
    // is now reported (audit C3).
    let err = partial_solver
        .solve()
        .expect_err("an angle between points is not defined");
    assert!(err.to_string().contains("not a Line2D"), "{err}");

    // quantifier_elim! macro
    let q_exists = vec![(Quantifier::Exists, "x".to_string())];
    let cond_eq = vec![(0, RelationalOp::Equal)];
    let qe_res = quantifier_elim!(&q_exists, &[f], &cond_eq, &vars).unwrap();
    assert!(qe_res);
}

#[test]
fn test_custom_shape_mesh3d_cube_properties_and_transforms() {
    use algebra_engine::cad::Mesh3D;

    let mut cube = Mesh3D::cube(2.0, 3.0, 4.0);
    assert_eq!(cube.vertices.len(), 8);
    assert_eq!(cube.triangles.len(), 12);

    // Bounding Box
    let bbox = cube.compute_bounding_box();
    assert_eq!(bbox.min, [-1.0, -1.5, -2.0]);
    assert_eq!(bbox.max, [1.0, 1.5, 2.0]);
    assert_eq!(bbox.size(), [2.0, 3.0, 4.0]);

    // Volume: 2 * 3 * 4 = 24.0
    let vol = cube.compute_volume();
    assert!((vol - 24.0).abs() < 1e-10);

    // Surface Area: 2 * (2*3 + 3*4 + 2*4) = 2 * (6 + 12 + 8) = 52.0
    let area = cube.compute_surface_area();
    assert!((area - 52.0).abs() < 1e-10);

    // Translation & Scaling
    cube.translate(10.0, 20.0, 30.0);
    let center = cube.compute_centroid();
    assert!((center[0] - 10.0).abs() < 1e-10);
    assert!((center[1] - 20.0).abs() < 1e-10);
    assert!((center[2] - 30.0).abs() < 1e-10);

    cube.scale(2.0, 2.0, 2.0);
    let vol_scaled = cube.compute_volume();
    assert!((vol_scaled - 24.0 * 8.0).abs() < 1e-9); // 24 * 2^3 = 192.0
}

#[test]
fn test_shape_importer_and_exporter_obj() {
    use algebra_engine::cad::{Mesh3D, ShapeExporter, ShapeImporter};

    let cube = Mesh3D::cube(5.0, 5.0, 5.0);
    let obj_str = ShapeExporter::export_obj(&cube);

    assert!(obj_str.contains("v 2.500000 2.500000 2.500000"));
    assert!(obj_str.contains("f "));

    // Re-import OBJ
    let imported = ShapeImporter::import_obj(&obj_str).unwrap();
    assert_eq!(imported.vertices.len(), 8);
    assert_eq!(imported.triangles.len(), 12);
    assert!((imported.compute_volume() - 125.0).abs() < 1e-9);
}

#[test]
fn test_shape_importer_stl_ascii() {
    use algebra_engine::cad::ShapeImporter;

    let stl_snippet = r#"
solid tetrahedron
  facet normal 0 0 0
    outer loop
      vertex 0.0 0.0 0.0
      vertex 1.0 0.0 0.0
      vertex 0.0 1.0 0.0
    endloop
  endfacet
  facet normal 0 0 0
    outer loop
      vertex 0.0 0.0 0.0
      vertex 0.0 1.0 0.0
      vertex 0.0 0.0 1.0
    endloop
  endfacet
  facet normal 0 0 0
    outer loop
      vertex 0.0 0.0 0.0
      vertex 0.0 0.0 1.0
      vertex 1.0 0.0 0.0
    endloop
  endfacet
  facet normal 0 0 0
    outer loop
      vertex 1.0 0.0 0.0
      vertex 0.0 0.0 1.0
      vertex 0.0 1.0 0.0
    endloop
  endfacet
endsolid tetrahedron
"#;

    let mesh = ShapeImporter::import_stl_ascii(stl_snippet).unwrap();
    assert_eq!(mesh.triangles.len(), 4);
    assert_eq!(mesh.vertices.len(), 12);
}

#[test]
fn test_shape_importer_svg_profile_and_extrusion() {
    use algebra_engine::cad::{Mesh3D, ShapeImporter};

    // 2D SVG rectangle profile: 10 x 5
    let svg_path = "M 0 0 L 10 0 L 10 5 L 0 5 Z";
    let profile = ShapeImporter::import_svg_path(svg_path).unwrap();
    assert_eq!(profile.len(), 4);

    // Extrude by height 8.0 -> Prism of dimensions 10 x 5 x 8 (Volume = 400.0)
    let extruded_mesh = Mesh3D::extrude_profile(&profile, 8.0);
    assert_eq!(extruded_mesh.vertices.len(), 8);
    let vol = extruded_mesh.compute_volume();
    assert!((vol - 400.0).abs() < 1e-10);
}
