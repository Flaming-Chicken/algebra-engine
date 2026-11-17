use algebra_advanced::knot::{BraidGenerator, BraidWord, KnotInvariants, LaurentPoly};

#[test]
fn test_braid_reduction_and_relations() {
    // sigma_1 * sigma_1^-1 -> empty
    let b1 = BraidWord::new(
        2,
        vec![
            BraidGenerator { index: 1, sign: 1 },
            BraidGenerator { index: 1, sign: -1 },
        ],
    );
    let reduced1 = b1.reduce();
    assert!(reduced1.word.is_empty());

    // Yang-Baxter braid relation: sigma_1 sigma_2 sigma_1 -> sigma_2 sigma_1 sigma_2
    let b2 = BraidWord::new(
        3,
        vec![
            BraidGenerator { index: 1, sign: 1 },
            BraidGenerator { index: 2, sign: 1 },
            BraidGenerator { index: 1, sign: 1 },
        ],
    );
    let reduced2 = b2.reduce();
    assert_eq!(reduced2.word[0].index, 2);
    assert_eq!(reduced2.word[1].index, 1);
    assert_eq!(reduced2.word[2].index, 2);
}

#[test]
fn test_trefoil_kauffman_bracket_and_jones() {
    let trefoil = BraidWord::trefoil();
    assert_eq!(trefoil.writhe(), 3);

    // Kauffman bracket of the closure of sigma_1^3 (positive crossings, right-handed trefoil):
    // <3_1> = A^-7 - A^-3 - A^5, the mirror of the left-handed A^7 - A^3 - A^-5.
    let bracket = KnotInvariants::kauffman_bracket(&trefoil).unwrap();
    assert_eq!(terms(&bracket), vec![(-7, 1), (-3, -1), (5, -1)]);

    // Normalized Jones polynomial: non-zero invariant terms
    // V(t) = t + t^3 - t^4 for the right-handed trefoil.
    let jones = KnotInvariants::jones_polynomial(&trefoil).unwrap();
    assert_eq!(terms(&jones), vec![(1, 1), (3, 1), (4, -1)]);
}

#[test]
fn test_figure_eight_bracket() {
    let fig8 = BraidWord::figure_eight();
    assert_eq!(fig8.writhe(), 0);

    let bracket = KnotInvariants::kauffman_bracket(&fig8).unwrap();
    // <4_1> has symmetric terms A^8, -A^4, 1, -A^-4, A^-8
    assert_eq!(*bracket.terms.get(&8).unwrap_or(&0.0), 1.0);
    assert_eq!(*bracket.terms.get(&0).unwrap_or(&0.0), 1.0);
    assert_eq!(*bracket.terms.get(&-8).unwrap_or(&0.0), 1.0);
}

#[test]
fn test_braid_simplicial_complex_and_hyperbolic_volume() {
    let trefoil = BraidWord::trefoil();
    let simplices = trefoil.to_simplicial_complex();
    assert!(!simplices.is_empty());
    // 0-simplices exist
    assert!(simplices.iter().any(|s| s.dimension == 0));
    // 1-simplices exist
    assert!(simplices.iter().any(|s| s.dimension == 1));
    // 2-simplices exist
    assert!(simplices.iter().any(|s| s.dimension == 2));

    // Trefoil is a torus knot -> hyperbolic volume = 0
    assert_eq!(trefoil.hyperbolic_volume(), 0.0);

    // Figure-eight knot is hyperbolic -> volume ~ 2.02988
    let fig8 = BraidWord::figure_eight();
    let vol = fig8.hyperbolic_volume();
    assert!((vol - 2.0298832).abs() < 1e-5);
}

fn braid(n: usize, word: &[(usize, i8)]) -> BraidWord {
    BraidWord::new(
        n,
        word.iter()
            .map(|&(index, sign)| BraidGenerator { index, sign })
            .collect(),
    )
}

