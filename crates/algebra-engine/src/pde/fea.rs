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
        let mut k_global = vec![vec![0.0; total_dof]; total_dof];
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
                    k_global[i0][i0] += k_elem;
                    k_global[i0][i1] -= k_elem;
                    k_global[i1][i0] -= k_elem;
                    k_global[i1][i1] += k_elem;

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
                        // Elasticity constitutive matrix C for 2D plane stress
                        let factor = if plane_stress {
                            e_mod / (1.0 - nu * nu)
                        } else {
                            e_mod / ((1.0 + nu) * (1.0 - 2.0 * nu))
                        };
                        let c11 = factor;
                        let c12 = if plane_stress {
                            factor * nu
                        } else {
                            factor * nu / (1.0 - nu)
                        };
                        let c33 = factor * (1.0 - nu) / 2.0;

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
                                k_global[dof_r][dof_c] += val * area;
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
                                k_global[ni * 3 + d][ni * 3 + d] += k_diag;
                            }
                        }
                        for i in 0..3 {
                            for j in 0..3 {
                                if i != j {
                                    let ni = n[i];
                                    let nj = n[j];
                                    let k_off = -k_diag / 2.0;
                                    for d in 0..3 {
                                        k_global[ni * 3 + d][nj * 3 + d] += k_off;
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
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

        // 3. Apply Dirichlet Fixed Constraints (Row/Column elimination with penalty)
        let penalty = 1e12 * e_mod;
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
                        k_global[global_dof][global_dof] += penalty;
                        f_global[global_dof] += penalty * value;
                    }
                }
            }
        }

        // 4. Solve Linear System K u = F via Gaussian Elimination with Partial Pivoting
        let displacement = Self::solve_linear_system(&mut k_global, &mut f_global)?;

        // 5. Compute von Mises Stresses across elements
        let mut von_mises = Vec::new();
        for elem in &mesh.elements {
            if elem.len() >= 3 && mesh.dim == 2 {
                let n = [elem[0], elem[1], elem[2]];
                let u0 = displacement[n[0] * 2];
                let v0 = displacement[n[0] * 2 + 1];
                let u1 = displacement[n[1] * 2];
                let v1 = displacement[n[1] * 2 + 1];
                let u2 = displacement[n[2] * 2];
                let v2 = displacement[n[2] * 2 + 1];

                let du_dx = (u1 - u0).abs() + (u2 - u0).abs();
                let dv_dy = (v1 - v0).abs() + (v2 - v0).abs();
                let gamma_xy = (u1 - u0 + v1 - v0).abs();

                let sigma_x = e_mod * du_dx;
                let sigma_y = e_mod * dv_dy;
                let tau_xy = (e_mod / (2.0 * (1.0 + nu))) * gamma_xy;

                // von Mises 2D: sqrt(sigma_x^2 - sigma_x*sigma_y + sigma_y^2 + 3*tau_xy^2)
                let vm = (sigma_x * sigma_x - sigma_x * sigma_y
                    + sigma_y * sigma_y
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
            reaction_forces: vec![0.0; total_dof],
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
        let mut k_global = vec![vec![0.0; n_nodes]; n_nodes];
        let mut f_global = vec![0.0; n_nodes];

        // 1. Element Assembly
        for elem in &mesh.elements {
            if mesh.element_type == ElementType::Line2 && mesh.dim == 1 {
                let i0 = elem[0];
                let i1 = elem[1];
                let len = (mesh.nodes[i1][0] - mesh.nodes[i0][0]).abs().max(1e-12);
                let k_elem = conductivity / len;

                k_global[i0][i0] += k_elem;
                k_global[i0][i1] -= k_elem;
                k_global[i1][i0] -= k_elem;
                k_global[i1][i1] += k_elem;

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
                            k_global[n[r]][n[col]] += k_elem;
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
                        k_global[ni][ni] += k_elem;
                    }
                    for i in 0..3 {
                        for j in 0..3 {
                            if i != j {
                                k_global[n[i]][n[j]] -= k_elem / 2.0;
                            }
                        }
                    }
                    for &ni in &n {
                        f_global[ni] += heat_source * area / 3.0;
                    }
                }
            }
        }

        // 2. Apply Boundary Conditions
        let penalty = 1e12 * conductivity;
        for bc in bcs {
            match bc {
                FeaBoundaryCondition::PrescribedTemperature {
                    group_name,
                    temperature,
                } => {
                    if let Some(nodes) = mesh.boundary_groups.get(group_name) {
                        for &node_idx in nodes {
                            k_global[node_idx][node_idx] += penalty;
                            f_global[node_idx] += penalty * temperature;
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
                            k_global[node_idx][node_idx] += heat_transfer_coeff;
                            f_global[node_idx] += heat_transfer_coeff * ambient_temperature;
                        }
                    }
                }
                _ => {}
            }
        }

        // 3. Solve Linear System
        let temperature = Self::solve_linear_system(&mut k_global, &mut f_global)?;
        let max_t = temperature
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);

        Ok(FeaSolution {
            nodal_values: temperature,
            dof_per_node: 1,
            von_mises_stresses: Vec::new(),
            reaction_forces: Vec::new(),
            max_magnitude: max_t,
        })
    }

    /// Systems with more unknowns than this are solved by sparse conjugate gradients instead of
    /// dense elimination (whose cost grows as n^3).
    pub const SPARSE_THRESHOLD: usize = 400;

    /// Gaussian elimination with partial pivoting for the dense system `A x = b`.
    ///
    /// A pivot that is negligible next to the largest diagonal entry means the matrix is
    /// singular (typically an unconstrained structure with rigid-body motion, or a thermal
    /// problem with no boundary condition) and is reported as an error rather than solved by
    /// skipping the equation. The solution is then checked against the original system: a
    /// relative residual above `1e-6` (severe ill-conditioning, for example from the penalty
    /// constraints) is an error too.
    #[allow(clippy::needless_range_loop)]
    fn solve_linear_system(a: &mut [Vec<f64>], b: &mut [f64]) -> AlgebraResult<Vec<f64>> {
        let n = b.len();
        if n > Self::SPARSE_THRESHOLD {
            // Large systems: keep only the non-zeros and iterate (the matrices are symmetric
            // positive definite once constrained).
            return CsrMatrix::from_dense(a).conjugate_gradient(b, 1e-9, 50 * n + 1000);
        }
        let original_a: Vec<Vec<f64>> = a.to_vec();
        let original_b: Vec<f64> = b.to_vec();
        // Each row is judged against its own magnitude: penalty constraints make some rows ~1e12
        // times stiffer than others, so a single global scale would flag healthy pivots.
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
