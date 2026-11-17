use algebra_core::interval::RealInterval;
use algebra_core::parser::{ExprParser, parse_operation};
use algebra_core::{ExprGraph, parse_value_uncertainty_and_unit};
use algebra_engine::SymbolicSolver;
use algebra_engine::UraeSession;
use algebra_engine::branch::{BranchCutFunctionKind, BranchCutManager, BranchSelection};
use algebra_engine::executor::{ExecutionContext, OperationExecutor};
use algebra_engine::numeric::{BigValue, EvalContext, NumericalEval, eval_uncertain};
use algebra_engine::simplify::Simplifier;
use std::collections::HashMap;
use std::f64::consts::PI;

#[test]
fn test_multivalued_powers_and_i_to_i() {
    // 1. All 3 cube roots of -1 via BranchCutManager::eval_power
    let mv = BranchCutManager::eval_power(
        -1.0,
        0.0,
        1.0 / 3.0,
        0.0,
        &BranchSelection::AllSheets { max_branches: 5 },
    )
    .unwrap();
    let vals = mv.all_values();
    assert_eq!(vals.len(), 3, "Expected 3 cube roots of -1, got {:?}", vals);
    for (r, im, _k) in &vals {
        // Compute z^3 and verify it equals -1 + 0i
        let re2 = r * r - im * im;
        let im2 = 2.0 * r * im;
        let re3 = re2 * r - im2 * im;
        let im3 = re2 * im + im2 * r;
        assert!((re3 + 1.0).abs() < 1e-10, "re3={} for ({}, {})", re3, r, im);
        assert!(im3.abs() < 1e-10, "im3={} for ({}, {})", im3, r, im);
    }

    // 2. Principal value of i^i = exp(-pi/2)
    let i_to_i = BranchCutManager::eval_power(0.0, 1.0, 0.0, 1.0, &BranchSelection::Principal)
        .unwrap()
        .primary_value();
    let expected = (-PI / 2.0).exp();
    assert!((i_to_i.0 - expected).abs() < 1e-12);
    assert!(i_to_i.1.abs() < 1e-12);

    // 3. evalf of i^i
    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let expr = parser.parse("i^i").unwrap();
    let val = graph.evalf(expr, &EvalContext::default()).unwrap();
    let (re, im) = val.as_complex();
    assert!((re - expected).abs() < 1e-10);
    assert!(im.abs() < 1e-10);
}

#[test]
fn test_lambert_w_all_real_and_complex_sheets() {
    let z = -0.2;
    // W_0(-0.2) and W_{-1}(-0.2) satisfy w * exp(w) == -0.2
    let w0 = BranchCutManager::eval(
        BranchCutFunctionKind::LambertW,
        z,
        0.0,
        &BranchSelection::Sheet(0),
    )
    .unwrap()
    .primary_value();
    let wm1 = BranchCutManager::eval(
        BranchCutFunctionKind::LambertW,
        z,
        0.0,
        &BranchSelection::Sheet(-1),
    )
    .unwrap()
    .primary_value();
    let w1 = BranchCutManager::eval(
        BranchCutFunctionKind::LambertW,
        z,
        0.0,
        &BranchSelection::Sheet(1),
    )
    .unwrap()
    .primary_value();

    assert!(w0.1.abs() < 1e-12);
    assert!(wm1.1.abs() < 1e-12);
    assert!(wm1.0 < -1.0 && w0.0 > -1.0 && w0.0 < 0.0);
    assert!(
        w1.1.abs() > 1.0,
        "Sheet k=1 of LambertW(-0.2) should be non-real"
    );

    for (wr, wi) in [w0, wm1, w1] {
        let exp_r = wr.exp() * wi.cos();
        let exp_i = wr.exp() * wi.sin();
        let prod_r = wr * exp_r - wi * exp_i;
        let prod_i = wr * exp_i + wi * exp_r;
        assert!((prod_r - z).abs() < 1e-10, "w*e^w re mismatch: {}", prod_r);
        assert!(prod_i.abs() < 1e-10, "w*e^w im mismatch: {}", prod_i);
    }

    // Indexed 2-arg functions via evalf
    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let ctx = EvalContext::default();

    let e_wm1 = parser.parse("lambert_w(-1, -0.2)").unwrap();
    let v_wm1 = graph.evalf(e_wm1, &ctx).unwrap().to_f64();
    assert!((v_wm1 - wm1.0).abs() < 1e-10);

    let e_ln1 = parser.parse("ln(-1, 1)").unwrap();
    let (lr, li) = graph.evalf(e_ln1, &ctx).unwrap().as_complex();
    assert!(lr.abs() < 1e-10);
    assert!((li - 3.0 * PI).abs() < 1e-10);

    let e_root = parser.parse("root(-8, 3, 1)").unwrap();
    let (rr, ri) = graph.evalf(e_root, &ctx).unwrap().as_complex();
    assert!((rr + 2.0).abs() < 1e-10);
    assert!(ri.abs() < 1e-10);
}

