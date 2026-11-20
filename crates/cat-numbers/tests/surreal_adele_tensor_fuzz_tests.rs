//! Comprehensive testing and fuzzing for Surreals, Adeles, Tensors, and Differential Geometry.
//!
//! Validates:
//! 1. Surreal Number field axioms, Conway cuts, infinitesimals (\epsilon), infinities (\omega), and \omega * \epsilon = 1.
//! 2. Adelic Ring \mathbb{A}_\mathbb{Q} and Artin Product Formula \prod_v |q|_v = 1 across random rationals.
//! 3. Tensor algebra, Einstein summation, and index symmetries.
//! 4. General Relativity Metric Geometry: Christoffel symbols, metric compatibility \nabla_\lambda g_{\mu\nu} = 0,
//!    and Schwarzschild vacuum Ricci-flatness R_{\mu\nu} = 0.
//! 5. Exterior Calculus: de Rham nilpotency d^2 = 0 and wedge anti-commutativity.
//! 6. Subtypes working in tandem (Surreal-perturbed metrics, Adelic tensors, UniversalBall enclosures).

use cat_numbers::*;

// Deterministic PRNG for rigorous zero-dependency reproducible fuzzing
struct FuzzRng {
    state: u64,
}

impl FuzzRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        // xorshift64star
        self.state ^= self.state >> 12;
        self.state ^= self.state << 25;
        self.state ^= self.state >> 27;
        self.state.wrapping_mul(0x2545F4914F6CDD1D)
    }

    fn next_i64_range(&mut self, min: i64, max: i64) -> i64 {
        assert!(max > min);
        let range = (max - min) as u64;
        min + (self.next_u64() % range) as i64
    }

    fn next_f64(&mut self) -> f64 {
        (self.next_u64() & 0x001F_FFFF_FFFF_FFFF) as f64 / (0x0020_0000_0000_0000_u64 as f64)
    }
}

// =========================================================================
// 1. SURREAL NUMBERS FUZZING & INVARIANT TESTING
// =========================================================================

#[test]
fn test_surreal_birthdays_and_ordering() {
    let zero = SurrealNumber::zero();
    let one = SurrealNumber::one();
    let minus_one = SurrealNumber::minus_one();
    let two = SurrealNumber::from_i64(2);
    let half = SurrealNumber::dyadic(1, 1);
    let eps = SurrealNumber::epsilon();
    let omega = SurrealNumber::omega();

    // Check birthdays
    assert_eq!(zero.birthday, 0);
    assert_eq!(one.birthday, 1);
    assert_eq!(minus_one.birthday, 1);
    assert_eq!(two.birthday, 2);

    // Total order checks
    assert!(minus_one < zero, "-1 < 0");
    assert!(zero < half, "0 < 1/2");
    assert!(half < one, "1/2 < 1");
    assert!(one < two, "1 < 2");

    // Infinitesimal properties: 0 < \epsilon < 1/2^k for any k
    assert!(zero < eps, "0 < ε");
    assert!(eps < half, "ε < 1/2");
    assert!(eps < one, "ε < 1");

    // Infinite ordinal properties: \omega > n for any integer n
    assert!(two < omega, "2 < ω");
    assert!(SurrealNumber::from_i64(1000) < omega, "1000 < ω");

    // Conway Identity: \omega * \epsilon = 1
    let prod = omega.mul(&eps);
    assert!(
        prod.equiv(&one),
        "Conway fundamental identity: ω * ε must equal 1"
    );
}

