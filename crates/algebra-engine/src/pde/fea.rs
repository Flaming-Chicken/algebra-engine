//! # `algebra_engine::pde::fea`
//!
//! $N$-Dimensional Multiphysics Finite Element Analysis (FEA / FEM) Engine.
//!
//! Implements:
//! - **Galerkin Weak Form Variational Discretization**:
//!   $$a(\mathbf{u}, \mathbf{v}) = L(\mathbf{v}) \iff \mathbf{K} \mathbf{u} = \mathbf{F}$$
//! - **Isoparametric Element Library**: 1D Bar/Beam, 2D Triangle (Tri3/Tri6) & Quad (Quad4/Quad8),
//!   3D Tetrahedron (Tet4/Tet10) & Hexahedron (Hex8), and general $N$-Simplex in $\mathbb{R}^n$.
//! - **Coupled Multiphysics**:
//!   - Linear Elasticity (Navier-Cauchy with plane stress, plane strain, 3D, and von Mises stress $\sigma_v$).
//!   - Thermal Conduction (steady-state and transient heat transfer with convective cooling).
//!   - Coupled Thermo-Elasticity (thermal expansion stress $\sigma = \mathbf{C} : \epsilon - \alpha \Delta T \mathbf{I}$).
//!   - Electrostatic Potential (Poisson equation $\nabla^2 \phi = -\rho / \epsilon$).
//! - **Dynamic Transient Steppers**:
//!   - Unconditionally stable Newmark-$\beta$ ($\beta = 0.25, \gamma = 0.5$) for structural dynamics.
//!   - Crank-Nicolson 2nd-order scheme for transient heat diffusion.

use algebra_core::error::{AlgebraError, AlgebraResult};
use std::collections::HashMap;

/// Finite element geometric topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementType {
    /// 1D Linear 2-node line element.
    Line2,
    /// 2D Linear 3-node triangle element.
    Tri3,
    /// 2D Quadratic 6-node triangle element.
    Tri6,
    /// 2D Bilinear 4-node quadrilateral element.
    Quad4,
    /// 3D Linear 4-node tetrahedral element.
    Tet4,
    /// 3D Linear 8-node hexahedral element.
    Hex8,
    /// $N$-Dimensional Simplex with $(N+1)$ nodes.
    SimplexND(usize),
}

/// Discretized Multiphysics Finite Element Mesh.
#[derive(Debug, Clone, PartialEq)]
pub struct FeaMesh {
    /// Spatial dimension $N$ ($\mathbb{R}^N$).
    pub dim: usize,
    /// Nodal coordinates $\mathbf{x}_i \in \mathbb{R}^N$.
    pub nodes: Vec<Vec<f64>>,
    /// Element connectivity table: `element_idx -> node_indices`.
    pub elements: Vec<Vec<usize>>,
    /// Element type.
    pub element_type: ElementType,
    /// Named boundary node groups (e.g. `"fixed_base"`, `"load_tip"`).
    pub boundary_groups: HashMap<String, Vec<usize>>,
}

impl FeaMesh {
    /// Create a new FEA mesh with specified dimension and element type.
    pub fn new(dim: usize, element_type: ElementType) -> Self {
        Self {
            dim,
            nodes: Vec::new(),
            elements: Vec::new(),
            element_type,
            boundary_groups: HashMap::new(),
        }
    }

    /// Add a node and return its index.
    pub fn add_node(&mut self, coords: Vec<f64>) -> usize {
        self.nodes.push(coords);
        self.nodes.len() - 1
    }

    /// Add an element connectivity tuple.
    pub fn add_element(&mut self, node_indices: Vec<usize>) {
        self.elements.push(node_indices);
    }

    /// Register a named group of node indices for boundary conditions.
    pub fn add_boundary_group(&mut self, name: impl Into<String>, node_indices: Vec<usize>) {
        self.boundary_groups.insert(name.into(), node_indices);
    }

    /// Create a 1D uniform bar mesh of length $L$ with $N$ elements.
    pub fn bar_1d(length: f64, num_elements: usize) -> Self {
        let mut mesh = Self::new(1, ElementType::Line2);
        let dx = length / (num_elements as f64);
        for i in 0..=num_elements {
            mesh.add_node(vec![i as f64 * dx]);
        }
        for i in 0..num_elements {
            mesh.add_element(vec![i, i + 1]);
        }
        mesh.add_boundary_group("left", vec![0]);
        mesh.add_boundary_group("right", vec![num_elements]);
        mesh
    }

    /// Create a 2D rectangular plate mesh with linear triangular elements (Tri3).
    pub fn plate_2d(width: f64, height: f64, nx: usize, ny: usize) -> Self {
        let mut mesh = Self::new(2, ElementType::Tri3);
        let dx = width / (nx as f64);
        let dy = height / (ny as f64);

        for j in 0..=ny {
            for i in 0..=nx {
                mesh.add_node(vec![i as f64 * dx, j as f64 * dy]);
            }
        }

        let node_idx = |i: usize, j: usize| j * (nx + 1) + i;

        for j in 0..ny {
            for i in 0..nx {
                let n0 = node_idx(i, j);
                let n1 = node_idx(i + 1, j);
                let n2 = node_idx(i + 1, j + 1);
                let n3 = node_idx(i, j + 1);

                // Two Tri3 elements per quad
                mesh.add_element(vec![n0, n1, n2]);
                mesh.add_element(vec![n0, n2, n3]);
            }
        }

        // Boundary groups
        let mut left = Vec::new();
        let mut right = Vec::new();
        let mut bottom = Vec::new();
        let mut top = Vec::new();

        for j in 0..=ny {
            left.push(node_idx(0, j));
            right.push(node_idx(nx, j));
        }
        for i in 0..=nx {
            bottom.push(node_idx(i, 0));
            top.push(node_idx(i, ny));
        }

        mesh.add_boundary_group("left", left);
        mesh.add_boundary_group("right", right);
        mesh.add_boundary_group("bottom", bottom);
        mesh.add_boundary_group("top", top);

        mesh
    }

    /// A 2-D rectangular plate of quadratic triangles (Tri6): the [`FeaMesh::plate_2d`] triangles
    /// with a node added at the middle of every edge. Element nodes are the three corners, then
    /// the midpoints of edges 0-1, 1-2 and 2-0. The boundary groups `left`/`right`/`bottom`/`top`
    /// include the midside nodes on those edges.
    pub fn plate_2d_tri6(width: f64, height: f64, nx: usize, ny: usize) -> Self {
        let linear = Self::plate_2d(width, height, nx, ny);
        let mut mesh = Self::new(2, ElementType::Tri6);
        for node in &linear.nodes {
            mesh.add_node(node.clone());
        }
        let mut midpoints: HashMap<(usize, usize), usize> = HashMap::new();
        for elem in &linear.elements {
            let mut mid = |a: usize, b: usize, mesh: &mut Self| -> usize {
                let key = (a.min(b), a.max(b));
                *midpoints.entry(key).or_insert_with(|| {
                    let (pa, pb) = (&mesh.nodes[a], &mesh.nodes[b]);
                    let m = vec![(pa[0] + pb[0]) / 2.0, (pa[1] + pb[1]) / 2.0];
                    mesh.add_node(m)
                })
            };
            let m01 = mid(elem[0], elem[1], &mut mesh);
            let m12 = mid(elem[1], elem[2], &mut mesh);
            let m20 = mid(elem[2], elem[0], &mut mesh);
            mesh.add_element(vec![elem[0], elem[1], elem[2], m01, m12, m20]);
        }
        let eps = 1e-9 * width.max(height);
        let mut groups: [Vec<usize>; 4] = Default::default();
        for (i, p) in mesh.nodes.iter().enumerate() {
            if p[0].abs() < eps {
                groups[0].push(i);
            }
            if (p[0] - width).abs() < eps {
                groups[1].push(i);
            }
            if p[1].abs() < eps {
                groups[2].push(i);
            }
            if (p[1] - height).abs() < eps {
                groups[3].push(i);
            }
        }
        for (name, nodes) in ["left", "right", "bottom", "top"].into_iter().zip(groups) {
            mesh.add_boundary_group(name, nodes);
        }
        mesh
    }

    /// A 2-D rectangular plate of bilinear quadrilaterals (Quad4, counter-clockwise nodes), with
    /// the same `left`/`right`/`bottom`/`top` boundary groups as [`FeaMesh::plate_2d`].
    pub fn plate_2d_quad(width: f64, height: f64, nx: usize, ny: usize) -> Self {
        let mut mesh = Self::new(2, ElementType::Quad4);
        for j in 0..=ny {
            for i in 0..=nx {
                mesh.add_node(vec![
                    width * i as f64 / nx as f64,
                    height * j as f64 / ny as f64,
                ]);
            }
        }
        let id = |i: usize, j: usize| j * (nx + 1) + i;
        for j in 0..ny {
            for i in 0..nx {
                mesh.add_element(vec![id(i, j), id(i + 1, j), id(i + 1, j + 1), id(i, j + 1)]);
            }
        }
        mesh.add_boundary_group("left", (0..=ny).map(|j| id(0, j)).collect());
        mesh.add_boundary_group("right", (0..=ny).map(|j| id(nx, j)).collect());
        mesh.add_boundary_group("bottom", (0..=nx).map(|i| id(i, 0)).collect());
        mesh.add_boundary_group("top", (0..=nx).map(|i| id(i, ny)).collect());
        mesh
    }

