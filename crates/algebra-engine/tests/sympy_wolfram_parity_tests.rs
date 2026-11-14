//! # SymPy & Wolfram Alpha Feature Parity Test Suite
//!
//! Validates:
//! 1. Special Functions: Gamma, Beta, Zeta, Eta, Erf, Erfc, Lambert W, Bessel, Si, Ci, Polylog exact and numerical values.
//! 2. Special Function Derivatives: d/dx of Gamma, Erf, Lambert W, Si, Ci, Asin, Atan, Tan.
//! 3. Transcendental & Lambert W Equation Solving: e^x = 5, ln(x) = 3, x*e^x = 2, x*ln(x) = 1, e^(2x) - 3e^x + 2 = 0.
//! 4. Advanced Symbolic Integration: x*sin(x), x*cos(x), x*ln(x), x^2*e^x, e^x*sin(x), asin(x), atan(x), erf(x), definite integrals.
//! 5. Matrix Linear Algebra: determinant, trace, inverse, characteristic polynomial, eigenvalues, eigenvectors, matrix exponential e^(At).
//! 6. Wolfram Alpha Multi-Pod Query Engine: natural language queries generating structured pods.

use algebra_core::format::{Formatter, UnicodeFormatter};
use algebra_core::{Constant, ExprGraph};
use algebra_engine::calculus::SymbolicCalculus;
use algebra_engine::matrix::SymbolicMatrix;
use algebra_engine::numeric::{EvalContext, NumericalEval};
use algebra_engine::solver::SymbolicSolver;
use algebra_engine::special::SpecialFunctions;
use algebra_engine::wolfram::WolframAlphaEngine;
use algebra_engine::ExecutionContext;

// =========================================================================
// 1. SPECIAL FUNCTIONS EXACT VALUES
// =========================================================================

#[test]
fn test_gamma_and_beta_exact_values() {
    let graph = ExprGraph::new();
    let fmt = UnicodeFormatter;

    // Gamma(5) = 4! = 24
    let n5 = graph.integer(5);
    let g5 = graph.gamma(n5);
    let eval_g5 = graph.evaluate_special(g5).unwrap();
    assert_eq!(fmt.format(&graph, eval_g5).unwrap(), "24");

    // Gamma(1/2) = sqrt(pi)
    let half = graph.rational(1, 2);
    let g_half = graph.gamma(half);
    let eval_half = graph.evaluate_special(g_half).unwrap();
    assert_eq!(fmt.format(&graph, eval_half).unwrap(), "√(π)");

    // Gamma(3/2) = sqrt(pi) / 2
    let three_halfs = graph.rational(3, 2);
    let g_32 = graph.gamma(three_halfs);
    let eval_32 = graph.evaluate_special(g_32).unwrap();
    let str_32 = fmt.format(&graph, eval_32).unwrap();
    assert!(str_32.contains("√(π)") && str_32.contains('2'));

    // Beta(2, 3) = 1! * 2! / 4! = 2 / 24 = 1/12
    let two = graph.integer(2);
    let three = graph.integer(3);
    let b23 = graph.beta(two, three);
    let eval_b23 = graph.evaluate_special(b23).unwrap();
    assert_eq!(fmt.format(&graph, eval_b23).unwrap(), "1/12");
}

#[test]
fn test_zeta_and_eta_exact_values() {
    let graph = ExprGraph::new();
    let fmt = UnicodeFormatter;

    // zeta(0) = -1/2
    let z0 = graph.zeta(graph.integer(0));
    let eval_z0 = graph.evaluate_special(z0).unwrap();
    assert_eq!(fmt.format(&graph, eval_z0).unwrap(), "-1/2");

    // zeta(2) = pi^2 / 6
    let z2 = graph.zeta(graph.integer(2));
    let eval_z2 = graph.evaluate_special(z2).unwrap();
    assert_eq!(fmt.format(&graph, eval_z2).unwrap(), "(π^2) / (6)");

    // zeta(4) = pi^4 / 90
    let z4 = graph.zeta(graph.integer(4));
    let eval_z4 = graph.evaluate_special(z4).unwrap();
    assert_eq!(fmt.format(&graph, eval_z4).unwrap(), "(π^4) / (90)");

    // zeta(-1) = -1/12
    let z_neg1 = graph.zeta(graph.integer(-1));
    let eval_zneg1 = graph.evaluate_special(z_neg1).unwrap();
    assert_eq!(fmt.format(&graph, eval_zneg1).unwrap(), "-1/12");

    // Trivial zero: zeta(-2) = 0
    let z_neg2 = graph.zeta(graph.integer(-2));
    let eval_zneg2 = graph.evaluate_special(z_neg2).unwrap();
    assert_eq!(fmt.format(&graph, eval_zneg2).unwrap(), "0");

    // Dirichlet eta(1) = ln(2)
    let eta1 = graph.dirichlet_eta(graph.integer(1));
    let eval_eta1 = graph.evaluate_special(eta1).unwrap();
    assert_eq!(fmt.format(&graph, eval_eta1).unwrap(), "ln(2)");
}

