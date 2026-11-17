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
