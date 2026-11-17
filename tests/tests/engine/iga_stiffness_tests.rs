//! Isogeometric stiffness: analytic derivatives and Gauss quadrature, checked against closed forms.

use algebra_engine::iga::{IgaStiffnessMatrix, NurbsPatchND, WeightedControlPoint};

fn point(x: f64, y: f64, z: f64, w: f64) -> WeightedControlPoint {
    WeightedControlPoint::new(vec![x, y, z], w)
}

/// Bilinear patch over a parallelogram with corners 0, a, b, a + b (storage order i * 2 + j,
/// u along `a` and v along `b`).
fn bilinear(a: [f64; 3], b: [f64; 3]) -> NurbsPatchND {
    let corner = |s: f64, t: f64| {
        point(
            s * a[0] + t * b[0],
            s * a[1] + t * b[1],
            s * a[2] + t * b[2],
            1.0,
        )
    };
    NurbsPatchND::surface_2d(
        1,
        1,
        vec![0.0, 0.0, 1.0, 1.0],
        vec![0.0, 0.0, 1.0, 1.0],
        2,
        2,
        vec![
            corner(0.0, 0.0),
            corner(0.0, 1.0),
            corner(1.0, 0.0),
            corner(1.0, 1.0),
        ],
    )
}

#[test]
fn the_unit_square_has_the_textbook_q1_stiffness_matrix() {
    let k = IgaStiffnessMatrix::assemble_2d(&bilinear([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]));
    // Node 0 = (0,0); 1 = (0,1) and 2 = (1,0) are edge neighbours; 3 = (1,1) is diagonal.
    for node in 0..4 {
        assert!(
            (k[node][node] - 2.0 / 3.0).abs() < 1e-12,
            "K[{node}][{node}] = {}",
            k[node][node]
        );
    }
    assert!((k[0][1] + 1.0 / 6.0).abs() < 1e-12 && (k[0][2] + 1.0 / 6.0).abs() < 1e-12);
    assert!((k[0][3] + 1.0 / 3.0).abs() < 1e-12);
    assert!((k[1][2] + 1.0 / 3.0).abs() < 1e-12);
}

#[test]
fn stiffness_is_invariant_under_scaling_shear_free_rotation_and_embedding() {
    let reference = IgaStiffnessMatrix::assemble_2d(&bilinear([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]));
    // Doubling a square leaves the 2D Dirichlet energy unchanged.
    let scaled = IgaStiffnessMatrix::assemble_2d(&bilinear([2.0, 0.0, 0.0], [0.0, 2.0, 0.0]));
    // A square lying in a tilted plane of 3D space is isometric to the unit square.
    let c = 1.0 / 3.0_f64.sqrt();
    let tilted = IgaStiffnessMatrix::assemble_2d(&bilinear(
        [1.0 / 2.0_f64.sqrt(), 0.0, 1.0 / 2.0_f64.sqrt()],
        [0.0, 1.0, 0.0],
    ));
    let _ = c;
    for a in 0..4 {
        for b in 0..4 {
            assert!((scaled[a][b] - reference[a][b]).abs() < 1e-12);
            assert!(
                (tilted[a][b] - reference[a][b]).abs() < 1e-12,
                "tilted [{a}][{b}]"
            );
        }
    }
}

#[test]
fn a_rectangle_weights_directions_by_its_aspect_ratio() {
    // 3 x 1 rectangle: K_00 = (1/3) (h/w + w/h) = (1/3)(1/3 + 3) with w = 3 along u, h = 1 along v.
    let k = IgaStiffnessMatrix::assemble_2d(&bilinear([3.0, 0.0, 0.0], [0.0, 1.0, 0.0]));
    assert!(
        (k[0][0] - (1.0 / 3.0) * (3.0 + 1.0 / 3.0)).abs() < 1e-12,
        "{}",
        k[0][0]
    );
}

