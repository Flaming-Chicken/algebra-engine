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

mod sparse {
    use algebra_core::ExprGraph;
    use algebra_engine::poly::{Polynomial, SparseMonomial, groebner_basis};
    use num_bigint::BigInt;
    use num_rational::BigRational;
    use std::cmp::Ordering;
    use std::collections::BTreeMap;

    type Term<'a> = (&'a [(&'a str, u32)], i64, i64);

    fn poly(graph: &ExprGraph, terms: &[Term]) -> Polynomial {
        let mut map = BTreeMap::new();
        for (powers, n, d) in terms {
            let mut mono = SparseMonomial::new();
            for (name, e) in *powers {
                mono.exps.insert(graph.symbols.get_or_intern(name), *e);
            }
            map.insert(mono, BigRational::new(BigInt::from(*n), BigInt::from(*d)));
        }
        Polynomial { terms: map }
    }

    fn rendered(graph: &ExprGraph, p: &Polynomial) -> Vec<(Vec<(String, u32)>, String)> {
        let mut out: Vec<_> = p
            .terms
            .iter()
            .map(|(m, c)| {
                let mut e: Vec<(String, u32)> = m
                    .exps
                    .iter()
                    .map(|(s, e)| (graph.symbols.resolve(*s).unwrap_or_default(), *e))
                    .collect();
                e.sort();
                (e, c.to_string())
            })
            .collect();
        out.sort();
        out
    }

    #[test]
    fn groebner_basis_returns_the_reduced_basis_not_its_input() {
        let g = ExprGraph::new();
        // I = <x^3 - 2xy, x^2 y - 2y^2 + x>: reduced basis { x^2, xy, y^2 - x/2 }.
        let f1 = poly(&g, &[(&[("x", 3)], 1, 1), (&[("x", 1), ("y", 1)], -2, 1)]);
        let f2 = poly(
            &g,
            &[
                (&[("x", 2), ("y", 1)], 1, 1),
                (&[("y", 2)], -2, 1),
                (&[("x", 1)], 1, 1),
            ],
        );
        let basis = groebner_basis(vec![f1.clone(), f2]);
        let shapes: Vec<_> = basis.iter().map(|p| rendered(&g, p)).collect();
        let x2 = vec![(vec![("x".to_string(), 2)], "1".to_string())];
        let xy = vec![(
            vec![("x".to_string(), 1), ("y".to_string(), 1)],
            "1".to_string(),
        )];
        let y2 = vec![
            (vec![("x".to_string(), 1)], "-1/2".to_string()),
            (vec![("y".to_string(), 2)], "1".to_string()),
        ];
        assert_eq!(basis.len(), 3, "{shapes:?}");
        for expected in [&x2, &xy, &y2] {
            assert!(
                shapes.iter().any(|s| s == expected),
                "missing {expected:?} in {shapes:?}"
            );
        }
        // The input was not already a basis, so returning it unchanged would have been wrong.
        assert_ne!(rendered(&g, &basis[0]), rendered(&g, &f1));
    }

    #[test]
    fn degenerate_ideals_and_idempotence() {
        let g = ExprGraph::new();
        assert!(groebner_basis(vec![]).is_empty());
        assert!(groebner_basis(vec![Polynomial::zero()]).is_empty());
        // <x, x + 1> is the unit ideal.
        let x = poly(&g, &[(&[("x", 1)], 1, 1)]);
        let x_plus_one = poly(&g, &[(&[("x", 1)], 1, 1), (&[], 1, 1)]);
        let unit = groebner_basis(vec![x, x_plus_one]);
        assert_eq!(unit.len(), 1);
        assert_eq!(rendered(&g, &unit[0]), vec![(vec![], "1".to_string())]);
        // A basis is a fixed point.
        let f = poly(&g, &[(&[("x", 2)], 1, 1), (&[("y", 1)], 1, 1)]);
        let h = poly(&g, &[(&[("x", 1), ("y", 1)], 1, 1), (&[("x", 1)], -1, 1)]);
        let once = groebner_basis(vec![f, h]);
        let twice = groebner_basis(once.clone());
        let a: Vec<_> = once.iter().map(|p| rendered(&g, p)).collect();
        let b: Vec<_> = twice.iter().map(|p| rendered(&g, p)).collect();
        assert_eq!(a, b);
    }

    #[test]
    fn the_grevlex_comparison_is_a_graded_reverse_lexicographic_order() {
        let g = ExprGraph::new();
        let mono = |powers: &[(&str, u32)]| {
            let mut m = SparseMonomial::new();
            for (n, e) in powers {
                m.exps.insert(g.symbols.get_or_intern(n), *e);
            }
            m
        };
        // Total degree first.
        assert_eq!(
            mono(&[("a", 1)]).cmp_grevlex(&mono(&[("a", 2)])),
            Ordering::Less
        );
        // Same degree: x^2 y > x y^2 (the last variable with the smaller exponent wins),
        // and x y z^... : with x > y > z, x y > z^2 > ... check xz vs y^2: y^2 has no z, so it wins.
        let (x, y, z) = ("x", "y", "z");
        let _ = (
            g.symbols.get_or_intern(x),
            g.symbols.get_or_intern(y),
            g.symbols.get_or_intern(z),
        );
        assert_eq!(
            mono(&[(x, 2), (y, 1)]).cmp_grevlex(&mono(&[(x, 1), (y, 2)])),
            Ordering::Greater
        );
        assert_eq!(
            mono(&[(y, 2)]).cmp_grevlex(&mono(&[(x, 1), (z, 1)])),
            Ordering::Greater
        );
        // Admissible: multiplying both sides by a monomial keeps the order.
        let (a, b) = (mono(&[(x, 1), (z, 1)]), mono(&[(y, 2)]));
        let m = mono(&[(x, 1), (y, 1), (z, 2)]);
        assert_eq!(a.cmp_grevlex(&b), a.mul(&m).cmp_grevlex(&b.mul(&m)));
        assert_eq!(a.cmp_grevlex(&a), Ordering::Equal);
    }
}
