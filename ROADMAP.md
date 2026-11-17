# URAE Grand Unified Master Roadmap & Engineering Blueprint

**Universal Rust Algebra Engine (URAE)**  
*Authoritative Master Specification for System Architecture, Completed Phases, Configurable Resource Budgets, Soft/Hard Caps, Domain Heuristics, Fluid Inline Notepad Frontend, CAD/FEA Simulations, N-Dimensional Design, and Extended Phased Roadmap*

**Planning index and defect ledger:** [`docs/PLANNING.md`](docs/PLANNING.md).

---

## 1. Executive Architecture & Workspace Inventory

The **Universal Rust Algebra Engine (URAE)** is a high-performance, axiomatic Computer Algebra System (CAS), symbolic-numerical computing engine, and computational geometry/multiphysics simulation platform in Rust.

```mermaid
graph TD
    Core["algebra-core<br/>(AST Arena DAG, Number, Traits, Config & Budgets, Operations)"]
    Engine["algebra-engine<br/>(Calculus, Systems, Gröbner, Analysis, ODE/PDE, Branch, Limits, Cartan, CAD, FEA)"]
    Advanced["algebra-advanced<br/>(Surreal, p-adic, Adele, Knots, Proof, Quantum, GPU, Clifford, Ritt-Wu, Meijer G)"]
    Facade["urae<br/>(Unified Facade, Macros & Prelude)"]
    CLI["urae-cli<br/>(REPL, JSON-RPC, Batch)"]
    FFI["urae-ffi<br/>(C-ABI, Rust-side Python/Jupyter wrappers; PyO3/ZMQ in Wave 2)"]
    Notebook["urae-notebook<br/>(Fluid In-Situ Reactive Notepad, 2D/3D Canvas & WebGPU)"]
    WASM["urae-wasm<br/>(WebAssembly Browser Target)"]

    Core --> Engine --> Facade
    Core --> Advanced --> Facade
    Facade --> CLI
    Facade --> FFI
    Facade --> Notebook
    Facade --> WASM
```

---

## 2. Completed & Current Milestones (Phases 1 to 31)

> **Reading the status column.** A check mark means the named code exists and has tests; it does not mean the subject is covered in depth. The 16 Nov 2026 audit (the earlier round is in `docs/AUDIT-2026-11-15.md`) found phases 8, 28 and 30 stated more strongly than the code supports and relabelled them. The other phases have not been audited line by line.

