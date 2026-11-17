use algebra_engine::combinatorics::{
    bell_number, catalan_number, combinations, derangements, factorial, integer_partitions,
    permutations, stirling_second_kind,
};
use algebra_engine::control::StateSpaceSystem;
use algebra_engine::numbertheory::{ContinuedFraction, extended_gcd, is_prime, legendre_symbol};
use algebra_engine::topology::SimplicialComplex;
use num_bigint::BigUint;

#[test]
fn test_combinatorics() {
    assert_eq!(factorial(5), BigUint::from(120u32));
    assert_eq!(combinations(5, 2), BigUint::from(10u32));
    assert_eq!(permutations(5, 2), BigUint::from(20u32));
    assert_eq!(derangements(4), BigUint::from(9u32));
    assert_eq!(catalan_number(3), BigUint::from(5u32));
    assert_eq!(catalan_number(4), BigUint::from(14u32));

    // Partitions of 4: (4, 3+1, 2+2, 2+1+1, 1+1+1+1) -> p(4) = 5
    assert_eq!(integer_partitions(4), BigUint::from(5u32));
    // Partitions of 5: p(5) = 7
    assert_eq!(integer_partitions(5), BigUint::from(7u32));

    assert_eq!(stirling_second_kind(4, 2), BigUint::from(7u32));
    assert_eq!(bell_number(3), BigUint::from(5u32));
}

#[test]
fn test_simplicial_topology() {
    let mut complex = SimplicialComplex::new();
    // Triangle (0, 1, 2)
    complex.add_simplex(&[0, 1, 2]);

    assert_eq!(complex.count_simplices(0), 3); // 3 vertices
    assert_eq!(complex.count_simplices(1), 3); // 3 edges
    assert_eq!(complex.count_simplices(2), 1); // 1 2-face

    // Euler characteristic for filled triangle: 3 - 3 + 1 = 1
    assert_eq!(complex.euler_characteristic(), 1);
}

#[test]
fn test_control_state_space() {
    let a = vec![vec![-2.0, 1.0], vec![0.0, -3.0]];
    let b = vec![vec![1.0], vec![1.0]];
    let c = vec![vec![1.0, 0.0]];
    let d = vec![vec![0.0]];

    let sys = StateSpaceSystem::new(a, b, c, d);
    assert_eq!(sys.is_stable_2x2(), Some(true));

    let ctrl_mat = sys.controllability_matrix_2x2().unwrap();
    assert_eq!(ctrl_mat.len(), 2);
}

#[test]
fn test_number_theory() {
    assert!(is_prime(17));
    assert!(is_prime(101));
    assert!(!is_prime(100));

    // Extended GCD: 30*x + 20*y = 10
    let (gcd, x, y) = extended_gcd(30, 20);
    assert_eq!(gcd, 10);
    assert_eq!(30 * x + 20 * y, 10);

    // Continued fraction of sqrt(2) approx 1.41421356 -> [1; 2, 2, 2, ...]
    let cf = ContinuedFraction::from_f64(std::f64::consts::SQRT_2, 5);
    assert_eq!(cf.terms[0], 1);
    assert_eq!(cf.terms[1], 2);

    // Legendre symbol (2/7) = 1 (2 is a quadratic residue mod 7: 3^2 = 9 = 2 mod 7)
    assert_eq!(legendre_symbol(2, 7), 1);
    // (3/7) = -1 (3 is not a quadratic residue mod 7)
    assert_eq!(legendre_symbol(3, 7), -1);
}

