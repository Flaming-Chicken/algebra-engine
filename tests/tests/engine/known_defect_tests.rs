//! Regression tests for defects tracked in `docs/PLANNING.md` (ledger ids A1, A3, B1-B4, B6).
//!
//! Each test asserts the correct behaviour and was written to fail on the code it replaced.

use algebra_core::ExprGraph;
use algebra_core::format::{Formatter, UnicodeFormatter};
use algebra_engine::calculus::SymbolicCalculus;
use algebra_engine::series::RecurrenceSolver;
use algebra_engine::simplify::Simplifier;
use algebra_engine::solver::SymbolicSolver;

fn show(g: &ExprGraph, e: algebra_core::ExprId) -> String {
    UnicodeFormatter
        .format(g, e)
        .unwrap_or_else(|err| format!("format error: {err:?}"))
}

#[test]
fn known_defect_linear_ode_with_zero_coefficient() {
    let g = ExprGraph::new();
    let x = g.symbols.get_or_intern("x");
    let out = g
        .solve_ode_linear(g.integer(0), g.integer(1), x)
        .map(|e| show(&g, e));
    assert_eq!(out.unwrap(), "x");
}

#[test]
fn summation_of_polynomial_summands_matches_direct_addition() {
    let g = ExprGraph::new();
    let k = g.symbols.get_or_intern("k");
    let n_sym = g.symbols.get_or_intern("n");
    let kk = g.symbol("k");
    let n = g.symbol("n");
    let square = g.pow(kk, g.integer(2));
    let cube = g.pow(kk, g.integer(3));
    let cases = [
        // (summand, lower, expected sum for k = lower..=10)
        (square, 1, (1..=10).map(|i| i * i).sum::<i64>()),
        (cube, 1, (1..=10).map(|i| i * i * i).sum::<i64>()),
        (
            g.add([g.mul([g.integer(2), kk]), g.integer(1)]),
            3,
            (3..=10).map(|i| 2 * i + 1).sum(),
        ),
        (
            g.mul([kk, g.add([kk, g.integer(1)])]),
            2,
            (2..=10).map(|i| i * (i + 1)).sum(),
        ),
    ];
    for (summand, lower, expected) in cases {
        let closed = g.summation(summand, k, g.integer(lower), n).unwrap();
        let at_ten = g.substitute(closed, n_sym, g.integer(10));
        let value = Simplifier::simplify(&g, at_ten).unwrap();
        assert_eq!(show(&g, value), expected.to_string(), "sum from {lower}");
    }
}

#[test]
fn summation_rejects_non_polynomial_summands_instead_of_guessing() {
    let g = ExprGraph::new();
    let k = g.symbols.get_or_intern("k");
    let sin_k = g.function("sin", [g.symbol("k")]);
    assert!(g.summation(sin_k, k, g.integer(1), g.symbol("n")).is_err());
}

/// Evaluate an expression in the single symbol `n` as `f64` (numbers, + - * / ^, sqrt).
fn eval_at(g: &ExprGraph, e: algebra_core::ExprId, n: f64) -> f64 {
    use algebra_core::ExprKind;
    match &g.get(e).kind {
        ExprKind::Number(num) => num.as_f64().expect("numeric literal"),
        ExprKind::Symbol(_) => n,
        ExprKind::Add(ts) => ts.iter().map(|&t| eval_at(g, t, n)).sum(),
        ExprKind::Mul(fs) => fs.iter().map(|&f| eval_at(g, f, n)).product(),
        ExprKind::Sub(l, r) => eval_at(g, *l, n) - eval_at(g, *r, n),
        ExprKind::Div(l, r) => eval_at(g, *l, n) / eval_at(g, *r, n),
        ExprKind::Neg(i) => -eval_at(g, *i, n),
        ExprKind::Pow(b, x) => eval_at(g, *b, n).powf(eval_at(g, *x, n)),
        ExprKind::Function { args, .. } => eval_at(g, args[0], n).sqrt(),
        other => panic!("unsupported node {other:?}"),
    }
}

