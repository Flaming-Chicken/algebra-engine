//! # Integration Test Suite for `algebra-engine`
//!
//! Tests covering:
//! - Gröbner basis computation, expression reduction, and ideal membership on `ExprGraph`.
//! - SMT translation directly from `ExprGraph` nodes into `SmtExpr`.
//! - Pluggable `SearchHeuristic` implementations:
//!   - `TreeSizeHeuristic`
//!   - `SchwartzZippelHeuristic`
//!   - `NeuralExpressionEmbedder`
//!   - `CompositeSearchHeuristic`
//! - `UraeSession` stateful persistence and execution.

use algebra_core::{ExprGraph, RelOp};
use algebra_engine::heuristic::{
    CompositeSearchHeuristic, HeuristicSearchEngine, NeuralExpressionEmbedder,
    SchwartzZippelHeuristic, SearchHeuristic, TreeSizeHeuristic,
};
use algebra_engine::poly::{GrobnerBasis, MonomialOrder};
use algebra_engine::session::UraeSession;
use algebra_engine::smt::SmtExpr;

#[test]
fn test_grobner_basis_on_expr_graph() {
    let graph = ExprGraph::new();

    // Variables: x, y
    let x = graph.symbol("x");
    let y = graph.symbol("y");
    let two = graph.integer(2);
    let one = graph.integer(1);

    // Generators: g1 = x^2 - y, g2 = x * y - x
    let x_sq = graph.pow(x, two);
    let g1 = graph.sub(x_sq, y);

    let xy = graph.mul([x, y]);
    let g2 = graph.sub(xy, x);

    let var_names = vec!["x".to_string(), "y".to_string()];
    let gb = GrobnerBasis::from_exprs(&graph, &[g1, g2], &var_names, MonomialOrder::Lex)
        .expect("Gröbner basis computation failed");

    // The ideal contains g1 and g2, so both must reduce to 0
    assert!(
        gb.contains_expr(&graph, g1).unwrap(),
        "Ideal must contain generator g1"
    );
    assert!(
        gb.contains_expr(&graph, g2).unwrap(),
        "Ideal must contain generator g2"
    );

    // Test reducing g1 + one: should reduce to constant 1
    let g1_plus_1 = graph.add([g1, one]);
    let rem = gb.reduce_expr(&graph, g1_plus_1).unwrap();
    let rem_node = graph.get(rem);
    assert!(
        matches!(
            rem_node.kind,
            algebra_core::ExprKind::Number(algebra_core::Number::Integer(1))
        ),
        "Remainder of g1 + 1 modulo <g1, g2> must be 1"
    );
}

#[test]
fn test_smt_expr_from_graph() {
    let graph = ExprGraph::new();

    // Expression: x^2 + 2*x + 1 == 0
    let x = graph.symbol("x");
    let two = graph.integer(2);
    let one = graph.integer(1);
    let zero = graph.integer(0);

    let x_sq = graph.pow(x, two);
    let two_x = graph.mul([two, x]);
    let poly = graph.add([x_sq, two_x, one]);
    let eq = graph.relational(RelOp::Equal, poly, zero);

    let smt_expr = SmtExpr::from_graph(&graph, eq).expect("SMT conversion failed");
    let smt_str = smt_expr.to_smtlib2();

    assert!(smt_str.starts_with("(= "));
    assert!(smt_str.contains("(+ "));
    assert!(smt_str.contains("(^ x 2)"));
    assert!(smt_str.contains("(* 2 x)"));
}

#[test]
fn test_pluggable_search_heuristics() {
    let graph = ExprGraph::new();

    let x = graph.symbol("x");
    let one = graph.integer(1);
    let x_plus_1 = graph.add([x, one]);

    // 1. TreeSizeHeuristic
    let size_h = TreeSizeHeuristic;
    let size_score = size_h.evaluate(&graph, x_plus_1, None);
    let x_score = size_h.evaluate(&graph, x, None);
    assert_eq!(size_h.name(), "TreeSizeHeuristic");
    assert!(
        x_score > size_score,
        "Simpler expression x must have higher priority than x + 1 under TreeSizeHeuristic"
    );

    // 2. SchwartzZippelHeuristic
    let sz_h = SchwartzZippelHeuristic;
    let sz_score = sz_h.evaluate(&graph, x, Some(x));
    assert_eq!(sz_h.name(), "SchwartzZippelHeuristic");
    assert!(sz_score > 0.0);

    // 3. NeuralExpressionEmbedder
    let neural_h = NeuralExpressionEmbedder::default();
    assert_eq!(neural_h.name(), "NeuralExpressionEmbedder");
    let neural_score = neural_h.evaluate(&graph, x, Some(x));
    assert!(neural_score > 0.0);

    // 4. CompositeSearchHeuristic
    let composite = CompositeSearchHeuristic::new(vec![
        (Box::new(SchwartzZippelHeuristic), 1.0),
        (Box::new(TreeSizeHeuristic), 0.5),
        (Box::new(neural_h), 0.25),
    ]);
    assert_eq!(composite.name(), "CompositeSearchHeuristic");
    let comp_score = composite.evaluate(&graph, x_plus_1, Some(x));
    assert!(comp_score > 0.0);

    // 5. HeuristicSearchEngine with custom heuristic
    let engine = HeuristicSearchEngine::new(algebra_core::EngineConfig::default())
        .with_heuristic(Box::new(composite));
    let candidate_weight = engine.weight_candidate(&graph, x_plus_1, Some(x));
    assert!(candidate_weight > 0.0);
}

#[test]
fn test_urae_session_variable_persistence() {
    let mut session = UraeSession::new();
    let res1 = session.execute_line("a = 5.0");
    assert!(!res1.is_error);
    assert_eq!(session.get_binding("a"), Some(5.0));

    let res2 = session.execute_line("eval a * 2");
    assert!(!res2.is_error);
    assert!(res2.output_text.contains("10") || res2.output_unicode.contains("10"));
}

#[test]
fn test_urae_session_domain_bounds() {
    let mut session = UraeSession::new();
    let res = session.execute_line("x in Reals [0, 100]");
    assert!(!res.is_error);
    assert!(session.get_domain("x").is_some());
    let domain = session.get_domain("x").unwrap();
    assert_eq!(domain.min_val, Some(0.0));
    assert_eq!(domain.max_val, Some(100.0));
}

#[test]
fn test_urae_session_calculus_evaluation() {
    let mut session = UraeSession::new();
    let res = session.execute_line("diff x^3, x");
    assert!(!res.is_error);
    assert!(res.output_unicode.contains("3 * x ^ 2") || res.output_latex.contains("3"));
}
