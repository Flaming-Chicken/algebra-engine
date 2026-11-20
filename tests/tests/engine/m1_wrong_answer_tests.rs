//! Regression tests for the wrong answers the audit extension reproduced (ledger `F5`, `F7`, `F8`, `F10`).

use algebra_core::Number;
use algebra_engine::control::StateSpaceSystem;
use algebra_engine::iga::CoxDeBoor;
use algebra_engine::pde::numerical::NumericalPdeSolver;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

fn hash_of(n: &Number) -> u64 {
    let mut h = DefaultHasher::new();
    n.hash(&mut h);
    h.finish()
}

#[test]
fn poisson_solver_honours_nonzero_dirichlet_values() {
    // -u'' = 0 with u(0) = 1 and u(1) = 3 is the line 1 + 2x (it used to return 1, 0, 0, 0, 3).
    let (x, u) =
        NumericalPdeSolver::solve_poisson_fem_1d((0.0, 1.0), 4, |_| 0.0, 1.0, 3.0).unwrap();
    for (xi, ui) in x.iter().zip(&u) {
        assert!((ui - (1.0 + 2.0 * xi)).abs() < 1e-12, "u({xi}) = {ui}");
    }
    // -u'' = 2 with u(0) = 1 and u(1) = 3: u = 1 + 3x - x^2; the linear-element nodal values are exact.
    for n in [2, 5, 16] {
        let (x, u) =
            NumericalPdeSolver::solve_poisson_fem_1d((0.0, 1.0), n, |_| 2.0, 1.0, 3.0).unwrap();
        for (xi, ui) in x.iter().zip(&u) {
            assert!(
                (ui - (1.0 + 3.0 * xi - xi * xi)).abs() < 1e-12,
                "n = {n}: u({xi}) = {ui}"
            );
        }
    }
    // A shifted interval and unequal boundary values: u(2) = -1, u(5) = 8, no source: slope 3.
    let (x, u) =
        NumericalPdeSolver::solve_poisson_fem_1d((2.0, 5.0), 3, |_| 0.0, -1.0, 8.0).unwrap();
    for (xi, ui) in x.iter().zip(&u) {
        assert!(
            (ui - (-1.0 + 3.0 * (xi - 2.0))).abs() < 1e-12,
            "u({xi}) = {ui}"
        );
    }
    // Zero boundary values are unchanged: -u'' = 2 on [0, 1] with u = 0 at both ends is x (1 - x).
    let (x, u) =
        NumericalPdeSolver::solve_poisson_fem_1d((0.0, 1.0), 8, |_| 2.0, 0.0, 0.0).unwrap();
    for (xi, ui) in x.iter().zip(&u) {
        assert!((ui - xi * (1.0 - xi)).abs() < 1e-12);
    }
    // One element has no interior node: the boundary values are returned as they are.
    let (_, u) =
        NumericalPdeSolver::solve_poisson_fem_1d((0.0, 1.0), 1, |_| 5.0, 4.0, 6.0).unwrap();
    assert_eq!(u, vec![4.0, 6.0]);
}

#[test]
fn b_spline_basis_is_a_partition_of_unity_including_the_end_point() {
    let knot_vectors: Vec<(usize, Vec<f64>)> = vec![
        (2, vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0]),
        (3, vec![0.0, 0.0, 0.0, 0.0, 0.5, 1.0, 1.0, 1.0, 1.0]),
        (2, vec![0.0, 0.0, 0.0, 0.25, 0.5, 0.5, 1.0, 1.0, 1.0]),
        (1, vec![0.0, 0.0, 1.0, 2.0, 3.0, 3.0]),
        (2, vec![-1.0, -1.0, -1.0, 0.0, 2.0, 2.0, 2.0]),
    ];
    for (p, knots) in &knot_vectors {
        let n = knots.len() - p - 1;
        let (lo, hi) = (knots[*p], knots[n]);
        for k in 0..=20 {
            let u = lo + (hi - lo) * k as f64 / 20.0;
            let sum: f64 = (0..n)
                .map(|i| CoxDeBoor::basis_function(i, *p, u, knots))
                .sum();
            assert!(
                (sum - 1.0).abs() < 1e-12,
                "p = {p}, knots {knots:?}, u = {u}: sum {sum}"
            );
        }
        // At the end point the last basis function is 1 and the others vanish (clamped knot vectors).
        let at_end: Vec<f64> = (0..n)
            .map(|i| CoxDeBoor::basis_function(i, *p, hi, knots))
            .collect();
        assert!((at_end[n - 1] - 1.0).abs() < 1e-12, "{knots:?}: {at_end:?}");
        assert!(
            at_end[..n - 1].iter().all(|v| v.abs() < 1e-12),
            "{knots:?}: {at_end:?}"
        );
        let at_start: Vec<f64> = (0..n)
            .map(|i| CoxDeBoor::basis_function(i, *p, lo, knots))
            .collect();
        assert!((at_start[0] - 1.0).abs() < 1e-12, "{knots:?}: {at_start:?}");
    }
}

