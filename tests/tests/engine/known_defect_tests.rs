//! Regression tests for defects tracked in `docs/PLANNING.md` (ledger ids A1-A4, A6, B1-B4, B6, B8, B10, B12).
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

fn clifford(kind: algebra_core::operation::CliffordOpKind) -> Result<String, String> {
    use algebra_core::operation::MathOperation;
    use algebra_engine::executor::{ExecutionContext, OperationExecutor};
    let g = ExprGraph::new();
    let res =
        OperationExecutor::execute(&g, &MathOperation::Clifford(kind), &ExecutionContext::new());
    if res.is_error {
        Err(res.error_msg.unwrap_or_default())
    } else {
        Ok(res.output_text)
    }
}

/// The executor's rotor rotation and vector product must agree with the multivector
/// implementation in `algebra-advanced` (rotor `cos(t/2) - sin(t/2) e_i e_j`).
#[test]
fn clifford_operations_agree_with_the_multivector_implementation() {
    use algebra_advanced::clifford::{BladeMask, CliffordMultivector, CliffordSignature};
    use algebra_core::operation::CliffordOpKind::{GeometricProduct, RotorSandwich};
    let sig = CliffordSignature { p: 3, q: 0, r: 0 };

    let (a, b) = (vec![1.0, 2.0, -0.5], vec![0.5, -1.0, 3.0]);
    let product = clifford(GeometricProduct {
        sig_p: 3,
        sig_q: 0,
        sig_r: 0,
        a_coords: a.clone(),
        b_coords: b.clone(),
    })
    .unwrap();
    let reference = CliffordMultivector::from_vector(&a, sig)
        .unwrap()
        .geometric_product(&CliffordMultivector::from_vector(&b, sig).unwrap())
        .unwrap();
    let scalar = reference
        .blades
        .get(&BladeMask::SCALAR)
        .copied()
        .unwrap_or(0.0);
    assert!(
        product.contains(&format!("= {scalar} ")),
        "{product} vs scalar {scalar}"
    );
    let e12 = reference.blades.get(&BladeMask(0b011)).copied().unwrap();
    assert!(
        product.contains(&format!("{e12} e1e2")),
        "{product} vs e12 {e12}"
    );

    for (plane, angle) in [((0, 1), 0.7_f64), ((1, 2), -1.9), ((0, 2), 2.5)] {
        let v = vec![1.0, 2.0, 3.0];
        let text = clifford(RotorSandwich {
            sig_p: 3,
            sig_q: 0,
            sig_r: 0,
            vector: v.clone(),
            angle_rad: angle,
            bivector_plane: plane,
        })
        .unwrap();
        let bivector = {
            let ei = CliffordMultivector::basis_vector(plane.0, sig).unwrap();
            let ej = CliffordMultivector::basis_vector(plane.1, sig).unwrap();
            ei.wedge(&ej).unwrap()
        };
        let rotor = CliffordMultivector::make_rotor_3d(&bivector, angle).unwrap();
        let expected = CliffordMultivector::from_vector(&v, sig)
            .unwrap()
            .rotor_sandwich(&rotor)
            .unwrap();
        let listed = text.split("v' = (").nth(1).unwrap().trim_end_matches(')');
        let got: Vec<f64> = listed.split(", ").map(|x| x.parse().unwrap()).collect();
        for (k, g_k) in got.iter().enumerate() {
            let want = expected
                .blades
                .get(&BladeMask::vector(k))
                .copied()
                .unwrap_or(0.0);
            assert!(
                (g_k - want).abs() < 1e-5,
                "plane {plane:?} angle {angle}: {text} vs axis {k} = {want}"
            );
        }
    }

    // Wrong sizes and unsupported signatures are errors, not canned text.
    assert!(
        clifford(GeometricProduct {
            sig_p: 3,
            sig_q: 0,
            sig_r: 0,
            a_coords: vec![1.0],
            b_coords: vec![1.0]
        })
        .is_err()
    );
    assert!(
        clifford(RotorSandwich {
            sig_p: 1,
            sig_q: 1,
            sig_r: 0,
            vector: vec![1.0, 0.0],
            angle_rad: 1.0,
            bivector_plane: (0, 1)
        })
        .is_err()
    );
}