| Phase | Subsystem | Status | Key Deliverables |
| :--- | :--- | :--- | :--- |
| **Phase 1** | **Axiomatic Core & Category Traits** | ✅ **100%** | `DifferentialRing`, `LieAlgebra`, `TropicalSemiring`, `HeytingAlgebra`, `DomainFeatureDetector`, modular `IdentityGroup` clusters, dynamic rule selector. |
| **Phase 2** | **Symbolic & Adaptive Numerical ODE/PDE** | ✅ **100%** | Exact ODE (linear, Bernoulli, Cauchy-Euler, Putzer systems), adaptive DOPRI5, stiff Radau IIA with Newton relaxation, symplectic Verlet, PDE wave/heat/FEM. |
| **Phase 3** | **Domain Lifting & Multi-Variate Systems** | ✅ **100%** | `DomainLiftRouter`, `RetractionCleaner` (Euler/Weierstrass/Laplace/Clifford); Bareiss/LU linear systems, Putzer matrix exponentials, SNF Diophantine lattice solver, multi-variable Newton-Raphson, and nested structure tensors $\mathcal{A} \otimes \mathcal{B}$. |
| **Phase 4** | **Gröbner Bases & Root Solvers** | ✅ **100%** | Multi-variate polynomials $K[x_1, \dots, x_n]$, monomial orderings (`Lex`, `GradedLex`, `GrevLex`), Buchberger algorithm with Gebauer-Möller criteria, Cardano/Ferrari radicals, Bring radical $\operatorname{BR}(a)$, Sturm sequences, Durand-Kerner complex roots, non-linear variety solvers. |
| **Phase 4+** | **Function Analysis & Zero Classification** | ✅ **100%** | `FunctionAnalyzer` (`algebra_engine::analysis`): zero multiplicity $m$ (`Simple`, `TangentialExtremum`, `InflectionCrossing`), 1D/ND critical points, local extrema, stationary/non-stationary inflection points, and Hessian eigenvalue spectral inertia classification (`LocalMinimum`, `LocalMaximum`, `SaddlePoint`, `Degenerate`). |
| **Phase 5** | **Painlevé, Surreals, $p$-adics, Adeles & Knots** | ✅ **100%** | Painlevé I–VI ($P_{\text{I}} - P_{\text{VI}}$), Heun equations, Conway Surreal numbers $\mathbf{No}$ ($\omega, \epsilon$), $p$-adic fields $\mathbb{Q}_p/\mathbb{Z}_p$, Hensel root lifter, Adele ring $\mathbb{A}_\mathbb{Q}$, Idele group $\mathbb{I}_K$, global Artin product formula $\prod_v |x|_v = 1$, Artin Braid groups $B_n$, Kauffman bracket $\langle K \rangle(A)$, Jones knot polynomials $V(t)$. |
| **Branch Engine** | **Branch Cuts & Multi-Valued Functions** | ✅ **100%** | `BranchCutManager` (`algebra_engine::branch`): `Principal`, `Sheet(k)`, `CustomCut`, `MultiValued`, `AllSheets`, `Unspecified`; discontinuity jumps $\Delta f(z)$, monodromy analytic continuation across paths $\gamma(t)$. |
| **Phase 6** | **Differential Geometry, Clifford, Logics & Extended Traits** | ✅ **100%** | Free tensor algebra $T(V)$, Clifford algebra $\operatorname{Cl}(p,q,r)$, rotor sandwiches $v' = R v R^\dagger$, Cartan calculus ($\mathrm{d}, \iota_X, \mathcal{L}_X, \star$), 3-valued logics (Łukasiewicz, Kleene, Gödel), Schwartz distributions ($\delta, \theta, \operatorname{p.v.}$), Extended category traits (`Semiring`, `NearRing`, `JordanAlgebra`, `AlternativeAlgebra`, `HopfAlgebra`). |
| **Phase 7** | **Performance Engine & Test Consolidation** | ✅ **100%** | Consolidated root `tests/` directory, dynamic `ResourceBudget` with soft/hard caps, hardware auto-detection, and $O(1)$ Schwartz-Zippel pre-filtering. |
| **Phase 8** | **Advanced Transcendental Risch & Limits Engine** | 🟡 **Partial** | Complete Risch differential field integrator, Trager-Rothstein Hermite reduction, multi-order L'Hôpital & Gruntz limit engine, Lagrange-Bürmann inversion, Transseries $\mathbb{T}$. **Audit 16 Nov 2026:** there is no complete Risch algorithm. Integration is table and pattern rules, integration by parts for polynomial times exp/sin/cos/ln, u-substitution and rational-function partial fractions; products such as x times sqrt(x + 1) still fail. Limits use dominant-term analysis and L Hopital, not full Gruntz. Transseries are not implemented. |
| **Phase 9** | **Fluid In-Situ Reactive Notepad Frontend** | ✅ **100%** | Single unified reactive notepad document; click-to-edit LaTeX transformation; right-gutter live 2D/3D graphs; left-gutter parameter scrubbers; below-line diagnostic dropdowns; fine-grained DAG delta invalidation at 60 FPS; interactive builder palettes (Matrix, Domain, Branch Cut, ODE, Units). |
| **Phase 10** | **Multi-Tier Probabilistic Heuristic Engine & Solution Certification** | ✅ **100%** | 4-Tier probabilistic path weighting in A* / Monte Carlo / E-Graphs, progressive multi-prime CRT re-verification ($\epsilon < 10^{-20}$), opportunistic deterministic proofs, and `Solution::Probabilistic` certificates when resource caps are reached. |
| **Phase 11** | **Complete KaTeX Rustdocs & Ergonomic DSL Macro Suite** | ✅ **100%** | Exhaustive KaTeX mathematical docstrings and complete DSL macro library (`solve!`, `diff!`, `integrate!`, `risch!`, `limit!`, `taylor!`, `laurent!`, `puiseux!`, `pade!`, `grad!`, `div!`, `curl!`, `laplacian!`, `jacobian!`, `hessian!`, `ode_linear!`, `pde_heat_1d!`, `verlet!`, `dopri5!`, `radau5!`, `groebner!`, `diophantine!`, `rotor!`, `clifford_sandwich!`, `ext_diff!`, `hodge!`, `commutator!`, `anticommutator!`, `laplace!`, `fourier!`, `transfer_fn!`, `block_feedback!`, `block_series!`, `block_parallel!`, `prob_test!`, `certify_zero!`). |
| **Phase 12** | **Cylindrical Algebraic Decomposition (CAD) & Quantifier Elimination** | 🟡 **Partial** | Collins & McCallum projection operators, multi-variate `CadPolynomial`, sign-invariant cylindrical cell decomposition `CadCell`, Tarski-Seidenberg quantifier elimination `QuantifierEliminator` ($\forall, \exists$), 2D/3D/ND geometric constraint solver `GeometricConstraintSolver`, and ergonomic macros `cad!`, `quantifier_elim!`. |
| **Phase 13** | **Geometric CAD, B-Rep & Engineering Machinery Generators** | 🟡 **Partial** | High-efficiency BVH spatial acceleration tree `BvhTree` ($O(\log N)$ ray casting and parity point-in-mesh tests), external shape importer/exporter (`ShapeImporter`, `ShapeExporter` for OBJ, STL, SVG), parametric engineering machinery generators (`InvoluteGear` spur/helical, `ThreadedScrew` ISO/Acme, `NacaAirfoil` 4-digit wings, `HelicalSpring`, `SteppedShaft`), and DSL macros `gear!`, `screw!`, `airfoil!`, `spring!`. |
| **Phase 14** | **Multiphysics FEA / FEM Simulation Suite** | 🟡 **Partial** | $N$-Dimensional Galerkin weak form variational solver `FeaEngine`, isotropic elasticity matrix $\mathbf{C}(E, \nu)$ for 1D/2D/3D (plane stress/plane strain), von Mises stress $\sigma_v$, steady-state & transient heat transfer with convective cooling, coupled thermo-elasticity, and DSL macros `fea_elasticity!`, `fea_heat!`, `fea_solve!`. |
| **Phase 15** | **Deep Cross-Tooling Simulation Pipeline** | 🟡 **Partial** | Seamless CAD-to-FEA bridge `CadFeaBridge` (converts `Mesh3D`, `InvoluteGear`, `ThreadedScrew`, `NacaAirfoil` to simulation meshes with auto-detected boundary sets), end-to-end `SimulationPipeline`, 3D deformed CAD mesh generator $\mathbf{v}' = \mathbf{v} + \alpha \mathbf{u}$, ASCII VTK export, and `simulate_cad!` macro. |
| **Phase 16** | **Next-Gen Differential Algebra & Tensor Curvature** | 🟡 **Partial** | Differential polynomial rings $K\{y_1, \dots, y_m\}$, Ritt-Wu characteristic sets & differential reduction `RittWuReducer`, non-commutative Weyl algebra $A_1 = K\langle x, \partial \rangle$ with canonical commutator $[\partial, x] = 1$, general metric tensor `MetricTensor` with exact 2nd derivatives, Christoffel symbols $\Gamma^\sigma_{\mu\nu}$, Riemann $R^\rho{}_{\sigma\mu\nu}$, Ricci $R_{\mu\nu}$, Ricci scalar $R$, Einstein $G_{\mu\nu}$, and Kretschmann $K$, with macros `metric_curvature!`, `char_set!`. |
| **Phase 17** | **GPU Compute, 3D Complex Visualizer & Ecosystem Tooling** | ✅ **100%** | Interactive complex domain coloring `DomainColoring` (continuous phase HSV & logarithmic luminance contours), multi-sheet 3D Riemann surfaces `RiemannSurface` ($\sqrt{z}, \ln z$), RK4 vector field streamline tracer `VectorFieldVisualizer`, WebGPU WGSL compute shader emission `WgslShaderGenerator` (multi-point grids, matrix-free FEM, complex fractals), standalone zero-dependency interactive HTML dashboard exporter `StandaloneHtmlExporter`, and DSL macros `domain_color!`, `streamline_2d!`, `wgsl_grid!`. |
| **Phase 18** | **Python Wrapper & In-Memory Jupyter Controller (`urae-py` & `urae-kernel`)** | 🚧 **Foundation Ready (Wave 2 Wire/FFI)** | `urae-py`: Rust-side Python session & expression wrapper (`PyExprGraph`, `py_diff`, `py_integrate`, `py_simplify`, `py_solve`, `PyInvoluteGear`, `PyQuantumCircuit`; PyO3 `#[pymodule]` C-extension bindings scheduled for Wave 2). `urae-kernel`: In-memory Jupyter notebook cell/session & MIME-bundle execution controller (`NotebookSessionController`, `MimeBundle` with `text/latex` & `text/html`; ZeroMQ wire-protocol kernel scheduled for Wave 2). |
| **Phase 19** | **Language Server Protocol & Mathematical IDE Tooling (`urae-lsp`)** | ✅ **100%** | Dedicated LSP language server `UraeLanguageServer` for VS Code, Neovim, and Zed with real-time syntax/dimension `diagnostics`, intelligent `completions` (100+ functions, Greek symbols, CAD macros), LaTeX `hover` tooltips, and automated `code_actions` (E-Graph simplification). |
| **Phase 20** | **Symplectic Geometric Mechanics & Quantum Circuits** | ✅ **100%** | Symplectic energy-preserving Hamiltonian integrators `SymplecticIntegrator` (2nd-Order Verlet, 4th-Order Suzuki-Yoshida) preserving canonical 2-form $\omega$ over $10^7+$ steps with $O(h^4)$ bounded energy drift, and universal $N$-qubit quantum circuit simulator `QuantumCircuit` (Bell states, GHZ states, OpenQASM 2.0 export), with macros `symplectic_integrate!`, `quantum_bell_state!`, `quantum_ghz_state!`. |
| **Phase 21** | **High-Performance Automatic Differentiation, Neural ODEs & PINNs** | ✅ **100%** | Exact 1st-order Dual numbers $\mathbb{D}$ and 2nd-order Hyper-Dual numbers $\mathbb{D}_2$ for exact gradients $\nabla f(\mathbf{x})$ and Hessians $\mathbf{H} = \nabla^2 f(\mathbf{x})$ in a single pass without symbolic explosion; tape-free Vector-Jacobian Products (VJP); continuous-depth Neural ODEs (`NeuralOde`) with adjoint sensitivity backpropagation; and Physics-Informed Neural Network (PINN) PDE residual loss evaluator (`PinnResidual` for Burgers, Heat, Poisson), with macros `autodiff_forward!`, `autodiff_hessian!`, `neural_ode_step!`, `pinn_burgers_residual!`. |
| **Phase 22** | **Topological Data Analysis (TDA), Persistent Homology & Discrete Hodge** | 🟡 **Partial** | Vietoris-Rips filtration over point clouds and FEA meshes (`VietorisRipsFiltration`), boundary matrix reduction over $\mathbb{F}_2 = \operatorname{GF}(2)$ (`PersistenceDiagram`), persistence barcodes for $H_0, H_1, H_2$, persistent Betti numbers $\beta_k(\epsilon)$, and discrete Hodge 0-Laplacian / harmonic forms (`DiscreteHodge`), with macros `vietoris_rips!`, `persistence_diagram!`, `discrete_laplacian_0!`. |
| **Phase 23** | **Non-Euclidean Computational Geometry & Hyperbolic/Spherical Tessellations** | 🟡 **Partial** | Poincaré Disk model $\mathbb{D}$ (`PoincareDiskPoint`) with exact metric $ds^2$, Möbius isometries $\operatorname{PSU}(1, 1)$, and geodesic arcs; Upper Half-Plane $\mathbb{H}^2$ (`UpperHalfPlanePoint`) with $\operatorname{PSL}(2, \mathbb{R})$ and Cayley transform; Spherical Geometry $\mathbb{S}^2$ (`SphericalPoint`) with great-circle geodesics and Girard excess; Gauss-Bonnet hyperbolic triangle area defects (`HyperbolicTriangle`); regular $\{p, q\}$ Schläfli hyperbolic tessellations (`HyperbolicTessellation`); and Schwarz-Christoffel conformal mappings (`SchwarzChristoffel`), with macros `hyperbolic_dist!`, `spherical_dist!`, `hyperbolic_triangle_area!`, `spherical_triangle_area!`. |
| **Phase 24** | **SMT-LIB2 Automated Theorem Prover (ATP) Bridge & Formal Verification** | ✅ **100%** | Standard compliant SMT-LIB2 format serializer (`SmtProblem`, `SmtExpr`, `SmtSort`, `SmtLogic`) for Z3/CVC5 across `QF_NRA`, `QF_LRA`, `QF_BV`, `QF_NIA`; and constructive proof certificate generator (`ProofCertificate`) for Lean 4 theorems and Coq lemmas with automated tactics (`linarith`, `ring`, `omega`, `nlinarith`), with macros `smt_problem!`, `smt_assert!`, `proof_certificate!`. |
| **Phase 25** | **Geometric Deep Learning & $SE(3)$ Equivariance** | ✅ **100%** | Real/complex spherical harmonics $Y_{\ell m}$ (`SphericalHarmonics`), exact Clebsch-Gordan tensor product decomposition $\mathcal{D}^{(\ell_1)} \otimes \mathcal{D}^{(\ell_2)} \to \bigoplus \mathcal{D}^{(\ell)}$ (`ClebschGordan`), $SE(3)$ steerable convolutional message passing (`SteerableConvLayer`), and Clifford $Cl(3, 0)$ multivector neural layer (`CliffordLayer`), with macros `spherical_harmonic!`, `clebsch_gordan!`, `clifford_layer_forward!`. |
| **Phase 26** | **Quantum Field Theory & Feynman Diagram Amplitude Calculus** | ✅ **100%** | Recursive Dirac gamma trace engine $\operatorname{Tr}[\gamma^{\mu_1} \dots \gamma^{\mu_n}]$ and slashed 4-momenta traces (`DiracGamma`), massless Weyl spinors & angle/square brackets $\langle i j \rangle, [i j]$ (`SpinorHelicity`), Parke-Taylor tree-level MHV $n$-gluon scattering amplitudes, Passarino-Veltman 1-loop reductions $A_0, B_0, B_1$ (`PassarinoVeltman`), and $(2n-1)!!$ combinatorial Wick pairings (`WickContraction`), with macros `dirac_trace!`, `dirac_slashed_trace!`, `spinor_bracket_angle!`, `spinor_bracket_square!`, `parke_taylor_mhv!`, `passarino_veltman_b1!`. |
| **Phase 27** | **Holonomic $D$-Modules & Zeilberger Creative Telescoping** | 🟡 **Partial** | $N$-variable Weyl algebra $A_n = K\langle x_1, \dots, x_n, \partial_1, \dots, \partial_n \rangle$ with $[\partial_i, x_j] = \delta_{ij}\mathbf{I}$ and Leibniz normal form (`WeylDOperator`), left Gröbner bases & polynomial division (`WeylGrobnerBasis`), discrete creative telescoping for hypergeometric sequences $L(n, S_n) F(n, k) = (S_k - 1) G(n, k)$ (`ZeilbergerAlgorithm`), and differential creative telescoping for hyperexponential integrals $L(x, \partial_x) f(x, y) = \partial_y g(x, y)$ (`AlmkvistZeilberger`), with macros `weyl_x!`, `weyl_d!`, `weyl_commute!`, `zeilberger_binomial_proof!`, `almkvist_zeilberger_gaussian!`. |
| **Phase 28** | **Affine Kac-Moody & Virasoro Conformal Field Theory** | 🟡 **Foundation** | Virasoro Lie algebra $[L_m, L_n] = (m - n) L_{m+n} + \frac{c}{12}(m^3 - m)\delta_{m+n, 0}$ (`VirasoroAlgebra`) with exact Jacobi identity verification, minimal model Kac tables $h_{r, s}(c)$, affine loop current algebras $[J^a_m, J^b_n] = i f^{abc} J^c_{m+n} + k \, m \, \delta^{ab} \delta_{m+n, 0}$ (`KacMoodyAlgebra`), Sugawara central charge construction $c = \frac{k \dim(\mathfrak{g})}{k + h^\vee}$, and Operator Product Expansion (OPE) Laurent pole analyzer (`OperatorProductExpansion`), with macros `virasoro_bracket!`, `kac_moody_bracket!`, `sugawara_central_charge!`, `kac_conformal_weight!`. **Audit 16 Nov 2026:** `cft.rs` (142 lines) is closed-form formulas: the Virasoro bracket and Jacobi check, minimal-model weights, su(2)-only affine brackets, the Sugawara formula, and the OPE pole coefficients of T(z)T(w) and T(z)phi(w). There is no general Kac-Moody algebra, no representation theory and no correlation functions. |
| **Phase 29** | **Exceptional Lie Groups ($G_2, F_4, E_6, E_7, E_8$) & Octonionic Fano Plane Algebras** | ✅ **100%** | 8D normed alternative non-associative division algebra $\mathbb{O}$ with Fano plane multiplication matrix and associator $[x, y, z] = (xy)z - x(yz)$ (`Octonion`), 27D exceptional Jordan algebra $\mathcal{H}_3(\mathbb{O})$ with Jordan product $X \circ Y = \frac{1}{2}(XY + YX)$, trace, and Freudenthal cubic determinant (`AlbertAlgebra`), and 8D $E_8$ root lattice $\Gamma_8$ with all 240 root vectors of length $\sqrt{2}$, simple root basis, $8 \times 8$ Cartan matrix, and Weyl reflection operators (`E8Lattice`), with macros `octonion!`, `octonion_mul!`, `octonion_associator!`, `albert_det!`, `e8_weyl_reflect!`. |
| **Phase 30** | **$p$-Adic Hodge Theory & Fontaine Period Rings** | 🟡 **Foundation** | Fontaine filtered $(\Phi, N)$-modules with semilinear Frobenius $\Phi$, nilpotent monodromy $N$ ($N\Phi = p\Phi N$), and Hodge filtration $\operatorname{Fil}^\bullet D$ (`FontaineModule`), local Galois representation admissibility classification (crystalline, semistable, de Rham) and Hodge-Tate weights (`PadicGaloisRepresentation`), and cyclotomic character Tate twists $\mathbb{Q}_p(r)$ shifting weights by $-r$ and scaling Frobenius eigenvalues by $p^{-r}$ (`TateTwist`), with macros `fontaine_module!`, `tate_twist_weights!`, `tate_twist_frob!`. **Audit 16 Nov 2026:** `padic_hodge.rs` (171 lines) holds the matrices of a filtered (Phi, N)-module with a check of N Phi = p Phi N, and labels it crystalline (N = 0) or semistable (N nilpotent). Fontaine period rings (B_cris, B_dR), weak admissibility and the Galois side are not implemented. |
| **Phase 31** | **Advanced N-Dimensional CAD Topology & Isogeometric Analysis (IGA)** | ✅ **100%** | Cox-de Boor recursive B-spline basis functions and exact derivatives (`CoxDeBoor`), arbitrary $N$-dimensional NURBS patches with weighted projective control nets, surface evaluation, and exact metric tensor determinants $|\det \mathbf{J}|$ (`NurbsPatchND`), direct isogeometric analysis (IGA) stiffness matrix integration preserving CAD geometries during FEM analysis (`IgaStiffnessMatrix`), and multi-patch B-Rep topology containers (`MultiPatchBRep`), with macros `cox_de_boor_basis!`, `iga_stiffness_entry_2d!`. |