#[test]
fn test_complex_evalf_div_and_inverse_trig() {
    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let ctx = EvalContext::default();

    // (1 + i) / (1 - i) == i
    let div_expr = parser.parse("(1 + i) / (1 - i)").unwrap();
    let (re, im) = graph.evalf(div_expr, &ctx).unwrap().as_complex();
    assert!(re.abs() < 1e-12);
    assert!((im - 1.0).abs() < 1e-12);

    // Out-of-real-domain asin(2) should return complex result with sin(asin(2)) = 2
    let asin_expr = parser.parse("asin(2)").unwrap();
    let (ar, ai) = graph.evalf(asin_expr, &ctx).unwrap().as_complex();
    assert!((ar - PI / 2.0).abs() < 1e-10);
    assert!(ai.abs() > 1.0);
    // sin(ar + i*ai) = sin(ar)*cosh(ai) + i*cos(ar)*sinh(ai)
    let sin_re = ar.sin() * ai.cosh();
    let sin_im = ar.cos() * ai.sinh();
    assert!((sin_re - 2.0).abs() < 1e-10);
    assert!(sin_im.abs() < 1e-10);
}

#[test]
fn test_branch_cut_safe_simplification_and_multibranch_solver() {
    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);

    // ln(exp(2*i*pi)) must NOT simplify to 2*i*pi
    let expr = parser.parse("ln(exp(2*i*pi))").unwrap();
    let simp = Simplifier::simplify(&graph, expr).unwrap();
    let ctx = EvalContext::default();
    let val = graph.evalf(simp, &ctx).unwrap();
    match val {
        BigValue::Real(r) => assert!(r.abs() < 1e-10, "Expected ~0, got {}", r),
        BigValue::Complex(r, im) => {
            assert!(
                r.abs() < 1e-10 && im.abs() < 1e-10,
                "Expected ~0, got {}+{}i",
                r,
                im
            );
        }
    }

    // Solve exp(x) = -1 -> i*pi (complex branch)
    let x_sym = graph.symbols.get_or_intern("x");
    let eq_exp = parser.parse("exp(x) = -1").unwrap();
    let sols_exp = graph.solveset(eq_exp, x_sym).unwrap();
    assert!(
        !sols_exp.is_empty(),
        "exp(x) = -1 should yield complex solution i*pi"
    );
    let s_val = graph.evalf(sols_exp[0], &ctx).unwrap().as_complex();
    assert!(s_val.0.abs() < 1e-10);
    assert!((s_val.1 - PI).abs() < 1e-10);

    // Solve sin(x) = 1/2 -> both principal and pi - principal branches
    let eq_sin = parser.parse("sin(x) = 1/2").unwrap();
    let sols_sin = graph.solveset(eq_sin, x_sym).unwrap();
    assert!(
        sols_sin.len() >= 2,
        "sin(x) = 1/2 should return >= 2 principal period solutions"
    );
    let sin_vals: Vec<f64> = sols_sin
        .iter()
        .map(|&s| graph.evalf(s, &ctx).unwrap().to_f64())
        .collect();
    assert!(sin_vals.iter().any(|&v| (v - PI / 6.0).abs() < 1e-10));
    assert!(sin_vals.iter().any(|&v| (v - 5.0 * PI / 6.0).abs() < 1e-10));

    // Solve x*exp(x) = -0.2 -> both W_0(-0.2) and W_{-1}(-0.2)
    let eq_w = parser.parse("x * exp(x) = -0.2").unwrap();
    let sols_w = graph.solveset(eq_w, x_sym).unwrap();
    assert!(
        sols_w.len() >= 2,
        "x*exp(x) = -0.2 should return both real Lambert-W branches"
    );
}

