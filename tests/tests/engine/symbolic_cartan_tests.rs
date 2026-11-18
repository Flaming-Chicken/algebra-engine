//! Differential forms with symbolic coefficients (ledger C5): exterior derivative, wedge,
//! interior product and Lie derivative checked by numerical evaluation at sample points.

use algebra_core::parser::ExprParser;
use algebra_core::{ExprGraph, ExprId};
use algebra_engine::cartan::FormBasis;
use algebra_engine::cartan_symbolic::SymbolicForm;

fn setup() -> (ExprGraph, Vec<ExprId>) {
    let g = ExprGraph::new();
    let coords = ["x", "y", "z"].iter().map(|n| g.symbol(n)).collect();
    (g, coords)
}

fn expr(g: &ExprGraph, s: &str) -> ExprId {
    ExprParser::new(g).parse(s).unwrap()
}

const POINTS: [[f64; 3]; 3] = [[0.7, -1.2, 0.4], [1.9, 0.3, -0.8], [-0.5, 0.9, 1.3]];

fn at(form: &SymbolicForm, g: &ExprGraph, p: &[f64; 3], blade: &[usize]) -> f64 {
    form.evaluate(g, p)
        .unwrap()
        .get(&FormBasis {
            indices: blade.to_vec(),
        })
        .copied()
        .unwrap_or(0.0)
}

#[test]
fn exterior_derivative_of_a_function_and_of_a_one_form() {
    let (g, c) = setup();
    // d(x^2 y + sin z) = 2xy dx + x^2 dy + cos z dz.
    let f = SymbolicForm::function(&c, expr(&g, "x^2 * y + sin(z)"));
    let df = f.exterior_derivative(&g).unwrap();
    for p in &POINTS {
        let [x, y, z] = *p;
        assert!((at(&df, &g, p, &[0]) - 2.0 * x * y).abs() < 1e-9);
        assert!((at(&df, &g, p, &[1]) - x * x).abs() < 1e-9);
        assert!((at(&df, &g, p, &[2]) - z.cos()).abs() < 1e-9);
    }

    // d(P dx + Q dy + R dz) is the curl: (R_y - Q_z) dy^dz + (P_z - R_x) dz^dx... in basis order
    // dx^dy, dx^dz, dy^dz the coefficients are Q_x - P_y, R_x - P_z, R_y - Q_z.
    let w = SymbolicForm::one_form(
        &c,
        &[expr(&g, "y * z"), expr(&g, "x^2 * z"), expr(&g, "x * y^2")],
    )
    .unwrap();
    let dw = w.exterior_derivative(&g).unwrap();
    for p in &POINTS {
        let [x, y, z] = *p;
        assert!((at(&dw, &g, p, &[0, 1]) - (2.0 * x * z - z)).abs() < 1e-9);
        assert!((at(&dw, &g, p, &[0, 2]) - (y * y - y)).abs() < 1e-9);
        assert!((at(&dw, &g, p, &[1, 2]) - (2.0 * x * y - x * x)).abs() < 1e-9);
    }
}

#[test]
fn d_squared_is_zero_and_the_leibniz_rule_holds() {
    let (g, c) = setup();
    let w = SymbolicForm::one_form(
        &c,
        &[
            expr(&g, "sin(x * y) + z^2"),
            expr(&g, "exp(z) * x"),
            expr(&g, "y^3 * cos(x)"),
        ],
    )
    .unwrap();
    let ddw = w
        .exterior_derivative(&g)
        .unwrap()
        .exterior_derivative(&g)
        .unwrap();
    let f = SymbolicForm::function(&c, expr(&g, "x * y * z + exp(x)"));
    let ddf = f
        .exterior_derivative(&g)
        .unwrap()
        .exterior_derivative(&g)
        .unwrap();
    for p in &POINTS {
        for (blade, v) in ddw.evaluate(&g, p).unwrap() {
            assert!(v.abs() < 1e-9, "d(dw) blade {:?} = {v}", blade.indices);
        }
        for (blade, v) in ddf.evaluate(&g, p).unwrap() {
            assert!(v.abs() < 1e-9, "d(df) blade {:?} = {v}", blade.indices);
        }
    }

    // d(a ^ b) = da ^ b - a ^ db for 1-forms a, b.
    let a = SymbolicForm::one_form(&c, &[expr(&g, "y"), expr(&g, "x * z"), expr(&g, "1")]).unwrap();
    let b =
        SymbolicForm::one_form(&c, &[expr(&g, "z^2"), expr(&g, "x"), expr(&g, "sin(y)")]).unwrap();
    let lhs = a.wedge(&g, &b).unwrap().exterior_derivative(&g).unwrap();
    let da_b = a.exterior_derivative(&g).unwrap().wedge(&g, &b).unwrap();
    let a_db = a.wedge(&g, &b.exterior_derivative(&g).unwrap()).unwrap();
    for p in &POINTS {
        let (l, r1, r2) = (
            lhs.evaluate(&g, p).unwrap(),
            da_b.evaluate(&g, p).unwrap(),
            a_db.evaluate(&g, p).unwrap(),
        );
        let blade = FormBasis {
            indices: vec![0, 1, 2],
        };
        let get =
            |m: &std::collections::BTreeMap<FormBasis, f64>| m.get(&blade).copied().unwrap_or(0.0);
        assert!((get(&l) - (get(&r1) - get(&r2))).abs() < 1e-9);
    }
}