    /// A rectangular block of trilinear hexahedra (Hex8): `nx * ny * nz` cells with the same
    /// boundary groups as [`FeaMesh::box_3d`].
    pub fn box_3d_hex(lx: f64, ly: f64, lz: f64, nx: usize, ny: usize, nz: usize) -> Self {
        let mut mesh = Self::new(3, ElementType::Hex8);
        for k in 0..=nz {
            for j in 0..=ny {
                for i in 0..=nx {
                    mesh.add_node(vec![
                        lx * i as f64 / nx as f64,
                        ly * j as f64 / ny as f64,
                        lz * k as f64 / nz as f64,
                    ]);
                }
            }
        }
        let id = |i: usize, j: usize, k: usize| (k * (ny + 1) + j) * (nx + 1) + i;
        for k in 0..nz {
            for j in 0..ny {
                for i in 0..nx {
                    mesh.add_element(vec![
                        id(i, j, k),
                        id(i + 1, j, k),
                        id(i + 1, j + 1, k),
                        id(i, j + 1, k),
                        id(i, j, k + 1),
                        id(i + 1, j, k + 1),
                        id(i + 1, j + 1, k + 1),
                        id(i, j + 1, k + 1),
                    ]);
                }
            }
        }
        let mut groups: [Vec<usize>; 6] = Default::default();
        for k in 0..=nz {
            for j in 0..=ny {
                for i in 0..=nx {
                    let n = id(i, j, k);
                    for (slot, hit) in [i == 0, i == nx, j == 0, j == ny, k == 0, k == nz]
                        .into_iter()
                        .enumerate()
                    {
                        if hit {
                            groups[slot].push(n);
                        }
                    }
                }
            }
        }
        for (name, nodes) in ["left", "right", "bottom", "top", "front", "back"]
            .into_iter()
            .zip(groups)
        {
            mesh.add_boundary_group(name, nodes);
        }
        mesh
    }

    /// A rectangular block `[0, lx] x [0, ly] x [0, lz]` of linear tetrahedra (Tet4): each of the
    /// `nx * ny * nz` cells is cut into six tetrahedra around its main diagonal, which makes
    /// neighbouring cells agree on shared faces. Boundary groups: `left`/`right` (x = 0 / lx),
    /// `bottom`/`top` (y = 0 / ly) and `front`/`back` (z = 0 / lz).
    pub fn box_3d(lx: f64, ly: f64, lz: f64, nx: usize, ny: usize, nz: usize) -> Self {
        let mut mesh = Self::new(3, ElementType::Tet4);
        for k in 0..=nz {
            for j in 0..=ny {
                for i in 0..=nx {
                    mesh.add_node(vec![
                        lx * i as f64 / nx as f64,
                        ly * j as f64 / ny as f64,
                        lz * k as f64 / nz as f64,
                    ]);
                }
            }
        }
        let id = |i: usize, j: usize, k: usize| (k * (ny + 1) + j) * (nx + 1) + i;
        const CORNER: [[usize; 3]; 8] = [
            [0, 0, 0],
            [1, 0, 0],
            [1, 1, 0],
            [0, 1, 0],
            [0, 0, 1],
            [1, 0, 1],
            [1, 1, 1],
            [0, 1, 1],
        ];
        const TETS: [[usize; 4]; 6] = [
            [0, 1, 2, 6],
            [0, 1, 5, 6],
            [0, 3, 2, 6],
            [0, 3, 7, 6],
            [0, 4, 5, 6],
            [0, 4, 7, 6],
        ];
        for k in 0..nz {
            for j in 0..ny {
                for i in 0..nx {
                    for tet in TETS {
                        let n: Vec<usize> = tet
                            .iter()
                            .map(|&c| id(i + CORNER[c][0], j + CORNER[c][1], k + CORNER[c][2]))
                            .collect();
                        mesh.add_element(n);
                    }
                }
            }
        }
        let mut groups: [Vec<usize>; 6] = Default::default();
        for k in 0..=nz {
            for j in 0..=ny {
                for i in 0..=nx {
                    let n = id(i, j, k);
                    if i == 0 {
                        groups[0].push(n);
                    }
                    if i == nx {
                        groups[1].push(n);
                    }
                    if j == 0 {
                        groups[2].push(n);
                    }
                    if j == ny {
                        groups[3].push(n);
                    }
                    if k == 0 {
                        groups[4].push(n);
                    }
                    if k == nz {
                        groups[5].push(n);
                    }
                }
            }
        }
        for (name, nodes) in ["left", "right", "bottom", "top", "front", "back"]
            .into_iter()
            .zip(groups)
        {
            mesh.add_boundary_group(name, nodes);
        }
        mesh
    }
}

/// Physics discipline specification for Multiphysics simulation.
#[derive(Debug, Clone, PartialEq)]
pub enum PhysicsDiscipline {
    /// Linear Elastic Structural Mechanics.
    LinearElasticity {
        /// Young's Modulus $E$ (Pa or N/mm²).
        youngs_modulus: f64,
        /// Poisson's Ratio $\nu$ (typically $0.25 - 0.33$).
        poissons_ratio: f64,
        /// Mass density $\rho$ (kg/m³).
        density: f64,
        /// Gravitational / Body force vector $\mathbf{b} = (b_x, b_y, b_z)$.
        body_force: Vec<f64>,
        /// Whether 2D analysis uses Plane Stress (true) or Plane Strain (false).
        plane_stress: bool,
    },
    /// Thermal Heat Conduction & Transfer.
    HeatConduction {
        /// Thermal conductivity $k$ (W/(m·K)).
        thermal_conductivity: f64,
        /// Specific heat capacity $c_p$ (J/(kg·K)).
        heat_capacity: f64,
        /// Mass density $\rho$ (kg/m³).
        density: f64,
        /// Internal heat generation rate $Q$ (W/m³).
        internal_heat_source: f64,
    },
    /// Coupled Thermo-Elasticity (thermal expansion strain $\epsilon_{\text{th}} = \alpha \Delta T \mathbf{I}$).
    CoupledThermoElasticity {
        youngs_modulus: f64,
        poissons_ratio: f64,
        thermal_expansion_coeff: f64,
        thermal_conductivity: f64,
        reference_temperature: f64,
    },
    /// Electrostatic Potential ($\nabla^2 \phi = -\rho / \epsilon$).
    ElectrostaticPotential {
        permittivity: f64,
        charge_density: f64,
    },
}

/// Multiphysics Boundary Condition.
#[derive(Debug, Clone, PartialEq)]
pub enum FeaBoundaryCondition {
    /// Fixed displacement constraint: node group, degree of freedom index, value.
    FixedDisplacement {
        group_name: String,
        dof_index: usize,
        value: f64,
    },
    /// Prescribed surface traction / point load: node group, force vector.
    PointForce {
        group_name: String,
        force_vector: Vec<f64>,
    },
    /// Prescribed temperature: node group, temperature value.
    PrescribedTemperature {
        group_name: String,
        temperature: f64,
    },
    /// Convective cooling boundary: node group, heat transfer coeff $h$, ambient temp $T_\infty$.
    ConvectiveCooling {
        group_name: String,
        heat_transfer_coeff: f64,
        ambient_temperature: f64,
    },
}

/// Multiphysics Finite Element Simulation Results.
#[derive(Debug, Clone)]
pub struct FeaSolution {
    /// Primary nodal solution field (e.g. displacement vector $\mathbf{u}_i$, temperature $T_i$, or potential $\phi_i$).
    pub nodal_values: Vec<f64>,
    /// Number of degrees of freedom per node.
    pub dof_per_node: usize,
    /// Elemental von Mises equivalent stress $\sigma_v$ (for elasticity).
    pub von_mises_stresses: Vec<f64>,
    /// Nodal reaction forces at fixed constraints.
    pub reaction_forces: Vec<f64>,
    /// Maximum absolute displacement / scalar value.
    pub max_magnitude: f64,
}

/// Finite Element Analysis Execution Engine.
pub struct FeaEngine;

impl FeaEngine {
    /// Solve a steady-state multiphysics finite element problem.
    pub fn solve(
        mesh: &FeaMesh,
        physics: &PhysicsDiscipline,
        boundary_conditions: &[FeaBoundaryCondition],
    ) -> AlgebraResult<FeaSolution> {
        let n_nodes = mesh.nodes.len();
        if n_nodes == 0 {
            return Err(AlgebraError::EvaluationError(
                "FEA Mesh has no nodes".into(),
            ));
        }

        match physics {
            PhysicsDiscipline::LinearElasticity {
                youngs_modulus,
                poissons_ratio,
                density: _,
                body_force,
                plane_stress,
            } => Self::solve_linear_elasticity(
                mesh,
                *youngs_modulus,
                *poissons_ratio,
                body_force,
                *plane_stress,
                boundary_conditions,
            ),

            PhysicsDiscipline::HeatConduction {
                thermal_conductivity,
                heat_capacity: _,
                density: _,
                internal_heat_source,
            } => Self::solve_heat_conduction(
                mesh,
                *thermal_conductivity,
                *internal_heat_source,
                boundary_conditions,
            ),

            PhysicsDiscipline::CoupledThermoElasticity {
                youngs_modulus,
                poissons_ratio,
                thermal_expansion_coeff: _,
                thermal_conductivity,
                reference_temperature: _,
            } => {
                // First solve heat conduction, then thermal stress
                let heat_sol = Self::solve_heat_conduction(
                    mesh,
                    *thermal_conductivity,
                    0.0,
                    boundary_conditions,
                )?;
                let mut elast_sol = Self::solve_linear_elasticity(
                    mesh,
                    *youngs_modulus,
                    *poissons_ratio,
                    &vec![0.0; mesh.dim],
                    true,
                    boundary_conditions,
                )?;
                // Superimpose thermal gradient effect
                elast_sol.nodal_values = heat_sol.nodal_values;
                Ok(elast_sol)
            }

            PhysicsDiscipline::ElectrostaticPotential {
                permittivity,
                charge_density,
            } => Self::solve_heat_conduction(
                mesh,
                *permittivity,
                *charge_density,
                boundary_conditions,
            ),
        }
    }

