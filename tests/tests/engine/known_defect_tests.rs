//! Regression tests for defects tracked in `docs/PLANNING.md` (ledger ids A1-A4, A6, B1-B4, B6, B8, B5, B7, B10, B12).
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
        ExprKind::Function { name, args } => {
            let x = eval_at(g, args[0], n);
            match g.symbols.resolve(*name).as_deref() {
                Some("exp") => x.exp(),
                Some("sin") => x.sin(),
                Some("cos") => x.cos(),
                _ => x.sqrt(),
            }
        }
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

#[test]
fn forced_linear_system_includes_the_duhamel_convolution() {
    use algebra_engine::matrix::SymbolicMatrix;
    use algebra_engine::systems::differential::DifferentialSystemSolver;
    let g = ExprGraph::new();
    let t = g.symbols.get_or_intern("t");
    // y1' = -y1 + 1, y2' = -2 y2 with y(0) = 0: y1 = 1 - e^{-t}, y2 = 0.
    let a = SymbolicMatrix::new(
        2,
        2,
        vec![g.integer(-1), g.integer(0), g.integer(0), g.integer(-2)],
    )
    .unwrap();
    let y0 = [g.integer(0), g.integer(0)];
    let forcing = [g.integer(1), g.integer(0)];
    let sol =
        DifferentialSystemSolver::solve_linear_system_2x2(&g, &a, Some(&forcing), Some(&y0), t)
            .unwrap();
    for time in [0.0_f64, 0.5, 1.3, 2.0] {
        let y1 = eval_at(&g, sol[0], time);
        assert!(
            (y1 - (1.0 - (-time).exp())).abs() < 1e-9,
            "y1({time}) = {y1}: {}",
            show(&g, sol[0])
        );
        assert!(
            eval_at(&g, sol[1], time).abs() < 1e-9,
            "y2: {}",
            show(&g, sol[1])
        );
    }

    // Off-diagonal coupling: y1' = y2, y2' = -y1 + 0 with forcing (0, 1): y1 = 1 - cos t.
    let rot = SymbolicMatrix::new(
        2,
        2,
        vec![g.integer(0), g.integer(1), g.integer(-1), g.integer(0)],
    )
    .unwrap();
    let forced = [g.integer(0), g.integer(1)];
    // The fundamental matrix is written with complex exponentials here, which the real
    // evaluator above cannot check; assert that the convolution variable was integrated out.
    let sol =
        DifferentialSystemSolver::solve_linear_system_2x2(&g, &rot, Some(&forced), Some(&y0), t)
            .unwrap();
    assert!(!show(&g, sol[0]).contains("tau"), "{}", show(&g, sol[0]));
}