### 2.1 Scope named in the plan with no implementation

A search of `crates/` (excluding tests, docs and notebook example text) on 17 Nov 2026 found no code for the
items below. The phases marked 🟡 above are marked because of these gaps.

| Phase | Named in the spec, not found in code |
| :--- | :--- |
| 8 | Transseries |
| 12 | Vincent–Akritas–Strzeboński real-root isolation (Sturm isolation exists) |
| 13 | B-Rep topology structures and Euler operators, BSP trees, knot insertion, degree elevation, convex hull / Voronoi / Delaunay |
| 14 | Weak-form generator, Hex20/Hex27, plate theory, hyperelasticity, Navier–Stokes / Taylor–Hood / SUPG, Maxwell / Nédélec elements, h/p refinement and error estimators, CG / GMRES / BiCGSTAB / multigrid (the notebook has example titles only) |
| 15 | Automatic mesh generation |
| 16 | Meijer G and Slater's theorem, Weyl tensor, Killing-vector solver (differential pseudo-division now exists) |
| 18 | PyO3 `#[pymodule]` and ZeroMQ kernel transport (comments only; already marked Wave 2) |
| 22 | Alpha complexes |
| 23 | Beltrami–Klein model, hyperbolic Voronoi / Delaunay |
| 24 | Z3/CVC5 subprocess bridge (the SMT-LIB2 serialiser exists; nothing invokes a solver) |
| 27 | General creative telescoping (only fixed showcase proofs) |
| 30 | Period rings (B_cris, B_dR), weak admissibility |

