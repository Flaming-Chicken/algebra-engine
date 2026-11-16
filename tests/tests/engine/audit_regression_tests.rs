//! Comprehensive regression and property-based checks covering the
//! URAE Nov 2026 audit fixes across formatting, calculus, solving, factoring,
//! limits, simplification, exact eval, number theory, parser, and JSON API.

use algebra_core::format::{Formatter, LatexFormatter, UnicodeFormatter};
use algebra_core::parser::{parse_operation, ExprParser};
use algebra_core::ExprGraph;
use algebra_engine::calculus::SymbolicCalculus;
use algebra_engine::executor::{ExecutionContext, OperationExecutor};
use algebra_engine::numbertheory::{
    big_prime_factors, extended_gcd_big, integer_prime_factors, is_prime, is_prime_big,
};
use algebra_engine::numeric::{EvalContext, NumericalEval};
use algebra_engine::simplify::{AlgebraicTransformations, SymbolicSimplifier};
use algebra_engine::solver::SymbolicSolver;
use num_bigint::{BigInt, BigUint};
use serde::Deserialize;
use std::collections::HashMap;
use urae_cli::json_api::{process_json_request, UraeJsonResponse};

#[derive(Deserialize)]
struct AuditExactItem {
    expr: String,
    expected: String,
}

#[derive(Deserialize)]
struct AuditBigFactorItem {
    n: String,
    factors: Vec<String>,
}

#[derive(Deserialize)]
struct AuditGcdItem {
    a: String,
    b: String,
    gcd: String,
}

#[derive(Deserialize)]
struct AuditNumberTheory {
    primes: Vec<u64>,
    big_primes: Vec<String>,
    composites: Vec<u64>,
    big_factorizations: Vec<AuditBigFactorItem>,
    gcds: Vec<AuditGcdItem>,
}

#[derive(Deserialize)]
struct AuditDetItem {
    matrix: String,
    expected: String,
}

#[derive(Deserialize)]
struct AuditLimitItem {
    expr: String,
    var: String,
    point: String,
    expected: f64,
}

#[derive(Deserialize)]
struct AuditDefIntItem {
    expr: String,
    var: String,
    lower: String,
    upper: String,
    expected: f64,
}

#[derive(Deserialize)]
struct AuditOracles {
    exact_arithmetic: Vec<AuditExactItem>,
    number_theory: AuditNumberTheory,
    determinants: Vec<AuditDetItem>,
    limits: Vec<AuditLimitItem>,
    definite_integrals: Vec<AuditDefIntItem>,
}

#[derive(Deserialize)]
struct AuditRootFixtures {
    audit_oracles: AuditOracles,
}

fn eval_at(graph: &ExprGraph, expr: algebra_core::ExprId, var: &str, val: f64) -> f64 {
    let mut b = HashMap::new();
    b.insert(var.to_string(), val);
    let ctx = EvalContext::with_bindings(15, b);
    graph.evalf(expr, &ctx).unwrap().to_f64()
}

fn run_cli(cmd: &str) -> algebra_engine::executor::OperationResult {
    let graph = ExprGraph::new();
    let ctx = ExecutionContext::new();
    let op = parse_operation(cmd);
    OperationExecutor::execute(&graph, &op, &ctx)
}

