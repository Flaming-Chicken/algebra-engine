//! The geometric constraint solver satisfies every constraint, not just distance and coincidence.

use algebra_engine::{GeometricConstraint as C, GeometricConstraintSolver, GeometricEntity};

fn fixed(name: &str, x: f64, y: f64) -> (String, GeometricEntity) {
    (
        name.to_string(),
        GeometricEntity::Point2D {
            name: name.to_string(),
            x: Some(x),
            y: Some(y),
        },
    )
}

fn free(name: &str) -> (String, GeometricEntity) {
    (
        name.to_string(),
        GeometricEntity::Point2D {
            name: name.to_string(),
            x: None,
            y: None,
        },
    )
}

fn line(name: &str, a: &str, b: &str) -> (String, GeometricEntity) {
    (
        name.to_string(),
        GeometricEntity::Line2D {
            name: name.to_string(),
            p1: a.to_string(),
            p2: b.to_string(),
        },
    )
}

fn solver(
    entities: Vec<(String, GeometricEntity)>,
    constraints: Vec<C>,
) -> GeometricConstraintSolver {
    let mut s = GeometricConstraintSolver::new();
    for (name, entity) in entities {
        s.add_entity(name, entity);
    }
    for c in constraints {
        s.add_constraint(c);
    }
    s
}