#[test]
fn a_rational_quarter_annulus_reproduces_its_exact_area() {
    // Quarter annulus r = 1 .. R = 2, degree 2 in u (exact circle arcs, weights 1, sqrt(2)/2, 1)
    // and degree 1 in v. For coordinates x_a, the energy x^T K x = integral of |grad x|^2 = area.
    let w = 2.0_f64.sqrt() / 2.0;
    let arc = |radius: f64| {
        [
            point(radius, 0.0, 0.0, 1.0),
            point(radius, radius, 0.0, w),
            point(0.0, radius, 0.0, 1.0),
        ]
    };
    let (inner, outer) = (arc(1.0), arc(2.0));
    // Storage order i * n_v + j: u index i along the arc, v index j from inner to outer.
    let mut control = Vec::new();
    for i in 0..3 {
        control.push(inner[i].clone());
        control.push(outer[i].clone());
    }
    let patch = NurbsPatchND::surface_2d(
        2,
        1,
        vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
        vec![0.0, 0.0, 1.0, 1.0],
        3,
        2,
        control,
    );
    let k = IgaStiffnessMatrix::assemble_2d(&patch);
    let energy = |axis: usize| -> f64 {
        let x: Vec<f64> = patch
            .control_points
            .iter()
            .map(|p| p.coords[axis])
            .collect();
        let mut total = 0.0;
        for a in 0..6 {
            for b in 0..6 {
                total += x[a] * k[a][b] * x[b];
            }
        }
        total
    };
    let area = 3.0 * std::f64::consts::PI / 4.0;
    assert!(
        (energy(0) - area).abs() < 1e-9,
        "x energy {} vs area {area}",
        energy(0)
    );
    assert!(
        (energy(1) - area).abs() < 1e-9,
        "y energy {} vs area {area}",
        energy(1)
    );
    // Constants have zero energy: rows sum to zero (partition of unity).
    for (a, row) in k.iter().enumerate() {
        let sum: f64 = row.iter().sum();
        assert!(sum.abs() < 1e-9, "row {a} sums to {sum}");
    }
    // Symmetric and with positive diagonal.
    for a in 0..6 {
        assert!(k[a][a] > 0.0);
        for b in 0..6 {
            assert!((k[a][b] - k[b][a]).abs() < 1e-12);
        }
    }
}

#[test]
fn analytic_tangents_match_a_fine_finite_difference() {
    let w = 2.0_f64.sqrt() / 2.0;
    let patch = NurbsPatchND::surface_2d(
        2,
        1,
        vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
        vec![0.0, 0.0, 1.0, 1.0],
        3,
        2,
        vec![
            point(1.0, 0.0, 0.0, 1.0),
            point(2.0, 0.0, 0.5, 1.0),
            point(1.0, 1.0, 0.0, w),
            point(2.0, 2.0, 0.5, w),
            point(0.0, 1.0, 0.0, 1.0),
            point(0.0, 2.0, 0.5, 1.0),
        ],
    );
    let (u, v, h) = (0.37, 0.61, 1e-6);
    let (s_u, s_v) = patch.tangents(u, v);
    let centre = |du: f64, dv: f64| patch.evaluate_surface(u + du, v + dv);
    for d in 0..3 {
        let fd_u = (centre(h, 0.0)[d] - centre(-h, 0.0)[d]) / (2.0 * h);
        let fd_v = (centre(0.0, h)[d] - centre(0.0, -h)[d]) / (2.0 * h);
        assert!(
            (s_u[d] - fd_u).abs() < 1e-7,
            "S_u[{d}]: {} vs {fd_u}",
            s_u[d]
        );
        assert!(
            (s_v[d] - fd_v).abs() < 1e-7,
            "S_v[{d}]: {} vs {fd_v}",
            s_v[d]
        );
    }
}

#[test]
fn out_of_range_entries_are_zero() {
    let patch = bilinear([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
    assert_eq!(
        IgaStiffnessMatrix::integrate_stiffness_entry_2d(&patch, 0, 9),
        0.0
    );
}
