//! Integration Tests for Phase 15: Deep Cross-Tooling Simulation Pipeline & CAD-to-FEA Bridge.

use algebra_engine::cad::{InvoluteGear, Mesh3D, NacaAirfoil};
use algebra_engine::pde::{FeaBoundaryCondition, PhysicsDiscipline};
use algebra_engine::pipeline::{CadFeaBridge, SimulationPipeline};
use algebra_engine::simulate_cad;

#[test]
fn test_cad_to_fea_bridge_box_mesh() {
    let box_mesh = Mesh3D::cube(8.0, 8.0, 12.0);
    let fea_mesh = CadFeaBridge::mesh3d_to_fea(&box_mesh);

    assert_eq!(fea_mesh.nodes.len(), 8);
    assert_eq!(fea_mesh.elements.len(), 12);
    assert!(fea_mesh.boundary_groups.contains_key("min_z"));
    assert!(fea_mesh.boundary_groups.contains_key("max_z"));
    assert!(fea_mesh.boundary_groups.contains_key("fixed_base"));
    assert!(fea_mesh.boundary_groups.contains_key("load_top"));
}

#[test]
fn test_involute_gear_structural_simulation() {
    let gear = InvoluteGear::spur(2.0, 8, 10.0);
    let pipeline = SimulationPipeline::from_gear(&gear)
        .with_physics(PhysicsDiscipline::LinearElasticity {
            youngs_modulus: 200_000.0,
            poissons_ratio: 0.29,
            density: 7850.0,
            body_force: vec![0.0, 0.0, 0.0],
            plane_stress: true,
        })
        .with_boundary_condition(FeaBoundaryCondition::FixedDisplacement {
            group_name: "bore_hole".into(),
            dof_index: 0,
            value: 0.0,
        })
        .with_boundary_condition(FeaBoundaryCondition::FixedDisplacement {
            group_name: "bore_hole".into(),
            dof_index: 1,
            value: 0.0,
        })
        .with_boundary_condition(FeaBoundaryCondition::FixedDisplacement {
            group_name: "bore_hole".into(),
            dof_index: 2,
            value: 0.0,
        })
        .with_boundary_condition(FeaBoundaryCondition::PointForce {
            group_name: "teeth_flanks".into(),
            force_vector: vec![500.0, -200.0, 0.0],
        });

    let result = pipeline
        .run_analysis()
        .expect("Gear structural analysis failed");

    assert!(result.max_magnitude() > 0.0);
    assert_eq!(
        result.original_cad_mesh.vertices.len(),
        result.fea_mesh.nodes.len()
    );

    // Generate 3D deformed CAD mesh with 100x magnification factor
    let deformed = result.generate_deformed_cad_mesh(100.0);
    assert_eq!(
        deformed.vertices.len(),
        result.original_cad_mesh.vertices.len()
    );
    assert_eq!(
        deformed.triangles.len(),
        result.original_cad_mesh.triangles.len()
    );
}

#[test]
fn test_naca_airfoil_thermal_simulation() {
    let wing = NacaAirfoil::new("2412", 100.0, 300.0);
    let fea_mesh = CadFeaBridge::airfoil_to_fea(&wing);

    assert!(fea_mesh.boundary_groups.contains_key("wing_root"));
    assert!(fea_mesh.boundary_groups.contains_key("wing_tip"));

    let physics = PhysicsDiscipline::HeatConduction {
        thermal_conductivity: 45.0, // Structural steel
        heat_capacity: 460.0,
        density: 7850.0,
        internal_heat_source: 0.0,
    };

    let bcs = vec![
        FeaBoundaryCondition::PrescribedTemperature {
            group_name: "wing_root".into(),
            temperature: 150.0,
        },
        FeaBoundaryCondition::PrescribedTemperature {
            group_name: "wing_tip".into(),
            temperature: 25.0,
        },
    ];

    let pipeline = SimulationPipeline {
        cad_mesh: wing.generate_3d_mesh(),
        fea_mesh,
        physics,
        boundary_conditions: bcs,
    };

    let result = pipeline
        .run_analysis()
        .expect("Airfoil thermal analysis failed");
    assert!(result.max_magnitude() >= 25.0 && result.max_magnitude() <= 150.01);
}

#[test]
fn test_simulation_result_vtk_export() {
    let box_mesh = Mesh3D::cube(4.0, 4.0, 4.0);
    // All three translations of the base are fixed. (Fixing only x left rigid-body motion, which
    // the solver used to hide by returning zeros for the free equations.)
    let mut pipeline = SimulationPipeline::from_cad(box_mesh);
    for dof_index in 0..3 {
        pipeline = pipeline.with_boundary_condition(FeaBoundaryCondition::FixedDisplacement {
            group_name: "fixed_base".into(),
            dof_index,
            value: 0.0,
        });
    }
    let pipeline = pipeline.with_boundary_condition(FeaBoundaryCondition::PointForce {
        group_name: "load_top".into(),
        force_vector: vec![0.0, 0.0, -100.0],
    });

    let result = pipeline.run_analysis().expect("Box FEA failed");
    let vtk_str = result.export_vtk_ascii();

    assert!(vtk_str.contains("# vtk DataFile Version 3.0"));
    assert!(vtk_str.contains("POINTS 8 float"));
    assert!(vtk_str.contains("POLYGONS 12"));
    assert!(vtk_str.contains("POINT_DATA 8"));
}

#[test]
fn test_simulate_cad_macro() {
    let cad = Mesh3D::cube(6.0, 6.0, 6.0);
    let physics = PhysicsDiscipline::HeatConduction {
        thermal_conductivity: 20.0,
        heat_capacity: 1.0,
        density: 1.0,
        internal_heat_source: 0.0,
    };
    let bcs = vec![
        FeaBoundaryCondition::PrescribedTemperature {
            group_name: "fixed_base".into(),
            temperature: 80.0,
        },
        FeaBoundaryCondition::PrescribedTemperature {
            group_name: "load_top".into(),
            temperature: 20.0,
        },
    ];

    let result = simulate_cad!(cad, physics, bcs).expect("simulate_cad macro failed");
    assert!(result.max_magnitude() > 0.0);
}

#[test]
fn test_under_constrained_structures_are_reported_not_solved_with_zeros() {
    // Only the x translation of the base is fixed: the box can translate in y and z and rotate.
    let pipeline = SimulationPipeline::from_cad(Mesh3D::cube(4.0, 4.0, 4.0))
        .with_boundary_condition(FeaBoundaryCondition::FixedDisplacement {
            group_name: "fixed_base".into(),
            dof_index: 0,
            value: 0.0,
        })
        .with_boundary_condition(FeaBoundaryCondition::PointForce {
            group_name: "load_top".into(),
            force_vector: vec![0.0, 0.0, -100.0],
        });
    let error = pipeline.run_analysis().unwrap_err();
    assert!(error.to_string().contains("singular"), "{error}");
}
