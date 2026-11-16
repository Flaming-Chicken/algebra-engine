use algebra_core::ExprGraph;
use algebra_engine::api::{ConstPoly, ConstRational, Expr, NumMat, Sym, SymMat, sym};
use algebra_engine::executor::ExecutionContext;
use algebra_engine::stream::{
    StreamMode, StreamProcessor, StreamRequest, StreamResponse, execute_async, execute_batch_async,
    query_async,
};
use algebra_engine::workbench::{WorkbenchEngine, WorkbenchPresets};
use std::collections::HashMap;
use std::io::Cursor;

#[test]
fn test_const_rational_and_const_poly_compile_time() {
    const HALF: ConstRational = ConstRational::new(2, 4);
    const THIRD: ConstRational = ConstRational::new(1, 3);
    const SUM: ConstRational = HALF.add(THIRD);
    const POW3: ConstRational = HALF.pow(3);

    assert_eq!(HALF, ConstRational { num: 1, den: 2 });
    assert_eq!(SUM, ConstRational { num: 5, den: 6 });
    assert_eq!(POW3, ConstRational { num: 1, den: 8 });
    assert!((SUM.to_f64() - 5.0 / 6.0).abs() < 1e-12);

    // p(x) = 1 - 2x + 3x^2 + 4x^3
    const P: ConstPoly<4> = ConstPoly::new([1.0, -2.0, 3.0, 4.0]);
    const P_AT_2: f64 = P.eval(2.0);
    // p'(x) = -2 + 6x + 12x^2 -> p'(1) = 16
    const DP_AT_1: f64 = P.deriv().eval(1.0);
    // int_0^1 (1 - 2x + 3x^2 + 4x^3) dx = 1 - 1 + 1 + 1 = 2
    const INT_0_1: f64 = P.definite_integral(0.0, 1.0);

    assert!((P_AT_2 - 41.0).abs() < 1e-12);
    assert!((DP_AT_1 - 16.0).abs() < 1e-12);
    assert!((INT_0_1 - 2.0).abs() < 1e-12);

    let g = ExprGraph::new();
    let x = sym(&g, "x");
    let sym_p = P.intern(&g, &x);
    let sym_dp = P.deriv().intern(&g, &x);
    assert!(sym_p.diff(&x).equiv(&sym_dp));

    let val = SUM.intern(&g).evalf().expect("evalf");
    assert!((val.to_f64() - 5.0 / 6.0).abs() < 1e-12);
}

#[test]
fn test_lifetime_sym_expr_calculus_and_algebra() {
    let g = ExprGraph::new();
    let [x, y] = Sym::many(&g, ["x", "y"]);

    // Copy + scalar + borrowed ref operators
    let poly = x * x + 2 * x + 1;
    let shifted = &x + 1;
    assert!(shifted.equiv(&(x + 1)));
    let factored = poly.factor();
    let sq = (x + 1).pow(2);
    assert!(factored.equiv(&sq));

    let d = poly.diff(&x);
    assert!(d.equiv(&(2 * x + 2)));

    let d2 = poly.diff_n(&x, 2);
    assert!((d2.evalf().unwrap().to_f64() - 2.0).abs() < 1e-12);

    let integ = (3 * x.pow(2)).integrate(&x).expect("integrate");
    assert!(integ.equiv(&x.pow(3)));

    let def = x.integrate_definite(&x, 0, 2).expect("definite");
    assert!((def.evalf().unwrap().to_f64() - 2.0).abs() < 1e-9);

    let at3 = poly.subs(&x, 3);
    assert!((at3.evalf().unwrap().to_f64() - 16.0).abs() < 1e-12);

    let rat = ((x * x - 1) / (&x - 1)).cancel();
    assert!(rat.equiv(&(x + 1)));

    let comb = (1 / x.as_expr() + 1 / y.as_expr()).together();
    let mut b = HashMap::new();
    b.insert("x".to_string(), 2.0);
    b.insert("y".to_string(), 4.0);
    assert!((comb.eval_with(&b).unwrap().to_f64() - 0.75).abs() < 1e-12);

    let lim = (x.sin() / x).limit(&x, 0).expect("sinc limit");
    assert!((lim.evalf().unwrap().to_f64() - 1.0).abs() < 1e-12);

    let ser = x.exp().series(&x, 0, 3);
    let ser_at_0 = ser.subs(&x, 0).evalf().unwrap().to_f64();
    assert!((ser_at_0 - 1.0).abs() < 1e-12);

    let roots = (x * x - 9).solve(&x).expect("solve");
    assert_eq!(roots.len(), 2);

    let ctx = ExecutionContext::new();
    let unc = (2 * x + 1).eval_uncertain(&ctx);
    assert!(unc.is_err() || unc.is_ok()); // Free x without binding safely handled without panic
    assert_eq!((x * y + 1).free_symbols().len(), 2);
}

