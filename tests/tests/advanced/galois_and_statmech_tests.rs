use algebra_advanced::galois::{FieldExtension, GaloisGroupType};
use algebra_advanced::statmech::{CanonicalPartitionFunction, bose_einstein, fermi_dirac};

#[test]
fn test_galois_theory() {
    let q_sqrt2 = FieldExtension::quadratic(2);
    assert_eq!(q_sqrt2.degree, 2);
    assert!(q_sqrt2.is_galois);
    assert!(q_sqrt2.is_solvable_by_radicals());

    let g_s5 = GaloisGroupType::Symmetric(5);
    assert!(!g_s5.is_solvable()); // S_5 is not solvable by radicals (Abel-Ruffini)

    let g_s4 = GaloisGroupType::Symmetric(4);
    assert!(g_s4.is_solvable()); // S_4 is solvable
}

#[test]
fn test_statistical_mechanics() {
    // Two-level system: E_0 = 0 (g_0 = 1), E_1 = 1.0 (g_1 = 1)
    let sys = CanonicalPartitionFunction::new(vec![(0.0, 1), (1.0, 1)]);
    let beta = 1.0;

    // Z = e^0 + e^-1 = 1 + 1/e ≈ 1.367879
    let z = sys.evaluate_z(beta);
    assert!((z - (1.0 + (-1.0f64).exp())).abs() < 1e-6);

    let free_e = sys.free_energy(beta);
    assert!(free_e < 0.0);

    // Fermi-Dirac at T -> 0
    let fd = fermi_dirac(0.5, 1.0, 100.0); // E < \mu -> occupation ~ 1
    assert!(fd > 0.99);

    // Bose-Einstein
    let be = bose_einstein(2.0, 1.0, 1.0).unwrap();
    assert!(be > 0.0);
}

fn z(c: &[i64]) -> Vec<num_bigint::BigInt> {
    c.iter().map(|&x| num_bigint::BigInt::from(x)).collect()
}

#[test]
fn galois_groups_of_known_polynomials() {
    use GaloisGroupType::*;
    use algebra_advanced::galois::galois_group_over_q as gal;
    // Quadratics and products of quadratics.
    assert_eq!(gal(&z(&[-2, 0, 1])).unwrap(), Cyclic(2)); // x^2 - 2
    assert_eq!(gal(&z(&[-4, 0, 1])).unwrap(), Identity); // x^2 - 4 splits over Q
    assert_eq!(gal(&z(&[6, 0, -5, 0, 1])).unwrap(), KleinFour); // (x^2-2)(x^2-3)
    assert_eq!(gal(&z(&[-1, -1, 1])).unwrap(), Cyclic(2)); // x^2 - x - 1 (golden ratio)
    // Cubics: disc square gives C3, otherwise S3.
    assert_eq!(gal(&z(&[-2, 0, 0, 1])).unwrap(), Symmetric(3)); // x^3 - 2
    assert_eq!(gal(&z(&[1, -3, 0, 1])).unwrap(), Cyclic(3)); // x^3 - 3x + 1, disc 81
    assert_eq!(gal(&z(&[-1, -1, 0, 1])).unwrap(), Symmetric(3)); // x^3 - x - 1, disc -23
    assert_eq!(gal(&z(&[2, -2, 0, 1])).unwrap(), Symmetric(3)); // x^3 - 2x + 2 (Eisenstein)
    // Reducible cubic: (x - 1)(x^2 - 2) has group C2.
    assert_eq!(gal(&z(&[2, -2, -1, 1])).unwrap(), Cyclic(2));
    // Quartics: one example of each possible group.
    assert_eq!(gal(&z(&[-1, -1, 0, 0, 1])).unwrap(), Symmetric(4)); // x^4 - x - 1
    assert_eq!(gal(&z(&[12, 8, 0, 0, 1])).unwrap(), Alternating(4)); // x^4 + 8x + 12
    assert_eq!(gal(&z(&[-2, 0, 0, 0, 1])).unwrap(), Dihedral(4)); // x^4 - 2
    assert_eq!(gal(&z(&[2, 0, 4, 0, 1])).unwrap(), Cyclic(4)); // x^4 + 4x^2 + 2
    assert_eq!(gal(&z(&[1, 0, 0, 0, 1])).unwrap(), KleinFour); // x^4 + 1 (Q(zeta_8))
    assert_eq!(gal(&z(&[1, 0, -10, 0, 1])).unwrap(), KleinFour); // min poly of sqrt2 + sqrt3
    // Non-monic input has the same group as its monic model.
    assert_eq!(gal(&z(&[-3, 0, 0, 2])).unwrap(), Symmetric(3)); // 2x^3 - 3 ~ x^3 - 12
    assert_eq!(gal(&z(&[-2, 0, 0, 2])).unwrap(), Cyclic(2)); // 2(x - 1)(x^2 + x + 1)
    // The trivial and quadratic extension constructors.
    assert_eq!(FieldExtension::quadratic(4).degree, 1);
    assert_eq!(FieldExtension::quadratic(1).galois_group, Some(Identity));
    assert_eq!(FieldExtension::quadratic(-3).degree, 2);
}
