use algebra_core::ExprGraph;
use algebra_core::format::{Formatter, UnicodeFormatter};
use algebra_engine::calculus::SymbolicCalculus;
use algebra_engine::numeric::{EvalContext, NumericalEval};
use std::collections::HashMap;

fn eval_single(graph: &ExprGraph, expr: algebra_core::ExprId, var: &str, val: f64) -> f64 {
    let mut b = HashMap::new();
    b.insert(var.to_string(), val);
    let ctx = EvalContext::with_bindings(53, b);
    graph.evalf(expr, &ctx).unwrap().to_f64()
}

#[test]
fn test_diff_polynomial() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let x_sym = graph.symbols.get_or_intern("x");

    let two = graph.integer(2);
    let x_sq = graph.pow(x, two); // x^2

    let dx = graph.diff(x_sq, x_sym); // d/dx (x^2) = 2x
    assert!((eval_single(&graph, dx, "x", 3.0) - 6.0).abs() < 1e-12);
    assert!((eval_single(&graph, dx, "x", -2.5) - (-5.0)).abs() < 1e-12);
}

#[test]
fn test_diff_sin_cos() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let x_sym = graph.symbols.get_or_intern("x");

    let sin_x = graph.function("sin", [x]);
    let d_sin = graph.diff(sin_x, x_sym); // d/dx sin(x) = cos(x)

    assert_eq!(UnicodeFormatter.format(&graph, d_sin).unwrap(), "cos(x)");
    assert!((eval_single(&graph, d_sin, "x", 0.7) - 0.7_f64.cos()).abs() < 1e-12);
}

#[test]
fn test_integrate_basic() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let x_sym = graph.symbols.get_or_intern("x");

    // int cos(x) dx = sin(x)
    let cos_x = graph.function("cos", [x]);
    let int_cos = graph.integrate(cos_x, x_sym).unwrap();

    assert_eq!(int_cos, graph.function("sin", [x]));
}

#[test]
fn test_limit_lhopital() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let x_sym = graph.symbols.get_or_intern("x");

    // lim_{x -> 0} x / x = 1 via L'Hopital
    let ratio = graph.div(x, x);
    let zero = graph.integer(0);
    let lim = graph.limit(ratio, x_sym, zero).unwrap();

    assert_eq!(lim, graph.integer(1));
}

#[test]
fn test_gosper_summation() {
    let graph = ExprGraph::new();
    let k = graph.symbol("k");
    let k_sym = graph.symbols.get_or_intern("k");
    let n = graph.symbol("n");
    let one = graph.integer(1);

    // sum_{k=1}^n k = n*(n+1)/2
    let sum = graph.summation(k, k_sym, one, n).unwrap();

    assert_ne!(sum, k);
    assert!((eval_single(&graph, sum, "n", 10.0) - 55.0).abs() < 1e-12);
    assert!((eval_single(&graph, sum, "n", 20.0) - 210.0).abs() < 1e-12);
}

#[test]
fn test_fractional_and_variational_calculus() {
    use algebra_engine::calculus::{FractionalCalculus, VariationalCalculus};

    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let x_sym = graph.symbols.get_or_intern("x");

    // Caputo derivative D^{0.5} (x^2) produces integral operator over Gamma(0.5)
    let two = graph.integer(2);
    let x_sq = graph.pow(x, two);
    let f_diff = FractionalCalculus::caputo_derivative(&graph, x_sq, x_sym, 0.5).unwrap();
    let f_str = UnicodeFormatter.format(&graph, f_diff).unwrap();
    assert!(
        f_str.contains("Gamma") && f_str.contains('x'),
        "got {f_str}"
    );

    // Euler-Lagrange for L = 0.5 * v^2 - V(q) => d/dt(v) - (-dV/dq)
    let q = graph.symbol("q");
    let q_sym = graph.symbols.get_or_intern("q");
    let v = graph.symbol("v");
    let v_sym = graph.symbols.get_or_intern("v");
    let _t = graph.symbol("t");
    let t_sym = graph.symbols.get_or_intern("t");

    let v_sq = graph.pow(v, two);
    let kinetic = graph.mul([graph.float(0.5), v_sq]);
    let pot = graph.function("V", [q]);
    let lagrangian = graph.sub(kinetic, pot);

    let el_eq = VariationalCalculus::euler_lagrange(&graph, lagrangian, q_sym, v_sym, t_sym);
    let el_str = UnicodeFormatter.format(&graph, el_eq).unwrap();
    assert!(
        el_str.contains("dt") || el_str.contains('v'),
        "got {el_str}"
    );
}

#[test]
fn inverse_function_aliases_are_one_function_and_have_derivatives() {
    // `arctan` and `atan` (and the other long and short forms) build the same node, so the
    // derivative of either spelling is known. The integrator used to emit `arctan` while `diff`
    // only understood `atan`, leaving d/dx of the antiderivative of 1/(x^2 + 1) unevaluated.
    use algebra_core::ExprGraph;
    use algebra_core::ExprKind;
    use algebra_core::parser::ExprParser;
    let g = ExprGraph::new();
    let parse = |s: &str| ExprParser::new(&g).parse(s).unwrap();
    for (long, short) in [
        ("arctan(x)", "atan(x)"),
        ("arcsin(x)", "asin(x)"),
        ("arccos(x)", "acos(x)"),
        ("arcsinh(x)", "asinh(x)"),
    ] {
        assert_eq!(parse(long), parse(short), "{long} and {short} are one node");
        let ExprKind::Symbol(x) = g.get(g.symbol("x")).kind else {
            unreachable!()
        };
        let d = g.diff(parse(long), x);
        assert!(
            !matches!(g.get(d).kind, ExprKind::Derivative { .. }),
            "d/dx {long} must be evaluated, got an unevaluated derivative"
        );
    }
    // The antiderivative of 1/(x^2 + 1) now differentiates back to the integrand.
    let ExprKind::Symbol(x) = g.get(g.symbol("x")).kind else {
        unreachable!()
    };
    let f = parse("1/(x^2 + 1)");
    let anti = g.integrate(f, x).unwrap();
    let back = g.diff(anti, x);
    use algebra_engine::numeric::{EvalContext, NumericalEval};
    let ctx = EvalContext::with_bindings(53, [("x".to_string(), 0.7)].into_iter().collect());
    let want = 1.0 / (1.0 + 0.7 * 0.7);
    let got = g.evalf(back, &ctx).unwrap().to_f64();
    assert!((got - want).abs() < 1e-12, "{got} vs {want}");
}

#[test]
fn the_arctan_integral_of_a_perfect_square_is_simplified() {
    use algebra_core::ExprKind;
    use algebra_core::format::{Formatter, UnicodeFormatter};
    use algebra_core::parser::ExprParser;
    let g = ExprGraph::new();
    let ExprKind::Symbol(x) = g.get(g.symbol("x")).kind else {
        unreachable!()
    };
    let show = |s: &str| {
        let e = ExprParser::new(&g).parse(s).unwrap();
        UnicodeFormatter
            .format(&g, g.integrate(e, x).unwrap())
            .unwrap()
    };
    // Perfect squares fold: sqrt(1) = 1 and sqrt(4) = 2.
    assert_eq!(show("1/(x^2+1)"), "atan(x)");
    assert_eq!(show("1/(x^2+4)"), "1/2 · atan(1/2 · x)");
    // An irrational constant keeps its radical.
    let text = show("1/(x^2+2)");
    assert!(text.contains("atan") && text.contains("√(2)"), "{text}");
}
