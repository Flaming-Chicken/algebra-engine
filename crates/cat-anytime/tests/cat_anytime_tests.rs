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

    let schedule = WavefrontSchedule::build(&diag).unwrap();
    assert!(schedule.levels.len() >= 2);

    let mut initial_wires = HashMap::new();
    initial_wires.insert(w0, ScalarAtom::Integer(3));
    initial_wires.insert(w1, ScalarAtom::Integer(4));

    let results = schedule.evaluate(initial_wires).unwrap();
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

    // Tier-0 initialisation must take at most 50 microseconds. A single wall-clock sample is at
    // the mercy of the scheduler (a loaded CI machine can stall any one call for milliseconds),
    // so judge the best of many starts: the minimum is a stable measure of the cost itself.
    let mut best_micros = u128::MAX;
    for _ in 0..200 {
        let start = Instant::now();
        let probe = AnytimeHandle::start(&term, Duration::from_millis(8));
        best_micros = best_micros.min(start.elapsed().as_micros());
        drop(probe);
    }
    assert!(
        best_micros <= 50,
        "Tier-0 anytime initialization took {best_micros} µs at best, must be <= 50 µs"
    );
    let mut handle = AnytimeHandle::start(&term, Duration::from_millis(8));

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

#[test]
fn wavefront_levels_do_not_depend_on_edge_order() {
    // w3 = w2 * w0 is listed BEFORE the edge that produces w2.
    let mut diag = OpenHypergraph::new();
    let (w0, w1, w2, w3) = (
        diag.alloc_wire(),
        diag.alloc_wire(),
        diag.alloc_wire(),
        diag.alloc_wire(),
    );
    diag.add_input_port(w0, ObjId(0));
    diag.add_input_port(w1, ObjId(0));
    diag.add_edge(HyperEdge::binary(w2, w0, w3, HyperOp::Mul));
    diag.add_edge(HyperEdge::binary(w0, w1, w2, HyperOp::Add));

    let schedule = WavefrontSchedule::build(&diag).unwrap();
    assert_eq!(schedule.levels.len(), 2);
    assert_eq!(schedule.levels[0].edges.len(), 1);
    let mut inputs = HashMap::new();
    inputs.insert(w0, ScalarAtom::Integer(3));
    inputs.insert(w1, ScalarAtom::Integer(4));
    let out = schedule.evaluate(inputs).unwrap();
    assert_eq!(out.get(&w3), Some(&ScalarAtom::Integer(21)));
}

#[test]
fn wavefront_reports_cycles_missing_wires_and_unsupported_operations() {
    // A cycle: each edge feeds the other.
    let mut cyclic = OpenHypergraph::new();
    let (a, b) = (cyclic.alloc_wire(), cyclic.alloc_wire());
    cyclic.add_edge(HyperEdge::binary(a, a, b, HyperOp::Add));
    cyclic.add_edge(HyperEdge::binary(b, b, a, HyperOp::Add));
    assert_eq!(
        WavefrontSchedule::build(&cyclic).unwrap_err(),
        WavefrontError::Cycle
    );

    // A wire read with no value is an error, not zero.
    let mut diag = OpenHypergraph::new();
    let (x, y, z) = (diag.alloc_wire(), diag.alloc_wire(), diag.alloc_wire());
    diag.add_input_port(x, ObjId(0));
    diag.add_edge(HyperEdge::binary(x, y, z, HyperOp::Add));
    let schedule = WavefrontSchedule::build(&diag).unwrap();
    let mut only_x = HashMap::new();
    only_x.insert(x, ScalarAtom::Integer(1));
    assert_eq!(
        schedule.evaluate(only_x).unwrap_err(),
        WavefrontError::MissingWire(y)
    );

    // An operation without a scalar meaning is reported.
    let mut op = OpenHypergraph::new();
    let (p, q, r) = (op.alloc_wire(), op.alloc_wire(), op.alloc_wire());
    op.add_input_port(p, ObjId(0));
    op.add_input_port(q, ObjId(0));
    op.add_edge(HyperEdge::binary(p, q, r, HyperOp::LieBracket));
    let schedule = WavefrontSchedule::build(&op).unwrap();
    let mut inputs = HashMap::new();
    inputs.insert(p, ScalarAtom::Integer(1));
    inputs.insert(q, ScalarAtom::Integer(2));
    assert!(matches!(
        schedule.evaluate(inputs),
        Err(WavefrontError::Unsupported(_))
    ));
}

#[test]
fn wavefront_inverts_and_rejects_division_by_zero() {
    let mut diag = OpenHypergraph::new();
    let (x, y) = (diag.alloc_wire(), diag.alloc_wire());
    diag.add_input_port(x, ObjId(0));
    diag.add_edge(HyperEdge::unary(x, y, HyperOp::Inv));
    let schedule = WavefrontSchedule::build(&diag).unwrap();

    let mut two = HashMap::new();
    two.insert(x, ScalarAtom::Integer(2));
    assert_eq!(
        schedule.evaluate(two).unwrap().get(&y),
        Some(&ScalarAtom::Rational(1, 2))
    );
    let mut zero = HashMap::new();
    zero.insert(x, ScalarAtom::Zero);
    assert!(matches!(
        schedule.evaluate(zero),
        Err(WavefrontError::Arithmetic(_))
    ));
}
