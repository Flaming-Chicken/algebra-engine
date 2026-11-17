use algebra_advanced::codegen::{CodeGenerator, TargetLanguage};
use algebra_core::ExprGraph;

#[test]
fn test_emit_rust_code() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");

    // x^2 + 5
    let two = graph.integer(2);
    let five = graph.integer(5);
    let x_sq = graph.pow(x, two);
    let expr = graph.add([x_sq, five]);

    let rust_code = CodeGenerator::emit_code(&graph, expr, TargetLanguage::Rust).unwrap();
    assert_eq!(rust_code, "(x.powf(2.0) + 5.0)");
}

#[test]
fn test_compile_closure_evaluation() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let x_sym = graph.symbols.get_or_intern("x");

    // f(x) = x + 3
    let three = graph.integer(3);
    let expr = graph.add([x, three]);

    let f = CodeGenerator::compile_fn1(&graph, expr, x_sym);
    let result = f(10.0);

    assert_eq!(result, 13.0);
}

#[test]
fn compiled_closures_agree_with_the_tree_evaluator() {
    use algebra_engine::numeric::{EvalContext, NumericalEval};
    let graph = ExprGraph::new();
    let (x, y, z) = (graph.symbol("x"), graph.symbol("y"), graph.symbol("z"));
    let ids: Vec<_> = ["x", "y", "z"]
        .iter()
        .map(|n| graph.symbols.get_or_intern(n))
        .collect();

    // sin(x) * exp(y / 3) + x^2 - z / (1 + y^2)
    let expr = graph.sub(
        graph.add([
            graph.mul([
                graph.function("sin", [x]),
                graph.function("exp", [graph.div(y, graph.integer(3))]),
            ]),
            graph.pow(x, graph.integer(2)),
        ]),
        graph.div(
            z,
            graph.add([graph.integer(1), graph.pow(y, graph.integer(2))]),
        ),
    );
    let compiled = CodeGenerator::compile_fn_n(&graph, expr, &ids);
    for point in [
        [0.3, 1.1, -2.0],
        [1.7, -0.4, 0.5],
        [-2.2, 3.0, 9.0],
        [0.0, 0.0, 0.0],
    ] {
        let mut substituted = expr;
        for (&id, &value) in ids.iter().zip(&point) {
            substituted = graph.substitute(substituted, id, graph.float(value));
        }
        let reference = graph
            .evalf(substituted, &EvalContext::new(53))
            .unwrap()
            .to_f64();
        let got = compiled(&point);
        assert!(
            (got - reference).abs() < 1e-12 * reference.abs().max(1.0),
            "{point:?}: {got} vs {reference}"
        );
    }

    // The two- and one-variable wrappers use the same path.
    let f2 = CodeGenerator::compile_fn2(&graph, graph.mul([x, y]), ids[0], ids[1]);
    assert_eq!(f2(3.0, 4.0), 12.0);
    let f1 = CodeGenerator::compile_fn1(&graph, graph.function("cos", [x]), ids[0]);
    assert!((f1(0.5) - 0.5_f64.cos()).abs() < 1e-15);
}

#[test]
fn unbound_symbols_and_missing_arguments_give_nan_not_a_wrong_number() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let x_id = graph.symbols.get_or_intern("x");
    // `a` is neither the variable nor a parameter: the bytecode compiler would read it as `x`.
    let expr = graph.add([x, graph.symbol("a")]);
    let f = CodeGenerator::compile_fn1(&graph, expr, x_id);
    assert!(f(2.0).is_nan());

    let g = CodeGenerator::compile_fn_n(&graph, x, &[x_id, graph.symbols.get_or_intern("y")]);
    assert!(g(&[1.0]).is_nan(), "too few arguments");
    assert_eq!(g(&[1.0, 2.0]), 1.0);
}

#[test]
fn constants_pi_and_e_are_bound() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let x_id = graph.symbols.get_or_intern("x");
    let expr = graph.mul([graph.symbol("pi"), x]);
    let f = CodeGenerator::compile_fn1(&graph, expr, x_id);
    assert!((f(2.0) - 2.0 * std::f64::consts::PI).abs() < 1e-12);
}

#[test]
fn the_bytecode_compiler_rejects_unbound_symbols() {
    use algebra_core::vm::{VmCompiler, VmError};
    let graph = ExprGraph::new();
    let expr = graph.add([graph.symbol("x"), graph.symbol("a")]);
    // `a` was silently compiled as the variable before.
    assert!(matches!(
        VmCompiler::compile(&graph, expr, "x", &[]),
        Err(VmError::CompilationError(_))
    ));
    // Bound as a parameter, it compiles.
    assert!(VmCompiler::compile(&graph, expr, "x", &["a".to_string()]).is_ok());
}
