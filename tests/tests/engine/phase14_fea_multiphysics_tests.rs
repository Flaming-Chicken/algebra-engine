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

#[test]
fn uniaxial_patch_test_gives_the_exact_stress_in_plane_stress_and_plane_strain() {
    // A plate stretched by prescribed end displacement d over length L, free to contract
    // sideways. Linear fields are reproduced exactly by constant-strain triangles, so the
    // result is exact on any mesh.
    let (len, height, d, e, nu) = (6.0_f64, 2.0, 0.03, 200.0, 0.3);
    for plane_stress in [true, false] {
        let mesh = FeaMesh::plate_2d(len, height, 6, 3);
        let physics = PhysicsDiscipline::LinearElasticity {
            youngs_modulus: e,
            poissons_ratio: nu,
            density: 1.0,
            body_force: vec![0.0, 0.0],
            plane_stress,
        };
        let fix =
            |group: &str, dof_index: usize, value: f64| FeaBoundaryCondition::FixedDisplacement {
                group_name: group.into(),
                dof_index,
                value,
            };
        let bcs = vec![
            fix("left", 0, 0.0),
            fix("right", 0, d),
            fix("bottom", 1, 0.0),
        ];
        let sol = FeaEngine::solve(&mesh, &physics, &bcs).unwrap();

        let eps = d / len;
        // Plane stress: sigma_x = E eps, sigma_z = 0. Plane strain with sigma_y = 0:
        // sigma_x = E eps / (1 - nu^2) and sigma_z = nu sigma_x.
        let (sigma_x, vm) = if plane_stress {
            (e * eps, e * eps)
        } else {
            let sx = e * eps / (1.0 - nu * nu);
            let sz = nu * sx;
            (sx, (0.5 * (sx * sx + sz * sz + (sz - sx).powi(2))).sqrt())
        };
        for (k, v) in sol.von_mises_stresses.iter().enumerate() {
            assert!(
                (v - vm).abs() < 1e-9 * vm.max(1.0),
                "plane_stress={plane_stress} element {k}: {v} vs {vm} (sigma_x {sigma_x})"
            );
        }
        // The sides contract freely: v = -nu' eps y with nu' = nu (stress) or nu/(1-nu) (strain).
        let nu_eff = if plane_stress { nu } else { nu / (1.0 - nu) };
        let top_node = mesh.nodes.len() - 1; // x = L, y = H
        let tip_v = sol.nodal_values[top_node * 2 + 1];
        assert!(
            (tip_v + nu_eff * eps * height).abs() < 1e-9,
            "tip v {tip_v}"
        );
    }
}

#[test]
fn tetrahedral_solid_reproduces_uniaxial_stress_exactly() {
    // A block stretched by an end displacement and free to contract sideways: a linear field,
    // reproduced exactly by linear tetrahedra on any mesh.
    let (lx, ly, lz, d, e, nu) = (4.0_f64, 2.0, 1.5, 0.02, 210.0, 0.25);
    let mesh = FeaMesh::box_3d(lx, ly, lz, 4, 2, 2);
    assert_eq!(mesh.elements.len(), 4 * 2 * 2 * 6);
    let physics = PhysicsDiscipline::LinearElasticity {
        youngs_modulus: e,
        poissons_ratio: nu,
        density: 1.0,
        body_force: vec![0.0, 0.0, 0.0],
        plane_stress: false,
    };
    let fix = |group: &str, dof_index: usize, value: f64| FeaBoundaryCondition::FixedDisplacement {
        group_name: group.into(),
        dof_index,
        value,
    };
    let bcs = vec![
        fix("left", 0, 0.0),
        fix("right", 0, d),
        fix("bottom", 1, 0.0),
        fix("front", 2, 0.0),
    ];
    let sol = FeaEngine::solve(&mesh, &physics, &bcs).unwrap();
    let eps = d / lx;
    let sigma = e * eps;
    for (k, v) in sol.von_mises_stresses.iter().enumerate() {
        assert!(
            (v - sigma).abs() < 1e-9 * sigma,
            "element {k}: {v} vs {sigma}"
        );
    }
    // Every node: u = eps x, v = -nu eps y, w = -nu eps z.
    for (i, p) in mesh.nodes.iter().enumerate() {
        let want = [eps * p[0], -nu * eps * p[1], -nu * eps * p[2]];
        for c in 0..3 {
            let got = sol.nodal_values[i * 3 + c];
            assert!(
                (got - want[c]).abs() < 1e-10,
                "node {i} comp {c}: {got} vs {want:?}"
            );
        }
    }
    // The fixed end reacts with the total force sigma * ly * lz.
    let total: f64 = mesh.boundary_groups["left"]
        .iter()
        .map(|&n| sol.reaction_forces[n * 3])
        .sum();
    assert!(
        (total + sigma * ly * lz).abs() < 1e-8 * sigma * ly * lz,
        "reaction {total}"
    );
}

