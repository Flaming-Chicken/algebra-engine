//! Special functions against tabulated values and exact identities (the previous Bessel `Y`/`K`,
//! Airy, zeta and `J` for large arguments returned wrong numbers).

use algebra_engine::numeric::{
    eval_airy_ai, eval_bessel_i, eval_bessel_j, eval_bessel_k, eval_bessel_y, eval_beta, eval_erf,
    eval_erfc, eval_log_gamma, eval_zeta,
};

fn close(got: f64, want: f64, rel: f64, what: &str) {
    let scale = want.abs().max(1e-300);
    assert!(
        (got - want).abs() <= rel * scale,
        "{what}: got {got:e}, want {want:e} (relative {:e})",
        (got - want).abs() / scale
    );
}

#[test]
fn bessel_j_matches_tables_and_the_neumann_identity() {
    close(
        eval_bessel_j(0, 1.0),
        0.765_197_686_557_966_6,
        1e-13,
        "J0(1)",
    );
    close(
        eval_bessel_j(1, 1.0),
        0.440_050_585_744_933_5,
        1e-13,
        "J1(1)",
    );
    close(
        eval_bessel_j(0, 10.0),
        -0.245_935_764_451_348_3,
        1e-12,
        "J0(10)",
    );
    close(
        eval_bessel_j(2, 5.0),
        0.046_565_116_277_752_22,
        1e-12,
        "J2(5)",
    );
    assert!(
        eval_bessel_j(0, 2.404_825_557_695_773).abs() < 1e-13,
        "first zero of J0"
    );
    close(
        eval_bessel_j(3, -2.0),
        -eval_bessel_j(3, 2.0),
        1e-14,
        "J3 is odd",
    );
    assert_eq!(eval_bessel_j(0, 0.0), 1.0);
    assert_eq!(eval_bessel_j(2, 0.0), 0.0);
    // J0^2 + 2 sum_{k >= 1} J_k^2 = 1, at small and large arguments (the old series gave -1.5e8 at 40).
    for x in [0.5, 3.0, 17.0, 40.0, 120.0, 900.0] {
        let mut total = eval_bessel_j(0, x).powi(2);
        for k in 1..(x as i64 + 80) {
            total += 2.0 * eval_bessel_j(k, x).powi(2);
        }
        assert!(
            (total - 1.0).abs() < 1e-11,
            "Neumann identity at {x}: {total}"
        );
    }
    // The Hankel expansion takes over beyond 2000 and agrees with the integral just below it.
    let (below, above) = (eval_bessel_j(0, 1999.999), eval_bessel_j(0, 2000.001));
    assert!(
        (below - above).abs() < 2.0 * 0.002 * 0.03,
        "{below} vs {above}"
    );
    assert!(eval_bessel_j(0, 1.0e5).abs() < 0.01);
}

#[test]
fn bessel_y_matches_tables_and_the_wronskian() {
    close(
        eval_bessel_y(0, 1.0),
        0.088_256_964_215_676_97,
        1e-12,
        "Y0(1)",
    );
    close(
        eval_bessel_y(1, 1.0),
        -0.781_212_821_300_288_7,
        1e-12,
        "Y1(1)",
    );
    close(
        eval_bessel_y(0, 2.0),
        0.510_375_672_649_745_1,
        1e-12,
        "Y0(2)",
    );
    close(
        eval_bessel_y(1, 2.0),
        -0.107_032_431_540_937_54,
        1e-12,
        "Y1(2)",
    );
    close(
        eval_bessel_y(0, 10.0),
        0.055_671_167_283_599_39,
        1e-11,
        "Y0(10)",
    );
    close(
        eval_bessel_y(2, 3.0),
        -0.160_400_393_484_923_74,
        1e-12,
        "Y2(3)",
    );
    assert!(eval_bessel_y(0, 0.0).is_nan() && eval_bessel_y(0, -1.0).is_nan());
    // J_{n+1} Y_n - J_n Y_{n+1} = 2 / (pi x).
    for x in [0.05, 1.0, 7.3, 50.0, 300.0, 3000.0] {
        for n in [0, 1, 3] {
            let w = eval_bessel_j(n + 1, x) * eval_bessel_y(n, x)
                - eval_bessel_j(n, x) * eval_bessel_y(n + 1, x);
            close(
                w,
                2.0 / (std::f64::consts::PI * x),
                1e-9, // Y_4(0.05) is about 1e6, so the products carry its quadrature error
                &format!("Wronskian n={n}, x={x}"),
            );
        }
    }
}

