# URAE Planning Index and Status Ledger

*Checked 17 Nov 2026 against `dev` at v0.4.0 (`e483860`). This is the one page that says what is planned, what has
been done, and what is still open. Every status below was checked against the code, not copied from another plan.*

## 1. Planning documents and what each is for

| Document | Role | Status |
| :--- | :--- | :--- |
| [`ROADMAP.md`](../ROADMAP.md) | What each of the 31 phases was meant to deliver, with an evidence-based status per phase | Current. Section 2 carries the status; section 5 lists intended scope, not shipped scope |
| **This file** | Index, defect ledger, open work in priority order | Current |
| [`AUDIT-2026-11-15.md`](AUDIT-2026-11-15.md) | First quality audit (correctness, efficiency, competitors) | Historical. Findings were fixed in 0.3.2 and 0.4.0; see section 3 |
| `CODEBASE_AUDIT_AND_REMEDIATION_PLAN.md` | Stub/defect catalogue (A1–A11, B1–B12, C1–C15) with remediation roadmap, written at v0.3.2 | Source of the ledger in section 2; item status here supersedes it. Not yet on `dev` when this page was written |
| [`AUDIT-PANICS-AND-UNWRAPS.md`](AUDIT-PANICS-AND-UNWRAPS.md) | Inventory of production `panic!`/`unwrap` sites | Open work, see section 4 |
| [`GUI_DESIGN_PLAN.md`](GUI_DESIGN_PLAN.md) | Five-phase GUI rework | Unstarted apart from the workbench panel; its own status table is the reference |
| [`../DOCUMENTATION_STANDARDS.md`](../DOCUMENTATION_STANDARDS.md) | Rustdoc/KaTeX conventions | Reference, not a plan |
| [`../CHANGELOG.md`](../CHANGELOG.md) | What shipped, per release | Reference |
| Portfolio-level plan | `PLAN.md` in the parent workspace; its section 4.1 summarises this project | Links here for detail |

Reading order for newcomers: ROADMAP section 2 (what exists) → this file (what is wrong or missing) → the audit
documents only when a finding needs its original evidence.

## 2. Stub and defect ledger

"Open" means the cited code is unchanged since the plan was written and still shows the described behaviour.
"Re-check" means the file changed after the plan, so the description may be stale; it has not been re-verified
item by item.

### A. Silent stubs (feature claims that return placeholder output)

| ID | Item | Status | Evidence at `dev` |
| :--- | :--- | :--- | :--- |
| A1 | `pade!` macro | **Open** | `macros.rs:110` computes a Taylor series and then returns the symbol quotient `P_num / Q_den`; it never builds a Padé approximant |
| A2 | `MathOperation::Clifford` products | Re-check | `executor.rs` changed after the plan |
| A3 | `MathOperation::Cartan` ignores the form string | **Open** | `executor.rs:1822` builds `DifferentialForm::dx(0, …)` regardless of `form1_str` |
| A4 | `System` Log/Preset/Export/Clear, `AiQuery` echo only | Re-check | `executor.rs` changed after the plan |
| A5 | `creative_telescoping_integral` | **Open** | `risch.rs:129` ignores its four arguments and returns the fixed Gaussian ODE |
| A6 | `puiseux_series` | Re-check | `series.rs` changed (Laurent path only) |
| A7 | `EmpiricalDistributionMut::{pdf, cdf}` | **Open** | `distributions.rs:56,60` return constants |
| A8 | `ZeilbergerAlgorithm` / `AlmkvistZeilberger` | **Open** | Three hard-coded showcase proofs; no general creative telescoping |
| A9 | `ProofTrace::export_lean4/export_coq` | **Open** | Static boilerplate ending in `sorry`; trace ignored |
| A10 | Agent offline fallback | **Open** | `agent.rs:222` substitutes the fixed `f(x)` explanation |
| A11 | GUI `.tex/.py/.lean` export, presentation export, `WorkspaceTab` routing | **Open** | `app.rs:1327` wraps the raw buffer; no formatting |

### B. Formula and algorithm defects

