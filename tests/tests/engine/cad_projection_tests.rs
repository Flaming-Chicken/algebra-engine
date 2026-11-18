//! Cylindrical algebraic decomposition with an exact projection phase (two variables).

use algebra_engine::cad::RelationalOp;
use algebra_engine::cad::projection::{BiPoly, coprime_basis, project, real_roots, resultant_y};
use algebra_engine::cad::{
    CadCellType, CadEngine, CadPolynomial, Quantifier, QuantifierEliminator,
};
use algebra_engine::gosper::QPoly;
use num_traits::ToPrimitive;

fn vars2() -> Vec<String> {
    vec!["x".to_string(), "y".to_string()]
}

/// A polynomial in (x, y) from `(x_exponent, y_exponent, coefficient)` terms.
fn poly(terms: &[(usize, usize, f64)]) -> CadPolynomial {
    let mut p = CadPolynomial::zero(vars2());
    for &(ex, ey, c) in terms {
        p.terms.insert(vec![ex, ey], c);
    }
    p
}

/// Number of leaf cells above each base cell.
fn fibre_counts(tree: &algebra_engine::cad::CadCell) -> Vec<usize> {
    tree.children.iter().map(|c| c.count_leaf_cells()).collect()
}

#[test]
fn the_unit_circle_has_thirteen_cells() {
    // x^2 + y^2 - 1: base roots -1 and 1 (tangent fibres there), fibres of 1, 3, 5, 3, 1 cells.
    let circle = poly(&[(2, 0, 1.0), (0, 2, 1.0), (0, 0, -1.0)]);
    let tree = CadEngine::decompose(&[circle], &vars2(), 1e-10).unwrap();
    assert_eq!(
        tree.children.len(),
        5,
        "x < -1, x = -1, -1 < x < 1, x = 1, x > 1"
    );
    assert_eq!(fibre_counts(&tree), vec![1, 3, 5, 3, 1]);
    assert_eq!(tree.count_leaf_cells(), 13);
    // The base cells alternate sector, section, sector, section, sector.
    let kinds: Vec<CadCellType> = tree.children.iter().map(|c| c.cell_type).collect();
    assert_eq!(
        kinds,
        vec![
            CadCellType::Sector,
            CadCellType::Section,
            CadCellType::Sector,
            CadCellType::Section,
            CadCellType::Sector
        ]
    );
}

#[test]
fn a_cubic_needs_the_discriminant_not_the_slice_through_zero() {
    // f = y^3 - 3y + x. Its discriminant in y is 108 - 27 x^2, with roots x = -2 and x = 2: that is
    // where the fibre changes from one real root to three (a double root appears). The old
    // lifting set the other variable to 0, found the single root x = 0 and missed both.
    let f = poly(&[(0, 3, 1.0), (0, 1, -3.0), (1, 0, 1.0)]);
    let tree = CadEngine::decompose(&[f], &vars2(), 1e-10).unwrap();
    assert_eq!(tree.children.len(), 5, "base roots are exactly -2 and 2");
    let xs: Vec<f64> = tree
        .children
        .iter()
        .map(|c| *c.sample_point.last().unwrap())
        .collect();
    assert!(
        (xs[1] + 2.0).abs() < 1e-9 && (xs[3] - 2.0).abs() < 1e-9,
        "{xs:?}"
    );
    // One real root, a double root plus a simple one, three roots, again, one root: 3 + 5 + 7 + 5 + 3.
    assert_eq!(fibre_counts(&tree), vec![3, 5, 7, 5, 3]);
    assert_eq!(tree.count_leaf_cells(), 23);
}

#[test]
fn quantifier_elimination_uses_the_correct_cells() {
    // f = y^3 - 3y + x, g = y - 1, h = x - 2 (indices 0, 1, 2).
    let f = poly(&[(0, 3, 1.0), (0, 1, -3.0), (1, 0, 1.0)]);
    let g = poly(&[(0, 1, 1.0), (0, 0, -1.0)]);
    let h = poly(&[(1, 0, 1.0), (0, 0, -2.0)]);
    let both = vec![
        (Quantifier::Exists, "x".to_string()),
        (Quantifier::Exists, "y".to_string()),
    ];
    // f = 0 with y > 1 forces x < 2 (y^3 - 3y > -2 for y > 1), so adding x > 2 is unsatisfiable...
    let unsat = vec![
        (0, RelationalOp::Equal),
        (1, RelationalOp::GreaterThan),
        (2, RelationalOp::GreaterThan),
    ];
    assert!(
        !QuantifierEliminator::eliminate(
            &both,
            &[f.clone(), g.clone(), h.clone()],
            &unsat,
            &vars2()
        )
        .unwrap()
    );
    // ...while x < 2 is satisfiable (x = 0, y = sqrt 3).
    let sat = vec![
        (0, RelationalOp::Equal),
        (1, RelationalOp::GreaterThan),
        (2, RelationalOp::LessThan),
    ];
    assert!(QuantifierEliminator::eliminate(&both, &[f, g, h], &sat, &vars2()).unwrap());

    // exists x, y on the circle with y > 1: unsatisfiable; with y > 0.5: satisfiable.
    let circle = poly(&[(2, 0, 1.0), (0, 2, 1.0), (0, 0, -1.0)]);
    let above_one = poly(&[(0, 1, 1.0), (0, 0, -1.0)]);
    let above_half = poly(&[(0, 1, 1.0), (0, 0, -0.5)]);
    let cond = vec![(0, RelationalOp::Equal), (1, RelationalOp::GreaterThan)];
    assert!(
        !QuantifierEliminator::eliminate(&both, &[circle.clone(), above_one], &cond, &vars2())
            .unwrap()
    );
    assert!(
        QuantifierEliminator::eliminate(&both, &[circle, above_half], &cond, &vars2()).unwrap()
    );
}

