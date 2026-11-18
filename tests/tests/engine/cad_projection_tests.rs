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
    // Three variables are rejected instead of decomposed unsoundly.
    let vars3 = vec!["x".to_string(), "y".to_string(), "z".to_string()];
    let mut p = CadPolynomial::zero(vars3.clone());
    p.terms.insert(vec![1, 0, 0], 1.0);
    assert!(CadEngine::decompose(&[p], &vars3, 1e-10).is_err());
}
