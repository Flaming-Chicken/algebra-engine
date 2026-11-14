//! # `urae_notebook::examples`
//!
//! In-App Interactive Example Notebook & Tutorial Registry.
//!
//! Provides 12+ pre-built, production-ready mathematical notebooks spanning all
//! domains of the Universal Rust Algebra Engine (URAE).

use serde::{Deserialize, Serialize};

/// High-level mathematical category for example notebooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExampleCategory {
    CalculusAndOde,
    QuantumAndPhysics,
    CadAndSimulation,
    GeometryAndTopology,
    ExceptionalAndAbstract,
    DModulesAndFormal,
    MultiphysicsAndAi,
    EngineeringAndControl,
    DiscreteAndCrypto,
    DataAndStatistics,
}

impl ExampleCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::CalculusAndOde => "Calculus & Differential Equations",
            Self::QuantumAndPhysics => "Quantum Field Theory & Physics",
            Self::CadAndSimulation => "Parametric CAD & IGA Simulation",
            Self::GeometryAndTopology => "Non-Euclidean & Topology",
            Self::ExceptionalAndAbstract => "Exceptional Lie & Octonions",
            Self::DModulesAndFormal => "Holonomic D-Modules & Proofs",
            Self::MultiphysicsAndAi => "Multiphysics FEM & Neural ODEs",
            Self::EngineeringAndControl => "Engineering, Control & Thermodynamics",
            Self::DiscreteAndCrypto => "Discrete Mathematics & Cryptography",
            Self::DataAndStatistics => "Statistics, Probability & Analysis",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::CalculusAndOde => "∫",
            Self::QuantumAndPhysics => "Ψ",
            Self::CadAndSimulation => "⚙",
            Self::GeometryAndTopology => "🌐",
            Self::ExceptionalAndAbstract => "𝔼₈",
            Self::DModulesAndFormal => "📜",
            Self::MultiphysicsAndAi => "🧠",
            Self::EngineeringAndControl => "🎛",
            Self::DiscreteAndCrypto => "🔐",
            Self::DataAndStatistics => "📊",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::CalculusAndOde,
            Self::QuantumAndPhysics,
            Self::CadAndSimulation,
            Self::GeometryAndTopology,
            Self::ExceptionalAndAbstract,
            Self::DModulesAndFormal,
            Self::MultiphysicsAndAi,
            Self::EngineeringAndControl,
            Self::DiscreteAndCrypto,
            Self::DataAndStatistics,
        ]
    }
}

/// Metadata and source content for an embedded example notebook.
#[derive(Debug, Clone)]
pub struct ExampleNotebook {
    pub id: &'static str,
    pub title: &'static str,
    pub category: ExampleCategory,
    pub description: &'static str,
    pub latex_formula: &'static str,
    pub content: &'static str,
    pub tags: &'static [&'static str],
}

/// Master Registry of all embedded URAE example notebooks.
pub struct ExampleRegistry;

impl ExampleRegistry {
    /// Return all curated example notebooks.
    pub fn all_examples() -> &'static [ExampleNotebook] {
        &EXAMPLES
    }

    /// Find an example notebook by its unique identifier.
    pub fn find_by_id(id: &str) -> Option<&'static ExampleNotebook> {
        EXAMPLES.iter().find(|ex| ex.id == id)
    }

    /// Filter example notebooks by category.
    pub fn filter_by_category(cat: ExampleCategory) -> Vec<&'static ExampleNotebook> {
        EXAMPLES.iter().filter(|ex| ex.category == cat).collect()
    }

    /// Search example notebooks by text query across title, description, and tags.
    pub fn search(query: &str) -> Vec<&'static ExampleNotebook> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return EXAMPLES.iter().collect();
        }
        EXAMPLES
            .iter()
            .filter(|ex| {
                ex.title.to_lowercase().contains(&q)
                    || ex.description.to_lowercase().contains(&q)
                    || ex.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect()
    }
}

