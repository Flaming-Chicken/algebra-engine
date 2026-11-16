use algebra_core::{ExprGraph, parse_operation};
use algebra_engine::executor::{ExecutionContext, OperationExecutor};
use algebra_engine::session::UraeSession;
use urae::prelude::{QueryEngine, QueryResponse, ResultPod};
use urae_cli::json_api::process_json_request;

#[test]
fn test_session_functions_aliases_ans_exact_and_assumptions() {
    let mut session = UraeSession::new();

    // 1. Multi-arg & nested-paren user functions without identifier corruption
    let def_f = session.execute_line("f(x, y) = x^2 + y^2");
    assert!(!def_f.is_error, "f def error: {}", def_f.output_text);
    let def_g = session.execute_line("g(x) = exp(x) + x");
    assert!(!def_g.is_error, "g def error: {}", def_g.output_text);

    let f_val = session.eval("f(3, 4)").expect("f(3, 4) should evaluate");
    assert_eq!(f_val, "25");

    // `g(2)` must expand `exp(2) + 2`, NOT corrupt `exp` into `e2p`
    let g_res = session.execute_line("g(0)");
    assert!(!g_res.is_error, "g(0) failed: {}", g_res.output_text);
    assert_eq!(
        g_res.output_text, "1",
        "exp(0) + 0 = 1, got {}",
        g_res.output_text
    );

    // Nested parentheses inside function args: `f(sin(t), cos(t))` -> `sin(t)^2 + cos(t)^2` -> `1`
    let nested = session.execute_line("simplify f(sin(t), cos(t))");
    assert!(!nested.is_error, "nested failed: {}", nested.output_text);
    assert!(
        nested.output_text == "1" || nested.output_text.contains("sin"),
        "unexpected nested trig result: {}",
        nested.output_text
    );

    // 2. `ans` chaining
    let d = session.diff("x^3", "x").expect("diff x^3");
    assert!(d.contains("3") && d.contains("x"), "got {d}");
    let integrated = session.integrate("ans", "x").expect("integrate ans");
    assert!(
        integrated.contains("x^3") || integrated.contains("x³"),
        "got {integrated}"
    );

    // 3. Computed scalar assignments, symbolic aliases, and exact rationals
    let a = session.eval("a = 1/3").expect("rational assign");
    assert!(a.contains("1/3") || a.contains("0.333"), "got {a}");
    let sum = session.eval("a + 1/6").expect("exact rational sum");
    assert_eq!(sum, "1/2", "expected exact 1/2, got {sum}");

    session.execute_line("r = 3");
    let area = session
        .eval("area = pi * r^2")
        .expect("computed assignment");
    assert!(area.contains("28.274") || area.contains("9"), "got {area}");

    session.execute_line("p = (x + 1)^2");
    let p_exp = session.eval("expand p").expect("expand alias p");
    assert!(p_exp.contains("x^2") || p_exp.contains("x²"), "got {p_exp}");

    // 4. Persistent `assume` filtering & `clear`
    let ax = session.assume("x > 0");
    assert!(!ax.is_error, "assume x > 0 failed: {}", ax.output_text);
    let pos_roots = session.solve("x^2 - 9 = 0", "x").expect("solve positive");
    assert!(
        pos_roots.contains('3') && !pos_roots.contains("-3"),
        "expected only +3 under x > 0, got {pos_roots}"
    );

    let ay = session.assume("y <= 0");
    assert!(!ay.is_error, "assume y <= 0 failed: {}", ay.output_text);
    let nonpos_roots = session.solve("y^2 - 4 = 0", "y").expect("solve y <= 0");
    assert!(
        nonpos_roots.contains("-2") && !nonpos_roots.contains("[2"),
        "expected -2 under y <= 0, got {nonpos_roots}"
    );

    // `eval_latex` ergonomic helper
    let tex = session.eval_latex("diff sin(x^2), x").expect("eval_latex");
    assert!(
        tex.contains("cos") && tex.contains("\\cdot"),
        "expected LaTeX cos derivative, got {tex}"
    );

    // `clear` resets bindings, assumptions, aliases, and functions
    session.execute_line("clear");
    let cleared_roots = session
        .solve("x^2 - 9 = 0", "x")
        .expect("solve after clear");
    assert!(
        cleared_roots.contains('3') && cleared_roots.contains("-3"),
        "expected both 3 and -3 after clear, got {cleared_roots}"
    );
}

