//! # `algebra_core::operation`
//!
//! Strongly-typed Mathematical Operation AST and Command Pipeline.
//!
//! Unifies natural language intent, imperative transformations, equation solving,
//! discrete & continuous analysis, matrices, transforms, physics, logic, interval arithmetic,
//! esoteric operators (Knuth up-arrows, tropical semirings), formal proofs,
//! and notebook symbol declarations into a single pipeline.

use crate::domain::DomainBound;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Strategy for algebraic simplification and pattern rewriting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimplifyStrategy {
    /// Full E-Graph equality saturation
    Standard,
    /// Polynomial/expression expansion
    Expand,
    /// Polynomial factorization
    Factor,
    /// Rational fraction combination
    Together,
    /// Common factor cancellation
    Cancel,
    /// Horner polynomial form
    Horner { var: Option<String> },
    /// Partial fraction decomposition
    PartialFractions { var: Option<String> },
    /// Collect terms by variable powers
    Collect { var: String },
    /// Minimize AST tree depth
    MinTree,
    /// Minimize operator count
    MinOps,
    /// Minimize leaf count
    MinLeaf,
}

/// Esoteric hyperoperations and Knuth up-arrows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HyperOpKind {
    /// $a \uparrow^n b$
    KnuthUpArrow { a: u64, arrows: usize, b: u64 },
    /// $a \uparrow\uparrow b$
    Tetration { a: u64, b: u64 },
    /// $a \uparrow\uparrow\uparrow b$
    Pentation { a: u64, b: u64 },
    /// General $H_n(a, b)$
    General { n: usize, a: u64, b: u64 },
    /// Ackermann $A(m, n)$
    Ackermann { m: u64, n: u64 },
    /// Super-Logarithm $\text{slog}_b(x)$
    SuperLog { base: f64, val: f64 },
}

/// Tropical semirings and min-plus / max-plus operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TropicalOpKind {
    MaxPlusAdd {
        a: f64,
        b: f64,
    },
    MaxPlusMul {
        a: f64,
        b: f64,
    },
    MinPlusAdd {
        a: f64,
        b: f64,
    },
    MinPlusMul {
        a: f64,
        b: f64,
    },
    LogSumExp {
        x: f64,
        y: f64,
        epsilon: f64,
    },
    MatrixMul {
        semiring: String,
        rows: usize,
        cols: usize,
        a_data: Vec<f64>,
        b_data: Vec<f64>,
    },
}

/// Linear algebra & matrix operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MatrixOpKind {
    Determinant { matrix_str: String },
    Inverse { matrix_str: String },
    Trace { matrix_str: String },
    Transpose { matrix_str: String },
    CharPoly { matrix_str: String, var: String },
    Multiply { a_str: String, b_str: String },
    Rref { matrix_str: String },
    Rank { matrix_str: String },
    Nullspace { matrix_str: String },
    Eigenvalues { matrix_str: String },
}

/// Discrete combinatorics and partition functions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombinatoricsOpKind {
    Factorial { n: u64 },
    Combinations { n: u64, k: u64 },
    Permutations { n: u64, k: u64 },
    Derangements { n: u64 },
    Catalan { n: u64 },
    Bell { n: u64 },
    Stirling2 { n: u64, k: u64 },
    Partitions { n: u64 },
}

/// Integral and functional transforms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransformOpKind {
    Laplace {
        expression: String,
        time_var: String,
        freq_var: String,
    },
    InverseLaplace {
        expression: String,
        freq_var: String,
        time_var: String,
    },
    Fourier {
        expression: String,
        time_var: String,
        freq_var: String,
    },
}

/// Polynomial algebra and elimination operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolynomialOpKind {
    Roots {
        poly_str: String,
    },
    Discriminant {
        poly_str: String,
        var: String,
    },
    Resultant {
        poly1_str: String,
        poly2_str: String,
        var: String,
    },
    Gcd {
        poly1_str: String,
        poly2_str: String,
    },
}