#[test]
fn test_universal_prime_decomposition_and_classification() {
    use algebra_engine::numbertheory::{
        PrimeClassification, decompose_eisenstein, decompose_galois, decompose_gaussian,
        decompose_integer, decompose_modulo, decompose_padic, decompose_rational,
        decompose_universal,
    };

    // 1. Rational Integers (Z)
    let dec_60 = decompose_integer(60);
    assert_eq!(dec_60.factors.len(), 3);
    assert_eq!(dec_60.factors[0].factor_repr, "2");
    assert_eq!(dec_60.factors[0].exponent, 2);
    assert!(
        dec_60.factors[0]
            .classifications
            .contains(&PrimeClassification::EvenPrime)
    );

    assert_eq!(dec_60.factors[1].factor_repr, "3");
    assert!(
        dec_60.factors[1]
            .classifications
            .contains(&PrimeClassification::EisensteinRamified)
    );
    assert!(
        dec_60.factors[1]
            .classifications
            .contains(&PrimeClassification::GaussianInert)
    );

    assert_eq!(dec_60.factors[2].factor_repr, "5");
    assert!(
        dec_60.factors[2]
            .classifications
            .contains(&PrimeClassification::GaussianSplit { u: 1, v: 2 })
    );

    // Negative integer: preserves sign unit -1
    let dec_neg = decompose_integer(-13);
    assert_eq!(dec_neg.unit_part, "-1");
    assert_eq!(dec_neg.factors[0].factor_repr, "13");
    assert!(
        dec_neg.factors[0]
            .classifications
            .contains(&PrimeClassification::GaussianSplit { u: 2, v: 3 })
    );

    // 2. Rational Numbers (Q)
    let dec_q = decompose_rational(21, 40); // (3 * 7) / (2^3 * 5)
    assert_eq!(dec_q.factors.len(), 4);
    let p_2 = dec_q.factors.iter().find(|f| f.factor_repr == "2").unwrap();
    assert_eq!(p_2.exponent, -3);
    assert!(
        p_2.classifications
            .contains(&PrimeClassification::RationalDenominatorPrime)
    );

    let p_7 = dec_q.factors.iter().find(|f| f.factor_repr == "7").unwrap();
    assert_eq!(p_7.exponent, 1);
    assert!(
        p_7.classifications
            .contains(&PrimeClassification::RationalNumeratorPrime)
    );

    // 3. Gaussian Integers Z[i]
    // 3 + 4i = (2 + i)^2 * (-i) or associate, norm 25 = 5^2
    let dec_gi = decompose_gaussian(3, 4);
    assert_eq!(dec_gi.number_system, "Gaussian Integers (ℤ[i])");
    assert!(!dec_gi.factors.is_empty());
    assert!(dec_gi.factors.iter().all(|f| {
        f.classifications
            .contains(&PrimeClassification::GaussianSplitPrime)
    }));

    // 5 in Z[i] splits as (1 + 2i)(1 - 2i)
    let dec_5_gi = decompose_gaussian(5, 0);
    assert_eq!(dec_5_gi.factors.len(), 2);
    assert!(
        dec_5_gi.factors[0]
            .classifications
            .contains(&PrimeClassification::GaussianSplitPrime)
    );

    // 1 + i is ramified
    let dec_ram = decompose_gaussian(1, 1);
    assert_eq!(
        dec_ram.factors[0].classifications,
        vec![PrimeClassification::GaussianRamifiedPrime]
    );

    // 3 in Z[i] is inert
    let dec_3_gi = decompose_gaussian(3, 0);
    assert_eq!(
        dec_3_gi.factors[0].classifications,
        vec![PrimeClassification::GaussianInertPrime]
    );

    // 4. Eisenstein Integers Z[w]
    // 3 is ramified: associate of (1 - w)^2
    let dec_3_ei = decompose_eisenstein(3, 0);
    assert_eq!(dec_3_ei.number_system, "Eisenstein Integers (ℤ[ω])");
    assert!(
        dec_3_ei.factors[0]
            .classifications
            .contains(&PrimeClassification::EisensteinRamifiedPrime)
    );

    // 7 in Z[w] splits
    let dec_7_ei = decompose_eisenstein(7, 0);
    assert!(dec_7_ei.factors.iter().any(|f| {
        f.classifications
            .contains(&PrimeClassification::EisensteinSplitPrime)
    }));

    // 2 in Z[w] is inert
    let dec_2_ei = decompose_eisenstein(2, 0);
    assert_eq!(
        dec_2_ei.factors[0].classifications,
        vec![PrimeClassification::EisensteinInertPrime]
    );

    // 5. Modular Rings Z/nZ
    // 12 in Z/15Z: gcd(12, 15) = 3, zero-divisor, prime ideal generator
    let dec_mod = decompose_modulo(12, 15);
    assert!(
        dec_mod
            .classification_summary
            .iter()
            .any(|s| s.contains("gcd(x, n) = 3"))
    );
    assert!(
        dec_mod
            .classification_summary
            .iter()
            .any(|s| s.contains("Modular prime/maximal ideal"))
    );

    // 4 in Z/15Z: gcd(4, 15) = 1, unit with multiplicative order 2 (4^2 = 16 = 1 mod 15)
    let dec_mod_unit = decompose_modulo(4, 15);
    assert!(
        dec_mod_unit
            .classification_summary
            .iter()
            .any(|s| s.contains("order 2"))
    );

    // 6 in Z/9Z: nilpotent (6^2 = 36 = 0 mod 9)
    let dec_mod_nil = decompose_modulo(6, 9);
    assert!(
        dec_mod_nil
            .classification_summary
            .iter()
            .any(|s| s.contains("nilpotent"))
    );

    // 10 in Z/15Z: idempotent (10^2 = 100 = 10 mod 15)
    let dec_mod_idem = decompose_modulo(10, 15);
    assert!(
        dec_mod_idem
            .classification_summary
            .iter()
            .any(|s| s.contains("idempotent"))
    );

    // 6. p-Adic Field Q_p
    // 45 in Q_3: 45 = 3^2 * 5. Valuation v_3(45) = 2, unit is 5
    let dec_padic = decompose_padic(45, 3);
    assert_eq!(dec_padic.factors[0].factor_repr, "3");
    assert_eq!(dec_padic.factors[0].exponent, 2);
    assert_eq!(dec_padic.unit_part, "5");

    // 7. Galois Field GF(p^k)
    // In GF(7): q = 7, group order = 6. Element 3 has order 6 (primitive generator: 3^1=3, 3^2=2, 3^3=6, 3^4=4, 3^5=5, 3^6=1 mod 7)
    let dec_gf = decompose_galois(3, 7, 1);
    assert!(
        dec_gf
            .classification_summary
            .iter()
            .any(|s| s.contains("Primitive Generator"))
    );

    // 8. Universal string parser and domain routing
    let univ_int = decompose_universal("120", None).unwrap();
    assert_eq!(univ_int.number_system, "Integers (ℤ)");

    let univ_gauss = decompose_universal("3 + 4i", None).unwrap();
    assert_eq!(univ_gauss.number_system, "Gaussian Integers (ℤ[i])");

    let univ_eisen = decompose_universal("7", Some("Eisenstein")).unwrap();
    assert_eq!(univ_eisen.number_system, "Eisenstein Integers (ℤ[ω])");

    let univ_mod = decompose_universal("12", Some("Modulo(15)")).unwrap();
    assert_eq!(univ_mod.number_system, "Modular Ring (ℤ/15ℤ)");

    let univ_padic = decompose_universal("45", Some("PAdics(p=3)")).unwrap();
    assert_eq!(univ_padic.number_system, "p-Adic Field (ℚ_3)");

    let univ_rat = decompose_universal("21/40", None).unwrap();
    assert_eq!(univ_rat.number_system, "Rationals (ℚ)");
}