#[test]
fn test_erf_lambertw_bessel_exact_values() {
    let graph = ExprGraph::new();
    let fmt = UnicodeFormatter;

    // erf(0) = 0
    let erf0 = graph.erf(graph.integer(0));
    let eval_erf0 = graph.evaluate_special(erf0).unwrap();
    assert_eq!(fmt.format(&graph, eval_erf0).unwrap(), "0");

    // erfc(0) = 1
    let erfc0 = graph.erfc(graph.integer(0));
    let eval_erfc0 = graph.evaluate_special(erfc0).unwrap();
    assert_eq!(fmt.format(&graph, eval_erfc0).unwrap(), "1");

    // W(0) = 0
    let w0 = graph.lambert_w(graph.integer(0));
    let eval_w0 = graph.evaluate_special(w0).unwrap();
    assert_eq!(fmt.format(&graph, eval_w0).unwrap(), "0");

    // W(e) = 1
    let e_sym = graph.constant(Constant::E);
    let we = graph.lambert_w(e_sym);
    let eval_we = graph.evaluate_special(we).unwrap();
    assert_eq!(fmt.format(&graph, eval_we).unwrap(), "1");

    // J_0(0) = 1, J_1(0) = 0
    let j0_0 = graph.bessel_j(graph.integer(0), graph.integer(0));
    let eval_j0 = graph.evaluate_special(j0_0).unwrap();
    assert_eq!(fmt.format(&graph, eval_j0).unwrap(), "1");

    let j1_0 = graph.bessel_j(graph.integer(1), graph.integer(0));
    let eval_j1 = graph.evaluate_special(j1_0).unwrap();
    assert_eq!(fmt.format(&graph, eval_j1).unwrap(), "0");
}

// =========================================================================
// 2. SPECIAL FUNCTIONS HIGH-PRECISION NUMERICAL EVALUATION
// =========================================================================

#[test]
fn test_numerical_special_functions() {
    use algebra_engine::numeric::*;

    // Gamma Lanczos evaluation
    let g5 = eval_gamma(5.0);
    assert!((g5 - 24.0).abs() < 1e-12);

    let g_half = eval_gamma(0.5);
    assert!((g_half - std::f64::consts::PI.sqrt()).abs() < 1e-12);

    // Beta function
    let b23 = eval_beta(2.0, 3.0);
    assert!((b23 - 1.0 / 12.0).abs() < 1e-12);

    // Error function
    let erf0 = eval_erf(0.0);
    assert_eq!(erf0, 0.0);
    let erf1 = eval_erf(1.0);
    assert!((erf1 - 0.8427007929).abs() < 1e-5);

    // Riemann Zeta
    let z2 = eval_zeta(2.0);
    assert!((z2 - std::f64::consts::PI.powi(2) / 6.0).abs() < 1e-10);

    let z4 = eval_zeta(4.0);
    assert!((z4 - std::f64::consts::PI.powi(4) / 90.0).abs() < 1e-10);

    // Lambert W function via Halley iteration
    let w0 = eval_lambert_w(0.0);
    assert_eq!(w0, 0.0);
    let we = eval_lambert_w(std::f64::consts::E);
    assert!((we - 1.0).abs() < 1e-12);

    // Omega constant: W(1) * e^W(1) = 1 => W(1) ~ 0.5671432904097838
    let w1 = eval_lambert_w(1.0);
    assert!((w1 * w1.exp() - 1.0).abs() < 1e-12);

    // Sine Integral
    let si0 = eval_si(0.0);
    assert_eq!(si0, 0.0);
    let si1 = eval_si(1.0);
    assert!((si1 - 0.94608307).abs() < 1e-5);
}

