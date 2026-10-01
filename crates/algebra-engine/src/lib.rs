//! # `algebra-engine`
//!
//! Computational Core of the Universal Rust Algebra Engine (URAE).
//!
//! Unifies formal algebraic manipulation, analytical and discrete calculus,
//! multilinear and tensor algebra, E-Graph simplification, equation solving,
//! multi-variate systems of equations, automatic domain lifting & retraction,
//! ODE & PDE solvers (symbolic and adaptive numerical), physics dimensional analysis,
//! combinatorics, topology, control systems, number theory, esoteric operators,
//! and centralized operation execution.

pub mod analysis;
pub mod autodiff;
pub mod branch;
pub mod cad;
pub mod calculus;
pub mod cartan;
pub mod cft;
pub mod combinatorics;
pub mod control;
pub mod curvature;
pub mod diffalg;
pub mod diffeq;
pub mod distributions;
pub mod exceptional_lie;
pub mod executor;
pub mod geometric_dl;
pub mod geometry;
pub mod gpu;
pub mod heuristic;
pub mod html_export;
pub mod hyperop;
pub mod iga;
pub mod interconnect;
pub mod lagrange;
pub mod lifting;
pub mod limits;
pub mod logic;
pub mod macros;
pub mod matrix;
pub mod neural_ode;
pub mod noneuclidean;
pub mod numbertheory;
pub mod numeric;
pub mod ode;
pub mod padic_hodge;
pub mod pde;
pub mod physics;
pub mod pinn;
pub mod pipeline;
pub mod poly;
pub mod qft;
pub mod quantum_circuit;
pub mod risch;
pub mod series;
pub mod session;
pub mod sets;
pub mod simplify;
pub mod smt;
pub mod solver;
pub mod sparse;
pub mod special;
pub mod symplectic;
pub mod systems;
pub mod tda;
pub mod tensor;
pub mod topology;
pub mod transforms;
pub mod tropical;
pub mod visualizer;
pub mod weyl_dmodules;