Present but thin, so ✅ above means "exists with tests", not depth: TDA (338 lines), Weyl-algebra Gröbner
bases, curvature (Christoffel, Riemann, Ricci, Einstein), QFT (Wick pairing, spinor helicity, Dirac traces),
CFT and p-adic Hodge.

---

## 3. Resource Budgets, Soft/Hard Caps & Graceful Degradation

To support environments ranging from embedded WebAssembly to multi-terabyte HPC computing clusters, URAE employs a two-tier **Soft Cap vs. Hard Cap Architecture** managed by a dynamic `ResourceBudget`.

```mermaid
flowchart TD
    subgraph BudgetControl ["Resource Budget Lifecycle"]
        Exec["Algebraic Operation Execution"] --> Check{"Budget Check"}
        Check -->|Below Soft Cap| Normal["Normal Execution (Full Rule Set, High-Order Series)"]
        Check -->|Exceeds Soft Cap| Soft["Soft Cap Reached: Graceful Degradation<br/>(Prune expansion rules, switch to bounded basis, partial trajectories)"]
        Soft --> CheckHard{"Exceeds Hard Cap?"}
        CheckHard -->|No| Partial["Return Best Partial Result with Diagnostic Warning"]
        CheckHard -->|Yes| Hard["Hard Cap Reached: Safe Rollback<br/>(Clean AlgebraError::ResourceLimitExceeded)"]
    end
```

