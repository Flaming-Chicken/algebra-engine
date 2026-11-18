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

fn tet_volume(mesh: &algebra_engine::pde::FeaMesh, e: &[usize]) -> f64 {
    let p: Vec<&Vec<f64>> = e.iter().map(|&i| &mesh.nodes[i]).collect();
    let a = [p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]];
    let b = [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]];
    let c = [p[3][0] - p[0][0], p[3][1] - p[0][1], p[3][2] - p[0][2]];
    (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
        + a[2] * (b[0] * c[1] - b[1] * c[0]))
        .abs()
        / 6.0
}

#[test]
fn a_bored_gear_is_meshed_as_a_solid_and_analysed_with_real_elasticity() {
    use algebra_engine::pde::ElementType;
    let gear = InvoluteGear::spur(2.0, 12, 8.0);
    let fea = CadFeaBridge::gear_to_fea(&gear);
    let cad = gear.generate_3d_mesh();

    assert_eq!(fea.element_type, ElementType::Tet4);
    assert_eq!(
        fea.nodes.len(),
        cad.vertices.len(),
        "FEA nodes are the CAD vertices"
    );
    // The tetrahedra fill the solid exactly: their volumes add up to the closed mesh's volume.
    let tets: f64 = fea.elements.iter().map(|e| tet_volume(&fea, e)).sum();
    let solid = cad.compute_volume();
    assert!(
        (tets - solid).abs() < 1e-8 * solid,
        "tets {tets} vs solid {solid}"
    );
    assert!(
        fea.elements.iter().all(|e| tet_volume(&fea, e) > 1e-12),
        "no degenerate tetrahedra"
    );

    // Real 3-D elasticity: clamp the bore, load the teeth; the bore carries exactly the load.
    let force = [500.0, -200.0, 30.0];
    let mut pipeline =
        SimulationPipeline::from_gear(&gear).with_physics(PhysicsDiscipline::LinearElasticity {
            youngs_modulus: 200_000.0,
            poissons_ratio: 0.29,
            density: 7850.0,
            body_force: vec![0.0, 0.0, 0.0],
            plane_stress: false,
        });
    for dof in 0..3 {
        pipeline = pipeline.with_boundary_condition(FeaBoundaryCondition::FixedDisplacement {
            group_name: "bore_hole".into(),
            dof_index: dof,
            value: 0.0,
        });
    }
    pipeline = pipeline.with_boundary_condition(FeaBoundaryCondition::PointForce {
        group_name: "teeth_flanks".into(),
        force_vector: force.to_vec(),
    });
    let result = pipeline.run_analysis().expect("solid gear analysis");
    assert_eq!(result.fea_mesh.element_type, ElementType::Tet4);
    assert!(result.max_magnitude() > 0.0);
    assert!(result.max_von_mises_stress() > 0.0);
    let bore = &result.fea_mesh.boundary_groups["bore_hole"];
    for (dof, f) in force.iter().enumerate() {
        let reaction: f64 = bore
            .iter()
            .map(|&n| result.solution.reaction_forces[n * 3 + dof])
            .sum();
        assert!(
            (reaction + f).abs() < 1e-6 * f.abs().max(1.0),
            "dof {dof}: reaction {reaction} vs load {f}"
        );
    }

    // A gently twisted helical gear is still meshed as a solid with nearly the same volume; a
    // strongly twisted one would fold the stacked prisms (the outline turns against the bore
    // ring), so it keeps the surface fallback instead of producing inverted tetrahedra.
    let gentle = InvoluteGear::helical(1.5, 50, 8.0, 3.0);
    let fea = CadFeaBridge::gear_to_fea(&gentle);
    assert_eq!(fea.element_type, ElementType::Tet4);
    let tets: f64 = fea.elements.iter().map(|e| tet_volume(&fea, e)).sum();
    let solid = gentle.generate_3d_mesh().compute_volume();
    assert!(
        (tets - solid).abs() < 0.01 * solid,
        "gentle helical tets {tets} vs solid {solid}"
    );
    let steep = InvoluteGear::helical(1.5, 14, 8.0, 20.0);
    assert_eq!(
        CadFeaBridge::gear_to_fea(&steep).element_type,
        ElementType::Tri3
    );

    // Without a bore there is no annular cap to extrude: the surface fallback is used.
    let mut plain = InvoluteGear::spur(2.0, 12, 8.0);
    plain.bore_radius = 0.0;
    assert_eq!(
        CadFeaBridge::gear_to_fea(&plain).element_type,
        ElementType::Tri3
    );
}