#[test]
fn matrix_rank_counts_pivots_not_rows_passed() {
    let rank = |m: &[Vec<f64>]| StateSpaceSystem::matrix_rank(m);
    assert_eq!(
        rank(&[vec![0.0, 1.0], vec![0.0, 0.0]]),
        1,
        "the case that gave 0"
    );
    assert_eq!(rank(&[vec![0.0, 0.0], vec![0.0, 0.0]]), 0);
    assert_eq!(
        rank(&[
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0]
        ]),
        3
    );
    assert_eq!(rank(&[vec![1.0, 2.0], vec![2.0, 4.0]]), 1);
    assert_eq!(
        rank(&[
            vec![0.0, 0.0, 1.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 0.0]
        ]),
        2
    );
    assert_eq!(
        rank(&[vec![0.0, 1.0, 2.0, 3.0], vec![0.0, 0.0, 0.0, 4.0]]),
        2,
        "a wide matrix with a zero column"
    );
    assert_eq!(rank(&[vec![0.0], vec![0.0], vec![5.0]]), 1, "a tall matrix");
    assert_eq!(
        rank(&[
            vec![1.0, 2.0, 3.0],
            vec![4.0, 5.0, 6.0],
            vec![7.0, 8.0, 9.0]
        ]),
        2
    );
    // The rank does not depend on the scale of the matrix.
    assert_eq!(rank(&[vec![1e-12, 0.0], vec![0.0, 1e-12]]), 2);
    assert_eq!(rank(&[vec![1e9, 2e9], vec![2e9, 4e9]]), 1);
    assert_eq!(rank(&[]), 0);
}

#[test]
fn matrix_exponential_does_not_cancel_for_large_negative_eigenvalues() {
    let exp = |m: &[Vec<f64>]| StateSpaceSystem::matrix_exponential(m);
    // e^{-50 I} = e^{-50} I = 1.93e-22 I (the unscaled series summed to garbage).
    let e = exp(&[vec![-50.0, 0.0], vec![0.0, -50.0]]);
    let want = (-50.0_f64).exp();
    assert!(
        (e[0][0] - want).abs() < 1e-12 * want && (e[1][1] - want).abs() < 1e-12 * want,
        "{e:?}"
    );
    // A rotation by 10 radians: [[cos 10, sin 10], [-sin 10, cos 10]].
    let r = exp(&[vec![0.0, 10.0], vec![-10.0, 0.0]]);
    let (c, s) = (10.0_f64.cos(), 10.0_f64.sin());
    for (got, want) in [(r[0][0], c), (r[0][1], s), (r[1][0], -s), (r[1][1], c)] {
        assert!((got - want).abs() < 1e-11, "{r:?}");
    }
    // A nilpotent matrix: e^{[[0, t], [0, 0]]} = [[1, t], [0, 1]].
    let n = exp(&[vec![0.0, 7.0], vec![0.0, 0.0]]);
    assert!(
        (n[0][1] - 7.0).abs() < 1e-13 && (n[0][0] - 1.0).abs() < 1e-13 && n[1][0].abs() < 1e-15
    );
    // The zero matrix gives the identity, and e^{A} e^{-A} = I for a mixed matrix.
    let z = exp(&[vec![0.0, 0.0], vec![0.0, 0.0]]);
    assert_eq!(z, vec![vec![1.0, 0.0], vec![0.0, 1.0]]);
    let a = vec![
        vec![3.0, -2.0, 1.0],
        vec![0.5, -4.0, 2.0],
        vec![1.0, 1.0, -1.0],
    ];
    let neg: Vec<Vec<f64>> = a.iter().map(|r| r.iter().map(|v| -v).collect()).collect();
    let (ea, en) = (exp(&a), exp(&neg));
    let product: Vec<Vec<f64>> = (0..3)
        .map(|i| {
            (0..3)
                .map(|j| (0..3).map(|k| ea[i][k] * en[k][j]).sum())
                .collect()
        })
        .collect();
    for (i, row) in product.iter().enumerate() {
        for (j, v) in row.iter().enumerate() {
            let want = if i == j { 1.0 } else { 0.0 };
            assert!((v - want).abs() < 1e-9, "{product:?}");
        }
    }
}

