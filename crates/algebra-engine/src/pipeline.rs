//! # `algebra_engine::pipeline`
//!
//! Deep Cross-Tooling Simulation Pipeline & CAD-to-FEA Bridge.
//!
//! Provides end-to-end integration across:
//! - **CAD Geometry $\to$ Conforming FEA Mesh**:
//!   Converts arbitrary 3D parametric CAD geometry (`Mesh3D`, `InvoluteGear`, `ThreadedScrew`, `NacaAirfoil`,
//!   or imported OBJ/STL shapes) into a simulation-ready `FeaMesh` with automated boundary detection.
//! - **Symbolic Physics Specification**:
//!   Couples linear elasticity, thermal heat conduction, and thermo-elasticity.
//! - **Simulation Execution**:
//!   Resolves the variational weak form via `FeaEngine`.
//! - **Deformed CAD Shape Synthesis & Visualization**:
//!   Generates deformed 3D CAD meshes $\mathbf{v}' = \mathbf{v} + \alpha \mathbf{u}$ and computes von Mises stress
//!   distributions, temperature fields, and VTK data export.

use crate::cad::{InvoluteGear, Mesh3D, NacaAirfoil, ThreadedScrew};
use crate::pde::{
    ElementType, FeaBoundaryCondition, FeaEngine, FeaMesh, FeaSolution, PhysicsDiscipline,
};
use algebra_core::error::AlgebraResult;

/// Bridge utility for converting CAD representations to FEA simulation meshes.
pub struct CadFeaBridge;

impl CadFeaBridge {
    /// Convert any 3D surface mesh (`Mesh3D`) into a 2D/3D shell `FeaMesh` with auto-detected boundary groups.
    pub fn mesh3d_to_fea(cad_mesh: &Mesh3D) -> FeaMesh {
        let mut fea = FeaMesh::new(3, ElementType::Tri3);

        // Copy vertices as 3D nodes
        for v in &cad_mesh.vertices {
            fea.add_node(v.to_vec());
        }

        // Copy triangles as elements
        for t in &cad_mesh.triangles {
            fea.add_element(t.to_vec());
        }

        // Auto-detect boundary groups based on bounding box extremes
        let aabb = cad_mesh.compute_bounding_box();
        let eps = 1e-4 * (aabb.size()[0].max(aabb.size()[1]).max(aabb.size()[2])).max(1.0);

        let mut min_x = Vec::new();
        let mut max_x = Vec::new();
        let mut min_y = Vec::new();
        let mut max_y = Vec::new();
        let mut min_z = Vec::new();
        let mut max_z = Vec::new();

        for (idx, v) in cad_mesh.vertices.iter().enumerate() {
            if (v[0] - aabb.min[0]).abs() < eps {
                min_x.push(idx);
            }
            if (v[0] - aabb.max[0]).abs() < eps {
                max_x.push(idx);
            }
            if (v[1] - aabb.min[1]).abs() < eps {
                min_y.push(idx);
            }
            if (v[1] - aabb.max[1]).abs() < eps {
                max_y.push(idx);
            }
            if (v[2] - aabb.min[2]).abs() < eps {
                min_z.push(idx);
            }
            if (v[2] - aabb.max[2]).abs() < eps {
                max_z.push(idx);
            }
        }

        fea.add_boundary_group("min_x", min_x);
        fea.add_boundary_group("max_x", max_x);
        fea.add_boundary_group("min_y", min_y);
        fea.add_boundary_group("max_y", max_y);
        fea.add_boundary_group("min_z", min_z);
        fea.add_boundary_group("max_z", max_z);
        fea.add_boundary_group(
            "fixed_base",
            fea.boundary_groups
                .get("min_z")
                .cloned()
                .unwrap_or_default(),
        );
        fea.add_boundary_group(
            "load_top",
            fea.boundary_groups
                .get("max_z")
                .cloned()
                .unwrap_or_default(),
        );

        fea
    }