#[test]
fn test_audit_oracles_from_json() {
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/reference_oracles.json"
    ))
    .expect("reference_oracles.json");
    let fixtures: AuditRootFixtures = serde_json::from_str(&raw).expect("parse audit_oracles");
    let oracles = fixtures.audit_oracles;

    for item in &oracles.exact_arithmetic {
        let res = run_cli(&format!("eval {}", item.expr));
        assert!(!res.is_error, "eval {} failed: {:?}", item.expr, res.error_msg);
        assert_eq!(res.output_text.trim(), item.expected, "eval {}", item.expr);
    }

    for &p in &oracles.number_theory.primes {
        assert!(is_prime(p), "{} should be prime", p);
    }
    for s in &oracles.number_theory.big_primes {
        let b: BigUint = s.parse().unwrap();
        assert!(is_prime_big(&b), "{} should be big prime", s);
    }
    for &c in &oracles.number_theory.composites {
        assert!(!is_prime(c), "{} should be composite", c);
    }
    for bf in &oracles.number_theory.big_factorizations {
        let n: BigUint = bf.n.parse().unwrap();
        let fac: Vec<String> = big_prime_factors(&n)
            .into_iter()
            .map(|(p, _)| p.to_string())
            .collect();
        assert_eq!(fac, bf.factors, "factorization of {}", bf.n);
    }
    for g in &oracles.number_theory.gcds {
        let a: BigInt = g.a.parse().unwrap();
        let b: BigInt = g.b.parse().unwrap();
        let exp: BigInt = g.gcd.parse().unwrap();
        let (got, x, y) = extended_gcd_big(&a, &b);
        assert_eq!(got, exp);
        assert_eq!(&a * x + &b * y, got);
    }

    for det_item in &oracles.determinants {
        let res = run_cli(&format!("det {}", det_item.matrix));
        assert!(!res.is_error, "det {} failed: {:?}", det_item.matrix, res.error_msg);
        assert_eq!(res.output_text.trim(), det_item.expected);
    }

    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    for lim in &oracles.limits {
        let e = parser.parse(&lim.expr).unwrap();
        let p = parser.parse(&lim.point).unwrap();
        let sym = graph.symbols.get_or_intern(&lim.var);
        let ans = graph.limit(e, sym, p).unwrap();
        let val = graph.evalf(ans, &EvalContext::default()).unwrap().to_f64();
        assert!((val - lim.expected).abs() < 1e-9, "lim {} = {}, expected {}", lim.expr, val, lim.expected);
    }

    for di in &oracles.definite_integrals {
        let e = parser.parse(&di.expr).unwrap();
        let l = parser.parse(&di.lower).unwrap();
        let u = parser.parse(&di.upper).unwrap();
        let sym = graph.symbols.get_or_intern(&di.var);
        let ans = graph.integrate_definite(e, sym, l, u).unwrap();
        let val = graph.evalf(ans, &EvalContext::default()).unwrap().to_f64();
        assert!((val - di.expected).abs() < 1e-9, "int {} = {}, expected {}", di.expr, val, di.expected);
    }
}

#[test]
fn test_audit_printer_parenthesization_and_missing_symbol() {
    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let uni = UnicodeFormatter;
    let tex = LatexFormatter;

    // 1. Power of sum / difference must parenthesize base
    let e1 = parser.parse("(x + 1)^5").unwrap();
    let s1 = uni.format(&graph, e1).unwrap();
    assert!(s1.starts_with('(') && s1.contains(")^5"), "got {}", s1);

    // 2. Product of differences must parenthesize factors and format subtraction cleanly
    let e2 = parser.parse("(x - 3)*(x - 2)").unwrap();
    let s2 = uni.format(&graph, e2).unwrap();
    assert!(!s2.contains("+ -"), "unexpected '+ -' in {}", s2);
    assert!(s2.contains("(x - 3)") || s2.contains("(x − 3)"), "got {}", s2);

    // 3. Nested power must parenthesize base: (x^a)^b
    let x = graph.symbol("x");
    let a = graph.symbol("a");
    let b = graph.symbol("b");
    let xa = graph.pow(x, a);
    let xab = graph.pow(xa, b);
    let sxab = uni.format(&graph, xab).unwrap();
    assert!(sxab.starts_with("(x^a)^b"), "got {}", sxab);

    // 4. Fractional exponent: 2^(1/2)
    let half = graph.div(graph.integer(1), graph.integer(2));
    let root2 = graph.pow(graph.integer(2), half);
    assert_eq!(uni.format(&graph, root2).unwrap(), "2^(1/2)");

    // 5. (a + b)/(c - d)^2 parenthesizes denominator difference inside power
    let e5 = parser.parse("(a + b)/(c - d)^2").unwrap();
    let s5_uni = uni.format(&graph, e5).unwrap();
    let s5_tex = tex.format(&graph, e5).unwrap();
    assert!(!s5_uni.contains("+ -") && s5_uni.contains("(c"), "got {}", s5_uni);
    assert!(s5_tex.contains("\\frac"), "got {}", s5_tex);
}

