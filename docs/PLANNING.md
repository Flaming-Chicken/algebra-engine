# URAE Planning Index and Status Ledger

*Checked 17 Nov 2026 against `dev` at v0.4.0 (`e483860`). This is the one page that says what is planned, what has
been done, and what is still open. Every status below was checked against the code, not copied from another plan.*

## 1. Planning documents and what each is for

| Document | Role | Status |
| :--- | :--- | :--- |
| [`ROADMAP.md`](../ROADMAP.md) | What each of the 31 phases was meant to deliver, with an evidence-based status per phase | Current. Section 2 carries the status; section 5 lists intended scope, not shipped scope |
| **This file** | Index, defect ledger, open work in priority order | Current |
| [`AUDIT-2026-11-15.md`](AUDIT-2026-11-15.md) | First quality audit (correctness, efficiency, competitors) | Historical. Findings were fixed in 0.3.2 and 0.4.0; see section 3 |
| `CODEBASE_AUDIT_AND_REMEDIATION_PLAN.md` | Stub/defect catalogue (A1–A11, B1–B12, C1–C15) with remediation roadmap, written at v0.3.2 | Source of the ledger in section 2; item status here supersedes it (status banner added at its top) |
| [`AUDIT-PANICS-AND-UNWRAPS.md`](AUDIT-PANICS-AND-UNWRAPS.md) | Inventory of production `panic!`/`unwrap` sites | Open work, see section 4 |
| [`GUI_DESIGN_PLAN.md`](GUI_DESIGN_PLAN.md) | Five-phase GUI rework | Unstarted apart from the workbench panel; its own status table is the reference |
| [`../DOCUMENTATION_STANDARDS.md`](../DOCUMENTATION_STANDARDS.md) | Rustdoc/KaTeX conventions | Reference, not a plan |
| [`../CHANGELOG.md`](../CHANGELOG.md) | What shipped, per release | Reference |
| Portfolio-level plan | `PLAN.md` in the parent workspace; its section 4.1 summarises this project | Links here for detail |

Reading order for newcomers: ROADMAP section 2 (what exists) → this file (what is wrong or missing) → the audit
documents only when a finding needs its original evidence.

## 2. Stub and defect ledger

"Open" means the cited code is unchanged since the plan was written and still shows the described behaviour.
Items marked "reproduced" were run against `dev` (17 Nov 2026); the rest were confirmed by reading the cited
code. No item remains unchecked. Fixed defects have a regression test in
`tests/tests/engine/known_defect_tests.rs`.

### A. Silent stubs (feature claims that return placeholder output)

| ID | Item | Status | Evidence at `dev` |
| :--- | :--- | :--- | :--- |
| A1 | `pade!` macro | **Fixed** | `pade!` now calls `calculus::pade_approximant`, which solves the exact rational system from the Taylor coefficients and errors when they are not rational. The macro now returns `AlgebraResult<ExprId>` |
| A2 | `MathOperation::Clifford` products | **Fixed** | Clifford vector product and rotor rotation compute real results (Euclidean signature for rotations), cross-checked against the `algebra-advanced` multivector implementation; wrong sizes or other signatures are errors |
| A3 | `MathOperation::Cartan` ignores the form string | **Fixed** | Cartan operations parse the typed form (`dx`, `2 dx + dz`, `dx^dy`, constant coefficients) and print the resulting terms; unreadable input is an error. Coefficients are still constants (C5) |
| A4 | `System` Log/Preset/Export/Clear, `AiQuery` echo only | **Fixed** | `export <format> <expr>` renders the expression (latex, unicode, mathml). `log`, `preset` and AI queries return errors saying where they are handled; `clear` is session state and is intercepted by `Session` |
| A5 | `creative_telescoping_integral` | **Partly fixed** | `creative_telescoping_integral` returns an explicit not-implemented error instead of the fixed Gaussian ODE for any input; the Gaussian case remains as `AlmkvistZeilberger::gaussian_integral_ode`. General creative telescoping is still open (A8) |
| A6 | `puiseux_series` | **Partly fixed** | denominator 1 returns the Taylor series; fractional exponents return an explicit not-implemented error instead of placeholder coefficients. A real Puiseux expansion is still open |
| A7 | `EmpiricalDistributionMut::{pdf, cdf}` | **Fixed** | `EmpiricalDistributionMut` keeps only running moments, so `pdf`/`cdf` return an error instead of constants |
| A8 | `ZeilbergerAlgorithm` / `AlmkvistZeilberger` | **Open** | Three hard-coded showcase proofs; no general creative telescoping |
| A9 | `ProofTrace::export_lean4/export_coq` | **Fixed** | `export_lean4_with` / `export_coq_with` state each step as a theorem about the real expressions, closed by `ring`, with untranslatable steps listed as comments. Output is not machine-checked by URAE; the skeleton `export_lean4`/`export_coq` remain but are documented as such. CLI and agent use the new methods |
| A10 | Agent offline fallback | **Fixed** | an unreachable AI provider now returns an error (API key redacted) instead of a fixed `f(x)` explanation |
| A11 | GUI `.tex/.py/.lean` export, presentation export, `WorkspaceTab` routing | **Partly fixed** | `.tex`, `.py` and `.lean` export now translate the parsed document (`urae_notebook::export`): equations and expressions are typeset, emitted as SymPy, or stated as Lean theorems; lines that do not parse are kept as comments. Still open: the presentation print/export and the `WorkspaceTab` routing |