    /// Tetrahedral solid mesh of a bored gear, or `None` when the gear mesh is not a stack of
    /// identical layers (no bore).
    fn extruded_gear_solid(gear: &InvoluteGear, mesh_3d: &Mesh3D) -> Option<FeaMesh> {
        let bore = gear
            .bore_radius
            .min(0.8 * gear.root_diameter() / 2.0)
            .max(0.0);
        if bore <= 1e-9 {
            return None;
        }
        let per_layer = gear.generate_2d_profile(6).len() + 48;
        let total = mesh_3d.vertices.len();
        if !total.is_multiple_of(per_layer) || total / per_layer < 2 {
            return None;
        }
        let layers = total / per_layer;
        // The bottom cap: triangles lying entirely in the first layer (the walls span two layers).
        let bottom: Vec<[usize; 3]> = mesh_3d
            .triangles
            .iter()
            .copied()
            .filter(|t| t.iter().all(|&i| i < per_layer))
            .collect();
        if bottom.is_empty() {
            return None;
        }
        // The same triangle at every layer must keep its orientation: a strong helical twist
        // rotates the outline against the (unrotated) bore ring and can fold a triangle, which
        // would give inverted tetrahedra. Then the solid mesh is not used.
        let area = |layer: usize, t: &[usize; 3]| {
            let v = |i: usize| mesh_3d.vertices[i + layer * per_layer];
            let (a, b, c) = (v(t[0]), v(t[1]), v(t[2]));
            (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
        };
        for tri in &bottom {
            let reference = area(0, tri);
            if reference.abs() < 1e-12 {
                return None;
            }
            if (1..layers).any(|l| area(l, tri) * reference <= 1e-12) {
                return None;
            }
        }
        let mut fea = FeaMesh::new(3, ElementType::Tet4);
        for v in &mesh_3d.vertices {
            fea.add_node(v.to_vec());
        }
        for layer in 0..layers - 1 {
            let (lo, hi) = (layer * per_layer, (layer + 1) * per_layer);
            for tri in &bottom {
                // Sort by vertex index so neighbouring prisms cut their shared quad faces along
                // the same diagonal (a conforming mesh).
                let mut v = *tri;
                v.sort_unstable();
                let (p0, p1, p2) = (v[0], v[1], v[2]);
                let up = |i: usize| i + (hi - lo);
                let base = |i: usize| i + lo;
                fea.add_element(vec![base(p0), base(p1), base(p2), up(base(p2))]);
                fea.add_element(vec![base(p0), base(p1), up(base(p1)), up(base(p2))]);
                fea.add_element(vec![base(p0), up(base(p0)), up(base(p1)), up(base(p2))]);
            }
        }
        Some(fea)
    }

    /// Convert an Involute Gear to a specialized simulation `FeaMesh` with named `"bore_hole"` and `"teeth_flanks"`.
    ///
    /// A gear with a bore is meshed as a **solid** of linear tetrahedra (`Tet4`): the bottom cap's
    /// triangles are extruded through the layers of the gear mesh and each triangular prism is cut
    /// into three tetrahedra, so the analysis is real 3-D elasticity and the FEA nodes are exactly
    /// the CAD vertices (displacements map straight onto the deformed shape). A gear without a
    /// bore has no annular cap to extrude and falls back to the surface triangulation, whose 3-D
    /// `Tri3` element is only a pseudo-stiffness (see ledger C11).
    pub fn gear_to_fea(gear: &InvoluteGear) -> FeaMesh {
        let mesh_3d = gear.generate_3d_mesh();
        let mut fea = Self::extruded_gear_solid(gear, &mesh_3d)
            .unwrap_or_else(|| Self::mesh3d_to_fea(&mesh_3d));

        let min_r = mesh_3d
            .vertices
            .iter()
            .map(|v| (v[0] * v[0] + v[1] * v[1]).sqrt())
            .fold(f64::INFINITY, f64::min);
        let max_r = mesh_3d
            .vertices
            .iter()
            .map(|v| (v[0] * v[0] + v[1] * v[1]).sqrt())
            .fold(0.0, f64::max);
        let span_r = (max_r - min_r).max(1e-6);

        let mut bore_nodes = Vec::new();
        let mut tooth_nodes = Vec::new();

        for (idx, v) in mesh_3d.vertices.iter().enumerate() {
            let r = (v[0] * v[0] + v[1] * v[1]).sqrt();
            if (r - min_r) <= span_r * 0.35 {
                bore_nodes.push(idx);
            }
            if (max_r - r) <= span_r * 0.45 {
                tooth_nodes.push(idx);
            }
        }

        fea.add_boundary_group("bore_hole", bore_nodes);
        fea.add_boundary_group("teeth_flanks", tooth_nodes);
        fea
    }

    /// Convert a Threaded Fastener to a simulation `FeaMesh` with `"bolt_head"` and `"shank_tip"`.
    pub fn screw_to_fea(screw: &ThreadedScrew) -> FeaMesh {
        let mesh_3d = screw.generate_3d_mesh();
        let mut fea = Self::mesh3d_to_fea(&mesh_3d);

        let head_z = screw.total_length;
        let mut head_nodes = Vec::new();
        let mut tip_nodes = Vec::new();

        for (idx, v) in mesh_3d.vertices.iter().enumerate() {
            if v[2] >= head_z - 1e-3 {
                head_nodes.push(idx);
            } else if v[2] <= 1e-3 {
                tip_nodes.push(idx);
            }
        }

        fea.add_boundary_group("bolt_head", head_nodes);
        fea.add_boundary_group("shank_tip", tip_nodes);
        fea
    }

    /// Convert a NACA Airfoil wing to a simulation `FeaMesh` with `"wing_root"` and `"wing_tip"`.
    pub fn airfoil_to_fea(airfoil: &NacaAirfoil) -> FeaMesh {
        let mesh_3d = airfoil.generate_3d_mesh();
        let mut fea = Self::mesh3d_to_fea(&mesh_3d);

        let span = airfoil.span;
        let mut root_nodes = Vec::new();
        let mut tip_nodes = Vec::new();

        for (idx, v) in mesh_3d.vertices.iter().enumerate() {
            if v[2] <= 1e-3 {
                root_nodes.push(idx);
            } else if v[2] >= span - 1e-3 {
                tip_nodes.push(idx);
            }
        }

        fea.add_boundary_group("wing_root", root_nodes);
        fea.add_boundary_group("wing_tip", tip_nodes);
        fea
    }
}

/// Unified Cross-Tooling Simulation Pipeline.
#[derive(Debug, Clone)]
pub struct SimulationPipeline {
    pub cad_mesh: Mesh3D,
    pub fea_mesh: FeaMesh,
    pub physics: PhysicsDiscipline,
    pub boundary_conditions: Vec<FeaBoundaryCondition>,
}

impl SimulationPipeline {
    /// Initialize a new simulation pipeline from a CAD mesh.
    pub fn from_cad(cad_mesh: Mesh3D) -> Self {
        let fea_mesh = CadFeaBridge::mesh3d_to_fea(&cad_mesh);
        Self {
            cad_mesh,
            fea_mesh,
            physics: PhysicsDiscipline::LinearElasticity {
                youngs_modulus: 200_000.0,
                poissons_ratio: 0.3,
                density: 7850.0,
                body_force: vec![0.0, 0.0, 0.0],
                plane_stress: true,
            },
            boundary_conditions: Vec::new(),
        }
    }