| ID | Item | Status | Evidence at `dev` |
| :--- | :--- | :--- | :--- |
| B1 | `summation` wrong on non-trivial summands | Re-check | `calculus.rs` changed after the plan |
| B2 | `caputo_derivative` wrong for α ≠ 0.5 | Re-check | `calculus.rs` changed; the `Gamma(n − α)` prefactor is still built the same way |
| B3 | `solve_pde_heat_1d` | **Open** | `solver.rs:456`: spatial part is `cos(k_x)` with no `x`; decay uses `k_x`, not `k_x²` |
| B4 | `solve_ode_linear` with `p = 0` | Re-check | `solver.rs` changed; the method now simplifies its result |
| B5 | `laurent_series` / `lagrange_inversion` | **Partly fixed** | Laurent now handles pole order 0 and simplifies; Lagrange inversion unchanged |
| B6 | `RecurrenceSolver::solve_linear_constant` order ≥ 2 | Re-check | `series.rs` changed (Laurent only); the order ≥ 2 branch is unchanged |
| B7 | `differential.rs` forcing term drops the Duhamel integral | **Open** | File unchanged |
| B8 | `cat_eval.rs` stamps an order-3 enclosure on a linear tangent | Re-check | File changed |
| B9 | `scheduler.rs` shrinks enclosures by a constant factor without re-evaluating | Re-check | File changed |
| B10 | `weil_ideal.rs` Itô multiply arm returns no drift | **Open** | File unchanged |
| B11 | `kauffman_bracket` hard-codes four knots | **Open** | File unchanged |
| B12 | `quantum.rs` time-ordered evolution not integrated | **Open** | File unchanged |

### C. Oversimplifications

C1 (IGA Jacobian), C2 (differential pseudo-remainder), C4 (gear and screw builders), C5 (Cartan forms hold no
`ExprId` coefficients), C6 (neural-ODE adjoint), C7 (control: Routh–Hurwitz, Ackermann), C8 (PDE
classification), C10 (FEA solver: dense, no CG or CSR), C11 (interconnect indexing), C13 (visualizer routing),
C14 (e-graph extraction, wavefront, unification) and C15 (LSP completion catalogue) are in files that have not
changed since the plan, so they are **Open**. C3 (CAD projection), C9 (lemma miner, MCTS) and C12 (Gröbner
coefficients) are in files that changed and are **Re-check**.

## 3. Fixed since the first audit

Released in 0.3.2 and 0.4.0 (see `CHANGELOG.md`): limits through radicals and variable exponents, two-sided limits
through `abs`/`sign`, formatter precedence and exponent rendering, base and radical merging in the simplifier, Itô
rules for `dW`, `x²−b²` logarithmic integrals, cleared-fraction and `ln` equation solving, composite labelling in
number-theory output, the GUI plan status, and the documentation corrections to phases 8, 28 and 30.

## 4. Open work, in priority order

1. **Make stubs honest before making them real.** For A1–A11, either implement the feature or return an explicit
   "not implemented" error and remove the claim from ROADMAP, README and the notebook help. A wrong answer
   presented as a result is worse than a refusal. `pade!` (A1), Cartan (A3) and the heat PDE (B3) are the smallest.
2. **Numerical and symbolic correctness (B1–B12).** Write a test per "Re-check" item first, then fix what fails.
   Each fix needs a test that fails on the old code.
3. **Panic hygiene.** About 63 production `panic!`/`unwrap` sites per `AUDIT-PANICS-AND-UNWRAPS.md`; replace
   them with typed errors, then enable `clippy::unwrap_used` for non-test code.
4. **ROADMAP scope with no implementation.** Listed in `ROADMAP.md` section 2.1; decide per item whether to
   build it or drop the claim.
5. **Known engine gaps:** `x*sqrt(x+1)` integration, the real-versus-complex solver convention (make it a typed
   outcome), identity results that omit excluded points, `factor_primes` speed on large semiprimes.
6. **GUI plan** (`GUI_DESIGN_PLAN.md`): web basics (viewport, `100dvh`, local KaTeX in the service worker) →
   route notebook evaluation through the background evaluator → remaining phases.
7. **Engineering standards sweep:** `missing_docs`, property tests, runnable examples.