#[test]
fn body_force_and_unsupported_elements_are_handled_honestly() {
    // A hanging bar under its own weight (1-D) is covered elsewhere; here a 2-D plate hanging
    // from its top edge must stretch downward (body force was ignored for 2-D elements).
    let mesh = FeaMesh::plate_2d(2.0, 4.0, 4, 8);
    let physics = PhysicsDiscipline::LinearElasticity {
        youngs_modulus: 100.0,
        poissons_ratio: 0.3,
        density: 1.0,
        body_force: vec![0.0, -1.0],
        plane_stress: true,
    };
    let fix = |dof_index: usize| FeaBoundaryCondition::FixedDisplacement {
        group_name: "top".into(),
        dof_index,
        value: 0.0,
    };
    let sol = FeaEngine::solve(&mesh, &physics, &[fix(0), fix(1)]).unwrap();
    let bottom_v = sol.nodal_values[1]; // node 0 is the bottom-left corner (y = 0)
    assert!(
        bottom_v < -1e-3,
        "plate must sag under its weight, got {bottom_v}"
    );
    // Reaction at the support balances the weight: body force x area.
    let total: f64 = mesh.boundary_groups["top"]
        .iter()
        .map(|&n| sol.reaction_forces[n * 2 + 1])
        .sum();
    assert!((total - 8.0).abs() < 1e-8, "reaction {total} vs weight 8");

    // An element type the elasticity solver does not implement is an error, not silence.
    let mut quad = FeaMesh::new(2, algebra_engine::pde::ElementType::Tri6);
    for p in [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]] {
        quad.add_node(p.to_vec());
    }
    quad.add_element(vec![0, 1, 2, 3]);
    assert!(FeaEngine::solve(&quad, &physics, &[]).is_err());
}