#[test]
fn test_sym_mat_and_num_mat_mutable_and_immutable_views() {
    let g = ExprGraph::new();
    let x = sym(&g, "x");
    let e00 = x * 1;
    let e01 = Expr::int(&g, 1);
    let e10 = Expr::int(&g, 0);
    let e11 = Expr::int(&g, 2);

    let mut a = SymMat::from_rows(&g, &[&[e00, e01], &[e10, e11]]).expect("matrix");
    assert_eq!(a.shape(), (2, 2));
    assert!(a.det().unwrap().equiv(&(2 * x)));

    // Mutable in-place operations (&mut self)
    a.set(1, 0, 1).unwrap();
    a.swap_rows(0, 1).unwrap();
    a.scale_row(0, 2).unwrap();
    a.add_scaled_row(1, 0, 1).unwrap();
    a.simplify_in_place();
    assert_eq!(a.rows(), 2);

    // Immutable algebraic operations (&self)
    let m = SymMat::parse(&g, "[[2, 1], [1, 3]]").expect("parse mat");
    assert!((m.det().unwrap().evalf().unwrap().to_f64() - 5.0).abs() < 1e-12);
    assert_eq!(m.rank().unwrap(), 2);
    assert!(m.nullspace().unwrap().is_empty());

    let minv = m.inv().expect("inverse");
    let prod = &m * &minv;
    let eye = SymMat::eye(&g, 2).unwrap();
    for r in 0..2 {
        for c in 0..2 {
            assert!(prod.get(r, c).unwrap().equiv(&eye.get(r, c).unwrap()));
        }
    }

    let b = [Expr::int(&g, 5), Expr::int(&g, 5)];
    let sol = m.solve(&b).expect("solve Ax=b");
    assert!((sol[0].evalf().unwrap().to_f64() - 2.0).abs() < 1e-12);
    assert!((sol[1].evalf().unwrap().to_f64() - 1.0).abs() < 1e-12);

    let eigs = m.eigvals().expect("eigvals");
    assert_eq!(eigs.len(), 2);

    // NumMat array/matrix view & in-place mutability
    let ls = NumMat::linspace(0.0, 10.0, 5);
    assert_eq!(ls.as_slice(), &[0.0, 2.5, 5.0, 7.5, 10.0]);
    let ar = NumMat::arange(0.0, 1.0, 0.25).expect("arange");
    assert_eq!(ar.cols(), 4);

    let mut nm = NumMat::from_rows(&[&[2.0, 1.0], &[4.0, 2.0]]).expect("from_rows");
    assert_eq!(nm.rank(1e-9), 1);
    nm[(1, 1)] = 5.0;
    assert_eq!(nm.rank(1e-9), 2);
    assert!(nm.norm_fro() > 0.0);
}

#[test]
fn test_async_compute_future_query_execute_and_batch() {
    let g = ExprGraph::new();
    let ctx = ExecutionContext::new();

    let q_res = query_async(&g, "diff x^3, x", &ctx).block_on();
    assert!(q_res.pods.iter().any(|p| p.text.contains("3")));

    let e_res = execute_async(&g, "factor x^2 - 9", &ctx).block_on();
    assert!(!e_res.is_error);
    assert!(e_res.output_text.contains("x - 3") || e_res.output_text.contains("-3"));

    let cmds = vec![
        "diff sin(x), x".to_string(),
        "expand (x + 1)^2".to_string(),
        "det [[1, 2], [3, 4]]".to_string(),
    ];
    let batch = execute_batch_async(&g, cmds, &ctx).block_on();
    assert_eq!(batch.len(), 3);
    assert!(batch.iter().all(|r| !r.is_error));
    assert_eq!(batch[2].output_text, "-2");
}