### B. Formula and algorithm defects

| ID | Item | Status | Evidence at `dev` |
| :--- | :--- | :--- | :--- |
| B1 | `summation` wrong on non-trivial summands | **Fixed** | polynomial summands in the summation variable are summed by Faulhaber's formula (degree up to 12); anything else is an error |
| B2 | `caputo_derivative` wrong for α ≠ 0.5 | **Fixed** | kernel exponent is `α + 1 − n` |
| B3 | `solve_pde_heat_1d` | **Fixed** | modes are `cos(k_x x)` and `exp(-α² k_x² t)`, using the symbols passed in |
| B4 | `solve_ode_linear` with `p = 0` | **Fixed** | `y' = 1` gives `x`; each stage is simplified before it is integrated again |
| B5 | `laurent_series` / `lagrange_inversion` | **Fixed** | Laurent handles pole order 0 and simplifies; `lagrange_inversion` is exact: it expands `f` to rational coefficients, takes `y^n/n · [w^(n-1)] (w/f)^n` by power-series arithmetic (so `sin w` and `e^w − 1` work), and returns errors for non-invertible `f` or non-rational coefficients. Checked against `arcsin`, `ln(1+y)` and the Catalan inverse of `w + w²` |
| B6 | `RecurrenceSolver::solve_linear_constant` order ≥ 2 | **Fixed** | orders 1 and 2 (distinct real roots, repeated root); complex roots and order ≥ 3 return an error |
| B7 | `differential.rs` forcing term drops the Duhamel integral | **Fixed** | forced 2×2 systems add `∫₀ᵗ Φ(t−τ) g(τ) dτ` (an unevaluated integral when no antiderivative is found) instead of the diagonal-entry product; checked against `y1 = 1 − e^{-t}` |
| B8 | `cat_eval.rs` stamps an order-3 enclosure on a linear tangent | **Fixed** | the tangent-line tier is labelled `taylor_order_1` with remainder order 2; with no expansion variable the result is exact |
| B9 | `scheduler.rs` shrinks enclosures by a constant factor without re-evaluating | **Fixed** | `AnytimeHandle` no longer shrinks enclosures by a fixed factor. `refine_with` adopts only a re-evaluated ball that is contained in the current one and strictly tighter; `run_to_budget` takes the re-evaluator. API change: `refine_step`/`run_to_budget` now take a closure |
| B10 | `weil_ideal.rs` Itô multiply arm returns no drift | **Fixed (misdiagnosed)** | the product rule was already right: `second_order` is the `dW²` coefficient, which equals `dt` in Itô calculus. The comment was misleading; `WeilJet::ito_drift` now reads it, with a test of `d(W²) = 2W dW + dt` and `d(W³)` |
| B11 | `kauffman_bracket` hard-codes four knots | **Fixed** | `kauffman_bracket` is a real state sum over all smoothings (up to 22 crossings) with loop counting on the closure; it returns `Result`, so malformed or oversized braids are errors. Verified by the textbook Jones polynomial `t + t³ − t⁴` of the right-handed trefoil, braid-relation and Markov invariance. Convention note: σ₁³ now gives `A⁻⁷ − A⁻³ − A⁵` (the old table held the mirror image) |
| B12 | `quantum.rs` time-ordered evolution not integrated | **Fixed** | `DysonSeries::first_order_term` keeps an unevaluated integral instead of returning the integrand; `first_order_term_between` integrates from `t0` to `t`. Time ordering for higher orders is still not implemented |

