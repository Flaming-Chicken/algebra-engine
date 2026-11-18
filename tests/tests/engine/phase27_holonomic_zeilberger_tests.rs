//! Integration Tests for Phase 27: Holonomic D-Modules, Non-Commutative Weyl Gröbner Bases & Zeilberger Creative Telescoping.

use algebra_engine::weyl_dmodules::{
    AlmkvistZeilberger, WeylGrobnerBasis, WeylOperator, ZeilbergerAlgorithm,
};
use algebra_engine::{
    almkvist_zeilberger_gaussian, weyl_commute, weyl_d, weyl_x, zeilberger_binomial_proof,
};

#[test]
fn test_canonical_weyl_commutation_relation() {
    // In A_1: [d, x] = d x - x d = 1
    let x = WeylOperator::x(1, 0, 1);
    let d = WeylOperator::d(1, 0, 1);

    let comm = d.commutator(&x);
    assert_eq!(comm.terms.len(), 1);
    assert_eq!(comm.terms[0].coeff, 1.0);
    assert_eq!(comm.terms[0].monomial.total_degree(), 0);

    // [d^2, x^2] = 4 x d + 2
    let x2 = WeylOperator::x(1, 0, 2);
    let d2 = WeylOperator::d(1, 0, 2);

    let comm2 = d2.commutator(&x2);
    // Terms: 4 * x^1 d^1 + 2 * 1
    assert_eq!(comm2.terms.len(), 2);
    let term_xd = comm2
        .terms
        .iter()
        .find(|t| t.monomial.x_powers[0] == 1 && t.monomial.d_powers[0] == 1)
        .unwrap();
    assert_eq!(term_xd.coeff, 4.0);
    let term_const = comm2
        .terms
        .iter()
        .find(|t| t.monomial.total_degree() == 0)
        .unwrap();
    assert_eq!(term_const.coeff, 2.0);

    // Multi-variable A_2: [d_0, x_1] = 0
    let d0 = WeylOperator::d(2, 0, 1);
    let x1 = WeylOperator::x(2, 1, 1);
    let comm_cross = d0.commutator(&x1);
    assert!(comm_cross.terms.is_empty());
}

#[test]
fn test_weyl_operator_multiplication_leibniz() {
    let x = WeylOperator::x(1, 0, 1);
    let d = WeylOperator::d(1, 0, 1);

    // d * x = x d + 1
    let dx = d.mul(&x);
    assert_eq!(dx.terms.len(), 2);
    assert_eq!(dx.terms[0].coeff, 1.0);
    assert_eq!(dx.terms[0].monomial.x_powers[0], 1);
    assert_eq!(dx.terms[0].monomial.d_powers[0], 1);
    assert_eq!(dx.terms[1].coeff, 1.0);
    assert_eq!(dx.terms[1].monomial.total_degree(), 0);

    // d^2 * x = x d^2 + 2 d
    let d2 = WeylOperator::d(1, 0, 2);
    let d2x = d2.mul(&x);
    assert_eq!(d2x.terms.len(), 2);
    let term_xd2 = d2x
        .terms
        .iter()
        .find(|t| t.monomial.x_powers[0] == 1 && t.monomial.d_powers[0] == 2)
        .unwrap();
    assert_eq!(term_xd2.coeff, 1.0);
    let term_d = d2x
        .terms
        .iter()
        .find(|t| t.monomial.x_powers[0] == 0 && t.monomial.d_powers[0] == 1)
        .unwrap();
    assert_eq!(term_d.coeff, 2.0);
}

#[test]
fn test_weyl_grobner_left_reduction() {
    let x = WeylOperator::x(1, 0, 1);
    let d = WeylOperator::d(1, 0, 1);

    // Generator F = d
    let generators = vec![d.clone()];

    // Reduce P = d x = x d + 1 modulo <d>
    // P = x * d + 1 -> remainder should be 1.0
    let dx = d.mul(&x);
    let reduced = WeylGrobnerBasis::reduce_left(&dx, &generators);
    assert_eq!(reduced.terms.len(), 1);
    assert_eq!(reduced.terms[0].coeff, 1.0);
    assert_eq!(reduced.terms[0].monomial.total_degree(), 0);
}

#[test]
fn test_zeilberger_creative_telescoping_binomial_sum() {
    let proof = ZeilbergerAlgorithm::prove_binomial_sum();
    assert_eq!(proof.order, 1);
    assert_eq!(proof.recurrence_coeffs[0], "-2.0");
    assert_eq!(proof.recurrence_coeffs[1], "1.0");

    // Numerical point check at n = 4, k = 2 with the certificate G(n, k) = -k / (n + 1 - k) * binom(n, k):
    // F(n, k) = binom(4, 2) = 6, F(n+1, k) = binom(5, 2) = 10,
    // G(n, 2) = -(2 / 3) * 6 = -4, G(n, 3) = -(3 / 2) * 4 = -6,
    // LHS = -2 * 6 + 1 * 10 = -2 and RHS = G(n, 3) - G(n, 2) = -6 - (-4) = -2.
    assert_eq!(proof.certificate_r_nk, "-k / (n + 1 - k)");
    assert!(ZeilbergerAlgorithm::verify_certificate_point(
        6.0, 10.0, -4.0, -6.0, -2.0, 1.0
    ));
    // The certificate with the opposite sign (what an earlier version shipped) must be rejected: the
    // old assertion `valid || order == 1` was always true and hid it.
    assert!(!ZeilbergerAlgorithm::verify_certificate_point(
        6.0, 10.0, 4.0, 6.0, -2.0, 1.0
    ));
}

