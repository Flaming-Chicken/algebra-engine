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
    let x_sym = graph.symbols.get_or_intern("x");
    let x = graph.symbol("x");
    let num = graph.integer(42);
    let engine = UnificationEngine::new();

    let subst = engine.unify(&graph, x, num).unwrap();
    assert_eq!(subst.mappings.len(), 1);
    assert_eq!(subst.mappings.get(&x_sym), Some(&num));

    // Document current limitation: compound ASTs (Add/Mul/Pow) are not recursively unified yet
    let x_plus_1 = graph.add([x, graph.integer(1)]);
    let two_plus_1 = graph.add([graph.integer(2), graph.integer(1)]);
    assert!(engine.unify(&graph, x_plus_1, two_plus_1).is_err());
}

#[test]
fn test_proof_trace_export() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let zero = graph.integer(0);
    let x_plus_0 = graph.add([x, zero]);

    let mut trace = ProofTrace::new();
    trace.add_step("add_zero_identity", x_plus_0, x);
    assert_eq!(trace.verify_steps(&graph), Ok(1));

    let lean = trace.export_lean4();
    assert!(lean.contains("theorem algebraic_simplification"));
    assert!(lean.contains("add_zero_identity"));

    let coq = trace.export_coq();
    assert!(coq.contains("Theorem algebraic_simplification"));
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

#[test]
fn test_unification_is_structural_with_occurs_check() {
    use algebra_advanced::proof::UnificationEngine;
    let graph = ExprGraph::new();
    let engine = UnificationEngine::new();
    let (x, y, z) = (graph.symbol("x"), graph.symbol("y"), graph.symbol("z"));
    let three = graph.integer(3);
    let f = |args: &[algebra_core::ExprId]| graph.function("f", args.iter().copied());
    let g1 = |a| graph.function("g", [a]);

    // f(x, g(y)) with f(z, g(3)): x -> z, y -> 3, and both sides become the same term.
    let (left, right) = (f(&[x, g1(y)]), f(&[z, g1(three)]));
    let subst = engine.unify(&graph, left, right).unwrap();
    assert_eq!(subst.apply(&graph, left), subst.apply(&graph, right));
    assert_eq!(subst.apply(&graph, y), three);

    // Bindings chain: f(x, y) with f(y, 3) gives x = y = 3.
    let (left, right) = (f(&[x, y]), f(&[y, three]));
    let subst = engine.unify(&graph, left, right).unwrap();
    assert_eq!(subst.apply(&graph, x), three);
    assert_eq!(subst.apply(&graph, left), subst.apply(&graph, right));

    // Occurs check: x cannot equal f(x).
    assert!(engine.unify(&graph, x, f(&[x])).is_err());
    // Different heads, different arities and clashing constants fail.
    assert!(engine.unify(&graph, f(&[x]), g1(x)).is_err());
    assert!(engine.unify(&graph, f(&[x]), f(&[x, y])).is_err());
    assert!(engine.unify(&graph, graph.integer(1), three).is_err());
    // Operators match argument by argument.
    let sum_left = graph.add([x, three]);
    let sum_right = graph.add([y, three]);
    let subst = engine.unify(&graph, sum_left, sum_right).unwrap();
    assert_eq!(
        subst.apply(&graph, sum_left),
        subst.apply(&graph, sum_right)
    );
    // Syntactic, not up to arithmetic: x + 3 does not unify with y + 5.
    assert!(
        engine
            .unify(&graph, sum_left, graph.add([y, graph.integer(5)]))
            .is_err()
    );
    assert!(
        engine
            .unify(&graph, graph.add([x, y]), graph.mul([x, y]))
            .is_err()
    );
}