### 3.1 Soft vs. Hard Cap Envelopes

| Engine Subsystem | Soft Cap Behavior (Graceful Degradation) | Hard Cap Behavior (Safe Error Rollback) | Default Soft Cap | Default Hard Cap | HPC / Workstation Cap |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **E-Graph Saturation (`egg`)** | Disable expanding rules ($x \to x+0$, distributive expansions); permit only reductions and extractions. | Terminate saturation immediately and extract the current minimum-cost AST from the e-graph. | 8,000 e-nodes | 25,000 e-nodes | 250,000 e-nodes |
| **Gröbner Bases (Buchberger / F4)** | Switch to homogeneous truncated degree basis; prune S-pairs with virtual sugar degree $d > d_{\text{trunc}}$. | Stop S-pair reduction and return the current generating set with an `IncompleteBasis` diagnostic flag. | 5,000 S-pairs | 50,000 S-pairs | 1,000,000 S-pairs |
| **Numerical ODE & PDE Solvers** | Log stiffness warning; clamp minimum adaptive step size to $h_{\min}$. | Terminate integration loop and return computed partial trajectory $[0, t_{\text{last}}]$. | 2,000 steps | 20,000 steps | 500,000 steps |
| **Non-Linear Systems (Newton/Halley)** | Apply adaptive Levenberg-Marquardt damping $\lambda \leftarrow 10\lambda$ to escape local extrema. | Abort iteration and return closest approximate root with residual norm $\|\mathbf{F}(\mathbf{x})\|$. | 30 iterations | 100 iterations | 1,000 iterations |
| **Series & Limits (Taylor, Laurent, LIF)** | Truncate series at order $N_{\text{soft}}$ and append explicit asymptotic Big-O remainder $O(x^{N_{\text{soft}}})$. | Abort expansion and report singularity branch depth exceeded. | Order 12 | Order 50 | Order 250 |
| **Braid Words & Knot Skein Trees** | Apply Reidemeister I/II greedy loop removals before expanding skein binary trees. | Return bounded Laurent polynomial with unreduced crossings. | Crossing # 16 | Crossing # 32 | Crossing # 128 |

---

## 4. Delivery Schedule

The earlier Phases 1–20 schedule and its milestone table duplicated section 2 and had fallen out of date, so
they are removed. Status lives in section 2; open work, in priority order, lives in
[`docs/PLANNING.md`](docs/PLANNING.md).

---

## 5. Detailed Specifications for Strategic Phases

> **These are intended scope, not shipped scope.** Several bullets below describe features that do not exist yet;
> section 2.1 lists them and `docs/PLANNING.md` tracks the work.

### 5.1 Phase 12: Cylindrical Algebraic Decomposition (CAD) & Real Quantifier Elimination

> **Status (17 Nov 2026).** Found: Collins, McCallum and Brown projection, `CadCell`, Sturm root isolation, a quantifier-elimination entry point (`cad::QuantifierEliminator::eliminate`). Not found: VAS real-root isolation. Partial: the geometric constraint solver handles only `Distance` and `Coincident` (ledger C3).
- **Projection Operators**: Collins projection, McCallum projection, Brown augmented projection producing univariate polynomial delineating sets.
- **Real Root Isolation**: Vincent-Akritas-Strzeboński (VAS) continuous fraction isolation for exact real roots $\alpha \in (a, b)$ with minimal polynomial $p(\alpha) = 0$.
- **Cylindrical Lifting**: Constructing stack of sign-invariant cylindrical cells over $\mathbb{R}^n$.
- **Quantifier Elimination**: Evaluating $\exists x_1 \dots \exists x_k \, \Phi(x_1, \dots, x_n)$ into quantifier-free equivalent semi-algebraic conditions.
- **Parametric Geometric Constraint Solver**: Solving 2D/3D/ND geometric sketches with tangency, perpendicularity, distance, and fixed area constraints.