#[test]
fn test_unified_json_api_latex_and_query_pods() {
    let graph = ExprGraph::new();

    for (cmd, expected_tex_fragment) in [
        ("det [[1, 2], [3, 4]]", "-2"),
        ("inv [[1, 2], [3, 4]]", "\\begin{pmatrix}"),
        ("solve x^2 - 4 = 0, x", "\\in \\left\\{"),
        ("integrate x^2, x", "\\frac"),
        ("diff sin(x^2), x", "\\operatorname{cos}"),
    ] {
        let req = serde_json::json!({ "input": cmd }).to_string();
        let resp_str = process_json_request(&graph, &req);
        let resp: serde_json::Value = serde_json::from_str(&resp_str).expect("valid json response");
        assert_eq!(
            resp["success"].as_bool(),
            Some(true),
            "cmd failed: {cmd} -> {resp}"
        );
        let latex = resp["output_latex"].as_str().unwrap_or("");
        assert!(
            !latex.starts_with("\\text{"),
            "cmd `{cmd}` produced fallback \\text{{...}} LaTeX: {latex}"
        );
        assert!(
            latex.contains(expected_tex_fragment),
            "cmd `{cmd}` LaTeX `{latex}` missing `{expected_tex_fragment}`"
        );
    }

    // Structured QueryEngine non-`x` free variable companion, integrate steps, and context bindings
    let mut ctx = ExecutionContext::new();
    ctx.bindings.insert("a".to_string(), 2.0);
    let resp: QueryResponse = QueryEngine::query(&graph, "t^2 - 9", &ctx);
    assert!(
        resp.pods
            .iter()
            .any(|p: &ResultPod| p.title == "Calculus companion"),
        "expected Calculus companion for t^2 - 9, pods: {:?}",
        resp.pods.iter().map(|p| &p.title).collect::<Vec<_>>()
    );
    assert!(
        resp.pods
            .iter()
            .any(|p| p.title == "Roots / Zeros" && p.text.contains("t =")),
        "expected Roots / Zeros in t for t^2 - 9"
    );

    let int_resp = QueryEngine::query(&graph, "integrate x*sin(x), x", &ctx);
    let step_pod = int_resp
        .pods
        .iter()
        .find(|p| p.title == "Step-by-step solution")
        .expect("expected Step-by-step solution pod for integrate x*sin(x)");
    assert!(
        step_pod.latex.contains("\\begin{aligned}"),
        "expected aligned LaTeX steps, got {}",
        step_pod.latex
    );

    // Context binding should enable numeric approximation pod for `a * 5` -> 10.0
    let bound_resp = QueryEngine::query(&graph, "a * 5", &ctx);
    assert!(
        bound_resp
            .pods
            .iter()
            .any(|p| p.title == "Numerical approximation" && p.text.starts_with("10")),
        "expected Numerical approximation = 10 using binding a=2, pods: {:?}",
        bound_resp.pods
    );
}

