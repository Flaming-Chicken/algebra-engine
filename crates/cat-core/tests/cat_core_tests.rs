use cat_core::*;
use smallvec::smallvec;

#[test]
fn test_lifetime_free_arena_deduplication() {
    let ctx = CatRuntimeContext::new();

    let obj1 = ctx.intern_obj(ObjectDescriptor::linear_scalar(DimVector::length()));
    let obj2 = ctx.intern_obj(ObjectDescriptor::linear_scalar(DimVector::length()));
    assert_eq!(
        obj1, obj2,
        "Hash-consing must deduplicate identical object descriptors"
    );

    let s1 = ctx.intern_scalar(ScalarAtom::complex_rational(1, 2, 3, 4));
    let s2 = ctx.intern_scalar(ScalarAtom::complex_rational(2, 4, 6, 8));
    assert_eq!(
        s1, s2,
        "Gaussian rationals must reduce to canonical form and share ScalarId"
    );

    let mut diag1 = OpenHypergraph::new();
    let w0 = diag1.alloc_wire();
    let w1 = diag1.alloc_wire();
    diag1.add_edge(HyperEdge::binary(w0, w1, w0, HyperOp::Add));

    let mut diag2 = OpenHypergraph::new();
    let v1 = diag2.alloc_wire();
    let v0 = diag2.alloc_wire();
    diag2.add_edge(HyperEdge::binary(v0, v1, v0, HyperOp::Add));

    let d1 = ctx.intern_diagram(diag1);
    let d2 = ctx.intern_diagram(diag2);
    assert_eq!(
        d1, d2,
        "1-WL canonicalization must produce identical DiagramId for isomorphic hypergraphs"
    );
}

#[test]
fn test_dimensional_fibration_and_natural_units() {
    let length = DimVector::length();
    let time = DimVector::time();
    let vel = length / time;
    assert_eq!(vel, DimVector::velocity());

    let mass = DimVector::mass();
    let energy = mass * vel * vel;
    assert_eq!(energy, DimVector::energy());

    // Fractional power: (L^2)^(1/2) = L^1
    let l2 = length * length;
    let l_sqrt = l2.sqrt().expect("Square root of L^2 must succeed");
    assert_eq!(l_sqrt, length);

    // Natural units reduction: \hbar = c = 1
    // Energy [M L^2 T^-2] and Momentum [M L T^-1] and Mass [M] all reduce to same dimension [E]
    let gauge = UnitGaugeSystem::Natural;
    let red_energy = gauge.reduce(energy);
    let red_momentum = gauge.reduce(DimVector::momentum());
    let red_mass = gauge.reduce(mass);
    assert_eq!(red_energy, red_momentum);
    assert_eq!(red_energy, red_mass);

    // se(3) blocks
    let se3_lin = DimVector::se3_linear();
    let se3_ang = DimVector::se3_angular();
    assert_eq!(se3_lin, DimVector::velocity());
    assert!(!se3_ang.is_dimensionless());
}

#[test]
fn test_complex_types_and_gaussian_rationals() {
    // i^2 == -1 in Gaussian rationals
    let i = ScalarAtom::imaginary_unit();
    let i_squared = ScalarLattice::mul(&i, &i);
    assert_eq!(i_squared, ScalarAtom::Integer(-1));

    // (1/2 + 3/4 i) * (1/2 - 3/4 i) = 1/4 + 9/16 = 13/16
    let z1 = ScalarAtom::complex_rational(1, 2, 3, 4);
    let z2 = ScalarAtom::complex_rational(1, 2, -3, 4);
    let prod = ScalarLattice::mul(&z1, &z2);
    assert_eq!(prod, ScalarAtom::rational(13, 16));

    // Pushout addition: Integer(5) + Gaussian(2 + 3i) = Gaussian(7 + 3i)
    let sum = ScalarLattice::add(
        &ScalarAtom::Integer(5),
        &ScalarAtom::GaussianInt { re: 2, im: 3 },
    );
    assert_eq!(sum, ScalarAtom::GaussianInt { re: 7, im: 3 });
}

