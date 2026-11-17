//! Integration Tests for Phase 14: Multiphysics FEA / FEM Simulation Suite.

use algebra_engine::pde::{FeaBoundaryCondition, FeaEngine, FeaMesh, PhysicsDiscipline};
use algebra_engine::{fea_elasticity, fea_heat, fea_solve};

#[test]
fn test_fea_1d_bar_axial_tension() {
    // 1D Bar: Length L = 10.0, 5 elements, E = 1000.0, Force P = 100.0
    // Analytical tip deflection delta = (P * L) / (A * E) = (100 * 10) / (1 * 1000) = 1.0
    let mesh = FeaMesh::bar_1d(10.0, 5);

    let physics = PhysicsDiscipline::LinearElasticity {
        youngs_modulus: 1000.0,
        poissons_ratio: 0.3,
        density: 1.0,
        body_force: vec![0.0],
        plane_stress: true,
    };

    let bcs = vec![
        FeaBoundaryCondition::FixedDisplacement {
            group_name: "left".into(),
            dof_index: 0,
            value: 0.0,
        },
        FeaBoundaryCondition::PointForce {
            group_name: "right".into(),
            force_vector: vec![100.0],
        },
    ];

    let sol = FeaEngine::solve(&mesh, &physics, &bcs).expect("FEA solve failed");

    assert_eq!(sol.nodal_values.len(), 6);
    // Tip deflection at node 5 (right)
    let tip_disp = sol.nodal_values[5];
    assert!(
        (tip_disp - 1.0).abs() < 1e-4,
        "Expected tip displacement ~ 1.0, got {}",
        tip_disp
    );
    // Left boundary node fixed at 0.0
    assert!(sol.nodal_values[0].abs() < 1e-6);
}

#[test]
fn test_fea_2d_cantilever_beam_linear_elasticity() {
    // 2D Cantilever Plate: Width 10.0, Height 2.0, 10x4 elements
    let mesh = FeaMesh::plate_2d(10.0, 2.0, 10, 4);

    let physics = PhysicsDiscipline::LinearElasticity {
        youngs_modulus: 200_000.0, // Steel ~200 GPa
        poissons_ratio: 0.3,
        density: 7850.0,
        body_force: vec![0.0, 0.0],
        plane_stress: true,
    };

    let bcs = vec![
        // Clamp left edge: Ux = 0, Uy = 0
        FeaBoundaryCondition::FixedDisplacement {
            group_name: "left".into(),
            dof_index: 0,
            value: 0.0,
        },
        FeaBoundaryCondition::FixedDisplacement {
            group_name: "left".into(),
            dof_index: 1,
            value: 0.0,
        },
        // Downward load on right tip: Fy = -500 N
        FeaBoundaryCondition::PointForce {
            group_name: "right".into(),
            force_vector: vec![0.0, -500.0],
        },
    ];

    let sol = FeaEngine::solve(&mesh, &physics, &bcs).expect("Cantilever FEA solve failed");

    // Tip deflection should be downward (negative Y)
    assert!(sol.max_magnitude > 0.0);
    assert!(!sol.von_mises_stresses.is_empty());
    // Maximum von Mises stress should be positive
    let max_vm = sol.von_mises_stresses.iter().cloned().fold(0.0, f64::max);
    assert!(max_vm > 0.0);
}

#[test]
fn test_fea_1d_heat_conduction_linear_gradient() {
    // 1D Bar: L = 5.0, 10 elements, k = 10.0, T(0) = 100.0, T(5) = 0.0
    // Analytical solution: T(x) = 100 - 20*x
    let mesh = FeaMesh::bar_1d(5.0, 10);

    let physics = PhysicsDiscipline::HeatConduction {
        thermal_conductivity: 10.0,
        heat_capacity: 1.0,
        density: 1.0,
        internal_heat_source: 0.0,
    };

    let bcs = vec![
        FeaBoundaryCondition::PrescribedTemperature {
            group_name: "left".into(),
            temperature: 100.0,
        },
        FeaBoundaryCondition::PrescribedTemperature {
            group_name: "right".into(),
            temperature: 0.0,
        },
    ];

    let sol = FeaEngine::solve(&mesh, &physics, &bcs).expect("Heat conduction solve failed");

    assert_eq!(sol.nodal_values.len(), 11);
    assert!((sol.nodal_values[0] - 100.0).abs() < 1e-3);
    assert!(sol.nodal_values[10].abs() < 1e-3);

    // Midpoint x = 2.5 (node 5) -> T = 50.0
    let mid_temp = sol.nodal_values[5];
    assert!(
        (mid_temp - 50.0).abs() < 1e-3,
        "Expected mid temperature 50.0, got {}",
        mid_temp
    );
}

