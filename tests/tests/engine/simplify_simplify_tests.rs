use algebra_core::ExprGraph;
use algebra_engine::simplify::Simplifier;

#[test]
fn test_egraph_simplification() {
    let graph = ExprGraph::new();

    // x + 0 -> x
    let x = graph.symbol("x");
    let zero = graph.integer(0);
    let add_zero = graph.add([x, zero]);

    let simplified = Simplifier::simplify(&graph, add_zero).unwrap();
    assert_eq!(simplified, x);
}

#[test]
fn test_matrix_identity_simplification() {
    let graph = ExprGraph::new();

    // A * I -> A
    let a = graph.symbol("A");
    let eye = graph.symbol("I");
    let mul_eye = graph.mul([a, eye]);

    let simplified = Simplifier::simplify(&graph, mul_eye).unwrap();
    assert_eq!(simplified, a);
}

#[test]
fn test_quaternion_rewrite_simplification() {
    let graph = ExprGraph::new();

    // qi * qj -> qk (plain i, j, k are ordinary variables)
    let qi = graph.symbol("qi");
    let qj = graph.symbol("qj");
    let qk = graph.symbol("qk");
    let mul_ij = graph.mul([qi, qj]);

    let simplified = Simplifier::simplify(&graph, mul_ij).unwrap();
    assert_eq!(simplified, qk);
}

#[test]
fn test_algebraic_format_transformations() {
    use algebra_core::Domain;
    use algebra_engine::simplify::AlgebraicTransformations;

    let graph = ExprGraph::new();

    // 1. Test expand: (x + 1)^2 -> x^2 + 2x + 1
    let x = graph.symbol("x");
    let zero = graph.integer(0);
    let one = graph.integer(1);
    let two = graph.integer(2);
    let x_plus_1 = graph.add([x, one]);
    let pow_expr = graph.pow(x_plus_1, two);

    let expanded = AlgebraicTransformations::expand(&graph, pow_expr);
    assert_ne!(expanded, pow_expr);

    // 2. Test collect: a*x + b*x -> (a + b)*x
    let a = graph.symbol("a");
    let b = graph.symbol("b");
    let ax = graph.mul([a, x]);
    let bx = graph.mul([b, x]);
    let ax_plus_bx = graph.add([ax, bx]);
    let x_sym = graph.symbols.get_or_intern("x");

    let collected = AlgebraicTransformations::collect(&graph, ax_plus_bx, x_sym);
    assert_ne!(collected, ax_plus_bx);

    // 3. Test together: 1/x + 1/y -> (x + y) / (x*y)
    let y = graph.symbol("y");
    let div_x = graph.div(one, x);
    let div_y = graph.div(one, y);
    let sum_div = graph.add([div_x, div_y]);

    let together_expr = AlgebraicTransformations::together(&graph, sum_div);
    assert_ne!(together_expr, sum_div);

    // 4. Test cancel: (x + 1) / (x + 1) -> 1
    let div_self = graph.div(x_plus_1, x_plus_1);
    let canceled = AlgebraicTransformations::cancel(&graph, div_self);
    assert_eq!(canceled, one);

    // 5. Test domain promotion across algebras
    let c_dom = Domain::Complex;
    let h_dom = Domain::Hypercomplex { dimension: 4 }; // Quaternion
    let m_dom = Domain::Matrix {
        rows: 2,
        cols: 2,
        element_domain: Box::new(Domain::Reals),
    };

    let promoted_h = Domain::promote(&c_dom, &h_dom);
    assert_eq!(promoted_h, h_dom);

    let promoted_m = Domain::promote(&c_dom, &m_dom);
    assert_eq!(
        promoted_m,
        Domain::Matrix {
            rows: 2,
            cols: 2,
            element_domain: Box::new(Domain::Complex),
        }
    );

    // 6. Test cost metric extractions (min_tree, min_ops, min_leaf)
    let add_zero = graph.add([x, zero]);
    let mt = Simplifier::min_tree(&graph, add_zero).unwrap();
    let mo = Simplifier::min_ops(&graph, add_zero).unwrap();
    let ml = Simplifier::min_leaf(&graph, add_zero).unwrap();
    assert_eq!(mt, x);
    assert_eq!(mo, x);
    assert_eq!(ml, x);

    // 7. Test horner polynomial form: x^2 + 2x + 1 -> ((1*x + 2)*x + 1)
    let x_sq = graph.pow(x, two);
    let two_x = graph.mul([two, x]);
    let poly = graph.add([x_sq, two_x, one]);
    let horner_expr = AlgebraicTransformations::horner(&graph, poly, x_sym);
    assert_ne!(horner_expr, poly);

    // 8. Test polynomial factoring: x^2 - 4 -> (x - 2)(x + 2)
    let four = graph.integer(4);
    let x_sq_minus_4 = graph.sub(x_sq, four);
    let factored = AlgebraicTransformations::factor(&graph, x_sq_minus_4);
    assert_ne!(factored, x_sq_minus_4);

    // 9. Test partial fraction decomposition: 1 / (x^2 - 1)
    let x_sq_minus_1 = graph.sub(x_sq, one);
    let rational = graph.div(one, x_sq_minus_1);
    let pf = AlgebraicTransformations::partial_fractions(&graph, rational, x_sym);
    assert_ne!(pf, rational);
}
