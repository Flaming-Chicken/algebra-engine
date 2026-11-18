//! Integration Tests for Phase 26: Quantum Field Theory & Feynman Diagram Amplitude Calculus.

use algebra_engine::qft::{
    DiracGamma, PassarinoVeltman, SpinorHelicity, WeylSpinor, WickContraction,
};
use algebra_engine::{
    dirac_slashed_trace, dirac_trace, parke_taylor_mhv, passarino_veltman_b1, spinor_bracket_angle,
    spinor_bracket_square,
};

#[test]
fn test_dirac_gamma_traces_and_parity() {
    // Identity trace
    assert_eq!(DiracGamma::trace_gamma(&[]), 4.0);

    // 2-gamma traces: Tr[g0 g0] = 4, Tr[g1 g1] = -4, Tr[g0 g1] = 0
    assert_eq!(DiracGamma::trace_gamma(&[0, 0]), 4.0);
    assert_eq!(DiracGamma::trace_gamma(&[1, 1]), -4.0);
    assert_eq!(DiracGamma::trace_gamma(&[2, 2]), -4.0);
    assert_eq!(DiracGamma::trace_gamma(&[3, 3]), -4.0);
    assert_eq!(DiracGamma::trace_gamma(&[0, 1]), 0.0);

    // Odd-number gamma trace vanishes
    assert_eq!(DiracGamma::trace_gamma(&[0, 1, 2]), 0.0);
    assert_eq!(DiracGamma::trace_gamma(&[0]), 0.0);

    // 4-gamma trace: Tr[g0 g1 g0 g1] = 4(g01 g01 - g00 g11 + g01 g10) = 4(0 - (1)(-1) + 0) = +4
    assert_eq!(DiracGamma::trace_gamma(&[0, 1, 0, 1]), 4.0);

    // Slashed momentum trace
    let p1 = [2.0, 1.0, 0.0, 1.0];
    let p2 = [3.0, 0.0, 2.0, 1.0];
    // p1 . p2 = 2*3 - 1*0 - 0*2 - 1*1 = 6 - 1 = 5
    let tr_p1_p2 = DiracGamma::trace_slashed_product(&[p1, p2]);
    assert_eq!(tr_p1_p2, 4.0 * 5.0);
}

#[test]
fn test_spinor_helicity_brackets_and_mandelstam() {
    // Massless collinear/back-to-back 4-momenta
    let p1 = [2.0, 0.0, 0.0, 2.0];
    let p2 = [2.0, 0.0, 0.0, -2.0];

    let s1 = WeylSpinor::from_null_momentum(&p1);
    let s2 = WeylSpinor::from_null_momentum(&p2);

    let (ang12_re, ang12_im) = SpinorHelicity::angle_bracket(&s1, &s2);
    let (ang21_re, ang21_im) = SpinorHelicity::angle_bracket(&s2, &s1);

    // Antisymmetry: <1 2> = -<2 1>
    assert!((ang12_re + ang21_re).abs() < 1e-10);
    assert!((ang12_im + ang21_im).abs() < 1e-10);

    // Mandelstam s12 = (p1 + p2)^2 = (4, 0, 0, 0)^2 = 16.0 = 2 p1.p2 (the old assertion accepted any
    // finite value, which hid s = 0 for the spinor of a momentum along -z, and a wrong sign).
    let s12 = SpinorHelicity::mandelstam_s(&s1, &s2);
    assert!((s12 - 16.0).abs() < 1e-9, "s12 = {s12}");
}

#[test]
fn test_parke_taylor_tree_mhv_4gluon() {
    let p1 = [5.0, 3.0, 4.0, 0.0];
    let p2 = [5.0, -3.0, -4.0, 0.0];
    let p3 = [5.0, 4.0, -3.0, 0.0];
    let p4 = [5.0, -4.0, 3.0, 0.0];

    let s1 = WeylSpinor::from_null_momentum(&p1);
    let s2 = WeylSpinor::from_null_momentum(&p2);
    let s3 = WeylSpinor::from_null_momentum(&p3);
    let s4 = WeylSpinor::from_null_momentum(&p4);

    let spinors = vec![s1, s2, s3, s4];
    let (amp_re, amp_im) = SpinorHelicity::parke_taylor_tree_mhv(&spinors, 0, 1);
    assert!(amp_re.is_finite());
    assert!(amp_im.is_finite());
}

#[test]
fn test_passarino_veltman_1loop_reduction() {
    let a0 = PassarinoVeltman::a0(100.0, 0.001, 1.0);
    assert!(a0 > 0.0);

    let b0 = PassarinoVeltman::b0(50.0, 10.0, 10.0, 0.001, 1.0);
    assert!(b0.is_finite());

    let b1 = PassarinoVeltman::b1(50.0, 10.0, 10.0, 0.001, 1.0);
    assert!(b1.is_finite());
}