#[test]
fn wedge_is_anticommutative_and_repeated_indices_vanish() {
    let (g, c) = setup();
    let a = SymbolicForm::one_form(&c, &[expr(&g, "x"), expr(&g, "y"), expr(&g, "z")]).unwrap();
    let b = SymbolicForm::one_form(&c, &[expr(&g, "y"), expr(&g, "z^2"), expr(&g, "x")]).unwrap();
    let ab = a.wedge(&g, &b).unwrap();
    let ba = b.wedge(&g, &a).unwrap();
    let aa = a.wedge(&g, &a).unwrap();
    for p in &POINTS {
        for blade in [[0usize, 1], [0, 2], [1, 2]] {
            assert!((at(&ab, &g, p, &blade) + at(&ba, &g, p, &blade)).abs() < 1e-9);
            assert!(at(&aa, &g, p, &blade).abs() < 1e-9);
        }
    }
    // dx ^ dy ^ dx = 0 by construction.
    let t = SymbolicForm::term(&g, &c, expr(&g, "1"), &[0, 1, 0]).unwrap();
    assert!(t.terms.is_empty());
    let flipped = SymbolicForm::term(&g, &c, expr(&g, "2"), &[1, 0]).unwrap();
    assert!((at(&flipped, &g, &POINTS[0], &[0, 1]) + 2.0).abs() < 1e-12);
}

#[test]
fn interior_product_and_cartans_formula() {
    let (g, c) = setup();
    let v = [expr(&g, "y"), expr(&g, "-x"), expr(&g, "1")]; // a rotation plus a translation
    // i_X (dx ^ dy) = X^x dy - X^y dx = y dy + x dx.
    let area = SymbolicForm::term(&g, &c, expr(&g, "1"), &[0, 1]).unwrap();
    let i = area.interior_product(&g, &v).unwrap();
    for p in &POINTS {
        assert!((at(&i, &g, p, &[0]) - p[0]).abs() < 1e-9);
        assert!((at(&i, &g, p, &[1]) - p[1]).abs() < 1e-9);
    }
    // i_X i_X = 0.
    let ii = i.interior_product(&g, &v).unwrap();
    for p in &POINTS {
        assert!(at(&ii, &g, p, &[]).abs() < 1e-9);
    }

    // The rotation field X = y d/dx - x d/dy has L_X (x^2 + y^2) = 0 (rotation invariance) and
    // L_X f = X(f) for a function. Take f = x^2 y: X(f) = y * 2xy - x * x^2.
    let r2 = SymbolicForm::function(&c, expr(&g, "x^2 + y^2"));
    let rot = [expr(&g, "y"), expr(&g, "-x"), expr(&g, "0")];
    let l = r2.lie_derivative(&g, &rot).unwrap();
    let f = SymbolicForm::function(&c, expr(&g, "x^2 * y"));
    let lf = f.lie_derivative(&g, &rot).unwrap();
    for p in &POINTS {
        let [x, y, _] = *p;
        assert!(at(&l, &g, p, &[]).abs() < 1e-9);
        assert!((at(&lf, &g, p, &[]) - (2.0 * x * y * y - x * x * x)).abs() < 1e-9);
    }

    // L_X (dx) = d(X^x) = dy for the rotation field; L_X (dy) = d(-x) = -dx.
    let dx = SymbolicForm::one_form(&c, &[expr(&g, "1"), expr(&g, "0"), expr(&g, "0")]).unwrap();
    let ldx = dx.lie_derivative(&g, &rot).unwrap();
    for p in &POINTS {
        assert!(at(&ldx, &g, p, &[0]).abs() < 1e-9);
        assert!((at(&ldx, &g, p, &[1]) - 1.0).abs() < 1e-9);
    }
}

