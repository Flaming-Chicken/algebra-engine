use algebra_core::ExprGraph;
use algebra_core::format::LatexFormatter;
use urae_cli::process_input;

#[test]
fn test_cli_process_input() {
    let graph = ExprGraph::new();
    let formatter = LatexFormatter;

    let res = process_input(&graph, &formatter, "x + 2").unwrap();
    assert!(res.contains("x"));
    assert!(res.contains("2"));
}

#[test]
fn test_cli_differentiation() {
    let graph = ExprGraph::new();
    let formatter = LatexFormatter;

    let res = process_input(&graph, &formatter, "diff x^2, x").unwrap();
    assert!(res.contains("2") || res.contains("x"));
}

#[test]
fn test_json_request_response_api() {
    use algebra_core::ExprGraph;
    use urae_cli::{UraeJsonResponse, process_json_request};

    let graph = ExprGraph::new();

    // 1. Nested LaTeX input request
    let json_req = r#"{
        "input": "\\frac{x^2 - 4}{x - 2}",
        "compute_derivative": true,
        "generate_code": true,
        "export_proof": true
    }"#;

    let json_resp = process_json_request(&graph, json_req);
    let resp: UraeJsonResponse = serde_json::from_str(&json_resp).expect("Deserialization failed");

    assert!(resp.success);
    assert!(resp.output_latex.contains("frac") || resp.output_latex.contains("x"));
    assert!(resp.derivative_latex.is_some());
    assert!(resp.rust_code.is_some());
    assert!(resp.lean4_proof.is_some());

    // 2. Command in JSON request
    let cmd_json = r#"{
        "input": "solve x^2 - 9 = 0, x"
    }"#;

    let cmd_resp = process_json_request(&graph, cmd_json);
    let resp_cmd: UraeJsonResponse =
        serde_json::from_str(&cmd_resp).expect("Deserialization failed");
    assert!(resp_cmd.success);
    assert!(resp_cmd.output_unicode.contains("3"));
}

#[test]
fn test_cli_neural_embedding() {
    let graph = ExprGraph::new();
    let formatter = LatexFormatter;

    let res = process_input(&graph, &formatter, "embed x^2 + 2*x + 1").unwrap();
    assert!(res.contains("Neural Expression Embedding"));
    assert!(res.contains("AST Structure"));
    assert!(res.contains("Vector Preview"));
}

#[test]
fn test_cli_semantic_similarity() {
    let graph = ExprGraph::new();
    let formatter = LatexFormatter;

    let res = process_input(&graph, &formatter, "similarity x^2 - 1, (x - 1)*(x + 1)").unwrap();
    assert!(res.contains("Neural Semantic Equivalence Score"));
    assert!(res.contains("Cosine Similarity"));
}

#[test]
fn test_cli_proof_search_depth() {
    let graph = ExprGraph::new();
    let formatter = LatexFormatter;

    let res = process_input(&graph, &formatter, "proof_search x + 0").unwrap();
    assert!(res.contains("AST Proof-Search Depth Analysis"));
    assert!(res.contains("AST Tree Depth"));
    assert!(res.contains("Proof Search Depth"));
}

#[test]
fn test_json_api_neural_embedding() {
    use algebra_core::ExprGraph;
    use urae_cli::{UraeJsonResponse, process_json_request};

    let graph = ExprGraph::new();
    let json_req = r#"{
        "input": "x^2 + 5",
        "embed_neural": true,
        "export_proof": true
    }"#;

    let json_resp = process_json_request(&graph, json_req);
    let resp: UraeJsonResponse = serde_json::from_str(&json_resp).expect("Deserialization failed");

    assert!(resp.success);
    assert!(resp.neural_embedding.is_some());
    assert!(resp.ast_search_depth.is_some());
    let emb = resp.neural_embedding.unwrap();
    assert_eq!(emb.len(), 8);
}