#[test]
fn test_quaternion_and_octonion_cocycle_algebra() {
    // Quaternion basis: i^2 = j^2 = k^2 = -1, ij = k, ji = -k
    let q_i = CocycleMultivector::quaternion(0.0, 1.0, 0.0, 0.0);
    let q_j = CocycleMultivector::quaternion(0.0, 0.0, 1.0, 0.0);
    let q_k = CocycleMultivector::quaternion(0.0, 0.0, 0.0, 1.0);

    let ij = q_i.geometric_product(&q_j);
    assert_eq!(ij.blades, q_k.blades, "Quaternion ij must equal k");

    let ji = q_j.geometric_product(&q_i);
    let minus_k = q_k.scale(-1.0);
    assert_eq!(ji.blades, minus_k.blades, "Quaternion ji must equal -k");

    let ii = q_i.geometric_product(&q_i);
    assert_eq!(ii.blades[0], -1.0, "i^2 must equal -1");

    // Octonions: verify non-associativity: (e_1 * e_2) * e_4 != e_1 * (e_2 * e_4)
    let mut e1_blades = [0.0; 8];
    e1_blades[1] = 1.0;
    let mut e2_blades = [0.0; 8];
    e2_blades[2] = 1.0;
    let mut e4_blades = [0.0; 8];
    e4_blades[4] = 1.0;

    let e1 = CocycleMultivector::octonion(e1_blades);
    let e2 = CocycleMultivector::octonion(e2_blades);
    let e4 = CocycleMultivector::octonion(e4_blades);

    let e1_e2 = e1.geometric_product(&e2);
    let lhs = e1_e2.geometric_product(&e4);

    let e2_e4 = e2.geometric_product(&e4);
    let rhs = e1.geometric_product(&e2_e4);

    assert_ne!(lhs.blades, rhs.blades, "Octonions must be non-associative");
    // Octonion alternative property: (x * x) * y == x * (x * y)
    let e1_e1 = e1.geometric_product(&e1);
    let alt_lhs = e1_e1.geometric_product(&e2);
    let alt_rhs = e1.geometric_product(&e1_e2);
    assert_eq!(
        alt_lhs.blades, alt_rhs.blades,
        "Octonions must satisfy left alternative law"
    );
}

#[test]
fn test_universal_ball_complex_disk_enclosure() {
    // Disk D1: center 1 + 2i, radius 0.1
    // Disk D2: center 3 - 4i, radius 0.2
    let b1 = UniversalBall::complex_disk(1.0, 2.0, 0.1);
    let b2 = UniversalBall::complex_disk(3.0, -4.0, 0.2);

    // Sum
    let sum = b1.add(&b2);
    let (s_re, s_im) = sum.center_complex();
    assert!((s_re - 4.0).abs() < 1e-12);
    assert!((s_im - (-2.0)).abs() < 1e-12);
    assert!((sum.radius() - 0.3).abs() < 1e-12);

    // Product center: (1 + 2i)(3 - 4i) = 3 - 4i + 6i + 8 = 11 + 2i
    let prod = b1.mul(&b2);
    let (p_re, p_im) = prod.center_complex();
    assert!((p_re - 11.0).abs() < 1e-12);
    assert!((p_im - 2.0).abs() < 1e-12);

    // Exact containment check
    let exact_point = ScalarAtom::ComplexF64 { re: 1.05, im: 2.05 };
    assert!(b1.contains_scalar(&exact_point));
}

#[test]
fn test_matrix_exponentiation_dimensional_validity() {
    let l = DimVector::length();
    let t = DimVector::time();
    let v = l / t;

    // Mixed-unit state vector [position (L), velocity (L/T)]
    let row_dims = smallvec![l, v];
    let col_dims = smallvec![l, v];

    let se3_block = ObjectDescriptor::graded_matrix(2, 2, row_dims, col_dims);
    assert!(
        se3_block.check_matrix_exp_validity().is_ok(),
        "Consistent graded matrix exp must succeed"
    );

    // Inconsistent dimensions
    let bad_col = smallvec![t, l];
    let bad_mat = ObjectDescriptor::graded_matrix(2, 2, smallvec![l, v], bad_col);
    assert!(
        bad_mat.check_matrix_exp_validity().is_err(),
        "Inconsistent graded matrix exp must fail"
    );
}