#[test]
fn test_audit_calculus_diff_and_risch_differential_oracle() {
    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let x_sym = graph.symbols.get_or_intern("x");

    // Generalized power rule: d/dx(2^x), d/dx(a^x), d/dx(x^x)
    for (expr_s, test_x, expected_deriv) in [
        ("2^x", 1.5, 2.0f64.powf(1.5) * 2.0f64.ln()),
        ("x^x", 2.0, 4.0 * (2.0f64.ln() + 1.0)),
        ("log(x, 2)", 4.0, 1.0 / (4.0 * 2.0f64.ln())),
    ] {
        let e = parser.parse(expr_s).unwrap();
        let d = graph.diff(e, x_sym);
        let got = eval_at(&graph, d, "x", test_x);
        assert!(
            (got - expected_deriv).abs() < 1e-8,
            "diff({}) at {} = {}, expected {}",
            expr_s,
            test_x,
            got,
            expected_deriv
        );
    }

    // Symbolic differentiation vs integration fundamental theorem check: d/dx int(f) == f
    let test_points = [0.35, 0.72, 1.25];
    for expr_s in [
        "1/x",
        "x/(x^2 + 1)",
        "1/(x^2 + 2*x + 5)",
        "1/(x*(x + 1))",
        "log(x)",
        "sin(x)^2",
        "cos(x)^3",
        "sin(x)*cos(x)",
        "sqrt(1 - x^2)",
        "1/sqrt(1 - x^2)",
    ] {
        let e = parser.parse(expr_s).unwrap();
        let integral = graph
            .integrate(e, x_sym)
            .unwrap_or_else(|err| panic!("integrate({expr_s}) failed: {err}"));
        let back = graph.diff(integral, x_sym);
        for &pt in &test_points {
            if expr_s.contains("1 - x^2") && pt >= 1.0 {
                continue;
            }
            let expected = eval_at(&graph, e, "x", pt);
            let got = eval_at(&graph, back, "x", pt);
            assert!(
                (got - expected).abs() < 1e-6,
                "FTC failed for {} at x={}: d/dx(int)={}, orig={}",
                expr_s,
                pt,
                got,
                expected
            );
        }
    }

    // Divergent integral 1/x over [-1, 1] must error cleanly
    let inv_x = parser.parse("1/x").unwrap();
    let m1 = parser.parse("-1").unwrap();
    let p1 = parser.parse("1").unwrap();
    assert!(
        graph.integrate_definite(inv_x, x_sym, m1, p1).is_err(),
        "Definite integral of 1/x over [-1, 1] must flag pole/divergence"
    );
}

#[test]
fn test_audit_solver_roots_and_edge_cases() {
    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let x_sym = graph.symbols.get_or_intern("x");
    let uni = UnicodeFormatter;

    // 2^x = 8 -> [3]
    let eq1 = parser.parse("2^x - 8").unwrap();
    let r1 = graph.solveset(eq1, x_sym).unwrap();
    assert_eq!(r1.len(), 1);
    assert!((eval_at(&graph, r1[0], "x", 0.0) - 3.0).abs() < 1e-12);

    // sqrt(x) = -1 -> [] (no real solution)
    let eq2 = parser.parse("sqrt(x) + 1").unwrap();
    let r2 = graph.solveset(eq2, x_sym).unwrap();
    assert!(r2.is_empty(), "sqrt(x) = -1 must have no solutions, got {:?}", r2);

    // 1/x = 0 -> []
    let eq3 = parser.parse("1/x").unwrap();
    let r3 = graph.solveset(eq3, x_sym).unwrap();
    assert!(r3.is_empty(), "1/x = 0 must have no solutions");

    // abs(x) = 3 -> [-3, 3]
    let eq4 = parser.parse("abs(x) - 3").unwrap();
    let r4 = graph.solveset(eq4, x_sym).unwrap();
    let mut vals4: Vec<f64> = r4.iter().map(|&r| eval_at(&graph, r, "x", 0.0)).collect();
    vals4.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(vals4.len(), 2);
    assert!((vals4[0] + 3.0).abs() < 1e-9 && (vals4[1] - 3.0).abs() < 1e-9);

    // x^2 - 2*x*y + y^2 = 0 -> [y]
    let eq5 = parser.parse("x^2 - 2*x*y + y^2").unwrap();
    let r5 = graph.solveset(eq5, x_sym).unwrap();
    assert_eq!(r5.len(), 1);
    assert_eq!(uni.format(&graph, r5[0]).unwrap(), "y");

    // a*x^2 + b*x + c = 0 -> two symbolic quadratic formula roots
    let eq6 = parser.parse("a*x^2 + b*x + c").unwrap();
    let r6 = graph.solveset(eq6, x_sym).unwrap();
    assert_eq!(r6.len(), 2);
    for &root in &r6 {
        let s = uni.format(&graph, root).unwrap();
        assert!(s.contains('a') && s.contains('b') && s.contains('c'), "unexpected root: {}", s);
    }

    // x^4 + x^3 + x^2 + x + 1 = 0 -> exact radical roots (no rational float approximations)
    let eq7 = parser.parse("x^4 + x^3 + x^2 + x + 1").unwrap();
    let r7 = graph.solveset(eq7, x_sym).unwrap();
    assert_eq!(r7.len(), 4);
    for &root in &r7 {
        let s = uni.format(&graph, root).unwrap();
        assert!(!s.contains("416020"), "must use exact radicals, not convergent float ratio: {}", s);
    }
}