fn sequence(coeffs: &[f64], initial: &[f64], len: usize) -> Vec<f64> {
    let mut out = initial.to_vec();
    while out.len() < len {
        let next = coeffs
            .iter()
            .enumerate()
            .map(|(i, c)| c * out[out.len() - 1 - i])
            .sum();
        out.push(next);
    }
    out
}

#[test]
fn second_order_recurrences_match_the_sequence_they_define() {
    let g = ExprGraph::new();
    let n = g.symbols.get_or_intern("n");
    let cases: [(&[f64], &[f64]); 4] = [
        (&[1.0, 1.0], &[0.0, 1.0]),  // Fibonacci
        (&[1.0, 1.0], &[2.0, 1.0]),  // Lucas
        (&[3.0, -2.0], &[1.0, 5.0]), // roots 1 and 2
        (&[4.0, -4.0], &[1.0, 3.0]), // repeated root 2
    ];
    for (coeffs, initial) in cases {
        let closed = RecurrenceSolver::solve_linear_constant(&g, coeffs, initial, n).unwrap();
        let expected = sequence(coeffs, initial, 16);
        for (k, want) in expected.iter().enumerate() {
            let got = eval_at(&g, closed, k as f64);
            assert!(
                (got - want).abs() < 1e-6 * want.abs().max(1.0),
                "{coeffs:?} {initial:?}: a({k}) = {got}, expected {want}; closed form {}",
                show(&g, closed)
            );
        }
    }
}

#[test]
fn unsupported_recurrences_are_errors_not_wrong_formulas() {
    let g = ExprGraph::new();
    let n = g.symbols.get_or_intern("n");
    // Complex roots, third order, and missing initial values.
    assert!(RecurrenceSolver::solve_linear_constant(&g, &[0.0, -1.0], &[1.0, 0.0], n).is_err());
    assert!(
        RecurrenceSolver::solve_linear_constant(&g, &[1.0, 1.0, 1.0], &[0.0, 0.0, 1.0], n).is_err()
    );
    assert!(RecurrenceSolver::solve_linear_constant(&g, &[1.0, 1.0], &[1.0], n).is_err());
}

#[test]
fn heat_equation_modes_have_spatial_dependence_and_squared_decay() {
    let g = ExprGraph::new();
    let x = g.symbols.get_or_intern("x");
    let t = g.symbols.get_or_intern("t");
    let (spatial, temporal) = g.solve_pde_heat_1d(g.symbol("a"), x, t).unwrap();
    assert_eq!(show(&g, spatial), "cos(k_x · x)");
    let decay = show(&g, temporal);
    assert!(decay.contains("a^2") && decay.contains("k_x^2"), "{decay}");
}

#[test]
fn caputo_kernel_exponent_is_alpha_plus_one_minus_n() {
    use algebra_core::ExprKind;
    use algebra_engine::calculus::FractionalCalculus;
    let g = ExprGraph::new();
    let x = g.symbols.get_or_intern("x");
    let f = g.pow(g.symbol("x"), g.integer(2));
    for alpha in [0.3_f64, 0.7, 1.5, 2.25] {
        let n = alpha.ceil();
        let out = FractionalCalculus::caputo_derivative(&g, f, x, alpha).unwrap();
        let ExprKind::Div(integral, _) = g.get(out).kind.clone() else {
            panic!("expected a quotient")
        };
        let ExprKind::Integral { expr, .. } = g.get(integral).kind.clone() else {
            panic!("expected an integral")
        };
        let ExprKind::Div(_, kernel) = g.get(expr).kind.clone() else {
            panic!("expected a quotient integrand")
        };
        let ExprKind::Pow(_, exponent) = g.get(kernel).kind.clone() else {
            panic!("expected a power")
        };
        let got = match g.get(exponent).kind {
            ExprKind::Number(ref num) => num.as_f64().unwrap(),
            _ => panic!("non-numeric exponent"),
        };
        assert!(
            (got - (alpha + 1.0 - n)).abs() < 1e-12,
            "alpha {alpha}: {got}"
        );
    }
}