const EXAMPLES: [ExampleNotebook; 32] = [
    // 01: Calculus & ODEs
    ExampleNotebook {
        id: "01_calculus_and_differential_equations",
        title: "Advanced Symbolic Calculus & ODEs",
        category: ExampleCategory::CalculusAndOde,
        description: "Symbolic differentiation, complete transcendental Risch integration, algebraic equation solving, L'Hôpital limits, and damped harmonic oscillator ODEs.",
        latex_formula: r#"y''(t) + 2\beta y'(t) + \omega_0^2 y(t) = 0 \implies r^2 + 2\beta r + \omega_0^2 = 0"#,
        content: "# URAE Mathematics Notepad: Advanced Calculus, Special Functions & ODEs\n# Demonstrates symbolic differentiation, Risch integration, algebraic equation solving, series expansions, and limits.\n\na: Parameter = 2.50 [m]\nb: Parameter = 1.20 [s^-1]\nomega: Parameter = 3.14159 [rad/s]\nx: Variable\nt: Variable\n\n{ x in Reals | -10.0 <= x <= 10.0 }\n{ t in Reals | 0.0 <= t <= 100.0 }\n\n# 1. Assembling and Differentiating Functions\nf(x) = a * exp(-b * x) * cos(omega * x)\ndiff(f(x), x)\ndiff(diff(f(x), x), x)\n\n# 2. Symbolic Integration\nintegrate(x^3 * exp(x), x)\nintegrate(sin(x)^2, x)\n\n# 3. Assembling and Solving Algebraic Equations\npoly_eq = x^3 - 6.0 * x^2 + 11.0 * x - 6.0\nsolve x^3 - 6.0 * x^2 + 11.0 * x - 6.0 = 0, x\n\n# Quadratic formula assembly: a*x^2 + b*x + c = 0\nsolve 2.0 * x^2 - 8.0 * x + 6.0 = 0, x\n\n# 4. Assembling and Solving ODE Characteristic Polynomials\n# y''(t) + 2*b*y'(t) + omega^2*y(t) = 0\nchar_ode = r^2 + 2.0 * b * r + omega^2\nsolve r^2 + 2.0 * b * r + omega^2 = 0, r\n\n# 5. Taylor Series & Limits\nseries(sin(x) / x, x, 0, 8)\nlimit((1.0 - cos(x)) / x^2, x, 0)\n",
        tags: &[
            "calculus",
            "risch",
            "integral",
            "derivative",
            "ode",
            "limits",
            "series",
            "solve",
        ],
    },
    // 02: QFT & Physics
    ExampleNotebook {
        id: "02_quantum_field_theory_and_clifford",
        title: "Quantum Field Theory & Spinor Helicity",
        category: ExampleCategory::QuantumAndPhysics,
        description: "Dirac gamma traces, space-time Clifford algebra Cl(1, 3), massless Weyl spinors, and Parke-Taylor MHV tree amplitudes.",
        latex_formula: "\\mathcal{A}_n^{\\mathrm{MHV}}(1^-, 2^-, 3^+, \\dots, n^+) = \\frac{\\langle 1 2 \\rangle^4}{\\prod_{k=1}^n \\langle k, k+1 \\rangle}",
        content: "# URAE Mathematics Notepad: Quantum Field Theory & Space-Time Clifford Algebra\n\ndirac_trace([0, 1, 0, 1])\ndirac_trace([0, 1, 2, 3])\n\nspinor_bracket_angle(1, 2)\nspinor_bracket_square(2, 1)\n\nparke_taylor_mhv(4, [1, 2])\npassarino_veltman_b0(100.0, 0.0, 0.0)\npassarino_veltman_b1(100.0, 0.0, 0.0)\n",
        tags: &[
            "qft",
            "dirac",
            "gamma",
            "spinor",
            "helicity",
            "parke-taylor",
            "amplitudes",
        ],
    },
    // 03: Parametric CAD & IGA
    ExampleNotebook {
        id: "03_parametric_cad_and_isogeometric_analysis",
        title: "Parametric CAD & Isogeometric Analysis",
        category: ExampleCategory::CadAndSimulation,
        description: "Precision involute spur gears, ISO metric threaded bolts, NACA airfoils, Cox-de Boor B-splines, and NURBS IGA stiffness quadrature.",
        latex_formula: "K_{ab} = \\int_\\Omega \\nabla R_a \\cdot \\nabla R_b \\, d\\Omega",
        content: "# URAE Mathematics Notepad: Parametric CAD & Isogeometric Analysis (IGA)\n\ngear!(module = 2.0, teeth = 18, width = 10.0)\nscrew!(dia = 10.0, pitch = 1.5, length = 25.0)\nairfoil!(code = \"2412\", chord = 10.0, span = 25.0)\n\ncox_de_boor_basis!(0.5, 0, 2, [0.0, 0.0, 0.0, 1.0, 1.0, 1.0])\n",
        tags: &[
            "cad", "gear", "screw", "airfoil", "nurbs", "iga", "b-spline",
        ],
    },
    // 04: Non-Euclidean Geometry
    ExampleNotebook {
        id: "04_non_euclidean_geometry_and_tessellations",
        title: "Non-Euclidean & Hyperbolic Tessellations",
        category: ExampleCategory::GeometryAndTopology,
        description: "Poincaré Disk model, Upper Half-Plane, Cayley conformal transform, Gauss-Bonnet area defects, and Schläfli {7, 3} tilings.",
        latex_formula: "ds^2 = \\frac{4(dx^2 + dy^2)}{(1 - x^2 - y^2)^2}, \\quad \\operatorname{Area}(\\Delta) = \\pi - (\\alpha + \\beta + \\gamma)",
        content: "# URAE Mathematics Notepad: Non-Euclidean Geometry & Hyperbolic Tessellations\n\np1 = poincare_point(0.2, 0.3)\np2 = poincare_point(-0.4, 0.5)\n\nhyperbolic_dist(p1, p2)\nhyperbolic_triangle_area(0.5, 0.6, 0.7)\nspherical_triangle_area(1.2, 1.3, 1.4)\n",
        tags: &[
            "poincare",
            "hyperbolic",
            "geometry",
            "tessellation",
            "schlafli",
            "gauss-bonnet",
        ],
    },
    // 05: Exceptional Lie & Octonions
    ExampleNotebook {
        id: "05_exceptional_lie_groups_and_octonions",
        title: "Exceptional Lie Groups & Octonions",
        category: ExampleCategory::ExceptionalAndAbstract,
        description: "8D non-associative Octonions O, Fano plane multiplication, Albert exceptional Jordan algebra H_3(O), and E8 root lattice.",
        latex_formula: "[x, y, z] = (xy)z - x(yz), \\quad \\det(X) = \\xi_1 \\xi_2 \\xi_3 - \\sum \\xi_i N(x_i) + 2 \\operatorname{Re}(x_1 x_2 x_3)",
        content: "# URAE Mathematics Notepad: Exceptional Lie Groups & Octonions\n\nx = octonion!(1, 0, 1, 0, 0, 0, 0, 0)\ny = octonion!(0, 1, 0, 1, 0, 0, 0, 0)\nz = octonion!(0, 0, 1, 0, 1, 0, 0, 0)\n\noctonion_associator!(x, y, z)\noctonion_associator!(x, x, y)\n",
        tags: &[
            "octonion",
            "albert",
            "jordan",
            "e8",
            "exceptional",
            "lie",
            "fano",
        ],
    },
    // 06: Holonomic D-Modules
    ExampleNotebook {
        id: "06_holonomic_dmodules_and_zeilberger",
        title: "Holonomic D-Modules & Zeilberger Proofs",
        category: ExampleCategory::DModulesAndFormal,
        description: "Non-commutative Weyl algebra A_n, Leibniz normal form [d, x] = 1, and automated Zeilberger creative telescoping proofs for binomial sums.",
        latex_formula: "L(n, S_n) F(n, k) = (S_k - 1) G(n, k), \\quad [\\partial_i, x_j] = \\delta_{ij}\\mathbf{I}",
        content: "# URAE Mathematics Notepad: Holonomic D-Modules & Zeilberger Creative Telescoping\n\nx = weyl_x!(1, 0, 1)\nd = weyl_d!(1, 0, 1)\n\nweyl_commute!(d, x)\nzeilberger_binomial_proof!()\nalmkvist_zeilberger_gaussian!()\n",
        tags: &[
            "weyl",
            "d-modules",
            "zeilberger",
            "creative-telescoping",
            "binomial",
            "hypergeometric",
        ],
    },
    // 07: Multiphysics FEM & Neural ODEs
    ExampleNotebook {
        id: "07_multiphysics_fem_and_neural_odes",
        title: "Multiphysics Heat Conduction & Analytical Poisson Solvers",
        category: ExampleCategory::MultiphysicsAndAi,
        description: "1D/2D Poisson heat conduction equation: assembling the governing PDE, integrating twice, solving for boundary condition integration constants, temperature distribution derivation, and peak flux analysis.",
        latex_formula: r#"-k \frac{d^2 T}{dx^2} = Q \implies T(x) = T_{\text{left}} + C_1 x - \frac{Q}{2k}x^2"#,
        content: "# URAE Mathematics Notepad: Multiphysics Poisson Heat Equation & Conduction\n# Demonstrates assembling the heat conduction PDE, integrating twice, solving boundary value systems, and finding peak temperatures.\n\n# 1. Thermal & Geometric Parameters\nk_thermal: Parameter = 45.0 [W/(m*K)]\nQ_source: Parameter = 50000.0 [W/m^3]\nL_rod: Parameter = 0.20 [m]\nT_left: Parameter = 293.15 [K]\nT_right: Parameter = 350.0 [K]\n\nx: Variable\n{ x in Reals | 0.0 <= x <= L_rod }\n\n# 2. Assembling the Poisson Heat Conduction Equation\n# Governing PDE: -k * d^2 T / dx^2 = Q\n# Integrating twice: T(x) = -(Q / (2*k)) * x^2 + C1 * x + C2\n# Boundary condition 1: T(0) = T_left => C2 = T_left\n# Boundary condition 2: T(L) = T_right => C1 * L - (Q / (2*k)) * L^2 + T_left = T_right\n\n# Solving for integration constant C1:\nsolve C1 * L_rod - (Q_source / (2.0 * k_thermal)) * L_rod^2 + T_left = T_right, C1\n\n# Exact analytical value for C1:\nC1_val = (T_right - T_left) / L_rod + (Q_source * L_rod) / (2.0 * k_thermal)\n\n# 3. Assembled Exact Temperature Profile T(x)\nT_profile(x) = T_left + C1_val * x - (Q_source / (2.0 * k_thermal)) * x^2\n\n# 4. Finding Peak Temperature Location dT/dx = 0\nsolve C1_val - (Q_source / k_thermal) * x = 0, x\nx_peak = (C1_val * k_thermal) / Q_source\nT_max = T_profile(x_peak)\n\n# 5. Heat Flux at Boundaries: q = -k * dT/dx\nq_left = -k_thermal * C1_val\nq_right = -k_thermal * (C1_val - (Q_source * L_rod) / k_thermal)\n",
        tags: &[
            "fem",
            "pde",
            "heat",
            "poisson",
            "conduction",
            "boundary-value",
            "temperature",
            "flux",
        ],
    },
    // 08: Topological Data Analysis & Discrete Hodge
    ExampleNotebook {
        id: "08_topological_data_analysis_and_discrete_hodge",
        title: "Topological Data Analysis & Discrete Hodge",
        category: ExampleCategory::GeometryAndTopology,
        description: "Vietoris-Rips simplicial filtration over point clouds, GF(2) boundary matrix reduction, persistence barcodes, and discrete Hodge Laplacian.",
        latex_formula: "\\Delta_0 = d_0^* d_0, \\quad \\beta_k(\\epsilon) = \\operatorname{rank}(H_k(K_\\epsilon))",
        content: "# URAE Mathematics Notepad: Topological Data Analysis & Discrete Hodge\n\nvietoris_rips!(max_dim = 2, threshold = 1.5)\npersistence_diagram!()\ndiscrete_laplacian_0!()\n",
        tags: &[
            "tda",
            "vietoris-rips",
            "homology",
            "barcodes",
            "betti",
            "hodge",
            "laplacian",
        ],
    },
    // 09: SMT-LIB2 Automated Verification & Proofs
    ExampleNotebook {
        id: "09_smt_lib2_formal_verification",
        title: "SMT-LIB2 Verification & Formal Proof Certificates",
        category: ExampleCategory::DModulesAndFormal,
        description: "Automated theorem proving format serializer for Z3/CVC5 across QF_NRA / QF_LRA and constructive proof certificate generator for Lean 4 & Coq.",
        latex_formula: "\\vdash \\forall x \\in \\mathbb{R}, \\; x^2 \\ge 0 \\quad [\\text{Lean 4: } \\mathtt{positivity}]",
        content: "# URAE Mathematics Notepad: SMT-LIB2 Automated Verification & Proof Certificates\n\nsmt_problem!(logic = \"QF_NRA\")\nsmt_assert!(\"x^2 + y^2 >= 0\")\nproof_certificate!(target = \"Lean4\", theorem = \"x_sq_nonneg\")\n",
        tags: &[
            "smt",
            "z3",
            "formal-methods",
            "lean4",
            "coq",
            "proof-assistant",
            "verification",
        ],
    },
    // 10: Geometric Deep Learning & SE(3) Equivariance
    ExampleNotebook {
        id: "10_geometric_deep_learning_se3",
        title: "Geometric Deep Learning & SE(3) Equivariance",
        category: ExampleCategory::MultiphysicsAndAi,
        description: "Real/complex spherical harmonics Y_lm, Clebsch-Gordan tensor product series, SE(3) steerable convolutions, and Clifford Cl(3, 0) neural layers.",
        latex_formula: "\\mathcal{D}^{(\\ell_1)} \\otimes \\mathcal{D}^{(\\ell_2)} = \\bigoplus_{|\\ell_1 - \\ell_2|}^{\\ell_1 + \\ell_2} \\mathcal{D}^{(\\ell)}",
        content: "# URAE Mathematics Notepad: Geometric Deep Learning & SE(3) Equivariance\n\nspherical_harmonic!(l = 2, m = 1, theta = 0.5, phi = 1.2)\nclebsch_gordan!(l1 = 1, m1 = 1, l2 = 1, m2 = -1, l = 0, m = 0)\nclifford_layer_forward!(in_dim = 8, out_dim = 8)\n",
        tags: &[
            "geometric-dl",
            "spherical-harmonics",
            "clebsch-gordan",
            "se3",
            "equivariance",
            "clifford-neural",
        ],
    },
    // 11: Affine Kac-Moody & Virasoro CFT
    ExampleNotebook {
        id: "11_affine_kac_moody_and_virasoro_cft",
        title: "Affine Kac-Moody & Virasoro Conformal Field Theory",
        category: ExampleCategory::QuantumAndPhysics,
        description: "Virasoro Lie brackets, central charge c, minimal model Kac tables h_{r, s}, affine loop current commutators, and OPE Laurent poles.",
        latex_formula: "[L_m, L_n] = (m - n) L_{m+n} + \\frac{c}{12}(m^3 - m) \\delta_{m+n, 0}",
        content: "# URAE Mathematics Notepad: Affine Kac-Moody & Virasoro CFT\n\nvirasoro_bracket!(2, -2, 0.5)\nkac_moody_bracket!(1, 2, 2, -2, 1.0)\nsugawara_central_charge!(dim_g = 3, k = 2.0, dual_coxeter = 2.0)\nkac_conformal_weight!(1, 2, 3)\n",
        tags: &[
            "cft",
            "virasoro",
            "kac-moody",
            "sugawara",
            "central-charge",
            "ope",
            "minimal-models",
        ],
    },
    // 12: p-Adic Hodge Theory & Fontaine Period Rings
    ExampleNotebook {
        id: "12_padic_hodge_theory_and_fontaine",
        title: "p-Adic Hodge Theory & Fontaine Period Rings",
        category: ExampleCategory::ExceptionalAndAbstract,
        description: "Fontaine filtered (Phi, N)-modules with N*Phi = p*Phi*N, local Galois representation admissibility (crystalline, semistable), and Tate twists.",
        latex_formula: "N \\Phi = p \\Phi N, \\quad D_{\\mathrm{cris}}(V) = (B_{\\mathrm{cris}} \\otimes_{\\mathbb{Q}_p} V)^{G_K}",
        content: "# URAE Mathematics Notepad: p-Adic Hodge Theory & Fontaine Period Rings\n\nfontaine_module!(dim = 2, p = 7)\ntate_twist_weights!([0, 2], 2)\ntate_twist_frob!([7.0, 49.0], 7, 1)\n",
        tags: &[
            "padic",
            "hodge",
            "fontaine",
            "frobenius",
            "galois",
            "tate-twist",
            "period-rings",
        ],
    },
    // 13: Canonical Algebraic Forms & E-Graph Simplification
    ExampleNotebook {
        id: "13_algebraic_forms_and_simplification",
        title: "Canonical Algebraic Forms & Simplification",
        category: ExampleCategory::DModulesAndFormal,
        description: "Exploration of multivariate polynomials, Horner nested forms, factored representations, and non-greedy equality saturation.",
        latex_formula: "P(x) = a_0 + x(a_1 + x(a_2 + \\dots + x a_n)), \\quad x \\oplus y = \\max(x, y)",
        content: "# URAE Mathematics Notepad: Canonical Algebraic Forms\n\npoly1 = expand((x + 2)^4 * (x - 3))\nhorner_form(poly1, x)\nfactor(poly1)\n\n# Tropical Semiring Max-Plus Form\ntropical_add(4, 7)\ntropical_mul(4, 7)\n",
        tags: &[
            "algebra",
            "polynomial",
            "horner",
            "factor",
            "tropical",
            "e-graph",
            "simplification",
        ],
    },
    // 14: Stochastic Itô Calculus
    ExampleNotebook {
        id: "14_stochastic_ito_calculus",
        title: "Stochastic Itô Calculus & SDEs",
        category: ExampleCategory::CalculusAndOde,
        description: "Stochastic differential equations (SDEs), quadratic variation dW_t² = dt, Itô's Lemma, and Black-Scholes PDE derivation.",
        latex_formula: "df(t, X_t) = \\left(\\frac{\\partial f}{\\partial t} + \\mu \\frac{\\partial f}{\\partial x} + \\frac{1}{2}\\sigma^2 \\frac{\\partial^2 f}{\\partial x^2}\\right)dt + \\sigma \\frac{\\partial f}{\\partial x} dW_t",
        content: "# URAE Mathematics Notepad: Stochastic Itô Calculus & Diffusion\n\nmu: Parameter = 0.05 [s^-1]\nsigma: Parameter = 0.20 [s^-0.5]\nS: Variable\n\n# Geometric Brownian Motion\ndX_t = mu * S * dt + sigma * S * dW_t\nito_lemma(ln(S), S, mu, sigma)\nito_quadratic_variation(dW_t)\n",
        tags: &[
            "stochastic",
            "ito",
            "calculus",
            "brownian",
            "sde",
            "black-scholes",
            "diffusion",
        ],
    },
    // 15: Non-Linear Finite Element Analysis
    ExampleNotebook {
        id: "15_nonlinear_fea_analysis",
        title: "Non-Linear FEA & Large Deformations",
        category: ExampleCategory::CadAndSimulation,
        description: "Symbolic 1D non-linear hyperelastic bar deformation alongside 3D CAD mesh Newton-Raphson stiffness assembly and stress visualization.",
        latex_formula: "\\mathbf{K}_{\\text{tan}}(\\mathbf{u}) \\Delta \\mathbf{u} = \\mathbf{F}_{\\text{ext}} - \\mathbf{F}_{\\text{int}}(\\mathbf{u})",
        content: "# URAE Mathematics Notepad: Non-Linear Finite Element Analysis\n\n# Part 1: Symbolic Hyperelasticity\nu(x) = (F / (A * E)) * (x + alpha * x^2)\nsigma_vm(x) = E * diff(u(x), x)\n\n# Part 2: Numerical Mesh & Newton-Raphson\nmesh = mesh_3d_beam(length = 200.0, height = 20.0, depth = 10.0, elements = 216)\nfea_sol = solve_nonlinear_fea(mesh, youngs_modulus = 210000.0, yield_stress = 250.0, load = 5000.0)\nfea_stress_contours(fea_sol)\n",
        tags: &[
            "fea",
            "nonlinear",
            "stress",
            "elasticity",
            "mesh",
            "newton-raphson",
            "simulation",
        ],
    },
    // 16: Transfer Functions & Control Engineering
    ExampleNotebook {
        id: "16_transfer_functions_and_control",
        title: "Transfer Functions & Control Engineering",
        category: ExampleCategory::EngineeringAndControl,
        description: "Laplace domain transfer functions H(s), passive RLC second-order filters, active Sallen-Key Op-Amps, pole-zero stability, and Bode plots.",
        latex_formula: "H(s) = \\frac{\\omega_0^2}{s^2 + 2\\zeta \\omega_0 s + \\omega_0^2}, \\quad s_{1,2} = -\\zeta \\omega_0 \\pm j \\omega_0 \\sqrt{1 - \\zeta^2}",
        content: "# URAE Mathematics Notepad: Transfer Functions & Control Engineering\n\nR: Parameter = 1000.0 [ohm]\nL: Parameter = 0.1 [H]\nC: Parameter = 1e-6 [F]\n\n# Passive RLC Low-Pass Filter Transfer Function\nH_rlc(s) = 1 / (L * C * s^2 + R * C * s + 1)\nbode_plot(H_rlc(s), omega_range = [10.0, 100000.0])\npole_zero_map(H_rlc(s))\n\n# Active Sallen-Key Op-Amp Low-Pass Filter\nH_opamp(s) = 2.0 / (s^2 + 1.414 * s + 1.0)\nstep_response(H_opamp(s))\n",
        tags: &[
            "transfer-function",
            "control",
            "rlc",
            "op-amp",
            "bode",
            "laplace",
            "stability",
            "poles-zeros",
        ],
    },
    // 17: Thermodynamic Cycles & Statistical Mechanics
    ExampleNotebook {
        id: "17_thermodynamics_and_statistical_mechanics",
        title: "Thermodynamic Cycles & Statistical Mechanics",
        category: ExampleCategory::EngineeringAndControl,
        description: "Equations of state (Ideal Gas, Van der Waals), symbolic Maxwell relations, Carnot cycle P-V and T-S diagram trajectories, and thermal efficiency.",
        latex_formula: "\\left(P + \\frac{a}{V_m^2}\\right)(V_m - b) = R T, \\quad \\eta_{\\text{Carnot}} = 1 - \\frac{T_C}{T_H}",
        content: "# URAE Mathematics Notepad: Thermodynamics & Statistical Mechanics\n\nT_h: Parameter = 600.0 [K]\nT_c: Parameter = 300.0 [K]\nR_gas: Parameter = 8.314 [J/(mol*K)]\n\n# Van der Waals Equation of State\nvdw_P(V, T) = (R_gas * T) / (V - 0.0427) - 3.59 / (V^2)\n\n# Maxwell Relations Verification\n# (∂T/∂V)_S = -(∂P/∂S)_V\nmaxwell_relation_check(\"T\", \"V\", \"S\")\n\n# Carnot Cycle P-V and T-S Trajectory\ncarnot_efficiency = 1.0 - T_c / T_h\npv_cycle_diagram(T_hot = T_h, T_cold = T_c, compression_ratio = 4.0)\n",
        tags: &[
            "thermodynamics",
            "carnot",
            "van-der-waals",
            "entropy",
            "pv-diagram",
            "maxwell-relations",
            "statmech",
        ],
    },
    // 18: Multi-Valued, Modal & Intuitionistic Logic Systems
    ExampleNotebook {
        id: "18_formal_and_multivalued_logic_systems",
        title: "Multi-Valued, Modal & Intuitionistic Logic Systems",
        category: ExampleCategory::DModulesAndFormal,
        description: "Propositional boolean logic, Kleene K3, Łukasiewicz Ł3, Bochvar nonsense propagation, Gödel G3 intuitionistic logic, modal S5 (Box/Diamond), and DPLL SAT solving.",
        latex_formula: "A \\to B = \\neg A \\lor B, \\quad \\Box A = \\neg \\Diamond \\neg A, \\quad \\nu(A \\to B) = \\min(1, 1 - \\nu(A) + \\nu(B))",
        content: "# URAE Mathematics Notepad: Multi-Valued, Modal & Intuitionistic Logic Systems\n\n# 1. Classical Propositional Logic & DPLL SAT Solving\np = bool_var(\"p\")\nq = bool_var(\"q\")\nexpr_tautology = bool_implies(p, bool_or([p, q]))\nbool_sat_solve(bool_and([p, bool_not(p)]))\n\n# 2. Three-Valued Logic Systems (Kleene K3 vs Łukasiewicz Ł3)\n# Values: True (1), False (0), Unknown (u)\nlogic_k3_implies(Unknown, Unknown)\nlogic_l3_implies(Unknown, Unknown)\nlogic_bochvar_and(True, Unknown)\nlogic_godel_not(Unknown)\n\n# 3. Modal S5 Necessity (□) and Possibility (◇)\nmodal_necessity(Unknown)\nmodal_possibility(Unknown)\nmodal_duality_check(\"Box A <=> ~Dia ~A\")\n",
        tags: &[
            "logic",
            "boolean",
            "sat",
            "dpll",
            "kleene",
            "lukasiewicz",
            "bochvar",
            "godel",
            "modal",
            "intuitionistic",
        ],
    },
    // 19: Number Theory & Cryptography
    ExampleNotebook {
        id: "19_number_theory_and_cryptography",
        title: "Number Theory, Elliptic Curves & RSA",
        category: ExampleCategory::DiscreteAndCrypto,
        description: "Elliptic curve arithmetic over finite fields GF(p), Weierstrass point addition, Pollard's rho integer factorization, and RSA keypair generation.",
        latex_formula: "y^2 \\equiv x^3 + a x + b \\pmod{p}, \\quad P + Q = R",
        content: "# URAE Mathematics Notepad: Number Theory & Elliptic Curve Cryptography\n\n# 1. Modular Arithmetic & Prime Testing\np = 6277101735386680763835789423207666416083908700390324961279\nis_prime(p)\nmod_pow(7, 100, 101)\nmod_inverse(17, 3120)\n\n# 2. Elliptic Curve over Finite Field GF(p)\ncurve_ecc = elliptic_curve(a = 0, b = 7, p = p) # secp256k1 style\nG = curve_point(curve_ecc, x = 12, y = 34)\nP = curve_mul(G, 1337)\ncurve_point_add(G, P)\n\n# 3. Factorization & Discrete Logarithm\npollard_rho(10403)\n",
        tags: &[
            "number-theory",
            "crypto",
            "rsa",
            "elliptic-curve",
            "modular",
            "prime",
            "factorization",
        ],
    },
    // 20: Graph Theory & Combinatorics
    ExampleNotebook {
        id: "20_graph_theory_and_combinatorics",
        title: "Spectral Graph Theory & Combinatorics",
        category: ExampleCategory::DiscreteAndCrypto,
        description: "Graph Laplacian matrix, spectral gap / algebraic connectivity, shortest path Dijkstra, Max-Flow Min-Cut, and chromatic polynomial.",
        latex_formula: "L = D - A, \\quad \\lambda_2(L) > 0 \\iff G \\text{ is connected}",
        content: "# URAE Mathematics Notepad: Spectral Graph Theory & Combinatorics\n\n# 1. Graph Construction & Adjacency\nG = graph_from_edges([(1, 2), (2, 3), (3, 4), (4, 1), (1, 3)])\nA = graph_adjacency_matrix(G)\nD = graph_degree_matrix(G)\n\n# 2. Graph Laplacian & Spectral Gap\nL = graph_laplacian(G)\nspectral_gap = graph_algebraic_connectivity(G)\ngraph_cheeger_constant(G)\n\n# 3. Network Flow & Paths\nshortest_path_dijkstra(G, start = 1, end = 4)\nmax_flow_edmonds_karp(G, source = 1, sink = 4)\nchromatic_polynomial(G, lambda)\n",
        tags: &[
            "graph",
            "spectral",
            "laplacian",
            "combinatorics",
            "dijkstra",
            "max-flow",
            "connectivity",
        ],
    },
    // 21: Differential Geometry & General Relativity
    ExampleNotebook {
        id: "21_differential_geometry_and_relativity",
        title: "Differential Geometry & General Relativity",
        category: ExampleCategory::GeometryAndTopology,
        description: "Riemannian metric tensors, Christoffel symbols of the second kind, Riemann curvature, Ricci tensor, and Schwarzschild black hole geodesic orbits.",
        latex_formula: "R^\\rho_{\\sigma\\mu\\nu} = \\partial_\\mu \\Gamma^\\rho_{\\nu\\sigma} - \\partial_\\nu \\Gamma^\\rho_{\\mu\\sigma} + \\Gamma^\\rho_{\\mu\\lambda}\\Gamma^\\lambda_{\\nu\\sigma} - \\Gamma^\\rho_{\\nu\\lambda}\\Gamma^\\lambda_{\\mu\\sigma}",
        content: "# URAE Mathematics Notepad: Differential Geometry & General Relativity\n\nr: Variable\ntheta: Variable\nphi: Variable\nt: Variable\nM: Parameter = 1.0\n\n# 1. Schwarzschild Spacetime Metric Tensor g_μν\n# ds² = -(1 - 2M/r) dt² + (1 - 2M/r)⁻¹ dr² + r² (dθ² + sin²θ dφ²)\ng = metric_tensor(\"schwarzschild\", coords = [t, r, theta, phi], mass = M)\n\n# 2. Christoffel Symbols of the 2nd Kind\nGamma = christoffel_symbols(g)\n\n# 3. Curvature & Geodesic Equations\nRiemann = riemann_tensor(g)\nRicci = ricci_tensor(g)\nRicci_scalar = ricci_scalar(g)\ngeodesic_orbit(g, initial_r = 10.0, angular_momentum = 3.75)\n",
        tags: &[
            "geometry",
            "tensor",
            "relativity",
            "riemann",
            "ricci",
            "christoffel",
            "schwarzschild",
            "curvature",
        ],
    },
    // 22: Abstract Algebra & Galois Theory
    ExampleNotebook {
        id: "22_abstract_algebra_and_galois_theory",
        title: "Abstract Algebra, Rings & Galois Theory",
        category: ExampleCategory::ExceptionalAndAbstract,
        description: "Polynomial splitting fields, solvability by radicals, Galois groups Gal(L/K), quotient rings, ideals, and group character tables.",
        latex_formula: "\\operatorname{Gal}(\\mathbb{Q}(\\zeta_n)/\\mathbb{Q}) \\cong (\\mathbb{Z}/n\\mathbb{Z})^\\times, \\quad G/N \\cong \\operatorname{Im}(\\phi)",
        content: "# URAE Mathematics Notepad: Abstract Algebra & Galois Theory\n\nx: Variable\n\n# 1. Polynomial Splitting Field & Galois Group\npoly = x^5 - 4*x + 2 # Eisenstein at p=2 -> Irreducible, S_5 Galois group\ngalois_group(poly, x)\nis_solvable_by_radicals(poly, x)\n\n# 2. Cyclotomic Polynomials & Automorphisms\ncyclotomic_poly(n = 12, var = x)\nsplitting_field(x^3 - 2, x)\n\n# 3. Finite Group Structures & Quotient Rings\nG = symmetric_group(4) # S_4\ngroup_order(G)\ngroup_conjugacy_classes(G)\ngroup_character_table(G)\n",
        tags: &[
            "abstract-algebra",
            "galois",
            "splitting-field",
            "group-theory",
            "rings",
            "ideals",
            "cyclotomic",
        ],
    },
    // 23: Complex Analysis & Contour Integration
    ExampleNotebook {
        id: "23_complex_analysis_and_contour_integration",
        title: "Complex Analysis & Contour Residues",
        category: ExampleCategory::CalculusAndOde,
        description: "Cauchy's Integral Theorem, Laurent series expansions, pole classification, and Residue Theorem calculation of stubborn real improper integrals.",
        latex_formula: "\\oint_\\gamma f(z)\\,dz = 2\\pi i \\sum_{k=1}^n \\operatorname{Res}(f, z_k), \\quad \\int_{-\\infty}^\\infty \\frac{\\cos(x)}{x^2 + a^2}\\,dx = \\frac{\\pi e^{-a}}{a}",
        content: "# URAE Mathematics Notepad: Complex Analysis & Contour Integration\n\nz: Variable\nx: Variable\na: Parameter = 2.0\n\n# 1. Complex Laurent Series & Pole Classification\nf(z) = exp(z) / (z^2 * (z - 1))\nlaurent_series(f(z), z, center = 0, order_min = -2, order_max = 3)\ncomplex_singularities(f(z), z)\nresidue(f(z), z, point = 0)\nresidue(f(z), z, point = 1)\n\n# 2. Real Improper Integration via Semicircular Contour\n# Evaluate ∫_{-∞}^{∞} 1 / (x⁴ + 1) dx\ncontour_integrate(1 / (z^4 + 1), z, contour = \"upper_half_circle\")\nintegrate(1 / (x^4 + 1), x, -Infinity, Infinity)\n",
        tags: &[
            "complex-analysis",
            "residue",
            "contour-integral",
            "laurent",
            "cauchy",
            "poles",
            "holomorphy",
        ],
    },
    // 24: Fluid Dynamics & Navier-Stokes
    ExampleNotebook {
        id: "24_fluid_dynamics_and_navier_stokes",
        title: "Fluid Dynamics & Navier-Stokes Momentum Equations",
        category: ExampleCategory::MultiphysicsAndAi,
        description: "Analytical and computational fluid dynamics: Navier-Stokes momentum equations, Hagen-Poiseuille laminar channel flow derivation, parabolic velocity profile assembly, flow rate integration, and shear stress.",
        latex_formula: r#"\mu \frac{d^2 u}{dy^2} = \frac{dP}{dx} \implies u(y) = \frac{G h^2}{2\mu}\left(1 - \frac{y^2}{h^2}\right), \quad Q = \int_{-h}^h u(y)\,dy = \frac{2 G h^3}{3\mu}"#,
        content: "# URAE Mathematics Notepad: Fluid Dynamics & Incompressible Navier-Stokes\n# Demonstrates assembling the Navier-Stokes momentum equations, integrating for laminar channel flow, matching boundary conditions, and computing shear stress.\n\n# 1. Flow & Fluid Parameters\nmu: Parameter = 0.001 [Pa*s]\nrho: Parameter = 1000.0 [kg/m^3]\nh: Parameter = 0.01 [m]\nG: Parameter = 50.0 [Pa/m]\n\ny: Variable\n{ y in Reals | -h <= y <= h }\n\n# 2. Assembling the 1D Steady Navier-Stokes Momentum Equation\n# For steady laminar flow between parallel plates:\n# mu * d^2 u / dy^2 = dP/dx = -G\n# Integrating twice yields: u(y) = -(G / (2*mu)) * y^2 + C1 * y + C2\n# Boundary conditions (no-slip): u(h) = 0, u(-h) = 0 => C1 = 0, C2 = G*h^2 / (2*mu)\n\n# 3. Assembled Analytical Velocity Profile u(y)\nu_channel(y) = (G * h^2 / (2.0 * mu)) * (1.0 - (y / h)^2)\n\n# 4. Centerline Maximum Velocity & Mean Velocity\nu_max = (G * h^2) / (2.0 * mu)\nu_mean = (2.0 / 3.0) * u_max\n\n# 5. Volume Flow Rate & Wall Shear Stress\n# Volumetric flow rate per unit depth: Q = integrate(u(y), y, -h, h)\nQ_flow = (2.0 * G * h^3) / (3.0 * mu)\n\n# Wall shear stress tau_w = mu * |du/dy| at y = h\ntau_wall = G * h\n\n# 6. Reynolds Number Verification\nRe_flow = (rho * u_mean * (2.0 * h)) / mu\n",
        tags: &[
            "cfd",
            "navier-stokes",
            "fluid-dynamics",
            "poiseuille",
            "viscosity",
            "shear-stress",
            "pde",
            "momentum",
        ],
    },
    // 25: Statistics & Bayesian Inference
    ExampleNotebook {
        id: "25_statistics_and_bayesian_inference",
        title: "Statistics, Probability & Bayesian MCMC",
        category: ExampleCategory::DataAndStatistics,
        description: "Parametric distributions (Gaussian, Student's t, Beta-Binomial conjugate), Maximum Likelihood Estimation (MLE), and Metropolis-Hastings MCMC posterior sampling.",
        latex_formula: "p(\\theta \\mid \\mathcal{D}) = \\frac{p(\\mathcal{D} \\mid \\theta)\\,p(\\theta)}{\\int p(\\mathcal{D} \\mid \\theta')\\,p(\\theta')\\,d\\theta'}, \\quad \\alpha = \\min\\left(1, \\frac{p(\\theta^* \\mid \\mathcal{D})}{p(\\theta_{t-1} \\mid \\mathcal{D})}\\right)",
        content: "# URAE Mathematics Notepad: Statistics, Probability & Bayesian Inference\n\n# 1. Continuous & Discrete Distributions\ndist_norm = distribution_normal(mean = 0.0, std = 1.0)\ndist_beta = distribution_beta(alpha = 2.0, beta = 5.0)\npdf(dist_norm, x = 1.96)\ncdf(dist_norm, x = 1.96)\n\n# 2. Conjugate Beta-Binomial Bayesian Updating\n# Prior Beta(2, 2) + Data (7 heads, 3 tails) -> Posterior Beta(9, 5)\nprior_beta = distribution_beta(alpha = 2.0, beta = 2.0)\nposterior_beta = bayesian_conjugate_update(prior_beta, successes = 7, trials = 10)\ncredible_interval(posterior_beta, level = 0.95)\n\n# 3. Markov Chain Monte Carlo (Metropolis-Hastings)\nmcmc_samples = metropolis_hastings(target_log_prob = \"-0.5 * theta^2\", iterations = 5000, step_size = 0.2)\nmcmc_summary(mcmc_samples)\n",
        tags: &[
            "statistics",
            "bayesian",
            "mcmc",
            "distributions",
            "probability",
            "conjugate-prior",
            "inference",
        ],
    },
    // 26: Advanced Linear Algebra & Matrix Analysis
    ExampleNotebook {
        id: "26_matrix_analysis_and_krylov",
        title: "Matrix Analysis, SVD & Krylov Subspaces",
        category: ExampleCategory::DataAndStatistics,
        description: "Singular Value Decomposition (SVD), condition numbers, Schur triangularization, Arnoldi iterations, and GMRES sparse iterative linear solvers.",
        latex_formula: "\\mathbf{A} = \\mathbf{U} \\mathbf{\\Sigma} \\mathbf{V}^T, \\quad \\mathcal{K}_m(\\mathbf{A}, \\mathbf{b}) = \\operatorname{span}\\{\\mathbf{b}, \\mathbf{A}\\mathbf{b}, \\mathbf{A}^2\\mathbf{b}, \\dots, \\mathbf{A}^{m-1}\\mathbf{b}\\}",
        content: "# URAE Mathematics Notepad: Matrix Analysis, SVD & Krylov Subspaces\n\n# 1. Matrix Decompositions & Condition Number\nA = [[4.0, 1.0, 2.0], [1.0, 5.0, 3.0], [2.0, 3.0, 6.0]]\nsvd_decomp = matrix_svd(A)\nschur_decomp = matrix_schur(A)\ncond_num = matrix_condition_number(A)\nmatrix_exponential(A)\n\n# 2. Large Sparse Systems & Krylov Iterations\n# Arnoldi Iteration for Krylov Subspace K_m(A, b)\nb = [1.0, 0.0, 1.0]\nkrylov_basis = arnoldi_iteration(A, b, subspace_dim = 3)\nsol_gmres = gmres_solve(A, b, restart = 10, max_iter = 50, tol = 1e-6)\n",
        tags: &[
            "linear-algebra",
            "matrix",
            "svd",
            "krylov",
            "gmres",
            "arnoldi",
            "eigenvalues",
            "condition-number",
        ],
    },
    // 27: Silicon Photonics & Graphene Mode Distribution
    ExampleNotebook {
        id: "27_photonic_chip_graphene",
        title: "Silicon Photonics & Graphene Waves",
        category: ExampleCategory::QuantumAndPhysics,
        description: "Electromagnetic wave propagation in dielectric slab waveguides with graphene surface cladding: Helmholtz wave equation, boundary condition matching, transcendental TE dispersion relation assembly, eigenvalue solving, and modal attenuation.",
        latex_formula: r#"u \tan(u) = \sqrt{V^2 - u^2}, \quad V = k_0 \frac{d}{2}\sqrt{n_{\text{core}}^2 - n_{\text{clad}}^2}, \quad n_{\text{eff}} = \sqrt{n_{\text{core}}^2 - \left(\frac{u}{k_0 a}\right)^2}"#,
        content: "# URAE Mathematics Notepad: Silicon Photonics & Graphene Mode Distribution\n# Models electromagnetic wave propagation in a dielectric slab waveguide with graphene surface cladding.\n# Assembles the Helmholtz wave equation, interface boundary conditions, the transcendental TE dispersion relation, and modal attenuation.\n\n# 1. Waveguide Geometry and Material Parameters\nlambda: Parameter = 1.55 [um]\nlambda in Reals [1.2, 1.7]\n\nd_core: Parameter = 0.22 [um]\nd_core in Reals [0.10, 0.50]\n\nn_si = 3.48\nn_sio2 = 1.44\n\n# Core half-width a = d_core / 2\na = d_core / 2.0\n\n# Free-space wavenumber k0 = 2*pi / lambda\nk0 = 2.0 * 3.14159265 / lambda\n\n# 2. Normalized Frequency (V-Number) Assembly\n# V = k0 * a * sqrt(n_si^2 - n_sio2^2)\nV = k0 * a * sqrt(n_si^2 - n_sio2^2)\n\n# 3. Transverse Resonance & Dispersion Relation Assembly\n# In core (|y| <= a): Ey''(y) + ky^2 * Ey = 0 => Ey(y) = A * cos(ky * y)\n# In cladding (|y| > a): Ey''(y) - gamma^2 * Ey = 0 => Ey(y) = C * exp(-gamma * (|y| - a))\n# Matching boundary continuity of Ey and dEy/dy at y = a yields:\n# u * tan(u) = w, where u = ky * a, w = gamma * a, and u^2 + w^2 = V^2\nu: Variable\n{ u in Reals | 0.01 <= u <= V }\n\ndispersion_te(u) = u * tan(u) - sqrt(V^2 - u^2)\n\n# Assembling the dispersion equation and solving for guided mode eigenvalue u\nsolve u * tan(u) - sqrt(V^2 - u^2) = 0, u\n\n# Solved fundamental mode eigenvalue u0:\nu0 = 1.346\nw0 = sqrt(V^2 - u0^2)\n\n# 4. Effective Refractive Index\n# beta^2 = k0^2 * n_si^2 - (u0/a)^2 => n_eff = beta / k0\nn_eff = sqrt(n_si^2 - (u0 / (k0 * a))^2)\n\n# 5. Electric Field Mode Profile Ey(y)\ny: Variable\n{ y in Reals | -1.0 <= y <= 1.0 }\n\n# Assembled piecewise field profile across core and cladding\nE_core(y) = cos(u0 * (y / a))\nE_clad(y) = cos(u0) * exp(-w0 * (abs(y) - a) / a)\n\n# 6. Graphene Cladding Perturbation & Attenuation\n# Surface field intensity at interface y = a:\nI_surf = cos(u0)^2\n\n# Mode power confinement factor on graphene sheet:\nGamma_graphene = I_surf / (a * (1.0 + sin(2.0 * u0) / (2.0 * u0)) + (I_surf * a) / w0)\n\n# Modal attenuation from surface conductivity: alpha = Gamma * sigma_0 / (eps_0 * c * n_eff)\nalpha_attenuation = Gamma_graphene * 0.023 / n_eff\n",
        tags: &[
            "photonics",
            "optics",
            "graphene",
            "waveguide",
            "electromagnetics",
            "helmholtz",
            "dispersion",
            "modes",
            "attenuation",
        ],
    },
    // 28: Differential Equations: ODEs, PDEs & Solitons
    ExampleNotebook {
        id: "28_differential_equations_ode_pde",
        title: "Differential Equations: ODEs, PDEs & Solitons",
        category: ExampleCategory::CalculusAndOde,
        description: "Assembly and solving of ordinary and partial differential equations: 2nd-order damped harmonic oscillator ODE characteristic equation and analytical solution, 1D Wave equation separation of variables, and Korteweg-de Vries (KdV) traveling wave soliton reduction.",
        latex_formula: r#"r^2 + 2\zeta\omega r + \omega^2 = 0 \implies y(t) = e^{-\zeta\omega t}\left(C_1\cos(\omega_d t) + C_2\sin(\omega_d t)\right), \quad u(x,t) = \frac{c}{2}\operatorname{sech}^2\left(\frac{\sqrt{c}}{2}(x - ct)\right)"#,
        content: "# URAE Mathematics Notepad: Differential Equations (ODEs, PDEs & Nonlinear Reductions)\n# Demonstrates assembling differential equations, solving characteristic polynomials for roots, deriving analytical solutions, and reducing nonlinear PDEs.\n\n# =========================================================================\n# 1. LINEAR 2ND-ORDER DAMPED HARMONIC OSCILLATOR ODE\n# Governing equation: y''(t) + 2*zeta*omega*y'(t) + omega^2*y(t) = 0\n# =========================================================================\nzeta: Parameter = 0.20\nomega: Parameter = 3.00 [rad/s]\nt: Variable\n{ t in Reals | 0.0 <= t <= 10.0 }\n\n# Assembling the ODE differential expression:\node_osc = diff(y, t, 2) + 2 * zeta * omega * diff(y, t, 1) + omega^2 * y\n\n# Setting up the algebraic characteristic equation: r^2 + 2*zeta*omega*r + omega^2 = 0\n# Solving for roots:\nsolve r^2 + 2 * zeta * omega * r + omega^2 = 0, r\n\n# Damped natural frequency: omega_d = omega * sqrt(1 - zeta^2)\nomega_d = omega * sqrt(1.0 - zeta^2)\n\n# Assembling the exact analytical response for initial state y(0) = 1.0, y'(0) = 0:\ny_resp(t) = exp(-zeta * omega * t) * (cos(omega_d * t) + (zeta * omega / omega_d) * sin(omega_d * t))\n\n# =========================================================================\n# 2. LINEAR 2ND-ORDER PDE: 1D WAVE EQUATION\n# Governing equation: u_tt - c^2 * u_xx = 0\n# Separation of variables: u(x, t) = X(x) * T(t) => X'' + k^2*X = 0, T'' + (c*k)^2*T = 0\n# =========================================================================\nc_wave: Parameter = 2.0 [m/s]\nL_string: Parameter = 1.0 [m]\nx: Variable\n{ x in Reals | 0.0 <= x <= L_string }\n\n# Mode wavenumber and angular frequency for fundamental harmonic:\nk_mode = 3.14159265 / L_string\nomega_wave = c_wave * k_mode\n\n# Assembled standing wave solution: u(x, t) = sin(k*x) * cos(omega*t)\nu_standing(x) = sin(k_mode * x) * cos(omega_wave * 0.25)\n\n# =========================================================================\n# 3. NONLINEAR PDE: KORTEWEG-DE VRIES (KdV) SOLITON REDUCTION\n# Governing equation: u_t + 6*u*u_x + u_xxx = 0\n# Traveling wave reduction: xi = x - c*t, u(x, t) = phi(xi)\n# Yields: -c*phi' + 6*phi*phi' + phi''' = 0\n# Integrating once: -c*phi + 3*phi^2 + phi'' = 0\n# Multiplying by phi' and integrating: (phi')^2 = c*phi^2 - 2*phi^3\n# =========================================================================\nc_soliton: Parameter = 4.0 [m/s]\n\n# Exact analytical solitary wave solution phi(xi) = (c/2) * sech^2(sqrt(c)/2 * xi):\nsoliton(x) = (c_soliton / 2.0) / (cosh(sqrt(c_soliton) / 2.0 * x))^2\n",
        tags: &[
            "diffeq",
            "ode",
            "pde",
            "soliton",
            "kdv",
            "harmonic-oscillator",
            "wave-equation",
            "analytical",
            "nonlinear",
        ],
    },
    // 29: Modern & Classical Control: SISO, MIMO & Discrete
    ExampleNotebook {
        id: "29_control_theory_siso_mimo",
        title: "Modern & Classical Control: SISO, MIMO & State-Space",
        category: ExampleCategory::EngineeringAndControl,
        description: "Assembling control system models: state-space matrices, characteristic polynomial determinant derivation, solving for poles, transfer function assembly, controllability matrix rank test, and state-feedback pole placement.",
        latex_formula: r#"\det(s\mathbf{I} - \mathbf{A}) = s^2 + 2\zeta\omega_n s + \omega_n^2 = 0, \quad \mathcal{C} = [\mathbf{B} \quad \mathbf{AB}], \quad H(s) = \mathbf{C}(s\mathbf{I} - \mathbf{A})^{-1}\mathbf{B}"#,
        content: "# URAE Mathematics Notepad: Modern & Classical Control Systems (SISO, MIMO & Pole Placement)\n# Demonstrates assembling state-space models, computing characteristic polynomials, solving for open-loop poles, and verifying controllability.\n\n# 1. Continuous SISO System Parameters\nwn: Parameter = 3.00 [rad/s]\nzeta: Parameter = 0.35\n\ns: Variable\n\n# 2. Assembling the State-Space Matrices\n# x_dot = A * x + B * u, y = C * x + D * u\n# A = [[0, 1], [-wn^2, -2*zeta*wn]], B = [[0], [wn^2]], C = [[1, 0]], D = [[0]]\nA = [[0.0, 1.0], [-9.0, -2.1]]\nB = [[0.0], [9.0]]\nC = [[1.0, 0.0]]\n\n# 3. Assembling the Characteristic Polynomial & Solving for Poles\n# det(s*I - A) = det([[s, -1], [wn^2, s + 2*zeta*wn]]) = s^2 + 2*zeta*wn*s + wn^2\nchar_poly = s^2 + 2.0 * zeta * wn * s + wn^2\n\n# Solving for open-loop poles:\nsolve s^2 + 2.0 * zeta * wn * s + wn^2 = 0, s\n\n# 4. Assembled Closed-Form Transfer Function H(s)\n# H(s) = C * inv(s*I - A) * B = wn^2 / (s^2 + 2*zeta*wn*s + wn^2)\nH_siso(s) = wn^2 / (s^2 + 2.0 * zeta * wn * s + wn^2)\n\n# Frequency response: substitute s = j * omega\n# Natural frequency and damping metrics:\nomega_peak = wn * sqrt(1.0 - 2.0 * zeta^2)\npeak_overshoot = exp(-3.14159265 * zeta / sqrt(1.0 - zeta^2))\n\n# 5. Assembling Controllability Matrix C = [B, A*B]\n# A * B = [[0, 1], [-9, -2.1]] * [[0], [9]] = [[9], [-18.9]]\n# Controllability matrix: C_mat = [[0, 9], [9, -18.9]]\ndet_controllability = 0.0 * (-18.9) - 9.0 * 9.0 # = -81.0 != 0 => Completely Controllable!\n\n# 6. State-Feedback Pole Placement Design (Ackermann Assembly)\n# Desired closed-loop poles at p1 = -4.0, p2 = -5.0\n# Desired polynomial: (s + 4)*(s + 5) = s^2 + 9*s + 20\n# Equating with det(s*I - (A - B*K)) = s^2 + (2*zeta*wn + 9*k2)*s + (wn^2 + 9*k1)\n# Solving for feedback gains k1, k2:\nsolve 9.0 + 9.0 * k1 = 20.0, k1\nsolve 2.1 + 9.0 * k2 = 9.0, k2\nk1_gain = 11.0 / 9.0\nk2_gain = 6.9 / 9.0\n",
        tags: &[
            "control",
            "state-space",
            "siso",
            "mimo",
            "poles",
            "transfer-function",
            "controllability",
            "ackermann",
            "pole-placement",
        ],
    },
    // 30: 1D Viscous Burgers PDE & Cole-Hopf Reduction
    ExampleNotebook {
        id: "30_burgers_viscous_shock_pde",
        title: "1D Viscous Burgers PDE & Shock Dissipation",
        category: ExampleCategory::MultiphysicsAndAi,
        description: "Salvaged multiphysics PDE benchmark: 1D viscous Burgers equation u_t + u u_x = nu u_xx, Cole-Hopf linearization, travelling shock profile u(x,t) = c - a*tanh((a/(2*nu))*(x - c*t)), and Method-of-Lines energy decay.",
        latex_formula: r#"\frac{\partial u}{\partial t} + u \frac{\partial u}{\partial x} = \nu \frac{\partial^2 u}{\partial x^2}, \quad u = -2\nu \frac{\phi_x}{\phi}, \quad \phi_t = \nu \phi_{xx}"#,
        content: "# URAE Mathematics Notepad: 1D Viscous Burgers PDE & Cole-Hopf Shock\n# Nonlinear advection steepens smooth fronts while viscosity nu smooths shocks.\n\n# 1. Physical Viscosity & Wave Speed Parameters\nnu: Parameter = 0.05 [m^2/s]\nnu in Reals [0.005, 0.25]\n\nc_shock: Parameter = 1.0 [m/s]\na_amp: Parameter = 0.8 [m/s]\nt_eval: Parameter = 0.25 [s]\nt_eval in Reals [0.0, 2.0]\n\n# 2. Travelling Viscous Shock Profile (Exact Cole-Hopf Solution)\nx: Variable\n{ x in Reals | -2.0 <= x <= 3.0 }\n\nu_shock(x) = c_shock - a_amp * tanh((a_amp / (2.0 * nu)) * (x - c_shock * t_eval))\n\n# 3. Cole-Hopf Linearization Residual Check (phi_t - nu * phi_xx = 0)\n# Verify Burgers PINN residual zero-order identity:\npinn_burgers_residual\n\n# 4. Characteristic Reynolds Number & Shock Thickness delta ~ 2*nu / a_amp\nshock_thickness = 2.0 * nu / a_amp\n",
        tags: &[
            "burgers",
            "pde",
            "shock",
            "cole-hopf",
            "method-of-lines",
            "viscosity",
            "fluid-dynamics",
        ],
    },
    // 31: Lorenz Strange Attractor & Self-Optimizing Meta-Integrator
    ExampleNotebook {
        id: "31_lorenz_strange_attractor_meta",
        title: "Lorenz Strange Attractor & Meta-Integrator Regimes",
        category: ExampleCategory::CalculusAndOde,
        description: "Autonomous chaotic Lorenz-63 dynamical system (sigma=10, rho=28, beta=8/3): fixed-point equilibria, linearized Jacobian eigenvalues, divergence rate, and Tsit5 / SelfOptimizingMeta adaptive ODE stepping.",
        latex_formula: r#"\dot{x} = \sigma(y - x), \quad \dot{y} = x(\rho - z) - y, \quad \dot{z} = xy - \beta z, \quad \nabla \cdot \mathbf{F} = -(\sigma + 1 + \beta)"#,
        content: "# URAE Mathematics Notepad: Lorenz-63 Attractor & Meta-Integrator Dynamics\n# Models atmospheric convection chaos & non-stiff/oscillatory ODE stepping regimes.\n\n# 1. Classic Chaotic Parameters\nsigma: Parameter = 10.0\nrho: Parameter = 28.0\nrho in Reals [0.5, 40.0]\nbeta: Parameter = 2.6667\n\n# 2. Convective Butterfly Equilibria C+, C- = (+/- sqrt(beta*(rho - 1)), +/- sqrt(beta*(rho - 1)), rho - 1)\nz_eq = rho - 1.0\nx_eq = sqrt(beta * z_eq)\n\n# 3. Phase-Space Volume Contraction (Dissipation Divergence)\ndiv_flow = -(sigma + 1.0 + beta)\n\n# 4. Pitchfork Bifurcation Threshold\nsolve r_bif - 1.0 = 0, r_bif\n",
        tags: &[
            "lorenz",
            "chaos",
            "attractor",
            "ode",
            "meta-integrator",
            "tsit5",
            "dynamical-systems",
        ],
    },
    // 32: Hexagonal Lattices (`Hecs`) & Robotics SE(3) Pose Kinematics (`URScript`)
    ExampleNotebook {
        id: "32_hecs_and_urscript_robotics_se3",
        title: "Hexagonal Grids (Hecs) & URScript SE(3) Robotics",
        category: ExampleCategory::GeometryAndTopology,
        description: "Salvaged spatial geometry modules integrated into URAE: axial/cube hexagonal grid Manhattan metrics (Hecs<T>) paired with SE(3) Lie group rigid transforms and URScript axis-angle kinematics.",
        latex_formula: r#"d_{\text{hex}}(a, b) = \frac{|q_a - q_b| + |r_a - r_b| + |s_a - s_b|}{2}, \quad T = \begin{pmatrix} R & \mathbf{p} \\ \mathbf{0}^T & 1 \end{pmatrix} \in \mathrm{SE}(3)"#,
        content: "# URAE Mathematics Notepad: Hexagonal Lattices & SE(3) Robotics Kinematics\n# Combines discrete axial hex distance metrics with continuous rigid-body pose transforms.\n\n# 1. Axial Hexagonal Cube Constraint: q + r + s = 0\nq_coord: Parameter = 3\nr_coord: Parameter = -1\ns_coord = -(q_coord + r_coord)\n\n# Hex Manhattan norm from origin\nd_hex = (abs(q_coord) + abs(r_coord) + abs(s_coord)) / 2\n\n# 2. SE(3) Screw & URScript Pose Composition\n# Translation p = [x_tcp, y_tcp, z_tcp] and wrist axis-angle rotation rx\nx_tcp: Parameter = 0.45 [m]\ny_tcp: Parameter = -0.18 [m]\nz_tcp: Parameter = 0.32 [m]\ntcp_norm = sqrt(x_tcp^2 + y_tcp^2 + z_tcp^2)\n",
        tags: &[
            "hecs",
            "hex-grid",
            "urscript",
            "se3",
            "robotics",
            "kinematics",
            "rigid-transform",
        ],
    },
];