#[test]
fn resultants_discriminants_and_real_roots_are_exact() {
    let bi = |coeffs: &[&[i64]]| BiPoly {
        coeffs: coeffs.iter().map(|c| QPoly::from_ints(c)).collect(),
    };
    // Res_y(y^2 - x, y - 2) = f(2) = 4 - x.
    let f = bi(&[&[0, -1], &[0], &[1]]); // -x + 0 y + y^2
    let g = bi(&[&[-2], &[1]]);
    assert_eq!(resultant_y(&f, &g).unwrap(), QPoly::from_ints(&[4, -1]));
    // The projection of the circle contains x^2 - 1 (the discriminant, up to a constant factor).
    let circle = bi(&[&[-1, 0, 1], &[0], &[1]]);
    let projected = project(&[circle]).unwrap();
    let roots: Vec<f64> = coprime_basis(&projected)
        .iter()
        .flat_map(real_roots)
        .map(|r| r.to_f64().unwrap())
        .collect();
    let mut roots = roots;
    roots.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(roots.len(), 2);
    assert!(
        (roots[0] + 1.0).abs() < 1e-12 && (roots[1] - 1.0).abs() < 1e-12,
        "{roots:?}"
    );

    // Real roots: x^2 - 2, x^3 - 2 (one real root) and a rational root found exactly.
    let sqrt2 = real_roots(&QPoly::from_ints(&[-2, 0, 1]));
    assert_eq!(sqrt2.len(), 2);
    assert!((sqrt2[1].to_f64().unwrap() - 2.0_f64.sqrt()).abs() < 1e-15);
    let cbrt2 = real_roots(&QPoly::from_ints(&[-2, 0, 0, 1]));
    assert_eq!(cbrt2.len(), 1);
    assert!((cbrt2[0].to_f64().unwrap() - 2.0_f64.cbrt()).abs() < 1e-15);
    // A rational root comes back as the midpoint of a tiny isolating interval, not necessarily exactly.
    let rational = real_roots(&QPoly::from_ints(&[-6, 5, 1])); // (x + 6)(x - 1)
    assert_eq!(rational.len(), 2);
    assert!((rational[0].to_f64().unwrap() + 6.0).abs() < 1e-18);
    assert!((rational[1].to_f64().unwrap() - 1.0).abs() < 1e-18);
    // A repeated factor is reduced to its distinct roots.
    assert_eq!(real_roots(&QPoly::from_ints(&[1, -2, 1])).len(), 1);
    // No real roots.
    assert!(real_roots(&QPoly::from_ints(&[1, 0, 1])).is_empty());
}

#[test]
fn unsupported_and_degenerate_inputs_are_errors_not_wrong_answers() {
    // A polynomial with a repeated factor in y: (y - x)^2 = y^2 - 2 x y + x^2.
    let repeated = poly(&[(0, 2, 1.0), (1, 1, -2.0), (2, 0, 1.0)]);
    assert!(CadEngine::decompose(&[repeated], &vars2(), 1e-10).is_err());
    // Two polynomials sharing the factor y - x.
    let a = poly(&[(0, 1, 1.0), (1, 0, -1.0)]);
    let b = poly(&[(0, 2, 1.0), (1, 2, 0.0), (2, 0, -1.0)]); // y^2 - x^2 = (y - x)(y + x)
    assert!(CadEngine::decompose(&[a, b], &vars2(), 1e-10).is_err());
    // Four variables are rejected instead of decomposed unsoundly.
    let vars4: Vec<String> = ["x", "y", "z", "w"].iter().map(|v| v.to_string()).collect();
    let mut p = CadPolynomial::zero(vars4.clone());
    p.terms.insert(vec![1, 0, 0, 0], 1.0);
    assert!(CadEngine::decompose(&[p], &vars4, 1e-10).is_err());
    // A repeated factor in z: (z - x)^2.
    let mut repeated_z = CadPolynomial::zero(vars3());
    repeated_z.terms.insert(vec![0, 0, 2], 1.0);
    repeated_z.terms.insert(vec![1, 0, 1], -2.0);
    repeated_z.terms.insert(vec![2, 0, 0], 1.0);
    assert!(CadEngine::decompose(&[repeated_z], &vars3(), 1e-10).is_err());
}