#[test]
fn test_tropical_newton_polygon_padic_roots() {
    use algebra_engine::poly::NewtonPolygon;
    use algebra_engine::tropical::MinPlus;

    // Polynomial g(x) = (x - 2)(x - 4) = 8 - 6x + x^2 over Q_2
    // Roots: 2 (val 1), 4 (val 2)
    let np = NewtonPolygon::from_integer_coeffs(&[8, -6, 1], 2);
    assert_eq!(np.vertices.len(), 3);
    assert_eq!(np.segments.len(), 2);

    let vals = np.padic_root_valuations();
    // Segment 1: from (0, 3) to (1, 1), slope -2 -> root val 2 (count 1)
    // Segment 2: from (1, 1) to (2, 0), slope -1 -> root val 1 (count 1)
    assert_eq!(vals[0], (2.0, 1));
    assert_eq!(vals[1], (1.0, 1));

    // Tropical evaluation at w = -2: min(3 + 0, 1 - 2, 0 - 4) = -4
    let trop_val = np.eval_tropical(-2.0);
    assert_eq!(trop_val, MinPlus::val(-4.0));

    // Irreducible Eisenstein cubic: f(x) = 2 + 6x + x^3 over Q_2
    // Points: (0, 1), (1, 1), (3, 0). (1, 1) is strictly above chord.
    // Lower hull has single segment from (0, 1) to (3, 0) with slope -1/3.
    let np_eisenstein = NewtonPolygon::from_integer_coeffs(&[2, 6, 0, 1], 2);
    assert_eq!(np_eisenstein.vertices.len(), 2);
    assert_eq!(np_eisenstein.segments.len(), 1);
    let eisenstein_vals = np_eisenstein.padic_root_valuations();
    assert_eq!(eisenstein_vals[0].1, 3); // 3 roots
    assert!((eisenstein_vals[0].0 - 1.0 / 3.0).abs() < 1e-10); // valuation 1/3
}

