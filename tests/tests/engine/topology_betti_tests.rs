//! Exact Betti numbers of simplicial complexes (ledger `A14`): the old code approximated `beta_0` from face
//! counts and returned zeros above it, so a circle gave `[1, 0]`.

use algebra_engine::topology::SimplicialComplex;

fn complex(simplices: &[&[usize]]) -> SimplicialComplex {
    let mut c = SimplicialComplex::new();
    for s in simplices {
        c.add_simplex(s);
    }
    c
}

/// chi = sum (-1)^k beta_k must hold for every complex (a check that does not share the rank code).
fn assert_euler_identity(c: &SimplicialComplex) {
    let alternating: i64 = c
        .betti_numbers()
        .iter()
        .enumerate()
        .map(|(k, &b)| if k % 2 == 0 { b as i64 } else { -(b as i64) })
        .sum();
    assert_eq!(alternating, c.euler_characteristic());
}

#[test]
fn betti_numbers_of_standard_spaces() {
    // A circle (the hollow triangle).
    let circle = complex(&[&[0, 1], &[1, 2], &[0, 2]]);
    assert_eq!(circle.betti_numbers(), vec![1, 1]);
    // The filled triangle is contractible.
    assert_eq!(complex(&[&[0, 1, 2]]).betti_numbers(), vec![1, 0, 0]);
    // Two points, and a point plus a circle.
    assert_eq!(complex(&[&[0], &[1]]).betti_numbers(), vec![2]);
    assert_eq!(
        complex(&[&[0, 1], &[1, 2], &[0, 2], &[7]]).betti_numbers(),
        vec![2, 1]
    );
    // The boundary of a tetrahedron is a 2-sphere.
    let sphere = complex(&[&[0, 1, 2], &[0, 1, 3], &[0, 2, 3], &[1, 2, 3]]);
    assert_eq!(sphere.betti_numbers(), vec![1, 0, 1]);
    // The solid tetrahedron is contractible.
    assert_eq!(complex(&[&[0, 1, 2, 3]]).betti_numbers(), vec![1, 0, 0, 0]);
    // A wedge of two circles (two triangles sharing a vertex).
    let wedge = complex(&[&[0, 1], &[1, 2], &[0, 2], &[0, 3], &[3, 4], &[0, 4]]);
    assert_eq!(wedge.betti_numbers(), vec![1, 2]);
    // The 7-vertex torus: beta = (1, 2, 1), chi = 0.
    let torus_triangles: Vec<[usize; 3]> = (0..7)
        .flat_map(|i| [[i, (i + 1) % 7, (i + 3) % 7], [i, (i + 2) % 7, (i + 3) % 7]])
        .collect();
    let mut torus = SimplicialComplex::new();
    for t in &torus_triangles {
        torus.add_simplex(t);
    }
    assert_eq!(torus.count_simplices(0), 7);
    assert_eq!(torus.count_simplices(1), 21);
    assert_eq!(torus.count_simplices(2), 14);
    assert_eq!(torus.betti_numbers(), vec![1, 2, 1]);
    // The 6-vertex real projective plane: over the rationals beta = (1, 0, 0) (its H_1 = Z/2 is torsion).
    let rp2: [[usize; 3]; 10] = [
        [1, 2, 3],
        [1, 3, 4],
        [1, 4, 5],
        [1, 5, 6],
        [1, 6, 2],
        [2, 3, 5],
        [3, 4, 6],
        [4, 5, 2],
        [5, 6, 3],
        [6, 2, 4],
    ];
    let mut plane = SimplicialComplex::new();
    for t in &rp2 {
        plane.add_simplex(t);
    }
    assert_eq!(plane.euler_characteristic(), 1);
    assert_eq!(plane.betti_numbers(), vec![1, 0, 0]);
    for c in [&circle, &sphere, &wedge, &torus, &plane] {
        assert_euler_identity(c);
    }
}

#[test]
fn boundary_of_boundary_is_zero_and_empty_input_is_harmless() {
    let c = complex(&[&[0, 1, 2, 3], &[2, 3, 4]]);
    for k in 2..=3 {
        let (dk, dk1) = (c.boundary_matrix(k), c.boundary_matrix(k - 1));
        // d_{k-1} d_k = 0.
        for i in 0..dk1.len() {
            for j in 0..dk[0].len() {
                let entry: i64 = (0..dk.len()).map(|l| dk1[i][l] * dk[l][j]).sum();
                assert_eq!(entry, 0, "d{} d{} != 0 at ({i}, {j})", k - 1, k);
            }
        }
    }
    let mut empty = SimplicialComplex::new();
    empty.add_simplex(&[]);
    assert_eq!(empty.betti_numbers(), Vec::<usize>::new());
    assert_eq!(empty.euler_characteristic(), 0);
    // Duplicate vertices in the input are one vertex.
    assert_eq!(complex(&[&[3, 3, 4]]).betti_numbers(), vec![1, 0]);
}
