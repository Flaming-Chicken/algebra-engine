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

use crate::cad::{HelicalSpring, InvoluteGear, Mesh3D, NacaAirfoil, ThreadedScrew};
use crate::pde::{
    ElementType, FeaBoundaryCondition, FeaEngine, FeaMesh, FeaSolution, PhysicsDiscipline,
};
use algebra_core::error::AlgebraResult;
use std::f64::consts::PI;

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
        let mut fea = FeaMesh::new(3, ElementType::Tet4);
        for v in &mesh_3d.vertices {
            fea.add_node(v.to_vec());
        }
        let node_of = |layer: usize, local: usize| layer * per_layer + local;
        Self::stack_prisms(
            &mut fea,
            &bottom,
            layers,
            &node_of,
            &|mesh: &FeaMesh, layer: usize, t: &[usize; 3]| {
                let v = |i: usize| &mesh.nodes[node_of(layer, i)];
                let (a, b, c) = (v(t[0]), v(t[1]), v(t[2]));
                (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
            },
        )?;
        Some(fea)
    }

    /// Fill a stack of identical layers with tetrahedra: every base triangle (given by *local*
    /// vertex ids) is extruded between consecutive layers and each prism is cut into three
    /// tetrahedra. `node(layer, local)` gives the FEA node of a local vertex in a layer.
    ///
    /// Local ids are sorted before cutting so neighbouring prisms split their shared quad faces
    /// along the same diagonal (a conforming mesh). `None` if a base triangle is degenerate or
    /// changes orientation between layers (a strong twist that would give inverted tetrahedra).
    fn stack_prisms(
        fea: &mut FeaMesh,
        base: &[[usize; 3]],
        layers: usize,
        node: &dyn Fn(usize, usize) -> usize,
        orientation: &dyn Fn(&FeaMesh, usize, &[usize; 3]) -> f64,
    ) -> Option<()> {
        if base.is_empty() || layers < 2 {
            return None;
        }
        // `orientation(mesh, layer, triangle)` is a signed measure of how the base triangle sits
        // in that layer (its planar area for a straight stack, its normal along the path for a
        // swept one); it must keep its sign through the stack.
        for tri in base {
            let reference = orientation(fea, 0, tri);
            if reference.abs() < 1e-12 {
                return None;
            }
            if (1..layers).any(|l| orientation(fea, l, tri) * reference <= 1e-12) {
                return None;
            }
        }
        for layer in 0..layers - 1 {
            for tri in base {
                let mut v = *tri;
                v.sort_unstable();
                let lo = |i: usize| node(layer, i);
                let hi = |i: usize| node(layer + 1, i);
                let (p0, p1, p2) = (v[0], v[1], v[2]);
                fea.add_element(vec![lo(p0), lo(p1), lo(p2), hi(p2)]);
                fea.add_element(vec![lo(p0), lo(p1), hi(p1), hi(p2)]);
                fea.add_element(vec![lo(p0), hi(p0), hi(p1), hi(p2)]);
            }
        }
        Some(())
    }

    /// Tetrahedral solid mesh of a wing: the outline cap (a fan from the section centroid) is
    /// extruded through the spanwise sections. The FEA nodes are the CAD vertices followed by one
    /// extra centroid node per intermediate section (the CAD mesh only has centroids at the two
    /// ends). `None` for a strongly twisted wing whose sections would fold the prisms.
    fn extruded_airfoil_solid(airfoil: &NacaAirfoil, mesh_3d: &Mesh3D) -> Option<FeaMesh> {
        let n_prof = airfoil.generate_2d_coordinates(20).len();
        let sections = 8;
        // Layout of `generate_3d_mesh`: sections * n_prof outline vertices, then the bottom and
        // top centroids.
        if mesh_3d.vertices.len() != sections * n_prof + 2 {
            return None;
        }
        let (bottom_centre, top_centre) = (sections * n_prof, sections * n_prof + 1);
        let mut fea = FeaMesh::new(3, ElementType::Tet4);
        for v in &mesh_3d.vertices {
            fea.add_node(v.to_vec());
        }
        // Centroid of each intermediate section, at the section's own twist.
        let mut middle = Vec::new();
        for s in 1..sections - 1 {
            let ring: Vec<&Vec<f64>> = (0..n_prof).map(|i| &fea.nodes[s * n_prof + i]).collect();
            let cx = ring.iter().map(|p| p[0]).sum::<f64>() / n_prof as f64;
            let cy = ring.iter().map(|p| p[1]).sum::<f64>() / n_prof as f64;
            let z = ring[0][2];
            middle.push((cx, cy, z));
        }
        let first_extra = fea.nodes.len();
        for (cx, cy, z) in &middle {
            fea.add_node(vec![*cx, *cy, *z]);
        }
        // Base triangles: the fan around the bottom centroid, in local ids (the centroid is local
        // id `n_prof`).
        let base: Vec<[usize; 3]> = mesh_3d
            .triangles
            .iter()
            .filter(|t| t.contains(&bottom_centre))
            .map(|t| t.map(|i| if i == bottom_centre { n_prof } else { i }))
            .collect();
        let node = |layer: usize, local: usize| {
            if local < n_prof {
                layer * n_prof + local
            } else if layer == 0 {
                bottom_centre
            } else if layer == sections - 1 {
                top_centre
            } else {
                first_extra + layer - 1
            }
        };
        let node_of = &node;
        Self::stack_prisms(
            &mut fea,
            &base,
            sections,
            &node,
            &|mesh: &FeaMesh, layer: usize, t: &[usize; 3]| {
                let v = |i: usize| &mesh.nodes[node_of(layer, i)];
                let (a, b, c) = (v(t[0]), v(t[1]), v(t[2]));
                (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
            },
        )?;
        Some(fea)
    }

    /// Tetrahedral solid mesh of a helical spring: the wire's circular cross-section (a fan around
    /// the ring centre) is swept along the helix and each step between consecutive rings is cut
    /// into tetrahedra. The FEA nodes are the CAD vertices followed by one centre node per
    /// interior ring. `None` if a triangle of the cross-section would fold between rings.
    fn extruded_spring_solid(spring: &HelicalSpring, mesh_3d: &Mesh3D) -> Option<FeaMesh> {
        let steps_per_turn = 24;
        let circ = 8;
        let total_steps = (spring.active_coils * steps_per_turn as f64) as usize;
        let rings = total_steps + 1;
        if total_steps < 1 || mesh_3d.vertices.len() != rings * circ + 2 {
            return None;
        }
        let (bottom_centre, top_centre) = (rings * circ, rings * circ + 1);
        let mut fea = FeaMesh::new(3, ElementType::Tet4);
        for v in &mesh_3d.vertices {
            fea.add_node(v.to_vec());
        }
        let r_coil = spring.mean_diameter / 2.0;
        let first_extra = fea.nodes.len();
        for s in 1..total_steps {
            let theta = (s as f64 / steps_per_turn as f64) * 2.0 * PI;
            fea.add_node(vec![
                r_coil * theta.cos(),
                r_coil * theta.sin(),
                (s as f64 / steps_per_turn as f64) * spring.pitch,
            ]);
        }
        let base: Vec<[usize; 3]> = mesh_3d
            .triangles
            .iter()
            .filter(|t| t.contains(&bottom_centre))
            .map(|t| t.map(|i| if i == bottom_centre { circ } else { i }))
            .collect();
        let node = |layer: usize, local: usize| {
            if local < circ {
                layer * circ + local
            } else if layer == 0 {
                bottom_centre
            } else if layer == total_steps {
                top_centre
            } else {
                first_extra + layer - 1
            }
        };
        // Orientation of a cross-section triangle: its normal along the direction to the next
        // ring's centre (the last ring looks back at the previous one).
        let orientation = |mesh: &FeaMesh, layer: usize, t: &[usize; 3]| {
            let p = |i: usize| &mesh.nodes[node(layer, i)];
            let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
            let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let w = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let n = [
                u[1] * w[2] - u[2] * w[1],
                u[2] * w[0] - u[0] * w[2],
                u[0] * w[1] - u[1] * w[0],
            ];
            let (this, other) = if layer < total_steps {
                (node(layer, circ), node(layer + 1, circ))
            } else {
                (node(layer - 1, circ), node(layer, circ))
            };
            let d = [
                mesh.nodes[other][0] - mesh.nodes[this][0],
                mesh.nodes[other][1] - mesh.nodes[this][1],
                mesh.nodes[other][2] - mesh.nodes[this][2],
            ];
            n[0] * d[0] + n[1] * d[1] + n[2] * d[2]
        };
        Self::stack_prisms(&mut fea, &base, rings, &node, &orientation)?;
        Some(fea)
    }

    /// Convert a helical spring to a simulation `FeaMesh` (a solid of linear tetrahedra, see
    /// [`CadFeaBridge::extruded_spring_solid`]) with the end-face groups `"spring_bottom"` and
    /// `"spring_top"`. Falls back to the surface triangulation if the sweep would fold.
    pub fn spring_to_fea(spring: &HelicalSpring) -> FeaMesh {
        let mesh_3d = spring.generate_3d_mesh();
        let mut fea = Self::extruded_spring_solid(spring, &mesh_3d)
            .unwrap_or_else(|| Self::mesh3d_to_fea(&mesh_3d));
        // The end faces are the first and last rings plus their centres (vertex order is by ring).
        let ring = |lo: bool| -> Vec<usize> {
            let circ = 8;
            let rings = (mesh_3d.vertices.len() - 2) / circ;
            let range = if lo {
                0..circ
            } else {
                (rings - 1) * circ..rings * circ
            };
            let centre = mesh_3d.vertices.len() - if lo { 2 } else { 1 };
            range.chain(std::iter::once(centre)).collect()
        };
        fea.add_boundary_group("spring_bottom", ring(true));
        fea.add_boundary_group("spring_top", ring(false));
        fea
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

        // `bore_hole`: exactly the vertices on the bore cylinder (none for a gear without a bore).
        // `teeth_flanks`: the outline vertices above the pitch circle (tooth tips and the upper
        // flanks), where the mating gear loads the tooth. Both are selected by geometry, not by
        // a fraction of the radial range, so they do not depend on the bore size or tooth count.
        let bore_radius = gear
            .bore_radius
            .min(0.8 * gear.root_diameter() / 2.0)
            .max(0.0);
        let pitch_radius = gear.pitch_diameter() / 2.0;
        let mut bore_nodes = Vec::new();
        let mut tooth_nodes = Vec::new();
        for (idx, v) in mesh_3d.vertices.iter().enumerate() {
            let r = (v[0] * v[0] + v[1] * v[1]).sqrt();
            if bore_radius > 1e-9 && (r - bore_radius).abs() < 1e-6 * bore_radius.max(1.0) {
                bore_nodes.push(idx);
            } else if r > pitch_radius {
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
    ///
    /// The wing is meshed as a **solid** of linear tetrahedra (the section is extruded through the
    /// spanwise sections, see [`CadFeaBridge::gear_to_fea`] for the scheme), so structural
    /// analysis is real 3-D elasticity; a strongly twisted wing keeps the surface triangulation.
    /// The FEA nodes are the CAD vertices plus one extra centroid node per intermediate section.
    pub fn airfoil_to_fea(airfoil: &NacaAirfoil) -> FeaMesh {
        let mesh_3d = airfoil.generate_3d_mesh();
        let mut fea = Self::extruded_airfoil_solid(airfoil, &mesh_3d)
            .unwrap_or_else(|| Self::mesh3d_to_fea(&mesh_3d));

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
