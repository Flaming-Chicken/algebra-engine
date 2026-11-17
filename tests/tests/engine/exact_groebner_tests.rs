//! Exact Gröbner bases over Q, checked against textbook results (Cox, Little, O'Shea).

use algebra_engine::poly::exact::{ExactGrobner, ExactPoly};
use algebra_engine::poly::multivariate::MonomialOrder;

type T = (Vec<usize>, i64, i64);

fn poly(terms: &[T], vars: usize, order: MonomialOrder) -> ExactPoly {
    ExactPoly::from_terms(terms, vars, order)
}

/// Render as a sorted list of `(exponents, "p/q")` for exact comparison.
fn shape(p: &ExactPoly) -> Vec<(Vec<usize>, String)> {
    p.terms
        .iter()
        .map(|(e, c)| (e.clone(), c.to_string()))
        .collect()
}

#[test]
fn graded_lex_example_has_a_rational_coefficient() {
    // I = <x^3 - 2xy, x^2 y - 2y^2 + x> in grlex: GB = { x^2, xy, y^2 - x/2 } (CLO 2.7).
    let order = MonomialOrder::GradedLex;
    let f1 = poly(&[(vec![3, 0], 1, 1), (vec![1, 1], -2, 1)], 2, order);
    let f2 = poly(
        &[(vec![2, 1], 1, 1), (vec![0, 2], -2, 1), (vec![1, 0], 1, 1)],
        2,
        order,
    );
    let gb = ExactGrobner::compute(&[f1.clone(), f2.clone()]);
    assert!(gb.is_groebner_basis());
    let expected = [
        poly(&[(vec![2, 0], 1, 1)], 2, order),
        poly(&[(vec![1, 1], 1, 1)], 2, order),
        poly(&[(vec![0, 2], 1, 1), (vec![1, 0], -1, 2)], 2, order),
    ];
    assert_eq!(gb.basis.len(), 3, "{:?}", gb.basis);
    for e in &expected {
        assert!(
            gb.basis.iter().any(|b| shape(b) == shape(e)),
            "missing {:?} in {:?}",
            shape(e),
            gb.basis.iter().map(shape).collect::<Vec<_>>()
        );
    }
    assert!(gb.contains(&f1) && gb.contains(&f2));
    // y^2 - x/2 is in the ideal, x alone is not.
    assert!(gb.contains(&expected[2]));
    assert!(!gb.contains(&poly(&[(vec![1, 0], 1, 1)], 2, order)));
}

#[test]
fn lex_elimination_example_triangularises_the_system() {
    // I = <x^2 + y^2 + z^2 - 1, x^2 + z^2 - y, x - z>, lex x > y > z (CLO 2.7):
    // GB = { x - z, y - 2 z^2, z^4 + z^2/2 - 1/4 }.
    let order = MonomialOrder::Lex;
    let g1 = poly(
        &[
            (vec![2, 0, 0], 1, 1),
            (vec![0, 2, 0], 1, 1),
            (vec![0, 0, 2], 1, 1),
            (vec![0, 0, 0], -1, 1),
        ],
        3,
        order,
    );
    let g2 = poly(
        &[
            (vec![2, 0, 0], 1, 1),
            (vec![0, 0, 2], 1, 1),
            (vec![0, 1, 0], -1, 1),
        ],
        3,
        order,
    );
    let g3 = poly(&[(vec![1, 0, 0], 1, 1), (vec![0, 0, 1], -1, 1)], 3, order);
    let gb = ExactGrobner::compute(&[g1, g2, g3]);
    assert!(gb.is_groebner_basis());
    let expected = [
        poly(&[(vec![1, 0, 0], 1, 1), (vec![0, 0, 1], -1, 1)], 3, order),
        poly(&[(vec![0, 1, 0], 1, 1), (vec![0, 0, 2], -2, 1)], 3, order),
        poly(
            &[
                (vec![0, 0, 4], 1, 1),
                (vec![0, 0, 2], 1, 2),
                (vec![0, 0, 0], -1, 4),
            ],
            3,
            order,
        ),
    ];
    assert_eq!(gb.basis.len(), 3);
    for e in &expected {
        assert!(
            gb.basis.iter().any(|b| shape(b) == shape(e)),
            "missing {:?}",
            shape(e)
        );
    }
}

#[test]
fn exactness_matters_when_coefficients_are_not_dyadic() {
    // <x - y/3, y^2 - 1/3>: x = y/3 so x^2 = y^2/9 = 1/27. Floating point cannot represent 1/3
    // or 1/27, so this relation only holds exactly in Q.
    let order = MonomialOrder::Lex;
    let f1 = poly(&[(vec![1, 0], 1, 1), (vec![0, 1], -1, 3)], 2, order);
    let f2 = poly(&[(vec![0, 2], 1, 1), (vec![0, 0], -1, 3)], 2, order);
    let gb = ExactGrobner::compute(&[f1, f2]);
    assert!(gb.is_groebner_basis());
    let x_squared_minus = poly(&[(vec![2, 0], 1, 1), (vec![0, 0], -1, 27)], 2, order);
    assert!(gb.contains(&x_squared_minus));
    let off_by_a_hair = poly(&[(vec![2, 0], 1, 1), (vec![0, 0], -1, 28)], 2, order);
    assert!(!gb.contains(&off_by_a_hair));
}

#[test]
fn trivial_and_degenerate_ideals() {
    let order = MonomialOrder::GrevLex;
    // <x, x + 1> is the whole ring: the reduced basis is {1}.
    let x = poly(&[(vec![1], 1, 1)], 1, order);
    let x_plus_one = poly(&[(vec![1], 1, 1), (vec![0], 1, 1)], 1, order);
    let gb = ExactGrobner::compute(&[x, x_plus_one]);
    assert_eq!(gb.basis.len(), 1);
    assert_eq!(shape(&gb.basis[0]), vec![(vec![0], "1".to_string())]);
    assert!(gb.contains(&poly(&[(vec![5], 7, 3)], 1, order)));

    // The zero ideal has an empty basis and contains only zero.
    let empty = ExactGrobner::compute(&[ExactPoly::zero(1, order)]);
    assert!(empty.basis.is_empty());
    assert!(empty.contains(&ExactPoly::zero(1, order)));
    assert!(!empty.contains(&poly(&[(vec![1], 1, 1)], 1, order)));
}

#[test]
fn the_floating_point_polynomials_convert_exactly() {
    use algebra_engine::poly::multivariate::{Monomial, MultiPoly, Term};
    let order = MonomialOrder::Lex;
    let p = MultiPoly::from_terms(
        vec![
            Term::new(0.5, Monomial::new(vec![2, 0])),
            Term::new(-3.0, Monomial::new(vec![0, 1])),
        ],
        2,
        vec!["x".into(), "y".into()],
        order,
    );
    let exact = ExactPoly::from_multi_poly(&p).unwrap();
    assert_eq!(
        shape(&exact),
        vec![
            (vec![0, 1], "-3".to_string()),
            (vec![2, 0], "1/2".to_string())
        ]
    );
}
