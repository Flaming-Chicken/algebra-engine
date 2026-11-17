use cat_anytime::*;
use cat_core::ball::UniversalBall;
use cat_core::handles::{DiagramId, ObjId, ScalarId};
use cat_core::hypergraph::{HyperEdge, HyperOp, OpenHypergraph};
use cat_core::scalar::ScalarAtom;
use cat_core::term::{CatTerm, EnrichedSpine};
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[test]
fn test_wavefront_parallel_hypergraph_evaluation() {
    let mut diag = OpenHypergraph::new();
    let w0 = diag.alloc_wire();
    let w1 = diag.alloc_wire();
    let w2 = diag.alloc_wire();
    let w3 = diag.alloc_wire();

    diag.add_input_port(w0, ObjId(0));
    diag.add_input_port(w1, ObjId(0));

    // Level 0: w2 = w0 + w1
    diag.add_edge(HyperEdge::binary(w0, w1, w2, HyperOp::Add));
    // Level 1: w3 = w2 * w0
    diag.add_edge(HyperEdge::binary(w2, w0, w3, HyperOp::Mul));

    let schedule = WavefrontSchedule::build(&diag);
    assert!(schedule.levels.len() >= 2);

    let mut initial_wires = HashMap::new();
    initial_wires.insert(w0, ScalarAtom::Integer(3));
    initial_wires.insert(w1, ScalarAtom::Integer(4));

    let results = schedule.evaluate(initial_wires);
    // w2 = 3 + 4 = 7
    assert_eq!(results.get(&w2), Some(&ScalarAtom::Integer(7)));
    // w3 = 7 * 3 = 21
    assert_eq!(results.get(&w3), Some(&ScalarAtom::Integer(21)));
}

#[test]
fn test_anytime_handle_latency_and_refinement() {
    let ball = UniversalBall::complex_disk(1.0, 2.0, 0.5);
    let term = CatTerm::new(
        ObjId(0),
        EnrichedSpine::new(ScalarId(0), ball),
        DiagramId(0),
    );

    let start = Instant::now();
    let mut handle = AnytimeHandle::start(&term, Duration::from_millis(8));
    let latency = start.elapsed();

    // Verify Tier-0 latency <= 50 microseconds
    assert!(
        latency.as_micros() <= 50,
        "Tier-0 anytime initialization took {} µs, must be <= 50 µs",
        latency.as_micros()
    );

    assert_eq!(handle.current_ball.radius(), 0.5);

    // Refinement only adopts balls a re-evaluation justified: a tighter ball inside the current one.
    let mut halve = |ball: &UniversalBall| match ball {
        UniversalBall::ComplexDisk {
            center_re,
            center_im,
            radius,
        } => Some(UniversalBall::complex_disk(
            *center_re,
            *center_im,
            radius / 2.0,
        )),
        _ => None,
    };
    handle.run_to_budget(&mut halve);
    assert!(handle.current_ball.radius() < 0.5);

    // Unsound candidates are rejected: wider, not contained, or equal.
    let before = handle.current_ball.clone();
    assert!(!handle.refine_with(UniversalBall::complex_disk(1.0, 2.0, 5.0)));
    assert!(!handle.refine_with(UniversalBall::complex_disk(9.0, 9.0, 1e-9)));
    assert!(!handle.refine_with(before.clone()));
    assert_eq!(handle.current_ball, before);

    // With nothing to re-evaluate with, nothing shrinks.
    let mut fresh = AnytimeHandle::start(&term, Duration::from_millis(8));
    fresh.run_to_budget(|_| None);
    assert_eq!(fresh.current_ball.radius(), 0.5);

    // Real intervals: a contained tighter interval is adopted, a wider one is not.
    let interval = CatTerm::new(
        ObjId(0),
        EnrichedSpine::new(
            ScalarId(0),
            UniversalBall::RealInterval { min: 0.0, max: 2.0 },
        ),
        DiagramId(0),
    );
    let mut h = AnytimeHandle::start(&interval, Duration::from_millis(8));
    assert!(h.refine_with(UniversalBall::RealInterval { min: 0.5, max: 1.5 }));
    assert!(!h.refine_with(UniversalBall::RealInterval {
        min: -1.0,
        max: 3.0
    }));
}

#[test]
fn test_amitsur_levitzki_identity_zero_tester() {
    let p = 101u64;
    // 4 arbitrary 2x2 matrices over F_{101}
    let a1 = Matrix2Fp::new(p, 12, 34, 56, 78);
    let a2 = Matrix2Fp::new(p, 90, 23, 45, 67);
    let a3 = Matrix2Fp::new(p, 89, 12, 34, 56);
    let a4 = Matrix2Fp::new(p, 77, 88, 99, 11);

    // By Amitsur-Levitzki, S_4(A1, A2, A3, A4) MUST vanish identically!
    let s4 = AmitsurLevitzki::standard_polynomial_s4(&[a1, a2, a3, a4]);
    assert!(
        s4.is_zero(),
        "Standard polynomial S_4 must vanish identically on 2x2 matrices"
    );
}

#[test]
fn test_se3_rotor_1000_steps_zero_norm_drift() {
    let mut rotor = Se3Rotor::identity();
    let omega = [0.1, 0.2, -0.3];
    let v = [1.0, 0.5, -0.2];
    let dt = 0.001; // 1 ms per step

    for _ in 0..1000 {
        rotor = rotor.step(omega, v, dt);
    }

    let drift = rotor.norm_deviation();
    assert!(
        drift < 1e-15,
        "SE(3) rotor norm drift over 1000 steps was {:.3e}, must be < 1e-15",
        drift
    );
}