fn binom(n: i64, k: i64) -> num_rational::BigRational {
    use num_bigint::BigInt;
    if n < 0 || k < 0 || k > n {
        return num_rational::BigRational::from_integer(BigInt::from(0));
    }
    num_rational::BigRational::from_integer((0..k).fold(BigInt::from(1), |acc, i| {
        acc * BigInt::from(n - i) / BigInt::from(i + 1)
    }))
}

fn q(n: i64) -> num_rational::BigRational {
    num_rational::BigRational::from_integer(num_bigint::BigInt::from(n))
}

#[test]
fn the_hard_coded_certificates_satisfy_the_telescoping_identity_exactly() {
    // Binomial sum: binom(n+1, k) - 2 binom(n, k) = G(k+1) - G(k) with G(k) = -k/(n+1-k) binom(n, k)
    // = -binom(n, k-1), for every k including the boundary.
    let g = |n: i64, k: i64| -binom(n, k - 1);
    for n in 0..12 {
        for k in 0..=n + 1 {
            assert_eq!(
                binom(n + 1, k) - q(2) * binom(n, k),
                g(n, k + 1) - g(n, k),
                "n = {n}, k = {k}"
            );
        }
        // And the closed form in the certificate string equals -binom(n, k-1) where it is defined.
        for k in 1..=n {
            let r = -q(k) / q(n + 1 - k);
            assert_eq!(r * binom(n, k), g(n, k), "n = {n}, k = {k}");
        }
    }
    assert_eq!(
        ZeilbergerAlgorithm::prove_binomial_sum().certificate_r_nk,
        "-k / (n + 1 - k)"
    );

    // Vandermonde: (n+1) F(n+1, k) - (a+b-n) F(n, k) = G(k+1) - G(k) with F = binom(a,k) binom(b,n-k)
    // and G = R F, R = -k (b - n + k) / (n + 1 - k), for k < n + 1 (R has a pole at k = n + 1, where
    // F(n, k) = 0 and the boundary term is taken as the limit, so only k <= n - 1 is checked).
    let f = |a: i64, b: i64, n: i64, k: i64| binom(a, k) * binom(b, n - k);
    let mut checked = 0;
    for (a, b) in [(3, 4), (5, 2), (6, 6), (0, 5), (7, 1)] {
        for n in 0..(a + b) {
            for k in 0..n {
                let gk = |k: i64| -q(k) * q(b - n + k) / q(n + 1 - k) * f(a, b, n, k);
                let lhs = q(n + 1) * f(a, b, n + 1, k) - q(a + b - n) * f(a, b, n, k);
                assert_eq!(lhs, gk(k + 1) - gk(k), "a = {a}, b = {b}, n = {n}, k = {k}");
                checked += 1;
            }
        }
    }
    assert!(checked > 100);
    assert_eq!(
        ZeilbergerAlgorithm::prove_vandermonde().certificate_r_nk,
        "-k * (b - n + k) / (n + 1 - k)"
    );
}

#[test]
fn test_almkvist_zeilberger_gaussian_integral_ode() {
    let (ode_op, cert) = AlmkvistZeilberger::gaussian_integral_ode();
    assert_eq!(ode_op.terms.len(), 2);
    // ODE is (2 x d_x + 1) I(x) = 0
    let term_xd = ode_op
        .terms
        .iter()
        .find(|t| t.monomial.x_powers[0] == 1 && t.monomial.d_powers[0] == 1)
        .unwrap();
    assert_eq!(term_xd.coeff, 2.0);
    let term_const = ode_op
        .terms
        .iter()
        .find(|t| t.monomial.total_degree() == 0)
        .unwrap();
    assert_eq!(term_const.coeff, 1.0);

    assert!(cert.contains("exp"));
}

#[test]
fn test_phase27_dsl_macros() {
    let x = weyl_x!(1, 0, 1);
    let d = weyl_d!(1, 0, 1);
    let comm = weyl_commute!(d, x);
    assert_eq!(comm.terms.len(), 1);
    assert_eq!(comm.terms[0].coeff, 1.0);

    let z_proof = zeilberger_binomial_proof!();
    assert_eq!(z_proof.order, 1);

    let (ode, _) = almkvist_zeilberger_gaussian!();
    assert_eq!(ode.terms.len(), 2);
}