#[test]
fn test_special_functions_weyl_annihilators() {
    use algebra_engine::special::{
        BesselJ, ErrorFunctionErf, HermiteH, HolonomicFunction, LegendreP,
    };

    // 1. Bessel J_0(x): x^2 d^2 + x d + x^2 = 0 (nu = 0)
    let j0 = BesselJ { nu: 0.0 };
    let op_j0 = j0.annihilator();
    assert_eq!(op_j0.num_vars, 1);
    assert_eq!(op_j0.terms.len(), 3);

    // 2. Hermite H_2(x): d^2 - 2x d + 4 = 0 (n = 2)
    let h2 = HermiteH { n: 2 };
    let op_h2 = h2.annihilator();
    assert_eq!(op_h2.terms.len(), 3);

    // 3. Legendre P_1(x): (1 - x^2) d^2 - 2x d + 2 = 0 (n = 1)
    let p1 = LegendreP { n: 1 };
    let op_p1 = p1.annihilator();
    assert_eq!(op_p1.terms.len(), 4);

    // 4. Error function erf(x): d^2 + 2x d = 0
    let erf = ErrorFunctionErf;
    let op_erf = erf.annihilator();
    assert_eq!(op_erf.terms.len(), 2);
}

#[test]
fn test_creative_telescoping_definite_integration() {
    use algebra_core::ExprGraph;
    use algebra_engine::risch::RischIntegrator;

    let graph = ExprGraph::new();
    let x_sym = graph.symbols.get_or_intern("x");
    let y_sym = graph.symbols.get_or_intern("y");

    // Gaussian integrand: exp(-x * y^2)
    let integrand = graph.symbol("f"); // placeholder

    // General creative telescoping is not implemented and must say so.
    assert!(
        RischIntegrator::creative_telescoping_integral(&graph, integrand, x_sym, y_sym).is_err()
    );

    // The Gaussian case is available directly.
    let (ode_op, certificate) =
        algebra_engine::weyl_dmodules::AlmkvistZeilberger::gaussian_integral_ode();
    assert_eq!(ode_op.num_vars, 1);
    assert_eq!(ode_op.terms.len(), 2);
    assert!(certificate.contains("exp"));
}
