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
code. No item remains unchecked. Reproduced defects have an `#[ignore]`d test in
`tests/tests/engine/known_defect_tests.rs` that asserts the correct behaviour; remove the `#[ignore]` when fixing.

### A. Silent stubs (feature claims that return placeholder output)

| ID | Item | Status | Evidence at `dev` |
| :--- | :--- | :--- | :--- |
| A1 | `pade!` macro | **Open** | `macros.rs:110` computes a Taylor series and then returns the symbol quotient `P_num / Q_den`; it never builds a Padé approximant |
| A2 | `MathOperation::Clifford` products | **Open** | `executor.rs:1968` ignores `a_coords`/`b_coords`; returns the string `a · b + a ∧ b` |
| A3 | `MathOperation::Cartan` ignores the form string | **Open** | `executor.rs:1822` builds `DifferentialForm::dx(0, …)` regardless of `form1_str` |
| A4 | `System` Log/Preset/Export/Clear, `AiQuery` echo only | **Open** | `executor.rs:2730-2748` format a confirmation string and change no state |
| A5 | `creative_telescoping_integral` | **Open** | `risch.rs:129` ignores its four arguments and returns the fixed Gaussian ODE |
| A6 | `puiseux_series` | **Open** | `series.rs:119` ignores `_expr`; emits symbolic coefficients `a_p_q` |
| A7 | `EmpiricalDistributionMut::{pdf, cdf}` | **Open** | `distributions.rs:56,60` return constants |
| A8 | `ZeilbergerAlgorithm` / `AlmkvistZeilberger` | **Open** | Three hard-coded showcase proofs; no general creative telescoping |
| A9 | `ProofTrace::export_lean4/export_coq` | **Open** | Static boilerplate ending in `sorry`; trace ignored |
| A10 | Agent offline fallback | **Open** | `agent.rs:222` substitutes the fixed `f(x)` explanation |
| A11 | GUI `.tex/.py/.lean` export, presentation export, `WorkspaceTab` routing | **Open** | `app.rs:1327` wraps the raw buffer; no formatting |

### B. Formula and algorithm defects

| ID | Item | Status | Evidence at `dev` |
| :--- | :--- | :--- | :--- |
| B1 | `summation` wrong on non-trivial summands | **Open, reproduced** | Any non-symbol summand is treated as constant: `sum k^2` returns `k^2 · (n - 1 + 1)`. Test `known_defect_summation_of_a_non_constant_summand` |
| B2 | `caputo_derivative` wrong for α ≠ 0.5 | **Open** | `calculus.rs:900`: kernel exponent is `n − α`; the Caputo kernel is `(t − τ)^(n − α − 1)`. They agree only at α = n − ½ |
| B3 | `solve_pde_heat_1d` | **Open** | `solver.rs:456`: spatial part is `cos(k_x)` with no `x`; decay uses `k_x`, not `k_x²` |
| B4 | `solve_ode_linear` with `p = 0` | **Open, reproduced** | `y' = 1` returns `1 / 0`. Test `known_defect_linear_ode_with_zero_coefficient` |
| B5 | `laurent_series` / `lagrange_inversion` | **Partly fixed** | Laurent now handles pole order 0 and simplifies; Lagrange inversion unchanged |
| B6 | `RecurrenceSolver::solve_linear_constant` order ≥ 2 | **Open, reproduced** | Fibonacci returns `0`. Test `known_defect_second_order_recurrence` |
| B7 | `differential.rs` forcing term drops the Duhamel integral | **Open** | File unchanged |
| B8 | `cat_eval.rs` stamps an order-3 enclosure on a linear tangent | **Open** | `cat_eval.rs:195-224` builds `f(a) + f'(a)(x − a)` and labels it `taylor_order_3` |
| B9 | `scheduler.rs` shrinks enclosures by a constant factor without re-evaluating | **Open** | `scheduler.rs:173-188` multiplies spans by 0.4 and radii by 0.5 |
| B10 | `weil_ideal.rs` Itô multiply arm returns no drift | **Open** | File unchanged |
| B11 | `kauffman_bracket` hard-codes four knots | **Open** | File unchanged |
| B12 | `quantum.rs` time-ordered evolution not integrated | **Open** | File unchanged |

### C. Oversimplifications

C1 (IGA Jacobian), C2 (differential pseudo-remainder), C4 (gear and screw builders), C5 (Cartan forms hold no
`ExprId` coefficients), C6 (neural-ODE adjoint), C7 (control: Routh–Hurwitz, Ackermann), C8 (PDE
classification), C10 (FEA solver: dense, no CG or CSR), C11 (interconnect indexing), C13 (visualizer routing),
C14 (e-graph extraction, wavefront, unification) and C15 (LSP completion catalogue) are in files that have not
changed since the plan, so they are **Open**. Three were checked directly after the files turned out to have
changed:

- **C3 Open.** `cad/mod.rs:581-612` solves only `Distance` and `Coincident` constraints, by placing a point along
  the x-axis; every other constraint is ignored (`_ => {}`).
- **C9 Open.** No `safetensors` or Candle code exists; `mcts.rs` computes its policy head from the lemma registry.
- **C12 Open.** `poly/grobner.rs` uses `f64` coefficients (`1.0 / lt.coeff`), not exact rationals, and the Frobenius
  method appears only in the module doc of `ode/symbolic.rs`: either implement it or remove the claim.

## 3. Fixed since the first audit

Released in 0.3.2 and 0.4.0 (see `CHANGELOG.md`): limits through radicals and variable exponents, two-sided limits
through `abs`/`sign`, formatter precedence and exponent rendering, base and radical merging in the simplifier, Itô
rules for `dW`, `x²−b²` logarithmic integrals, cleared-fraction and `ln` equation solving, composite labelling in
number-theory output, the GUI plan status, and the documentation corrections to phases 8, 28 and 30.

## 4. Open work, in priority order

1. **Make stubs honest before making them real.** For A1–A11, either implement the feature or return an explicit
   "not implemented" error and remove the claim from ROADMAP, README and the notebook help. A wrong answer
   presented as a result is worse than a refusal. `pade!` (A1), Cartan (A3) and the heat PDE (B3) are the smallest.
2. **Numerical and symbolic correctness (B1–B12).** B1, B4 and B6 already have
   `#[ignore]`d tests; add one per remaining item, then fix. Each fix needs a test that fails on the old code.
3. **Panic hygiene.** About 63 production `panic!`/`unwrap` sites per `AUDIT-PANICS-AND-UNWRAPS.md`; replace
   them with typed errors, then enable `clippy::unwrap_used` for non-test code.
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
- **`AUDIT-PANICS-AND-UNWRAPS.md`** counts sites at v0.3.2; recount before using the 63 figure as a target.

