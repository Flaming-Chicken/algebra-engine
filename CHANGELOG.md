# Changelog

All notable changes to `algebra-engine` (URAE) are documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and Semantic Versioning.

## [0.5.0] - 2026-11-17

### Added
- `IgaStiffnessMatrix::assemble_2d`, `NurbsPatchND::rational_basis` and `tangents`: the isogeometric Laplace-Beltrami stiffness matrix with analytic NURBS derivatives.
- `RittWuReducer::reduce` (differential pseudo-division) and `is_reduced_wrt`; `DiffPolynomial` normalisation and arithmetic (`add`, `sub`, `mul`, `scale`, `coefficient_of`, `initial`).
- `UraeLanguageServer::completions_at`: offers names defined above the cursor, newest first, before the catalogue.
- `SymbolicOdeSolver::frobenius_series_at_zero`: Frobenius series at a regular singular point (the module documentation previously claimed this method without implementing it).
- `poly::exact`: exact Gröbner bases over the rationals (`ExactPoly`, `ExactGrobner`) with a decision procedure for ideal membership. The floating-point `GrobnerBasis` is documented as numeric.
- Notebook export (`urae_notebook::export`): LaTeX, SymPy and Lean 4 files are built from the parsed document. The LaTeX export previously wrapped the raw buffer, the Python export put it in a docstring, and the Lean export was a two-line stub.
- `DysonSeries::first_order_term_between` (integral from `t0` to `t`) and `WeilJet::ito_drift`.
- `ProofTrace::export_lean4_with` and `export_coq_with`: each step is a theorem about the real expressions it rewrote (closed by `ring`; unsupported steps are listed as comments). The CLI and agent export use them.
- `calculus::pade_approximant`: exact Padé `[l/m]` approximants from rational Taylor coefficients.
- Closed forms for second-order linear recurrences with distinct or repeated real roots.
- Cartan operations (wedge, `d`, interior product, Lie derivative, Hodge star) read the form that was typed (`dx`, `2 dx + dz`, `dx^dy`).
- Summation of any polynomial summand by Faulhaber's formula (degree up to 12).
- Clifford vector products and rotor rotations compute real results; `export <format> <expr>` renders the expression in LaTeX, Unicode or MathML.

### Changed
- `CodeGenerator::compile_fn1`, `compile_fn2` and `compile_fn_n` compile to bytecode once and evaluate on `f64` per call instead of cloning the graph and substituting symbolically on every call.
- `VmCompiler` reports an unbound symbol as a compilation error; it used to compile it as the variable and return wrong numbers.
- `AdjointSensitivitySolver::compute_gradient` back-propagates exactly through the RK4 steps of `NeuralOde::forward` (fourth-order accurate gradients). It was a first-order scheme with an error of about 1% at 100 steps, and it ignored its `vector_field` argument.
- FEA solves report an under-constrained (singular) system as an error and verify the solution's residual. Previously a singular equation was skipped and its unknown reported as `0`, so a structure with rigid-body motion returned plausible-looking displacements.
- `WavefrontSchedule::build` and `evaluate` return `Result` (`WavefrontError`): levels follow dependencies rather than edge listing order, cycles are detected, and a missing wire, an unsupported operation or a division by zero is an error instead of silently `0`. `Inv` and `Identity` are now evaluated.
- `RittWuReducer::characteristic_set` computes a characteristic set (Ritt's procedure) instead of sorting its input; `{y'', y, y'}` now gives `{y}`.
- `UniversalBridge::weyl_to_diff_polynomial` returns `Result` and `DiffPolynomial` to `WeylDOperator` is now `TryFrom`: conversions that would drop an `x` power, flatten a nonlinear term or merge indeterminates are errors instead of silently wrong output.
- Differential-equation classification reports PDEs as linear, semilinear, quasilinear or nonlinear by the location of the nonlinearity (the KdV equation is now semilinear, Burgers quasilinear). Powers and products of derivatives are detected as nonlinear.
- Production code no longer contains reachable `unwrap`, `expect` or `panic!` calls outside documented operator panics; the workspace lints `clippy::unwrap_used`, `expect_used` and `panic` are on (test code exempt). A failed notebook launch is reported instead of panicking, and a poisoned token lock is recovered.
- `KnotInvariants::kauffman_bracket` and `jones_polynomial` return `Result` and compute a state sum (previously a lookup table for three knots and a product of `A^±1` otherwise). A positive `σ₁³` now gives `A⁻⁷ − A⁻³ − A⁵`, the standard right-handed trefoil (the old table held its mirror image).
- `AnytimeHandle::refine_step` and `run_to_budget` take a re-evaluation closure and `refine_with` accepts only sound (contained, tighter) balls; enclosures no longer shrink by a fixed factor.
- Planning consolidated: `docs/PLANNING.md` indexes the planning documents and tracks every known stub and defect against the code; `ROADMAP.md` marks phases 12-16, 22, 23 and 27 as Partial and lists the scope that has no implementation.
- `pade!` returns `AlgebraResult<ExprId>` and computes a real approximant (it returned a placeholder quotient).
- `creative_telescoping_integral`, `EmpiricalDistributionMut::{pdf, cdf}` and an unreachable AI provider return errors instead of a fixed Gaussian ODE, constants and a canned `f(x)` explanation.
- `puiseux_series` with a fractional exponent, `log`, `preset` and AI queries in the executor return errors instead of placeholder output.
- Unsupported summands, complex-root and higher-order recurrences return an error instead of a wrong formula.

### Fixed
- `GrobnerProver` (lemma mining) reduced the difference of two expressions modulo an empty ideal, so it could not use identities such as `sin^2 + cos^2 = 1`. It now extracts the side relations of the transcendental atoms present and decides membership exactly over the rationals.
- `GeometricConstraintSolver::solve` ignored every constraint except distance and coincidence (placing a point along the x-axis); it now solves parallel, perpendicular, angle, tangent and fixed-area constraints too by damped Gauss-Newton, and reports inconsistent systems.
- `IgaStiffnessMatrix::integrate_stiffness_entry_2d` ignored the NURBS basis and returned a fixed weight of 1.0 on the diagonal and 0.1 elsewhere times the area; it now integrates the real gradient inner product (the unit square gives the textbook Q1 matrix). `metric_determinant_2d` uses analytic tangents.
- `DiffPolynomial::differentiate` left repeated indeterminates in a term (`y'` twice), which made degrees and leaders wrong; it now normalises.
- LSP hover evaluated only the hovered line, so any line using an earlier definition showed an error or a wrong value; it now evaluates the document up to that line in order.
- `StateSpaceSystem::is_stable` judged systems with three or more states by the sign of the trace; it now applies Routh-Hurwitz (continuous) or Schur-Cohn (discrete) to the characteristic polynomial.
- `frequency_response` returned zeros for every system that was not a two-state SISO one, and ignored the sample time of discrete systems; it now evaluates `C(sI - A)^-1 B + D` in general.
- `pole_placement_ackermann` supports any controllable single-input system (it stopped at two states).
- `UnificationEngine::unify` only handled a top-level symbol or number and failed on everything else. It is now a structural first-order unifier with an occurs check; `SubstitutionMap::apply` applies the result.
- The CLI proof export gated its step on unifying an expression with its own simplification, which almost never succeeded and is not an equivalence test; it now uses the equivalence check.
- `lagrange_inversion` divided `w / f` before differentiating, which is `0/0` at the expansion point for every `f` except the identity. It now computes the exact series of the inverse function and rejects non-invertible input.
- Forced 2×2 linear systems omitted the Duhamel convolution of the forcing term.
- The symbolic tier labelled a tangent-line approximation as order 3; it is a first-order expansion with an order-2 remainder.
- `DysonSeries::first_order_term` returned the integrand when the integral had no closed form.
- `sum k^2` returned `k^2 * n` (any non-symbol summand was treated as constant).
- `y' = 1` returned an expression containing `1 / 0`.
- The Fibonacci recurrence returned `0`.
- Caputo derivative used the wrong kernel exponent for orders other than n - 1/2.
- `solve_pde_heat_1d` returned `cos(k_x)` with no `x` and a decay rate of `k_x` instead of `k_x^2`.