#[test]
fn test_matrix_depth_and_equation_systems() {
    let graph = ExprGraph::new();
    let ctx = ExecutionContext::new();

    // Eigenvalues 2x2 numeric & symbolic diagonal
    let eig_op = parse_operation("eigenvalues [[1, 2], [2, 1]]");
    let eig_res = OperationExecutor::execute(&graph, &eig_op, &ctx);
    assert!(!eig_res.is_error, "eig err: {}", eig_res.output_text);
    assert!(
        eig_res.output_text.contains('3') && eig_res.output_text.contains("-1"),
        "expected eigenvalues 3 and -1, got {}",
        eig_res.output_text
    );

    let eig_sym_op = parse_operation("eigenvalues [[a, 0], [0, b]]");
    let eig_sym_res = OperationExecutor::execute(&graph, &eig_sym_op, &ctx);
    assert!(
        !eig_sym_res.is_error,
        "sym eig err: {}",
        eig_sym_res.output_text
    );
    assert!(
        eig_sym_res.output_text.contains('a') && eig_sym_res.output_text.contains('b'),
        "expected symbolic eigenvalues [a, b], got {}",
        eig_sym_res.output_text
    );

    // RREF, Rank, Nullspace
    let rref_op = parse_operation("rref [[1, 2], [3, 4]]");
    let rref_res = OperationExecutor::execute(&graph, &rref_op, &ctx);
    assert!(!rref_res.is_error, "rref err: {}", rref_res.output_text);
    assert_eq!(rref_res.metadata.get("rank").map(String::as_str), Some("2"));

    let rank_op = parse_operation("rank [[1, 2, 3], [2, 4, 6]]");
    let rank_res = OperationExecutor::execute(&graph, &rank_op, &ctx);
    assert!(!rank_res.is_error, "rank err: {}", rank_res.output_text);
    assert_eq!(rank_res.output_text.trim(), "1");

    let ns_op = parse_operation("nullspace [[1, 2], [2, 4]]");
    let ns_res = OperationExecutor::execute(&graph, &ns_op, &ctx);
    assert!(!ns_res.is_error, "nullspace err: {}", ns_res.output_text);
    assert_eq!(
        ns_res.metadata.get("nullity").map(String::as_str),
        Some("1")
    );
    assert!(
        ns_res.output_text.contains("-2") && ns_res.output_text.contains('1'),
        "expected nullspace vector [-2, 1], got {}",
        ns_res.output_text
    );

    // Linear system: 2x + y = 5, x - y = 1 => x = 2, y = 1
    let sys_op = parse_operation("solve [2*x + y = 5, x - y = 1], [x, y]");
    let sys_res = OperationExecutor::execute(&graph, &sys_op, &ctx);
    assert!(!sys_res.is_error, "sys err: {}", sys_res.output_text);
    assert!(
        sys_res.output_text.contains("x = 2") && sys_res.output_text.contains("y = 1"),
        "expected x = 2, y = 1, got {}",
        sys_res.output_text
    );

    // Inconsistent linear system: x + y = 1, x + y = 2 => empty
    let inc_op = parse_operation("solve [x + y = 1, x + y = 2], [x, y]");
    let inc_res = OperationExecutor::execute(&graph, &inc_op, &ctx);
    assert!(
        !inc_res.is_error,
        "inconsistent system shouldn't error/panic: {}",
        inc_res.output_text
    );
    assert_eq!(
        inc_res.metadata.get("solution_count").map(String::as_str),
        Some("0")
    );

    // Symbolic linear system: a*x + y = 1, x - y = 0 => x = 1/(a+1), y = 1/(a+1)
    let sym_sys_op = parse_operation("solve [a*x + y = 1, x - y = 0], [x, y]");
    let sym_sys_res = OperationExecutor::execute(&graph, &sym_sys_op, &ctx);
    assert!(
        !sym_sys_res.is_error,
        "symbolic sys err: {}",
        sym_sys_res.output_text
    );
    assert!(
        sym_sys_res.output_text.contains('a') && sym_sys_res.output_text.contains("x ="),
        "expected parametric symbolic solution in `a`, got {}",
        sym_sys_res.output_text
    );

    // Nonlinear polynomial system: x^2 + y^2 = 5, x - y = 1 => (2, 1) and (-1, -2)
    let nl_op = parse_operation("solve [x^2 + y^2 = 5, x - y = 1], [x, y]");
    let nl_res = OperationExecutor::execute(&graph, &nl_op, &ctx);
    assert!(
        !nl_res.is_error,
        "nonlinear sys err: {}",
        nl_res.output_text
    );
    assert!(
        nl_res.output_text.contains("x = 2")
            && nl_res.output_text.contains("y = 1")
            && nl_res.output_text.contains("x = -1")
            && nl_res.output_text.contains("y = -2"),
        "expected (2, 1) and (-1, -2), got {}",
        nl_res.output_text
    );
}

#[test]
fn test_adversarial_malformed_inputs_never_panic() {
    let mut session = UraeSession::new();
    let graph = ExprGraph::new();
    let ctx = ExecutionContext::new();

    let adversarial = [
        "",
        "   ",
        "((((",
        "[[]]",
        "det [[1,2],[3]]",
        "inv [[1,2],[2,4]]",
        "rref [bad matrix]",
        "solve [",
        "eval 1/0",
        "limit 1/0, x, 0",
        "??? @@@ !!!",
        "f(x) = ",
        "solve []",
    ];

    for input in adversarial {
        let s_res = session.execute_line(input);
        let _ = (&s_res.output_text, &s_res.output_latex, s_res.is_error);

        let w_res = QueryEngine::query(&graph, input, &ctx);
        assert!(!w_res.intent.is_empty(), "empty intent for `{input}`");

        let json_req = serde_json::json!({ "input": input }).to_string();
        let json_out = process_json_request(&graph, &json_req);
        let _: serde_json::Value =
            serde_json::from_str(&json_out).expect("JSON API must always return valid JSON");
    }

    // Explicitly assert singular inverse returns error rather than 1/0 garbage
    let sing = session.execute_line("inv [[1, 2], [2, 4]]");
    assert!(
        sing.is_error,
        "singular inverse should return clean error, got {}",
        sing.output_text
    );
}