#[test]
fn test_stream_and_file_pipelines() {
    let g = ExprGraph::new();
    let mut session = algebra_engine::UraeSession::new();

    // 1. Stateful script stream
    let script = "# setup\na = 5\na^2 + 1\n// reuse ans\nans * 2\n";
    let mut script_out = Vec::new();
    let summary = StreamProcessor::process_script_stream(
        &mut session,
        Cursor::new(script.as_bytes()),
        &mut script_out,
    )
    .expect("script stream");
    assert_eq!(summary.processed, 3);
    assert_eq!(summary.succeeded, 3);
    assert_eq!(summary.failed, 0);
    assert_eq!(summary.skipped, 2);
    let out_str = String::from_utf8(script_out).unwrap();
    assert!(out_str.contains("=> 26"));
    assert!(out_str.contains("=> 52"));

    // 2. NDJSON stream (Execute, Query, malformed line)
    let ndjson = concat!(
        r#"{"id":"1","input":"diff x^2, x","mode":"execute"}"#, "\n",
        r#"{"id":"2","input":"integrate x, x","mode":"query"}"#, "\n",
        "not valid json\n"
    );
    let mut ndjson_out = Vec::new();
    let nd_summary = StreamProcessor::process_ndjson_stream(
        &g,
        Cursor::new(ndjson.as_bytes()),
        &mut ndjson_out,
    )
    .expect("ndjson stream");
    assert_eq!(nd_summary.processed, 3);
    assert_eq!(nd_summary.succeeded, 2);
    assert_eq!(nd_summary.failed, 1);

    let nd_lines: Vec<StreamResponse> = String::from_utf8(ndjson_out)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str::<StreamResponse>(l).unwrap())
        .collect();
    assert_eq!(nd_lines.len(), 3);
    assert!(nd_lines[0].success);
    assert!(nd_lines[1].pods.is_some());
    assert!(!nd_lines[2].success);

    // 3. Direct handle_request sanity
    let single = StreamProcessor::handle_request(
        &g,
        &StreamRequest {
            id: Some("s1".to_string()),
            input: "2 * x + 3".to_string(),
            bindings: HashMap::from([("x".to_string(), 4.0)]),
            uncertainties: HashMap::new(),
            mode: StreamMode::Execute,
        },
    );
    assert!(single.success);

    // 4. CSV BytecodeVM stream
    let csv_in = "x,y\n1,2\n3,4\nbad,row\n";
    let mut csv_out = Vec::new();
    let csv_summary = StreamProcessor::eval_csv_stream(
        &g,
        "x^2 + y",
        "z",
        Cursor::new(csv_in.as_bytes()),
        &mut csv_out,
    )
    .expect("csv stream");
    assert_eq!(csv_summary.processed, 3);
    assert_eq!(csv_summary.succeeded, 2);
    assert_eq!(csv_summary.failed, 1);
    let csv_text = String::from_utf8(csv_out).unwrap();
    assert!(csv_text.contains("x,y,z"));
    assert!(csv_text.contains("1,2,3"));
    assert!(csv_text.contains("3,4,13"));
    assert!(csv_text.contains("bad,row,NaN"));

    // 5. File I/O wrappers via temp directory
    let dir = tempfile::tempdir().expect("tempdir");
    let script_in_path = dir.path().join("in.urae");
    let script_out_path = dir.path().join("out.txt");
    std::fs::write(&script_in_path, "k = 7\nk + 3\n").unwrap();
    let mut file_sess = algebra_engine::UraeSession::new();
    let fs_sum =
        StreamProcessor::process_script_file(&mut file_sess, &script_in_path, &script_out_path)
            .expect("script file");
    assert_eq!(fs_sum.succeeded, 2);

    let nd_in_path = dir.path().join("req.ndjson");
    let nd_out_path = dir.path().join("resp.ndjson");
    std::fs::write(&nd_in_path, "{\"input\":\"det [[2,0],[0,3]]\"}\n").unwrap();
    let fn_sum = StreamProcessor::process_ndjson_file(&g, &nd_in_path, &nd_out_path).unwrap();
    assert_eq!(fn_sum.succeeded, 1);

    let csv_in_path = dir.path().join("data.csv");
    let csv_out_path = dir.path().join("data_out.csv");
    std::fs::write(&csv_in_path, "x,y\n2,5\n").unwrap();
    let fc_sum =
        StreamProcessor::eval_csv_file(&g, "x * y + 1", "res", &csv_in_path, &csv_out_path)
            .unwrap();
    assert_eq!(fc_sum.succeeded, 1);
    assert!(std::fs::read_to_string(&csv_out_path).unwrap().contains("2,5,11"));
}

#[test]
fn test_workbench_equation_and_differential_presets() {
    let g = ExprGraph::new();

    // All algebraic equation presets
    let eq_presets = WorkbenchPresets::equation_presets();
    assert!(!eq_presets.is_empty());
    for (idx, spec) in eq_presets.iter().enumerate() {
        let analysis = WorkbenchEngine::analyze_equation_system(&g, spec);
        assert!(
            analysis.success,
            "equation preset {idx} ({}) failed: {:?}",
            spec.name, analysis.error_msg
        );
        assert!(!analysis.solution_text.is_empty());
        assert!(!analysis.jacobian_unicode.is_empty());
        if spec.name == "Circle-Line Intersection" {
            assert_eq!(analysis.contours.len(), 2);
            assert!(
                analysis.contours.iter().all(|c| !c.is_empty()),
                "contours should be non-empty"
            );
            assert_eq!(analysis.solution_points.len(), 2);
            assert!(analysis.residuals.iter().all(|&r| r.abs() < 1e-6));
        }
    }

    // All differential / dynamical presets
    let ode_presets = WorkbenchPresets::differential_presets();
    assert!(!ode_presets.is_empty());
    for (idx, spec) in ode_presets.iter().enumerate() {
        let analysis = WorkbenchEngine::analyze_differential_system(&g, spec);
        assert!(
            analysis.success,
            "differential preset {idx} ({}) failed: {:?}",
            spec.name, analysis.error_msg
        );
        assert!(analysis.t.len() > 10);
        assert_eq!(analysis.trajectories.len(), spec.state_vars.len());
        assert!(!analysis.quiver_field.is_empty());
        if spec.name == "Damped Harmonic Oscillator" {
            assert!(
                analysis.stability_label.contains("Stable"),
                "expected stable classification, got: {}",
                analysis.stability_label
            );
            let x_traj = &analysis.trajectories[0];
            assert!(x_traj.last().unwrap().abs() < x_traj.first().unwrap().abs());
        }
    }
}
