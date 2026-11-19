use ahash::AHashMap;
use algebra_core::ExprGraph;
use algebra_engine::sparse::{CsrMatrix, MemoCache, SparsePoly};

#[test]
fn test_csr_matrix_vector_multiply() {
    use algebra_engine::numeric::{EvalContext, NumericalEval};
    use std::collections::HashMap;

    let graph = ExprGraph::new();
    let a = graph.symbol("a");
    let b = graph.symbol("b");
    let x1 = graph.symbol("x1");
    let x2 = graph.symbol("x2");

    // 2x2 CSR matrix [[a, 0], [0, b]] * [x1, x2]^T = [a*x1, b*x2]^T
    let csr = CsrMatrix::new(2, 2, vec![a, b], vec![0, 1], vec![0, 1, 2]).unwrap();
    let y = csr.multiply_vec(&graph, &[x1, x2]).unwrap();

    assert_eq!(y.len(), 2);
    assert_eq!(y[0], graph.mul([a, x1]));
    assert_eq!(y[1], graph.mul([b, x2]));

    let mut bindings = HashMap::new();
    bindings.insert("a".to_string(), 3.0);
    bindings.insert("b".to_string(), -4.0);
    bindings.insert("x1".to_string(), 5.0);
    bindings.insert("x2".to_string(), 2.5);
    let ctx = EvalContext::with_bindings(53, bindings);
    assert!((graph.evalf(y[0], &ctx).unwrap().to_f64() - 15.0).abs() < 1e-12);
    assert!((graph.evalf(y[1], &ctx).unwrap().to_f64() - (-10.0)).abs() < 1e-12);
}

#[test]
fn test_sparse_poly_to_expr() {
    use algebra_engine::numeric::{EvalContext, NumericalEval};
    use std::collections::HashMap;

    let graph = ExprGraph::new();
    let x_sym = graph.symbols.get_or_intern("x");

    // 3 * x^2
    let three = graph.integer(3);
    let mut terms = AHashMap::new();
    terms.insert(vec![2], three);

    let poly = SparsePoly::new(vec![x_sym], terms);
    let expr = poly.to_expr(&graph);

    let mut bindings = HashMap::new();
    bindings.insert("x".to_string(), 4.0);
    let ctx = EvalContext::with_bindings(53, bindings);
    // 3 * 4^2 = 48
    assert!((graph.evalf(expr, &ctx).unwrap().to_f64() - 48.0).abs() < 1e-12);
}

#[test]
fn test_memo_cache() {
    let cache = MemoCache::new();
    let expr = algebra_core::ExprId::new(0);
    let wrt = algebra_core::SymbolId::new(0);
    let rep = algebra_core::ExprId::new(1);

    let val1 = cache.get_or_insert_with(expr, wrt, rep, || algebra_core::ExprId::new(42));
    let val2 = cache.get_or_insert_with(expr, wrt, rep, || algebra_core::ExprId::new(99));

    assert_eq!(val1, val2);
    assert_eq!(val1.index(), 42);
}
