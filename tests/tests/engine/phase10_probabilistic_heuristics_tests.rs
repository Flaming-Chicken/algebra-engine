//! Integration Tests for Phase 10: Multi-Tier Probabilistic Heuristic Engine & Solution Certification.

use algebra_core::probabilistic::{ProbabilisticVerifier, Solution};
use algebra_core::{EngineConfig, ExprGraph, ResourceBudget};
use algebra_engine::heuristic::HeuristicSearchEngine;

#[test]
fn test_solution_enum_properties() {
    let det = Solution::Deterministic(42);
    assert!(det.is_deterministic());
    assert!(!det.is_probabilistic());
    assert_eq!(det.error_probability(), 0.0);
    assert_eq!(det.confidence(), 1.0);
    assert_eq!(det.into_inner(), 42);

    let prob = Solution::Probabilistic {
        result: "x^2 + 1",
        error_probability_upper_bound: 1e-20,
        sample_count: 24,
        method: "Schwartz-Zippel CRT",
    };
    assert!(!prob.is_deterministic());
    assert!(prob.is_probabilistic());
    assert_eq!(prob.error_probability(), 1e-20);
    assert!((prob.confidence() - 1.0).abs() < 1e-15);
    assert_eq!(prob.sample_count(), 24);
    assert_eq!(prob.method(), "Schwartz-Zippel CRT");

    let mapped = prob.map(|s| format!("Result: {s}"));
    assert_eq!(mapped.into_inner(), "Result: x^2 + 1");
}

#[test]
fn test_solution_display_formatting() {
    let det = Solution::Deterministic(100);
    assert_eq!(format!("{det}"), "100");

    let prob = Solution::Probabilistic {
        result: 100,
        error_probability_upper_bound: 1e-12,
        sample_count: 15,
        method: "Triple Mersenne CRT",
    };
    let s = format!("{prob}");
    assert!(s.contains("100"));
    assert!(s.contains("Probabilistic Certificate"));
    assert!(s.contains("Triple Mersenne CRT"));
}

#[test]
fn test_tier1_fast_schwartz_zippel_fingerprinting() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let y = graph.symbol("y");

    // (x + y) * (x - y) vs x^2 - y^2
    let add = graph.add([x, y]);
    let sub = graph.sub(x, y);
    let prod = graph.mul([add, sub]);

    let x2 = graph.pow(x, graph.integer(2));
    let y2 = graph.pow(y, graph.integer(2));
    let diff = graph.sub(x2, y2);

    let fp1 = ProbabilisticVerifier::fingerprint(&graph, prod, 42);
    let fp2 = ProbabilisticVerifier::fingerprint(&graph, diff, 42);
    assert_eq!(
        fp1, fp2,
        "Polynomial identity should yield identical fingerprint"
    );

    let weight =
        ProbabilisticVerifier::compute_candidate_heuristic_weight(&graph, diff, Some(prod));
    assert!(
        weight > 2.0,
        "Matching fingerprint should significantly boost candidate heuristic weight"
    );
}

#[test]
fn test_tier2_multi_prime_crt_identity_verification() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let y = graph.symbol("y");

    // (x + y)^2 vs x^2 + 2xy + y^2
    let sum = graph.add([x, y]);
    let sum_sq = graph.pow(sum, graph.integer(2));

    let x2 = graph.pow(x, graph.integer(2));
    let y2 = graph.pow(y, graph.integer(2));
    let two_xy = graph.mul([graph.integer(2), x, y]);
    let expanded = graph.add([x2, two_xy, y2]);

    let verif = ProbabilisticVerifier::verify_equivalence(&graph, sum_sq, expanded, 1e-15);
    assert!(verif.is_probabilistic());
    assert!(*verif.as_ref());
    assert!(verif.error_probability() <= 1e-15);
    assert!(verif.sample_count() >= 3);
}

