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

C1 (IGA Jacobian), C4 (gear and screw builders), C5 (Cartan forms hold no
`ExprId` coefficients), C6 (neural-ODE adjoint), C10 (FEA solver: dense, no CG or CSR), C13 (visualizer routing),
C14 (e-graph extraction, wavefront; unification is done, see below) are in files that have not
changed since the plan, so they are **Open**. Three were checked directly after the files turned out to have
changed:

- **C2 Fixed.** `RittWuReducer::reduce` is Ritt's differential pseudo-division (derivatives of the leader eliminated highest first using the matching derivative of the divisor, then algebraic pseudo-division by the leader); `characteristic_set` is Ritt's procedure (autoreduced subset, reduce the rest, repeat) and no longer just sorts its input. `DiffPolynomial` gained normalisation and arithmetic, and `differentiate` merges repeated indeterminates (it used to leave `y'` twice in a term). Two existing tests pinned the stub: `{y'', y, y'}` has characteristic set `{y}`, not the three polynomials.
- **C11 Fixed.** Weyl-operator and differential-polynomial conversions no longer lose information silently: `weyl_to_diff_polynomial` returns an error for an operator with an `x` power or several variables, and `TryFrom<DiffPolynomial> for WeylDOperator` accepts only terms linear in one indeterminate (so `y·y'` and `(y')²` are rejected). Linear constant-coefficient operators round-trip exactly.
- **C15 Partly fixed.** LSP hover now evaluates the lines above the cursor first, so earlier definitions are in scope (it evaluated the hovered line alone), and `completions_at` offers names defined above the cursor ahead of the catalogue. Still open: the catalogue itself is a hand-written list, not generated from a central function registry (none exists).
- **C14 unification Fixed.** `UnificationEngine::unify` is first-order Martelli–Montanari: structural decomposition of functions, operators and arithmetic nodes, an occurs check, and a most general unifier whose `apply` makes both sides equal. It is syntactic (no commutativity). The CLI proof export no longer uses unification to gate steps; it records the simplification only when the equivalence check confirms it.
- **C8 Fixed.** PDEs are classified as linear, semilinear, quasilinear or fully nonlinear by where the highest derivative sits (Burgers is quasilinear, KdV and sine-Gordon semilinear, Monge–Ampère fully nonlinear); ODEs keep linear/nonlinear. The linearity test is now term by term, so powers and products of derivatives are no longer missed.
- **C7 Fixed.** Stability uses Routh–Hurwitz (continuous) or Schur–Cohn (discrete) on the characteristic polynomial at every order (it was a trace heuristic above 2 states); `frequency_response` and the SISO helper solve `C(sI−A)⁻¹B + D` for any order, MIMO and discrete systems (they returned zeros above 2 states); Ackermann pole placement works for any controllable single-input system. Verified on companion matrices with known roots, partial fractions and the closed-loop characteristic polynomial.
- **C3 Open.** `cad/mod.rs:581-612` solves only `Distance` and `Coincident` constraints, by placing a point along
  the x-axis; every other constraint is ignored (`_ => {}`).
- **C9 Open.** No `safetensors` or Candle code exists; `mcts.rs` computes its policy head from the lemma registry.
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