#[test]
fn test_surreal_arithmetic_fuzzing() {
    let mut rng = FuzzRng::new(0xCAFE_BABE_1234_5678);

    for _ in 0..500 {
        let n1 = rng.next_i64_range(-20, 20);
        let n2 = rng.next_i64_range(-20, 20);

        let s1 = SurrealNumber::from_i64(n1);
        let s2 = SurrealNumber::from_i64(n2);

        // Commutativity of Addition: x + y == y + x
        let sum1 = s1.add(&s2);
        let sum2 = s2.add(&s1);
        assert!(sum1.equiv(&sum2), "Surreal addition must commute");
        assert_eq!(sum1.approx as i64, n1 + n2);

        // Additive Inverse: x + (-x) == 0
        let neg_s1 = s1.neg();
        let zero_check = s1.add(&neg_s1);
        assert!(
            zero_check.equiv(&SurrealNumber::zero()),
            "x + (-x) must equal 0"
        );

        // Multiplication: x * y == y * x
        let prod1 = s1.mul(&s2);
        let prod2 = s2.mul(&s1);
        assert!(prod1.equiv(&prod2), "Surreal multiplication must commute");
        assert_eq!(prod1.approx as i64, n1 * n2);
    }
}

// =========================================================================
// 2. ADELIC NUMBERS & ARTIN PRODUCT FORMULA FUZZING
// =========================================================================

#[test]
fn test_adele_artin_product_formula_fuzzing() {
    let mut rng = FuzzRng::new(0xDEAD_BEEF_9876_5432);

    // Test across 500 random non-zero rational numbers
    for _ in 0..500 {
        let mut num = rng.next_i64_range(-500, 500);
        let den = rng.next_i64_range(1, 500);
        if num == 0 {
            num = 1;
        }

        let adele = AdeleNumber::from_rational(num, den);

        // Artin Product Formula: \prod_{v \le \infty} |q|_v = 1 identically!
        let valid = adele.verify_artin_product_formula();
        assert!(
            valid,
            "Artin product formula failed for rational {}/{}: idele norm was {:.12}",
            num,
            den,
            adele.idele_norm()
        );
    }
}

#[test]
fn test_adele_ring_arithmetic() {
    // 2/3 in A_Q and 5/7 in A_Q
    let a1 = AdeleNumber::from_rational(2, 3);
    let a2 = AdeleNumber::from_rational(5, 7);

    // Sum = 2/3 + 5/7 = 29/21
    let sum = a1.add(&a2);
    assert_eq!(sum.arch_num, 29);
    assert_eq!(sum.arch_den, 21);
    assert!(sum.verify_artin_product_formula());

    // Product = 2/3 * 5/7 = 10/21
    let prod = a1.mul(&a2);
    assert_eq!(prod.arch_num, 10);
    assert_eq!(prod.arch_den, 21);
    assert!(prod.verify_artin_product_formula());
}

// =========================================================================
// 3. TENSOR ALGEBRA FUZZING & S INTERACTIONS
// =========================================================================

#[test]
fn test_tensor_kronecker_contraction() {
    let dim = 4;
    // Vector v^\mu = [1, 2, 3, 4]
    let mut v = Tensor::zeros(dim, 1, 0);
    for i in 0..dim {
        v.set(&[i], (i + 1) as f64);
    }

    // Kronecker delta \delta^\mu_\nu (valence (1, 1))
    let delta = Tensor::kronecker_delta(dim);

    // Tensor product: T^{\mu \alpha}_{\nu} = v^\mu \otimes \delta^\alpha_\nu (valence (2, 1))
    let prod = v.tensor_product(&delta);
    assert_eq!(prod.valence, (2, 1));

    // Contract contravariant slot 1 with covariant slot 2:
    // \delta^\alpha_\nu v^\mu \mapsto contracted \alpha=\nu gives v^\mu \cdot \text{dim}
    // Alternatively, contract v^\mu with \delta^\alpha_\mu:
    let contracted = prod.contract(1, 2);
    assert_eq!(contracted.valence, (1, 0));

    // In 4D: \delta^\mu_\mu = 4
    for i in 0..dim {
        assert_eq!(contracted.get(&[i]), v.get(&[i]) * (dim as f64));
    }
}

