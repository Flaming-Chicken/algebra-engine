//! Integration tests for Control Systems (SISO, MIMO, Continuous, Discrete, ZOH, Ackermann, Frequency Response).

use algebra_engine::control::{StateSpaceSystem, TimeDomain};

#[test]
fn test_siso_vs_mimo_classification() {
    // SISO 2x2: 1 input, 1 output
    let a_siso = vec![vec![0.0, 1.0], vec![-2.0, -3.0]];
    let b_siso = vec![vec![0.0], vec![1.0]];
    let c_siso = vec![vec![1.0, 0.0]];
    let d_siso = vec![vec![0.0]];
    let sys_siso = StateSpaceSystem::new(a_siso, b_siso, c_siso, d_siso);

    assert!(sys_siso.is_siso());
    assert!(!sys_siso.is_mimo());
    assert!(sys_siso.is_continuous());
    assert_eq!(sys_siso.time_domain, TimeDomain::Continuous);
    assert_eq!(sys_siso.num_states(), 2);
    assert_eq!(sys_siso.num_inputs(), 1);
    assert_eq!(sys_siso.num_outputs(), 1);

    // MIMO 4-state inverted pendulum / cart: 1 input (force), 2 outputs (cart position, pendulum angle)
    let a_mimo = vec![
        vec![0.0, 1.0, 0.0, 0.0],
        vec![0.0, -0.1, 2.5, 0.0],
        vec![0.0, 0.0, 0.0, 1.0],
        vec![0.0, -0.2, 25.0, 0.0],
    ];
    let b_mimo = vec![vec![0.0], vec![1.0], vec![0.0], vec![2.0]];
    let c_mimo = vec![vec![1.0, 0.0, 0.0, 0.0], vec![0.0, 0.0, 1.0, 0.0]];
    let d_mimo = vec![vec![0.0], vec![0.0]];
    let sys_mimo = StateSpaceSystem::new(a_mimo, b_mimo, c_mimo, d_mimo);

    assert!(!sys_mimo.is_siso());
    assert!(sys_mimo.is_mimo());
    assert_eq!(sys_mimo.num_states(), 4);
    assert_eq!(sys_mimo.num_inputs(), 1);
    assert_eq!(sys_mimo.num_outputs(), 2);
}

#[test]
fn test_kalman_controllability_and_observability_rank() {
    // Classic double integrator:
    // A = [0 1; 0 0], B = [0; 1], C = [1 0]
    // C_mat = [B  AB] = [0 1; 1 0] -> rank 2 (Controllable)
    // O_mat = [C; CA] = [1 0; 0 1] -> rank 2 (Observable)
    let a = vec![vec![0.0, 1.0], vec![0.0, 0.0]];
    let b = vec![vec![0.0], vec![1.0]];
    let c = vec![vec![1.0, 0.0]];
    let d = vec![vec![0.0]];
    let sys = StateSpaceSystem::new(a, b, c, d);

    assert_eq!(sys.controllability_rank(), 2);
    assert!(sys.is_controllable());
    assert_eq!(sys.observability_rank(), 2);
    assert!(sys.is_observable());

    // Uncontrollable system: B = [0; 0]
    let b_uncontrollable = vec![vec![0.0], vec![0.0]];
    let sys_uncontrollable = StateSpaceSystem::new(
        vec![vec![0.0, 1.0], vec![0.0, 0.0]],
        b_uncontrollable,
        vec![vec![1.0, 0.0]],
        vec![vec![0.0]],
    );
    assert_eq!(sys_uncontrollable.controllability_rank(), 0);
    assert!(!sys_uncontrollable.is_controllable());
}

#[test]
fn test_zoh_discretization_double_integrator() {
    // Continuous double integrator:
    // A = [0 1; 0 0], B = [0; 1]
    // For Ts = 0.1 s:
    // Ad = [1 0.1; 0 1]
    // Bd = [0.5 * 0.1^2; 0.1] = [0.005; 0.1]
    let a = vec![vec![0.0, 1.0], vec![0.0, 0.0]];
    let b = vec![vec![0.0], vec![1.0]];
    let c = vec![vec![1.0, 0.0]];
    let d = vec![vec![0.0]];
    let sys = StateSpaceSystem::new(a, b, c, d);

    let ts = 0.1;
    let dsys = sys.discretize_zoh(ts);

    assert!(dsys.is_discrete());
    assert_eq!(dsys.sample_time(), Some(0.1));

    // Check Ad
    assert!((dsys.a[0][0] - 1.0).abs() < 1e-6);
    assert!((dsys.a[0][1] - 0.1).abs() < 1e-6);
    assert!((dsys.a[1][0] - 0.0).abs() < 1e-6);
    assert!((dsys.a[1][1] - 1.0).abs() < 1e-6);

    // Check Bd
    assert!((dsys.b[0][0] - 0.005).abs() < 1e-6);
    assert!((dsys.b[1][0] - 0.1).abs() < 1e-6);
}