fn dist(a: &[f64], b: &[f64]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

#[test]
fn a_345_triangle_is_found_from_two_distances() {
    // P1 at the origin, P2 five units away on a circle, P3 fixed: |P1P2| = 5 and |P2P3| = 3 with
    // P3 = (4, 0)... the chosen unknown P2 must satisfy both distances (a 3-4-5 right triangle).
    let s = solver(
        vec![fixed("A", 0.0, 0.0), fixed("B", 4.0, 0.0), free("C")],
        vec![
            C::Distance("A".into(), "C".into(), 5.0),
            C::Distance("B".into(), "C".into(), 3.0),
        ],
    );
    let solution = s.solve().unwrap();
    let c = solution.get("C").unwrap();
    assert!((dist(&[0.0, 0.0], c) - 5.0).abs() < 1e-9, "{c:?}");
    assert!((dist(&[4.0, 0.0], c) - 3.0).abs() < 1e-9, "{c:?}");
}

#[test]
fn a_square_from_perpendicular_sides_and_equal_lengths() {
    // A = (0,0), B = (2,0); C and D free: BC perpendicular to AB, AD perpendicular to AB,
    // |BC| = 2, |AD| = 2, and D coincident-distance to C of 2.
    let s = solver(
        vec![
            fixed("A", 0.0, 0.0),
            fixed("B", 2.0, 0.0),
            free("C"),
            free("D"),
            line("AB", "A", "B"),
            line("BC", "B", "C"),
            line("AD", "A", "D"),
        ],
        vec![
            C::Perpendicular("AB".into(), "BC".into()),
            C::Perpendicular("AB".into(), "AD".into()),
            C::Distance("B".into(), "C".into(), 2.0),
            C::Distance("A".into(), "D".into(), 2.0),
            C::Distance("C".into(), "D".into(), 2.0),
        ],
    );
    let solution = s.solve().unwrap();
    let (c, d) = (solution.get("C").unwrap(), solution.get("D").unwrap());
    // C = (2, +-2), D = (0, same sign): the two choices are mirror images.
    assert!(
        (c[0] - 2.0).abs() < 1e-7 && (d[0]).abs() < 1e-7,
        "C {c:?} D {d:?}"
    );
    assert!(
        (c[1].abs() - 2.0).abs() < 1e-7 && (c[1] - d[1]).abs() < 1e-7,
        "C {c:?} D {d:?}"
    );
}

#[test]
fn parallel_angle_and_tangent_constraints() {
    // Parallel: L2 through (0,1) and a free point, parallel to the x-axis line L1.
    let parallel = solver(
        vec![
            fixed("A", 0.0, 0.0),
            fixed("B", 3.0, 0.0),
            fixed("C", 0.0, 1.0),
            free("D"),
            line("L1", "A", "B"),
            line("L2", "C", "D"),
        ],
        vec![
            C::Parallel("L1".into(), "L2".into()),
            C::Distance("C".into(), "D".into(), 2.0),
        ],
    )
    .solve()
    .unwrap();
    let d = parallel.get("D").unwrap();
    assert!(
        (d[1] - 1.0).abs() < 1e-7 && (d[0].abs() - 2.0).abs() < 1e-7,
        "{d:?}"
    );

    // Angle: the line from the origin to a free point makes 30 degrees with the x-axis.
    let angle = solver(
        vec![
            fixed("O", 0.0, 0.0),
            fixed("X", 1.0, 0.0),
            free("P"),
            line("axis", "O", "X"),
            line("ray", "O", "P"),
        ],
        vec![
            C::Angle("axis".into(), "ray".into(), std::f64::consts::FRAC_PI_6),
            C::Distance("O".into(), "P".into(), 2.0),
        ],
    )
    .solve()
    .unwrap();
    let p = angle.get("P").unwrap();
    assert!(
        (p[0] - 3.0_f64.sqrt()).abs() < 1e-7 && (p[1] - 1.0).abs() < 1e-7,
        "{p:?}"
    );

    // Tangent: a line through (0,0) and a free point P at distance 5 from the origin is tangent
    // to the unit circle centred at (3, 0)... the distance from the centre to the line is 1.
    let tangent = solver(
        vec![
            fixed("O", 0.0, 0.0),
            fixed("Ctr", 3.0, 0.0),
            free("P"),
            line("L", "O", "P"),
            (
                "circle".to_string(),
                GeometricEntity::Circle2D {
                    name: "circle".to_string(),
                    center: "Ctr".to_string(),
                    radius: Some(1.0),
                },
            ),
        ],
        vec![
            C::Tangent("L".into(), "circle".into()),
            C::Distance("O".into(), "P".into(), 5.0),
        ],
    )
    .solve()
    .unwrap();
    let p = tangent.get("P").unwrap();
    // Distance from (3,0) to the line through the origin and P.
    let d = (p[0] * 0.0 - p[1] * 3.0).abs() / p[0].hypot(p[1]);
    assert!((d - 1.0).abs() < 1e-7, "{p:?} gives {d}");
}

#[test]
fn a_fixed_polygon_area() {
    // A = (0,0), B = (4,0), C = (4, h), D = (0, h) with C.x and D.x fixed by distances so that
    // the rectangle's area is 12: h = 3.
    let s = solver(
        vec![
            fixed("A", 0.0, 0.0),
            fixed("B", 4.0, 0.0),
            fixed("D", 0.0, 3.0),
            free("C"),
        ],
        vec![
            C::Distance("D".into(), "C".into(), 4.0),
            C::Distance("B".into(), "C".into(), 3.0),
            C::FixedArea(vec!["A".into(), "B".into(), "C".into(), "D".into()], 12.0),
        ],
    );
    let c = s.solve().unwrap();
    let c = c.get("C").unwrap();
    assert!(
        (c[0] - 4.0).abs() < 1e-7 && (c[1] - 3.0).abs() < 1e-7,
        "{c:?}"
    );
}

#[test]
fn inconsistent_constraints_are_reported() {
    // Two points 10 apart cannot also be 3 apart.
    let s = solver(
        vec![fixed("A", 0.0, 0.0), fixed("B", 10.0, 0.0)],
        vec![C::Distance("A".into(), "B".into(), 3.0)],
    );
    let error = s.solve().unwrap_err();
    assert!(error.to_string().contains("inconsistent"), "{error}");

    // Distances that cannot all hold: C is 1 from A and 1 from B while A and B are 10 apart.
    let s = solver(
        vec![fixed("A", 0.0, 0.0), fixed("B", 10.0, 0.0), free("C")],
        vec![
            C::Distance("A".into(), "C".into(), 1.0),
            C::Distance("B".into(), "C".into(), 1.0),
        ],
    );
    assert!(s.solve().is_err());
}

#[test]
fn constraints_on_unknown_or_wrong_kind_entities_are_errors() {
    let s = solver(
        vec![fixed("A", 0.0, 0.0), free("B")],
        vec![C::Parallel("A".into(), "B".into())],
    );
    assert!(s.solve().is_err());
    let s = solver(
        vec![fixed("A", 0.0, 0.0)],
        vec![C::Distance("A".into(), "missing".into(), 1.0)],
    );
    assert!(s.solve().is_err());
}

fn point3(name: &str, c: [Option<f64>; 3]) -> (String, GeometricEntity) {
    (
        name.to_string(),
        GeometricEntity::Point3D {
            name: name.to_string(),
            coords: c,
        },
    )
}

fn dist3(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(u, v)| (u - v).powi(2))
        .sum::<f64>()
        .sqrt()
}

