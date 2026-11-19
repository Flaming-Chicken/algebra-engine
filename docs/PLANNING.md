# URAE Master Plan

*Checked 19 Nov 2026 against `dev` after v0.6.0 (the unreleased changes are listed in [`../CHANGELOG.md`](../CHANGELOG.md)).
This is the one page that says where the project is going and in what order. Defects and stubs are in
[`LEDGER.md`](LEDGER.md); how the system is built, and the target it grows toward, is in [`DESIGN.md`](DESIGN.md).*

## 1. The documents

| Document | Role | Kind |
| :--- | :--- | :--- |
| **This file** | Direction, milestones, work queue, process | Plan (maintained) |
| [`DESIGN.md`](DESIGN.md) | Invariants, architecture as built, subsystem designs, how the target grows from it | Design (maintained) |
| [`LEDGER.md`](LEDGER.md) | Status and evidence for every known stub, defect and oversimplification | Status (maintained) |
| [`../ROADMAP.md`](../ROADMAP.md) | What each of the 31 phases was meant to deliver, with an evidence-based status per phase | Scope and status by phase |
| [`specs/ARCHITECTURE_SPEC.md`](specs/ARCHITECTURE_SPEC.md) | The long-range target: unified numerics, CAD, multiphysics, AI, certified arithmetic, eight delivery waves | Specification (not a status report) |
| [`specs/GUI_DESIGN_PLAN.md`](specs/GUI_DESIGN_PLAN.md) | The GUI rework: responsive layout, accessibility, documentation registry | Specification |
| [`specs/GEAR_PROFILE_CURVES.md`](specs/GEAR_PROFILE_CURVES.md), [`specs/symbolic/`](specs/symbolic), [`specs/physics_informed_benchmarks/`](specs/physics_informed_benchmarks) | Mathematical references for the gear generator, lemma mining and MCTS, and the physics-informed benchmark suite | Reference |
| [`audits/`](audits) | The three audit reports the ledger was built from | Historical evidence; not maintained in place |
| [`../DOCUMENTATION_STANDARDS.md`](../DOCUMENTATION_STANDARDS.md), [`../CHANGELOG.md`](../CHANGELOG.md), [`../CONTRIBUTING.md`](../CONTRIBUTING.md) | Rustdoc conventions, release notes, contribution rules | Reference |

Reading order for a newcomer: `DESIGN.md` sections 1 to 3 (what it is), this file sections 2 to 4 (where it is going),
then the ledger entries for whatever you are about to touch. [`README.md`](README.md) in this folder has the same map
with the rules for keeping it true.

## 2. Where the project stands

- **Code.** Version 0.6.0 released; 20 workspace members (19 crates and the test crate); about 111,000 lines of Rust in
  `crates/`. 926 tests (unit, integration and doctests; one ignored) pass on `dev`, and CI runs lint, tests with coverage, WebAssembly
  tests, a mobile build check and a licence check on every pull request.
- **What works well.** Symbolic algebra, limits, series, summation with proofs, solving, exact Gröbner bases, exact
  cylindrical decomposition to three variables, constraint-based CAD, closed solid meshes for gears, screws, springs
  and wings, a finite-element suite with sparse solvers, special functions to about 12 digits, ODE and PDE solvers, the
  egui notebook and the other frontends.
- **What is not right yet.** The ledger lists 53 catalogued items (37 fixed, 11 partly fixed, 0 open, 2 stale, 1
  partly different, 2 not verified) plus the audit extension's tracks, which are mostly unverified. No catalogued
  wrong answer is reproduced and open any more; no read-confirmed item is open either.
- **What is thin.** Several advanced-domain modules exist with tests but little depth (ledger `A13`–`A16`, `B16`–`B19`),
  and `ROADMAP.md` section 2.1 lists scope named in the plans that has no code.

## 3. Direction

Correctness first, then honesty, then capability.

1. **Correctness.** The engine answers a very wide range of questions; the audits show it still returns confident wrong
   answers in a few well-defined places. Those are worth more than any new feature. The unverified part of the audit is
   checked by running it, so that every item has a verdict.
2. **Honesty.** Every public feature either works or fails with an error that says it is not implemented. Documents
   never claim more than the code does; the ledger is the arbiter.
3. **Capability.** Only then does the architecture specification's wave plan start, beginning with its foundations,
   built on what already exists (exact CAD, solid meshes, sparse FEA, certificates).

Principles that follow from [`DESIGN.md`](DESIGN.md) section 2: exact before floating point; refuse rather than guess;
test-first fixes with an oracle that does not share the implementation; one source of status.

## 4. Milestones

Versions are targets, not promises. A milestone ends when its exit gate is met and a release is cut.

