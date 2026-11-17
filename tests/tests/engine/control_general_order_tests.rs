//! Control systems of any order: stability, frequency response and pole placement are checked
//! against systems whose poles are known by construction (companion matrices).

use algebra_engine::control::StateSpaceSystem;

/// Real roots, quadratic factors `s^2 + p s + q`, and whether the system should be stable.
type Case = (&'static [f64], &'static [(f64, f64)], bool);

/// Coefficients (highest power first) of the monic polynomial with the given real roots times
/// quadratic factors `s^2 + p s + q`.
fn polynomial(real_roots: &[f64], quadratics: &[(f64, f64)]) -> Vec<f64> {
    let mut poly = vec![1.0];
    let mut multiply = |factor: &[f64]| {
        let mut out = vec![0.0; poly.len() + factor.len() - 1];
        for (i, a) in poly.iter().enumerate() {
            for (j, b) in factor.iter().enumerate() {
                out[i + j] += a * b;
            }
        }
        poly = out;
    };
    for &r in real_roots {
        multiply(&[1.0, -r]);
    }
    for &(p, q) in quadratics {
        multiply(&[1.0, p, q]);
    }
    poly
}

/// Controllable companion form of a monic polynomial: x' = A x + B u, y = x_1.
fn companion(poly: &[f64], discrete: bool) -> StateSpaceSystem {
    let n = poly.len() - 1;
    let mut a = vec![vec![0.0; n]; n];
    for i in 0..n - 1 {
        a[i][i + 1] = 1.0;
    }
    for j in 0..n {
        a[n - 1][j] = -poly[n - j];
    }
    let mut b = vec![vec![0.0]; n];
    b[n - 1][0] = 1.0;
    let mut c = vec![vec![0.0; n]];
    c[0][0] = 1.0;
    let d = vec![vec![0.0]];
    if discrete {
        StateSpaceSystem::new_discrete(a, b, c, d, 0.1)
    } else {
        StateSpaceSystem::new(a, b, c, d)
    }
}

#[test]
fn continuous_stability_follows_the_roots_at_every_order() {
    // (real roots, quadratic factors s^2 + p s + q, expected stable)
    let cases: [Case; 9] = [
        (&[-1.0, -2.0, -3.0], &[], true),
        // Trace is -9 < 0, yet one root is in the right half-plane.
        (&[-5.0, -5.0, 1.0], &[], false),
        (&[-1.0, -2.0, -3.0, -4.0], &[], true),
        (&[-1.0, -2.0, -3.0, 0.5], &[], false),
        (&[-3.0], &[(2.0, 5.0)], true),   // -1 +- 2j
        (&[-3.0], &[(-2.0, 5.0)], false), // 1 +- 2j
        (&[0.0, -1.0, -2.0], &[], false), // marginal
        (&[-1.0, -2.0], &[(0.5, 4.0), (1.0, 9.0)], true),
        (&[-1.0, -2.0], &[(-0.5, 4.0), (1.0, 9.0)], false),
    ];
    for (real, quadratic, stable) in cases {
        let system = companion(&polynomial(real, quadratic), false);
        assert_eq!(
            system.is_stable(),
            stable,
            "roots {real:?} and factors {quadratic:?}"
        );
    }
}

#[test]
fn discrete_stability_follows_the_roots_at_every_order() {
    let cases: [Case; 6] = [
        (&[0.5, -0.3, 0.9], &[], true),
        // Trace 2.2 < 3, yet one root lies outside the unit circle.
        (&[0.5, 0.5, 1.2], &[], false),
        (&[0.5, 0.2, -0.9, 0.1], &[], true),
        (&[0.5, 0.2, -0.9, -1.05], &[], false),
        (&[0.4], &[(-0.8, 0.5)], true),  // 0.4 +- 0.58j
        (&[0.4], &[(-1.6, 1.4)], false), // 0.8 +- 0.88j, modulus 1.18
    ];
    for (real, quadratic, stable) in cases {
        let system = companion(&polynomial(real, quadratic), true);
        assert_eq!(
            system.is_stable(),
            stable,
            "roots {real:?} and factors {quadratic:?}"
        );
    }
}

