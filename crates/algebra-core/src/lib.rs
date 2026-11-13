//! # `algebra-core`
//!
//! Foundation crate for Universal Rust Algebra Engine (URAE).
//!
//! Provides memory-safe, thread-safe, arena-allocated Expression DAGs with
//! lock-free hash-consing deduplication, mathematical domain typing, symbol interning,
//! rigorous interval arithmetic, abstract algebra traits, multi-notation parsing, operations, and formatting.

pub mod config;
pub mod domain;
pub mod error;
pub mod expr;
pub mod format;
pub mod graph;
pub mod id;
pub mod interval;
pub mod lemma_miner;
pub mod number;
pub mod numbers;
pub mod operation;
pub mod parallel;
pub mod parser;
pub mod probabilistic;
pub mod step;
pub mod symbol;
pub mod traits;
pub mod vm;

pub use auto_promoters::uint::Uint;
pub use config::{BudgetStatus, EngineConfig, ResourceBudget};
pub use domain::{Domain, DomainBound, DomainCategory, GoalDomain, MathContext};
pub use error::{AlgebraError, AlgebraResult};
pub use expr::{ExprKind, ExprNode, RelOp, SetOp};
pub use format::{FormatError, FormatResult, Formatter, LatexFormatter, UnicodeFormatter};
pub use graph::ExprGraph;
pub use id::{DomainId, ExprId, SymbolId};
pub use interval::RealInterval;
pub use lemma_miner::{
    DomainKind, EquivalenceHypothesis, ExpressionGenerator, ExpressionGeneratorConfig,
    FingerprintTriple, FormalEquivalenceProver, LearnedLemma, LemmaMiner, LemmaRegistry,
};
pub use number::{Constant, Number};
pub use numbers::{DualNumber, HyperrealNumber, Octonion, PAdicNumber, Quaternion, SurrealNumber};
pub use operation::{
    CombinatoricsOpKind, ComparisonKind, ControlOpKind, GaloisOpKind, HyperOpKind, IntervalOpKind,
    LogicOpKind, MathOperation, MatrixOpKind, NumberTheoryOpKind, PhysicsOpKind, PolynomialOpKind,
    ProofTarget, SimplifyStrategy, StatMechOpKind, SymbolRoleKind, SystemCommandKind,
    TopologyOpKind, TransformOpKind, TropicalOpKind,
};
pub use parser::{ExprParser, parse_domain_declaration, parse_operation};
pub use probabilistic::{ProbabilisticVerifier, Solution};
pub use step::{MathStep, StepByStepResult};
pub use symbol::SymbolTable;
pub use traits::{
    AbelianGroup, CommutativeRing, DifferentialRing, Field, Group, HeytingAlgebra, LieAlgebra,
    Magma, MathSet, Monoid, Ring, Semigroup, TensorAlgebra, TropicalSemiring, ValuationProvider,
    VectorSpace,
};
pub use vm::{BytecodeProgram, BytecodeVM, Instruction, VmCompiler, VmError};