#[test]
fn test_ackermann_pole_placement() {
    // Plant: A = [0 1; 0 0], B = [0; 1]
    // Desired poles: p1 = -2, p2 = -3
    // Desired char poly: (s+2)(s+3) = s^2 + 5s + 6
    // Closed loop matrix A - B*K = [0 1; -k0 -k1]
    // Char poly: s^2 + k1*s + k0 => k0 = 6, k1 = 5
    let a = vec![vec![0.0, 1.0], vec![0.0, 0.0]];
    let b = vec![vec![0.0], vec![1.0]];
    let c = vec![vec![1.0, 0.0]];
    let d = vec![vec![0.0]];
    let sys = StateSpaceSystem::new(a, b, c, d);

    let k = sys
        .pole_placement_ackermann(&[-2.0, -3.0])
        .expect("Pole placement should succeed");
    assert_eq!(k.len(), 2);
    assert!((k[0] - 6.0).abs() < 1e-6, "Expected k0 = 6.0, got {}", k[0]);
    assert!((k[1] - 5.0).abs() < 1e-6, "Expected k1 = 5.0, got {}", k[1]);
}

#[test]
fn test_frequency_response_bode_eval() {
    // 2nd order harmonic oscillator:
    // y'' + 2*zeta*wn*y' + wn^2*y = wn^2*u with wn = 2, zeta = 0.5
    // A = [0 1; -4 -2], B = [0; 4], C = [1 0], D = [0]
    // At w = 0 (DC), H(0) = C (-A)^(-1) B = 1.0 -> 0 dB, 0 deg
    let a = vec![vec![0.0, 1.0], vec![-4.0, -2.0]];
    let b = vec![vec![0.0], vec![4.0]];
    let c = vec![vec![1.0, 0.0]];
    let d = vec![vec![0.0]];
    let sys = StateSpaceSystem::new(a, b, c, d);

    let (mag_dc, phase_dc) = sys.eval_frequency_response_siso(0.001);
    assert!(mag_dc.abs() < 0.1, "Expected ~0 dB at DC, got {}", mag_dc);
    assert!(
        phase_dc.abs() < 1.0,
        "Expected ~0 deg at DC, got {}",
        phase_dc
    );

    // At resonance w = 2.0:
    let (mag_res, phase_res) = sys.eval_frequency_response_siso(2.0);
    assert!(
        mag_res.abs() < 0.5,
        "Expected ~0 dB at w=2 (Q=1), got {}",
        mag_res
    );
    assert!(
        (phase_res - (-90.0)).abs() < 1.0,
        "Expected -90 deg at resonance, got {}",
        phase_res
    );
}

#[test]
fn test_control_mimo_boundaries_and_discretization() {
    // Verify current MIMO / higher-order boundaries are explicit and deterministic
    let a_mimo = vec![
        vec![0.0, 1.0, 0.0],
        vec![0.0, 0.0, 1.0],
        vec![-1.0, -2.0, -3.0],
    ];
    let b_mimo = vec![vec![0.0, 1.0], vec![1.0, 0.0], vec![0.0, 1.0]];
    let c_mimo = vec![vec![1.0, 0.0, 0.0], vec![0.0, 1.0, 0.0]];
    let d_mimo = vec![vec![0.0, 0.0], vec![0.0, 0.0]];
    let sys = StateSpaceSystem::new(a_mimo, b_mimo, c_mimo, d_mimo);

    assert!(sys.is_mimo());
    assert_eq!(sys.controllability_rank(), 3);
    assert!(sys.is_controllable());

    // Ackermann and eval_frequency_response_siso currently restrict to 2-state SISO
    assert!(sys.pole_placement_ackermann(&[-1.0, -2.0, -3.0]).is_err());
    assert_eq!(sys.eval_frequency_response_siso(1.0), (0.0, 0.0));

    // ZOH discretization preserves dimensions and finiteness for higher-order MIMO systems
    let dsys = sys.discretize_zoh(0.05);
    assert!(dsys.is_discrete());
    assert_eq!(dsys.num_states(), 3);
    assert_eq!(dsys.num_inputs(), 2);
    assert_eq!(dsys.num_outputs(), 2);
    assert!(dsys.a.iter().flatten().all(|v| v.is_finite()));
    assert!(dsys.b.iter().flatten().all(|v| v.is_finite()));
}