#[test]
fn bessel_k_matches_tables_and_the_wronskian() {
    close(
        eval_bessel_k(0, 1.0),
        0.421_024_438_240_708_34,
        1e-13,
        "K0(1)",
    );
    close(
        eval_bessel_k(1, 1.0),
        0.601_907_230_197_234_6,
        1e-13,
        "K1(1)",
    );
    close(
        eval_bessel_k(0, 2.0),
        0.113_893_872_749_533_44,
        1e-13,
        "K0(2)",
    );
    close(
        eval_bessel_k(1, 2.0),
        0.139_865_881_816_522_43,
        1e-13,
        "K1(2)",
    );
    close(
        eval_bessel_k(0, 0.1),
        2.427_069_024_702_016_4,
        1e-13,
        "K0(0.1)",
    );
    close(
        eval_bessel_k(2, 1.0),
        1.624_838_898_635_177_4,
        1e-13,
        "K2(1)",
    );
    assert!(eval_bessel_k(0, 0.0).is_nan());
    // I_n K_{n+1} + I_{n+1} K_n = 1 / x.
    for x in [0.01, 0.5, 3.0, 20.0, 150.0] {
        for n in [0, 1, 4] {
            let w = eval_bessel_i(n, x) * eval_bessel_k(n + 1, x)
                + eval_bessel_i(n + 1, x) * eval_bessel_k(n, x);
            close(w, 1.0 / x, 1e-11, &format!("K Wronskian n={n}, x={x}"));
        }
    }
}

#[test]
fn airy_ai_matches_tables_and_satisfies_its_equation() {
    for (x, want) in [
        (0.0, 0.355_028_053_887_817_2),
        (1.0, 0.135_292_416_312_881_4),
        (-1.0, 0.535_560_883_292_352_1),
        (2.0, 0.034_924_130_423_274_38),
        (-2.0, 0.227_407_428_201_685_57),
        (5.0, 1.083_444_281_360_7e-4),
        (-5.0, 0.350_761_009_024_114_2),
        (10.0, 1.104_753_255_289_868_7e-10),
    ] {
        close(eval_airy_ai(x), want, 1e-9, &format!("Ai({x})"));
    }
    // Ai'' = x Ai, by central differences, on both sides of the switch to the asymptotic series.
    let h = 1e-3;
    for x in [-14.0, -9.0, -3.3, 0.7, 4.2, 9.5, 11.0, 15.0] {
        let second = (eval_airy_ai(x + h) - 2.0 * eval_airy_ai(x) + eval_airy_ai(x - h)) / (h * h);
        let want = x * eval_airy_ai(x);
        assert!(
            (second - want).abs() <= 1e-4 * (want.abs() + eval_airy_ai(x).abs() + 1e-6),
            "Airy equation at {x}: {second} vs {want}"
        );
    }
    // Continuity across the asymptotic switch at |x| = 10.
    for x in [10.0, -10.0] {
        let (a, b) = (eval_airy_ai(x - 1e-9), eval_airy_ai(x + 1e-9));
        assert!(
            (a - b).abs() <= 1e-6 * a.abs().max(1e-12) + 1e-9,
            "{a} vs {b} at {x}"
        );
    }
}