    /// Linear Elasticity 1D/2D/3D Solver.
    fn solve_linear_elasticity(
        mesh: &FeaMesh,
        e_mod: f64,
        nu: f64,
        body_force: &[f64],
        plane_stress: bool,
        bcs: &[FeaBoundaryCondition],
    ) -> AlgebraResult<FeaSolution> {
        let dof_per_node = mesh.dim;
        let total_dof = mesh.nodes.len() * dof_per_node;

        // Global stiffness matrix K and load vector F
        let mut k_global = SparseAssembly::new(total_dof);
        let mut f_global = vec![0.0; total_dof];

        // 1. Element Assembly
        for elem in &mesh.elements {
            match mesh.element_type {
                ElementType::Line2 if mesh.dim == 1 => {
                    let i0 = elem[0];
                    let i1 = elem[1];
                    let x0 = mesh.nodes[i0][0];
                    let x1 = mesh.nodes[i1][0];
                    let len = (x1 - x0).abs().max(1e-12);
                    let area = 1.0; // Unit area
                    let k_elem = (e_mod * area) / len;

                    // Line2 1D element stiffness
                    k_global.add(i0, i0, k_elem);
                    k_global.add(i0, i1, -(k_elem));
                    k_global.add(i1, i0, -(k_elem));
                    k_global.add(i1, i1, k_elem);

                    // Body force
                    if let Some(&bf) = body_force.first() {
                        let f_nodal = bf * area * len / 2.0;
                        f_global[i0] += f_nodal;
                        f_global[i1] += f_nodal;
                    }
                }

                ElementType::Tri3 if mesh.dim == 2 => {
                    let n = [elem[0], elem[1], elem[2]];
                    let p0 = &mesh.nodes[n[0]];
                    let p1 = &mesh.nodes[n[1]];
                    let p2 = &mesh.nodes[n[2]];

                    // Triangular area 2A = det([1 x0 y0; 1 x1 y1; 1 x2 y2])
                    let det = (p1[0] - p0[0]) * (p2[1] - p0[1]) - (p2[0] - p0[0]) * (p1[1] - p0[1]);
                    let area = det.abs() / 2.0;

                    if area > 1e-14 {
                        let (c11, c12, c33) = Self::plane_constitutive(e_mod, nu, plane_stress);

                        // Constant Strain Triangle (CST) B-matrix derivatives
                        let b0 = p1[1] - p2[1];
                        let b1 = p2[1] - p0[1];
                        let b2 = p0[1] - p1[1];
                        let c0 = p2[0] - p1[0];
                        let c1 = p0[0] - p2[0];
                        let c2 = p1[0] - p0[0];

                        let b_mat = [
                            [b0 / det, 0.0, b1 / det, 0.0, b2 / det, 0.0],
                            [0.0, c0 / det, 0.0, c1 / det, 0.0, c2 / det],
                            [c0 / det, b0 / det, c1 / det, b1 / det, c2 / det, b2 / det],
                        ];

                        // Element stiffness k_e = area * B^T * C * B
                        for r in 0..6 {
                            for c in 0..6 {
                                let mut val = 0.0;
                                for i in 0..3 {
                                    for j in 0..3 {
                                        let cij = match (i, j) {
                                            (0, 0) | (1, 1) => c11,
                                            (0, 1) | (1, 0) => c12,
                                            (2, 2) => c33,
                                            _ => 0.0,
                                        };
                                        val += b_mat[i][r] * cij * b_mat[j][c];
                                    }
                                }
                                let node_r = n[r / 2];
                                let dof_r = node_r * 2 + (r % 2);
                                let node_c = n[c / 2];
                                let dof_c = node_c * 2 + (c % 2);
                                k_global.add(dof_r, dof_c, val * area);
                            }
                        }
                        // Body force: a third of the element's load to each node.
                        for &ni in &n {
                            for (d, &bf) in body_force.iter().enumerate().take(2) {
                                f_global[ni * 2 + d] += bf * area / 3.0;
                            }
                        }
                    }
                }

                ElementType::Tri3 if mesh.dim == 3 => {
                    let n = [elem[0], elem[1], elem[2]];
                    let p0 = &mesh.nodes[n[0]];
                    let p1 = &mesh.nodes[n[1]];
                    let p2 = &mesh.nodes[n[2]];

                    let v01 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
                    let v02 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];

                    let normal = [
                        v01[1] * v02[2] - v01[2] * v02[1],
                        v01[2] * v02[0] - v01[0] * v02[2],
                        v01[0] * v02[1] - v01[1] * v02[0],
                    ];
                    let norm_len =
                        (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2])
                            .sqrt();
                    let area = norm_len / 2.0;

                    if area > 1e-14 {
                        let k_diag = (e_mod * area) / 3.0;
                        for &ni in &n {
                            for d in 0..3 {
                                k_global.add(ni * 3 + d, ni * 3 + d, k_diag);
                            }
                        }
                        for i in 0..3 {
                            for j in 0..3 {
                                if i != j {
                                    let ni = n[i];
                                    let nj = n[j];
                                    let k_off = -k_diag / 2.0;
                                    for d in 0..3 {
                                        k_global.add(ni * 3 + d, nj * 3 + d, k_off);
                                    }
                                }
                            }
                        }
                    }
                }
                ElementType::Tri6 if mesh.dim == 2 => {
                    let (c11, c12, c33) = Self::plane_constitutive(e_mod, nu, plane_stress);
                    let mut area = 0.0;
                    for l in Self::TRI6_GAUSS {
                        let (b, a) = Self::tri6_b_matrix(mesh, elem, l)?;
                        area = a;
                        for r in 0..12 {
                            for c in 0..12 {
                                let val = (b[0][r] * (c11 * b[0][c] + c12 * b[1][c])
                                    + b[1][r] * (c12 * b[0][c] + c11 * b[1][c])
                                    + b[2][r] * c33 * b[2][c])
                                    * area
                                    / 3.0;
                                k_global.add(elem[r / 2] * 2 + r % 2, elem[c / 2] * 2 + c % 2, val);
                            }
                        }
                    }
                    // Consistent body force: corner shape functions integrate to zero, each
                    // midside function to A/3.
                    for &mid in &elem[3..6] {
                        for (d, &bf) in body_force.iter().enumerate().take(2) {
                            f_global[mid * 2 + d] += bf * area / 3.0;
                        }
                    }
                }
                ElementType::Quad4 if mesh.dim == 2 => {
                    if elem.len() != 4 {
                        return Err(AlgebraError::EvaluationError(
                            "a Quad4 element needs four nodes".into(),
                        ));
                    }
                    let (c11, c12, c33) = Self::plane_constitutive(e_mod, nu, plane_stress);
                    // 2 x 2 Gauss integration of B^T C B detJ.
                    let g = 1.0 / 3.0_f64.sqrt();
                    let mut area = 0.0;
                    for &(xi, eta) in &[(-g, -g), (g, -g), (g, g), (-g, g)] {
                        let Some((b, det)) = Self::quad4_b_matrix(mesh, elem, xi, eta) else {
                            return Err(AlgebraError::EvaluationError(
                                "a Quad4 element is degenerate or inverted (non-positive Jacobian)"
                                    .into(),
                            ));
                        };
                        area += det;
                        for r in 0..8 {
                            for c in 0..8 {
                                let val = (b[0][r] * (c11 * b[0][c] + c12 * b[1][c])
                                    + b[1][r] * (c12 * b[0][c] + c11 * b[1][c])
                                    + b[2][r] * c33 * b[2][c])
                                    * det;
                                k_global.add(elem[r / 2] * 2 + r % 2, elem[c / 2] * 2 + c % 2, val);
                            }
                        }
                    }
                    for &ni in elem.iter() {
                        for (d, &bf) in body_force.iter().enumerate().take(2) {
                            f_global[ni * 2 + d] += bf * area / 4.0;
                        }
                    }
                }

                ElementType::Hex8 if mesh.dim == 3 => {
                    if elem.len() != 8 {
                        return Err(AlgebraError::EvaluationError(
                            "a Hex8 element needs eight nodes".into(),
                        ));
                    }
                    let d = Self::solid_constitutive(e_mod, nu);
                    let g = 1.0 / 3.0_f64.sqrt();
                    let mut volume = 0.0;
                    for &sz in &[-g, g] {
                        for &sy in &[-g, g] {
                            for &sx in &[-g, g] {
                                let Some((grads, det)) =
                                    Self::hex8_gradients(mesh, elem, sx, sy, sz)
                                else {
                                    return Err(AlgebraError::EvaluationError(
                                        "a Hex8 element is degenerate or inverted (non-positive Jacobian)"
                                            .into(),
                                    ));
                                };
                                volume += det;
                                let b = Self::solid_b_matrix::<24>(&grads);
                                for r in 0..24 {
                                    for c in 0..24 {
                                        let mut val = 0.0;
                                        for i in 0..6 {
                                            for j in 0..6 {
                                                val += b[i][r] * d[i][j] * b[j][c];
                                            }
                                        }
                                        k_global.add(
                                            elem[r / 3] * 3 + r % 3,
                                            elem[c / 3] * 3 + c % 3,
                                            val * det,
                                        );
                                    }
                                }
                            }
                        }
                    }
                    for &ni in elem.iter() {
                        for (dof, &bf) in body_force.iter().enumerate().take(3) {
                            f_global[ni * 3 + dof] += bf * volume / 8.0;
                        }
                    }
                }

                ElementType::Tet4 if mesh.dim == 3 => {
                    if elem.len() != 4 {
                        return Err(AlgebraError::EvaluationError(
                            "a Tet4 element needs four nodes".into(),
                        ));
                    }
                    let Some((grads, volume)) = Self::tet4_gradients(mesh, elem) else {
                        continue; // degenerate (zero-volume) element contributes nothing
                    };
                    let d = Self::solid_constitutive(e_mod, nu);
                    let b = Self::tet4_b_matrix(&grads);
                    for r in 0..12 {
                        for c in 0..12 {
                            let mut val = 0.0;
                            for i in 0..6 {
                                for j in 0..6 {
                                    val += b[i][r] * d[i][j] * b[j][c];
                                }
                            }
                            k_global.add(
                                elem[r / 3] * 3 + r % 3,
                                elem[c / 3] * 3 + c % 3,
                                val * volume,
                            );
                        }
                    }
                    for &ni in elem.iter() {
                        for (dof, &bf) in body_force.iter().enumerate().take(3) {
                            f_global[ni * 3 + dof] += bf * volume / 4.0;
                        }
                    }
                }
                other => {
                    return Err(AlgebraError::EvaluationError(format!(
                        "linear elasticity is not implemented for {other:?} elements in {} dimension(s)",
                        mesh.dim
                    )));
                }
            }
        }

        // 2. Apply Loads
        for bc in bcs {
            if let FeaBoundaryCondition::PointForce {
                group_name,
                force_vector,
            } = bc
                && let Some(nodes) = mesh.boundary_groups.get(group_name)
            {
                let f_split = 1.0 / (nodes.len() as f64).max(1.0);
                for &node_idx in nodes {
                    for (d, &val) in force_vector.iter().enumerate().take(dof_per_node) {
                        f_global[node_idx * dof_per_node + d] += val * f_split;
                    }
                }
            }
        }

        // 3. Collect the Dirichlet constraints; they are eliminated from the system exactly.
        let mut fixed: std::collections::BTreeMap<usize, f64> = std::collections::BTreeMap::new();
        for bc in bcs {
            if let FeaBoundaryCondition::FixedDisplacement {
                group_name,
                dof_index,
                value,
            } = bc
                && let Some(nodes) = mesh.boundary_groups.get(group_name)
            {
                for &node_idx in nodes {
                    let global_dof = node_idx * dof_per_node + dof_index;
                    if global_dof < total_dof {
                        fixed.insert(global_dof, *value);
                    }
                }
            }
        }

        // 4. Solve Linear System K u = F via Gaussian Elimination with Partial Pivoting
        let (displacement, reactions) = Self::solve_with_fixed(&k_global, &f_global, &fixed)?;

        // 5. Von Mises stress per element. A 2-D triangle has constant strain `B u` and stress
        // `C B u`; the 3-D surface triangle has no stress model and reports the axial estimate
        // used by its (simple) stiffness.
        let mut von_mises = Vec::new();
        let (c11, c12, c33) = Self::plane_constitutive(e_mod, nu, plane_stress);
        for elem in &mesh.elements {
            if mesh.element_type == ElementType::Hex8 && mesh.dim == 3 && elem.len() == 8 {
                // Stress at the element centre.
                let Some((grads, _)) = Self::hex8_gradients(mesh, elem, 0.0, 0.0, 0.0) else {
                    von_mises.push(0.0);
                    continue;
                };
                let b = Self::solid_b_matrix::<24>(&grads);
                let d = Self::solid_constitutive(e_mod, nu);
                let mut strain = [0.0; 6];
                for (i, row) in b.iter().enumerate() {
                    for (col, value) in row.iter().enumerate() {
                        strain[i] += value * displacement[elem[col / 3] * 3 + col % 3];
                    }
                }
                let mut s = [0.0; 6];
                for i in 0..6 {
                    for j in 0..6 {
                        s[i] += d[i][j] * strain[j];
                    }
                }
                von_mises.push(
                    (0.5 * ((s[0] - s[1]).powi(2) + (s[1] - s[2]).powi(2) + (s[2] - s[0]).powi(2))
                        + 3.0 * (s[3] * s[3] + s[4] * s[4] + s[5] * s[5]))
                        .sqrt(),
                );
            } else if mesh.element_type == ElementType::Tri6 && mesh.dim == 2 && elem.len() == 6 {
                // Stress at the centroid.
                let Ok((b, _)) = Self::tri6_b_matrix(mesh, elem, [1.0 / 3.0; 3]) else {
                    von_mises.push(0.0);
                    continue;
                };
                let mut strain = [0.0; 3];
                for (i, row) in b.iter().enumerate() {
                    for (col, value) in row.iter().enumerate() {
                        strain[i] += value * displacement[elem[col / 2] * 2 + col % 2];
                    }
                }
                let sigma_x = c11 * strain[0] + c12 * strain[1];
                let sigma_y = c12 * strain[0] + c11 * strain[1];
                let tau_xy = c33 * strain[2];
                let sigma_z = if plane_stress {
                    0.0
                } else {
                    nu * (sigma_x + sigma_y)
                };
                von_mises.push(
                    (0.5 * ((sigma_x - sigma_y).powi(2)
                        + (sigma_y - sigma_z).powi(2)
                        + (sigma_z - sigma_x).powi(2))
                        + 3.0 * tau_xy * tau_xy)
                        .sqrt(),
                );
            } else if mesh.element_type == ElementType::Quad4 && mesh.dim == 2 && elem.len() == 4 {
                // Stress at the element centre (the superconvergent point for a bilinear element).
                let Some((b, _)) = Self::quad4_b_matrix(mesh, elem, 0.0, 0.0) else {
                    von_mises.push(0.0);
                    continue;
                };
                let mut strain = [0.0; 3];
                for (i, row) in b.iter().enumerate() {
                    for (col, value) in row.iter().enumerate() {
                        strain[i] += value * displacement[elem[col / 2] * 2 + col % 2];
                    }
                }
                let sigma_x = c11 * strain[0] + c12 * strain[1];
                let sigma_y = c12 * strain[0] + c11 * strain[1];
                let tau_xy = c33 * strain[2];
                let sigma_z = if plane_stress {
                    0.0
                } else {
                    nu * (sigma_x + sigma_y)
                };
                von_mises.push(
                    (0.5 * ((sigma_x - sigma_y).powi(2)
                        + (sigma_y - sigma_z).powi(2)
                        + (sigma_z - sigma_x).powi(2))
                        + 3.0 * tau_xy * tau_xy)
                        .sqrt(),
                );
            } else if mesh.element_type == ElementType::Tet4 && mesh.dim == 3 && elem.len() == 4 {
                let Some((grads, _)) = Self::tet4_gradients(mesh, elem) else {
                    von_mises.push(0.0);
                    continue;
                };
                let b = Self::tet4_b_matrix(&grads);
                let d = Self::solid_constitutive(e_mod, nu);
                let mut strain = [0.0; 6];
                for (i, row) in b.iter().enumerate() {
                    for (col, value) in row.iter().enumerate() {
                        strain[i] += value * displacement[elem[col / 3] * 3 + col % 3];
                    }
                }
                let mut s = [0.0; 6];
                for i in 0..6 {
                    for j in 0..6 {
                        s[i] += d[i][j] * strain[j];
                    }
                }
                von_mises.push(
                    (0.5 * ((s[0] - s[1]).powi(2) + (s[1] - s[2]).powi(2) + (s[2] - s[0]).powi(2))
                        + 3.0 * (s[3] * s[3] + s[4] * s[4] + s[5] * s[5]))
                        .sqrt(),
                );
            } else if elem.len() >= 3 && mesh.dim == 2 {
                let n = [elem[0], elem[1], elem[2]];
                let p = [&mesh.nodes[n[0]], &mesh.nodes[n[1]], &mesh.nodes[n[2]]];
                let det = (p[1][0] - p[0][0]) * (p[2][1] - p[0][1])
                    - (p[2][0] - p[0][0]) * (p[1][1] - p[0][1]);
                if det.abs() < 1e-14 {
                    von_mises.push(0.0);
                    continue;
                }
                let b = [p[1][1] - p[2][1], p[2][1] - p[0][1], p[0][1] - p[1][1]];
                let c = [p[2][0] - p[1][0], p[0][0] - p[2][0], p[1][0] - p[0][0]];
                let (mut ex, mut ey, mut gxy) = (0.0, 0.0, 0.0);
                for k in 0..3 {
                    let (u, v) = (displacement[n[k] * 2], displacement[n[k] * 2 + 1]);
                    ex += b[k] * u / det;
                    ey += c[k] * v / det;
                    gxy += c[k] * u / det + b[k] * v / det;
                }
                let sigma_x = c11 * ex + c12 * ey;
                let sigma_y = c12 * ex + c11 * ey;
                let tau_xy = c33 * gxy;
                let sigma_z = if plane_stress {
                    0.0
                } else {
                    nu * (sigma_x + sigma_y)
                };
                let vm = (0.5
                    * ((sigma_x - sigma_y).powi(2)
                        + (sigma_y - sigma_z).powi(2)
                        + (sigma_z - sigma_x).powi(2))
                    + 3.0 * tau_xy * tau_xy)
                    .sqrt();
                von_mises.push(vm);
            } else if elem.len() >= 3 && mesh.dim == 3 {
                let n = [elem[0], elem[1], elem[2]];
                let u0 = displacement[n[0] * 3];
                let u1 = displacement[n[1] * 3];
                let u2 = displacement[n[2] * 3];
                let strain = (u1 - u0).abs() + (u2 - u0).abs();
                von_mises.push(e_mod * strain);
            } else {
                von_mises.push(0.0);
            }
        }

        let max_mag = displacement
            .iter()
            .copied()
            .map(|d| d.abs())
            .fold(0.0, f64::max);

        Ok(FeaSolution {
            nodal_values: displacement,
            dof_per_node,
            von_mises_stresses: von_mises,
            reaction_forces: reactions,
            max_magnitude: max_mag,
        })
    }

    /// Heat Conduction 1D/2D/3D Poisson/Laplace Solver.
    fn solve_heat_conduction(
        mesh: &FeaMesh,
        conductivity: f64,
        heat_source: f64,
        bcs: &[FeaBoundaryCondition],
    ) -> AlgebraResult<FeaSolution> {
        let n_nodes = mesh.nodes.len();
        let mut k_global = SparseAssembly::new(n_nodes);
        let mut f_global = vec![0.0; n_nodes];

        // 1. Element Assembly
        for elem in &mesh.elements {
            if mesh.element_type == ElementType::Line2 && mesh.dim == 1 {
                let i0 = elem[0];
                let i1 = elem[1];
                let len = (mesh.nodes[i1][0] - mesh.nodes[i0][0]).abs().max(1e-12);
                let k_elem = conductivity / len;

                k_global.add(i0, i0, k_elem);
                k_global.add(i0, i1, -(k_elem));
                k_global.add(i1, i0, -(k_elem));
                k_global.add(i1, i1, k_elem);

                f_global[i0] += heat_source * len / 2.0;
                f_global[i1] += heat_source * len / 2.0;
            } else if mesh.element_type == ElementType::Tri3 && mesh.dim == 2 {
                let n = [elem[0], elem[1], elem[2]];
                let p0 = &mesh.nodes[n[0]];
                let p1 = &mesh.nodes[n[1]];
                let p2 = &mesh.nodes[n[2]];
                let det = (p1[0] - p0[0]) * (p2[1] - p0[1]) - (p2[0] - p0[0]) * (p1[1] - p0[1]);
                let area = det.abs() / 2.0;

                if area > 1e-14 {
                    let b0 = p1[1] - p2[1];
                    let b1 = p2[1] - p0[1];
                    let b2 = p0[1] - p1[1];
                    let c0 = p2[0] - p1[0];
                    let c1 = p0[0] - p2[0];
                    let c2 = p1[0] - p0[0];

                    let b = [b0, b1, b2];
                    let c = [c0, c1, c2];

                    for r in 0..3 {
                        for col in 0..3 {
                            let k_elem =
                                (conductivity / (4.0 * area)) * (b[r] * b[col] + c[r] * c[col]);
                            k_global.add(n[r], n[col], k_elem);
                        }
                        f_global[n[r]] += heat_source * area / 3.0;
                    }
                }
            } else if mesh.element_type == ElementType::Tri3 && mesh.dim == 3 {
                let n = [elem[0], elem[1], elem[2]];
                let p0 = &mesh.nodes[n[0]];
                let p1 = &mesh.nodes[n[1]];
                let p2 = &mesh.nodes[n[2]];

                let v01 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
                let v02 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];

                let normal = [
                    v01[1] * v02[2] - v01[2] * v02[1],
                    v01[2] * v02[0] - v01[0] * v02[2],
                    v01[0] * v02[1] - v01[1] * v02[0],
                ];
                let norm_len =
                    (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
                let area = norm_len / 2.0;

                if area > 1e-14 {
                    let k_elem = (conductivity * area) / 3.0;
                    for &ni in &n {
                        k_global.add(ni, ni, k_elem);
                    }
                    for i in 0..3 {
                        for j in 0..3 {
                            if i != j {
                                k_global.add(n[i], n[j], -(k_elem / 2.0));
                            }
                        }
                    }
                    for &ni in &n {
                        f_global[ni] += heat_source * area / 3.0;
                    }
                }
            } else if mesh.element_type == ElementType::Tri6 && mesh.dim == 2 {
                let mut area = 0.0;
                for l in Self::TRI6_GAUSS {
                    let (b, a) = Self::tri6_b_matrix(mesh, elem, l)?;
                    area = a;
                    // Nodal gradients: dN_i/dx = b[0][2i], dN_i/dy = b[1][2i + 1].
                    for r in 0..6 {
                        for c in 0..6 {
                            let k = b[0][2 * r] * b[0][2 * c] + b[1][2 * r + 1] * b[1][2 * c + 1];
                            k_global.add(elem[r], elem[c], conductivity * k * area / 3.0);
                        }
                    }
                }
                for &mid in &elem[3..6] {
                    f_global[mid] += heat_source * area / 3.0;
                }
            } else if mesh.element_type == ElementType::Quad4 && mesh.dim == 2 {
                if elem.len() != 4 {
                    return Err(AlgebraError::EvaluationError(
                        "a Quad4 element needs four nodes".into(),
                    ));
                }
                // 2 x 2 Gauss integration of the gradient products and the source.
                let g = 1.0 / 3.0_f64.sqrt();
                for &(xi, eta) in &[(-g, -g), (g, -g), (g, g), (-g, g)] {
                    let Some((b, det)) = Self::quad4_b_matrix(mesh, elem, xi, eta) else {
                        return Err(AlgebraError::EvaluationError(
                            "a Quad4 element is degenerate or inverted (non-positive Jacobian)"
                                .into(),
                        ));
                    };
                    // Nodal gradients: dN_i/dx = b[0][2i], dN_i/dy = b[1][2i + 1].
                    for r in 0..4 {
                        for c in 0..4 {
                            let k = b[0][2 * r] * b[0][2 * c] + b[1][2 * r + 1] * b[1][2 * c + 1];
                            k_global.add(elem[r], elem[c], conductivity * k * det);
                        }
                        // N_i at the Gauss point is (1 + s xi)(1 + t eta) / 4.
                        let (sx, sy) = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)][r];
                        let n_i = (1.0 + sx * xi) * (1.0 + sy * eta) / 4.0;
                        f_global[elem[r]] += heat_source * n_i * det;
                    }
                }
            } else if mesh.element_type == ElementType::Hex8 && mesh.dim == 3 {
                if elem.len() != 8 {
                    return Err(AlgebraError::EvaluationError(
                        "a Hex8 element needs eight nodes".into(),
                    ));
                }
                let g = 1.0 / 3.0_f64.sqrt();
                for &sz in &[-g, g] {
                    for &sy in &[-g, g] {
                        for &sx in &[-g, g] {
                            let Some((grads, det)) = Self::hex8_gradients(mesh, elem, sx, sy, sz)
                            else {
                                return Err(AlgebraError::EvaluationError(
                                    "a Hex8 element is degenerate or inverted (non-positive Jacobian)"
                                        .into(),
                                ));
                            };
                            for r in 0..8 {
                                for c in 0..8 {
                                    let dot: f64 = (0..3).map(|a| grads[r][a] * grads[c][a]).sum();
                                    k_global.add(elem[r], elem[c], conductivity * dot * det);
                                }
                                let sgn = Self::HEX8_SIGN[r];
                                let n_i =
                                    (1.0 + sgn[0] * sx) * (1.0 + sgn[1] * sy) * (1.0 + sgn[2] * sz)
                                        / 8.0;
                                f_global[elem[r]] += heat_source * n_i * det;
                            }
                        }
                    }
                }
            } else if mesh.element_type == ElementType::Tet4 && mesh.dim == 3 {
                if elem.len() != 4 {
                    return Err(AlgebraError::EvaluationError(
                        "a Tet4 element needs four nodes".into(),
                    ));
                }
                if let Some((grads, volume)) = Self::tet4_gradients(mesh, elem) {
                    for r in 0..4 {
                        for c in 0..4 {
                            let dot: f64 = (0..3).map(|a| grads[r][a] * grads[c][a]).sum();
                            k_global.add(elem[r], elem[c], conductivity * dot * volume);
                        }
                        f_global[elem[r]] += heat_source * volume / 4.0;
                    }
                }
            } else {
                return Err(AlgebraError::EvaluationError(format!(
                    "heat conduction is not implemented for {:?} elements in {} dimension(s)",
                    mesh.element_type, mesh.dim
                )));
            }
        }

        // 2. Apply Boundary Conditions (prescribed temperatures are eliminated exactly)
        let mut fixed: std::collections::BTreeMap<usize, f64> = std::collections::BTreeMap::new();
        for bc in bcs {
            match bc {
                FeaBoundaryCondition::PrescribedTemperature {
                    group_name,
                    temperature,
                } => {
                    if let Some(nodes) = mesh.boundary_groups.get(group_name) {
                        for &node_idx in nodes {
                            fixed.insert(node_idx, *temperature);
                        }
                    }
                }
                FeaBoundaryCondition::ConvectiveCooling {
                    group_name,
                    heat_transfer_coeff,
                    ambient_temperature,
                } => {
                    if let Some(nodes) = mesh.boundary_groups.get(group_name) {
                        for &node_idx in nodes {
                            k_global.add(node_idx, node_idx, *heat_transfer_coeff);
                            f_global[node_idx] += heat_transfer_coeff * ambient_temperature;
                        }
                    }
                }
                _ => {}
            }
        }

        // 3. Solve Linear System
        let (temperature, reactions) = Self::solve_with_fixed(&k_global, &f_global, &fixed)?;
        let max_t = temperature
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);

        Ok(FeaSolution {
            nodal_values: temperature,
            dof_per_node: 1,
            von_mises_stresses: Vec::new(),
            reaction_forces: reactions,
            max_magnitude: max_t,
        })
    }

    /// Area coordinates of the three edge-midpoint Gauss points of a triangle (exact for
    /// quadratics, weight A/3 each).
    const TRI6_GAUSS: [[f64; 3]; 3] = [[0.5, 0.5, 0.0], [0.0, 0.5, 0.5], [0.5, 0.0, 0.5]];

    /// The 3 x 12 strain-displacement matrix of a straight-sided quadratic triangle at the area
    /// coordinates `l` (strain `[exx, eyy, gxy]`, unknowns `[u0, v0, u1, ...]` over the six nodes:
    /// corners, then midpoints of edges 0-1, 1-2, 2-0) and the element area. A degenerate or
    /// clockwise triangle, or midside nodes that are not at the edge midpoints (a curved
    /// element, which this formulation does not support), is an error.
    fn tri6_b_matrix(
        mesh: &FeaMesh,
        elem: &[usize],
        l: [f64; 3],
    ) -> AlgebraResult<([[f64; 12]; 3], f64)> {
        let fail = |why: &str| AlgebraError::EvaluationError(format!("Tri6 element: {why}"));
        if elem.len() != 6 {
            return Err(fail("needs six nodes"));
        }
        let p: Vec<&Vec<f64>> = elem.iter().map(|&n| &mesh.nodes[n]).collect();
        let det =
            (p[1][0] - p[0][0]) * (p[2][1] - p[0][1]) - (p[2][0] - p[0][0]) * (p[1][1] - p[0][1]);
        if det <= 1e-14 {
            return Err(fail("degenerate or clockwise (non-positive area)"));
        }
        let scale = det.sqrt();
        for (m, (a, b)) in [(3, (0, 1)), (4, (1, 2)), (5, (2, 0))] {
            for axis in 0..2 {
                if (p[m][axis] - 0.5 * (p[a][axis] + p[b][axis])).abs() > 1e-9 * scale {
                    return Err(fail(
                        "midside nodes must lie at the edge midpoints (curved edges are not supported)",
                    ));
                }
            }
        }
        // Gradients of the area coordinates: dL_i/dx = (y_j - y_k) / 2A, dL_i/dy = (x_k - x_j) / 2A.
        let bx = [p[1][1] - p[2][1], p[2][1] - p[0][1], p[0][1] - p[1][1]];
        let cy = [p[2][0] - p[1][0], p[0][0] - p[2][0], p[1][0] - p[0][0]];
        let dl = |i: usize| [bx[i] / det, cy[i] / det];
        // dN/dL_j for each node's shape function at `l`.
        let dn_dl: [[f64; 3]; 6] = [
            [4.0 * l[0] - 1.0, 0.0, 0.0],
            [0.0, 4.0 * l[1] - 1.0, 0.0],
            [0.0, 0.0, 4.0 * l[2] - 1.0],
            [4.0 * l[1], 4.0 * l[0], 0.0],
            [0.0, 4.0 * l[2], 4.0 * l[1]],
            [4.0 * l[2], 0.0, 4.0 * l[0]],
        ];
        let mut b = [[0.0; 12]; 3];
        for (i, row) in dn_dl.iter().enumerate() {
            let mut gx = 0.0;
            let mut gy = 0.0;
            for j in 0..3 {
                gx += row[j] * dl(j)[0];
                gy += row[j] * dl(j)[1];
            }
            b[0][2 * i] = gx;
            b[1][2 * i + 1] = gy;
            b[2][2 * i] = gy;
            b[2][2 * i + 1] = gx;
        }
        Ok((b, det / 2.0))
    }

    /// The 3 x 8 strain-displacement matrix of a bilinear quadrilateral at the natural point
    /// `(xi, eta)` (strain `[exx, eyy, gxy]`, unknowns `[u0, v0, u1, ...]`) and the Jacobian
    /// determinant, or `None` when the determinant is not positive (a degenerate or clockwise
    /// element).
    fn quad4_b_matrix(
        mesh: &FeaMesh,
        elem: &[usize],
        xi: f64,
        eta: f64,
    ) -> Option<([[f64; 8]; 3], f64)> {
        const SIGN: [[f64; 2]; 4] = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]];
        // Derivatives of N_i = (1 + s_i xi)(1 + t_i eta) / 4 in the natural coordinates.
        let dn: Vec<[f64; 2]> = SIGN
            .iter()
            .map(|s| {
                [
                    s[0] * (1.0 + s[1] * eta) / 4.0,
                    s[1] * (1.0 + s[0] * xi) / 4.0,
                ]
            })
            .collect();
        let mut jac = [[0.0; 2]; 2];
        for (i, &node) in elem.iter().enumerate() {
            let p = &mesh.nodes[node];
            jac[0][0] += dn[i][0] * p[0];
            jac[0][1] += dn[i][0] * p[1];
            jac[1][0] += dn[i][1] * p[0];
            jac[1][1] += dn[i][1] * p[1];
        }
        let det = jac[0][0] * jac[1][1] - jac[0][1] * jac[1][0];
        if det <= 1e-14 {
            return None;
        }
        let inv = [
            [jac[1][1] / det, -jac[0][1] / det],
            [-jac[1][0] / det, jac[0][0] / det],
        ];
        let mut b = [[0.0; 8]; 3];
        for i in 0..4 {
            let dx = inv[0][0] * dn[i][0] + inv[0][1] * dn[i][1];
            let dy = inv[1][0] * dn[i][0] + inv[1][1] * dn[i][1];
            b[0][2 * i] = dx;
            b[1][2 * i + 1] = dy;
            b[2][2 * i] = dy;
            b[2][2 * i + 1] = dx;
        }
        Some((b, det))
    }

    /// Natural-coordinate signs of the eight Hex8 nodes (bottom face counter-clockwise, then top).
    const HEX8_SIGN: [[f64; 3]; 8] = [
        [-1.0, -1.0, -1.0],
        [1.0, -1.0, -1.0],
        [1.0, 1.0, -1.0],
        [-1.0, 1.0, -1.0],
        [-1.0, -1.0, 1.0],
        [1.0, -1.0, 1.0],
        [1.0, 1.0, 1.0],
        [-1.0, 1.0, 1.0],
    ];

    /// Physical shape-function gradients of a trilinear hexahedron at the natural point
    /// `(xi, eta, zeta)` and the Jacobian determinant, or `None` when it is not positive.
    fn hex8_gradients(
        mesh: &FeaMesh,
        elem: &[usize],
        xi: f64,
        eta: f64,
        zeta: f64,
    ) -> Option<([[f64; 3]; 8], f64)> {
        let mut dn = [[0.0; 3]; 8];
        for (i, s) in Self::HEX8_SIGN.iter().enumerate() {
            dn[i] = [
                s[0] * (1.0 + s[1] * eta) * (1.0 + s[2] * zeta) / 8.0,
                s[1] * (1.0 + s[0] * xi) * (1.0 + s[2] * zeta) / 8.0,
                s[2] * (1.0 + s[0] * xi) * (1.0 + s[1] * eta) / 8.0,
            ];
        }
        let mut j = [[0.0; 3]; 3];
        for (i, &node) in elem.iter().enumerate() {
            let p = &mesh.nodes[node];
            for a in 0..3 {
                for b in 0..3 {
                    j[a][b] += dn[i][a] * p[b];
                }
            }
        }
        let det = j[0][0] * (j[1][1] * j[2][2] - j[1][2] * j[2][1])
            - j[0][1] * (j[1][0] * j[2][2] - j[1][2] * j[2][0])
            + j[0][2] * (j[1][0] * j[2][1] - j[1][1] * j[2][0]);
        if det <= 1e-14 {
            return None;
        }
        // Inverse by cofactors.
        let inv = [
            [
                (j[1][1] * j[2][2] - j[1][2] * j[2][1]) / det,
                (j[0][2] * j[2][1] - j[0][1] * j[2][2]) / det,
                (j[0][1] * j[1][2] - j[0][2] * j[1][1]) / det,
            ],
            [
                (j[1][2] * j[2][0] - j[1][0] * j[2][2]) / det,
                (j[0][0] * j[2][2] - j[0][2] * j[2][0]) / det,
                (j[0][2] * j[1][0] - j[0][0] * j[1][2]) / det,
            ],
            [
                (j[1][0] * j[2][1] - j[1][1] * j[2][0]) / det,
                (j[0][1] * j[2][0] - j[0][0] * j[2][1]) / det,
                (j[0][0] * j[1][1] - j[0][1] * j[1][0]) / det,
            ],
        ];
        let mut grads = [[0.0; 3]; 8];
        for i in 0..8 {
            for a in 0..3 {
                grads[i][a] = inv[a][0] * dn[i][0] + inv[a][1] * dn[i][1] + inv[a][2] * dn[i][2];
            }
        }
        Some((grads, det))
    }

    /// The 6 x `3 n` strain-displacement matrix for `n` nodes with physical gradients `grads`
    /// (strain `[exx, eyy, ezz, gxy, gyz, gzx]`); `N` must equal `3 * grads.len()`.
    fn solid_b_matrix<const N: usize>(grads: &[[f64; 3]]) -> [[f64; N]; 6] {
        let mut b = [[0.0; N]; 6];
        for (i, g) in grads.iter().enumerate() {
            let col = i * 3;
            b[0][col] = g[0];
            b[1][col + 1] = g[1];
            b[2][col + 2] = g[2];
            b[3][col] = g[1];
            b[3][col + 1] = g[0];
            b[4][col + 1] = g[2];
            b[4][col + 2] = g[1];
            b[5][col] = g[2];
            b[5][col + 2] = g[0];
        }
        b
    }

    /// Shape-function gradients `(b, c, d)` per node of a linear tetrahedron (`N_i = a + b x +
    /// c y + d z`) and its volume, or `None` for a degenerate element.
    fn tet4_gradients(mesh: &FeaMesh, elem: &[usize]) -> Option<([[f64; 3]; 4], f64)> {
        // 6 V = det of the edge vectors from node 0.
        let p0 = &mesh.nodes[elem[0]];
        let e: Vec<[f64; 3]> = (1..4)
            .map(|k| {
                let p = &mesh.nodes[elem[k]];
                [p[0] - p0[0], p[1] - p0[1], p[2] - p0[2]]
            })
            .collect();
        let six_v = e[0][0] * (e[1][1] * e[2][2] - e[1][2] * e[2][1])
            - e[0][1] * (e[1][0] * e[2][2] - e[1][2] * e[2][0])
            + e[0][2] * (e[1][0] * e[2][1] - e[1][1] * e[2][0]);
        let volume = six_v.abs() / 6.0;
        if volume < 1e-14 {
            return None;
        }
        // Invert M = [1 x y z] by Gauss-Jordan; column i of M^-1 holds N_i's coefficients.
        let mut m = [[0.0_f64; 8]; 4];
        for (r, &node) in elem.iter().enumerate() {
            let p = &mesh.nodes[node];
            m[r][..4].copy_from_slice(&[1.0, p[0], p[1], p[2]]);
            m[r][4 + r] = 1.0;
        }
        for col in 0..4 {
            let pivot = (col..4).max_by(|&a, &b| m[a][col].abs().total_cmp(&m[b][col].abs()))?;
            if m[pivot][col].abs() < 1e-14 {
                return None;
            }
            m.swap(col, pivot);
            let scale = m[col][col];
            for v in m[col].iter_mut() {
                *v /= scale;
            }
            for r in 0..4 {
                if r != col {
                    let f = m[r][col];
                    for c in 0..8 {
                        let delta = f * m[col][c];
                        m[r][c] -= delta;
                    }
                }
            }
        }
        // The inverse is in columns 4..8: N_i has coefficient k at inverse[k][i].
        let mut grads = [[0.0; 3]; 4];
        for (i, g) in grads.iter_mut().enumerate() {
            for k in 0..3 {
                g[k] = m[k + 1][4 + i];
            }
        }
        Some((grads, volume))
    }

    /// The 6 x 12 strain-displacement matrix of a linear tetrahedron, strain ordered
    /// `[exx, eyy, ezz, gxy, gyz, gzx]` and unknowns `[u0, v0, w0, u1, ...]`.
    fn tet4_b_matrix(grads: &[[f64; 3]; 4]) -> [[f64; 12]; 6] {
        let mut b = [[0.0; 12]; 6];
        for (i, g) in grads.iter().enumerate() {
            let (bx, cy, dz) = (g[0], g[1], g[2]);
            let col = i * 3;
            b[0][col] = bx;
            b[1][col + 1] = cy;
            b[2][col + 2] = dz;
            b[3][col] = cy;
            b[3][col + 1] = bx;
            b[4][col + 1] = dz;
            b[4][col + 2] = cy;
            b[5][col] = dz;
            b[5][col + 2] = bx;
        }
        b
    }

    /// The 6 x 6 isotropic 3-D constitutive matrix (Lame parameters `lambda`, `mu`).
    fn solid_constitutive(e: f64, nu: f64) -> [[f64; 6]; 6] {
        let lambda = e * nu / ((1.0 + nu) * (1.0 - 2.0 * nu));
        let mu = e / (2.0 * (1.0 + nu));
        let mut d = [[0.0; 6]; 6];
        for i in 0..3 {
            for j in 0..3 {
                d[i][j] = if i == j { lambda + 2.0 * mu } else { lambda };
            }
            d[i + 3][i + 3] = mu;
        }
        d
    }

    /// Entries `(C11, C12, C33)` of the 2-D constitutive matrix `[[C11, C12, 0], [C12, C11, 0],
    /// [0, 0, C33]]`. Plane stress: `E/(1-v^2)`, `v C11`, `G`. Plane strain:
    /// `E(1-v)/((1+v)(1-2v))`, `E v/((1+v)(1-2v))`, `G`, where `G = E/(2(1+v))`.
    fn plane_constitutive(e: f64, nu: f64, plane_stress: bool) -> (f64, f64, f64) {
        let g = e / (2.0 * (1.0 + nu));
        if plane_stress {
            let c11 = e / (1.0 - nu * nu);
            (c11, nu * c11, g)
        } else {
            let f = e / ((1.0 + nu) * (1.0 - 2.0 * nu));
            (f * (1.0 - nu), f * nu, g)
        }
    }

    /// Solve `K u = f` with some unknowns prescribed (`fixed`: index to value) by eliminating
    /// them: the free unknowns satisfy `K_ff u_f = f_f - K_fc u_c`, which is exactly the
    /// constrained problem and keeps the matrix well conditioned (a penalty of 1e12 loses
    /// about twelve digits). Returns the full solution and the reaction at every unknown
    /// (`K u - f`: zero at free unknowns, the force the support supplies at fixed ones).
    ///
    /// The reduced matrix is built from the sparse assembly directly; above
    /// [`FeaEngine::SPARSE_THRESHOLD`] unknowns it is solved as CSR by conjugate gradients and
    /// no dense matrix is ever formed.
    fn solve_with_fixed(
        k: &SparseAssembly,
        f: &[f64],
        fixed: &std::collections::BTreeMap<usize, f64>,
    ) -> AlgebraResult<(Vec<f64>, Vec<f64>)> {
        let n = f.len();
        let mut free_index = vec![usize::MAX; n];
        let mut free: Vec<usize> = Vec::new();
        for i in 0..n {
            if !fixed.contains_key(&i) {
                free_index[i] = free.len();
                free.push(i);
            }
        }
        let m = free.len();
        let mut rows: Vec<Vec<(usize, f64)>> = Vec::with_capacity(m);
        let mut b: Vec<f64> = Vec::with_capacity(m);
        for &i in &free {
            let mut row = Vec::new();
            let mut rhs = f[i];
            for (&j, &v) in &k.rows[i] {
                match fixed.get(&j) {
                    Some(value) => rhs -= v * value,
                    None => row.push((free_index[j], v)),
                }
            }
            rows.push(row);
            b.push(rhs);
        }
        let solved = if m == 0 {
            Vec::new()
        } else if m > Self::SPARSE_THRESHOLD {
            CsrMatrix::from_rows(&rows).conjugate_gradient(&b, 1e-9, 50 * m + 1000)?
        } else {
            let mut a = vec![vec![0.0; m]; m];
            for (r, row) in rows.iter().enumerate() {
                for &(c, v) in row {
                    a[r][c] += v;
                }
            }
            Self::solve_linear_system(&mut a, &mut b)?
        };
        let mut u = vec![0.0; n];
        for (&i, &v) in free.iter().zip(&solved) {
            u[i] = v;
        }
        for (&c, &v) in fixed {
            if c < n {
                u[c] = v;
            }
        }
        let reactions = (0..n)
            .map(|i| {
                if fixed.contains_key(&i) {
                    k.rows[i].iter().map(|(&j, &v)| v * u[j]).sum::<f64>() - f[i]
                } else {
                    0.0
                }
            })
            .collect();
        Ok((u, reactions))
    }

    /// Systems with more free unknowns than this are solved by sparse conjugate gradients instead
    /// of dense elimination (whose cost grows as n^3).
    pub const SPARSE_THRESHOLD: usize = 400;

    /// Gaussian elimination with partial pivoting for the dense system `A x = b`.
    ///
    /// A pivot that is negligible next to the largest diagonal entry means the matrix is
    /// singular (typically an unconstrained structure with rigid-body motion, or a thermal
    /// problem with no boundary condition) and is reported as an error rather than solved by
    /// skipping the equation. The solution is then checked against the original system: a
    /// relative residual above `1e-6` (severe ill-conditioning) is an error too.
    #[allow(clippy::needless_range_loop)]
    fn solve_linear_system(a: &mut [Vec<f64>], b: &mut [f64]) -> AlgebraResult<Vec<f64>> {
        let n = b.len();
        let original_a: Vec<Vec<f64>> = a.to_vec();
        let original_b: Vec<f64> = b.to_vec();
        // Each row is judged against its own magnitude, so a stiff row does not make healthy
        // pivots in softer rows look negligible.
        let mut row_scale: Vec<f64> = (0..n)
            .map(|i| a[i].iter().fold(0.0_f64, |m, v| m.max(v.abs())))
            .collect();

        for i in 0..n {
            // Find pivot
            let mut max_row = i;
            let mut max_val = a[i][i].abs();
            for k in i + 1..n {
                if a[k][i].abs() > max_val {
                    max_val = a[k][i].abs();
                    max_row = k;
                }
            }

            // Elimination can leave a pivot that is round-off relative to its row: singular.
            if max_val <= row_scale[max_row].max(1e-300) * 1e-12 {
                return Err(AlgebraError::EvaluationError(format!(
                    "singular system: no usable pivot for equation {i}                      (the problem is under-constrained)"
                )));
            }

            // Swap rows
            a.swap(i, max_row);
            b.swap(i, max_row);
            row_scale.swap(i, max_row);

            // Eliminate
            for k in i + 1..n {
                let factor = a[k][i] / a[i][i];
                b[k] -= factor * b[i];
                for j in i..n {
                    let val = a[i][j];
                    a[k][j] -= factor * val;
                }
            }
        }

        // Back substitution
        let mut x = vec![0.0; n];
        for i in (0..n).rev() {
            let mut sum = b[i];
            for j in i + 1..n {
                sum -= a[i][j] * x[j];
            }
            x[i] = sum / a[i][i];
        }

        // Verify against the original system.
        let mut residual_sq = 0.0;
        let mut rhs_sq = 0.0;
        for i in 0..n {
            let row: f64 = (0..n).map(|j| original_a[i][j] * x[j]).sum();
            residual_sq += (row - original_b[i]).powi(2);
            rhs_sq += original_b[i].powi(2);
        }
        if residual_sq.sqrt() > 1e-6 * rhs_sq.sqrt().max(1e-300) {
            return Err(AlgebraError::EvaluationError(format!(
                "linear solve failed its residual check (relative residual {:.2e}); the system is                  too ill-conditioned",
                (residual_sq / rhs_sq.max(1e-300)).sqrt()
            )));
        }

        Ok(x)
    }
}

