//! Integration Tests for Universal Cross-Domain Mathematical Interconnection & Bridges.

use algebra_engine::iga::{NurbsPatchND, WeightedControlPoint};
use algebra_engine::interconnect::UniversalBridge;
use algebra_engine::noneuclidean::{PoincareDiskPoint, UpperHalfPlanePoint};
use algebra_engine::qft::WeylSpinor;
use algebra_engine::weyl_dmodules::WeylDOperator;
use algebra_engine::{octonion, weyl_d};

#[test]
fn test_cayley_isomorphism_poincare_upper_half_plane_roundtrip() {
    let p_orig = PoincareDiskPoint::new(0.3, 0.4).unwrap();

    // D -> H^2
    let uhp = UniversalBridge::poincare_to_upper_half_plane(&p_orig);
    assert!(uhp.y > 0.0);

    // H^2 -> D
    let p_back = UniversalBridge::upper_half_plane_to_poincare(&uhp);
    assert!((p_back.x - p_orig.x).abs() < 1e-9);
    assert!((p_back.y - p_orig.y).abs() < 1e-9);

    // Test From/Into conversions
    let uhp_into: UpperHalfPlanePoint = p_orig.into();
    let p_from: PoincareDiskPoint = uhp_into.into();
    assert!((p_from.x - p_orig.x).abs() < 1e-9);
    assert!((p_from.y - p_orig.y).abs() < 1e-9);
}

#[test]
fn test_nurbs_to_fea_mesh_generation() {
    let knots_u = vec![0.0, 0.0, 1.0, 1.0];
    let knots_v = vec![0.0, 0.0, 1.0, 1.0];
    let pts = vec![
        WeightedControlPoint::new(vec![0.0, 0.0, 0.0], 1.0),
        WeightedControlPoint::new(vec![0.0, 2.0, 0.0], 1.0),
        WeightedControlPoint::new(vec![2.0, 0.0, 0.0], 1.0),
        WeightedControlPoint::new(vec![2.0, 2.0, 0.0], 1.0),
    ];
    let patch = NurbsPatchND::surface_2d(1, 1, knots_u, knots_v, 2, 2, pts);

    let fea_mesh = UniversalBridge::nurbs_to_fea_mesh(&patch, 4, 4);
    assert_eq!(fea_mesh.nodes.len(), 16);
    assert_eq!(fea_mesh.elements.len(), 18); // 3 * 3 * 2 = 18 triangles
}

#[test]
fn test_octonion_albert_jordan_embedding() {
    let oct = octonion!(1, 2, 3, 4, 5, 6, 7, 8);
    let albert = UniversalBridge::octonion_to_albert_element(&oct, 2.0);

    assert_eq!(albert.diag, [2.0, 2.0, 2.0]);
    assert_eq!(albert.off_diag[0], oct);
    assert_eq!(albert.trace(), 6.0);
}

#[test]
fn test_weyl_to_diff_polynomial_conversion() {
    let d2 = weyl_d!(1, 0, 2);
    let diff_poly = UniversalBridge::weyl_to_diff_polynomial(&d2, 0).unwrap();

    assert_eq!(diff_poly.terms.len(), 1);
    assert_eq!(diff_poly.terms[0].coeff, 1.0);
    assert_eq!(diff_poly.terms[0].factors[0].0.var_index, 0);
    assert_eq!(diff_poly.terms[0].factors[0].0.order, 2);

    // And back.
    let weyl_back = WeylDOperator::try_from(diff_poly).unwrap();
    assert_eq!(weyl_back.terms.len(), 1);
    assert_eq!(weyl_back.terms[0].monomial.d_powers[0], 2);
}

#[test]
fn test_clebsch_gordan_to_spherical_tensor_coupling() {
    // 1 \otimes 1 coupling with m1 = 1, m2 = -1 -> m_tot = 0
    let irreps = UniversalBridge::clebsch_gordan_decomposition(1, 1, 1, -1);
    assert!(!irreps.is_empty());
    for (l, m, cg) in irreps {
        assert_eq!(m, 0);
        assert!(l <= 2);
        assert!(cg.abs() > 0.0);
    }
}

#[test]
fn test_weyl_spinor_to_real_polarization_vector() {
    let spinor = WeylSpinor::from_null_momentum(&[1.0, 0.0, 1.0, 0.0]);

    let p_mu = UniversalBridge::weyl_spinor_polarization(&spinor);
    // Massless null vector: t^2 - (x^2 + y^2 + z^2) = 0
    let minkowski_norm = p_mu[0].powi(2) - (p_mu[1].powi(2) + p_mu[2].powi(2) + p_mu[3].powi(2));
    assert!(minkowski_norm.abs() < 1e-10);
}

#[test]
fn test_conversions_refuse_what_they_cannot_represent() {
    use algebra_engine::diffalg::{DiffIndeterminate, DiffPolynomial, DiffTerm};
    use algebra_engine::weyl_dmodules::{WeylDOperator, WeylMonomial, WeylTerm};

    // x * d has a non-constant coefficient: not a differential polynomial.
    let x_times_d = WeylDOperator {
        num_vars: 1,
        terms: vec![WeylTerm::new(1.0, WeylMonomial::new(vec![1], vec![1]))],
    };
    assert!(UniversalBridge::weyl_to_diff_polynomial(&x_times_d, 0).is_err());

    // y * y' is nonlinear: not a Weyl operator.
    let y = DiffIndeterminate::new(0, 0);
    let y1 = DiffIndeterminate::new(0, 1);
    let product = DiffPolynomial::new(vec![DiffTerm::new(1.0, vec![(y, 1), (y1, 1)])]);
    assert!(WeylDOperator::try_from(product).is_err());
    // (y')^2 likewise.
    let square = DiffPolynomial::new(vec![DiffTerm::new(1.0, vec![(y1, 2)])]);
    assert!(WeylDOperator::try_from(square).is_err());
    // Two indeterminates are not one operator.
    let two = DiffPolynomial::new(vec![
        DiffTerm::new(1.0, vec![(DiffIndeterminate::new(0, 1), 1)]),
        DiffTerm::new(1.0, vec![(DiffIndeterminate::new(1, 1), 1)]),
    ]);
    assert!(WeylDOperator::try_from(two).is_err());

    // A linear constant-coefficient operator round-trips exactly: 3 d^2 + 2 d + 5.
    let op = WeylDOperator {
        num_vars: 1,
        terms: vec![
            WeylTerm::new(3.0, WeylMonomial::new(vec![0], vec![2])),
            WeylTerm::new(2.0, WeylMonomial::new(vec![0], vec![1])),
            WeylTerm::new(5.0, WeylMonomial::new(vec![0], vec![0])),
        ],
    };
    let poly = UniversalBridge::weyl_to_diff_polynomial(&op, 0).unwrap();
    let back = WeylDOperator::try_from(poly).unwrap();
    assert_eq!(back.terms.len(), 3);
    for (a, b) in op.terms.iter().zip(&back.terms) {
        assert_eq!(a.coeff, b.coeff);
        assert_eq!(a.monomial.d_powers, b.monomial.d_powers);
    }
}
