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
pub mod api;
pub mod autodiff;
pub mod bigfloat;
pub mod branch;
pub mod cad;
pub mod calculus;
pub mod cartan;
pub mod cartan_symbolic;
pub mod cat_eval;
pub mod cft;
pub mod combinatorics;
pub mod complex_exact;
pub mod control;
pub mod curvature;
pub mod diffalg;
pub mod diffeq;
pub mod discrete;
pub mod distributions;
pub mod exceptional_lie;
pub mod executor;
pub mod geometric_dl;
pub mod geometry;
pub mod gosper;
pub mod gpu;
pub mod guess;
pub mod hecs;
pub mod heuristic;
pub mod html_export;
pub mod hyperop;
pub mod iga;
pub mod inequality;
pub mod interconnect;
pub mod intlist;
pub mod lagrange;
pub mod lemma_miner;
pub mod lifting;
pub mod limit_check;
pub mod limits;
pub mod linprog;
pub mod logic;
pub mod macros;
pub mod matrix;
pub mod mcts;
pub mod neural_ode;
pub mod noneuclidean;
pub mod numbertheory;
pub mod numeric;
pub mod numeric_commands;
pub mod ode;
pub mod ode_ic;
pub mod padic_hodge;
pub mod pde;
pub mod physics;
pub mod pinn;
pub mod pipeline;
pub mod poly;
pub mod qft;
pub mod quantum_circuit;
pub mod query;
pub mod radical;
mod rational_pf;
pub mod risch;
mod risch_exptrig;
mod risch_more;
pub mod series;
mod series_fast;
pub mod session;
pub mod sets;
pub mod simplify;
pub mod smt;
pub mod solver;
pub mod sparse;
pub mod special;
pub mod special_exact;
pub mod stream;
pub mod symplectic;
pub mod systems;
pub mod tda;
pub mod tensor;
pub mod topology;
pub mod transforms;
pub mod tropical;
pub mod urscript_pose;
pub mod vectorcalc;
pub mod visualizer;
pub mod weyl_dmodules;
pub mod workbench;
pub mod zeilberger;
pub mod zeilberger_proof;

pub use analysis::{
    ClassifiedZero, CriticalPoint1D, CriticalPointKind, CriticalPointND, FunctionAnalyzer, ZeroType,
};
pub use api::{ConstPoly, ConstRational, Expr, IntoExpr, NumMat, Sym, SymMat, parse_expr, sym};
pub use autodiff::{Dual, ForwardGradient, HyperDual, TapeFreeVjp};
pub use branch::{
    BranchCutFunctionKind, BranchCutGeometry, BranchCutManager, BranchSelection, MultiValuedResult,
};
pub use cad::{
    BoltHeadType, BoundingBox3D, BvhIntersection, BvhNode, BvhTree, CadCell, CadCellType,
    CadEngine, CadPolynomial, CsgBooleanOp, CustomShape, CycloidalGear, GeometricConstraint,
    GeometricConstraintSolver, GeometricEntity, HelicalSpring, InvoluteGear, Mesh3D, NacaAirfoil,
    Quantifier, QuantifierEliminator, Ray3D, RelationalOp, ShapeExporter, ShapeImporter, Sign,
    ThreadStandard, ThreadedScrew,
};
pub use calculus::{FractionalCalculus, SymbolicCalculus, VariationalCalculus};
pub use cartan::{DifferentialForm, FormBasis, VectorField};
pub use cartan_symbolic::SymbolicForm;
pub use cat_eval::{CatEvalError, CatEvalInput, CatEvalOutput, CatExprEvaluator};
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
pub use hecs::Hecs;
pub use heuristic::{
    CompositeSearchHeuristic, HeuristicSearchEngine, NeuralExpressionEmbedder,
    SchwartzZippelHeuristic, SearchCandidate, SearchHeuristic, TreeSizeHeuristic,
};
pub use html_export::StandaloneHtmlExporter;
pub use hyperop::{ackermann, hyperoperation, knuth_up_arrow, pentation, super_log, tetration};
pub use iga::{CoxDeBoor, IgaStiffnessMatrix, MultiPatchBRep, NurbsPatchND, WeightedControlPoint};
pub use interconnect::UniversalBridge;
pub use lagrange::{InversionSeriesResult, LagrangeBurmann};
pub use lemma_miner::{AutonomousLemmaMiningPipeline, GrobnerProver, MinedTheoremRecord};
pub use lifting::{
    DomainCompatibilityChecker, DomainLiftRouter, EulerLiftingFunctor, LiftingFunctor,
    RetractionCleaner, WeierstrassLiftingFunctor,
};
pub use limits::{LimitDirection, LimitEngine};
pub use linprog::{LinprogSolution, LinprogStatus, linprog_minimize, yun_square_free_z};
pub use logic::{AssumptionsContext, GeneralizedValuedLogic, SatSolver, ThreeValuedLogic};
pub use matrix::SymbolicMatrix;
pub use mcts::{
    MctsAction, MctsConfig, MctsNode, MctsPolicyHead, MctsSearchEngine, MctsSearchResult,
    MctsValueHead, ProofStep,
};
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
pub use query::{QueryEngine, QueryResponse, ResultPod};
pub use risch::{DifferentialFieldTower, ExtensionKind, HermiteReductionResult, RischIntegrator};
pub use series::SymbolicSeries;
pub use session::{SymbolSessionMeta, UraeSession};
pub use sets::MathSetRepresentation;
pub use simplify::{SchwartzZippel, Simplifier};
pub use smt::{ProofCertificate, SmtExpr, SmtLogic, SmtProblem, SmtSort};
pub use solver::{SolveOutcome, SymbolicSolver};
pub use sparse::CsrMatrix;
pub use special::SpecialFunctions;
pub use stream::{
    BatchSummary, ComputeFuture, StreamMode, StreamProcessor, StreamRequest, StreamResponse,
    execute_async, execute_batch_async, query_async,
};
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
pub use urscript_pose::Pose;
pub use visualizer::{ColorRgba, DomainColoring, RiemannSurface, VectorFieldVisualizer};
pub use weyl_dmodules::{
    AlmkvistZeilberger, WeylDOperator, WeylGrobnerBasis, WeylMonomial, WeylTerm,
    ZeilbergerAlgorithm, ZeilbergerResult,
};
pub use workbench::{
    ContourSegment, DifferentialSystemAnalysis, DifferentialSystemSpec, EquationSystemAnalysis,
    EquationSystemSpec, QuiverArrow, WorkbenchEngine, WorkbenchPresets,
};
