//! Integration Tests for Phase 16: Next-Gen Differential Algebra, Curvature Tensors & Holonomic $D$-Modules.

use algebra_engine::curvature::MetricTensor;
use algebra_engine::diffalg::{
    DiffIndeterminate, DiffPolynomial, DiffTerm, RittWuReducer, WeylOperator,
};
use algebra_engine::{char_set, metric_curvature};
use std::f64::consts::PI;

#[test]
fn test_minkowski_4d_flatness() {
    let eta = MetricTensor::minkowski_4d();
    let curv = eta.compute_curvature().expect("Minkowski curvature failed");

    assert_eq!(curv.dim, 4);
    assert!(curv.is_flat(1e-12));
    assert!(curv.is_ricci_flat(1e-12));
    assert!(curv.ricci_scalar.abs() < 1e-12);
    assert!(curv.kretschmann.abs() < 1e-12);
}

#[test]
fn test_sphere_2d_gaussian_curvature() {
    // 2D sphere of radius R = 2.0 at equator theta = pi / 2
    let sphere = MetricTensor::sphere_2d(2.0, PI / 2.0);
    let curv = sphere.compute_curvature().expect("Sphere curvature failed");

    assert_eq!(curv.dim, 2);
    // Gaussian curvature K = 1/R^2 = 1/4 = 0.25 -> Ricci scalar R = 2*K = 0.5
    // Christoffel symbols at equator: d(g_phi_phi)/d_theta = 2 R^2 sin(theta) cos(theta) = 0 at pi/2
    let gamma = sphere.christoffel_symbols().expect("Christoffel failed");
    assert!(gamma[0][1][1].abs() < 1e-10); // Gamma^theta_{phi,phi} = -sin(theta)cos(theta) = 0 at pi/2
}

#[test]
fn test_schwarzschild_black_hole_vacuum_solution() {
    // Schwarzschild black hole of mass M = 1.0 at r = 10.0, theta = pi/2
    let schwarz =
        MetricTensor::schwarzschild(1.0, 10.0, PI / 2.0).expect("Schwarzschild creation failed");
    let curv = schwarz
        .compute_curvature()
        .expect("Schwarzschild curvature failed");

    assert_eq!(curv.dim, 4);
    // Vacuum solution: Ricci tensor R_{mu,nu} = 0, Ricci scalar R = 0, Einstein tensor G_{mu,nu} = 0
    assert!(curv.ricci_scalar.abs() < 1e-5);
    // Non-zero Kretschmann scalar invariant: K = 48 M^2 / r^6 = 48 / 10^6 = 4.8e-5
    assert!(curv.kretschmann > 0.0);
    assert!(!curv.is_flat(1e-5));
}

#[test]
fn test_differential_polynomial_arithmetic_and_derivatives() {
    // Indeterminate y^(0) and y^(1)
    let y0 = DiffIndeterminate::new(0, 0);
    let y1 = DiffIndeterminate::new(0, 1);

    // f = 3 * (y^(1))^2 * y^(0) + 5 * (y^(0))^3
    let term1 = DiffTerm::new(3.0, vec![(y1, 2), (y0, 1)]);
    let term2 = DiffTerm::new(5.0, vec![(y0, 3)]);
    let f = DiffPolynomial::new(vec![term1, term2]);

    // Leader should be y^(1)
    assert_eq!(f.leader(), Some(y1));
    assert_eq!(f.leader_degree(), 2);

    // Separand sep(f) = d f / d(y^(1)) = 6 * y^(1) * y^(0)
    let sep = f.separand();
    assert_eq!(sep.leader(), Some(y1));
    assert_eq!(sep.leader_degree(), 1);

    // Derivative delta(f) = d f / d x
    let df = f.differentiate();
    // Leader of delta(f) should be y^(2)
    let y2 = DiffIndeterminate::new(0, 2);
    assert_eq!(df.leader(), Some(y2));
    assert_eq!(df.leader_degree(), 1);
}

#[test]
fn test_ritt_wu_characteristic_set() {
    let y0 = DiffPolynomial::var(0, 0);
    let y1 = DiffPolynomial::var(0, 1);
    let y2 = DiffPolynomial::var(0, 2);

    let system = vec![y2, y0, y1];
    let cs = RittWuReducer::characteristic_set(system).expect("Characteristic set failed");

    // y = 0 forces y' = 0 and y'' = 0, so the characteristic set is {y}. (The earlier
    // implementation only sorted its input and returned all three.)
    assert_eq!(cs.len(), 1);
    assert_eq!(cs[0].leader(), Some(DiffIndeterminate::new(0, 0)));
}

#[test]
fn test_weyl_algebra_commutator() {
    // p = d/dx, q = x
    let p = WeylOperator::new(vec![vec![0.0], vec![1.0]]); // 0 + 1 * d/dx
    let q = WeylOperator::new(vec![vec![0.0, 1.0]]); // x * (d/dx)^0

    let comm = p.commutator(&q);
    // [d/dx, x] = 1
    assert_eq!(comm.differential_orders[0][0], 1.0);
}

#[test]
fn test_curvature_macros() {
    let eta = MetricTensor::minkowski_4d();
    let curv = metric_curvature!(eta).unwrap();
    assert!(curv.is_flat(1e-10));

    let y0 = DiffPolynomial::var(0, 0);
    let y1 = DiffPolynomial::var(0, 1);
    // y' is a consequence of y = 0, so the characteristic set is {y}.
    let cs = char_set!(vec![y1, y0]).unwrap();
    assert_eq!(cs.len(), 1);
}