#[test]
fn test_branch_commands_in_executor() {
    let graph = ExprGraph::new();
    let ctx = ExecutionContext::default();

    let op_cut = parse_operation(r#"branch_cut("sqrt", sheet = 1, cut = "(-infinity, 0]")"#);
    let res_cut = OperationExecutor::execute(&graph, &op_cut, &ctx);
    assert!(
        !res_cut.is_error,
        "branch_cut error: {:?}",
        res_cut.error_msg
    );
    assert!(
        res_cut.output_text.contains("sqrt") && res_cut.output_text.contains("k=1"),
        "{}",
        res_cut.output_text
    );

    let op_branches = parse_operation("branches (-1)^(1/3)");
    let res_branches = OperationExecutor::execute(&graph, &op_branches, &ctx);
    assert!(
        !res_branches.is_error,
        "branches error: {:?}",
        res_branches.error_msg
    );
    assert!(
        res_branches.output_text.contains("k=0"),
        "{}",
        res_branches.output_text
    );
    assert!(
        res_branches.output_text.contains("k=1"),
        "{}",
        res_branches.output_text
    );
    assert!(
        res_branches.output_text.contains("k=2"),
        "{}",
        res_branches.output_text
    );
}

#[test]
fn test_interval_arithmetic_and_correlated_gum_uncertainty() {
    // 1. Elementary RealInterval bounds
    let iv = RealInterval::from_center_uncertainty(1.0, 0.2);
    let exp_iv = iv.exp();
    assert!(exp_iv.contains(1.0_f64.exp()));
    let sin_iv = RealInterval::new(0.0, PI).sin();
    assert!((sin_iv.sup - 1.0).abs() < 1e-12 && sin_iv.inf >= -1e-12);

    // 2. Correlated cancellation: a - a == 0 +/- 0
    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let mut bindings = HashMap::new();
    bindings.insert("a".to_string(), 5.0);
    bindings.insert("b".to_string(), 10.0);
    let mut uncertainties = HashMap::new();
    uncertainties.insert("a".to_string(), 0.3);
    uncertainties.insert("b".to_string(), 0.4);
    let ctx = EvalContext::with_bindings(53, bindings).with_uncertainties(uncertainties);

    let diff_expr = parser.parse("a - a").unwrap();
    let uv_diff = eval_uncertain(&graph, diff_expr, &ctx).unwrap();
    assert!(uv_diff.mean.abs() < 1e-12);
    assert!(
        uv_diff.std_dev < 1e-12,
        "Correlated a - a must cancel uncertainty, got {}",
        uv_diff.std_dev
    );

    // 3. Independent quadrature: a + b -> 15 +/- 0.5
    let sum_expr = parser.parse("a + b").unwrap();
    let uv_sum = eval_uncertain(&graph, sum_expr, &ctx).unwrap();
    assert!((uv_sum.mean - 15.0).abs() < 1e-12);
    assert!((uv_sum.std_dev - 0.5).abs() < 1e-12);

    // 4. Nonlinear propagation: d/da (a^2) * 0.3 = 2 * 5 * 0.3 = 3.0
    let sq_expr = parser.parse("a^2").unwrap();
    let uv_sq = eval_uncertain(&graph, sq_expr, &ctx).unwrap();
    assert!((uv_sq.mean - 25.0).abs() < 1e-12);
    assert!((uv_sq.std_dev - 3.0).abs() < 1e-12);
    assert!(uv_sq.interval.contains(25.0));

    // 5. Inline uncertainty expression: (10 +/- 0.2) * 2 -> 20 +/- 0.4
    let inline_expr = parser.parse("(10 +/- 0.2) * 2").unwrap();
    let uv_inline = eval_uncertain(&graph, inline_expr, &EvalContext::default()).unwrap();
    assert!((uv_inline.mean - 20.0).abs() < 1e-12);
    assert!((uv_inline.std_dev - 0.4).abs() < 1e-12);
}

#[test]
fn test_uncertainty_parsing_session_cli_and_notebook_e2e() {
    // 1. Shared parser helper
    let (v, s, u) = parse_value_uncertainty_and_unit("5.0 +/- 0.1 [m/s]").unwrap();
    assert!((v - 5.0).abs() < 1e-12);
    assert!((s.unwrap() - 0.1).abs() < 1e-12);
    assert_eq!(u.as_deref(), Some("m/s"));

    // 2. Stateful UraeSession
    let mut session = UraeSession::new();
    let r1 = session.execute_line("a = 3.0 +/- 0.3 [m]");
    assert!(!r1.is_error, "{:?}", r1.error_msg);
    let r2 = session.execute_line("b = 4.0 ± 0.4 [m]");
    assert!(!r2.is_error, "{:?}", r2.error_msg);
    let r3 = session.execute_line("eval sqrt(a^2 + b^2)");
    assert!(!r3.is_error, "{:?}", r3.error_msg);
    // hyp = 5, d(hyp)/da = 3/5, d(hyp)/db = 4/5 -> sqrt((0.6*0.3)^2 + (0.8*0.4)^2) = sqrt(0.1348) ≈ 0.36715
    assert!(
        r3.output_text.contains("5") && r3.output_text.contains('±'),
        "{}",
        r3.output_text
    );

    // 3. CLI JSON API with uncertainties & branches
    let graph = ExprGraph::new();
    let req_json = r#"{
        "input": "a + b",
        "bindings": {"a": 10.0, "b": 5.0},
        "uncertainties": {"a": 0.3, "b": 0.4},
        "eval_numerical": true
    }"#;
    let resp: urae_cli::UraeJsonResponse =
        serde_json::from_str(&urae_cli::process_json_request(&graph, req_json)).unwrap();
    assert!(resp.success);
    assert!((resp.eval_result.unwrap() - 15.0).abs() < 1e-12);
    assert!((resp.eval_uncertainty.unwrap() - 0.5).abs() < 1e-12);
    assert!(resp.eval_interval.is_some());

    let branch_json = r#"{"input": "branches (-1)^(1/3)"}"#;
    let branch_resp: urae_cli::UraeJsonResponse =
        serde_json::from_str(&urae_cli::process_json_request(&graph, branch_json)).unwrap();
    assert!(branch_resp.success);
    assert!(
        branch_resp
            .branch_values
            .as_ref()
            .map(|v| v.len())
            .unwrap_or(0)
            >= 3
    );

    // 4. NotebookState E2E with Parameter uncertainty & branch command
    let mut nb = urae_notebook::notebook::NotebookState::default();
    nb.session.raw_document_text = [
        "a: Parameter = 3.0 +/- 0.3 [m]",
        "b = 4.0 +/- 0.4 [m]",
        "a + b",
        "branches (-1)^(1/3)",
    ]
    .join("\n");
    nb.evaluate_all();

    assert_eq!(nb.parsed_lines.len(), 4);
    assert_eq!(nb.session.symbol_metadata["a"].uncertainty, Some(0.3));
    assert_eq!(nb.session.symbol_metadata["b"].uncertainty, Some(0.4));
    assert!((nb.parsed_lines[2].linearized_estimate.unwrap() - 7.0).abs() < 1e-12);
    assert!(
        nb.parsed_lines[2]
            .domain_info
            .as_deref()
            .unwrap_or("")
            .contains("± 0.5")
    );
    assert!(
        nb.parsed_lines[3].output_unicode.contains("k=0"),
        "{}",
        nb.parsed_lines[3].output_unicode
    );
}