#[test]
fn test_audit_factor_expand_roundtrip_property() {
    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);

    for poly_s in [
        "x^2 - 5*x + 6",
        "x^6 - 1",
        "x^4 + 4",
        "x^5 + x + 1",
        "x^3 - 3*x^2 + 3*x - 1",
        "x^2 - y^2",
        "x^2 + 2*x*y + y^2",
    ] {
        let p = parser.parse(poly_s).unwrap();
        let fac = AlgebraicTransformations::factor(&graph, p);
        let back = AlgebraicTransformations::expand(&graph, fac);
        let diff_zero = graph.simplify(graph.sub(back, p));
        let mut b = HashMap::new();
        b.insert("x".to_string(), 1.7);
        b.insert("y".to_string(), -2.3);
        let ctx = EvalContext::with_bindings(12, b);
        let val = graph.evalf(diff_zero, &ctx).unwrap().to_f64();
        assert!(val.abs() < 1e-8, "expand(factor({poly_s})) != {poly_s}: residual={val}");
    }
}

#[test]
fn test_audit_simplify_identities_limits_and_no_stubs() {
    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let uni = UnicodeFormatter;

    for (inp, expected) in [
        ("sin(x)^2 + cos(x)^2", "1"),
        ("(x^2 - 1)/(x - 1)", "x + 1"),
        ("x / x", "1"),
        ("log(exp(x))", "x"),
        ("(2*x)/2", "x"),
        ("1/(1/x)", "x"),
        ("sin(0)", "0"),
        ("log(1)", "0"),
        ("i^2", "-1"),
        ("(x + y)^2 - x^2 - 2*x*y - y^2", "0"),
    ] {
        let e = parser.parse(inp).unwrap();
        let s = graph.simplify(e);
        let rendered = uni.format(&graph, s).unwrap();
        assert_eq!(rendered, expected, "simplify({})", inp);
    }

    // Limit 1/x as x -> 0 must be undefined
    let x_sym = graph.symbols.get_or_intern("x");
    let inv_x = parser.parse("1/x").unwrap();
    let zero = parser.parse("0").unwrap();
    let l0 = graph.limit(inv_x, x_sym, zero).unwrap();
    assert!(matches!(
        graph.get(l0).kind,
        algebra_core::ExprKind::Number(algebra_core::Number::Constant(
            algebra_core::Constant::Undefined
        ))
    ));

    // Commands that previously returned fake placeholders now compute real answers
    assert_eq!(run_cli("det [[1,2],[3,4]]").output_text.trim(), "-2");
    assert_eq!(run_cli("trace [[1,2],[3,4]]").output_text.trim(), "5");
    assert!(!run_cli("sat A and not A").output_text.to_lowercase().contains("satisfiable (true)"));
    assert!(run_cli("sat A and not A").output_text.contains("Unsatisfiable"));
    assert!(run_cli("galois S5").output_text.contains("false"));
    assert!(run_cli("galois A4").output_text.contains("true"));
    assert_eq!(run_cli("sum k^2, k, 1, 10").output_text.trim(), "385");

    // 64-bit Miller-Rabin + Pollard rho speed/correctness
    let factors = integer_prime_factors(600851475143);
    assert_eq!(factors, vec![(71, 1), (839, 1), (1471, 1), (6857, 1)]);
}

