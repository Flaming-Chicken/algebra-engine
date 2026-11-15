pub use algebra_core::{
    config::{BudgetStatus, EngineConfig, ResourceBudget},
    domain::{Domain, DomainBound, DomainCategory, GoalDomain, MathContext},
    error::{AlgebraError, AlgebraResult},
    expr::{ExprKind, ExprNode, RelOp, SetOp},
    format::{FormatError, FormatResult, Formatter, LatexFormatter, UnicodeFormatter},
    graph::ExprGraph,
    id::{DomainId, ExprId, SymbolId},
    interval::RealInterval,
    number::{Constant, Number},
    numbers::{DualNumber, HyperrealNumber, Octonion, PAdicNumber, Quaternion, SurrealNumber},
    operation::{
        CombinatoricsOpKind, ComparisonKind, ControlOpKind, GaloisOpKind, HyperOpKind,
        IntervalOpKind, LogicOpKind, MathOperation, MatrixOpKind, NumberTheoryOpKind,
        PhysicsOpKind, PolynomialOpKind, ProofTarget, SimplifyStrategy, StatMechOpKind,
        SymbolRoleKind, SystemCommandKind, TopologyOpKind, TransformOpKind, TropicalOpKind,
    },
    parser::{
        ExprParser, PermissiveIntent, normalize_latex_input, parse_domain_declaration,
        parse_operation, parse_permissive_intent,
    },
    probabilistic::{ProbabilisticVerifier, Solution},
    step::{MathStep, StepByStepResult},
    symbol::SymbolTable,
    traits::{
        AbelianGroup, AlternativeAlgebra, CommutativeRing, DifferentialRing, Field, Group,
        HeytingAlgebra, HopfAlgebra, JordanAlgebra, LieAlgebra, Magma, MathSet, Monoid, NearRing,
        Ring, Semigroup, Semiring, TensorAlgebra, TropicalSemiring, VectorSpace,
    },
    vm::{BytecodeProgram, BytecodeVM, Instruction, VmCompiler, VmError},
};

pub use auto_promoters::uint::Uint;
pub use cat_numbers::Magnitude;