### 5.2 Phase 13: Geometric CAD, B-Rep & $N$-Dimensional Parametric Design

> **Status (17 Nov 2026).** Found: `BvhTree`, the `CsgBooleanOp` enum (Booleans work on implicit-surface operands only; mesh/extrusion operands are reported as unsupported), shape import/export, Cox-de Boor B-splines and NURBS patches, implicit contouring (marching tetrahedra, `Mesh3D::from_implicit`, 18 Nov). Not found: B-Rep topology structures and Euler operators, BSP trees, knot insertion, degree elevation, convex hull / Voronoi / Delaunay, Double Description / Fourier-Motzkin.
- **B-Rep Data Model**: Manifold and non-manifold solid model topologies: `Solid`, `Shell`, `Face`, `Loop`, `Edge`, `Vertex` with Euler operators ($MVFS, MEV, MEF, KEV$).
- **CSG Boolean Engine**: Exact boolean operations on polyhedral and curved solids using BSP trees and perturbation algorithms.
- **NURBS & Splines**: $B$-Spline basis function generation, Cox-de Boor recursion, rational B-spline curves/surfaces, knot insertion, degree elevation, curvature analysis.
- **Implicit Algebraic Geometry**: Dual Contouring and Marching Simplices in $\mathbb{R}^n$ for $f(x_1, \dots, x_n) = 0$.
- **$N$-Dimensional Polytope Engine**: Convex hull, Voronoi hyper-diagrams, Delaunay hyper-triangulations, Double Description Method / Fourier-Motzkin elimination.

### 5.3 Phase 14: Multiphysics FEA / FEM Simulation Suite

> **Status (17 Nov 2026).** Found: Tri6, Quad8, Tet10 and Hex8 elements, elasticity matrices, von Mises stress, a Galerkin solver (dense; ledger C10). Not found: weak-form generator, Hex20/Hex27, plates, hyperelasticity, Navier–Stokes / Taylor–Hood / SUPG, Nédélec elements, h/p refinement and error estimators, CG / GMRES / BiCGSTAB / multigrid.
- **Variational Weak Form Generator**: Automatically transforms strong-form symbolic PDEs into bilinear forms $a(u, v)$ and linear forms $L(v)$ using symbolic integration by parts and Cartan calculus.
- **Element Library in $\mathbb{R}^n$**:
  - 1D Elements: 2-node linear, 3-node quadratic, $p$-order spectral.
  - 2D Elements: 3-node triangular, 6-node quadratic triangular, 4-node quad, 8-node serendipity, 9-node biquadratic.
  - 3D Elements: 4-node tetrahedral, 10-node quadratic tet, 8-node hexahedral, 20-node serendipity hex, 27-node triquadratic hex.
  - $N$-D Elements: $N$-simplex elements with $(N+1)$ nodes, $N$-hypercube elements with $2^N$ nodes.
- **Physics Modules**:
  - **Structural Mechanics & Elasticity**: Linear & non-linear elasticity, plane stress/strain, Kirchhoff-Love & Reissner-Mindlin plates, hyperelastic materials (Mooney-Rivlin, Neo-Hookean), von Mises yield criteria.
  - **Thermal & Heat Transfer**: Steady-state and transient heat conduction, convection, radiation boundary conditions.
  - **Fluid Dynamics (CFD)**: Incompressible Navier-Stokes, Stokes flow, mixed $P_2-P_1$ Taylor-Hood elements, SUPG/PSPG streamline upwind stabilization.
  - **Electromagnetics**: Maxwell curl-curl formulations with $\mathbf{H}(\operatorname{curl})$ Nédélec edge elements.
- **Adaptive Solvers & Mesh Refinement**:
  - $h$-refinement (local cell splitting), $p$-refinement (order elevation), $hp$-adaptive strategy guided by Zienkiewicz-Zhu energy norm error estimators.
  - Conjugate Gradient (CG), GMRES, Preconditioned BiCGSTAB with Incomplete Cholesky / Algebraic Multigrid (AMG) preconditioners.

### 5.4 Phase 15: Cross-Tooling Integration Pipeline

> **Status (17 Nov 2026).** Found: `CadFeaBridge` (CAD solids to simulation meshes). Not found: automatic mesh generation (step 2) and a symbolic variational engine producing K and F (step 4). Steps 6 and 7 (60 FPS reactive 3D stress view) are not verified.
- **Unified Parametric Simulation Workflow**:
  1. Define parametric CAD geometry in the reactive notepad (e.g. `bracket = cylinder(r=r1, h=h1) - cylinder(r=r2, h=h1)`).
  2. Automatic mesh generator produces boundary-conforming simplicial mesh in background thread.
  3. User declares physics equation: `stress_analysis(bracket, load = F_load [N], material = StructuralSteel)`.
  4. Symbolic variational engine computes stiffness matrix $\mathbf{K}$ and load vector $\mathbf{F}$.
  5. FEA solver evaluates displacement $\mathbf{u}$ and stress tensor field $\sigma$.
  6. Right gutter of the notebook displays interactive 3D WebGPU canvas with von Mises stress heatmap and deformation animation.
  7. Modifying parameter scrubber $r1$ or $F_{\text{load}}$ instantly recalculates and updates the 3D visualization reactively at 60 FPS.

### 5.5 Phase 16: Next-Gen Differential Algebra & Tensor Curvature