#[test]
fn quadrilateral_elements_pass_the_patch_test_on_a_distorted_mesh() {
    // Bilinear isoparametric quadrilaterals reproduce a constant-strain state exactly even on
    // irregular quadrilaterals. Distort the interior nodes and stretch the plate uniaxially.
    let (len, height, d, e, nu) = (6.0_f64, 3.0, 0.02, 150.0, 0.3);
    for plane_stress in [true, false] {
        let mut mesh = FeaMesh::plate_2d_quad(len, height, 6, 3);
        for j in 1..3 {
            for i in 1..6 {
                let n = j * 7 + i;
                mesh.nodes[n][0] += 0.17 * ((i * 3 + j) % 3) as f64 - 0.12;
                mesh.nodes[n][1] += 0.11 * ((i + 2 * j) % 4) as f64 - 0.15;
            }
        }
        let physics = PhysicsDiscipline::LinearElasticity {
            youngs_modulus: e,
            poissons_ratio: nu,
            density: 1.0,
            body_force: vec![0.0, 0.0],
            plane_stress,
        };
        let fix =
            |group: &str, dof_index: usize, value: f64| FeaBoundaryCondition::FixedDisplacement {
                group_name: group.into(),
                dof_index,
                value,
            };
        let bcs = vec![
            fix("left", 0, 0.0),
            fix("right", 0, d),
            fix("bottom", 1, 0.0),
        ];
        let sol = FeaEngine::solve(&mesh, &physics, &bcs).unwrap();

        let eps = d / len;
        let nu_eff = if plane_stress { nu } else { nu / (1.0 - nu) };
        let sx = if plane_stress {
            e * eps
        } else {
            e * eps / (1.0 - nu * nu)
        };
        let vm = if plane_stress {
            sx
        } else {
            let sz = nu * sx;
            (0.5 * (sx * sx + sz * sz + (sz - sx).powi(2))).sqrt()
        };
        for (i, p) in mesh.nodes.iter().enumerate() {
            let want = [eps * p[0], -nu_eff * eps * p[1]];
            for c in 0..2 {
                let got = sol.nodal_values[i * 2 + c];
                assert!(
                    (got - want[c]).abs() < 1e-10,
                    "node {i} comp {c}: {got} vs {want:?}"
                );
            }
        }
        for (k, v) in sol.von_mises_stresses.iter().enumerate() {
            assert!(
                (v - vm).abs() < 1e-9 * vm,
                "plane_stress={plane_stress} element {k}: {v} vs {vm}"
            );
        }
    }

    // A clockwise (inverted) element is reported, not integrated with a negative area.
    let mut bad = FeaMesh::new(2, algebra_engine::pde::ElementType::Quad4);
    for p in [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]] {
        bad.add_node(p.to_vec());
    }
    bad.add_element(vec![0, 1, 2, 3]);
    let physics = PhysicsDiscipline::LinearElasticity {
        youngs_modulus: 1.0,
        poissons_ratio: 0.3,
        density: 1.0,
        body_force: vec![0.0, 0.0],
        plane_stress: true,
    };
    assert!(FeaEngine::solve(&bad, &physics, &[]).is_err());
}

#[test]
fn heat_conduction_on_quadrilaterals_and_tetrahedra() {
    let heat = |source: f64| PhysicsDiscipline::HeatConduction {
        thermal_conductivity: 2.0,
        heat_capacity: 1.0,
        density: 1.0,
        internal_heat_source: source,
    };
    let temperature = |group: &str, t: f64| FeaBoundaryCondition::PrescribedTemperature {
        group_name: group.into(),
        temperature: t,
    };

    // Bilinear quadrilaterals on a distorted mesh, hot left and cold right, insulated sides:
    // the exact solution is linear in x and the element reproduces it.
    let mut quad = FeaMesh::plate_2d_quad(5.0, 2.0, 5, 2);
    for i in 1..5 {
        quad.nodes[6 + i][0] += 0.13 * (i % 2) as f64 - 0.05;
        quad.nodes[6 + i][1] += 0.07 * ((i + 1) % 3) as f64 - 0.06;
    }
    let sol = FeaEngine::solve(
        &quad,
        &heat(0.0),
        &[temperature("left", 100.0), temperature("right", 0.0)],
    )
    .unwrap();
    for (i, p) in quad.nodes.iter().enumerate() {
        let want = 100.0 * (1.0 - p[0] / 5.0);
        assert!((sol.nodal_values[i] - want).abs() < 1e-9, "quad node {i}");
    }

    // Tetrahedra in a block: the same linear profile.
    let block = FeaMesh::box_3d(4.0, 1.0, 1.0, 4, 1, 1);
    let sol = FeaEngine::solve(
        &block,
        &heat(0.0),
        &[temperature("left", 50.0), temperature("right", 10.0)],
    )
    .unwrap();
    for (i, p) in block.nodes.iter().enumerate() {
        let want = 50.0 - 40.0 * p[0] / 4.0;
        assert!((sol.nodal_values[i] - want).abs() < 1e-9, "tet node {i}");
    }

    // A uniform source between two cold ends peaks at q L^2 / (8 k) in the middle.
    let slab = FeaMesh::plate_2d_quad(4.0, 1.0, 16, 1);
    let sol = FeaEngine::solve(
        &slab,
        &heat(2.0),
        &[temperature("left", 0.0), temperature("right", 0.0)],
    )
    .unwrap();
    let peak = sol.nodal_values.iter().cloned().fold(f64::MIN, f64::max);
    let exact = 2.0 * 16.0 / (8.0 * 2.0);
    assert!(
        (peak - exact).abs() < 0.03 * exact,
        "peak {peak} vs {exact}"
    );

    // An element type with no heat formulation is an error, not silently ignored.
    let mut hex = FeaMesh::new(3, algebra_engine::pde::ElementType::Tri6);
    hex.add_node(vec![0.0, 0.0, 0.0]);
    hex.add_element(vec![0; 8]);
    assert!(FeaEngine::solve(&hex, &heat(0.0), &[]).is_err());
}