#[test]
fn three_dimensional_points_are_solved_from_distances() {
    // Apex D at distance 1.2 from three base points of a right triangle: D = (0.5, 0.5, +-h) with
    // h^2 = 1.44 - 0.5.
    let mut solver = GeometricConstraintSolver::new();
    for (n, e) in [
        point3("A", [Some(0.0), Some(0.0), Some(0.0)]),
        point3("B", [Some(1.0), Some(0.0), Some(0.0)]),
        point3("C", [Some(0.0), Some(1.0), Some(0.0)]),
        point3("D", [None, None, None]),
    ] {
        solver.add_entity(n, e);
    }
    for base in ["A", "B", "C"] {
        solver.add_constraint(C::Distance(base.into(), "D".into(), 1.2));
    }
    let solved = solver.solve().unwrap();
    let d = &solved["D"];
    assert_eq!(d.len(), 3);
    for base in ["A", "B", "C"] {
        assert!(
            (dist3(&solved[base], d) - 1.2).abs() < 1e-8,
            "distance to {base}"
        );
    }
    let h = (1.44_f64 - 0.5).sqrt();
    assert!(
        (d[0] - 0.5).abs() < 1e-6 && (d[1] - 0.5).abs() < 1e-6,
        "{d:?}"
    );
    assert!((d[2].abs() - h).abs() < 1e-6, "{d:?}");

    // A chain with a 3-4-5 style spatial step: P at distance 3 from the origin, Q at distance 4
    // from P and 5 from the origin, with P pinned to the x axis through two known coordinates.
    let mut solver = GeometricConstraintSolver::new();
    for (n, e) in [
        point3("O", [Some(0.0), Some(0.0), Some(0.0)]),
        point3("P", [None, Some(0.0), Some(0.0)]),
        point3("Q", [None, None, None]),
    ] {
        solver.add_entity(n, e);
    }
    solver.add_constraint(C::Distance("O".into(), "P".into(), 3.0));
    solver.add_constraint(C::Distance("P".into(), "Q".into(), 4.0));
    solver.add_constraint(C::Distance("O".into(), "Q".into(), 5.0));
    let solved = solver.solve().unwrap();
    assert!((dist3(&solved["O"], &solved["P"]) - 3.0).abs() < 1e-8);
    assert!((dist3(&solved["P"], &solved["Q"]) - 4.0).abs() < 1e-8);
    assert!((dist3(&solved["O"], &solved["Q"]) - 5.0).abs() < 1e-8);
    assert!((solved["P"][1]).abs() < 1e-12 && (solved["P"][2]).abs() < 1e-12);

    // Coincidence in 3-D.
    let mut solver = GeometricConstraintSolver::new();
    for (n, e) in [
        point3("A", [Some(1.0), Some(-2.0), Some(3.0)]),
        point3("B", [None, None, None]),
    ] {
        solver.add_entity(n, e);
    }
    solver.add_constraint(C::Coincident("A".into(), "B".into()));
    let solved = solver.solve().unwrap();
    assert!(dist3(&solved["A"], &solved["B"]) < 1e-9);
}

#[test]
fn three_dimensional_constraints_report_what_they_cannot_do() {
    // Inconsistent distances in 3-D (triangle inequality violated) are an error.
    let mut solver = GeometricConstraintSolver::new();
    for (n, e) in [
        point3("A", [Some(0.0); 3]),
        point3("B", [Some(1.0), Some(0.0), Some(0.0)]),
        point3("D", [None, None, None]),
    ] {
        solver.add_entity(n, e);
    }
    solver.add_constraint(C::Distance("A".into(), "D".into(), 0.2));
    solver.add_constraint(C::Distance("B".into(), "D".into(), 0.2));
    assert!(solver.solve().is_err());

    // A 2-D point cannot be related to a 3-D point.
    let mut solver = GeometricConstraintSolver::new();
    let (n, e) = point3("A", [Some(0.0); 3]);
    solver.add_entity(n, e);
    let (n, e) = fixed("B", 1.0, 1.0);
    solver.add_entity(n, e);
    solver.add_constraint(C::Distance("A".into(), "B".into(), 1.0));
    assert!(solver.solve().is_err());
}