> **Status (17 Nov 2026).** Found: `RittWuReducer` with differential pseudo-division and Ritt's characteristic-set procedure, metric, Christoffel, Riemann, Ricci and Einstein tensors. Not found: Meijer G and Slater's theorem, Killing-vector solver, Weyl tensor. The Zeilberger bullet is a fixed set of showcase proofs (ledger A8).
- **Ritt-Wu Differential Algebra**: Characteristic sets of differential polynomial ideals, pseudodivision with respect to derivative leaders, checking differential algebraic variety inclusion.
- **Meijer $G$-Function Definite Integration**: Unified definite integration engine using Slater's theorem and contour Mellin transforms for special function integration.
- **Einstein Summation & General Relativity**:
  - Arbitrary metric $g_{\mu\nu}$ specification $\to$ metric inverse $g^{\mu\nu}$, Christoffel symbols $\Gamma^\sigma_{\mu\nu} = \frac{1}{2} g^{\sigma\lambda} (\partial_\mu g_{\nu\lambda} + \partial_\nu g_{\mu\lambda} - \partial_\lambda g_{\mu\nu})$.
  - Riemann curvature tensor $R^\rho_{\sigma\mu\nu} = \partial_\mu \Gamma^\rho_{\nu\sigma} - \partial_\nu \Gamma^\rho_{\mu\sigma} + \Gamma^\rho_{\mu\lambda} \Gamma^\lambda_{\nu\sigma} - \Gamma^\rho_{\nu\lambda} \Gamma^\lambda_{\mu\sigma}$.
  - Ricci curvature tensor $R_{\mu\nu} = R^\lambda_{\mu\lambda\nu}$, Ricci scalar $R = g^{\mu\nu} R_{\mu\nu}$, Einstein tensor $G_{\mu\nu} = R_{\mu\nu} - \frac{1}{2} R g_{\mu\nu}$, Weyl tensor $C_{\rho\sigma\mu\nu}$.
  - Automatic Killing vector solver: $\nabla_\mu \xi_\nu + \nabla_\nu \xi_\mu = 0$.
- **Zeilberger Algorithm**: Creative telescoping for holonomic $D$-modules proving $\sum_k F(n, k) = G(n)$.

### 5.6 Phase 17 & 18: GPU Compute, 3D Complex Visualizer & Ecosystem Tooling

> **Status (17 Nov 2026).** Found: WGSL/`wgpu` code, `DomainColoring`, Riemann surfaces, `urae-lsp`. Rayon is a dependency used for CRT verification only, not a parallel e-graph. PyO3 `#[pymodule]` and ZeroMQ are not implemented (Wave 2, as the bullets say). Standalone offline HTML/WASM export is not verified.
- **WebGPU & Vulkan Acceleration**: Compute shaders for matrix-free element stiffness integration and multi-point batch evaluation over $10^8$ vertices.
- **Parallel Lock-Free E-Graph Saturation**: Multi-core Rayon equality saturation with lock-free union-find tables.
- **3D Riemann Surface & Domain Coloring**: Interactive multi-sheet rendering for $w = \sqrt[n]{z}$ and $w = \ln(z)$, with HSV phase portraits.
- **Ecosystem Bridges**:
  - `urae-py`: Rust-side Python session & expression wrapper (PyO3 `#[pymodule]` C-extension bindings scheduled for Wave 2).
  - `urae-kernel`: In-memory Jupyter notebook cell/session & MIME-bundle execution controller (ZeroMQ wire-protocol kernel scheduled for Wave 2).
  - `urae-lsp`: Language Server Protocol implementation providing syntax highlighting, KaTeX hover cards, and dimensional unit validation in IDEs.
  - Self-contained standalone HTML/WASM export bundling the complete reactive notebook offline.

### 5.7 Phase 21: High-Performance Automatic Differentiation (AD), Neural ODEs & Physics-Informed ML (PINNs)

> **Status (17 Nov 2026).** Found: dual and hyper-dual numbers, VJP/JVP, Neural ODE, adjoint sensitivity, PINN residuals and the four macros. The adjoint is simplified (ledger C6).
- **Arbitrary-Order Forward-Mode AD**:
  - Hyper-Dual numbers $\mathbb{D}_k = a + b\,\epsilon_1 + c\,\epsilon_2 + d\,\epsilon_1\epsilon_2$ with $\epsilon_i^2 = 0$ for exact Jacobian and Hessian $\mathbf{H} = \nabla^2 f$ matrices in a single forward pass without symbolic explosion.
- **High-Throughput Reverse-Mode AD (VJP / JVP)**:
  - Tape-free reverse-mode automatic differentiation with vector-Jacobian products $\mathbf{v}^T \mathbf{J}$ utilizing reusable execution arenas.
- **Continuous-Depth Neural Ordinary Differential Equations (Neural ODEs)**:
  - Dynamic vector fields $\frac{d\mathbf{x}}{dt} = f_\theta(\mathbf{x}, t)$ integrated with URAE's adaptive DOPRI5 and symplectic Verlet solvers.
  - Continuous Adjoint Sensitivity Method: solving the augmented adjoint system $\frac{d\mathbf{a}}{dt} = -\mathbf{a}(t)^T \frac{\partial f}{\partial \mathbf{x}}$ backwards in time with $O(1)$ memory backpropagation.
- **Physics-Informed Neural Networks (PINNs)**:
  - Formulation of residual loss functions $\mathcal{L}_{\text{PINN}} = \|\partial_t u + u \partial_x u - \nu \partial_{xx} u\|^2$ directly composed with URAE's symbolic differential operators.
- **Ergonomic Macros**: `autodiff_forward!`, `autodiff_hessian!`, `neural_ode_step!`, `pinn_loss!`.

### 5.8 Phase 22: Topological Data Analysis (TDA), Persistent Homology & Hodge Cohomology

> **Status (17 Nov 2026).** Found: Vietoris–Rips filtration, persistence barcodes, Betti numbers, discrete Hodge Laplacian (338 lines in total; depth not audited). Not found: alpha complexes.
- **Simplicial Filtrations**:
  - Vietoris-Rips and Alpha complexes constructed over point clouds, FEA mesh nodes, and molecular structures.
- **Galois Field Boundary Reduction**:
  - Boundary matrix $\partial_k$ reduction algorithm over $\mathbb{F}_2 = \operatorname{GF}(2)$ computing persistence intervals $(\text{birth}, \text{death})$.
  - Persistence barcodes and Betti numbers $H_0$ (connected components), $H_1$ (loops/tunnels), and $H_2$ (voids/cavities).