#[test]
fn system_commands_do_what_they_say_or_report_that_they_cannot() {
    use algebra_core::ExprGraph;
    use algebra_core::parser::parse_operation;
    use algebra_engine::executor::{ExecutionContext, OperationExecutor};
    let run = |cmd: &str| {
        let g = ExprGraph::new();
        OperationExecutor::execute(&g, &parse_operation(cmd), &ExecutionContext::new())
    };
    let latex = run("export latex x^2/3");
    assert!(!latex.is_error, "{:?}", latex.error_msg);
    assert!(
        latex.output_text.contains("frac") && latex.output_text.contains("{x}^{2}"),
        "{}",
        latex.output_text
    );
    assert!(run("export mathml x+1").output_text.contains("<m"));
    assert!(run("export docx x+1").is_error);
    assert!(run("log debug").is_error);
    assert!(run("preset quadratic").is_error);
}

#[test]
fn symbolic_tier_labels_the_tangent_line_with_its_real_order() {
    use algebra_core::ExprGraph;
    use algebra_engine::cat_eval::{CatEvalInput, CatExprEvaluator};
    use cat_anytime::{AnytimeBudget, MultiFidelityOp};
    use cat_core::enclosure::{EvalForm, FidelityMode, UncertaintyKind};

    let g = ExprGraph::new();
    let x = g.symbol("x");
    let expr = g.function("sin", [x]);
    let mut input = CatEvalInput::new(g, expr);
    input.var_hint = Some("x".into());
    input.eval_point = Some(0.0);

    let tier1 = CatExprEvaluator::new()
        .eval_with_fidelity(
            &input,
            FidelityMode::SymbolicApprox,
            EvalForm::Canonical,
            &AnytimeBudget::default(),
        )
        .unwrap();
    // Tangent line of sin at 0 is x, with remainder O(x^2): order 2 from a first-order expansion.
    match tier1.enclosure.kind {
        UncertaintyKind::Asymptotic { order, .. } => assert_eq!(order, 2),
        other => panic!("expected an asymptotic enclosure, got {other:?}"),
    }
    assert_eq!(tier1.enclosure.method, "taylor_order_1");

    // Without an expansion variable the expression is only simplified: an exact result.
    let g2 = ExprGraph::new();
    let e2 = g2.add([g2.symbol("y"), g2.integer(0)]);
    let plain = CatExprEvaluator::new()
        .eval_with_fidelity(
            &CatEvalInput::new(g2, e2),
            FidelityMode::SymbolicApprox,
            EvalForm::Canonical,
            &AnytimeBudget::default(),
        )
        .unwrap();
    assert!(matches!(plain.enclosure.kind, UncertaintyKind::Exact));
}

#[test]
fn dyson_first_order_term_integrates_over_the_interval() {
    use algebra_advanced::quantum::DysonSeries;
    let g = ExprGraph::new();
    let t = g.symbols.get_or_intern("t");
    // H_I = 2 (constant): -i * 2 * (t - t0).
    let term = DysonSeries::first_order_term_between(&g, g.integer(2), t, g.symbol("t0"));
    let text = show(&g, Simplifier::simplify(&g, term).unwrap());
    assert!(
        text.contains("t0") && text.contains("t") && text.contains("i"),
        "{text}"
    );
    assert!(!text.contains("H_I"), "{text}");

    // An integrand without a closed form stays an integral instead of being returned as-is.
    let h = g.function("unknown_h", [g.symbol("t")]);
    let unevaluated = DysonSeries::first_order_term(&g, h, t);
    assert!(
        show(&g, unevaluated)
            != show(
                &g,
                g.mul([g.integer(-1), g.constant(algebra_core::Constant::I), h])
            ),
        "the integrand was returned in place of its integral"
    );
}

#[test]
fn ito_product_rule_carries_the_quadratic_variation_drift() {
    use cat_tangent::weil_ideal::{CalculusKind, WeilJet};
    // W is a Brownian path: dW has unit tangent. d(W^2) = 2 W dW + dt.
    let w = WeilJet::var(0.7, CalculusKind::Ito);
    let square = w.mul(&w);
    assert!((square.tangent - 1.4).abs() < 1e-12);
    assert!((square.ito_drift() - 1.0).abs() < 1e-12);
    // d(W^3) = 3 W^2 dW + 3 W dt: multiply once more.
    let cube = square.mul(&w);
    assert!(
        (cube.ito_drift() - 3.0 * 0.7).abs() < 1e-12,
        "{}",
        cube.ito_drift()
    );
    // No drift in ordinary or Stratonovich calculus.
    let smooth = WeilJet::var(0.7, CalculusKind::Continuous);
    assert_eq!(smooth.mul(&smooth).ito_drift(), 0.0);
}