fn terms(p: &LaurentPoly) -> Vec<(i64, i64)> {
    let mut v: Vec<(i64, i64)> = p
        .terms
        .iter()
        .map(|(&k, &c)| (k, c.round() as i64))
        .collect();
    v.sort();
    v
}

#[test]
fn bracket_is_a_state_sum_not_a_table_lookup() {
    // Hopf link: -A^4 - A^-4 (computed, not looked up).
    let hopf = KnotInvariants::kauffman_bracket(&braid(2, &[(1, 1), (1, 1)])).unwrap();
    assert_eq!(terms(&hopf), vec![(-4, -1), (4, -1)]);

    // Reidemeister I: a single kink is an unknot times -A^{+-3}.
    let kink = KnotInvariants::kauffman_bracket(&braid(2, &[(1, 1)])).unwrap();
    assert_eq!(kink.terms.len(), 1);
    assert_eq!(kink.terms.keys().next().map(|k| k.abs()), Some(3));

    // Mirror image: flipping every sign sends A -> A^-1.
    let trefoil = KnotInvariants::kauffman_bracket(&braid(2, &[(1, 1), (1, 1), (1, 1)])).unwrap();
    let mirror = KnotInvariants::kauffman_bracket(&braid(2, &[(1, -1), (1, -1), (1, -1)])).unwrap();
    let flipped: Vec<(i64, i64)> = {
        let mut v: Vec<_> = terms(&trefoil).into_iter().map(|(k, c)| (-k, c)).collect();
        v.sort();
        v
    };
    assert_eq!(terms(&mirror), flipped);

    // A word the old table did not cover: the (2,5) torus knot 5_1, whose Jones polynomial
    // -t^-7 + t^-6 - t^-5 + t^-4 + t^-2 (or its mirror) has five terms.
    let five = braid(2, &[(1, 1); 5]);
    let jones = KnotInvariants::jones_polynomial(&five).unwrap();
    assert_eq!(jones.terms.len(), 5, "{jones}");
}

#[test]
fn jones_polynomial_is_invariant_under_braid_relations_and_markov_moves() {
    let jones = |b: &BraidWord| terms(&KnotInvariants::jones_polynomial(b).unwrap());
    // Braid relation: s1 s2 s1 = s2 s1 s2.
    assert_eq!(
        jones(&braid(3, &[(1, 1), (2, 1), (1, 1), (1, 1), (1, 1)])),
        jones(&braid(3, &[(2, 1), (1, 1), (2, 1), (1, 1), (1, 1)]))
    );
    // Commuting generators.
    assert_eq!(
        jones(&braid(4, &[(1, 1), (3, -1), (1, 1), (1, 1)])),
        jones(&braid(4, &[(3, -1), (1, 1), (1, 1), (1, 1)]))
    );
    // Markov stabilisation: the trefoil on 2 strands equals sigma_1^3 sigma_2 on 3 strands.
    assert_eq!(
        jones(&braid(2, &[(1, 1), (1, 1), (1, 1)])),
        jones(&braid(3, &[(1, 1), (1, 1), (1, 1), (2, 1)]))
    );
    // The unknot has Jones polynomial 1 whatever its diagram.
    assert_eq!(jones(&braid(3, &[(1, 1), (2, 1)])), vec![(0, 1)]);
    // The trefoil is not the unknot and its two chiralities differ.
    let right = jones(&braid(2, &[(1, 1); 3]));
    let left = jones(&braid(2, &[(1, -1); 3]));
    assert_ne!(right, vec![(0, 1)]);
    assert_ne!(right, left);
}

#[test]
fn bracket_rejects_malformed_or_oversized_braids() {
    assert!(KnotInvariants::kauffman_bracket(&braid(2, &[(2, 1)])).is_err());
    assert!(KnotInvariants::kauffman_bracket(&braid(2, &[(0, 1)])).is_err());
    assert!(KnotInvariants::kauffman_bracket(&braid(2, &[(1, 1); 30])).is_err());
    assert!(KnotInvariants::kauffman_bracket(&braid(0, &[])).is_err());
}
