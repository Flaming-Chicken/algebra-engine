# Changelog

All notable changes to `algebra-engine` (URAE) are documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and Semantic Versioning.

## [Unreleased]

### Added
- `ProofTrace::export_lean4_with` and `export_coq_with`: each step is a theorem about the real expressions it rewrote (closed by `ring`; unsupported steps are listed as comments). The CLI and agent export use them.
- `calculus::pade_approximant`: exact Padé `[l/m]` approximants from rational Taylor coefficients.
- Closed forms for second-order linear recurrences with distinct or repeated real roots.
- Cartan operations (wedge, `d`, interior product, Lie derivative, Hodge star) read the form that was typed (`dx`, `2 dx + dz`, `dx^dy`).
- Summation of any polynomial summand by Faulhaber's formula (degree up to 12).
- Clifford vector products and rotor rotations compute real results; `export <format> <expr>` renders the expression in LaTeX, Unicode or MathML.

### Changed
- Planning consolidated: `docs/PLANNING.md` indexes the planning documents and tracks every known stub and defect against the code; `ROADMAP.md` marks phases 12-16, 22, 23 and 27 as Partial and lists the scope that has no implementation.
- `pade!` returns `AlgebraResult<ExprId>` and computes a real approximant (it returned a placeholder quotient).
- `creative_telescoping_integral`, `EmpiricalDistributionMut::{pdf, cdf}` and an unreachable AI provider return errors instead of a fixed Gaussian ODE, constants and a canned `f(x)` explanation.
- `puiseux_series` with a fractional exponent, `log`, `preset` and AI queries in the executor return errors instead of placeholder output.
- Unsupported summands, complex-root and higher-order recurrences return an error instead of a wrong formula.

### Fixed
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