pub use algebra_engine::{
    analysis::{
        ClassifiedZero, CriticalPoint1D, CriticalPointKind, CriticalPointND, FunctionAnalyzer,
        ZeroType,
    },
    autodiff::{Dual, ForwardGradient, HyperDual, TapeFreeVjp},
    branch::{
        BranchCutFunctionKind, BranchCutGeometry, BranchCutManager, BranchSelection,
        MultiValuedResult,
    },
    cad::{
        BoltHeadType, BvhIntersection, BvhNode, BvhTree, CadCell, CadCellType, CadEngine,
        CadPolynomial, CsgBooleanOp, CustomShape, GeometricConstraint, GeometricConstraintSolver,
        GeometricEntity, HelicalSpring, InvoluteGear, Mesh3D, NacaAirfoil, Quantifier,
        QuantifierEliminator, Ray3D, RelationalOp, ShapeExporter, ShapeImporter, Sign,
        ThreadStandard, ThreadedScrew,
    },
    calculus::{FractionalCalculus, SymbolicCalculus, VariationalCalculus},
    cartan::{DifferentialForm, FormBasis, VectorField},
    cft::{KacMoodyAlgebra, OperatorProductExpansion, VirasoroAlgebra},
    combinatorics::{
        bell_number, catalan_number, combinations, derangements, factorial, integer_partitions,
        permutations, stirling_second_kind,
    },
    control::StateSpaceSystem,
    curvature::{CurvatureAnalysis, MetricTensor},
    diffalg::{DiffIndeterminate, DiffPolynomial, DiffTerm, RittWuReducer, WeylOperator},
    distributions::EmpiricalDistributionMut,
    exceptional_lie::{AlbertAlgebra, E8Lattice, LieOctonion},
    executor::{ExecutionContext, OperationExecutor, OperationResult},
    geometric_dl::{ClebschGordan, CliffordLayer, SphericalHarmonics, SteerableConvLayer},
    geometry::{Circle2D, CoordinateSystem, CsgNode, Point2D, ResultantBuilder},
    gpu::{GpuKernelDescriptor, WgslShaderGenerator},
    hecs::Hecs,
    heuristic::{HeuristicSearchEngine, SearchCandidate},
    html_export::StandaloneHtmlExporter,
    hyperop::{ackermann, hyperoperation, knuth_up_arrow, pentation, super_log, tetration},
    iga::{CoxDeBoor, IgaStiffnessMatrix, MultiPatchBRep, NurbsPatchND, WeightedControlPoint},
    interconnect::UniversalBridge,
    lagrange::{InversionSeriesResult, LagrangeBurmann},
    lifting::{
        DomainCompatibilityChecker, DomainLiftRouter, EulerLiftingFunctor, LiftingFunctor,
        RetractionCleaner, WeierstrassLiftingFunctor,
    },
    limits::{LimitDirection, LimitEngine},
    logic::{
        AssumptionsContext, GeneralizedValuedLogic, SatSolver, ThreeValuedLogic, ThreeValuedSystem,
        ThreeValuedValue,
    },
    matrix::SymbolicMatrix,
    neural_ode::{AdjointSensitivitySolver, NeuralOde, NeuralOdeTrajectory},
    noneuclidean::{
        HyperbolicTessellation, HyperbolicTriangle, PoincareDiskPoint, SchwarzChristoffel,
        SphericalPoint, UpperHalfPlanePoint,
    },
    numbertheory::{ContinuedFraction, extended_gcd, is_prime, legendre_symbol},
    numeric::{BigValue, EvalContext, NumericalEval},
    ode::{
        ButcherTableau, NumericalOdeConfig, NumericalOdeMethod, NumericalOdeSolver, OdeTrajectory,
        OdeType, SymbolicOdeSolver,
    },
    padic_hodge::{FontaineModule, PadicGaloisRepresentation, TateTwist},
    pde::{
        BoundaryCondition, NumericalPdeSolver, PdeClassification, PdeGridSolution,
        SymbolicPdeSolver,
    },
    physics::{Constants, Dimensions, FourVector, HamiltonianSystem, LagrangianSystem, Quantity},
    pinn::PinnResidual,
    poly::{
        BringRadical, CardanoSolver, ComplexRoot, DurandKernerSolver, FerrariSolver, GrobnerBasis,
        MonomialOrder, MultiPoly, Polynomial, SturmSequence, Term,
    },
    qft::{DiracGamma, PassarinoVeltman, SpinorHelicity, WeylSpinor, WickContraction},
    quantum_circuit::{Complex64, QuantumCircuit, QuantumGate},
    risch::{DifferentialFieldTower, ExtensionKind, HermiteReductionResult, RischIntegrator},
    series::SymbolicSeries,
    session::{SymbolSessionMeta, UraeSession},
    sets::{ClassType, InfiniteSetKind, IntSetIterator, MathSetRepresentation},
    simplify::{
        AlgebraicTransformations, DetectedFeatures, DomainFeatureDetector, DynamicRuleSelector,
        SchwartzZippel, Simplifier,
    },
    smt::{ProofCertificate, SmtExpr, SmtLogic, SmtProblem, SmtSort},
    solver::{SolveOutcome, SymbolicSolver},
    sparse::CsrMatrix,
    special::SpecialFunctions,
    symplectic::{
        HamiltonianSystemND, HarmonicOscillator, KeplerOrbit2D, PhaseState, SymplecticIntegrator,
        SymplecticOrder, SymplecticTrajectory,
    },
    systems::{
        DifferentialSystemSolver, DiophantineSolution, DiophantineSystemSolver,
        LinearSolutionSpace, LinearSystemSolver, NumericalSystemConfig, NumericalSystemSolver,
        PolynomialSystemSolver, SmithNormalForm,
    },
    tda::{
        DiscreteHodge, PersistenceDiagram, PersistenceInterval, Simplex, VietorisRipsFiltration,
    },
    tensor::SymbolicTensor,
    topology::SimplicialComplex,
    transforms::SymbolicTransforms,
    tropical::{MaxPlus, MinPlus, TropicalMatrix, log_sum_exp},
    urscript_pose::Pose,
    weyl_dmodules::{
        AlmkvistZeilberger, WeylDOperator, WeylGrobnerBasis, WeylMonomial, WeylTerm,
        ZeilbergerAlgorithm, ZeilbergerResult,
    },
};