/// Physical dimensional quantity calculations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PhysicsOpKind {
    Quantity {
        val: f64,
        unit: String,
    },
    Convert {
        val: f64,
        from_unit: String,
        to_unit: String,
    },
}

/// Certified interval arithmetic operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IntervalOpKind {
    Add {
        a_lo: f64,
        a_hi: f64,
        b_lo: f64,
        b_hi: f64,
    },
    Mul {
        a_lo: f64,
        a_hi: f64,
        b_lo: f64,
        b_hi: f64,
    },
    Sub {
        a_lo: f64,
        a_hi: f64,
        b_lo: f64,
        b_hi: f64,
    },
    Div {
        a_lo: f64,
        a_hi: f64,
        b_lo: f64,
        b_hi: f64,
    },
}

/// Boolean and multi-valued logic operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogicOpKind {
    SatSolve { formula: String },
    TruthTable { formula: String },
}

/// Computational number theory operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NumberTheoryOpKind {
    /// Continued fraction expansion of real or rational
    ContinuedFraction { val_str: String, max_terms: usize },
    /// Primality check (Miller-Rabin)
    IsPrime { n: u64 },
    /// Arbitrary-precision / expression primality check
    IsPrimeExpr { expr_str: String },
    /// Extended Euclidean Algorithm $\gcd(a, b) = a x + b y$
    ExtendedGcd { a: i64, b: i64 },
    /// Arbitrary-precision / expression Extended GCD
    ExtendedGcdExpr { a_str: String, b_str: String },
    /// Arbitrary-precision / expression LCM
    LcmExpr { a_str: String, b_str: String },
    /// Legendre symbol $(a/p)$
    Legendre { a: i64, p: u64 },
    /// An integer function of one to three integer arguments: `totient`, `next_prime`, `prev_prime`, `divisors`,
    /// `sigma`, `tau`, `mobius`, `mod_inverse`, `pow_mod`, `primitive_root`.
    Arithmetic { function: String, args: Vec<i64> },
    /// Prime decomposition with algebraic prime classification across number systems
    PrimeDecomposition {
        expr_str: String,
        domain_hint: Option<String>,
    },
}

/// Linear control systems operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ControlOpKind {
    StateSpace {
        a: Vec<Vec<f64>>,
        b: Vec<Vec<f64>>,
        c: Vec<Vec<f64>>,
        d: Vec<Vec<f64>>,
    },
    Stability2x2 {
        a11: f64,
        a12: f64,
        a21: f64,
        a22: f64,
    },
    Controllability2x2 {
        a11: f64,
        a12: f64,
        a21: f64,
        a22: f64,
        b1: f64,
        b2: f64,
    },
    DiscretizeZoh {
        a: Vec<Vec<f64>>,
        b: Vec<Vec<f64>>,
        c: Vec<Vec<f64>>,
        d: Vec<Vec<f64>>,
        sample_time: f64,
    },
    RankTests {
        a: Vec<Vec<f64>>,
        b: Vec<Vec<f64>>,
        c: Vec<Vec<f64>>,
    },
    PolePlacement {
        a: Vec<Vec<f64>>,
        b: Vec<Vec<f64>>,
        desired_poles: Vec<f64>,
    },
    FrequencyResponse {
        a: Vec<Vec<f64>>,
        b: Vec<Vec<f64>>,
        c: Vec<Vec<f64>>,
        d: Vec<Vec<f64>>,
        omega: f64,
    },
}

/// Differential equation solving (Automatic ODE/PDE detection, classification, and solving).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffEqOpKind {
    ClassifyAndSolve {
        equation: String,
        dependent_var: Option<String>,
        independent_vars: Option<Vec<String>>,
        override_kind: Option<String>,
    },
}

/// Simplicial topology operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TopologyOpKind {
    EulerCharacteristic {
        simplices: Vec<Vec<usize>>,
    },
    CountSimplices {
        simplices: Vec<Vec<usize>>,
        dim: usize,
    },
}