pub use analysis::{
    ClassifiedZero, CriticalPoint1D, CriticalPointKind, CriticalPointND, FunctionAnalyzer, ZeroType,
};
pub use autodiff::{Dual, ForwardGradient, HyperDual, TapeFreeVjp};
pub use branch::{
    BranchCutFunctionKind, BranchCutGeometry, BranchCutManager, BranchSelection, MultiValuedResult,
};
pub use cad::{
    BoltHeadType, BoundingBox3D, BvhIntersection, BvhNode, BvhTree, CadCell, CadCellType,
    CadEngine, CadPolynomial, CsgBooleanOp, CustomShape, GeometricConstraint,
    GeometricConstraintSolver, GeometricEntity, HelicalSpring, InvoluteGear, Mesh3D, NacaAirfoil,
    Quantifier, QuantifierEliminator, Ray3D, RelationalOp, ShapeExporter, ShapeImporter, Sign,
    ThreadStandard, ThreadedScrew,
};
pub use calculus::{FractionalCalculus, SymbolicCalculus, VariationalCalculus};
pub use cartan::{DifferentialForm, FormBasis, VectorField};
pub use cft::{KacMoodyAlgebra, OperatorProductExpansion, VirasoroAlgebra};
pub use combinatorics::{
    bell_number, catalan_number, combinations, derangements, factorial, integer_partitions,
    permutations, stirling_second_kind,
};
pub use control::{StateSpaceSystem, TimeDomain};
pub use curvature::{CurvatureAnalysis, MetricTensor};
pub use diffalg::{DiffIndeterminate, DiffPolynomial, DiffTerm, RittWuReducer, WeylOperator};
pub use diffeq::{DiffEqClassifier, DiffEqDescriptor, DiffEqKind, DiffEqLinearity};
pub use distributions::EmpiricalDistributionMut;
pub use exceptional_lie::{AlbertAlgebra, E8Lattice, Octonion};
pub use executor::{ExecutionContext, OperationExecutor, OperationResult};
pub use geometric_dl::{ClebschGordan, CliffordLayer, SphericalHarmonics, SteerableConvLayer};
pub use geometry::{Circle2D, CoordinateSystem, CsgNode, Point2D, ResultantBuilder};
pub use gpu::{GpuKernelDescriptor, WgslShaderGenerator};
pub use heuristic::{
    CompositeSearchHeuristic, HeuristicSearchEngine, NeuralExpressionEmbedder,
    SchwartzZippelHeuristic, SearchCandidate, SearchHeuristic, TreeSizeHeuristic,
};
pub use html_export::StandaloneHtmlExporter;
pub use hyperop::{ackermann, hyperoperation, knuth_up_arrow, pentation, super_log, tetration};
pub use iga::{CoxDeBoor, IgaStiffnessMatrix, MultiPatchBRep, NurbsPatchND, WeightedControlPoint};
pub use interconnect::UniversalBridge;
pub use lagrange::{InversionSeriesResult, LagrangeBurmann};
pub use lifting::{
    DomainCompatibilityChecker, DomainLiftRouter, EulerLiftingFunctor, LiftingFunctor,
    RetractionCleaner, WeierstrassLiftingFunctor,
};
pub use limits::{LimitDirection, LimitEngine};
pub use logic::{AssumptionsContext, GeneralizedValuedLogic, SatSolver, ThreeValuedLogic};
pub use matrix::SymbolicMatrix;
pub use neural_ode::{AdjointSensitivitySolver, NeuralOde, NeuralOdeTrajectory};
pub use noneuclidean::{
    HyperbolicTessellation, HyperbolicTriangle, PoincareDiskPoint, SchwarzChristoffel,
    SphericalPoint, UpperHalfPlanePoint,
};
pub use numbertheory::{ContinuedFraction, extended_gcd, is_prime, legendre_symbol};
pub use numeric::{BigValue, EvalContext, NumericalEval};
pub use ode::{
    ButcherTableau, NumericalOdeConfig, NumericalOdeMethod, NumericalOdeSolver, OdeTrajectory,
    OdeType, SymbolicOdeSolver,
};
pub use padic_hodge::{FontaineModule, PadicGaloisRepresentation, TateTwist};
pub use pde::{
    BoundaryCondition, ElementType, FeaBoundaryCondition, FeaEngine, FeaMesh, FeaSolution,
    NumericalPdeSolver, PdeClassification, PdeGridSolution, PdeTravelingWaveReduction,
    PhysicsDiscipline, SymbolicPdeSolver,
};
pub use physics::{
    Constants, Dimensions, FourVector, HamiltonianSystem, LagrangianSystem, Quantity,
};
pub use pinn::PinnResidual;
pub use pipeline::{CadFeaBridge, SimulationPipeline, SimulationResult};
pub use poly::{
    BringRadical, CardanoSolver, ComplexRoot, DurandKernerSolver, FerrariSolver, GrobnerBasis,
    MonomialOrder, MultiPoly, Polynomial, SturmSequence, Term,
};
pub use qft::{DiracGamma, PassarinoVeltman, SpinorHelicity, WeylSpinor, WickContraction};
pub use quantum_circuit::{Complex64, QuantumCircuit, QuantumGate};
pub use risch::{DifferentialFieldTower, ExtensionKind, HermiteReductionResult, RischIntegrator};
pub use series::SymbolicSeries;
pub use session::{SymbolSessionMeta, UraeSession};
pub use sets::MathSetRepresentation;
pub use simplify::{SchwartzZippel, Simplifier};
pub use smt::{ProofCertificate, SmtExpr, SmtLogic, SmtProblem, SmtSort};
pub use solver::SymbolicSolver;
pub use sparse::CsrMatrix;
pub use special::SpecialFunctions;
pub use symplectic::{
    HamiltonianSystemND, HarmonicOscillator, KeplerOrbit2D, PhaseState, SymplecticIntegrator,
    SymplecticOrder, SymplecticTrajectory,
};
pub use systems::{
    DifferentialSystemSolver, DiophantineSolution, DiophantineSystemSolver, LinearSolutionSpace,
    LinearSystemSolver, NumericalSystemConfig, NumericalSystemSolver, PolynomialSystemSolver,
    SmithNormalForm,
};
pub use tda::{
    DiscreteHodge, PersistenceDiagram, PersistenceInterval, Simplex, VietorisRipsFiltration,
};
pub use tensor::SymbolicTensor;
pub use topology::SimplicialComplex;
pub use transforms::SymbolicTransforms;
pub use tropical::{MaxPlus, MinPlus, TropicalMatrix, log_sum_exp};
pub use visualizer::{ColorRgba, DomainColoring, RiemannSurface, VectorFieldVisualizer};
pub use weyl_dmodules::{
    AlmkvistZeilberger, WeylDOperator, WeylGrobnerBasis, WeylMonomial, WeylTerm,
    ZeilbergerAlgorithm, ZeilbergerResult,
};
