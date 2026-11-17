use algebra_advanced::proof::{ProofTrace, UnificationEngine};
use algebra_core::ExprGraph;

#[test]
fn test_unification_identical() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let engine = UnificationEngine::new();

    let subst = engine.unify(&graph, x, x).unwrap();
    assert!(subst.mappings.is_empty());
}

#[test]
fn test_unification_symbol_and_num() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let num = graph.integer(42);
    let engine = UnificationEngine::new();

    let subst = engine.unify(&graph, x, num).unwrap();
    assert_eq!(subst.mappings.len(), 1);
}

#[test]
fn test_proof_trace_export() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let zero = graph.integer(0);
    let x_plus_0 = graph.add([x, zero]);

    let mut trace = ProofTrace::new();
    trace.add_step("add_zero_identity", x_plus_0, x);

    let lean = trace.export_lean4();
    assert!(lean.contains("add_zero_identity"));

    let coq = trace.export_coq();
    assert!(coq.contains("add_zero_identity"));
}

#[test]
fn test_proof_trace_export_states_the_actual_steps() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let y = graph.symbol("y");
    let square = graph.pow(graph.add([x, y]), graph.integer(2));
    let expanded = graph.add([
        graph.pow(x, graph.integer(2)),
        graph.mul([graph.integer(2), x, y]),
        graph.pow(y, graph.integer(2)),
    ]);
    let sine = graph.function("sin", [x]);

    let mut trace = ProofTrace::new();
    trace.add_step("expand_square", square, expanded);
    trace.add_step("untranslatable", sine, sine);

    let lean = trace.export_lean4_with(&graph);
    assert!(lean.contains("theorem step_1 (x y : ℝ)"), "{lean}");
    assert!(
        lean.contains("((x + y) ^ 2)") && lean.contains("(2 * x * y)"),
        "{lean}"
    );
    assert!(lean.contains("ring"), "{lean}");
    assert!(
        lean.contains("step 2 (untranslatable): not translated"),
        "{lean}"
    );
    assert!(!lean.contains("trivial"), "{lean}");

    let coq = trace.export_coq_with(&graph);
    assert!(
        coq.contains("Lemma step_1 : forall x : R, forall y : R,"),
        "{coq}"
    );
    assert!(coq.contains("Proof. intros; ring. Qed."), "{coq}");
    assert!(coq.contains("not translated"), "{coq}");
}