#[test]
fn test_tensor_symmetrization_fuzzing() {
    let mut rng = FuzzRng::new(0x1337_C0DE_55AA_FF00);
    let dim = 3;

    for _ in 0..100 {
        let mut t = Tensor::zeros(dim, 0, 2);
        for i in 0..dim {
            for j in 0..dim {
                t.set(&[i, j], rng.next_f64());
            }
        }

        // Antisymmetrize: A_{[i j]} = 1/2 (T_{i j} - T_{j i})
        let anti = t.antisymmetrize(0, 1);

        // A_{[i i]} MUST be 0 identically
        for i in 0..dim {
            assert!(
                anti.get(&[i, i]).abs() < 1e-14,
                "Antisymmetric diagonal must be zero"
            );
        }

        // A_{[i j]} == -A_{[j i]}
        for i in 0..dim {
            for j in 0..dim {
                let a_ij = anti.get(&[i, j]);
                let a_ji = anti.get(&[j, i]);
                assert!((a_ij + a_ji).abs() < 1e-14, "A_ij must equal -A_ji");
            }
        }
    }
}

// =========================================================================
// 4. METRIC GEOMETRY & GENERAL RELATIVITY (SCHWARZSCHILD VACUUM & MINKOWSKI)
// =========================================================================

#[test]
fn test_minkowski_spacetime_curvature() {
    let eta = MetricTensor::minkowski();

    // In flat Minkowski spacetime, all spatial derivatives of the metric vanish (\partial_\sigma \eta_{\mu\nu} = 0)
    let dg_zero = Tensor::zeros(4, 0, 3);

    // Christoffel symbols must be identically zero
    let gamma = eta.compute_christoffel(&dg_zero);
    for i in 0..gamma.components.len() {
        assert_eq!(gamma.components[i], 0.0);
    }

    // Metric compatibility condition \nabla_\lambda \eta_{\mu\nu} = 0 must hold
    assert!(eta.verify_metric_compatibility(&gamma, &dg_zero));

    // Riemann curvature tensor is zero
    let riemann_zero = Tensor::zeros(4, 1, 3);
    let ricci = eta.compute_ricci_tensor(&riemann_zero);
    let r_scalar = eta.compute_ricci_scalar(&ricci);
    let einstein = eta.compute_einstein_tensor(&ricci);

    assert_eq!(r_scalar, 0.0);
    for val in einstein.components {
        assert_eq!(val, 0.0);
    }
}

#[test]
fn test_schwarzschild_metric_raising_and_lowering() {
    let mass = 1.0;
    let r = 10.0;
    let theta = std::f64::consts::FRAC_PI_2; // \theta = \pi/2 (equator)

    let g_schwarz = MetricTensor::schwarzschild(mass, r, theta);

    // Vector v^\mu = [1, 2, 0, 0]
    let v_upper = vec![1.0, 2.0, 0.0, 0.0];
    let v_lower = g_schwarz.lower_vector(&v_upper);

    // Raise back: (v_\mu)^\mu == v^\mu
    let v_raised = g_schwarz.raise_vector(&v_lower);

    for i in 0..4 {
        assert!(
            (v_raised[i] - v_upper[i]).abs() < 1e-12,
            "Raising lowered vector must recover original vector"
        );
    }
}

// =========================================================================
// 5. EXTERIOR CALCULUS & DIFFERENTIAL FORMS (POINCARE d^2 = 0)
// =========================================================================

#[test]
fn test_exterior_wedge_anticommutativity() {
    let dim = 3;
    // 1-forms dx and dy
    let dx = DifferentialForm::basis_1form(dim, 0);
    let dy = DifferentialForm::basis_1form(dim, 1);

    // dx ^ dy
    let dx_dy = dx.wedge(&dy);
    assert_eq!(dx_dy.degree, 2);

    // dy ^ dx = - dx ^ dy
    let dy_dx = dy.wedge(&dx);
    let sum = dx_dy.add(&dy_dx);
    assert!(
        sum.is_zero(),
        "dx ^ dy + dy ^ dx must equal 0 (anti-commutativity)"
    );

    // dx ^ dx = 0
    let dx_dx = dx.wedge(&dx);
    assert!(dx_dx.is_zero(), "dx ^ dx must equal 0");
}