#[test]
fn test_fea_2d_heat_conduction_with_convection() {
    // 2D Plate: 6x6 elements, T(bottom) = 80.0, Convective cooling on top: h = 2.0, T_inf = 20.0
    let mesh = FeaMesh::plate_2d(4.0, 4.0, 6, 6);

    let physics = PhysicsDiscipline::HeatConduction {
        thermal_conductivity: 15.0,
        heat_capacity: 1.0,
        density: 1.0,
        internal_heat_source: 0.0,
    };

    let bcs = vec![
        FeaBoundaryCondition::PrescribedTemperature {
            group_name: "bottom".into(),
            temperature: 80.0,
        },
        FeaBoundaryCondition::ConvectiveCooling {
            group_name: "top".into(),
            heat_transfer_coeff: 2.0,
            ambient_temperature: 20.0,
        },
    ];

    let sol = FeaEngine::solve(&mesh, &physics, &bcs).expect("2D heat solve failed");
    assert!(sol.nodal_values.len() == 7 * 7);
    assert!(sol.max_magnitude > 20.0 && sol.max_magnitude <= 80.01);
}

#[test]
fn test_fea_dsl_macros() {
    let mesh = FeaMesh::bar_1d(4.0, 4);

    let bcs = vec![
        FeaBoundaryCondition::FixedDisplacement {
            group_name: "left".into(),
            dof_index: 0,
            value: 0.0,
        },
        FeaBoundaryCondition::PointForce {
            group_name: "right".into(),
            force_vector: vec![50.0],
        },
    ];

    let sol = fea_elasticity!(&mesh, E = 500.0, nu = 0.25, bcs = &bcs).unwrap();
    assert!(sol.nodal_values[4] > 0.0);

    let heat_bcs = vec![
        FeaBoundaryCondition::PrescribedTemperature {
            group_name: "left".into(),
            temperature: 50.0,
        },
        FeaBoundaryCondition::PrescribedTemperature {
            group_name: "right".into(),
            temperature: 10.0,
        },
    ];

    let heat_sol = fea_heat!(&mesh, k = 5.0, Q = 0.0, bcs = &heat_bcs).unwrap();
    assert!((heat_sol.nodal_values[0] - 50.0).abs() < 1e-2);

    let generic_sol = fea_solve!(
        &mesh,
        &PhysicsDiscipline::HeatConduction {
            thermal_conductivity: 5.0,
            heat_capacity: 1.0,
            density: 1.0,
            internal_heat_source: 0.0,
        },
        &heat_bcs
    )
    .unwrap();
    assert_eq!(generic_sol.nodal_values.len(), 5);
}

#[test]
fn large_systems_use_the_sparse_solver_and_match_the_exact_solution() {
    use algebra_engine::pde::CsrMatrix;

    // 1-D heat bar with 800 elements (801 unknowns, above the dense threshold): T is linear.
    let mesh = FeaMesh::bar_1d(10.0, 800);
    let physics = PhysicsDiscipline::HeatConduction {
        thermal_conductivity: 10.0,
        heat_capacity: 1.0,
        density: 1.0,
        internal_heat_source: 0.0,
    };
    let bcs = vec![
        FeaBoundaryCondition::PrescribedTemperature {
            group_name: "left".into(),
            temperature: 100.0,
        },
        FeaBoundaryCondition::PrescribedTemperature {
            group_name: "right".into(),
            temperature: 0.0,
        },
    ];
    let sol = FeaEngine::solve(&mesh, &physics, &bcs).expect("sparse heat solve");
    assert_eq!(sol.nodal_values.len(), 801);
    for (i, t) in sol.nodal_values.iter().enumerate() {
        let exact = 100.0 * (1.0 - i as f64 / 800.0);
        assert!((t - exact).abs() < 1e-3, "node {i}: {t} vs {exact}");
    }

    // CSR storage and CG on a small SPD matrix with a known solution.
    let a = vec![
        vec![4.0, 1.0, 0.0],
        vec![1.0, 3.0, 1.0],
        vec![0.0, 1.0, 2.0],
    ];
    let csr = CsrMatrix::from_dense(&a);
    assert_eq!(csr.nnz(), 7);
    let x = csr.conjugate_gradient(&[1.0, 2.0, 3.0], 1e-12, 50).unwrap();
    let back = csr.matvec(&x);
    for (got, want) in back.iter().zip([1.0, 2.0, 3.0]) {
        assert!((got - want).abs() < 1e-9);
    }

    // An indefinite or under-constrained matrix is an error, not an answer.
    let indefinite = CsrMatrix::from_dense(&[vec![1.0, 2.0], vec![2.0, 1.0]]);
    assert!(
        indefinite
            .conjugate_gradient(&[1.0, -1.0], 1e-12, 50)
            .is_err()
    );
    let zero_diag = CsrMatrix::from_dense(&[vec![0.0, 1.0], vec![1.0, 0.0]]);
    assert!(
        zero_diag
            .conjugate_gradient(&[1.0, 1.0], 1e-12, 50)
            .is_err()
    );
}