/// Statistical mechanics operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StatMechOpKind {
    PartitionFunction {
        energy_levels: Vec<(f64, usize)>,
        beta: f64,
    },
    FreeEnergy {
        energy_levels: Vec<(f64, usize)>,
        beta: f64,
    },
    InternalEnergy {
        energy_levels: Vec<(f64, usize)>,
        beta: f64,
    },
    FermiDirac {
        energy: f64,
        chemical_potential: f64,
        beta: f64,
    },
    BoseEinstein {
        energy: f64,
        chemical_potential: f64,
        beta: f64,
    },
}

/// Galois theory operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GaloisOpKind {
    QuadraticExtension { d: i64 },
    CheckSolvability { group_name: String },
}

/// Cartan exterior differential calculus operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CartanOpKind {
    Wedge {
        form1_str: String,
        form2_str: String,
        dim: usize,
    },
    ExteriorDerivative {
        form_str: String,
        dim: usize,
    },
    InteriorProduct {
        form_str: String,
        vector_field: Vec<f64>,
        dim: usize,
    },
    LieDerivative {
        form_str: String,
        vector_field: Vec<f64>,
        dim: usize,
    },
    /// Interior product with a vector field whose components are expressions in the coordinates
    /// (`x`, `y`, `z`, `t`), such as `["y", "-x", "0"]`; the form may have symbolic coefficients.
    InteriorProductSymbolic {
        form_str: String,
        vector_field: Vec<String>,
        dim: usize,
    },
    /// Lie derivative along a vector field with expression components (see
    /// [`CartanOpKind::InteriorProductSymbolic`]).
    LieDerivativeSymbolic {
        form_str: String,
        vector_field: Vec<String>,
        dim: usize,
    },
    /// Hodge star for a diagonal metric whose entries are expressions in the coordinates
    /// (`x`, `y`, `z`, `t`), such as `["1", "x^2", "x^2 * sin(y)^2"]` for spherical coordinates.
    HodgeStarMetric {
        form_str: String,
        metric: Vec<String>,
        dim: usize,
    },
    /// Hodge star for a general symmetric metric given as a matrix of expressions in the
    /// coordinates (up to four), e.g. `[["2", "1"], ["1", "2"]]`.
    HodgeStarMatrix {
        form_str: String,
        metric: Vec<Vec<String>>,
        dim: usize,
    },
    HodgeStar {
        form_str: String,
        dim: usize,
        is_minkowski: bool,
    },
}

/// Clifford geometric algebra and multivector operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CliffordOpKind {
    GeometricProduct {
        sig_p: usize,
        sig_q: usize,
        sig_r: usize,
        a_coords: Vec<f64>,
        b_coords: Vec<f64>,
    },
    RotorSandwich {
        sig_p: usize,
        sig_q: usize,
        sig_r: usize,
        vector: Vec<f64>,
        angle_rad: f64,
        bivector_plane: (usize, usize),
    },
}

/// Certified probabilistic simplification and root verification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CertifiedOpKind {
    Simplify {
        expression: String,
        eps: f64,
    },
    Solve {
        equation: String,
        var: String,
        eps: f64,
    },
    VerifyZero {
        expression: String,
        eps: f64,
    },
    VerifyEquivalence {
        lhs: String,
        rhs: String,
        eps: f64,
    },
}

/// Laplace domain LTI block diagram operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockDiagramOpKind {
    Feedback { g_expr: String, h_expr: String },
    Series { g1_expr: String, g2_expr: String },
    Parallel { g1_expr: String, g2_expr: String },
}

/// Symbolic Partial Differential Equation operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PdeOpKind {
    /// 1D Wave equation: $u_{tt} = c^2 u_{xx}$
    Wave1D {
        speed: String,
        x_var: String,
        t_var: String,
        initial_pos: Option<String>,
        initial_vel: Option<String>,
    },
    /// 1D Heat equation: $u_t = \alpha^2 u_{xx}$
    Heat1D {
        alpha: String,
        x_var: String,
        t_var: String,
        length: Option<String>,
    },
    /// 2D Laplace equation: $u_{xx} + u_{yy} = 0$
    Laplace2D {
        x_var: String,
        y_var: String,
        a_bound: Option<String>,
        b_bound: Option<String>,
    },
    /// 1D Transport / Advection equation: $u_t + c u_x = 0$
    Transport1D {
        speed: String,
        x_var: String,
        t_var: String,
        initial_state: Option<String>,
    },
    /// 2D Radial Laplacian Bessel eigenmode: $\nabla^2 u = k^2 u \implies J_n(k r)$
    RadialBessel {
        wave_num: String,
        r_var: String,
        order: usize,
    },
}

