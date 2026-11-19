use algebra_core::ExprGraph;
use algebra_engine::matrix::SymbolicMatrix;

#[test]
fn test_matrix_identity_and_multiply() {
    use algebra_engine::numeric::{EvalContext, NumericalEval};
    use std::collections::HashMap;

    let graph = ExprGraph::new();
    let eye = SymbolicMatrix::identity(&graph, 2);

    let a = graph.symbol("a");
    let b = graph.symbol("b");
    let c = graph.symbol("c");
    let d = graph.symbol("d");

    let mat = SymbolicMatrix::new(2, 2, vec![a, b, c, d]).unwrap();
    let prod = eye.multiply(&graph, &mat).unwrap();

    assert_eq!(prod.rows, 2);
    assert_eq!(prod.cols, 2);

    let mut bindings = HashMap::new();
    bindings.insert("a".to_string(), 2.0);
    bindings.insert("b".to_string(), 3.0);
    bindings.insert("c".to_string(), 5.0);
    bindings.insert("d".to_string(), 7.0);
    let ctx = EvalContext::with_bindings(53, bindings);
    assert!((graph.evalf(prod.get(0, 0), &ctx).unwrap().to_f64() - 2.0).abs() < 1e-12);
    assert!((graph.evalf(prod.get(0, 1), &ctx).unwrap().to_f64() - 3.0).abs() < 1e-12);
    assert!((graph.evalf(prod.get(1, 0), &ctx).unwrap().to_f64() - 5.0).abs() < 1e-12);
    assert!((graph.evalf(prod.get(1, 1), &ctx).unwrap().to_f64() - 7.0).abs() < 1e-12);
}

#[test]
fn test_matrix_determinant_2x2() {
    use algebra_engine::numeric::{EvalContext, NumericalEval};
    use std::collections::HashMap;

    let graph = ExprGraph::new();
    let a = graph.symbol("a");
    let b = graph.symbol("b");
    let c = graph.symbol("c");
    let d = graph.symbol("d");

    let mat = SymbolicMatrix::new(2, 2, vec![a, b, c, d]).unwrap();
    let det = mat.determinant(&graph).unwrap();

    // det([[2, 3], [5, 7]]) = 2*7 - 3*5 = -1
    let mut bindings = HashMap::new();
    bindings.insert("a".to_string(), 2.0);
    bindings.insert("b".to_string(), 3.0);
    bindings.insert("c".to_string(), 5.0);
    bindings.insert("d".to_string(), 7.0);
    let ctx = EvalContext::with_bindings(53, bindings);
    let det_val = graph.evalf(det, &ctx).unwrap().to_f64();
    assert!(
        (det_val - (-1.0)).abs() < 1e-12,
        "expected -1, got {det_val}"
    );
}

#[test]
fn test_matrix_lu_decomposition_2x2() {
    let graph = ExprGraph::new();
    let a = graph.symbol("a");
    let b = graph.symbol("b");
    let c = graph.symbol("c");
    let d = graph.symbol("d");

    let mat = SymbolicMatrix::new(2, 2, vec![a, b, c, d]).unwrap();
    let (l, u) = mat.lu_decomposition(&graph).unwrap();

    assert_eq!(l.rows, 2);
    assert_eq!(u.rows, 2);
    assert_eq!(l.get(0, 0), graph.integer(1));
    assert_eq!(l.get(0, 1), graph.integer(0));
    assert_eq!(u.get(0, 0), a);
}

#[test]
fn test_matrix_cholesky_decomposition_2x2() {
    let graph = ExprGraph::new();
    let a = graph.symbol("a");
    let b = graph.symbol("b");
    let c = graph.symbol("c");

    // Symmetric matrix [[a, b], [b, c]]
    let mat = SymbolicMatrix::new(2, 2, vec![a, b, b, c]).unwrap();
    let l = mat.cholesky_decomposition(&graph).unwrap();

    assert_eq!(l.rows, 2);
    assert_eq!(l.cols, 2);
    assert_eq!(l.get(0, 1), graph.integer(0));
}

#[test]
fn test_matrix_qr_decomposition_2x2() {
    let graph = ExprGraph::new();
    let a = graph.symbol("a");
    let b = graph.symbol("b");
    let c = graph.symbol("c");
    let d = graph.symbol("d");

    let mat = SymbolicMatrix::new(2, 2, vec![a, b, c, d]).unwrap();
    let (q, r) = mat.qr_decomposition(&graph).unwrap();

    assert_eq!(q.rows, 2);
    assert_eq!(r.rows, 2);
    assert_eq!(r.get(1, 0), graph.integer(0));
}

#[test]
fn test_matrix_pauli_generators() {
    let graph = ExprGraph::new();
    let (sx, sy, sz) = SymbolicMatrix::pauli_generators(&graph);

    assert_eq!(sx.rows, 2);
    assert_eq!(sy.rows, 2);
    assert_eq!(sz.rows, 2);
    assert_eq!(sz.get(0, 0), graph.integer(1));
    assert_eq!(sz.get(1, 1), graph.integer(-1));
}

#[test]
fn test_matrix_buffer_mutable() {
    use algebra_engine::matrix::{LieAlgebraFamily, MatrixBuffer};

    let mut buf = MatrixBuffer::zeros(2, 2);
    buf.set(0, 0, 1.5);
    buf.set(1, 1, 3.0);

    let eye = MatrixBuffer::identity(2);
    buf.add_in_place(&eye);
    assert_eq!(buf.get(0, 0), 2.5);
    assert_eq!(buf.get(1, 1), 4.0);

    let graph = ExprGraph::new();
    let sym_mat = buf.to_symbolic_matrix(&graph).unwrap();
    assert_eq!(sym_mat.rows, 2);

    // Lie Algebra Cartan Matrix for A_2 (su(3))
    let a2 = LieAlgebraFamily::A(2);
    let cartan_a2 = a2.cartan_matrix(&graph).unwrap();
    assert_eq!(cartan_a2.rows, 2);
    assert_eq!(cartan_a2.get(0, 0), graph.integer(2));
    assert_eq!(cartan_a2.get(0, 1), graph.integer(-1));
}