#[test]
fn mismatched_inputs_are_errors() {
    let (g, c) = setup();
    let f = SymbolicForm::function(&c, expr(&g, "x"));
    let w = SymbolicForm::one_form(&c, &[expr(&g, "1"), expr(&g, "0"), expr(&g, "0")]).unwrap();
    assert!(f.add(&g, &w).is_err(), "degrees differ");
    assert!(SymbolicForm::one_form(&c, &[expr(&g, "1")]).is_err());
    assert!(w.interior_product(&g, &[expr(&g, "1")]).is_err());
    assert!(f.evaluate(&g, &[1.0]).is_err());
    // A symbol that is not a coordinate cannot be evaluated.
    let h = SymbolicForm::function(&c, expr(&g, "x + q"));
    assert!(h.evaluate(&g, &[1.0, 2.0, 3.0]).is_err());
}

fn run(kind: algebra_core::operation::CartanOpKind) -> String {
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
fn the_cartan_operation_accepts_symbolic_coefficients() {
    use algebra_core::operation::CartanOpKind::{
        ExteriorDerivative, InteriorProduct, LieDerivative, Wedge,
    };

    // d(x^2 y dx) = x^2 dy^dx = -x^2 dx^dy.
    let d = run(ExteriorDerivative {
        form_str: "x^2*y dx".into(),
        dim: 3,
    });
    assert!(
        d.contains("(degree 2)") && d.contains("dx∧dy") && d.contains("x"),
        "{d}"
    );
    assert!(!d.starts_with("ERROR"), "{d}");

    // d(d(f)) = 0 shows up as `0`: d of a function gives a 1-form; the 1-form's d is zero.
    let closed = run(ExteriorDerivative {
        form_str: "2*x*y dx + x^2 dy".into(),
        dim: 2,
    });
    assert!(
        closed.contains("= 0 (degree 2)"),
        "an exact form is closed: {closed}"
    );

    // A non-closed form: d(-y dx + x dy) = 2 dx^dy.
    let rot = run(ExteriorDerivative {
        form_str: "-y dx + x dy".into(),
        dim: 2,
    });
    assert!(rot.contains("2") && rot.contains("dx∧dy"), "{rot}");

    // Wedge of symbolic forms.
    let w = run(Wedge {
        form1_str: "x dx".into(),
        form2_str: "y dy".into(),
        dim: 2,
    });
    assert!(w.contains("dx∧dy") && w.contains("(degree 2)"), "{w}");

    // Lie derivative of a symbolic function along d/dx: L_X (x^2) = 2x (degree 0).
    let l = run(LieDerivative {
        form_str: "x^2".into(),
        vector_field: vec![1.0, 0.0],
        dim: 2,
    });
    assert!(
        l.contains("(degree 0)") && l.contains("2") && !l.starts_with("ERROR"),
        "{l}"
    );

    // Interior product of a symbolic 2-form.
    let i = run(InteriorProduct {
        form_str: "x dx^dy".into(),
        vector_field: vec![1.0, 0.0],
        dim: 2,
    });
    assert!(i.contains("(degree 1)") && i.contains("dy"), "{i}");

    // Constant-coefficient behaviour is unchanged, and nonsense is still an error.
    assert!(
        run(ExteriorDerivative {
            form_str: "3 dx".into(),
            dim: 3
        })
        .contains("= 0")
    );
    assert!(
        run(ExteriorDerivative {
            form_str: "x dq".into(),
            dim: 3
        })
        .starts_with("ERROR")
    );
    assert!(
        run(ExteriorDerivative {
            form_str: "x dx +".into(),
            dim: 3
        })
        .starts_with("ERROR")
    );
}

#[test]
fn symbolic_form_parsing_handles_signs_parentheses_and_degrees() {
    let g = ExprGraph::new();
    let names: Vec<String> = ["x", "y", "z"].iter().map(|s| s.to_string()).collect();
    let f = SymbolicForm::parse(&g, "sin(x - y) dx - (y + 1) dy^dz", &names);
    // Mixed degrees (1-form plus 2-form) are rejected.
    assert!(f.is_err());
    let w = SymbolicForm::parse(&g, "sin(x - y) dx - (y + 1) dz", &names).unwrap();
    assert_eq!(w.degree, 1);
    let p = [0.5, 0.2, 0.0];
    let v = w.evaluate(&g, &p).unwrap();
    assert!((v[&FormBasis { indices: vec![0] }] - (0.3_f64).sin()).abs() < 1e-12);
    assert!((v[&FormBasis { indices: vec![2] }] + 1.2).abs() < 1e-12);
    let f0 = SymbolicForm::parse(&g, "x*y", &names).unwrap();
    assert_eq!(f0.degree, 0);
    assert!(SymbolicForm::parse(&g, "", &names).is_err());
    assert!(SymbolicForm::parse(&g, "dx^", &names).is_err());
    // A term can repeat a differential: dx^dx = 0 keeps the degree.
    let zero = SymbolicForm::parse(&g, "x dx^dx", &names).unwrap();
    assert_eq!(zero.degree, 2);
    assert_eq!(zero.to_text(&g), "0");
}

#[test]
fn hodge_star_codifferential_and_the_laplacian() {
    let (g, c) = setup();

    // star(f dx) = f dy^dz, star(f dy) = -f dx^dz, star(f dz) = f dx^dy on Euclidean R^3.
    let w = SymbolicForm::one_form(&c, &[expr(&g, "x*y"), expr(&g, "z"), expr(&g, "x^2")]).unwrap();
    let s = w.hodge_star(&g, false).unwrap();
    for p in &POINTS {
        let [x, y, z] = *p;
        assert!((at(&s, &g, p, &[1, 2]) - x * y).abs() < 1e-9);
        assert!((at(&s, &g, p, &[0, 2]) + z).abs() < 1e-9);
        assert!((at(&s, &g, p, &[0, 1]) - x * x).abs() < 1e-9);
    }
    // star star = +1 on R^3 for every degree.
    let ss = s.hodge_star(&g, false).unwrap();
    for p in &POINTS {
        for blade in [[0usize], [1], [2]] {
            assert!((at(&ss, &g, p, &blade) - at(&w, &g, p, &blade)).abs() < 1e-9);
        }
    }

    // It agrees with the constant-coefficient implementation on constant forms.
    let names: Vec<String> = ["x", "y", "z"].iter().map(|s| s.to_string()).collect();
    for text in ["2 dx - 3 dz", "dx^dy + 4 dy^dz", "5"] {
        for minkowski in [false, true] {
            let constant = algebra_engine::cartan::DifferentialForm::parse(text, 3, &names)
                .unwrap()
                .hodge_star(minkowski)
                .unwrap();
            let symbolic = SymbolicForm::parse(&g, text, &names)
                .unwrap()
                .hodge_star(&g, minkowski)
                .unwrap();
            let values = symbolic.evaluate(&g, &POINTS[0]).unwrap();
            for (basis, v) in &constant.terms {
                let got = values.get(basis).copied().unwrap_or(0.0);
                assert!(
                    (got - v).abs() < 1e-12,
                    "{text} minkowski={minkowski} {basis:?}"
                );
            }
        }
    }

    // Codifferential of a 1-form is minus the divergence: delta(P dx + Q dy + R dz) = -(P_x + Q_y + R_z).
    let v = SymbolicForm::one_form(
        &c,
        &[expr(&g, "x^2"), expr(&g, "y * z"), expr(&g, "sin(z)")],
    )
    .unwrap();
    let delta = v.codifferential(&g).unwrap();
    for p in &POINTS {
        let [x, _, z] = *p;
        let div = 2.0 * x + z + z.cos();
        assert!(
            (at(&delta, &g, p, &[]) + div).abs() < 1e-9,
            "got {}",
            at(&delta, &g, p, &[])
        );
    }

    // Laplace-de Rham on functions is minus the Laplacian.
    let f = SymbolicForm::function(&c, expr(&g, "x^2 + 3*y^2 + sin(z)"));
    let lap = f.laplace_de_rham(&g).unwrap();
    for p in &POINTS {
        let want = -(2.0 + 6.0 - p[2].sin());
        assert!((at(&lap, &g, p, &[]) - want).abs() < 1e-9);
    }
    // On a 1-form it acts componentwise on a flat space: Delta(f dx) = (-lap f) dx.
    let one =
        SymbolicForm::one_form(&c, &[expr(&g, "x^3 + y^2"), expr(&g, "0"), expr(&g, "0")]).unwrap();
    let lap1 = one.laplace_de_rham(&g).unwrap();
    for p in &POINTS {
        assert!((at(&lap1, &g, p, &[0]) + (6.0 * p[0] + 2.0)).abs() < 1e-9);
        assert!(at(&lap1, &g, p, &[1]).abs() < 1e-9);
        assert!(at(&lap1, &g, p, &[2]).abs() < 1e-9);
    }
}

#[test]
fn the_cartan_operation_computes_a_symbolic_hodge_star() {
    use algebra_core::operation::CartanOpKind::HodgeStar;
    let out = run(HodgeStar {
        form_str: "x^2 dx".into(),
        dim: 3,
        is_minkowski: false,
    });
    assert!(
        out.contains("dy∧dz") && out.contains("(degree 2)") && !out.starts_with("ERROR"),
        "{out}"
    );
}

#[test]
fn juxtaposed_coefficient_factors_multiply() {
    let g = ExprGraph::new();
    let names: Vec<String> = ["x", "y", "z"].iter().map(|s| s.to_string()).collect();
    let p = [2.0, 3.0, 0.5];
    let value = |text: &str, blade: &[usize]| {
        SymbolicForm::parse(&g, text, &names)
            .unwrap()
            .evaluate(&g, &p)
            .unwrap()
            .get(&FormBasis {
                indices: blade.to_vec(),
            })
            .copied()
            .unwrap_or(0.0)
    };
    assert!((value("x^2 y dx", &[0]) - 12.0).abs() < 1e-12);
    assert!((value("2 x y dz", &[2]) - 12.0).abs() < 1e-12);
    assert!((value("(x + 1) (y - 1) dy", &[1]) - 6.0).abs() < 1e-12);
    // Functions, powers and spaced operators are not split.
    assert!((value("sin(z) dx", &[0]) - 0.5_f64.sin()).abs() < 1e-12);
    assert!((value("x + y", &[]) - 5.0).abs() < 1e-12);
    assert!((value("x ^ 2 dy", &[1]) - 4.0).abs() < 1e-12);
    assert!((value("2 (y + 1) z dx", &[0]) - 4.0).abs() < 1e-12);

    // The notebook operation now reads `x^2 y dx` too.
    use algebra_core::operation::CartanOpKind::ExteriorDerivative;
    let out = run(ExteriorDerivative {
        form_str: "2 x y dx + x^2 dy".into(),
        dim: 2,
    });
    assert!(out.contains("= 0 (degree 2)"), "{out}");
}

#[test]
fn the_cartan_operation_accepts_vector_fields_with_expression_components() {
    use algebra_core::operation::CartanOpKind::{InteriorProductSymbolic, LieDerivativeSymbolic};
    let field = |c: &[&str]| c.iter().map(|s| s.to_string()).collect::<Vec<_>>();

    // i_X(dx^dy) for the rotation field X = y d/dx - x d/dy is y dy + x dx.
    let out = run(InteriorProductSymbolic {
        form_str: "dx^dy".into(),
        vector_field: field(&["y", "-x"]),
        dim: 2,
    });
    assert!(
        !out.starts_with("ERROR") && out.contains("(degree 1)"),
        "{out}"
    );
    assert!(out.contains("dx") && out.contains("dy"), "{out}");

    // L_X (x^2 y) = X(f) = y 2xy - x x^2 (a function, degree 0); L_X(x^2 + y^2) = 0.
    let lie = run(LieDerivativeSymbolic {
        form_str: "x^2*y".into(),
        vector_field: field(&["y", "-x"]),
        dim: 2,
    });
    assert!(
        !lie.starts_with("ERROR") && lie.contains("(degree 0)"),
        "{lie}"
    );
    let invariant = run(LieDerivativeSymbolic {
        form_str: "x^2 + y^2".into(),
        vector_field: field(&["y", "-x"]),
        dim: 2,
    });
    assert!(
        invariant.contains("= 0 (degree 0)"),
        "rotation leaves x^2 + y^2 unchanged: {invariant}"
    );

    // L_X dx = d(X^x) = d(y) = dy for the rotation field, and a non-constant field works too:
    // X = x^2 d/dx gives L_X (dx) = d(x^2) = 2x dx.
    let ldx = run(LieDerivativeSymbolic {
        form_str: "dx".into(),
        vector_field: field(&["x^2", "0"]),
        dim: 2,
    });
    assert!(
        ldx.contains("dx") && ldx.contains("(degree 1)") && !ldx.starts_with("ERROR"),
        "{ldx}"
    );

    // Unreadable components and a field of the wrong length are errors.
    assert!(
        run(InteriorProductSymbolic {
            form_str: "dx".into(),
            vector_field: field(&["x +", "1"]),
            dim: 2
        })
        .starts_with("ERROR")
    );
    assert!(
        run(LieDerivativeSymbolic {
            form_str: "dx".into(),
            vector_field: field(&["1"]),
            dim: 2
        })
        .starts_with("ERROR")
    );
}

#[test]
fn hodge_star_for_a_diagonal_metric_in_curvilinear_coordinates() {
    // Spherical coordinates (r, theta, phi), metric diag(1, r^2, r^2 sin^2 theta).
    let g = ExprGraph::new();
    let c: Vec<ExprId> = ["r", "t", "p"].iter().map(|n| g.symbol(n)).collect();
    let metric = [expr(&g, "1"), expr(&g, "r^2"), expr(&g, "r^2 * sin(t)^2")];
    let point = [2.0, 0.9, 0.3]; // r, theta, phi
    let (r, th) = (point[0], point[1]);

    let dr = SymbolicForm::one_form(&c, &[expr(&g, "1"), expr(&g, "0"), expr(&g, "0")]).unwrap();
    let dth = SymbolicForm::one_form(&c, &[expr(&g, "0"), expr(&g, "1"), expr(&g, "0")]).unwrap();
    let dph = SymbolicForm::one_form(&c, &[expr(&g, "0"), expr(&g, "0"), expr(&g, "1")]).unwrap();
    let value = |f: &SymbolicForm, blade: &[usize]| at(f, &g, &point, blade);

    // star dr = r^2 sin(theta) dtheta^dphi; star dtheta = -sin(theta) dr^dphi;
    // star dphi = dr^dtheta / sin(theta).
    let s = dr.hodge_star_diagonal(&g, &metric).unwrap();
    assert!((value(&s, &[1, 2]) - r * r * th.sin()).abs() < 1e-12);
    let s = dth.hodge_star_diagonal(&g, &metric).unwrap();
    assert!((value(&s, &[0, 2]) + th.sin()).abs() < 1e-12);
    let s = dph.hodge_star_diagonal(&g, &metric).unwrap();
    assert!((value(&s, &[0, 1]) - 1.0 / th.sin()).abs() < 1e-12);

    // star star = +1 on a Riemannian 3-manifold, for a mixed form.
    let mixed =
        SymbolicForm::one_form(&c, &[expr(&g, "r"), expr(&g, "t"), expr(&g, "p*r")]).unwrap();
    let twice = mixed
        .hodge_star_diagonal(&g, &metric)
        .unwrap()
        .hodge_star_diagonal(&g, &metric)
        .unwrap();
    for blade in [[0usize], [1], [2]] {
        assert!(
            (value(&twice, &blade) - value(&mixed, &blade)).abs() < 1e-10,
            "{blade:?}"
        );
    }

    // The volume form: star 1 = r^2 sin(theta) dr^dtheta^dphi.
    let one = SymbolicForm::function(&c, expr(&g, "1"));
    let vol = one.hodge_star_diagonal(&g, &metric).unwrap();
    assert!((value(&vol, &[0, 1, 2]) - r * r * th.sin()).abs() < 1e-12);

    // The Euclidean and Minkowski metrics reproduce the existing flat Hodge star.
    let flat_coords: Vec<ExprId> = ["x", "y", "z", "w"].iter().map(|n| g.symbol(n)).collect();
    let form = SymbolicForm::one_form(
        &flat_coords,
        &[expr(&g, "x"), expr(&g, "y"), expr(&g, "z"), expr(&g, "w")],
    )
    .unwrap();
    let p4 = [0.4, 0.5, 0.6, 0.7];
    for (metric, minkowski) in [
        (
            [expr(&g, "1"), expr(&g, "1"), expr(&g, "1"), expr(&g, "1")],
            false,
        ),
        (
            [expr(&g, "-1"), expr(&g, "1"), expr(&g, "1"), expr(&g, "1")],
            true,
        ),
    ] {
        let a = form
            .hodge_star_diagonal(&g, &metric)
            .unwrap()
            .evaluate(&g, &p4)
            .unwrap();
        let b = form
            .hodge_star(&g, minkowski)
            .unwrap()
            .evaluate(&g, &p4)
            .unwrap();
        for blade in [[1usize, 2, 3], [0, 2, 3], [0, 1, 3], [0, 1, 2]] {
            let key = FormBasis {
                indices: blade.to_vec(),
            };
            let (x, y) = (
                a.get(&key).copied().unwrap_or(0.0),
                b.get(&key).copied().unwrap_or(0.0),
            );
            assert!((x - y).abs() < 1e-12, "{blade:?}: {x} vs {y}");
        }
    }
    // A wrong-length metric is an error.
    assert!(form.hodge_star_diagonal(&g, &metric[..2]).is_err());
}

#[test]
fn the_cartan_operation_takes_a_metric_for_the_hodge_star() {
    use algebra_core::operation::CartanOpKind::HodgeStarMetric;
    let metric = |m: &[&str]| m.iter().map(|s| s.to_string()).collect::<Vec<_>>();

    // Spherical coordinates (x = r, y = theta, z = phi): star dr is r^2 sin(theta) dtheta^dphi.
    let out = run(HodgeStarMetric {
        form_str: "dx".into(),
        metric: metric(&["1", "x^2", "x^2*sin(y)^2"]),
        dim: 3,
    });
    assert!(
        !out.starts_with("ERROR") && out.contains("(degree 2)") && out.contains("dy∧dz"),
        "{out}"
    );
    assert!(
        out.contains("sin") && out.contains("x"),
        "the coefficient depends on r and theta: {out}"
    );

    // The Euclidean metric gives the flat star: star dx = dy^dz.
    let flat = run(HodgeStarMetric {
        form_str: "dx".into(),
        metric: metric(&["1", "1", "1"]),
        dim: 3,
    });
    assert!(
        flat.contains("dy∧dz") && flat.contains("(degree 2)"),
        "{flat}"
    );

    // A metric of the wrong length or with an unreadable entry is an error.
    assert!(
        run(HodgeStarMetric {
            form_str: "dx".into(),
            metric: metric(&["1", "1"]),
            dim: 3
        })
        .starts_with("ERROR")
    );
    assert!(
        run(HodgeStarMetric {
            form_str: "dx".into(),
            metric: metric(&["1", "x +", "1"]),
            dim: 3
        })
        .starts_with("ERROR")
    );
}

#[test]
fn hodge_star_for_a_general_symmetric_metric() {
    let g = ExprGraph::new();
    let c: Vec<ExprId> = ["x", "y", "z"].iter().map(|n| g.symbol(n)).collect();
    let m = |rows: [[&str; 3]; 3]| -> Vec<Vec<ExprId>> {
        rows.iter()
            .map(|r| r.iter().map(|e| expr(&g, e)).collect())
            .collect()
    };
    let point = [0.7, -0.4, 1.1];
    let value = |f: &SymbolicForm, blade: &[usize]| at(f, &g, &point, blade);

    // A diagonal metric agrees with the diagonal implementation for forms of every degree.
    let diag = m([
        ["1", "0", "0"],
        ["0", "x^2", "0"],
        ["0", "0", "x^2*sin(y)^2"],
    ]);
    let diag_entries = [expr(&g, "1"), expr(&g, "x^2"), expr(&g, "x^2*sin(y)^2")];
    let forms = [
        SymbolicForm::function(&c, expr(&g, "x*y + 2")),
        SymbolicForm::one_form(&c, &[expr(&g, "x"), expr(&g, "y*z"), expr(&g, "1")]).unwrap(),
        SymbolicForm::term(&g, &c, expr(&g, "z + x"), &[0, 2]).unwrap(),
        SymbolicForm::term(&g, &c, expr(&g, "y"), &[0, 1, 2]).unwrap(),
    ];
    for f in &forms {
        let a = f.hodge_star_metric(&g, &diag).unwrap();
        let b = f.hodge_star_diagonal(&g, &diag_entries).unwrap();
        let (va, vb) = (
            a.evaluate(&g, &point).unwrap(),
            b.evaluate(&g, &point).unwrap(),
        );
        for (blade, x) in &vb {
            let y = va.get(blade).copied().unwrap_or(0.0);
            assert!(
                (x - y).abs() < 1e-10,
                "degree {} blade {:?}: {x} vs {y}",
                f.degree,
                blade.indices
            );
        }
    }

    // A constant non-diagonal metric: omega ^ star omega = <omega, omega>_g vol, with the inverse
    // metric (1/3)[[2,-1,0],[-1,2,0],[0,0,3]] and vol = sqrt(det g) = sqrt 3.
    let metric = m([["2", "1", "0"], ["1", "2", "0"], ["0", "0", "1"]]);
    let sqrt3 = 3.0_f64.sqrt();
    let dx = SymbolicForm::one_form(&c, &[expr(&g, "1"), expr(&g, "0"), expr(&g, "0")]).unwrap();
    let star = dx.hodge_star_metric(&g, &metric).unwrap();
    assert!((value(&dx.wedge(&g, &star).unwrap(), &[0, 1, 2]) - (2.0 / 3.0) * sqrt3).abs() < 1e-10);
    let mixed = SymbolicForm::one_form(&c, &[expr(&g, "1"), expr(&g, "2"), expr(&g, "0")]).unwrap();
    let star = mixed.hodge_star_metric(&g, &metric).unwrap();
    assert!((value(&mixed.wedge(&g, &star).unwrap(), &[0, 1, 2]) - 2.0 * sqrt3).abs() < 1e-10);
    let area = SymbolicForm::term(&g, &c, expr(&g, "1"), &[0, 1]).unwrap();
    let star = area.hodge_star_metric(&g, &metric).unwrap();
    assert!((value(&area.wedge(&g, &star).unwrap(), &[0, 1, 2]) - sqrt3 / 3.0).abs() < 1e-10);
    // star star = +1 on a Riemannian 3-manifold.
    for f in [&mixed, &area] {
        let twice = f
            .hodge_star_metric(&g, &metric)
            .unwrap()
            .hodge_star_metric(&g, &metric)
            .unwrap();
        let (vt, vf) = (
            twice.evaluate(&g, &point).unwrap(),
            f.evaluate(&g, &point).unwrap(),
        );
        for (blade, x) in &vf {
            assert!((vt.get(blade).copied().unwrap_or(0.0) - x).abs() < 1e-10);
        }
    }

    // A position-dependent non-diagonal metric in the plane: g = [[1 + x^2, x y], [x y, 1 + y^2]],
    // det = 1 + x^2 + y^2 and g^{00} = (1 + y^2) / det.
    let plane: Vec<ExprId> = ["x", "y"].iter().map(|n| g.symbol(n)).collect();
    let g2 = vec![
        vec![expr(&g, "1 + x^2"), expr(&g, "x*y")],
        vec![expr(&g, "x*y"), expr(&g, "1 + y^2")],
    ];
    let dx2 = SymbolicForm::one_form(&plane, &[expr(&g, "1"), expr(&g, "0")]).unwrap();
    let wedge = dx2
        .wedge(&g, &dx2.hodge_star_metric(&g, &g2).unwrap())
        .unwrap();
    let p = [0.7, -0.4];
    let det = 1.0 + p[0] * p[0] + p[1] * p[1];
    let got = wedge.evaluate(&g, &p).unwrap()[&FormBasis {
        indices: vec![0, 1],
    }];
    assert!((got - (1.0 + p[1] * p[1]) / det * det.sqrt()).abs() < 1e-10);

    // Shape errors.
    assert!(dx.hodge_star_metric(&g, &metric[..2]).is_err());
    assert!(
        dx.hodge_star_metric(
            &g,
            &[
                vec![expr(&g, "1")],
                vec![expr(&g, "1")],
                vec![expr(&g, "1")]
            ]
        )
        .is_err()
    );
}

#[test]
fn the_cartan_operation_takes_a_full_metric_matrix_for_the_hodge_star() {
    use algebra_core::operation::CartanOpKind::HodgeStarMatrix;
    let matrix = |rows: &[&[&str]]| -> Vec<Vec<String>> {
        rows.iter()
            .map(|r| r.iter().map(|s| s.to_string()).collect())
            .collect()
    };
    // g = [[2, 1], [1, 2]] (det 3): star dx = sqrt 3 (g^{00} dy - g^{01} dx...) i.e. a 1-form in 2-D.
    let out = run(HodgeStarMatrix {
        form_str: "dx".into(),
        metric: matrix(&[&["2", "1"], &["1", "2"]]),
        dim: 2,
    });
    assert!(
        !out.starts_with("ERROR") && out.contains("(degree 1)"),
        "{out}"
    );
    assert!(
        out.contains("dx") && out.contains("dy"),
        "a non-diagonal metric mixes dx and dy: {out}"
    );

    // With the identity matrix it is the flat star: star dx = dy in the plane.
    let flat = run(HodgeStarMatrix {
        form_str: "dx".into(),
        metric: matrix(&[&["1", "0"], &["0", "1"]]),
        dim: 2,
    });
    let flat_result = flat.split_once(" = ").map(|(_, r)| r).unwrap_or(&flat);
    assert!(
        flat_result.contains("dy") && !flat_result.contains("dx"),
        "{flat}"
    );

    // A non-square or wrong-sized matrix and an unreadable entry are errors.
    assert!(
        run(HodgeStarMatrix {
            form_str: "dx".into(),
            metric: matrix(&[&["1", "0"]]),
            dim: 2
        })
        .starts_with("ERROR")
    );
    assert!(
        run(HodgeStarMatrix {
            form_str: "dx".into(),
            metric: matrix(&[&["1", "x +"], &["0", "1"]]),
            dim: 2
        })
        .starts_with("ERROR")
    );
}