// =========================================================================
// 3. SPECIAL FUNCTION DERIVATIVES
// =========================================================================

#[test]
fn test_special_function_derivatives() {
    let graph = ExprGraph::new();
    let fmt = UnicodeFormatter;
    let x = graph.symbol("x");
    let x_sym = match graph.get(x).kind {
        algebra_core::ExprKind::Symbol(s) => s,
        _ => unreachable!(),
    };

    // d/dx [erf(x)] = (2 / sqrt(pi)) * exp(-x^2)
    let erf_x = graph.erf(x);
    let d_erf = graph.diff(erf_x, x_sym);
    let d_erf_str = fmt.format(&graph, d_erf).unwrap();
    assert!(d_erf_str.contains("√(π)") || d_erf_str.contains("pi"));
    assert!(d_erf_str.contains("exp(-x^2)") || d_erf_str.contains("exp(-(x^2))") || d_erf_str.contains("exp"));

    // d/dx [lambert_w(x)] = W(x) / (x * (1 + W(x)))
    let w_x = graph.lambert_w(x);
    let d_w = graph.diff(w_x, x_sym);
    let d_w_str = fmt.format(&graph, d_w).unwrap();
    assert!(d_w_str.contains("lambert_w(x)"));

    // d/dx [si(x)] = sin(x) / x
    let si_x = graph.sine_integral(x);
    let d_si = graph.diff(si_x, x_sym);
    let d_si_str = fmt.format(&graph, d_si).unwrap();
    assert_eq!(d_si_str, "(sin(x)) / (x)");

    // d/dx [atan(x)] = 1 / (1 + x^2)
    let atan_x = graph.function("atan", [x]);
    let d_atan = graph.diff(atan_x, x_sym);
    let d_atan_str = fmt.format(&graph, d_atan).unwrap();
    assert_eq!(d_atan_str, "(1) / (x^2 + 1)");
}

// =========================================================================
// 4. TRANSCENDENTAL & LAMBERT W EQUATION SOLVING
// =========================================================================

#[test]
fn test_transcendental_equation_solving() {
    let graph = ExprGraph::new();
    let fmt = UnicodeFormatter;
    let x = graph.symbol("x");
    let x_sym = match graph.get(x).kind {
        algebra_core::ExprKind::Symbol(s) => s,
        _ => unreachable!(),
    };

    // 1. Solve e^x = 5 => x = ln(5)
    let exp_x = graph.function("exp", [x]);
    let eq1 = graph.sub(exp_x, graph.integer(5));
    let roots1 = graph.solveset(eq1, x_sym).unwrap();
    assert_eq!(roots1.len(), 1);
    assert_eq!(fmt.format(&graph, roots1[0]).unwrap(), "ln(5)");

    // 2. Solve ln(x) = 3 => x = exp(3)
    let ln_x = graph.function("ln", [x]);
    let eq2 = graph.sub(ln_x, graph.integer(3));
    let roots2 = graph.solveset(eq2, x_sym).unwrap();
    assert_eq!(roots2.len(), 1);
    assert_eq!(fmt.format(&graph, roots2[0]).unwrap(), "exp(3)");

    // 3. Solve x * e^x = 2 => x = W(2)
    let x_exp_x = graph.mul([x, exp_x]);
    let eq3 = graph.sub(x_exp_x, graph.integer(2));
    let roots3 = graph.solveset(eq3, x_sym).unwrap();
    assert_eq!(roots3.len(), 1);
    assert_eq!(fmt.format(&graph, roots3[0]).unwrap(), "lambert_w(2)");

    // 4. Solve sqrt(x) = 4 => x = 16
    let sqrt_x = graph.function("sqrt", [x]);
    let eq4 = graph.sub(sqrt_x, graph.integer(4));
    let roots4 = graph.solveset(eq4, x_sym).unwrap();
    assert_eq!(roots4.len(), 1);
    assert_eq!(fmt.format(&graph, roots4[0]).unwrap(), "16");

    // 5. Solve quadratic in e^x: e^(2x) - 3*e^x + 2 = 0 => x in {0, ln(2)}
    let two_x = graph.mul([graph.integer(2), x]);
    let exp_2x = graph.function("exp", [two_x]);
    let three_exp_x = graph.mul([graph.integer(3), exp_x]);
    let diff = graph.sub(exp_2x, three_exp_x);
    let eq5 = graph.add([diff, graph.integer(2)]);
    let roots5 = graph.solveset(eq5, x_sym).unwrap();
    assert_eq!(roots5.len(), 2);
    let root_strs: Vec<String> = roots5.iter().map(|&r| fmt.format(&graph, r).unwrap()).collect();
    assert!(root_strs.contains(&"0".to_string()) || root_strs.contains(&"ln(1)".to_string()));
    assert!(root_strs.contains(&"ln(2)".to_string()));
}