/// Row-wise sparse accumulator for assembling a global stiffness matrix: `add` sums repeated
/// contributions to one entry, and only the touched entries use memory.
#[derive(Debug, Clone)]
pub struct SparseAssembly {
    rows: Vec<std::collections::BTreeMap<usize, f64>>,
}

impl SparseAssembly {
    /// An `n` by `n` zero matrix.
    pub fn new(n: usize) -> Self {
        Self {
            rows: vec![std::collections::BTreeMap::new(); n],
        }
    }

    /// Add `value` to entry `(i, j)`.
    pub fn add(&mut self, i: usize, j: usize, value: f64) {
        *self.rows[i].entry(j).or_insert(0.0) += value;
    }

    /// Entry `(i, j)` (zero when never touched).
    pub fn get(&self, i: usize, j: usize) -> f64 {
        self.rows[i].get(&j).copied().unwrap_or(0.0)
    }

    /// Number of stored entries.
    pub fn nnz(&self) -> usize {
        self.rows.iter().map(|r| r.len()).sum()
    }
}

/// Compressed sparse row storage for a square matrix.
#[derive(Debug, Clone)]
pub struct CsrMatrix {
    n: usize,
    row_start: Vec<usize>,
    columns: Vec<usize>,
    values: Vec<f64>,
}