#[test]
fn a_wing_is_meshed_as_a_solid_and_bends_under_a_tip_load() {
    use algebra_engine::pde::ElementType;
    let mut wing = NacaAirfoil::new("2412", 100.0, 600.0);
    let fea = CadFeaBridge::airfoil_to_fea(&wing);
    let cad = wing.generate_3d_mesh();
    assert_eq!(fea.element_type, ElementType::Tet4);
    // The CAD vertices come first, followed by one extra centroid node per interior section.
    assert_eq!(fea.nodes.len(), cad.vertices.len() + 6);
    for (n, v) in fea.nodes.iter().zip(&cad.vertices) {
        assert_eq!(n.as_slice(), v.as_slice());
    }
    // The tetrahedra fill the wing exactly.
    let tets: f64 = fea.elements.iter().map(|e| tet_volume(&fea, e)).sum();
    let solid = cad.compute_volume();
    assert!(
        (tets - solid).abs() < 1e-8 * solid,
        "tets {tets} vs solid {solid}"
    );
    assert!(fea.elements.iter().all(|e| tet_volume(&fea, e) > 1e-12));
    // No node is left out of the mesh (an unused node would make the system singular).
    let mut used = vec![false; fea.nodes.len()];
    for e in &fea.elements {
        for &i in e {
            used[i] = true;
        }
    }
    assert!(used.iter().all(|u| *u), "every node belongs to an element");

    // A tip load in +y with the root clamped: the wing deflects towards +y, more towards the tip,
    // and the root reactions balance the load.
    let force = [0.0, 40.0, 0.0];
    let mut pipeline = SimulationPipeline::from_cad(cad.clone());
    pipeline.fea_mesh = fea.clone();
    pipeline = pipeline.with_physics(PhysicsDiscipline::LinearElasticity {
        youngs_modulus: 70_000.0,
        poissons_ratio: 0.33,
        density: 2700.0,
        body_force: vec![0.0, 0.0, 0.0],
        plane_stress: false,
    });
    for dof in 0..3 {
        pipeline = pipeline.with_boundary_condition(FeaBoundaryCondition::FixedDisplacement {
            group_name: "wing_root".into(),
            dof_index: dof,
            value: 0.0,
        });
    }
    pipeline = pipeline.with_boundary_condition(FeaBoundaryCondition::PointForce {
        group_name: "wing_tip".into(),
        force_vector: force.to_vec(),
    });
    let result = pipeline.run_analysis().expect("wing structural analysis");
    let uy = |z: f64| {
        let nodes: Vec<usize> = (0..cad.vertices.len())
            .filter(|&i| (cad.vertices[i][2] - z).abs() < 1e-6)
            .collect();
        nodes
            .iter()
            .map(|&i| result.solution.nodal_values[i * 3 + 1])
            .sum::<f64>()
            / nodes.len() as f64
    };
    let (mid, tip) = (uy(600.0 * 3.0 / 7.0), uy(600.0));
    assert!(
        tip > mid && mid > 0.0,
        "deflection grows towards the tip: mid {mid}, tip {tip}"
    );
    let root = &result.fea_mesh.boundary_groups["wing_root"];
    for (dof, f) in force.iter().enumerate() {
        let reaction: f64 = root
            .iter()
            .map(|&n| result.solution.reaction_forces[n * 3 + dof])
            .sum();
        assert!(
            // Tolerance relative to the size of the whole load (a component that should be zero
            // carries solver noise of that order).
            (reaction + f).abs() < 1e-5 * 40.0,
            "dof {dof}: {reaction} vs {f}"
        );
    }

    // A twisted wing (rotated sections never fold) is a solid as well.
    wing.twist_deg = 8.0;
    let twisted = CadFeaBridge::airfoil_to_fea(&wing);
    assert_eq!(twisted.element_type, ElementType::Tet4);
    let tets: f64 = twisted
        .elements
        .iter()
        .map(|e| tet_volume(&twisted, e))
        .sum();
    let solid = wing.generate_3d_mesh().compute_volume();
    assert!(
        (tets - solid).abs() < 0.02 * solid,
        "twisted tets {tets} vs solid {solid}"
    );
}
