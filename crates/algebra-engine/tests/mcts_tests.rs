//! Integration tests for Monte Carlo Tree Search (MCTS) Proof Guidance and
//! Autonomous Rewrite Lemma Mining with Gröbner Basis Verification.

use algebra_core::ExprGraph;
use algebra_core::lemma_miner::{
    DomainKind, ExpressionGenerator, ExpressionGeneratorConfig, FingerprintTriple,
    FormalEquivalenceProver, LearnedLemma, LemmaRegistry,
};
use algebra_engine::lemma_miner::{AutonomousLemmaMiningPipeline, GrobnerProver};
use algebra_engine::mcts::{
    MctsAction, MctsConfig, MctsPolicyHead, MctsSearchEngine, MctsValueHead,
};
use std::collections::HashMap;

#[test]
fn test_expression_generator_synthesis() {
    let graph = ExprGraph::new();
    let config = ExpressionGeneratorConfig {
        max_depth: 3,
        var_names: vec!["x".to_string(), "y".to_string()],
        domain: DomainKind::Polynomial,
        constants: vec![-1, 0, 1, 2],
        seed: 42,
    };
    let mut generator = ExpressionGenerator::new(config);
    let batch = generator.generate_batch(&graph, 25);

    assert_eq!(batch.len(), 25);
    for &expr in &batch {
        assert!(expr.as_usize() < graph.len());
    }
}

#[test]
fn test_fingerprint_triple_consistency() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let y = graph.symbol("y");

    // E1 = (x + y)^2
    let x_plus_y = graph.add([x, y]);
    let two = graph.integer(2);
    let e1 = graph.pow(x_plus_y, two);

    // E2 = x^2 + 2*x*y + y^2
    let x_sq = graph.pow(x, two);
    let y_sq = graph.pow(y, two);
    let two_xy = graph.mul([two, x, y]);
    let e2 = graph.add([x_sq, two_xy, y_sq]);

    let sym_x = graph.symbols.get_or_intern("x");
    let sym_y = graph.symbols.get_or_intern("y");

    let mut env31 = HashMap::new();
    let mut env61 = HashMap::new();
    let mut env64 = HashMap::new();

    env31.insert(sym_x, 17);
    env31.insert(sym_y, 43);
    env61.insert(sym_x, 1009);
    env61.insert(sym_y, 2017);
    env64.insert(sym_x, 999983);
    env64.insert(sym_y, 777773);

    let fp1 = FingerprintTriple::compute(&graph, e1, &env31, &env61, &env64).expect("fp1 valid");
    let fp2 = FingerprintTriple::compute(&graph, e2, &env31, &env61, &env64).expect("fp2 valid");

    assert_eq!(
        fp1, fp2,
        "Schwartz-Zippel fingerprints must match for algebraically identical expressions"
    );
}

#[test]
fn test_grobner_prover_formal_verification() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let y = graph.symbol("y");

    // (x + y)^2 and x^2 + 2*x*y + y^2
    let x_plus_y = graph.add([x, y]);
    let two = graph.integer(2);
    let lhs = graph.pow(x_plus_y, two);

    let x_sq = graph.pow(x, two);
    let y_sq = graph.pow(y, two);
    let two_xy = graph.mul([two, x, y]);
    let rhs = graph.add([x_sq, two_xy, y_sq]);

    let prover = GrobnerProver::default();
    let var_names = vec!["x".to_string(), "y".to_string()];

    let is_proven = prover.prove_equivalence(&graph, lhs, rhs, &var_names);
    assert!(
        is_proven,
        "Grobner basis and expansion must formally prove (x+y)^2 == x^2 + 2xy + y^2"
    );

    // Non-equivalent expressions: (x + y) and (x - y)
    let non_equiv = graph.sub(x, y);
    assert!(
        !prover.prove_equivalence(&graph, x_plus_y, non_equiv, &var_names),
        "Non-equivalent expressions must not be proven"
    );
}

#[test]
fn test_lemma_registry_deduplication() {
    let registry = LemmaRegistry::new();
    assert_eq!(registry.len(), 0);
    assert!(registry.is_empty());

    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let y = graph.symbol("y");

    let lemma1 = LearnedLemma {
        id: "comm_add_1".to_string(),
        name: "Commutativity of addition".to_string(),
        lhs: graph.add([x, y]),
        rhs: graph.add([y, x]),
        var_names: vec!["x".to_string(), "y".to_string()],
        proof_method: "Axiomatic".to_string(),
        is_bidirectional: true,
    };

    assert!(registry.register(lemma1.clone()));
    assert_eq!(registry.len(), 1);

    // Duplicate registration should be rejected
    assert!(!registry.register(lemma1));
    assert_eq!(registry.len(), 1);

    let retrieved = registry.find_by_id("comm_add_1");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "Commutativity of addition");
}