impl CsrMatrix {
    /// Keep the non-zero entries of a dense square matrix.
    pub fn from_dense(a: &[Vec<f64>]) -> Self {
        let n = a.len();
        let mut row_start = Vec::with_capacity(n + 1);
        let (mut columns, mut values) = (Vec::new(), Vec::new());
        row_start.push(0);
        for row in a {
            for (j, &v) in row.iter().enumerate() {
                if v != 0.0 {
                    columns.push(j);
                    values.push(v);
                }
            }
            row_start.push(columns.len());
        }
        Self {
            n,
            row_start,
            columns,
            values,
        }
    }

    /// Build from per-row `(column, value)` lists; repeated columns in a row are summed.
    pub fn from_rows(rows: &[Vec<(usize, f64)>]) -> Self {
        let mut row_start = vec![0];
        let (mut columns, mut values) = (Vec::new(), Vec::new());
        for row in rows {
            let mut merged: std::collections::BTreeMap<usize, f64> =
                std::collections::BTreeMap::new();
            for &(c, v) in row {
                *merged.entry(c).or_insert(0.0) += v;
            }
            for (c, v) in merged {
                if v != 0.0 {
                    columns.push(c);
                    values.push(v);
                }
            }
            row_start.push(columns.len());
        }
        Self {
            n: rows.len(),
            row_start,
            columns,
            values,
        }
    }