#[test]
fn test_tier2_refutation_of_inequivalent_expressions() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let y = graph.symbol("y");

    // x^2 + y^2 != (x + y)^2
    let x2 = graph.pow(x, graph.integer(2));
    let y2 = graph.pow(y, graph.integer(2));
    let sum_of_sq = graph.add([x2, y2]);

    let sum = graph.add([x, y]);
    let sq_of_sum = graph.pow(sum, graph.integer(2));

    let verif = ProbabilisticVerifier::verify_equivalence(&graph, sum_of_sq, sq_of_sum, 1e-10);
    assert!(verif.is_deterministic());
    assert!(
        !(*verif.as_ref()),
        "Refutation should be deterministically disproven by counterexample"
    );
}

#[test]
fn test_heuristic_search_engine_simplification() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");

    // x + 0
    let zero = graph.integer(0);
    let x_plus_zero = graph.add([x, zero]);

    let engine = HeuristicSearchEngine::default();
    let sol = engine.simplify_with_certification(&graph, x_plus_zero, 1e-15);

    assert_eq!(sol.into_inner(), x, "x + 0 simplifies directly to x");
}

#[test]
fn test_heuristic_search_engine_hard_cap_graceful_fallback() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");

    let config = EngineConfig {
        budget: ResourceBudget {
            egraph_nodes: (1, 2), // Extremely tight budget to trigger hard cap
            ..ResourceBudget::default()
        },
        ..EngineConfig::default()
    };

    let engine = HeuristicSearchEngine::new(config);
    let sol = engine.simplify_with_certification(&graph, x, 1e-12);

    assert_eq!(sol.into_inner(), x);
}

#[test]
fn test_simplifier_simplify_certified() {
    use algebra_engine::simplify::Simplifier;
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let zero = graph.integer(0);
    let add_zero = graph.add([x, zero]);

    let cert = Simplifier::simplify_certified(&graph, add_zero, 1e-15).unwrap();
    assert_eq!(*cert.as_ref(), x);
    assert!(cert.confidence() >= 1.0 - 1e-15);
}

#[test]
fn test_symbolic_solver_solve_certified() {
    use algebra_engine::solver::SymbolicSolver;
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let x_sym = graph.symbols.get_or_intern("x");

    // x^2 - 4 == 0 -> x = 2, -2
    let x2 = graph.pow(x, graph.integer(2));
    let eq = graph.sub(x2, graph.integer(4));

    let sol = graph.solveset_certified(eq, x_sym, 1e-12).unwrap();
    let roots = sol.into_inner();
    assert_eq!(roots.len(), 2);
}

#[test]
fn test_transcendental_trig_identity_verification() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");

    // sin(x)^2 + cos(x)^2 vs 1
    let sin_x = graph.function("sin", [x]);
    let cos_x = graph.function("cos", [x]);
    let sin2 = graph.pow(sin_x, graph.integer(2));
    let cos2 = graph.pow(cos_x, graph.integer(2));
    let pythagoras = graph.add([sin2, cos2]);
    let one = graph.integer(1);

    let verif = ProbabilisticVerifier::verify_equivalence(&graph, pythagoras, one, 1e-10);
    assert!(verif.is_probabilistic());
    assert!(*verif.as_ref());
    assert!(verif.error_probability() <= 1e-6);
}