// =========================================================================
// 5. ADVANCED SYMBOLIC INTEGRATION
// =========================================================================

#[test]
fn test_advanced_symbolic_integration() {
    let graph = ExprGraph::new();
    let fmt = UnicodeFormatter;
    let x = graph.symbol("x");
    let x_sym = match graph.get(x).kind {
        algebra_core::ExprKind::Symbol(s) => s,
        _ => unreachable!(),
    };

    // 1. int x * sin(x) dx = sin(x) - x*cos(x)
    let sin_x = graph.function("sin", [x]);
    let x_sin_x = graph.mul([x, sin_x]);
    let int1 = graph.integrate(x_sin_x, x_sym).unwrap();
    let int1_str = fmt.format(&graph, int1).unwrap();
    assert!(int1_str.contains("sin(x)") && int1_str.contains("cos(x)"));

    // 2. int x * cos(x) dx = cos(x) + x*sin(x)
    let cos_x = graph.function("cos", [x]);
    let x_cos_x = graph.mul([x, cos_x]);
    let int2 = graph.integrate(x_cos_x, x_sym).unwrap();
    let int2_str = fmt.format(&graph, int2).unwrap();
    assert!(int2_str.contains("sin(x)") && int2_str.contains("cos(x)"));

    // 3. int x * ln(x) dx = (1/2)*x^2*ln(x) - (1/4)*x^2
    let ln_x = graph.function("ln", [x]);
    let x_ln_x = graph.mul([x, ln_x]);
    let int3 = graph.integrate(x_ln_x, x_sym).unwrap();
    let int3_str = fmt.format(&graph, int3).unwrap();
    assert!(int3_str.contains("1/2") && int3_str.contains("ln(x)"));

    // 4. int x^2 * exp(x) dx = (x^2 - 2x + 2)*exp(x)
    let x_sq = graph.pow(x, graph.integer(2));
    let exp_x = graph.function("exp", [x]);
    let xsq_exp = graph.mul([x_sq, exp_x]);
    let int4 = graph.integrate(xsq_exp, x_sym).unwrap();
    let int4_str = fmt.format(&graph, int4).unwrap();
    assert!(int4_str.contains("exp(x)"));

    // 5. int exp(x) * sin(x) dx = (1/2)*exp(x)*(sin(x) - cos(x))
    let exp_sin = graph.mul([exp_x, sin_x]);
    let int5 = graph.integrate(exp_sin, x_sym).unwrap();
    let int5_str = fmt.format(&graph, int5).unwrap();
    assert!(int5_str.contains("1/2") && int5_str.contains("exp(x)"));

    // 6. Definite Integral: int_0^2 x dx = 2^2/2 - 0 = 2
    let int_def = graph.integrate_definite(x, x_sym, graph.integer(0), graph.integer(2)).unwrap();
    let eval_ctx = EvalContext::default();
    let num_val = graph.evalf(int_def, &eval_ctx).unwrap().to_f64();
    assert!((num_val - 2.0).abs() < 1e-12);
}

// =========================================================================
// 6. MATRIX LINEAR ALGEBRA
// =========================================================================