#[test]
fn test_wick_contractions_combinatorics() {
    // (2n - 1)!!
    // n = 1: 1!! = 1
    let pairs1 = WickContraction::all_pairings(1);
    assert_eq!(pairs1.len(), 1);

    // n = 2: 3!! = 3
    let pairs2 = WickContraction::all_pairings(2);
    assert_eq!(pairs2.len(), 3);

    // n = 3: 5!! = 15
    let pairs3 = WickContraction::all_pairings(3);
    assert_eq!(pairs3.len(), 15);

    // n = 4: 7!! = 105
    let pairs4 = WickContraction::all_pairings(4);
    assert_eq!(pairs4.len(), 105);
}

#[test]
fn test_phase26_dsl_macros() {
    let tr = dirac_trace!(&[0, 0]);
    assert_eq!(tr, 4.0);

    let p = [1.0, 0.0, 0.0, 0.0];
    let tr_p = dirac_slashed_trace!(&[p, p]);
    assert_eq!(tr_p, 4.0);

    let s1 = WeylSpinor::from_null_momentum(&[1.0, 0.0, 0.0, 1.0]);
    let s2 = WeylSpinor::from_null_momentum(&[1.0, 0.0, 0.0, -1.0]);

    let (ang_re, ang_im) = spinor_bracket_angle!(&s1, &s2);
    assert!(ang_re.is_finite() && ang_im.is_finite());

    let (sq_re, sq_im) = spinor_bracket_square!(&s1, &s2);
    assert!(sq_re.is_finite() && sq_im.is_finite());

    let spinors = vec![s1, s2, s1, s2];
    let (pt_re, pt_im) = parke_taylor_mhv!(&spinors, 0, 1);
    assert!(pt_re.is_finite() && pt_im.is_finite());

    let b1 = passarino_veltman_b1!(50.0, 10.0, 10.0, 0.01, 1.0);
    assert!(b1.is_finite());
}

fn minkowski_dot(p: &[f64; 4], q: &[f64; 4]) -> f64 {
    p[0] * q[0] - p[1] * q[1] - p[2] * q[2] - p[3] * q[3]
}

/// A massless momentum with the given energy along the direction.
fn null_momentum(energy: f64, direction: [f64; 3]) -> [f64; 4] {
    let norm = (direction[0].powi(2) + direction[1].powi(2) + direction[2].powi(2)).sqrt();
    [
        energy,
        energy * direction[0] / norm,
        energy * direction[1] / norm,
        energy * direction[2] / norm,
    ]
}

#[test]
fn mandelstam_invariants_equal_two_p_dot_q_for_any_direction() {
    // Includes momenta along +z, -z and the other axes (E + pz = 0 for -z used to give a zero spinor).
    let directions = [
        [0.0, 0.0, 1.0],
        [0.0, 0.0, -1.0],
        [1.0, 0.0, 0.0],
        [0.0, -1.0, 0.0],
        [1.0, 2.0, 3.0],
        [-3.0, 0.5, -2.0],
        [0.3, -0.4, -5.0],
    ];
    let energies = [1.0, 2.5, 7.0];
    for (i, d1) in directions.iter().enumerate() {
        for (j, d2) in directions.iter().enumerate() {
            let (p, q) = (
                null_momentum(energies[i % 3], *d1),
                null_momentum(energies[j % 3], *d2),
            );
            let (s1, s2) = (
                WeylSpinor::from_null_momentum(&p),
                WeylSpinor::from_null_momentum(&q),
            );
            let s = SpinorHelicity::mandelstam_s(&s1, &s2);
            let want = 2.0 * minkowski_dot(&p, &q);
            assert!(
                (s - want).abs() < 1e-9 * want.abs().max(1.0),
                "{d1:?} / {d2:?}: s = {s}, 2 p.q = {want}"
            );
        }
    }
}

#[test]
fn angle_brackets_satisfy_antisymmetry_and_the_schouten_identity() {
    let moms = [
        null_momentum(3.0, [1.0, 2.0, -1.0]),
        null_momentum(2.0, [0.0, 0.0, -1.0]),
        null_momentum(5.0, [-1.0, 0.5, 2.0]),
        null_momentum(1.5, [2.0, -2.0, 0.1]),
    ];
    let sp: Vec<WeylSpinor> = moms.iter().map(WeylSpinor::from_null_momentum).collect();
    let ang = |i: usize, j: usize| SpinorHelicity::angle_bracket(&sp[i], &sp[j]);
    let mul = |a: (f64, f64), b: (f64, f64)| (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0);
    // <ij> = -<ji>, and <ii> = 0.
    for i in 0..4 {
        let z = ang(i, i);
        assert!(z.0.abs() < 1e-12 && z.1.abs() < 1e-12);
        for j in 0..4 {
            let (a, b) = (ang(i, j), ang(j, i));
            assert!((a.0 + b.0).abs() < 1e-10 && (a.1 + b.1).abs() < 1e-10);
        }
    }
    // Schouten: <ij><kl> + <ik><lj> + <il><jk> = 0.
    let (a, b, c) = (
        mul(ang(0, 1), ang(2, 3)),
        mul(ang(0, 2), ang(3, 1)),
        mul(ang(0, 3), ang(1, 2)),
    );
    assert!((a.0 + b.0 + c.0).abs() < 1e-9 && (a.1 + b.1 + c.1).abs() < 1e-9);
}
