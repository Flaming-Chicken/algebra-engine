//! Polynomial systems whose triangular basis has degree >= 2 in intermediate variables (ledger `C16`): the solver
//! used to back-substitute only when a variable's leading monomial had degree one and silently returned no
//! solutions otherwise (`x^2 - y = 0, y^2 - 1 = 0` has four).

use algebra_core::ExprGraph;
use algebra_core::parser::ExprParser;
use algebra_engine::poly::roots::ComplexRoot;
use algebra_engine::systems::polynomial::PolynomialSystemSolver;

fn solve(equations: &[&str], vars: &[&str]) -> Vec<Vec<ComplexRoot>> {
    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let ids: Vec<_> = equations
        .iter()
        .map(|e| parser.parse(e).expect("parse"))
        .collect();
    let names: Vec<String> = vars.iter().map(|v| v.to_string()).collect();
    PolynomialSystemSolver::solve(&graph, &ids, &names).expect("a solution set")
}

fn near(z: &ComplexRoot, re: f64, im: f64) -> bool {
    (z.re - re).abs() < 1e-8 && (z.im - im).abs() < 1e-8
}

fn contains(solutions: &[Vec<ComplexRoot>], point: &[(f64, f64)]) -> bool {
    solutions
        .iter()
        .any(|s| s.iter().zip(point).all(|(z, (re, im))| near(z, *re, *im)))
}

#[test]
fn a_quadratic_chain_branches_over_every_root() {
    // x^2 = y and y^2 = 1: y = 1 gives x = +-1, y = -1 gives x = +-i.
    let s = solve(&["x^2 - y", "y^2 - 1"], &["x", "y"]);
    assert_eq!(s.len(), 4, "{s:?}");
    for point in [
        [(1.0, 0.0), (1.0, 0.0)],
        [(-1.0, 0.0), (1.0, 0.0)],
        [(0.0, 1.0), (-1.0, 0.0)],
        [(0.0, -1.0), (-1.0, 0.0)],
    ] {
        assert!(contains(&s, &point), "missing {point:?} in {s:?}");
    }
}

#[test]
fn circle_meets_line_and_a_three_variable_symmetric_system() {
    let h = 0.5_f64.sqrt();
    let s = solve(&["x^2 + y^2 - 1", "x - y"], &["x", "y"]);
    assert_eq!(s.len(), 2, "{s:?}");
    assert!(contains(&s, &[(h, 0.0), (h, 0.0)]) && contains(&s, &[(-h, 0.0), (-h, 0.0)]));

    // x + y + z = 6, xy + yz + zx = 11, xyz = 6: the six permutations of (1, 2, 3).
    let s = solve(
        &["x + y + z - 6", "x*y + y*z + z*x - 11", "x*y*z - 6"],
        &["x", "y", "z"],
    );
    assert_eq!(s.len(), 6, "{s:?}");
    for perm in [
        [1.0, 2.0, 3.0],
        [1.0, 3.0, 2.0],
        [2.0, 1.0, 3.0],
        [2.0, 3.0, 1.0],
        [3.0, 1.0, 2.0],
        [3.0, 2.0, 1.0],
    ] {
        assert!(
            contains(&s, &[(perm[0], 0.0), (perm[1], 0.0), (perm[2], 0.0)]),
            "missing {perm:?}"
        );
    }
}

#[test]
fn nested_radicals_satisfy_their_equations() {
    // x^2 = 2 and y^2 = x: four solutions; check the equations rather than a closed form.
    let s = solve(&["x^2 - 2", "y^2 - x"], &["x", "y"]);
    assert_eq!(s.len(), 4, "{s:?}");
    for point in &s {
        let (x, y) = (&point[0], &point[1]);
        // |x^2 - 2| and |y^2 - x| in complex arithmetic.
        let (x2_re, x2_im) = (x.re * x.re - x.im * x.im, 2.0 * x.re * x.im);
        let (y2_re, y2_im) = (y.re * y.re - y.im * y.im, 2.0 * y.re * y.im);
        assert!(
            (x2_re - 2.0).abs() < 1e-8 && x2_im.abs() < 1e-8,
            "{point:?}"
        );
        assert!(
            (y2_re - x.re).abs() < 1e-8 && (y2_im - x.im).abs() < 1e-8,
            "{point:?}"
        );
    }
}

#[test]
fn degenerate_systems_are_errors_or_empty_not_wrong() {
    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let names = vec!["x".to_string(), "y".to_string()];
    // A line in the plane is a positive-dimensional variety.
    let line = vec![parser.parse("x - y").unwrap()];
    assert!(PolynomialSystemSolver::solve(&graph, &line, &names).is_err());
    // x = 1 and x = 2 are inconsistent.
    let clash = vec![
        parser.parse("x - 1").unwrap(),
        parser.parse("x - 2").unwrap(),
    ];
    assert!(PolynomialSystemSolver::solve(&graph, &clash, &names).is_err());
}