- **Discrete de Rham Cohomology & Hodge Decomposition**:
  - Computational de Rham cohomology groups $H_{\text{dR}}^k(M)$ and harmonic differential forms ($\Delta \omega = 0$) using Hodge decomposition $\omega = \mathrm{d}\alpha + \delta\beta + \gamma$.

### 5.9 Phase 23: Non-Euclidean Computational Geometry (Hyperbolic & Spherical)

> **Status (17 Nov 2026).** Found: Poincaré disk, upper half-plane, {p,q} tessellations, Schwarz–Christoffel. Not found: Beltrami–Klein model, hyperbolic Voronoi / Delaunay.
- **Hyperbolic Geometry Models**:
  - Poincaré disk, Beltrami-Klein, and Upper Half-Plane $\mathbb{H}^2, \mathbb{H}^3$ with exact geodesic distance metrics:
    $$d(u, v) = \operatorname{arcosh}\left(1 + 2\frac{\|u - v\|^2}{(1 - \|u\|^2)(1 - \|v\|^2)}\right)$$
  - Hyperbolic Voronoi diagrams, Delaunay triangulations, and $\{p, q\}$ regular non-Euclidean tessellations (e.g. $\{7, 3\}, \{5, 4\}$).
- **Conformal Mapping**:
  - Schwarz-Christoffel transformation engine for conformal mapping of polygonal domains to canonical discs.

### 5.10 Phase 24: SMT-LIB2 Automated Theorem Prover (ATP) Bridge & Formal Verification

> **Status (17 Nov 2026).** Found: SMT-LIB2 serialiser (`SmtProblem` and friends). Not found: any call to Z3 or CVC5. Lean 4 / Coq export is a fixed skeleton ending in `sorry` (ledger A9), so the "verified proof scripts" bullet is not met.
- **SMT-LIB2 Solver Interface**:
  - Translation of URAE expressions to SMT-LIB2 format for Quantifier-Free Non-Linear Real Arithmetic (`QF_NRA`), Bit-Vectors (`QF_BV`), and Arrays.
  - Subprocess / WASM bridge for Z3 / CVC5 satisfiability checks and model/counterexample extraction.
- **Certified Proof Certificates**:
  - Constructive proof generator outputting verified Lean 4 and Coq proof scripts for all algebraic transformations and simplifications.

### 5.11 Phase 25: Geometric Deep Learning & $SE(3)$ Equivariance

> **Status (17 Nov 2026).** Found: spherical harmonics, Clebsch–Gordan, steerable convolutions, GELU layers (depth not audited).
- **Spherical Harmonics & Clebsch-Gordan**:
  - Real and complex spherical harmonics $Y_{\ell m}$ and exact Clebsch-Gordan coefficients $\langle \ell_1 m_1 \ell_2 m_2 | \ell m \rangle$.
- **$SE(3)$ Steerable Convolutions**:
  - Angular filter decomposition $W(\mathbf{r}) = R(\|\mathbf{r}\|) \sum Y_{\ell m}(\hat{\mathbf{r}}) \mathbf{C}$ for point clouds and meshes.
- **Clifford Multivector Layers**:
  - Geometric algebra $Cl(3, 0)$ multivector neural networks with grade-preserving non-linear activations (GELU).

### 5.12 Phase 26: Quantum Field Theory & Feynman Diagram Amplitude Calculus

> **Status (17 Nov 2026).** Found: Wick pairing, Feynman diagram generation, Passarino–Veltman, spinor helicity, Dirac traces (356 lines in `qft.rs`; depth not audited).
- **Perturbative QFT & Wick Contractions**:
  - Symbolic Wick contraction engine and Feynman diagram generation.
- **Passarino-Veltman 1-Loop Reduction**:
  - Reduction of 1-loop tensor integrals to standard scalar master integrals ($A_0, B_0, C_0, D_0$).
- **Spinor Helicity Formalism & Dirac Traces**:
  - Angle/square spinor brackets $\langle i j \rangle, [i j]$ and Dirac gamma matrix trace evaluator $\operatorname{Tr}[\gamma^\mu \gamma^\nu \dots]$ in $D$ spacetime dimensions.

### 5.13 Phase 27: Holonomic $D$-Modules & Zeilberger Creative Telescoping

> **Status (17 Nov 2026).** Found: Weyl algebra and left-ideal Gröbner basis computation. Creative telescoping is three fixed showcase proofs, not an algorithm (ledger A5, A8).
- **Non-commutative Weyl Algebras**:
  - Gröbner bases for left ideals in $A_n = K\langle x_1, \dots, x_n, \partial_1, \dots, \partial_n \rangle$.
- **Creative Telescoping**:
  - Automated proofs of hypergeometric summation identities $\sum_k F(n, k) = G(n)$ and definite integrals.

### 5.14 Phase 28: Affine Kac-Moody & Virasoro Conformal Field Theory

> **Status (17 Nov 2026).** Found: Virasoro algebra, Kac–Moody, OPE, Sugawara (`cft.rs`, 142 lines; marked Foundation in section 2).
- **Virasoro Algebra**:
  - Central charge $c$ Lie algebra bracket $[L_m, L_n] = (m - n)L_{m+n} + \frac{c}{12}(m^3 - m)\delta_{m+n, 0}$.
- **Kac-Moody & OPEs**:
  - Affine current algebras $\hat{\mathfrak{g}}$, Operator Product Expansions, and Sugawara energy-momentum tensors.

### 5.15 Phase 29: Universal `cargo-ndk` Android Mobile Deployment [COMPLETE]
- [x] Multi-target cross-compilation toolchain for `aarch64-linux-android`, `armv7-linux-androideabi`, `x86_64-linux-android`.
- [x] Dual `cdylib` / `rlib` targets in `crates/urae-notebook` and `crates/urae-ffi`.
- [x] Native activity entry point `android_main` in `crates/urae-notebook/src/lib.rs`.
- [x] Automated compilation scripts (`scripts/build-android.ps1`, `scripts/build-android.sh`).