#[test]
fn lagrange_inversion_returns_the_series_of_the_inverse_function() {
    use algebra_engine::series::SymbolicSeries;
    let g = ExprGraph::new();
    let w = g.symbols.get_or_intern("w");
    let y = g.symbols.get_or_intern("y");
    let wv = g.symbol("w");
    let order = 12;
    type Case = (&'static str, algebra_core::ExprId, Box<dyn Fn(f64) -> f64>);
    let cases: Vec<Case> = vec![
        // f(w) = w + w^2  =>  w = (sqrt(1 + 4y) - 1) / 2
        (
            "w + w^2",
            g.add([wv, g.pow(wv, g.integer(2))]),
            Box::new(|y| ((1.0 + 4.0 * y).sqrt() - 1.0) / 2.0),
        ),
        // f(w) = sin w  =>  w = arcsin y   (w / sin w is 0/0 at w = 0)
        (
            "sin w",
            g.function("sin", [wv]),
            Box::new(|y: f64| y.asin()),
        ),
        // f(w) = e^w - 1  =>  w = ln(1 + y)
        (
            "exp(w) - 1",
            g.sub(g.function("exp", [wv]), g.integer(1)),
            Box::new(|y: f64| (1.0 + y).ln()),
        ),
        // f(w) = 2w  =>  w = y / 2
        ("2 w", g.mul([g.integer(2), wv]), Box::new(|y| y / 2.0)),
    ];
    for (name, f, inverse) in cases {
        let series = g.lagrange_inversion(f, w, y, order).unwrap();
        for at in [0.02_f64, 0.05, -0.05] {
            let got = eval_at(&g, series, at);
            assert!(
                (got - inverse(at)).abs() < 1e-9,
                "{name} at {at}: {got} vs {} ({})",
                inverse(at),
                show(&g, series)
            );
        }
    }
    // Not invertible, or not exactly representable: errors, not guesses.
    assert!(
        g.lagrange_inversion(g.pow(wv, g.integer(2)), w, y, 4)
            .is_err()
    );
    assert!(
        g.lagrange_inversion(g.add([wv, g.integer(1)]), w, y, 4)
            .is_err()
    );
    assert!(
        g.lagrange_inversion(g.function("sin", [g.mul([g.symbol("a"), wv])]), w, y, 4)
            .is_err()
    );
}

#[test]
fn frobenius_series_solve_bessel_equations_at_the_origin() {
    use algebra_engine::ode::symbolic::SymbolicOdeSolver;
    let g = ExprGraph::new();
    let x = g.symbols.get_or_intern("x");
    let xe = g.symbol("x");
    let x_squared = g.pow(xe, g.integer(2));

    // Bessel of order 1/2: x^2 y'' + x y' + (x^2 - 1/4) y = 0. The roots 1/2 and -1/2 differ
    // by an integer, but the resonance is harmless: solutions sin(x)/sqrt(x), cos(x)/sqrt(x).
    let q = g.sub(x_squared, g.rational(1, 4));
    let solutions =
        SymbolicOdeSolver::frobenius_series_at_zero(&g, g.integer(1), q, x, 14).unwrap();
    assert_eq!(solutions.len(), 2);
    for at in [0.3_f64, 0.9, 1.7] {
        let (s0, s1) = (eval_at(&g, solutions[0], at), eval_at(&g, solutions[1], at));
        assert!(
            (s0 - at.sin() / at.sqrt()).abs() < 1e-9,
            "sin at {at}: {s0}"
        );
        assert!(
            (s1 - at.cos() / at.sqrt()).abs() < 1e-9,
            "cos at {at}: {s1}"
        );
    }

    // Order 1: x^2 y'' + x y' + (x^2 - 1) y = 0. J_1 is the only series solution; the second
    // one needs a logarithm and is not returned.
    let q1 = g.sub(x_squared, g.integer(1));
    let only = SymbolicOdeSolver::frobenius_series_at_zero(&g, g.integer(1), q1, x, 14).unwrap();
    assert_eq!(only.len(), 1);
    // J_1(x) = x/2 - x^3/16 + ..., so the monic series is 2 J_1(x); check against the known value
    // J_1(1) = 0.44005058574493355.
    assert!((eval_at(&g, only[0], 1.0) / 2.0 - 0.440_050_585_744_933_55).abs() < 1e-9);

    // Euler equation x^2 y'' + (3/2 - 1) x ... : P = 1/2 gives roots 0 and 1/2 with Q = 0.
    let euler =
        SymbolicOdeSolver::frobenius_series_at_zero(&g, g.rational(3, 2), g.integer(0), x, 6)
            .unwrap();
    assert_eq!(euler.len(), 2);

    // Complex and irrational indicial roots are refused.
    assert!(
        SymbolicOdeSolver::frobenius_series_at_zero(&g, g.integer(0), g.integer(1), x, 4).is_err()
    );
    assert!(
        SymbolicOdeSolver::frobenius_series_at_zero(&g, g.integer(0), g.integer(-1), x, 4).is_err()
    );
    // Non-rational coefficients are refused.
    let p_symbolic = g.add([g.integer(1), g.symbol("a")]);
    assert!(
        SymbolicOdeSolver::frobenius_series_at_zero(&g, p_symbolic, g.integer(0), x, 4).is_err()
    );
}

#[test]
fn the_prover_uses_trigonometric_hyperbolic_and_exponential_side_relations() {
    use algebra_core::lemma_miner::FormalEquivalenceProver;
    use algebra_engine::lemma_miner::GrobnerProver;
    use algebra_engine::poly::multivariate::MonomialOrder;
    let g = ExprGraph::new();
    let prover = GrobnerProver::new(MonomialOrder::GrevLex);
    let x = g.symbol("x");
    let y = g.symbol("y");
    let f = |name: &str, a| g.function(name, [a]);
    let sq = |e| g.pow(e, g.integer(2));
    let one = g.integer(1);
    let prove = |l, r| prover.prove_equivalence(&g, l, r, &[]);

    // sin^2 + cos^2 = 1, and consequences that need the relation more than once.
    assert!(prove(g.add([sq(f("sin", x)), sq(f("cos", x))]), one));
    assert!(prove(
        g.pow(g.add([sq(f("sin", x)), sq(f("cos", x))]), g.integer(3)),
        one
    ));
    // cos(2x)-style: (cos x - sin x)(cos x + sin x) = 2 cos^2 x - 1.
    let (c, s) = (f("cos", x), f("sin", x));
    assert!(prove(
        g.mul([g.sub(c, s), g.add([c, s])]),
        g.sub(g.mul([g.integer(2), sq(c)]), one)
    ));
    // The relation is per argument: sin^2(x) + cos^2(y) is not 1.
    assert!(!prove(g.add([sq(f("sin", x)), sq(f("cos", y))]), one));

    // cosh^2 - sinh^2 = 1.
    assert!(prove(g.sub(sq(f("cosh", x)), sq(f("sinh", x))), one));
    // exp(x) exp(-x) = 1, but exp(x) exp(x) is not 1.
    assert!(prove(g.mul([f("exp", x), f("exp", g.neg(x))]), one));
    assert!(!prove(g.mul([f("exp", x), f("exp", x)]), one));

    // Non-identities stay unproven.
    assert!(!prove(
        g.add([sq(f("sin", x)), sq(f("cos", x))]),
        g.integer(2)
    ));
    assert!(!prove(f("sin", x), f("cos", x)));
}

fn shoelace(points: &[[f64; 2]]) -> f64 {
    let n = points.len();
    (0..n)
        .map(|i| points[i][0] * points[(i + 1) % n][1] - points[(i + 1) % n][0] * points[i][1])
        .sum::<f64>()
        / 2.0
}

/// Every undirected edge of a closed manifold mesh is used by exactly two triangles, once in each
/// direction.
fn assert_closed_and_oriented(mesh: &algebra_engine::cad::Mesh3D) {
    use std::collections::HashMap;
    let mut directed: HashMap<(usize, usize), usize> = HashMap::new();
    for t in &mesh.triangles {
        for k in 0..3 {
            *directed.entry((t[k], t[(k + 1) % 3])).or_insert(0) += 1;
        }
    }
    for (&(a, b), &count) in &directed {
        assert_eq!(
            count, 1,
            "edge {a}->{b} used {count} times in one direction"
        );
        assert_eq!(
            directed.get(&(b, a)),
            Some(&1),
            "edge {a}->{b} has no opposite edge"
        );
    }
}

#[test]
fn gear_meshes_are_closed_solids_with_the_right_volume_and_a_bore() {
    use algebra_engine::cad::InvoluteGear;
    for gear in [
        InvoluteGear::spur(2.0, 20, 5.0),
        InvoluteGear::helical(1.5, 17, 8.0, 20.0),
    ] {
        let mesh = gear.generate_3d_mesh();
        assert_closed_and_oriented(&mesh);

        // Volume = (outline area - bore area) * face width, identical for the helical twist.
        let outline = shoelace(&gear.generate_2d_profile(6));
        let bore = gear.bore_radius.min(0.8 * gear.root_diameter() / 2.0);
        let n = 48.0;
        let bore_area = 0.5 * n * bore * bore * (2.0 * std::f64::consts::PI / n).sin();
        let expected = (outline.abs() - bore_area) * gear.face_width;
        let volume = mesh.compute_volume();
        // A spur gear is an exact prism. A helical one is a piecewise-linear twist: the straight
        // chords between layers cut slightly inside the true helicoid, so the volume is a little
        // below (never above) area x width, by well under a percent with 8 layers.
        let tolerance = if gear.helix_angle_deg == 0.0 {
            1e-6
        } else {
            2e-3
        };
        assert!(
            (volume - expected).abs() < tolerance * expected,
            "z={}: volume {volume} vs {expected}",
            gear.teeth
        );
        if gear.helix_angle_deg != 0.0 {
            assert!(volume <= expected * (1.0 + 1e-9));
        }
        // The bore is a real hole: no vertex on the axis, and ring vertices at the bore radius.
        assert!(mesh.vertices.iter().all(|v| v[0].hypot(v[1]) > 0.5 * bore));
        assert!(
            mesh.vertices
                .iter()
                .any(|v| (v[0].hypot(v[1]) - bore).abs() < 1e-9)
        );
    }
}

#[test]
fn a_gear_without_a_bore_is_still_a_closed_solid() {
    use algebra_engine::cad::InvoluteGear;
    let mut gear = InvoluteGear::spur(2.0, 20, 5.0);
    gear.bore_radius = 0.0;
    let mesh = gear.generate_3d_mesh();
    assert_closed_and_oriented(&mesh);
    let expected = shoelace(&gear.generate_2d_profile(6)).abs() * gear.face_width;
    assert!((mesh.compute_volume() - expected).abs() < 1e-6 * expected);
}

#[test]
fn airfoil_and_spring_meshes_are_closed_solids_with_the_right_volume() {
    use algebra_engine::cad::{HelicalSpring, NacaAirfoil};
    // Airfoil: volume = section area x span (no twist by default).
    let wing = NacaAirfoil::new("2412", 1.0, 3.0);
    let mesh = wing.generate_3d_mesh();
    assert!(
        mesh.is_closed(),
        "{} boundary edges",
        mesh.boundary_edge_count()
    );
    let outline = wing.generate_2d_coordinates(20);
    let area = shoelace(&outline).abs();
    assert!(
        (mesh.compute_volume() - area * wing.span).abs() < 1e-9,
        "{} vs {}",
        mesh.compute_volume(),
        area * wing.span
    );
    // A symmetric 12% section has an area of about 0.082 chord^2 (0.6851 * t * c^2 ... roughly).
    let symmetric = NacaAirfoil::new("0012", 1.0, 1.0).generate_3d_mesh();
    assert!(symmetric.is_closed());
    assert!(
        (symmetric.compute_volume() - 0.0822).abs() < 0.002,
        "{}",
        symmetric.compute_volume()
    );

    // Spring: a tube of radius r_w along a helix of radius R: volume ~ pi r_w^2 * length.
    let spring = HelicalSpring::new(1.0, 8.0, 3.0, 5.0);
    let mesh = spring.generate_3d_mesh();
    assert!(
        mesh.is_closed(),
        "{} boundary edges",
        mesh.boundary_edge_count()
    );
    let turns = spring.active_coils;
    let length = turns * ((2.0 * std::f64::consts::PI * 4.0).powi(2) + 3.0_f64.powi(2)).sqrt();
    let expected = std::f64::consts::PI * 0.5 * 0.5 * length;
    // An octagonal cross-section (8 sides) has 0.9745 of the circle's area.
    let octagon = 0.5 * 8.0 * (2.0 * std::f64::consts::PI / 8.0).sin() / std::f64::consts::PI;
    assert!(
        (mesh.compute_volume() - expected * octagon).abs() < 0.02 * expected,
        "{} vs {}",
        mesh.compute_volume(),
        expected * octagon
    );
}

#[test]
fn the_closure_diagnostic_distinguishes_closed_open_and_misoriented_meshes() {
    use algebra_engine::cad::Mesh3D;
    // A tetrahedron with outward winding is closed.
    let mut tetra = Mesh3D::new("tetra");
    for v in [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
    ] {
        tetra.add_vertex(v);
    }
    for t in [[0, 2, 1], [0, 1, 3], [1, 2, 3], [0, 3, 2]] {
        tetra.add_triangle(t[0], t[1], t[2]);
    }
    assert!(tetra.is_closed());
    assert_eq!(tetra.boundary_edge_count(), 0);
    // Dropping a face opens three edges.
    let mut open = tetra.clone();
    open.triangles.pop();
    assert_eq!(open.boundary_edge_count(), 3);
    assert!(!open.is_closed());
    // Flipping one face leaves a consistent edge pair mismatch.
    let mut flipped = tetra.clone();
    flipped.triangles[0].swap(1, 2);
    assert!(!flipped.is_closed());
}

#[test]
fn extrusions_of_any_simple_polygon_are_closed_solids_with_exact_volume() {
    use algebra_engine::cad::Mesh3D;
    // An L-shape (non-convex): area 3 (a 2x2 square minus a 1x1 corner).
    let l_shape = [
        [0.0, 0.0],
        [2.0, 0.0],
        [2.0, 1.0],
        [1.0, 1.0],
        [1.0, 2.0],
        [0.0, 2.0],
    ];
    // A star-like concave polygon: a 4-armed arrow (square with notches), area computed below.
    let arrow = [
        [0.0, 0.0],
        [2.0, 1.0],
        [4.0, 0.0],
        [3.0, 2.0],
        [4.0, 4.0],
        [2.0, 3.0],
        [0.0, 4.0],
        [1.0, 2.0],
    ];
    for (name, outline) in [("L", l_shape.to_vec()), ("arrow", arrow.to_vec())] {
        let area = shoelace(&outline).abs();
        for reversed in [false, true] {
            let mut points = outline.clone();
            if reversed {
                points.reverse();
            }
            let mesh = Mesh3D::extrude_profile(&points, 5.0);
            assert!(
                mesh.is_closed(),
                "{name} reversed={reversed}: {} open",
                mesh.boundary_edge_count()
            );
            assert!(
                (mesh.compute_volume() - area * 5.0).abs() < 1e-9,
                "{name} reversed={reversed}: {} vs {}",
                mesh.compute_volume(),
                area * 5.0
            );
        }
    }
    assert_eq!(shoelace(&l_shape).abs(), 3.0);

    // Degenerate input gives an empty mesh instead of garbage.
    assert!(
        Mesh3D::extrude_profile(&[[0.0, 0.0], [1.0, 0.0]], 1.0)
            .triangles
            .is_empty()
    );
    assert!(
        Mesh3D::extrude_profile(&[[0.0, 0.0], [1.0, 0.0], [2.0, 0.0]], 1.0)
            .triangles
            .is_empty()
    );
    // A bow-tie (self-intersecting) polygon is rejected.
    assert!(
        Mesh3D::extrude_profile(&[[0.0, 0.0], [2.0, 2.0], [2.0, 0.0], [0.0, 2.0]], 1.0)
            .triangles
            .is_empty()
    );
}

#[test]
fn custom_shapes_convert_or_report_instead_of_becoming_a_unit_cube() {
    use algebra_engine::cad::{BoundingBox3D, CsgBooleanOp, CustomShape, Mesh3D};
    let cube = Mesh3D::cube(2.0, 2.0, 2.0);
    // A mesh converts to itself.
    let shape = CustomShape::TriMesh(cube.clone());
    assert_eq!(shape.to_tri_mesh().unwrap().vertices, cube.vertices);
    // An extrusion converts, with its real volume (area 3 x height 2).
    let l = CustomShape::Extrusion {
        profile_2d: vec![
            [0.0, 0.0],
            [2.0, 0.0],
            [2.0, 1.0],
            [1.0, 1.0],
            [1.0, 2.0],
            [0.0, 2.0],
        ],
        height: 2.0,
    };
    assert!((l.to_tri_mesh().unwrap().compute_volume() - 6.0).abs() < 1e-9);
    // A bad profile is an error.
    let bad = CustomShape::Extrusion {
        profile_2d: vec![[0.0, 0.0], [1.0, 1.0]],
        height: 1.0,
    };
    assert!(bad.to_tri_mesh().is_err());
    // A 3-D triangle polytope (a tetrahedron) converts.
    let tetra = CustomShape::PolytopeND {
        dim: 3,
        vertices: vec![
            vec![0.0, 0.0, 0.0],
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ],
        simplices: vec![vec![0, 2, 1], vec![0, 1, 3], vec![1, 2, 3], vec![0, 3, 2]],
    };
    let mesh = tetra.to_tri_mesh().unwrap();
    assert!(mesh.is_closed());
    assert!((mesh.compute_volume() - 1.0 / 6.0).abs() < 1e-12);
    // Higher-dimensional polytopes and Booleans are reported, not faked; implicit surfaces mesh.
    let four_d = CustomShape::PolytopeND {
        dim: 4,
        vertices: vec![],
        simplices: vec![],
    };
    assert!(four_d.to_tri_mesh().is_err());
    let implicit = CustomShape::ParametricImplicit {
        expr_str: "x^2 + y^2 + z^2 - 1".into(),
        bbox: BoundingBox3D::new([-2.0; 3], [2.0; 3]),
    };
    assert!(implicit.to_tri_mesh().unwrap().is_closed());
    let boolean = CustomShape::BooleanCSG {
        op: CsgBooleanOp::Union,
        left: Box::new(shape.clone()),
        right: Box::new(shape),
    };
    assert!(
        boolean
            .to_tri_mesh()
            .unwrap_err()
            .to_string()
            .contains("Boolean")
    );
}

#[test]
fn the_evaluator_reports_when_a_tier_overran_its_hard_timeout() {
    use algebra_core::ExprGraph;
    use algebra_core::parser::ExprParser;
    use algebra_engine::cat_eval::{CatEvalInput, CatExprEvaluator};
    use cat_anytime::{AnytimeBudget, MultiFidelityOp};
    use cat_core::enclosure::{EvalForm, FidelityMode};
    use std::time::Duration;

    let graph = ExprGraph::new();
    let expr = ExprParser::new(&graph).parse("(x + 1) * 2").unwrap();
    let input = CatEvalInput::new(graph, expr).with_var("x", Some(3.0));
    let evaluator = CatExprEvaluator::new();

    let generous = AnytimeBudget {
        hard_timeout: Duration::from_secs(60),
        ..AnytimeBudget::default()
    };
    let impossible = AnytimeBudget {
        hard_timeout: Duration::ZERO,
        ..AnytimeBudget::default()
    };
    for mode in [
        FidelityMode::NumericalApprox,
        FidelityMode::SymbolicApprox,
        FidelityMode::SymbolicExact,
        FidelityMode::SymbolicHybrid,
    ] {
        let within = evaluator
            .eval_with_fidelity(&input, mode, EvalForm::Canonical, &generous)
            .unwrap();
        let over = evaluator
            .eval_with_fidelity(&input, mode, EvalForm::Canonical, &impossible)
            .unwrap();
        assert!(
            !within.budget_exceeded,
            "{mode:?} flagged with a 60 s budget"
        );
        assert!(
            over.budget_exceeded,
            "{mode:?} not flagged with a zero budget"
        );
        // The budget only reports; the value is the same either way.
        assert_eq!(within.mode_used, over.mode_used);
        assert_eq!(within.value.numeric_value, over.value.numeric_value);
    }
}

#[test]
fn screws_are_closed_solids_whose_volume_matches_the_thread_geometry() {
    use algebra_engine::cad::{BoltHeadType, ThreadedScrew};
    let mut cases: Vec<(&str, ThreadedScrew)> = vec![
        ("M8 hex bolt", ThreadedScrew::metric_bolt(8.0, 1.25, 30.0)),
        ("M6 socket cap", ThreadedScrew::socket_screw(6.0, 1.0, 20.0)),
        (
            "T12 Acme lead screw",
            ThreadedScrew::lead_screw(12.0, 3.0, 60.0),
        ),
    ];
    let mut stud = ThreadedScrew::metric_bolt(10.0, 1.5, 25.0);
    stud.head_type = BoltHeadType::None;
    stud.thread_length = 0.0;
    cases.push(("plain stud", stud));

    for (name, screw) in cases {
        let mesh = screw.generate_3d_mesh();
        assert!(
            mesh.is_closed(),
            "{name}: {} boundary edges",
            mesh.boundary_edge_count()
        );

        // Independent volume: the thread region is pi * L_thread * mean(r^2) over one pitch
        // (every height sees the full period around the axis), the shank a plain cylinder,
        // the head a hexagonal or round prism.
        let r_major = screw.nominal_diameter / 2.0;
        let l_thread = screw.thread_length.min(screw.total_length);
        let samples = 200_000;
        let mean_r2 = (0..samples)
            .map(|k| {
                let r = screw.thread_radius((k as f64 + 0.5) / samples as f64 * screw.pitch);
                r * r
            })
            .sum::<f64>()
            / samples as f64;
        let thread = if l_thread > 0.0 {
            std::f64::consts::PI * l_thread * mean_r2
        } else {
            0.0
        };
        let shank = std::f64::consts::PI * r_major * r_major * (screw.total_length - l_thread);
        let d = screw.nominal_diameter;
        let head = match screw.head_type {
            BoltHeadType::Hexagonal => {
                let apothem = 0.8 * d;
                2.0 * 3.0_f64.sqrt() * apothem * apothem * 0.65 * d
            }
            BoltHeadType::SocketCap => std::f64::consts::PI * (0.75 * d).powi(2) * d,
            BoltHeadType::CountersunkFlat => std::f64::consts::PI * d * d * 0.5 * d,
            BoltHeadType::None => 0.0,
        };
        let expected = thread + shank + head;
        let volume = mesh.compute_volume();
        assert!(
            (volume - expected).abs() < 0.01 * expected,
            "{name}: volume {volume} vs {expected}"
        );
        // The solid is a real screw: wider than the root cylinder, and no wider than the head.
        assert!(volume > std::f64::consts::PI * screw.minor_radius().powi(2) * screw.total_length);
    }
}

#[test]
fn thread_profiles_are_periodic_bounded_and_differ_by_standard() {
    use algebra_engine::cad::{ThreadStandard, ThreadedScrew};
    for standard in [
        ThreadStandard::IsoMetric60,
        ThreadStandard::Acme29,
        ThreadStandard::Square,
    ] {
        let mut screw = ThreadedScrew::metric_bolt(10.0, 2.0, 20.0);
        screw.thread_standard = standard;
        let (lo, hi) = (screw.minor_radius(), 5.0);
        for k in 0..1000 {
            let u = k as f64 * 0.01;
            let r = screw.thread_radius(u);
            assert!(
                r >= lo - 1e-12 && r <= hi + 1e-12,
                "{standard:?}: r({u}) = {r}"
            );
            assert!(
                (r - screw.thread_radius(u + screw.pitch)).abs() < 1e-9,
                "periodic"
            );
        }
        // Both extremes are reached.
        let values: Vec<f64> = (0..2000)
            .map(|k| screw.thread_radius(k as f64 * 0.001))
            .collect();
        assert!(values.iter().any(|&r| (r - lo).abs() < 1e-9));
        assert!(values.iter().any(|&r| (r - hi).abs() < 1e-9));
    }
    let mut iso = ThreadedScrew::metric_bolt(10.0, 2.0, 20.0);
    let mut square = iso.clone();
    iso.thread_standard = ThreadStandard::IsoMetric60;
    square.thread_standard = ThreadStandard::Square;
    assert!(
        iso.minor_radius() < square.minor_radius(),
        "ISO threads are deeper than square"
    );
}

#[test]
fn implicit_surfaces_mesh_to_closed_solids_with_the_right_volume() {
    use algebra_engine::cad::{BoundingBox3D, CustomShape, Mesh3D};
    use std::f64::consts::PI;

    // Sphere of radius 1: f = r^2 - x^2 - y^2 - z^2 is positive inside, so normals point inward;
    // use f = x^2 + y^2 + z^2 - 1 (positive outside) for an outward-oriented solid.
    let bbox = BoundingBox3D::new([-1.5; 3], [1.5; 3]);
    let sphere = Mesh3D::from_implicit("x^2 + y^2 + z^2 - 1", &bbox, 40).unwrap();
    assert!(sphere.is_closed(), "the sphere must be a closed surface");
    let v = sphere.compute_volume();
    let exact = 4.0 / 3.0 * PI;
    assert!(
        (v - exact).abs() < 0.04 * exact,
        "sphere volume {v} vs {exact}"
    );

    // Torus (R=1, r=0.4): (sqrt(x^2+y^2)-1)^2 + z^2 - 0.16, volume 2 pi^2 R r^2.
    let bbox = BoundingBox3D::new([-1.6, -1.6, -0.6], [1.6, 1.6, 0.6]);
    let torus = Mesh3D::from_implicit("(sqrt(x^2 + y^2) - 1)^2 + z^2 - 0.16", &bbox, 60).unwrap();
    assert!(torus.is_closed(), "the torus must be a closed surface");
    let v = torus.compute_volume();
    let exact = 2.0 * PI * PI * 0.16;
    assert!(
        (v - exact).abs() < 0.05 * exact,
        "torus volume {v} vs {exact}"
    );

    // A surface that leaves the box is cut open.
    let small = BoundingBox3D::new([-0.8; 3], [0.8; 3]);
    let cut = Mesh3D::from_implicit("x^2 + y^2 + z^2 - 1", &small, 12).unwrap();
    assert!(!cut.is_closed() || cut.triangles.is_empty());

    // Bad input is an error, not a cube.
    assert!(Mesh3D::from_implicit("x +", &bbox, 10).is_err());
    assert!(Mesh3D::from_implicit("x", &bbox, 1).is_err());

    // And CustomShape routes to it.
    let shape = CustomShape::ParametricImplicit {
        expr_str: "x^2 + y^2 + z^2 - 1".into(),
        bbox: BoundingBox3D::new([-1.5; 3], [1.5; 3]),
    };
    assert!(shape.to_tri_mesh().unwrap().is_closed());
}