#[test]
fn hexahedral_elements_pass_the_patch_tests_on_a_distorted_mesh() {
    let (lx, ly, lz, d, e, nu) = (3.0_f64, 2.0, 2.0, 0.015, 180.0, 0.28);
    let mut mesh = FeaMesh::box_3d_hex(lx, ly, lz, 3, 2, 2);
    assert_eq!(mesh.elements.len(), 12);
    // Move the interior nodes off the grid (a node is interior when on no boundary group).
    let on_boundary: std::collections::HashSet<usize> =
        mesh.boundary_groups.values().flatten().copied().collect();
    for n in 0..mesh.nodes.len() {
        if !on_boundary.contains(&n) {
            mesh.nodes[n][0] += 0.11;
            mesh.nodes[n][1] -= 0.09;
            mesh.nodes[n][2] += 0.07;
        }
    }
    let physics = PhysicsDiscipline::LinearElasticity {
        youngs_modulus: e,
        poissons_ratio: nu,
        density: 1.0,
        body_force: vec![0.0, 0.0, 0.0],
        plane_stress: false,
    };
    let fix = |group: &str, dof_index: usize, value: f64| FeaBoundaryCondition::FixedDisplacement {
        group_name: group.into(),
        dof_index,
        value,
    };
    let bcs = vec![
        fix("left", 0, 0.0),
        fix("right", 0, d),
        fix("bottom", 1, 0.0),
        fix("front", 2, 0.0),
    ];
    let sol = FeaEngine::solve(&mesh, &physics, &bcs).unwrap();
    let eps = d / lx;
    for (i, p) in mesh.nodes.iter().enumerate() {
        let want = [eps * p[0], -nu * eps * p[1], -nu * eps * p[2]];
        for c in 0..3 {
            let got = sol.nodal_values[i * 3 + c];
            assert!(
                (got - want[c]).abs() < 1e-10,
                "node {i} comp {c}: {got} vs {want:?}"
            );
        }
    }
    for (k, v) in sol.von_mises_stresses.iter().enumerate() {
        assert!((v - e * eps).abs() < 1e-9 * e * eps, "element {k}: {v}");
    }

    // Heat: linear profile between a hot and a cold face, exact on the distorted mesh.
    let heat = PhysicsDiscipline::HeatConduction {
        thermal_conductivity: 3.0,
        heat_capacity: 1.0,
        density: 1.0,
        internal_heat_source: 0.0,
    };
    let temperature = |group: &str, t: f64| FeaBoundaryCondition::PrescribedTemperature {
        group_name: group.into(),
        temperature: t,
    };
    let sol = FeaEngine::solve(
        &mesh,
        &heat,
        &[temperature("left", 90.0), temperature("right", 30.0)],
    )
    .unwrap();
    for (i, p) in mesh.nodes.iter().enumerate() {
        let want = 90.0 - 60.0 * p[0] / lx;
        assert!(
            (sol.nodal_values[i] - want).abs() < 1e-9,
            "node {i}: {}",
            sol.nodal_values[i]
        );
    }

    // An inverted hexahedron is an error.
    let mut flipped = FeaMesh::box_3d_hex(1.0, 1.0, 1.0, 1, 1, 1);
    flipped.elements[0].swap(0, 4);
    flipped.elements[0].swap(1, 5);
    flipped.elements[0].swap(2, 6);
    flipped.elements[0].swap(3, 7);
    assert!(FeaEngine::solve(&flipped, &physics, &[]).is_err());
}
