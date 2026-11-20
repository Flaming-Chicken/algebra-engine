//! Linear algebra against algebraic identities on random integer matrices. The oracles do not use the routine
//! under test: products and sums are evaluated numerically, and each identity relates two different routines or
//! checks a defining property (A * inverse(A) = I, A v = lambda v, det(AB) = det(A) det(B), ...).

use algebra_core::ExprGraph;
use algebra_engine::matrix::SymbolicMatrix;
use algebra_engine::numeric::{BigValue, EvalContext, NumericalEval};

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn entry(&mut self) -> i64 {
        (self.next() % 11) as i64 - 5
    }
}

fn literal(rows: &[Vec<i64>]) -> String {
    format!(
        "[{}]",
        rows.iter()
            .map(|r| format!(
                "[{}]",
                r.iter().map(i64::to_string).collect::<Vec<_>>().join(",")
            ))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn random(rng: &mut Lcg, n: usize) -> Vec<Vec<i64>> {
    (0..n)
        .map(|_| (0..n).map(|_| rng.entry()).collect())
        .collect()
}

fn complex(graph: &ExprGraph, id: algebra_core::ExprId) -> (f64, f64) {
    match graph.evalf(id, &EvalContext::default()).unwrap() {
        BigValue::Real(r) => (r, 0.0),
        BigValue::Complex(r, i) => (r, i),
    }
}

fn matrix_values(graph: &ExprGraph, m: &SymbolicMatrix, n: usize) -> Vec<Vec<(f64, f64)>> {
    (0..n)
        .map(|r| (0..n).map(|c| complex(graph, m.get(r, c))).collect())
        .collect()
}

fn close(a: (f64, f64), b: (f64, f64), scale: f64) -> bool {
    (a.0 - b.0).abs() <= 1e-7 * (1.0 + scale) && (a.1 - b.1).abs() <= 1e-7 * (1.0 + scale)
}

#[test]
fn determinant_inverse_trace_and_eigenvalues_satisfy_their_identities() {
    let mut rng = Lcg(2026);
    let (mut inverses, mut eigen_checks) = (0, 0);
    for trial in 0..30 {
        let n = 2 + (trial % 3); // 2, 3, 4
        let (a_rows, b_rows) = (random(&mut rng, n), random(&mut rng, n));
        let graph = ExprGraph::new();
        let a = SymbolicMatrix::from_expr_str(&graph, &literal(&a_rows)).unwrap();
        let b = SymbolicMatrix::from_expr_str(&graph, &literal(&b_rows)).unwrap();
        let ab = a.multiply(&graph, &b).unwrap();
        let (da, db, dab) = (
            complex(&graph, a.determinant(&graph).unwrap()),
            complex(&graph, b.determinant(&graph).unwrap()),
            complex(&graph, ab.determinant(&graph).unwrap()),
        );
        let scale = dab.0.abs().max((da.0 * db.0).abs());
        assert!(
            close(dab, (da.0 * db.0, 0.0), scale),
            "det(AB) != det(A) det(B) for {a_rows:?} {b_rows:?}"
        );

        if da.0.abs() > 0.5 {
            let inverse = a.inverse(&graph).unwrap();
            let product = a.multiply(&graph, &inverse).unwrap();
            let values = matrix_values(&graph, &product, n);
            for r in 0..n {
                for c in 0..n {
                    let want = if r == c { 1.0 } else { 0.0 };
                    assert!(
                        close(values[r][c], (want, 0.0), 1.0),
                        "A * inverse(A) != I at ({r},{c}) for {a_rows:?}: {:?}",
                        values[r][c]
                    );
                }
            }
            inverses += 1;
        }

        if let Ok(eigenvalues) = a.eigenvalues(&graph) {
            if eigenvalues.len() == n {
                let values: Vec<(f64, f64)> =
                    eigenvalues.iter().map(|&e| complex(&graph, e)).collect();
                let sum = values
                    .iter()
                    .fold((0.0, 0.0), |s, v| (s.0 + v.0, s.1 + v.1));
                let trace = complex(&graph, a.trace(&graph).unwrap());
                assert!(
                    close(sum, trace, trace.0.abs()),
                    "sum of eigenvalues != trace for {a_rows:?}: {sum:?} vs {trace:?}"
                );
                let product = values.iter().fold((1.0, 0.0), |p, v| {
                    (p.0 * v.0 - p.1 * v.1, p.0 * v.1 + p.1 * v.0)
                });
                assert!(
                    close(product, da, da.0.abs()),
                    "product of eigenvalues != det for {a_rows:?}: {product:?} vs {da:?}"
                );
                eigen_checks += 1;
            }
        }
    }
    assert!(
        inverses > 20,
        "only {inverses} invertible matrices exercised"
    );
    assert!(
        eigen_checks > 20,
        "only {eigen_checks} eigenvalue sets checked"
    );
}

#[test]
fn rank_nullspace_and_rref_are_consistent() {
    let mut rng = Lcg(77);
    for trial in 0..50 {
        let n = 3 + (trial % 2);
        // Make rank-deficient matrices often: the last row is a combination of the first two.
        let mut rows = random(&mut rng, n);
        if trial % 2 == 0 {
            rows[n - 1] = (0..n).map(|c| rows[0][c] + 2 * rows[1][c]).collect();
        }
        let graph = ExprGraph::new();
        let a = SymbolicMatrix::from_expr_str(&graph, &literal(&rows)).unwrap();
        let rank = a.rank(&graph).unwrap();
        let null = a.nullspace(&graph).unwrap();
        assert_eq!(rank + null.len(), n, "rank-nullity for {rows:?}");
        for v in &null {
            for r in 0..n {
                let mut sum = (0.0, 0.0);
                for c in 0..n {
                    let (x, y) = (rows[r][c] as f64, complex(&graph, v[c]));
                    sum = (sum.0 + x * y.0, sum.1 + x * y.1);
                }
                assert!(
                    close(sum, (0.0, 0.0), 1.0),
                    "A v != 0 for {rows:?}, v = row {r}: {sum:?}"
                );
            }
        }
        // A rank-deficient matrix has determinant zero.
        if rank < n {
            assert!(
                complex(&graph, a.determinant(&graph).unwrap()).0.abs() < 1e-9,
                "{rows:?}"
            );
        }
    }
}

/// A random 3x3 integer matrix with integer eigenvalues: `P D P^-1` for a unimodular `P` built from elementary
/// row operations (so the inverse is an integer matrix too).
fn with_integer_eigenvalues(rng: &mut Lcg) -> Vec<Vec<i64>> {
    let mut p = vec![vec![1, 0, 0], vec![0, 1, 0], vec![0, 0, 1]];
    let mut p_inv = p.clone();
    for _ in 0..4 {
        let (i, j) = ((rng.next() % 3) as usize, (rng.next() % 3) as usize);
        if i == j {
            continue;
        }
        let f = (rng.next() % 3) as i64 - 1;
        // row_i += f * row_j on P; the inverse gets col_j -= f * col_i.
        for c in 0..3 {
            let d = f * p[j][c];
            p[i][c] += d;
        }
        for r in 0..3 {
            let d = f * p_inv[r][i];
            p_inv[r][j] -= d;
        }
    }
    let d: Vec<i64> = (0..3).map(|_| (rng.next() % 7) as i64 - 3).collect();
    let mut a = vec![vec![0i64; 3]; 3];
    for r in 0..3 {
        for c in 0..3 {
            a[r][c] = (0..3).map(|k| p[r][k] * d[k] * p_inv[k][c]).sum();
        }
    }
    a
}

#[test]
fn eigenvectors_satisfy_a_v_equals_lambda_v() {
    let mut rng = Lcg(5);
    let mut checked = 0;
    for _ in 0..40 {
        let rows = with_integer_eigenvalues(&mut rng);
        let graph = ExprGraph::new();
        let a = SymbolicMatrix::from_expr_str(&graph, &literal(&rows)).unwrap();
        let Ok(pairs) = a.eigenvectors(&graph) else {
            continue;
        };
        for (lambda, v) in pairs {
            let l = complex(&graph, lambda);
            let vals: Vec<(f64, f64)> = v.iter().map(|&e| complex(&graph, e)).collect();
            if vals.len() != 3 {
                continue;
            }
            let norm: f64 = vals
                .iter()
                .map(|z| z.0 * z.0 + z.1 * z.1)
                .sum::<f64>()
                .sqrt();
            assert!(
                norm > 1e-9,
                "the zero vector is not an eigenvector ({rows:?})"
            );
            for r in 0..3 {
                let mut av = (0.0, 0.0);
                for c in 0..3 {
                    av = (
                        av.0 + rows[r][c] as f64 * vals[c].0,
                        av.1 + rows[r][c] as f64 * vals[c].1,
                    );
                }
                let lv = (
                    l.0 * vals[r].0 - l.1 * vals[r].1,
                    l.0 * vals[r].1 + l.1 * vals[r].0,
                );
                assert!(
                    close(av, lv, norm * (1.0 + l.0.abs() + l.1.abs())),
                    "A v != lambda v for {rows:?}: {av:?} vs {lv:?}"
                );
            }
            checked += 1;
        }
    }
    assert!(checked > 20, "only {checked} eigenpairs checked");
}

#[test]
fn eigenvectors_have_the_right_dimension_and_multiplicity() {
    let graph = ExprGraph::new();
    let eig = |lit: &str| {
        SymbolicMatrix::from_expr_str(&graph, lit)
            .unwrap()
            .eigenvectors(&graph)
            .unwrap()
    };
    // 3x3: vectors have three components (a 2x2 formula returned two).
    for (_, v) in eig("[[2,0,0],[0,3,0],[0,0,5]]") {
        assert_eq!(v.len(), 3);
    }
    // A diagonal 2x2 matrix: the zero vector was returned for the first eigenvalue.
    for (l, v) in eig("[[2,0],[0,3]]") {
        let vals: Vec<(f64, f64)> = v.iter().map(|&e| complex(&graph, e)).collect();
        assert!(
            vals.iter().any(|z| z.0.abs() + z.1.abs() > 1e-9),
            "zero eigenvector for {l:?}"
        );
    }
    // The identity has a two-dimensional eigenspace; a Jordan block has a one-dimensional one.
    assert_eq!(eig("[[2,0],[0,2]]").len(), 2);
    assert_eq!(eig("[[1,1],[0,1]]").len(), 1);
    // An irreducible quadratic: a numerical vector, still an eigenvector.
    let rows = vec![vec![2i64, 1], vec![1, 1]];
    let a = SymbolicMatrix::from_expr_str(&graph, &literal(&rows)).unwrap();
    for (lambda, v) in a.eigenvectors(&graph).unwrap() {
        let l = complex(&graph, lambda);
        let x: Vec<(f64, f64)> = v.iter().map(|&e| complex(&graph, e)).collect();
        for r in 0..2 {
            let av = (0..2).fold(0.0, |s, c| s + rows[r][c] as f64 * x[c].0);
            assert!(
                (av - l.0 * x[r].0).abs() < 1e-8,
                "A v != lambda v: {av} vs {}",
                l.0 * x[r].0
            );
        }
    }
}