#[test]
fn test_audit_parser_implicit_mul_factorial_mod_and_json_api() {
    let graph = ExprGraph::new();

    // JSON API must route commands cleanly, not try to parse "det ..." as Expr
    let resp_raw = process_json_request(&graph, r#"{"input": "det [[1,2],[3,4]]"}"#);
    let resp_det: UraeJsonResponse = serde_json::from_str(&resp_raw).expect("parse json response");
    assert!(resp_det.success, "json det failed: {:?}", resp_det.error_msg);
    assert_eq!(resp_det.output_unicode.trim(), "-2");

    // JSON API implicit mul + modulo + factorial + 2-arg log + expression gcd
    assert!(run_cli("eval 2x + 3").is_error); // free symbol x
    assert_eq!(run_cli("diff 2x^3, x").output_text, run_cli("diff 2*x^3, x").output_text);
    assert_eq!(run_cli("expand (x+1)(x-1)").output_text, run_cli("expand (x+1)*(x-1)").output_text);
    assert_eq!(run_cli("eval 5!").output_text.trim(), "120");
    assert_eq!(run_cli("eval 10 % 3").output_text.trim(), "1");
    assert_eq!(run_cli("eval log(8, 2)").output_text.trim(), "3");
    assert!(run_cli("gcd 2^67 - 1, 193707721").output_text.contains("gcd(147573952589676412927, 193707721) = 193707721"));
    assert!(run_cli("is_prime 2^89 - 1").output_text.contains("= true"));
    assert!(run_cli("prime_factors 2^67 - 1").output_text.contains("193707721 * 761838257287"));
    assert!(run_cli("lim (1+1/x)^x, x, oo").output_text.trim().ends_with("= e"));
    assert!(!run_cli("matmul [[1,2],[3,4]], [[5,6],[7,8]]").is_error);
}

#[test]
fn test_audit_depth_overhaul_steps_1_to_4() {
    use algebra_engine::limits::{LimitDirection, LimitEngine};
    use algebra_engine::logic::Predicate;
    use algebra_engine::series::SymbolicSeries as _;
    use algebra_engine::transforms::SymbolicTransforms as _;

    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let x_sym = graph.symbols.get_or_intern("x");
    let uni = UnicodeFormatter;

    // 1. Differentiation: trig, hyperbolic, inverse hyperbolic, abs
    let diff_cases = [
        ("sec(x)", 0.5_f64, 0.5_f64.tan() / 0.5_f64.cos()),
        ("csc(x)", 0.5, -1.0 / (0.5_f64.sin() * 0.5_f64.tan())),
        ("cot(x)", 0.5, -1.0 / (0.5_f64.sin().powi(2))),
        ("sech(x)", 0.5, -0.5_f64.tanh() / 0.5_f64.cosh()),
        ("csch(x)", 0.5, -1.0 / (0.5_f64.sinh() * 0.5_f64.tanh())),
        ("coth(x)", 0.5, -1.0 / (0.5_f64.sinh().powi(2))),
        ("asinh(x)", 0.5, 1.0 / (1.0_f64 + 0.25).sqrt()),
        ("acosh(x)", 2.0, 1.0 / (4.0_f64 - 1.0).sqrt()),
        ("atanh(x)", 0.4, 1.0 / (1.0 - 0.16)),
        ("abs(x)", -2.5, -1.0),
        ("abs(x)", 2.5, 1.0),
    ];
    for (expr_s, pt, expected) in diff_cases {
        let e = parser.parse(expr_s).unwrap();
        let d = graph.diff(e, x_sym);
        let got = eval_at(&graph, d, "x", pt);
        assert!(
            (got - expected).abs() < 1e-8,
            "diff({}) at {} = {}, expected {}",
            expr_s,
            pt,
            got,
            expected
        );
    }

    // 2. Risch / integration fundamental theorem check: d/dx(integrate(f)) == f
    for expr_s in [
        "sec(x)^2",
        "cos(x)^(-2)",
        "x*exp(-x^2)",
        "x*sin(x^2)",
        "cos(x)*sin(x)^3",
        "sec(x)*tan(x)",
        "1/sqrt(1 + x^2)",
    ] {
        let e = parser.parse(expr_s).unwrap();
        let antideriv = graph
            .integrate(e, x_sym)
            .unwrap_or_else(|err| panic!("integrate({}) failed: {:?}", expr_s, err));
        let deriv_back = graph.diff(antideriv, x_sym);
        for &pt in &[0.3, 0.7] {
            let expected = eval_at(&graph, e, "x", pt);
            let got = eval_at(&graph, deriv_back, "x", pt);
            assert!(
                (got - expected).abs() < 1e-6,
                "d/dx int({}) at {} = {}, expected {}",
                expr_s,
                pt,
                got,
                expected
            );
        }
    }
    for expr_s in ["1/(x*ln(x))", "ln(x)/x", "1/(x^3 - 1)"] {
        let e = parser.parse(expr_s).unwrap();
        let antideriv = graph
            .integrate(e, x_sym)
            .unwrap_or_else(|err| panic!("integrate({}) failed: {:?}", expr_s, err));
        let deriv_back = graph.diff(antideriv, x_sym);
        for &pt in &[1.5, 2.2] {
            let expected = eval_at(&graph, e, "x", pt);
            let got = eval_at(&graph, deriv_back, "x", pt);
            assert!(
                (got - expected).abs() < 1e-6,
                "d/dx int({}) at {} = {}, expected {}",
                expr_s,
                pt,
                got,
                expected
            );
        }
    }

    // 3. Improper & singular definite integrals + divergent rejection
    for (expr_s, lo_s, hi_s, expected) in [
        ("1/sqrt(x)", "0", "1", 2.0),
        ("exp(-x)", "0", "oo", 1.0),
        ("1/x^2", "1", "oo", 1.0),
    ] {
        let e = parser.parse(expr_s).unwrap();
        let lo = parser.parse(lo_s).unwrap();
        let hi = parser.parse(hi_s).unwrap();
        let ans = graph
            .integrate_definite(e, x_sym, lo, hi)
            .unwrap_or_else(|err| panic!("def int {} [{}, {}] failed: {:?}", expr_s, lo_s, hi_s, err));
        let val = graph.evalf(ans, &EvalContext::default()).unwrap().to_f64();
        assert!(
            (val - expected).abs() < 1e-7,
            "def int {} [{}, {}] = {}, expected {}",
            expr_s,
            lo_s,
            hi_s,
            val,
            expected
        );
    }
    for (expr_s, lo_s, hi_s) in [("1/x", "-1", "1"), ("1/x^2", "-1", "1")] {
        let e = parser.parse(expr_s).unwrap();
        let lo = parser.parse(lo_s).unwrap();
        let hi = parser.parse(hi_s).unwrap();
        let res = graph.integrate_definite(e, x_sym, lo, hi);
        assert!(
            res.is_err(),
            "divergent integral {} on [{}, {}] unexpectedly succeeded: {:?}",
            expr_s,
            lo_s,
            hi_s,
            res
        );
    }

    // 4. Series & directional limits
    let sin_x = parser.parse("sin(x)").unwrap();
    let zero = graph.integer(0);
    let laur = graph.laurent_series(sin_x, x_sym, zero, 0, 3).unwrap();
    let laur_s = uni.format(&graph, laur).unwrap();
    assert!(!laur_s.contains("^0"), "laurent(sin(x)) leaked zero term: {}", laur_s);
    let sinc = parser.parse("sin(x)/x").unwrap();
    let tay = graph.taylor_series(sinc, x_sym, zero, 4);
    let tay_val = eval_at(&graph, tay, "x", 0.2);
    let tay_expected = 1.0 - 0.2_f64.powi(2) / 6.0 + 0.2_f64.powi(4) / 120.0;
    assert!((tay_val - tay_expected).abs() < 1e-10, "taylor(sin(x)/x) = {}", tay_val);

    let lim_rad = run_cli("lim (sqrt(x+1) - 1)/x, x, 0");
    assert!(!lim_rad.is_error, "{:?}", lim_rad.error_msg);
    assert!(lim_rad.output_text.contains("1/2"), "got {}", lim_rad.output_text);
    let inv_x = parser.parse("1/x").unwrap();
    let r_lim = LimitEngine::limit_directed(&graph, inv_x, x_sym, zero, LimitDirection::FromRight).unwrap();
    let l_lim = LimitEngine::limit_directed(&graph, inv_x, x_sym, zero, LimitDirection::FromLeft).unwrap();
    assert!(uni.format(&graph, r_lim).unwrap().contains("∞") || uni.format(&graph, r_lim).unwrap().contains("oo"));
    assert!(uni.format(&graph, l_lim).unwrap().contains("-"));

    // 5. Symbolic sums (Faulhaber, geometric, p-series)
    assert_eq!(run_cli("sum k^2, k, 1, 100000").output_text.trim(), "333338333350000");
    let sum_k5_num = run_cli("sum k^5, k, 1, 5");
    assert_eq!(sum_k5_num.output_text.trim(), "4425");
    let sum_k5_sym = run_cli("sum k^5, k, 1, n");
    assert!(!sum_k5_sym.is_error, "{:?}", sum_k5_sym.error_msg);
    let sum_geo = run_cli("sum (1/2)^k, k, 0, oo");
    assert_eq!(sum_geo.output_text.trim(), "2");
    let sum_basel = run_cli("sum 1/k^2, k, 1, oo");
    assert!(!sum_basel.is_error, "{:?}", sum_basel.error_msg);
    assert!(sum_basel.output_text.contains("π") || sum_basel.output_text.contains("pi"), "got {}", sum_basel.output_text);

    // 6. Solver: radical extraneous root filtering, log combination, difference of squares, assumptions
    let rad_eq = parser.parse("sqrt(x + 2) = x").unwrap();
    let rad_sols = graph.solveset(rad_eq, x_sym).unwrap();
    let rad_vals: Vec<f64> = rad_sols
        .iter()
        .map(|&s| graph.evalf(s, &EvalContext::default()).unwrap().to_f64())
        .collect();
    assert_eq!(rad_vals.len(), 1, "expected only [2], got {:?}", rad_vals);
    assert!((rad_vals[0] - 2.0).abs() < 1e-9);

    let log_eq = parser.parse("log(x) + log(x - 3) = log(4)").unwrap();
    let log_sols = graph.solveset(log_eq, x_sym).unwrap();
    let log_vals: Vec<f64> = log_sols
        .iter()
        .map(|&s| graph.evalf(s, &EvalContext::default()).unwrap().to_f64())
        .collect();
    assert_eq!(log_vals.len(), 1, "expected only [4], got {:?}", log_vals);
    assert!((log_vals[0] - 4.0).abs() < 1e-9);

    let diff_sq = parser.parse("x^2 - y^2 = 0").unwrap();
    let diff_sq_sols = graph.solveset(diff_sq, x_sym).unwrap();
    let diff_sq_strs: Vec<String> = diff_sq_sols
        .iter()
        .map(|&s| uni.format(&graph, s).unwrap())
        .collect();
    assert!(diff_sq_strs.contains(&"y".to_string()), "got {:?}", diff_sq_strs);
    assert!(diff_sq_strs.contains(&"-y".to_string()), "got {:?}", diff_sq_strs);

    let mut ctx = ExecutionContext::new();
    ctx.assumptions.assume(x_sym, Predicate::Positive);
    let op_solve = parse_operation("solve x^2 - 4 = 0, x");
    let solve_pos = OperationExecutor::execute(&graph, &op_solve, &ctx);
    assert!(solve_pos.output_text.contains("[2]"), "expected only [2], got {}", solve_pos.output_text);

    // 7. Transforms & Exact Trig Evaluation
    let s_sym = graph.symbols.get_or_intern("s");
    let t_sym = graph.symbols.get_or_intern("t");
    let w_sym = graph.symbols.get_or_intern("w");
    let ilap_in = parser.parse("1 / (s * (s + 1))").unwrap();
    let ilap_out = graph.inverse_laplace_transform(ilap_in, s_sym, t_sym).unwrap();
    let ilap_val = eval_at(&graph, ilap_out, "t", 1.0);
    assert!((ilap_val - (1.0 - (-1.0_f64).exp())).abs() < 1e-8, "inv_laplace = {}", ilap_val);

    let f_in = parser.parse("exp(-a*t)*heaviside(t)").unwrap();
    let f_out = graph.fourier_transform(f_in, t_sym, w_sym).unwrap();
    let mut fb = HashMap::new();
    fb.insert("a".to_string(), 2.0);
    fb.insert("w".to_string(), 3.0);
    let f_ctx = EvalContext::with_bindings(15, fb);
    let f_val = graph.evalf(f_out, &f_ctx).unwrap();
    match f_val {
        algebra_engine::numeric::BigValue::Complex(re, im) => {
            assert!((re - 2.0 / 13.0).abs() < 1e-8 && (im + 3.0 / 13.0).abs() < 1e-8, "got ({}, {})", re, im);
        }
        other => panic!("expected complex 1/(2+3i), got {:?}", other),
    }

    assert_eq!(run_cli("eval sin(pi/6)").output_text.trim(), "1/2");
    assert_eq!(run_cli("eval tan(pi/4)").output_text.trim(), "1");

    // 8. JSON API variable inference, format->parse roundtrip, expand(factor(p))
    let raw_infer = process_json_request(
        &graph,
        r#"{"input": "t^2 + 3*t", "compute_derivative": true}"#,
    );
    let resp_infer: UraeJsonResponse = serde_json::from_str(&raw_infer).unwrap();
    assert!(resp_infer.success);
    let d_tex = resp_infer.derivative_latex.unwrap_or_default();
    assert!(d_tex.contains('t') && d_tex.contains('2') && d_tex.contains('3'), "got {}", d_tex);

    for src in ["(x - 3)*(x - 2)", "(a + b)/(c - d)^2"] {
        let e = parser.parse(src).unwrap();
        let formatted = uni.format(&graph, e).unwrap();
        let reparsed = parser
            .parse(&formatted)
            .unwrap_or_else(|err| panic!("failed to reparse '{}': {:?}", formatted, err));
        let mut b = HashMap::new();
        b.insert("x".to_string(), 5.3);
        b.insert("a".to_string(), 1.2);
        b.insert("b".to_string(), 2.4);
        b.insert("c".to_string(), 4.1);
        b.insert("d".to_string(), 1.7);
        let c = EvalContext::with_bindings(15, b);
        let v1 = graph.evalf(e, &c).unwrap().to_f64();
        let v2 = graph.evalf(reparsed, &c).unwrap().to_f64();
        assert!((v1 - v2).abs() < 1e-10, "roundtrip mismatch {} -> {} ({} vs {})", src, formatted, v1, v2);
    }

    let poly = parser.parse("x^2 - 5*x + 6").unwrap();
    let factored = AlgebraicTransformations::factor(&graph, poly);
    let expanded = AlgebraicTransformations::expand(&graph, factored);
    for &pt in &[-2.0, 0.5, 4.0] {
        let v1 = eval_at(&graph, poly, "x", pt);
        let v2 = eval_at(&graph, expanded, "x", pt);
        assert!((v1 - v2).abs() < 1e-10, "expand(factor(p)) mismatch at {}", pt);
    }

    // 9. End-to-end depth parity: comma-args inside fn calls, trig/ln/Euler simplification,
    // exact integer eval for ln(e^5)/i^2, trig solve simplification, Proof/Compare, and ODE solving.
    assert!(run_cli("diff log(x, 2), x").output_text.contains("ln(2)"), "got {}", run_cli("diff log(x, 2), x").output_text);
    assert_eq!(run_cli("simplify sin(pi/6)").output_text.trim(), "1/2");
    assert_eq!(run_cli("simplify exp(i*pi)").output_text.trim(), "-1");
    assert_eq!(run_cli("eval ln(e^5)").output_text.trim(), "5");
    assert_eq!(run_cli("eval i^2").output_text.trim(), "-1");
    assert!(run_cli("integrate sin(x), x, 0, pi").output_text.starts_with("2 "), "got {}", run_cli("integrate sin(x), x, 0, pi").output_text);
    assert!(run_cli("integrate 1/x, x, 1, e").output_text.starts_with("1 "), "got {}", run_cli("integrate 1/x, x, 1, e").output_text);

    let solve_sin = run_cli("solve sin(x) = 0, x");
    assert!(!solve_sin.output_text.contains("asin("), "unsimplified solve sin(x)=0: {}", solve_sin.output_text);
    let solve_cos = run_cli("solve cos(x) = 1/2, x");
    assert!(solve_cos.output_text.contains("π / 3") || solve_cos.output_text.contains("pi / 3"), "got {}", solve_cos.output_text);

    let prove_ok = run_cli("prove sin(x)^2 + cos(x)^2 = 1");
    assert!(!prove_ok.is_error && prove_ok.output_text.contains("Verified"), "got {}", prove_ok.output_text);
    let prove_false = run_cli("prove sin(x) = 0");
    assert!(prove_false.is_error, "false identity must error, got: {}", prove_false.output_text);

    let cmp_eq = run_cli("compare (x + 1)^2, x^2 + 2*x + 1");
    assert!(!cmp_eq.is_error && cmp_eq.output_text.starts_with("Equal:"), "got {}", cmp_eq.output_text);

    let ode1 = run_cli("solve_ode y' + y = 0, y, x");
    assert!(!ode1.is_error && ode1.output_text.contains("exp(-x)") && ode1.output_text.contains("C1"), "got {}", ode1.output_text);
    let ode2 = run_cli("solve_ode y'' + y = 0, y, x");
    assert!(!ode2.is_error && ode2.output_text.contains("cos(x)") && ode2.output_text.contains("sin(x)"), "got {}", ode2.output_text);
}