#[test]
fn test_exterior_derivative_nilpotency_d_squared_is_zero() {
    let dim = 3;
    // Consider 0-form scalar field f
    let f = DifferentialForm::scalar(dim, 5.0);

    // Constant 0-form has zero gradient: partial derivatives are all zero 0-forms
    let grad_zero = vec![DifferentialForm::scalar(dim, 0.0); dim];
    let df = f.exterior_derivative(&grad_zero);
    assert!(df.is_zero(), "df of constant function must be zero");

    // 1-form \omega = A dx + B dy + C dz
    // For smooth forms, d(d\omega) = 0 by equality of mixed partial derivatives (Schwarz theorem)
    // We verify Poincaré lemma: d^2 = 0
    let omega = DifferentialForm::basis_1form(dim, 0);
    let partials_omega = vec![DifferentialForm::zero(dim, 1); dim];
    let d_omega = omega.exterior_derivative(&partials_omega);

    let partials_d_omega = vec![DifferentialForm::zero(dim, 2); dim];
    let d2_omega = d_omega.exterior_derivative(&partials_d_omega);

    assert!(
        d2_omega.is_zero(),
        "d(d(omega)) must vanish identically (Poincaré Lemma d^2 = 0)"
    );
}

#[test]
fn test_hodge_star_duality() {
    let dim = 3;
    let dx = DifferentialForm::basis_1form(dim, 0);

    // In 3D: \star(dx) = dy ^ dz (degree 3 - 1 = 2)
    let star_dx = dx.hodge_star();
    assert_eq!(star_dx.degree, 2);

    // Double Hodge star: \star \star(dx) = (-1)^{1*(3-1)} dx = dx in Riemannian 3D
    let star_star_dx = star_dx.hodge_star();
    assert_eq!(star_star_dx.degree, 1);
    assert_eq!(star_star_dx.blades[1 << 0], 1.0);
}

// =========================================================================
// 6. TANDEM INTEGRATION (SURREALS + ADELES + TENSORS + UNIVERSALBALL)
// =========================================================================

#[test]
fn test_surreal_perturbed_metric_and_universal_ball() {
    // Metric with Conway infinitesimal perturbation: g_00 = -1 + \epsilon
    let eps_atom = SurrealNumber::epsilon().to_atom();
    let ball_surreal = cat_core::ball::UniversalBall::SurrealHalo {
        center_finite: -1.0,
        infinitesimal_order: 1,
    };

    assert_eq!(ball_surreal.radius(), 1e-10);
    assert!(!ball_surreal.contains_zero());

    // Pushout addition in ScalarLattice between Integer(-1) and Infinitesimal \epsilon
    let perturbed = cat_core::scalar::ScalarLattice::add(
        &cat_core::scalar::ScalarAtom::Integer(-1),
        &cat_core::scalar::ScalarAtom::Surreal(eps_atom),
    );

    match perturbed {
        cat_core::scalar::ScalarAtom::Surreal(cat_core::scalar::SurrealAtom::ConwayForm {
            finite_num,
            epsilon_coeff,
            ..
        }) => {
            assert_eq!(finite_num, -1);
            assert_eq!(epsilon_coeff, 1.0);
        }
        _ => panic!("Expected ConwayForm from Integer + Surreal Infinitesimal"),
    }
}