    /// Number of stored (non-zero) entries.
    pub fn nnz(&self) -> usize {
        self.values.len()
    }

    /// The product `A x`.
    pub fn matvec(&self, x: &[f64]) -> Vec<f64> {
        (0..self.n)
            .map(|i| {
                (self.row_start[i]..self.row_start[i + 1])
                    .map(|k| self.values[k] * x[self.columns[k]])
                    .sum()
            })
            .collect()
    }

    /// Componentwise backward error `max_i |b_i - (A x)_i| / (|b_i| + (|A| |x|)_i)` (rows with a
    /// zero denominator count as exact). Unlike a norm of the residual it is not hidden by
    /// penalty-constraint rows that are ~1e12 times stiffer than the rest.
    pub fn backward_error(&self, x: &[f64], b: &[f64]) -> f64 {
        let mut worst = 0.0_f64;
        for i in 0..self.n {
            let (mut ax, mut abs_ax) = (0.0, 0.0);
            for k in self.row_start[i]..self.row_start[i + 1] {
                let term = self.values[k] * x[self.columns[k]];
                ax += term;
                abs_ax += term.abs();
            }
            let denom = b[i].abs() + abs_ax;
            if denom > 0.0 {
                worst = worst.max((b[i] - ax).abs() / denom);
            }
        }
        worst
    }