#[test]
fn equal_floats_are_one_number() {
    assert_eq!(Number::float(0.0), Number::float(-0.0));
    assert_eq!(hash_of(&Number::float(0.0)), hash_of(&Number::float(-0.0)));
    assert_eq!(Number::from_bits((-0.0_f64).to_bits()), Number::float(0.0));
    // Every NaN is one value: a NaN with a different payload hashes and compares like any other.
    let payload_nan = f64::from_bits(0x7ff8_0000_0000_0001);
    assert!(payload_nan.is_nan());
    assert_eq!(Number::float(payload_nan), Number::float(f64::NAN));
    assert_eq!(
        hash_of(&Number::float(payload_nan)),
        hash_of(&Number::float(f64::NAN))
    );
    // Ordinary values are untouched: 1.0 and -1.0 differ, and the bits round-trip.
    assert_ne!(Number::float(1.0), Number::float(-1.0));
    assert_eq!(Number::float(2.5), Number::from_bits(2.5_f64.to_bits()));
    assert_ne!(hash_of(&Number::float(1.0)), hash_of(&Number::float(-1.0)));
    assert_ne!(
        Number::float(f64::INFINITY),
        Number::float(f64::NEG_INFINITY)
    );
}

// --- B13: Smith normal form -------------------------------------------------------------------

use algebra_engine::systems::diophantine::DiophantineSystemSolver;

fn widen(m: &[Vec<i64>]) -> Vec<Vec<i128>> {
    m.iter()
        .map(|r| r.iter().map(|&v| v as i128).collect())
        .collect()
}

fn mat_mul(a: &[Vec<i128>], b: &[Vec<i128>]) -> Vec<Vec<i128>> {
    (0..a.len())
        .map(|i| {
            (0..b[0].len())
                .map(|j| (0..b.len()).map(|k| a[i][k] * b[k][j]).sum())
                .collect()
        })
        .collect()
}

/// Determinant by cofactor expansion (the matrices here are at most 4 x 4).
fn det(m: &[Vec<i64>]) -> i128 {
    if m.len() == 1 {
        return m[0][0] as i128;
    }
    (0..m.len())
        .map(|j| {
            let minor: Vec<Vec<i64>> = m[1..]
                .iter()
                .map(|r| {
                    r.iter()
                        .enumerate()
                        .filter(|(c, _)| *c != j)
                        .map(|(_, v)| *v)
                        .collect()
                })
                .collect();
            let sign = if j % 2 == 0 { 1 } else { -1 };
            sign * m[0][j] as i128 * det(&minor)
        })
        .sum()
}

fn check_snf(matrix: &[Vec<i64>]) {
    let snf = DiophantineSystemSolver::smith_normal_form(matrix).expect("a form");
    let (m, n) = (matrix.len(), matrix[0].len());
    // U A V = D with D the diagonal matrix of the invariant factors (and zero elsewhere).
    let uav = mat_mul(&mat_mul(&widen(&snf.u), &widen(matrix)), &widen(&snf.v));
    for i in 0..m {
        for j in 0..n {
            let want = if i == j && i < snf.diagonal.len() {
                snf.diagonal[i] as i128
            } else {
                0
            };
            assert_eq!(uav[i][j], want, "U A V != D for {matrix:?}: {uav:?}");
        }
    }
    // U and V are unimodular.
    assert_eq!(det(&snf.u).abs(), 1, "{matrix:?}");
    assert_eq!(det(&snf.v).abs(), 1, "{matrix:?}");
    // d_1 | d_2 | ... and the factors are positive.
    for w in snf.diagonal.windows(2) {
        assert!(
            w[0] > 0 && w[1] % w[0] == 0,
            "divisibility chain broken: {:?} for {matrix:?}",
            snf.diagonal
        );
    }
    assert!(snf.diagonal.iter().all(|&d| d > 0));
    assert_eq!(snf.rank, snf.diagonal.len());
}