    /// Initialize a pipeline from an Involute Gear.
    pub fn from_gear(gear: &InvoluteGear) -> Self {
        let cad_mesh = gear.generate_3d_mesh();
        let fea_mesh = CadFeaBridge::gear_to_fea(gear);
        Self {
            cad_mesh,
            fea_mesh,
            physics: PhysicsDiscipline::LinearElasticity {
                youngs_modulus: 210_000.0,
                poissons_ratio: 0.28,
                density: 7850.0,
                body_force: vec![0.0, 0.0, 0.0],
                plane_stress: true,
            },
            boundary_conditions: Vec::new(),
        }
    }

    /// Configure physics discipline.
    pub fn with_physics(mut self, physics: PhysicsDiscipline) -> Self {
        self.physics = physics;
        self
    }

    /// Add a boundary condition.
    pub fn with_boundary_condition(mut self, bc: FeaBoundaryCondition) -> Self {
        self.boundary_conditions.push(bc);
        self
    }

    /// Run the end-to-end FEA simulation analysis.
    pub fn run_analysis(&self) -> AlgebraResult<SimulationResult> {
        let solution = FeaEngine::solve(&self.fea_mesh, &self.physics, &self.boundary_conditions)?;
        Ok(SimulationResult {
            solution,
            original_cad_mesh: self.cad_mesh.clone(),
            fea_mesh: self.fea_mesh.clone(),
        })
    }
}

/// Simulation Result containing solution fields, deformed shapes, and analytics.
#[derive(Debug, Clone)]
pub struct SimulationResult {
    pub solution: FeaSolution,
    pub original_cad_mesh: Mesh3D,
    pub fea_mesh: FeaMesh,
}

impl SimulationResult {
    /// Generate a 3D deformed CAD mesh by displacing original vertices: $\mathbf{v}' = \mathbf{v} + \alpha \mathbf{u}$.
    #[allow(clippy::needless_range_loop)]
    pub fn generate_deformed_cad_mesh(&self, scale_factor: f64) -> Mesh3D {
        let mut deformed = self.original_cad_mesh.clone();
        deformed.name = format!("{}_Deformed_x{}", self.original_cad_mesh.name, scale_factor);

        let dof_per_node = self.solution.dof_per_node;
        for (i, v) in deformed.vertices.iter_mut().enumerate() {
            if (i + 1) * dof_per_node <= self.solution.nodal_values.len() {
                for d in 0..dof_per_node.min(3) {
                    let disp = self.solution.nodal_values[i * dof_per_node + d];
                    v[d] += scale_factor * disp;
                }
            }
        }

        deformed
    }