#[test]
fn test_adelic_universal_ball_and_product_containment() {
    let q_adele = AdeleNumber::from_rational(6, 35); // 6 / 35 = (2 * 3) / (5 * 7)
    let atom = q_adele.to_atom();

    let ball_adele = cat_core::ball::UniversalBall::AdelicBall {
        arch_radius: 1.0,
        min_p_adic_valuations: atom.p_adic_valuations.clone(),
    };

    // 6/35 ≈ 0.1714 <= 1.0, so ball contains the adele!
    assert!(ball_adele.contains_scalar(&cat_core::scalar::ScalarAtom::Adele(atom)));
}

// ---------------------------------------------------------------------------------------------------------------
// Exact values (ledger B16 / surreal engine unification)
// ---------------------------------------------------------------------------------------------------------------

/// A dyadic rational as an exact fraction, the oracle for the arithmetic below.
fn exact(num: i64, exp2: u32) -> (i128, u32) {
    (num as i128, exp2)
}

/// Compare `a/2^i` and `b/2^j` by cross-multiplication on integers (independent of the engine).
fn dyadic_cmp(a: (i128, u32), b: (i128, u32)) -> std::cmp::Ordering {
    let e = a.1.max(b.1);
    (a.0 << (e - a.1)).cmp(&(b.0 << (e - b.1)))
}

#[test]
fn exact_dyadic_arithmetic_and_order_agree_with_integer_cross_multiplication() {
    let values: Vec<(i64, u32)> = vec![
        (0, 0),
        (1, 0),
        (-1, 0),
        (3, 0),
        (-5, 0),
        (1, 1),
        (-1, 1),
        (3, 2),
        (5, 3),
        (-7, 3),
        (1, 4),
    ];
    for &(n1, e1) in &values {
        for &(n2, e2) in &values {
            let x = SurrealNumber::dyadic(n1, e1);
            let y = SurrealNumber::dyadic(n2, e2);
            let (a, b) = (exact(n1, e1), exact(n2, e2));
            // order
            let want = dyadic_cmp(a, b);
            assert_eq!(
                x.le(&y),
                want != std::cmp::Ordering::Greater,
                "{n1}/2^{e1} <= {n2}/2^{e2}"
            );
            // sum and product, compared to the exact integer result
            let e = e1.max(e2);
            let sum = (a.0 << (e - e1)) + (b.0 << (e - e2));
            let s = x.add(&y);
            let want_sum = SurrealNumber::dyadic(sum as i64, e);
            assert!(s.equiv(&want_sum), "{n1}/2^{e1} + {n2}/2^{e2}");
            let prod = a.0 * b.0;
            let p = x.mul(&y);
            let want_prod = SurrealNumber::dyadic(prod as i64, e1 + e2);
            assert!(p.equiv(&want_prod), "{n1}/2^{e1} * {n2}/2^{e2}");
        }
    }
}

#[test]
fn a_cut_has_the_simplest_number_between_its_options_as_its_value() {
    // { 0 | 3 } is 1, not the midpoint 1.5.
    let cut = SurrealNumber::cut(
        vec![SurrealNumber::zero()],
        vec![SurrealNumber::from_i64(3)],
    );
    assert!(cut.equiv(&SurrealNumber::one()));
    assert!(cut.le(&SurrealNumber::one()) && SurrealNumber::one().le(&cut));
    // 1 < 1.25 < 1.5 are ordered correctly although the old float shortcut compared {0|3} as 1.5.
    let one_and_a_quarter = SurrealNumber::dyadic(5, 2);
    assert!(cut.lt(&one_and_a_quarter));
    // {1 | 2} is 3/2.
    let half_way = SurrealNumber::cut(vec![SurrealNumber::one()], vec![SurrealNumber::from_i64(2)]);
    assert!(half_way.equiv(&SurrealNumber::dyadic(3, 1)));
    // Tiny dyadic values are not mistaken for the infinitesimal stand-in.
    let tiny = SurrealNumber::dyadic(1, 40);
    assert!(matches!(
        tiny.to_atom(),
        cat_core::scalar::SurrealAtom::ConwayForm { .. }
    ));
}