#[test]
fn smith_normal_form_enforces_the_divisibility_chain() {
    let form = |m: &[Vec<i64>]| {
        DiophantineSystemSolver::smith_normal_form(m)
            .unwrap()
            .diagonal
    };
    // diag(2, 3) is equivalent to diag(1, 6); the old code returned [2, 3].
    assert_eq!(form(&[vec![2, 0], vec![0, 3]]), vec![1, 6]);
    assert_eq!(form(&[vec![4, 0], vec![0, 6]]), vec![2, 12]);
    assert_eq!(
        form(&[vec![2, 0, 0], vec![0, 3, 0], vec![0, 0, 5]]),
        vec![1, 1, 30]
    );
    // The textbook example: [[2, 4, 4], [-6, 6, 12], [10, -4, -16]] has invariant factors 2, 6, 12.
    assert_eq!(
        form(&[vec![2, 4, 4], vec![-6, 6, 12], vec![10, -4, -16]]),
        vec![2, 6, 12]
    );
    // Already in form, rank deficient, rectangular, zero.
    assert_eq!(form(&[vec![2, 0], vec![0, 4]]), vec![2, 4]);
    assert_eq!(form(&[vec![0, 1], vec![0, 0]]), vec![1]);
    assert_eq!(form(&[vec![1, 2, 3], vec![2, 4, 6]]), vec![1]);
    assert_eq!(form(&[vec![0, 0], vec![0, 0]]), Vec::<i64>::new());
    assert_eq!(form(&[vec![6], vec![10], vec![15]]), vec![1]);
    for m in [
        vec![vec![2, 0], vec![0, 3]],
        vec![vec![2, 4, 4], vec![-6, 6, 12], vec![10, -4, -16]],
        vec![vec![0, 1], vec![0, 0]],
        vec![vec![3, 1, 4], vec![1, 5, 9], vec![2, 6, 5], vec![3, 5, 8]],
    ] {
        check_snf(&m);
    }
}

#[test]
fn smith_normal_form_invariants_hold_for_pseudo_random_matrices() {
    let mut state: u64 = 0x2545_f491_4f6c_dd1d;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for round in 0..300 {
        let (m, n) = (1 + (next() % 4) as usize, 1 + (next() % 4) as usize);
        let range = if round % 3 == 0 { 5 } else { 40 };
        let matrix: Vec<Vec<i64>> = (0..m)
            .map(|_| {
                (0..n)
                    .map(|_| (next() % (2 * range + 1)) as i64 - range as i64)
                    .collect()
            })
            .collect();
        check_snf(&matrix);
        // The rank agrees with Gaussian elimination.
        let float: Vec<Vec<f64>> = matrix
            .iter()
            .map(|r| r.iter().map(|&v| v as f64).collect())
            .collect();
        let snf = DiophantineSystemSolver::smith_normal_form(&matrix).unwrap();
        assert_eq!(
            snf.rank,
            StateSpaceSystem::matrix_rank(&float),
            "{matrix:?}"
        );
    }
}

#[test]
fn smith_normal_form_reports_overflow_instead_of_wrapping() {
    let huge = i64::MAX;
    // The entries fit, the invariant factors (their gcd structure) fit, but the transforms may not;
    // either a correct form or an explicit error is acceptable, never a wrong one.
    match DiophantineSystemSolver::smith_normal_form(&[
        vec![huge, huge - 1],
        vec![huge - 2, huge - 3],
    ]) {
        Ok(snf) => {
            let original = vec![vec![huge, huge - 1], vec![huge - 2, huge - 3]];
            let uav = mat_mul(&mat_mul(&widen(&snf.u), &widen(&original)), &widen(&snf.v));
            for (i, d) in snf.diagonal.iter().enumerate() {
                assert_eq!(uav[i][i], *d as i128);
            }
        }
        Err(e) => assert!(e.to_string().contains("grew"), "{e}"),
    }
    assert!(DiophantineSystemSolver::smith_normal_form(&[]).is_err());
    assert!(DiophantineSystemSolver::smith_normal_form(&[vec![1, 2], vec![3]]).is_err());
}