#[test]
fn test_autonomous_lemma_mining_pipeline() {
    let graph = ExprGraph::new();
    let registry = LemmaRegistry::new();
    let config = ExpressionGeneratorConfig {
        max_depth: 2,
        var_names: vec!["x".to_string()],
        domain: DomainKind::Polynomial,
        constants: vec![0, 1],
        seed: 12345,
    };

    let mut pipeline = AutonomousLemmaMiningPipeline::new(config, registry);
    let discovered = pipeline.mine_batch(&graph, 30, 4);

    // Any discovered lemmas must have been registered into the pipeline's registry
    for lemma in &discovered {
        assert!(pipeline.registry().find_by_id(&lemma.id).is_some());
    }
}

#[test]
fn test_mcts_policy_head_generation() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let y = graph.symbol("y");
    let expr = graph.mul([graph.add([x, y]), graph.sub(x, y)]);

    let policy = MctsPolicyHead::compute_policy(&graph, expr, None);
    assert!(!policy.is_empty());

    let total_prob: f64 = policy.iter().map(|(_, p)| p).sum();
    assert!(
        (total_prob - 1.0).abs() < 1e-5,
        "Policy distribution must be normalized to sum to 1.0, got {total_prob}"
    );

    // Expand must be present with positive prior
    let expand_action = policy.iter().find(|(a, _)| matches!(a, MctsAction::Expand));
    assert!(expand_action.is_some());
}

#[test]
fn test_mcts_value_head_complexity() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let simple = x;

    let complex = graph.add([
        graph.mul([graph.integer(3), graph.pow(x, graph.integer(3))]),
        graph.mul([graph.integer(2), graph.pow(x, graph.integer(2))]),
        graph.integer(7),
    ]);

    let val_head = MctsValueHead::default();
    let val_simple = val_head.evaluate(&graph, simple, None);
    let val_complex = val_head.evaluate(&graph, complex, None);

    assert!(
        val_simple > val_complex,
        "Simpler expression must receive higher value than complex expression"
    );
}

#[test]
fn test_mcts_search_simplification_and_trajectory() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let y = graph.symbol("y");

    // Product of binomials: (x + y)(x - y)
    let x_plus_y = graph.add([x, y]);
    let x_minus_y = graph.sub(x, y);
    let root = graph.mul([x_plus_y, x_minus_y]);

    let config = MctsConfig {
        c_puct: 1.414,
        max_iterations: 20,
        max_depth: 5,
        rollout_horizon: 3,
    };
    let value_head = MctsValueHead::default();
    let engine = MctsSearchEngine::new(config, value_head, None);

    let result = engine.search(&graph, root, None);
    assert_eq!(result.root_expr, root.index());
    assert!(result.iterations > 0);
}

#[test]
fn test_mcts_trig_identity_search() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let sin_x = graph.function("sin", [x]);
    let cos_x = graph.function("cos", [x]);
    let two = graph.integer(2);

    let sin_sq = graph.pow(sin_x, two);
    let cos_sq = graph.pow(cos_x, two);
    let root = graph.add([sin_sq, cos_sq]);
    let target = graph.integer(1);

    let config = MctsConfig {
        c_puct: 1.414,
        max_iterations: 25,
        max_depth: 5,
        rollout_horizon: 2,
    };
    let value_head = MctsValueHead::default();
    let engine = MctsSearchEngine::new(config, value_head, None);

    let result = engine.search(&graph, root, Some(target));
    assert!(
        result.is_target_reached || result.best_value >= 0.9,
        "MCTS should discover the trigonometric identity sin^2(x) + cos^2(x) -> 1"
    );
}

#[test]
fn test_lake_trajectory_export() {
    let temp_dir = std::env::temp_dir().join("spodeian_test_mcts_lake");
    let engine = MctsSearchEngine::new(MctsConfig::default(), MctsValueHead::default(), None);

    let search_res = algebra_engine::mcts::MctsSearchResult {
        root_expr: 0,
        best_expr: 1,
        trajectory: vec![],
        best_value: 0.95,
        iterations: 10,
        is_target_reached: true,
    };

    let path = engine
        .stream_trajectories_to_lake(&temp_dir, &[search_res])
        .expect("Streaming to lake must succeed");
    assert!(path.exists());

    let _ = std::fs::remove_dir_all(&temp_dir);
}