#[test]
fn test_quantized_transformer_neural_embeddings_and_search_guidance() {
    use algebra_engine::heuristic::{
        NeuralExpressionEmbedder, QuantizedTransformerWeights,
    };

    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let y = graph.symbol("y");
    let two = graph.integer(2);
    let one = graph.integer(1);

    // Expression A: x^2 + 1
    let x2 = graph.pow(x, two);
    let expr_a = graph.add([x2, one]);

    // Expression B: y^2 + 1 (structurally identical tree)
    let y2 = graph.pow(y, two);
    let expr_b = graph.add([y2, one]);

    // Expression C: sin(x) / cos(x) (structurally different)
    let sin_x = graph.function("sin", [x]);
    let cos_x = graph.function("cos", [x]);
    let expr_c = graph.div(sin_x, cos_x);

    let embedder = NeuralExpressionEmbedder::default();

    let emb_a = embedder.embed_expression(&graph, expr_a);
    let emb_b = embedder.embed_expression(&graph, expr_b);
    let emb_c = embedder.embed_expression(&graph, expr_c);

    assert_eq!(emb_a.len(), 8);
    assert_eq!(emb_b.len(), 8);
    assert_eq!(emb_c.len(), 8);

    // Verify unit length
    let norm_a: f32 = emb_a.iter().map(|&v| v * v).sum::<f32>().sqrt();
    assert!((norm_a - 1.0).abs() < 1e-4);

    // Expressions with identical tree structure have high similarity
    let sim_ab = NeuralExpressionEmbedder::cosine_similarity(&emb_a, &emb_b);
    let sim_ac = NeuralExpressionEmbedder::cosine_similarity(&emb_a, &emb_c);

    assert!(
        sim_ab > sim_ac,
        "Structural isomorphic expressions should have higher similarity: sim_ab={sim_ab} vs sim_ac={sim_ac}"
    );

    // Test Safetensors ingestion with INT8 quantization
    let header_json = serde_json::json!({
        "__metadata__": { "scale": "0.05" },
        "embeddings": {
            "dtype": "I8",
            "shape": [16, 8],
            "data_offsets": [0, 128]
        }
    })
    .to_string();

    let header_bytes = header_json.as_bytes();
    let header_len = header_bytes.len() as u64;

    let mut st_buffer = Vec::new();
    st_buffer.extend_from_slice(&header_len.to_le_bytes());
    st_buffer.extend_from_slice(header_bytes);
    // Add 128 bytes of INT8 data
    let int8_data: Vec<u8> = (0..128).map(|i| (i as i8) as u8).collect();
    st_buffer.extend_from_slice(&int8_data);

    let loaded_weights =
        QuantizedTransformerWeights::from_safetensors_bytes(&st_buffer).unwrap();
    assert_eq!(loaded_weights.vocab_size, 16);
    assert_eq!(loaded_weights.embed_dim, 8);
    assert_eq!(loaded_weights.scale, 0.05);
    assert_eq!(loaded_weights.token_embeddings.len(), 128);

    // Test HeuristicSearchEngine with neural embedder active
    let search_engine = HeuristicSearchEngine::default().with_neural_embedder(embedder);

    let weight_guided = search_engine.weight_candidate(&graph, expr_b, Some(expr_a));
    let weight_unguided =
        ProbabilisticVerifier::compute_candidate_heuristic_weight(&graph, expr_b, Some(expr_a));

    assert!(
        weight_guided >= weight_unguided,
        "Neural guidance should amplify promising structurally similar candidate: guided={weight_guided} vs unguided={weight_unguided}"
    );
}

#[test]
fn test_parallel_verification_and_weighting() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let y = graph.symbol("y");

    // (x + y)^2 vs x^2 + 2xy + y^2
    let sum = graph.add(vec![x, y]);
    let two = graph.integer(2);
    let lhs = graph.pow(sum, two);

    let x2 = graph.pow(x, two);
    let y2 = graph.pow(y, two);
    let xy = graph.mul(vec![two, x, y]);
    let rhs = graph.add(vec![x2, xy, y2]);

    // Test parallel equivalence verification
    let solution = ProbabilisticVerifier::verify_equivalence_parallel(&graph, lhs, rhs, 1e-12);
    assert!(solution.is_probabilistic(), "Expected probabilistic solution");
    assert_eq!(*solution.as_ref(), true);
    assert!(solution.confidence() > 0.99999);

    // Test parallel candidate weighting
    let search_engine = HeuristicSearchEngine::default();
    let candidates = vec![lhs, rhs, x2, y2, sum];
    let weights = search_engine.weight_candidates_parallel(&graph, &candidates, Some(rhs));
    assert_eq!(weights.len(), 5);
    for &w in &weights {
        assert!(w > 0.0);
    }
}