#[test]
fn zeta_matches_known_values() {
    let pi = std::f64::consts::PI;
    close(eval_zeta(2.0), pi * pi / 6.0, 1e-14, "zeta(2)");
    close(eval_zeta(3.0), 1.202_056_903_159_594_2, 1e-14, "zeta(3)");
    close(eval_zeta(1.5), 2.612_375_348_685_488, 1e-13, "zeta(1.5)");
    close(eval_zeta(10.0), 1.000_994_575_127_818_1, 1e-14, "zeta(10)");
    close(eval_zeta(0.5), -1.460_354_508_809_586_8, 1e-13, "zeta(1/2)");
    close(
        eval_zeta(-0.5),
        -0.207_886_224_977_354_5,
        1e-12,
        "zeta(-1/2)",
    );
    assert_eq!(eval_zeta(0.0), -0.5);
    close(eval_zeta(-1.0), -1.0 / 12.0, 1e-14, "zeta(-1)");
    // Near the pole: 1/(s - 1) + gamma - gamma_1 (s - 1).
    close(
        eval_zeta(1.0001),
        10000.0 + 0.577_215_664_901_532_9 + 7.28158e-6,
        1e-9,
        "zeta(1.0001)",
    );
    // Taylor expansion at 0: -1/2 - (ln(2 pi) / 2) s + (zeta''(0) / 2) s^2 with zeta''(0) = -2.0064, at s = 0.01
    // (the old value was -0.0394).
    close(eval_zeta(0.01), -0.509_29, 1e-4, "zeta(0.01)");
    assert!(eval_zeta(1.0).is_infinite());
}

#[test]
fn gamma_beta_and_erf_are_accurate() {
    close(
        eval_log_gamma(100.0),
        359.134_205_369_575_4,
        1e-12,
        "lnGamma(100)",
    );
    close(
        eval_log_gamma(0.5),
        0.572_364_942_924_700_1,
        1e-12,
        "lnGamma(1/2)",
    );
    for x in [0.3, 7.5, 150.2, 1000.5] {
        close(
            eval_log_gamma(x + 1.0) - eval_log_gamma(x),
            x.ln(),
            1e-12,
            &format!("lnGamma recurrence at {x}"),
        );
    }
    close(eval_beta(2.0, 3.0), 1.0 / 12.0, 1e-13, "B(2, 3)");
    close(
        eval_beta(0.5, 0.5),
        std::f64::consts::PI,
        1e-13,
        "B(1/2, 1/2)",
    );
    // B(a, b) = B(a + 1, b) + B(a, b + 1), including arguments where Gamma overflows (B(200, 200) was NaN).
    for (a, b) in [(200.0, 200.0), (3.5, 7.2), (800.0, 1.5)] {
        let big = eval_beta(a, b);
        assert!(big.is_finite() && big > 0.0, "B({a}, {b}) = {big}");
        close(
            big,
            eval_beta(a + 1.0, b) + eval_beta(a, b + 1.0),
            1e-11,
            "beta recurrence",
        );
        close(big, eval_beta(b, a), 1e-13, "beta symmetry");
    }
    for (x, want) in [
        (0.5, 0.520_499_877_813_046_5),
        (1.0, 0.842_700_792_949_714_9),
        (2.0, 0.995_322_265_018_952_7),
        (3.0, 0.999_977_909_503_001_4),
        (-1.0, -0.842_700_792_949_714_9),
    ] {
        close(eval_erf(x), want, 1e-14, &format!("erf({x})"));
    }
    close(eval_erfc(4.0), 1.541_725_790_028_002e-8, 1e-12, "erfc(4)");
    close(eval_erfc(5.0), 1.537_459_794_428_035e-12, 1e-11, "erfc(5)");
    close(
        eval_erfc(-4.0),
        2.0 - 1.541_725_790_028_002e-8,
        1e-15,
        "erfc(-4)",
    );
    assert_eq!(eval_erfc(0.0), 1.0);
}