### C. Oversimplifications

C5 (Cartan forms hold no
`ExprId` coefficients), C13 (visualizer routing),
C14 is done, see below, are in files that have not
changed since the plan, so they are **Open**. Three were checked directly after the files turned out to have
changed:

- **C2 Fixed.** `RittWuReducer::reduce` is Ritt's differential pseudo-division (derivatives of the leader eliminated highest first using the matching derivative of the divisor, then algebraic pseudo-division by the leader); `characteristic_set` is Ritt's procedure (autoreduced subset, reduce the rest, repeat) and no longer just sorts its input. `DiffPolynomial` gained normalisation and arithmetic, and `differentiate` merges repeated indeterminates (it used to leave `y'` twice in a term). Two existing tests pinned the stub: `{y'', y, y'}` has characteristic set `{y}`, not the three polynomials.
- **C11 Fixed.** Weyl-operator and differential-polynomial conversions no longer lose information silently: `weyl_to_diff_polynomial` returns an error for an operator with an `x` power or several variables, and `TryFrom<DiffPolynomial> for WeylDOperator` accepts only terms linear in one indeterminate (so `y·y'` and `(y')²` are rejected). Linear constant-coefficient operators round-trip exactly.
- **C15 Partly fixed.** LSP hover now evaluates the lines above the cursor first, so earlier definitions are in scope (it evaluated the hovered line alone), and `completions_at` offers names defined above the cursor ahead of the catalogue. Still open: the catalogue itself is a hand-written list, not generated from a central function registry (none exists).
- **C6 Fixed.** `AdjointSensitivitySolver::compute_gradient` is the discrete adjoint of the RK4 forward pass: it recomputes each step's four stages and back-propagates through them, so it returns the exact gradient of the numerical trajectory (it previously ignored the vector field and used a first-order backward Euler on the continuous adjoint, an O(h) error against a fourth-order forward solve). Checked against the analytic gradient with fourth-order convergence and against central finite differences of the discrete solver on a coupled nonlinear system (agreement 1e-8).
- **C1 Fixed.** `IgaStiffnessMatrix::integrate_stiffness_entry_2d` computes the Laplace–Beltrami stiffness `∫ ∇R_a·∇R_b dΩ` from the analytic NURBS derivatives (`NurbsPatchND::rational_basis`, `tangents`) and the surface metric, with Gauss–Legendre quadrature on every knot span (it used a weight of 1.0 on the diagonal and 0.1 off it); `assemble_2d` builds the full matrix, and `metric_determinant_2d` uses analytic tangents instead of forward differences. Verified: the unit-square Q1 matrix (2/3, −1/6, −1/3), invariance under scaling and tilting into 3D, rectangle aspect ratio, row sums zero, and the energy `xᵀKx` of a rational quarter annulus equals its exact area `3π/4` to 1e-9.
- **C13 Fixed (as clarified).** `CodeGenerator::compile_fn1/2/n` compile to bytecode once and run the VM per call (they cloned the graph and substituted into the tree on every call), fall back to the tree evaluator when the VM cannot compile, and return `NaN` for an unbound symbol or too few arguments; the VM compiler reports an unbound symbol instead of reading it as the variable. `RiemannSurface::sqrt_surface_with` takes a `RiemannHeight` (`Real`, `Imaginary`, `Modulus`). `algebra_advanced::gpu` is documented for what it is, multi-threaded CPU kernels (no SIMD intrinsics, no GPU): `GpuComputeKernel::enabled` now really selects the parallel or the sequential path (it was ignored), and `SimdPolyVector` gained `checked_add` / `checked_multiply` because the existing methods wrap on `i64` overflow. `GpuKernelDescriptor::validate` structurally checks generated WGSL (balanced delimiters, `@compute fn main`, workgroup size within the default 256-invocation limit and matching the descriptor, unique bindings, enough buffers); it is not a WGSL compiler, and nothing creates a GPU device or dispatches work. Real SIMD and WebGPU execution are a feature, not a defect, and are now documented as absent.
- **C10 Partly fixed.** The FEA linear solve no longer skips a singular equation and returns zero for it: a pivot that is round-off relative to its own row is an error ("under-constrained"), and the solution is checked against the original system (relative residual 1e-6). This exposed a test that fixed only one translation of a box and had been passing on zeros. Systems above 400 unknowns are now solved from CSR storage by Jacobi-preconditioned conjugate gradients (`CsrMatrix`, stopping on the componentwise backward error, which penalty rows cannot mask; an indefinite or under-constrained matrix is an error). Dirichlet constraints (fixed displacements, prescribed temperatures) are now eliminated exactly instead of penalised, and `FeaSolution::reaction_forces` is filled (it was always zero). Still open: assembly is into a dense matrix (O(n^2) memory).
- **C14 e-graph Fixed.** `cat-egraph`: `add` hash-conses (an existing node returns its class), `rebuild` restores congruence closure (`a = b` merges `f(a)` and `f(b)`, upward to a fixed point) and runs after every saturation round, `extract_min_cost` returns the *total* cost of the cheapest expression (it scored a node by its operator and operand count only, ignoring the operands' own cost), and `extract_term` builds the cheapest tree, computed by a fixed-point over classes so cyclic classes (`x = x + 0`) extract their finite form. With this C14 is complete: unification, wavefront and the e-graph.
- **C14 wavefront Fixed.** `WavefrontSchedule::build` assigns levels by dependency (Kahn's algorithm) so edge listing order no longer matters, and reports cycles; `evaluate` returns a `Result`, evaluates `Add`, `Mul`, `Neg`, `Inv` and `Identity`, and reports a missing wire, an unsupported operation or division by zero instead of substituting `0`. 
- **C14 unification Fixed.** `UnificationEngine::unify` is first-order Martelli–Montanari: structural decomposition of functions, operators and arithmetic nodes, an occurs check, and a most general unifier whose `apply` makes both sides equal. It is syntactic (no commutativity). The CLI proof export no longer uses unification to gate steps; it records the simplification only when the equivalence check confirms it.
- **C8 Fixed.** PDEs are classified as linear, semilinear, quasilinear or fully nonlinear by where the highest derivative sits (Burgers is quasilinear, KdV and sine-Gordon semilinear, Monge–Ampère fully nonlinear); ODEs keep linear/nonlinear. The linearity test is now term by term, so powers and products of derivatives are no longer missed.
- **C7 Fixed.** Stability uses Routh–Hurwitz (continuous) or Schur–Cohn (discrete) on the characteristic polynomial at every order (it was a trace heuristic above 2 states); `frequency_response` and the SISO helper solve `C(sI−A)⁻¹B + D` for any order, MIMO and discrete systems (they returned zeros above 2 states); Ackermann pole placement works for any controllable single-input system. Verified on companion matrices with known roots, partial fractions and the closed-loop characteristic polynomial.
- **A12 (found in the 17 Nov mesh audit) Fixed.** `CustomShape::to_tri_mesh` silently returned a unit cube for implicit surfaces, N-D polytopes and Boolean CSG; it now returns `Result`: meshes, extrusions, implicit surfaces and 3-D triangle polytopes convert, everything else is an error naming what is unsupported. `Mesh3D::extrude_profile` capped the ends with a fan from vertex 0, which is invalid for a non-convex polygon; it is now ear-clipped and orientation-independent (closed, exact volume for concave and clockwise outlines; a bow-tie or zero-area polygon gives an empty mesh). Implicit surfaces are now meshed by `Mesh3D::from_implicit` (marching tetrahedra, shared vertices, closed and outward-oriented when the surface stays inside the box; `to_tri_mesh` uses resolution 40). Boolean CSG works on implicit operands (`CustomShape::implicit_field`: union `min`, intersection `max`, difference `max(a,-b)`, meshed with `Mesh3D::from_field`; lens/union/difference volumes verified). Booleans on triangle meshes, extrusions and polytopes remain unimplemented (an error naming the limit); Phase 13's "CSG" claim should read as the enum only.
- **A13 (found in the stub-marker audit) Fixed.** The public `poly::groebner_basis` (documented as "Simplified Buchberger's algorithm") only removed zero polynomials and returned its input; it now computes the reduced Gröbner basis over the rationals through `poly::exact` (checked on the Cox–Little–O'Shea ideal, the unit ideal, the zero ideal and idempotence). `SparseMonomial::cmp_grevlex` compared degree and then the map of exponents, which is not an admissible graded reverse lexicographic order; it is now the real one (verified admissible: multiplication preserves it). A timing-based test (`Tier-0 latency <= 50 µs`, one wall-clock sample) failed under load; it now takes the best of 200 starts.
- **A14 (found by listing ignored parameters) Fixed.** `CatExprEvaluator::eval_with_fidelity` accepted an `AnytimeBudget` and ignored it, so `budget_exceeded` was always `false`. It now measures the tier and sets `budget_exceeded` when it ran past `hard_timeout`. A tier cannot be interrupted part-way (simplification and bridge verification run to completion), so this is a report, not preemption; a scheduler uses it to stop escalating. Verified for all four fidelity tiers with a 60 s and a zero budget. Remaining parameters that are deliberately unused: `Simplifier`-style placeholders are listed under A5/A6 (already fixed to errors).
- **C4 Partly fixed (gear, airfoil, spring, screw).** A closure audit of every CAD builder (`Mesh3D::boundary_edge_count` / `is_closed`) found the gear (real bore, annular caps), airfoil and spring (end caps) and the screw were not solids. All four are now closed, outward-oriented meshes with checked volumes. The screw (`ThreadedScrew::generate_3d_mesh`) is one height-field grid over polar angle and height: a helical thread of radius `thread_radius` (ISO 60°, Acme 29° or square profile, periodic in the pitch), a plain shank, a hex or round head and fan end faces; its volume equals `π·L·mean(r²) + shank + head` to within 1% (96 angles, 24 rows per pitch). Still open from C4: cycloidal, internal and profile-shift gear builders, and a socket recess or chamfers on the head.
- **C3 Partly fixed.** `GeometricConstraintSolver::solve` now enforces every constraint kind for 2D points (distance, coincident, parallel, perpendicular, angle, tangent, fixed polygon area) with damped Gauss–Newton (Levenberg–Marquardt) over all constraints at once, retried from deterministic perturbed starts when a collinear start stalls. A start that already satisfies the constraints is returned unchanged, so simple chains stay exact. Inconsistent or non-converging sets and constraints on unknown or wrong-kind entities are errors (every other constraint was ignored before). Verified on a 3-4-5 triangle, a square from perpendicularity and equal lengths, parallel/angle/tangent cases and a fixed area. Still open from C3: only `Point2D` unknowns are solved (no 3D or N-D points or circle radii as unknowns), and Collins projection still uses the simplified operator.
  the x-axis; every other constraint is ignored (`_ => {}`).
- **C9 Partly fixed.** `GrobnerProver` now proves equivalences modulo side relations: transcendental subterms (`sin`, `cos`, `sinh`, `cosh`, `exp`) become fresh variables, the relations `sin²+cos²=1`, `cosh²−sinh²=1` and `exp(a)·exp(−a)=1` are extracted automatically for the arguments that occur, and the difference is tested for membership in the exact rational Gröbner ideal (`poly::exact`); step 5 previously reduced modulo the empty ideal. Per-argument: `sin²x + cos²y` is not 1. The plan's statement that no safetensors/Candle code exists was wrong: `heuristic.rs` loads quantised weights through `sml-runtime`. What remains of C9 is that `MctsPolicyHead` still scores with rule-logit heuristics rather than the loaded model.
- **C12 Fixed.** `poly::exact` adds `ExactPoly` (rational coefficients) and `ExactGrobner`: Buchberger with the product criterion, minimalisation and full reduction, giving the unique reduced basis and an exact ideal-membership test. Verified on the Cox–Little–O'Shea examples (grlex `{x², xy, y² − x/2}`; lex `{x − z, y − 2z², z⁴ + z²/2 − 1/4}`) and a non-dyadic case. The older `GrobnerBasis` on `MultiPoly` still uses `f64` and is now documented as numeric. The Frobenius claim is now true: `SymbolicOdeSolver::frobenius_series_at_zero` solves `x²y'' + xP(x)y' + Q(x)y = 0` at the regular singular point 0 for rational Taylor data and rational indicial roots, including the resonant case when no logarithm is needed (checked against `sin x/√x`, `cos x/√x` and `J₁`); complex or irrational roots and logarithmic second solutions are reported, not guessed.
  method appears only in the module doc of `ode/symbolic.rs`: either implement it or remove the claim.

## 3. Fixed since the first audit

17 Nov 2026, after the ledger was verified: B5, B7–B12, A1 (`pade!`), A2 (Clifford), A3 (Cartan input), A4 (`export`), A5, A6 (honest errors), A7, A9 (Lean/Coq export), A10,, B1 (summation), B2 (Caputo kernel), B3 (heat PDE), B4 (linear ODE), B6 (second-order recurrences). The 0.4.0 fixes follow.

Released in 0.3.2 and 0.4.0 (see `CHANGELOG.md`): limits through radicals and variable exponents, two-sided limits
through `abs`/`sign`, formatter precedence and exponent rendering, base and radical merging in the simplifier, Itô
rules for `dW`, `x²−b²` logarithmic integrals, cleared-fraction and `ln` equation solving, composite labelling in
number-theory output, the GUI plan status, and the documentation corrections to phases 8, 28 and 30.

## 4. Open work, in priority order

1. **Make stubs honest before making them real.** A1–A7, A9 and A10 are done, A11 partly. For the rest of A1–A11, either implement the feature or return an explicit
   "not implemented" error and remove the claim from ROADMAP, README and the notebook help. A wrong answer
   presented as a result is worse than a refusal. A11 (GUI export formats) is the smallest; A8 needs a real algorithm.
2. **Numerical and symbolic correctness (B1–B12).** B1–B12 are fixed, with tests in
   `tests/tests/engine/known_defect_tests.rs`. For each remaining item add a failing test, then fix. Each fix needs a test that fails on the old code.
3. **Panic hygiene: done (17 Nov).** The recount found 29 production sites, not 63; reachable ones are removed and
   `clippy::unwrap_used`, `expect_used` and `panic` are enforced as warnings (errors under CI's `-D warnings`).
   Intentional panics remain only in the matrix operators and `Uint` arithmetic, which mirror `std`.
4. **ROADMAP scope with no implementation.** Listed in `ROADMAP.md` section 2.1; decide per item whether to
   build it or drop the claim.
5. **Known engine gaps:** `x*sqrt(x+1)` integration, the real-versus-complex solver convention (make it a typed
   outcome), identity results that omit excluded points, `factor_primes` speed on large semiprimes.
6. **GUI plan** (`GUI_DESIGN_PLAN.md`): web basics (viewport, `100dvh`, local KaTeX in the service worker) →
   route notebook evaluation through the background evaluator → remaining phases.
7. **Engineering standards sweep:** `missing_docs`, property tests, runnable examples.

## 5. Documentation mismatches found while reviewing

- **README example list.** Items such as "Fluid Dynamics & Incompressible Navier-Stokes" (24) and "Nonlinear FEA
  Structural Analysis" (10) are notebook titles. There is no Navier–Stokes solver (see `ROADMAP.md` section 2.1),
  and the FEA solver is dense and small-scale (C10). Retitle or describe what each notebook actually computes.
- **`DOCUMENTATION_STANDARDS.md`.** Its checklist (every public item has a KaTeX formula, zero doc warnings,
  doctests pass) is enforced in CI only for doc warnings and doctests. The workspace sets no `missing_docs` lint,
  so the "every public symbol is documented" rule is a convention, not a gate (open-work item 7).
- **`AUDIT-2026-11-15.md`** is a snapshot at `23f508b`. Its competitor comparison was removed by the reconciliation
  edit; its findings are tracked in section 3 above, not maintained in place.
- **`AUDIT-PANICS-AND-UNWRAPS.md`** counted 63 sites; the 17 Nov recount found 29 and the doc now says so.