/// Target language for formal theorem proving export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofTarget {
    Lean4,
    Coq,
    Both,
}

/// Structural comparison and isomorphism checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonKind {
    Isomorphic,
    Equivalent,
    General,
}

/// Notebook symbol declaration role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolRoleKind {
    Variable,
    Parameter,
    Constant,
}

/// Multi-valued Riemann surface and branch-cut operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BranchOpKind {
    /// Evaluate a multi-valued function on a specified Riemann sheet or custom branch cut.
    /// E.g., `branch_cut("sqrt", sheet = 1, cut = "(-infinity, 0]")` or `branch ln(-1), sheet = 1`.
    EvaluateSheet {
        function: String,
        expression: Option<String>,
        sheet: i64,
        cut: Option<String>,
    },
    /// Enumerate all algebraic branches or a finite sheet fiber of an expression.
    /// E.g., `branches (-1)^(1/3)` or `branches lambert_w(-0.2)`.
    AllBranches {
        expression: String,
        max_branches: usize,
    },
}

/// System / interactive REPL commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemCommandKind {
    Help,
    Vars,
    Params,
    Context,
    Log { level: String },
    Preset { name: String },
    Export { format: String, expr: String },
    Clear,
}

/// Central unified Mathematical Operation AST.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MathOperation {
    /// Pure mathematical expression to parse and format: e.g. `sin(x) * x^2`
    Expression { raw: String },

    /// Symbolic differentiation $d^n/dx^n$: e.g. `diff x^3, x`, `d/dx (x^3)`, `d^2/dx^2 (x^4)`
    Differentiate {
        expression: String,
        variable: Option<String>,
        order: usize,
        is_total: bool,
    },

    /// Symbolic or definite integration $\int_a^b f(x) dx$: e.g. `integrate cos(x), x`, `int cos(x) along [0, pi]`
    Integrate {
        expression: String,
        variable: Option<String>,
        lower: Option<String>,
        upper: Option<String>,
    },

    /// Equation root finding / solving: e.g. `solve x^2 - 4 = 0, x`, `given f(x) = 3 find x`
    Solve {
        equation: String,
        variable: Option<String>,
    },

    /// Multi-equation system solving: e.g. `solve [2*x + y = 5, x - y = 1], [x, y]`
    SolveSystem {
        equations: Vec<String>,
        variables: Vec<String>,
    },

    /// Algebraic simplification and transformations: e.g. `simplify (x+0)*1`, `expand (x+1)^3`, `factor x^2-1`
    Simplify {
        expression: String,
        strategy: SimplifyStrategy,
    },

    /// Numerical evaluation / reduction: e.g. `eval sqrt(pi^2)`
    Evaluate {
        expression: String,
        precision_digits: Option<usize>,
        bindings: HashMap<String, f64>,
    },

    /// Variable substitution: e.g. `substitute x = 2 in x^2 + 1`
    Substitute {
        expression: String,
        variable: String,
        value: String,
    },

    /// Analytical limit: e.g. `limit sin(x)/x as x -> 0`
    Limit {
        expression: String,
        variable: String,
        point: String,
        direction: Option<String>,
    },

    /// Series expansion: e.g. `series exp(x) around x = 0 to order 5`
    Series {
        expression: String,
        variable: String,
        point: String,
        order: usize,
    },

    /// Symbolic summation: e.g. `sum 1/n^2 from n=1 to infinity`
    Sum {
        expression: String,
        variable: String,
        lower: String,
        upper: String,
    },

    /// Linear algebra & matrices (det, inv, trace, charpoly, transpose)
    Matrix(MatrixOpKind),

    /// Discrete Combinatorics (factorial, combinations, permutations, catalan, bell, stirling, partitions)
    Combinatorics(CombinatoricsOpKind),

    /// Integral Transforms (Laplace, Fourier, Inverse Laplace)
    Transform(TransformOpKind),

    /// Polynomials (roots, discriminant, resultant, gcd)
    Polynomial(PolynomialOpKind),

    /// Physical Units & Dimensions
    Physics(PhysicsOpKind),

    /// Certified Interval Arithmetic
    Interval(IntervalOpKind),

    /// Boolean & SAT Logic
    Logic(LogicOpKind),

    /// Knuth Up-Arrow, Tetration & Hyperoperations
    HyperOp(HyperOpKind),

    /// Tropical semirings (Max-Plus, Min-Plus, Log-Sum-Exp)
    Tropical(TropicalOpKind),

    /// Computational Number Theory (Continued fractions, Primality, Legendre, GCD)
    NumberTheory(NumberTheoryOpKind),

    /// State-Space Control Systems
    Control(ControlOpKind),

    /// Simplicial Topology & Euler Characteristic
    Topology(TopologyOpKind),

    /// Statistical Mechanics & Quantum Thermodynamics
    StatMech(StatMechOpKind),

    /// Galois Theory & Field Extensions
    Galois(GaloisOpKind),

    /// Cartan Exterior Differential Calculus
    Cartan(CartanOpKind),

    /// Clifford Geometric Algebra & Multivectors
    Clifford(CliffordOpKind),

    /// Certified Probabilistic Simplification & Root Verification
    Certified(CertifiedOpKind),

    /// Laplace Domain LTI Block Diagram Algebra
    BlockDiagram(BlockDiagramOpKind),

    /// Symbolic Partial Differential Equations (Wave, Heat, Laplace, Transport, Bessel)
    Pde(PdeOpKind),

    /// Automatic Differential Equation solving (ODE/PDE detection, classification, and analytical reduction)
    DiffEq(DiffEqOpKind),

    /// Formal proof trace export to Lean 4 / Coq: e.g. `proof (x + 0) * 1`
    Proof {
        expression: String,
        target: ProofTarget,
    },

    /// Mathematical structure comparison / isomorphism
    Compare {
        left: String,
        right: String,
        kind: ComparisonKind,
    },

    /// Probability distribution declaration: e.g. `X ~ Normal(0, 1)`
    Distribution {
        name: String,
        dist_type: String,
        params: Vec<String>,
    },

    /// Symbol declaration: e.g. `a: Parameter = 2.0 [m]`, `x: Variable`
    Declaration {
        name: String,
        role: SymbolRoleKind,
        value: Option<f64>,
        unit: Option<String>,
    },

    /// Symbol declaration with explicit uncertainty (`a: Parameter = 5.0 +/- 0.1 [m]` or `a = 5.0 ± 0.1 [m]`).
    UncertainDeclaration {
        name: String,
        role: SymbolRoleKind,
        value: f64,
        uncertainty: f64,
        unit: Option<String>,
    },

    /// Multi-valued complex branch evaluation & Riemann sheet selection.
    Branch(BranchOpKind),

    /// Set-builder domain restriction: e.g. `{ x in Reals | -5 <= x <= 5 }`
    SetBuilder {
        var_name: String,
        domain_type: String,
        condition: String,
    },

    /// Domain bound restriction declaration: e.g. `a in Reals [0, 100]`, `x in Positive`, `a in [-5, 5]`
    DomainRestriction(DomainBound),

    /// Mathematical context configuration: e.g. `set algebra Quaternion`
    SetContext { key: String, value: String },

    /// Interactive REPL / System command: e.g. `help`, `vars`, `log info`
    System(SystemCommandKind),

    /// AI Copilot co-reasoning query: e.g. `agent Explain step by step how to differentiate x^3`
    AiQuery { prompt: String },
}