#[test]
fn pade_approximants_match_the_function_near_the_expansion_point() {
    use algebra_engine::calculus::pade_approximant;
    let g = ExprGraph::new();
    let x = g.symbols.get_or_intern("x");
    let xe = g.symbol("x");
    let zero = g.integer(0);

    // exp(x): [1/1] = (1 + x/2)/(1 - x/2); [2/2] is accurate to O(x^5).
    let ex = g.function("exp", [xe]);
    for (l, m, tolerance) in [(1, 1, 2e-4), (2, 2, 1e-7), (3, 2, 1e-8)] {
        let approx = pade_approximant(&g, ex, x, zero, l, m).unwrap();
        let got = eval_at(&g, approx, 0.1);
        assert!(
            (got - 0.1_f64.exp()).abs() < tolerance,
            "[{l}/{m}] at 0.1: {got} ({})",
            show(&g, approx)
        );
    }

    // A rational function is reproduced exactly by an approximant of its own degree.
    let geometric = g.div(g.integer(1), g.sub(g.integer(1), xe));
    let approx = pade_approximant(&g, geometric, x, zero, 0, 1).unwrap();
    assert!((eval_at(&g, approx, 0.3) - 1.0 / 0.7).abs() < 1e-12);
}

#[test]
fn pade_reports_when_it_cannot_build_the_approximant() {
    use algebra_engine::calculus::pade_approximant;
    let g = ExprGraph::new();
    let x = g.symbols.get_or_intern("x");
    // exp(x) about a symbolic point has non-rational Taylor coefficients.
    let ex = g.function("exp", [g.symbol("x")]);
    assert!(pade_approximant(&g, ex, x, g.symbol("a"), 1, 1).is_err());
}

fn cartan(kind: algebra_core::operation::CartanOpKind) -> String {
    use algebra_core::operation::MathOperation;
    use algebra_engine::executor::{ExecutionContext, OperationExecutor};
    let g = ExprGraph::new();
    let res =
        OperationExecutor::execute(&g, &MathOperation::Cartan(kind), &ExecutionContext::new());
    if res.is_error {
        format!("ERROR {}", res.error_msg.unwrap_or_default())
    } else {
        res.output_text
    }
}

#[test]
fn cartan_operations_use_the_form_that_was_typed() {
    use algebra_core::operation::CartanOpKind::{
        ExteriorDerivative, HodgeStar, InteriorProduct, Wedge,
    };
    let wedge = |a: &str, b: &str| Wedge {
        form1_str: a.into(),
        form2_str: b.into(),
        dim: 3,
    };
    assert!(cartan(wedge("dx", "dy")).contains("= dx∧dy (degree 2)"));
    // Antisymmetry and nilpotency come from the typed forms, not from fixed dx, dy.
    assert!(cartan(wedge("dy", "dx")).contains("= -dx∧dy"));
    assert!(cartan(wedge("dz", "dz")).contains("= 0 (degree 2)"));
    let mixed = cartan(wedge("2 dx + dz", "dy"));
    assert!(mixed.contains("= 2 dx∧dy - dy∧dz"), "{mixed}");
    // d of a constant-coefficient form is zero.
    assert!(
        cartan(ExteriorDerivative {
            form_str: "3 dx".into(),
            dim: 3
        })
        .contains("= 0")
    );
    // i_X (dx∧dy) with X = (1, 2, 0) is dy - 2 dx.
    let interior = cartan(InteriorProduct {
        form_str: "dx^dy".into(),
        vector_field: vec![1.0, 2.0, 0.0],
        dim: 3,
    });
    assert!(interior.contains("= -2 dx + dy"), "{interior}");
    // The Hodge star of dx in R^3 is dy∧dz.
    let star = cartan(HodgeStar {
        form_str: "dx".into(),
        dim: 3,
        is_minkowski: false,
    });
    assert!(star.contains("= dy∧dz"), "{star}");
    // Unreadable input is an error, not a canned answer.
    assert!(cartan(wedge("nonsense", "dy")).starts_with("ERROR"));
    assert!(cartan(wedge("dw", "dy")).starts_with("ERROR"));
}