## [0.4.0] - 2026-11-17

### Added
- Overhaul (`23f508b`): `cat-core` 4-fidelity evaluation, BSGS tensor canonicalisation, generalised transforms and a polyglot DSL.
- CAS depth overhaul with multivalued and uncertainty support, and an audit regression suite (`4c2a293`).
- Unified API, stream and workbench layers, naming clean-up, and a GUI architecture plan (`ab4d0f7`).
- Integration by parts for `x^n` times `exp`, `sin`, `cos` and `ln` for any degree, and `1/(x^2 - b^2)` integrated as a logarithm.
- Limits at +infinity by dominant-term analysis (radicals, variable exponents); two-sided limits through `abs`, `sign`, `floor` and `ceil` compare the one-sided values.
- Factoring of a difference of squares with multi-term roots (`a^2 + 2ab + b^2 - c^2`), using an exact multivariate polynomial square root.
- The simplifier merges repeated bases (`x * x` is `x^2`) and combines square roots of positive rationals (`sqrt(2) * sqrt(8)` is `4`).

### Changed
- All three formatters (LaTeX, Unicode, MathML) print with operator precedence: parentheses where needed, negative terms as subtraction. Output text changes, so anything matching printed strings must be updated.
- `ROADMAP.md` marks phases 8, 28 and 30 as Partial or Foundation, with what the code does.

### Fixed
- `factor_primes` printed a composite that Pollard rho could not split as a prime; it is now marked `[composite, not factored]`.
- `exp(2*x) = 7` returned `ln(-sqrt(7))` as a root; roots of equations quadratic in `e^x` print as `ln(7)/2` and `ln(7)/2 + i*pi`.
- An equation that is an identity after clearing denominators (`(x - 1)/(x - 1) = 1`) returned no solutions; it returns the variable, as `x = x` does.
- The Unicode parser converted only the first `sqrt(` in an input.

## [0.3.2] - 2026-11-15

### Added
- Salvaged `Magnitude` (`cat-numbers`), `Hecs<T>` hexagonal coordinates, and `Pose<T>` SE(3) kinematics (`urscript_pose`), re-exported via `urae::prelude`.
- `NumericalOdeMethod::SelfOptimizingMeta`, `ButcherTableau::tsit5()`, and `NumericalPdeSolver::solve_burgers_1d_mol`.
- `urae-py` (`pyproject.toml`, `OdeTrajectorySummary`, `py_*` bindings), `urae-kernel` Jupyter v5.3 message dispatcher, and `urae-wasm` JSON plotting/ODE endpoints.
- Typed `SolveOutcome` (`solveset_outcome`), general $N \times N$ cofactor/adjugate/charpoly + Bareiss RREF/nullspace, and `SymbolicCalculus::integrate_with_steps`.
- `linprog_minimize` (Simplex with Bland's anti-cycling rule) and `yun_square_free_z` (Yun square-free decomposition over $\mathbb{Z}[x]$).
- Truthful `Feature Matrix (URAE vs. SymPy vs. Symbolica)` in `README.md`.