    /// Maximum displacement / scalar field magnitude.
    pub fn max_magnitude(&self) -> f64 {
        self.solution.max_magnitude
    }

    /// Maximum von Mises stress $\sigma_{\max}$.
    pub fn max_von_mises_stress(&self) -> f64 {
        self.solution
            .von_mises_stresses
            .iter()
            .copied()
            .fold(0.0, f64::max)
    }

    /// Export simulation mesh and scalar results to standard ASCII VTK format for visualizers (ParaView / Three.js).
    pub fn export_vtk_ascii(&self) -> String {
        let mut vtk = String::new();
        vtk.push_str("# vtk DataFile Version 3.0\n");
        vtk.push_str("URAE Multiphysics Simulation Field\n");
        vtk.push_str("ASCII\n");
        vtk.push_str("DATASET POLYDATA\n");

        // Points
        let n_pts = self.fea_mesh.nodes.len();
        vtk.push_str(&format!("POINTS {} float\n", n_pts));
        for n in &self.fea_mesh.nodes {
            let x = n.first().copied().unwrap_or(0.0);
            let y = n.get(1).copied().unwrap_or(0.0);
            let z = n.get(2).copied().unwrap_or(0.0);
            vtk.push_str(&format!("{:.6} {:.6} {:.6}\n", x, y, z));
        }

        // Polygons / Triangles
        let n_elems = self.fea_mesh.elements.len();
        let total_idx_count: usize = self.fea_mesh.elements.iter().map(|e| e.len() + 1).sum();
        vtk.push_str(&format!("POLYGONS {} {}\n", n_elems, total_idx_count));
        for elem in &self.fea_mesh.elements {
            vtk.push_str(&format!("{}", elem.len()));
            for &idx in elem {
                vtk.push_str(&format!(" {}", idx));
            }
            vtk.push('\n');
        }

        // Point Data (Displacement or Temperature)
        vtk.push_str(&format!("POINT_DATA {}\n", n_pts));
        vtk.push_str("SCALARS NodalField float 1\n");
        vtk.push_str("LOOKUP_TABLE default\n");
        for (i, _) in self.fea_mesh.nodes.iter().enumerate() {
            let val = if i < self.solution.nodal_values.len() {
                self.solution.nodal_values[i * self.solution.dof_per_node]
            } else {
                0.0
            };
            vtk.push_str(&format!("{:.6}\n", val));
        }

        vtk
    }
}