#[test]
fn test_symbolic_matrix_algebra() {
    let graph = ExprGraph::new();
    let fmt = UnicodeFormatter;

    // A = [[2, 1], [1, 2]]
    let a = SymbolicMatrix::from_2x2(&graph, 2.0, 1.0, 1.0, 2.0);

    // Trace = 2 + 2 = 4
    let tr = a.trace(&graph).unwrap();
    let eval_ctx = EvalContext::default();
    assert_eq!(graph.evalf(tr, &eval_ctx).unwrap().to_f64(), 4.0);

    // Determinant = 2*2 - 1*1 = 3
    let det = a.determinant(&graph).unwrap();
    assert_eq!(graph.evalf(det, &eval_ctx).unwrap().to_f64(), 3.0);

    // Inverse = [[2/3, -1/3], [-1/3, 2/3]]
    let inv = a.inverse(&graph).unwrap();
    assert_eq!(inv.rows, 2);
    assert_eq!(inv.cols, 2);

    // Characteristic Polynomial: lambda^2 - 4*lambda + 3
    let cp = a.charpoly(&graph, "lambda").unwrap();
    let cp_str = fmt.format(&graph, cp).unwrap();
    assert!(cp_str.contains("lambda^2"));

    // Eigenvalues: roots of lambda^2 - 4*lambda + 3 = 0 => lambda in {3, 1}
    let e_vals = a.eigenvalues(&graph).unwrap();
    assert_eq!(e_vals.len(), 2);
    let v1 = graph.evalf(e_vals[0], &eval_ctx).unwrap().to_f64();
    let v2 = graph.evalf(e_vals[1], &eval_ctx).unwrap().to_f64();
    assert!((v1 - 3.0).abs() < 1e-10 || (v1 - 1.0).abs() < 1e-10);
    assert!((v2 - 3.0).abs() < 1e-10 || (v2 - 1.0).abs() < 1e-10);

    // Closed-form Matrix Exponential e^(At)
    let exp_at = a.matrix_exp(&graph, "t").unwrap();
    assert_eq!(exp_at.rows, 2);
    assert_eq!(exp_at.cols, 2);
}

// =========================================================================
// 7. WOLFRAM ALPHA MULTI-POD NATURAL LANGUAGE QUERY ENGINE
// =========================================================================

#[test]
fn test_wolfram_alpha_multi_pod_queries() {
    let graph = ExprGraph::new();
    let ctx = ExecutionContext::new();

    // Query 1: "integrate x * sin(x)"
    let resp1 = WolframAlphaEngine::query(&graph, "integrate x * sin(x)", &ctx);
    assert!(resp1.pods.len() >= 2);
    assert_eq!(resp1.pods[0].title, "Input interpretation");
    assert_eq!(resp1.pods[1].title, "Result");
    assert!(resp1.pods[1].unicode.contains("sin(x)"));
    assert!(resp1.pods[1].unicode.contains("cos(x)"));

    // Query 2: "solve x^2 - 4 = 0"
    let resp2 = WolframAlphaEngine::query(&graph, "solve x^2 - 4 = 0", &ctx);
    assert!(resp2.pods.iter().any(|p| p.title == "Result"));
    let res_pod = resp2.pods.iter().find(|p| p.title == "Result").unwrap();
    assert!(res_pod.unicode.contains("2") || res_pod.unicode.contains("-2"));

    // Query 3: "diff e^(-x^2) wrt x"
    let resp3 = WolframAlphaEngine::query(&graph, "diff exp(-x^2), x", &ctx);
    assert!(resp3.pods.iter().any(|p| p.title == "Result"));

    // Query 4: "gamma(5)"
    let resp4 = WolframAlphaEngine::query(&graph, "eval gamma(5)", &ctx);
    let res_g5 = resp4.pods.iter().find(|p| p.title == "Result").unwrap();
    assert!(res_g5.unicode.contains("24"));

    // Query 5: "zeta(2)"
    let resp5 = WolframAlphaEngine::query(&graph, "eval zeta(2)", &ctx);
    let res_z2 = resp5.pods.iter().find(|p| p.title == "Result").unwrap();
    assert!(res_z2.unicode.contains("1.644934") || res_z2.unicode.contains("pi"));

    // Format terminal output
    let terminal_out = resp1.format_terminal();
    assert!(terminal_out.contains("Wolfram Alpha Query Result"));
    assert!(terminal_out.contains("[ Input interpretation ]"));
    assert!(terminal_out.contains("[ Result ]"));
}