// Re-export all CAS DSL macros from algebra_engine
pub use algebra_engine::{
    airfoil, albert_det, almkvist_zeilberger_gaussian, anticommutator, autodiff_forward,
    autodiff_hessian, block_feedback, block_parallel, block_series, certify_zero, char_set,
    clebsch_gordan, clifford_layer_forward, clifford_sandwich, commutator, cox_de_boor_basis, curl,
    diff, diophantine, dirac_slashed_trace, dirac_trace, discrete_laplacian_0, div, domain_color,
    dopri5, e8_weyl_reflect, ext_diff, fea_elasticity, fea_heat, fea_solve, fontaine_module,
    fourier, gear, grad, groebner, hessian, hodge, hyperbolic_dist, hyperbolic_triangle_area,
    iga_stiffness_entry_2d, integrate, jacobian, kac_conformal_weight, kac_moody_bracket, laplace,
    laplacian, laurent, limit, metric_curvature, neural_ode_step, octonion, octonion_associator,
    octonion_mul, ode_linear, pade, parke_taylor_mhv, passarino_veltman_b1, pde_heat_1d,
    persistence_diagram, pinn_burgers_residual, prob_test, proof_certificate, puiseux,
    quantifier_elim, quantum_bell_state, quantum_ghz_state, radau5, risch, rotor, screw,
    simplify_certified, simulate_cad, smt_assert, smt_problem, solve, solve_certified,
    spherical_dist, spherical_harmonic, spherical_triangle_area, spinor_bracket_angle,
    spinor_bracket_square, spring, streamline_2d, sugawara_central_charge, symplectic_integrate,
    tate_twist_frob, tate_twist_weights, taylor, transfer_fn, verlet, vietoris_rips,
    virasoro_bracket, weyl_commute, weyl_d, weyl_x, wgsl_grid, zeilberger_binomial_proof,
};

#[cfg(feature = "advanced")]
pub use algebra_advanced::{
    adele::{Adele, ArtinProduct, Idele, Place},
    clifford::{BladeMask, CliffordMultivector, CliffordSignature, FreeTensor},
    codegen::{CodeGenerator, TargetLanguage},
    crypto::{EllipticCurve, EllipticPoint, InformationTheory, ReedSolomonCode},
    finite::{GaloisField, ModuloInt},
    galois::{FieldExtension, GaloisGroupType},
    gpu::{GpuComputeKernel, SimdPolyVector},
    knot::{BraidGenerator, BraidWord, KnotInvariants, LaurentPoly},
    morphism::{DomainMorphism, EquivalenceClassifier, EquivalenceKind, Functor, MonoidalCategory},
    padic::{HenselLifter, PadicNumber},
    proof::{ProofStep, ProofTrace, UnificationEngine},
    quantum::{BraKet, DysonSeries, GaugeTheory, QuantumOperator},
    statmech::CanonicalPartitionFunction,
    stochastic::{ItoProcess, WienerProcess},
    structure_tensor::StructureTensorAlgebra,
    surreal::SurrealNo,
    transcendents::{HeunEquation, PainleveEquation, PainleveKind},
};

#[cfg(all(feature = "advanced", feature = "agent"))]
pub use algebra_advanced::{
    AgentToolDefinition, AiConfig, AiProvider, AlgebraAgentInterface, ExplanationGenerator,
};