fn vars3() -> Vec<String> {
    vec!["x".to_string(), "y".to_string(), "z".to_string()]
}

/// A polynomial in (x, y, z) from `(x, y, z exponents, coefficient)` terms.
fn poly3(terms: &[(usize, usize, usize, f64)]) -> CadPolynomial {
    let mut p = CadPolynomial::zero(vars3());
    for &(ex, ey, ez, c) in terms {
        p.terms.insert(vec![ex, ey, ez], c);
    }
    p
}

fn leaves(cell: &algebra_engine::cad::CadCell) -> Vec<&algebra_engine::cad::CadCell> {
    if cell.children.is_empty() {
        vec![cell]
    } else {
        cell.children.iter().flat_map(leaves).collect()
    }
}

#[test]
fn the_unit_sphere_has_twenty_five_cells() {
    use algebra_engine::cad::Sign;
    // x^2 + y^2 + z^2 - 1. Over x < -1 and x > 1 one cell; over x = +-1 the fibre is the single
    // point (y, z) = (0, 0), cut into 1, 3, 1 cells in y with one root in z only at y = 0 (5 cells
    // in all); over -1 < x < 1 the y line has cuts at +-sqrt(1 - x^2) with 1, 3, 5, 3, 1 cells in z.
    let sphere = poly3(&[
        (2, 0, 0, 1.0),
        (0, 2, 0, 1.0),
        (0, 0, 2, 1.0),
        (0, 0, 0, -1.0),
    ]);
    let tree = CadEngine::decompose(&[sphere], &vars3(), 1e-10).unwrap();
    assert_eq!(tree.children.len(), 5);
    assert_eq!(fibre_counts(&tree), vec![1, 5, 13, 5, 1]);
    assert_eq!(tree.count_leaf_cells(), 25);
    // One cell inside the ball, six on the sphere, the rest outside.
    let signs: Vec<Sign> = leaves(&tree).iter().map(|l| l.signs[0]).collect();
    let count = |s: Sign| signs.iter().filter(|&&x| x == s).count();
    assert_eq!(
        (
            count(Sign::Negative),
            count(Sign::Zero),
            count(Sign::Positive)
        ),
        (1, 6, 18)
    );
}

#[test]
fn a_sphere_cut_by_a_plane_is_decomposed_along_their_intersection_circle() {
    use algebra_engine::cad::Sign;
    // The sphere and the plane z = x meet in a circle: the resultants must cut the x line at
    // x = +-1/sqrt(2) (where the circle's extremes lie), which neither polynomial does alone.
    let sphere = poly3(&[
        (2, 0, 0, 1.0),
        (0, 2, 0, 1.0),
        (0, 0, 2, 1.0),
        (0, 0, 0, -1.0),
    ]);
    let plane = poly3(&[(0, 0, 1, 1.0), (1, 0, 0, -1.0)]);
    let tree = CadEngine::decompose(&[sphere, plane], &vars3(), 1e-10).unwrap();
    let both_zero = leaves(&tree)
        .iter()
        .filter(|l| l.signs[0] == Sign::Zero && l.signs[1] == Sign::Zero)
        .count();
    assert!(both_zero >= 3, "the circle appears as cells: {both_zero}");
    let cuts: Vec<f64> = tree
        .children
        .iter()
        .filter(|c| c.cell_type == CadCellType::Section)
        .map(|c| c.sample_point[0])
        .collect();
    let half = 0.5_f64.sqrt();
    for want in [-1.0, -half, half, 1.0] {
        assert!(
            cuts.iter().any(|c| (c - want).abs() < 1e-6),
            "x cut {want} missing from {cuts:?}"
        );
    }
}

#[test]
fn the_trivariate_resultant_is_exact() {
    use algebra_engine::cad::projection::{TriPoly, bi_eval, resultant_z};
    use num_bigint::BigInt;
    use num_rational::BigRational;
    // res_z(z^2 - x, z - y) = y^2 - x.
    let q = |n: i64| BigRational::from_integer(BigInt::from(n));
    let bi = |coeffs: Vec<Vec<i64>>| BiPoly {
        coeffs: coeffs.iter().map(|c| QPoly::from_ints(c)).collect(),
    };
    // coefficients indexed [z power] -> BiPoly [y power] -> QPoly in x
    let f = TriPoly {
        coeffs: vec![bi(vec![vec![0, -1]]), bi(vec![]), bi(vec![vec![1]])],
    };
    let g = TriPoly {
        coeffs: vec![bi(vec![vec![0], vec![-1]]), bi(vec![vec![1]])],
    };
    let r = resultant_z(&f, &g).unwrap();
    for (x, y) in [(3, 2), (-1, 5), (0, 0), (7, -4)] {
        assert_eq!(bi_eval(&r, &q(x), &q(y)), q(y * y - x), "at ({x}, {y})");
    }
}