| Milestone | Target | Scope | Exit gate |
| :--- | :--- | :--- | :--- |
| **M1 Correct** | 0.7.0 | Fix the reproduced and read-confirmed wrong answers (done); finish the surreal unification (work queue 3). Verify the rest of the audit extension. Recount production panic sites. | No reproduced wrong answer in the ledger; every catalogued item has a verdict; the panic count is measured |
| **M2 Honest** | 0.8.0 | Remaining stubs implemented or turned into explicit errors; `ROADMAP.md` section 2.1 and the README example list matched to the code; the solver's real-versus-complex convention a typed outcome; identity results that name excluded points; `missing_docs` as a lint | Every public feature works or errors; `ROADMAP.md`, README and ledger agree; the GUI web basics (viewport, `100dvh`, local KaTeX) shipped |
| **M3 Foundations** | 0.9.0 | Architecture wave 1: carrier and operator traits, numerical sparse types with Krylov and multigrid solvers, compensated arithmetic and outward rounding, `const` and WGSL lowering | Wave 1 exit gate in the specification, with the baseline tests unchanged |
| **M4 Geometry** | 1.0.0 | Wave 2: multi-representation CAD (B-Rep, NURBS, SDF), exact offsets, a file-format hub, differentiable CAD; extends the exact CAD and solid meshes already shipped | Wave 2 exit gate |
| **M5 Discretisations** | after 1.0 | Waves 3 and 4: FVM, BEM, IGA extensions, VEM, HDG, FEEC; particle and kinetic methods, cloth and multibody | Wave 3 and 4 exit gates |
| **M6 Composition** | after 1.0 | Waves 5 and 6: cross-method couplers, the composition DSL, the automatic orchestrator, the subsystem registry | Wave 5 and 6 exit gates |
| **M7 Breadth and AI** | after 1.0 | Waves 7 and 8: multi-solution discovery, expansions, complete CAS, physics domains, SciML, AI analysis, formal kernel | Wave 7 and 8 exit gates |

The GUI plan runs alongside M2 and later as capacity allows; its own status table is the reference. The
engineering-standards sweep (`missing_docs`, property tests, runnable examples) is part of M2.

## 5. Work queue

In order. Each item starts with a failing test and ends with a ledger update.

1. ~~`F7` Poisson Dirichlet lifting, `F8` B-spline end point, `F10` rank and exponential, `F5` float hashing, `B13` Smith normal form, `B15` Shirshov factorisation~~ (done, 19 Nov 2026).
2. ~~`B16` exact surreal values, `C16` polynomial systems by branching, `A14` exact Betti numbers~~ (done, 19 Nov 2026; a Conway normal form for true transfinite numbers and a rational univariate representation remain as design work).
3. ~~`A13` equivalence tiers, `B17` Colombeau products, `B18` tangent functor rules, `B19` dimensions of `^` and calls, `C17` domain categories~~ (done, 19 Nov 2026).
4. Unify the two surreal engines (`algebra-advanced`, `cat-numbers`) behind one exact representation.
5. The rest of `F5` and `F10`, then the verification pass below.
6. The verification pass: `A15`, `A16`, the rest of `B20`, Tracks `E`, `F1–F3`, `F9`, `F11–F16`, `G–J`; a fresh scan of production panic sites.
7. Then M2.

Also open and not scheduled: integrals of `x·sqrt(x²+1)` (a radical of a non-linear base); factoring beyond about 50-bit
factors (an elliptic-curve method); the remainder of `R1` (arbitrary CAD surface meshes as solids).

## 6. Process

- **Branches.** All work goes to `dev` through a pull request; `main` holds reviewed releases only and is an ancestor of
  `dev`. `dev` is never deleted, so a release pull request is merged without `--delete-branch`.
- **A change** is a branch from `origin/dev`, a pull request, and a green CI run before merge. A failing check is fixed,
  not skipped. The Android and iOS check depends on a hosted macOS runner and has been seen to time out without running;
  that is an infrastructure failure, not a code failure, and the job can be rerun.
- **Tests first.** A defect fix adds a test that fails on the old code, with an oracle independent of the
  implementation (tabulated values, exact identities, a second algorithm). `cargo fmt`, `cargo clippy --workspace
  --all-targets --all-features -- -D warnings` and `cargo test --workspace` pass before a pull request opens.
- **Documents.** A change that fixes or finds a defect updates its ledger entry in the same pull request, and
  `CHANGELOG.md` under `[Unreleased]`. A status in any other document is a link to the ledger, not a copy. Where two
  documents disagree, the code wins, then the ledger, then this file.
- **Releases.** A bump pull request sets the workspace version and turns `[Unreleased]` into a dated section; a
  `dev` to `main` pull request carries the release (CI checks the version bump); `main` is then merged back into `dev`
  so the histories stay linked.
- **Audits.** An audit finding enters the ledger as *not verified* until it has been run (*reproduced*) or read
  (*read*); only then is it work.

## 7. Risks and open questions

- **Scope of the architecture specification.** It is far larger than any milestone; the exit gates of M3 and M4 are the
  first real test of whether the carrier design survives contact with the existing code (`DESIGN.md` section 6 lists the
  decisions to settle first).
- **Unverified audit tracks.** More than 50 catalogued claims have never been run. Until they are, the true defect count is
  unknown; M1 spends its effort there deliberately.
- **Numerical accuracy claims.** Numerical routines are verified to 12 to 15 digits against tables and identities, not to
  a certified bound; the certified-arithmetic spine in the specification (wave 1 and later) is the route to that.
- **CI capacity.** The mobile build job's runner shortage delays merges; it is rerun, not skipped, and the job's
  failure reason is checked before a merge.