#[test]
fn characteristic_polynomial_recovers_the_construction() {
    let poly = polynomial(&[-1.0, -2.0, 0.5], &[(2.0, 5.0)]);
    let system = companion(&poly, false);
    let found = StateSpaceSystem::characteristic_polynomial(&system.a);
    assert_eq!(found.len(), poly.len());
    for (a, b) in found.iter().zip(&poly) {
        assert!((a - b).abs() < 1e-9, "{found:?} vs {poly:?}");
    }
}

#[test]
fn frequency_response_of_a_three_state_system_matches_the_partial_fractions() {
    // H(s) = 1/(s+1) + 1/(s+2) + 1/(s+3) for A = diag(-1,-2,-3), B = C = ones.
    let system = StateSpaceSystem::new(
        vec![
            vec![-1.0, 0.0, 0.0],
            vec![0.0, -2.0, 0.0],
            vec![0.0, 0.0, -3.0],
        ],
        vec![vec![1.0], vec![1.0], vec![1.0]],
        vec![vec![1.0, 1.0, 1.0]],
        vec![vec![0.0]],
    );
    for omega in [0.0_f64, 0.7, 1.5, 12.0] {
        let (mut re, mut im) = (0.0, 0.0);
        for p in [1.0, 2.0, 3.0] {
            let denominator = p * p + omega * omega;
            re += p / denominator;
            im -= omega / denominator;
        }
        let (db, phase) = system.eval_frequency_response_siso(omega);
        assert!(
            (db - 20.0 * re.hypot(im).log10()).abs() < 1e-9,
            "magnitude at {omega}: {db}"
        );
        assert!(
            (phase - im.atan2(re).to_degrees()).abs() < 1e-9,
            "phase at {omega}: {phase}"
        );
    }
}

#[test]
fn mimo_frequency_response_has_one_entry_per_channel() {
    // Two decoupled first-order channels: y1 = u1/(s+1), y2 = 2 u2/(s+4).
    let system = StateSpaceSystem::new(
        vec![vec![-1.0, 0.0], vec![0.0, -4.0]],
        vec![vec![1.0, 0.0], vec![0.0, 2.0]],
        vec![vec![1.0, 0.0], vec![0.0, 1.0]],
        vec![vec![0.0, 0.0], vec![0.0, 0.0]],
    );
    let h = system.frequency_response(3.0).unwrap();
    assert_eq!(h.len(), 2);
    assert_eq!(h[0].len(), 2);
    let expected_11 = 20.0 * (1.0 / (1.0_f64 + 9.0).sqrt()).log10();
    let expected_22 = 20.0 * (2.0 / (16.0_f64 + 9.0).sqrt()).log10();
    assert!((h[0][0].0 - expected_11).abs() < 1e-9, "{:?}", h[0][0]);
    assert!((h[1][1].0 - expected_22).abs() < 1e-9, "{:?}", h[1][1]);
    // No cross-coupling: the off-diagonal channels are at the floor.
    assert!(h[0][1].0 < -200.0 && h[1][0].0 < -200.0);
}

#[test]
fn ackermann_places_the_requested_poles_at_any_order() {
    // An unstable, controllable 4-state plant, and a target pole set.
    let plant = companion(&polynomial(&[1.0, 2.0, -0.5, 3.0], &[]), false);
    let targets = [-1.0, -2.0, -3.0, -4.0];
    let gain = plant.pole_placement_ackermann(&targets).unwrap();
    assert_eq!(gain.len(), 4);

    // Closed loop A - B K, its characteristic polynomial must be prod (s - p_i).
    let n = 4;
    let mut closed = plant.a.clone();
    for j in 0..n {
        closed[n - 1][j] -= plant.b[n - 1][0] * gain[j];
    }
    let found = StateSpaceSystem::characteristic_polynomial(&closed);
    let wanted = polynomial(&targets, &[]);
    for (a, b) in found.iter().zip(&wanted) {
        assert!((a - b).abs() < 1e-8, "{found:?} vs {wanted:?}");
    }

    // An uncontrollable system is refused.
    let uncontrollable = StateSpaceSystem::new(
        vec![
            vec![-1.0, 0.0, 0.0],
            vec![0.0, -2.0, 0.0],
            vec![0.0, 0.0, -3.0],
        ],
        vec![vec![1.0], vec![1.0], vec![0.0]],
        vec![vec![1.0, 0.0, 0.0]],
        vec![vec![0.0]],
    );
    assert!(
        uncontrollable
            .pole_placement_ackermann(&[-1.0, -2.0, -3.0])
            .is_err()
    );
}