#[test]
fn constraints_are_eliminated_exactly_and_reactions_balance_the_load() {
    let mesh = FeaMesh::bar_1d(10.0, 5);
    let physics = PhysicsDiscipline::LinearElasticity {
        youngs_modulus: 1000.0,
        poissons_ratio: 0.3,
        density: 1.0,
        body_force: vec![0.0],
        plane_stress: true,
    };
    let bcs = vec![
        FeaBoundaryCondition::FixedDisplacement {
            group_name: "left".into(),
            dof_index: 0,
            value: 0.25,
        },
        FeaBoundaryCondition::PointForce {
            group_name: "right".into(),
            force_vector: vec![100.0],
        },
    ];
    let sol = FeaEngine::solve(&mesh, &physics, &bcs).unwrap();
    // The prescribed value is hit exactly (a penalty would miss it by ~1e-12 relative) and the
    // tip moves by the elastic extension on top of it.
    assert_eq!(sol.nodal_values[0], 0.25);
    assert!(
        (sol.nodal_values[5] - 1.25).abs() < 1e-12,
        "{}",
        sol.nodal_values[5]
    );
    // The support reaction equals the applied load, opposite in sign; free nodes carry none.
    assert!(
        (sol.reaction_forces[0] + 100.0).abs() < 1e-9,
        "{:?}",
        sol.reaction_forces
    );
    assert!(sol.reaction_forces[1..].iter().all(|r| *r == 0.0));
}

#[test]
fn a_large_plate_solves_without_a_dense_matrix() {
    use algebra_engine::pde::SparseAssembly;

    let mut a = SparseAssembly::new(3);
    a.add(0, 1, 2.0);
    a.add(0, 1, 3.0);
    assert_eq!(a.get(0, 1), 5.0);
    assert_eq!(a.get(2, 2), 0.0);
    assert_eq!(a.nnz(), 1);

    // A 2-D heat plate with 40 x 40 elements (1681 unknowns, 14 million dense entries) heated
    // on the bottom edge and cooled by convection on the top: the temperature must decrease
    // monotonically upward and stay between the two reservoirs.
    let mesh = FeaMesh::plate_2d(4.0, 4.0, 40, 40);
    let physics = PhysicsDiscipline::HeatConduction {
        thermal_conductivity: 15.0,
        heat_capacity: 1.0,
        density: 1.0,
        internal_heat_source: 0.0,
    };
    let bcs = vec![
        FeaBoundaryCondition::PrescribedTemperature {
            group_name: "bottom".into(),
            temperature: 80.0,
        },
        FeaBoundaryCondition::ConvectiveCooling {
            group_name: "top".into(),
            heat_transfer_coeff: 2.0,
            ambient_temperature: 20.0,
        },
    ];
    let sol = FeaEngine::solve(&mesh, &physics, &bcs).expect("large plate solve");
    assert_eq!(sol.nodal_values.len(), 41 * 41);
    let column = |j: usize| sol.nodal_values[j * 41 + 20];
    for j in 0..40 {
        assert!(column(j + 1) <= column(j) + 1e-9, "row {j}");
    }
    assert!((column(0) - 80.0).abs() < 1e-9);
    assert!(column(40) > 20.0 && column(40) < 80.0);
}
