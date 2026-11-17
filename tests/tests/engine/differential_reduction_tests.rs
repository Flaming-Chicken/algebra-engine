//! Differential pseudo-division and Ritt's characteristic-set procedure.

use algebra_engine::{DiffIndeterminate, DiffPolynomial, DiffTerm, RittWuReducer};

fn y(order: usize) -> DiffIndeterminate {
    DiffIndeterminate::new(0, order)
}

/// `terms` are `(coefficient, [(derivative order of y, power)])`.
fn poly(terms: &[(f64, &[(usize, usize)])]) -> DiffPolynomial {
    DiffPolynomial::new(
        terms
            .iter()
            .map(|(c, fs)| DiffTerm::new(*c, fs.iter().map(|&(o, p)| (y(o), p)).collect()))
            .collect(),
    )
    .normalise()
}

#[test]
fn differentiation_merges_repeated_indeterminates() {
    // delta(y y') = y'^2 + y y''
    let product = poly(&[(1.0, &[(0, 1), (1, 1)])]);
    assert_eq!(
        product.differentiate(),
        poly(&[(1.0, &[(1, 2)]), (1.0, &[(0, 1), (2, 1)])])
    );
}

#[test]
fn linear_reduction_eliminates_higher_derivatives() {
    // The third derivative reduced by y'' - y is y' (y''' = (y'' - y)' + y').
    let f = poly(&[(1.0, &[(3, 1)])]);
    let g = poly(&[(1.0, &[(2, 1)]), (-1.0, &[(0, 1)])]);
    assert_eq!(RittWuReducer::reduce(&f, &g), poly(&[(1.0, &[(1, 1)])]));
    // A polynomial that is already reduced is unchanged.
    let h = poly(&[(1.0, &[(1, 1)]), (2.0, &[(0, 1)])]);
    assert_eq!(RittWuReducer::reduce(&h, &g), h);
}

#[test]
fn reduction_uses_derivatives_of_a_nonlinear_element() {
    // f = y'^2 - y has f' = 2 y' y'' - y'. The consequence y'' - 1/2 reduces to zero:
    // 2 y' (y'' - 1/2) - f' = 0.
    let f = poly(&[(1.0, &[(1, 2)]), (-1.0, &[(0, 1)])]);
    let consequence = poly(&[(1.0, &[(2, 1)]), (-0.5, &[])]);
    assert!(RittWuReducer::reduce(&consequence, &f).is_zero());
    // y'' - 1 is not a consequence: its remainder is non-zero.
    let other = poly(&[(1.0, &[(2, 1)]), (-1.0, &[])]);
    assert!(!RittWuReducer::reduce(&other, &f).is_zero());
}

#[test]
fn pseudo_division_by_a_leader_of_higher_degree() {
    // y'^3 reduced by y'^2 - y: y'^3 = y' (y'^2 - y) + y y', remainder y y'.
    let f = poly(&[(1.0, &[(1, 3)])]);
    let g = poly(&[(1.0, &[(1, 2)]), (-1.0, &[(0, 1)])]);
    assert_eq!(
        RittWuReducer::reduce(&f, &g),
        poly(&[(1.0, &[(0, 1), (1, 1)])])
    );
    assert!(RittWuReducer::is_reduced_wrt(
        &poly(&[(1.0, &[(1, 1)])]),
        &g
    ));
    assert!(!RittWuReducer::is_reduced_wrt(&f, &g));
    assert!(!RittWuReducer::is_reduced_wrt(
        &poly(&[(1.0, &[(2, 1)])]),
        &g
    ));
}

#[test]
fn characteristic_set_drops_consequences_and_detects_inconsistency() {
    let f = poly(&[(1.0, &[(1, 2)]), (-1.0, &[(0, 1)])]);
    let consequence = poly(&[(1.0, &[(2, 1)]), (-0.5, &[])]);
    let set = RittWuReducer::characteristic_set(vec![consequence, f.clone()]).unwrap();
    assert_eq!(set, vec![f]);

    // y = 0 forces y' = 0 and y'' = 0, so {y'', y, y'} has the characteristic set {y}.
    let linear = RittWuReducer::characteristic_set(vec![
        poly(&[(1.0, &[(2, 1)])]),
        poly(&[(1.0, &[(0, 1)])]),
        poly(&[(1.0, &[(1, 1)])]),
    ])
    .unwrap();
    assert_eq!(linear, vec![poly(&[(1.0, &[(0, 1)])])]);

    // {y', 1} has no solution: the non-zero constant is the characteristic set.
    let inconsistent =
        RittWuReducer::characteristic_set(vec![poly(&[(1.0, &[(1, 1)])]), poly(&[(1.0, &[])])])
            .unwrap();
    assert_eq!(inconsistent, vec![poly(&[(1.0, &[])])]);

    // Equations in two unknowns that do not interact stay as they are.
    let a = poly(&[(1.0, &[(1, 1)]), (-1.0, &[(0, 1)])]);
    let b = DiffPolynomial::new(vec![DiffTerm::new(
        1.0,
        vec![(DiffIndeterminate::new(1, 1), 1)],
    )]);
    let pair = RittWuReducer::characteristic_set(vec![b.clone(), a.clone()]).unwrap();
    assert_eq!(pair.len(), 2);
    assert!(pair.contains(&a) && pair.contains(&b));
}