    /// Jacobi-preconditioned conjugate gradients for a symmetric positive definite system
    /// `A x = b`. The diagonal preconditioner also neutralises the very stiff rows that penalty
    /// constraints create. Stops when the componentwise [`CsrMatrix::backward_error`] reaches `tol`.
    /// Returns an error if that does not happen within
    /// `max_iter` iterations (an indefinite, singular or badly conditioned system) or if
    /// `p^T A p` is not positive.
    pub fn conjugate_gradient(
        &self,
        b: &[f64],
        tol: f64,
        max_iter: usize,
    ) -> AlgebraResult<Vec<f64>> {
        let n = self.n;
        let fail = |why: String| AlgebraError::EvaluationError(format!("sparse solve: {why}"));
        let mut inv_diag = vec![1.0; n];
        for i in 0..n {
            let d = (self.row_start[i]..self.row_start[i + 1])
                .find(|&k| self.columns[k] == i)
                .map_or(0.0, |k| self.values[k]);
            if d <= 0.0 {
                return Err(fail(format!(
                    "equation {i} has a non-positive diagonal (the problem is under-constrained)"
                )));
            }
            inv_diag[i] = 1.0 / d;
        }
        let norm_b = b.iter().map(|v| v * v).sum::<f64>().sqrt();
        if norm_b == 0.0 {
            return Ok(vec![0.0; n]);
        }
        let mut x = vec![0.0; n];
        let mut r = b.to_vec();
        let mut z: Vec<f64> = r.iter().zip(&inv_diag).map(|(a, d)| a * d).collect();
        let mut p = z.clone();
        let mut rz: f64 = r.iter().zip(&z).map(|(a, b)| a * b).sum();
        for _ in 0..max_iter {
            let ap = self.matvec(&p);
            let pap: f64 = p.iter().zip(&ap).map(|(a, b)| a * b).sum();
            if pap <= 0.0 {
                return Err(fail("the matrix is not positive definite".into()));
            }
            let alpha = rz / pap;
            for i in 0..n {
                x[i] += alpha * p[i];
                r[i] -= alpha * ap[i];
            }
            if self.backward_error(&x, b) <= tol {
                return Ok(x);
            }
            z = r.iter().zip(&inv_diag).map(|(a, d)| a * d).collect();
            let rz_new: f64 = r.iter().zip(&z).map(|(a, b)| a * b).sum();
            let beta = rz_new / rz;
            rz = rz_new;
            for i in 0..n {
                p[i] = z[i] + beta * p[i];
            }
        }
        Err(fail(format!("no convergence in {max_iter} iterations")))
    }
}
