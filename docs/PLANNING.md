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
- **What is not right yet.** The ledger lists 53 catalogued items (39 fixed, 12 partly fixed, 0 open, 2 stale, 0
  partly different, 0 not verified) plus the audit extension's tracks, which are mostly unverified. No catalogued
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
| **M2 Honest** | 0.8.0 | Remaining stubs implemented or turned into explicit errors; `ROADMAP.md` section 2.1 and the README matched to the code (done 19 Nov 2026: the README's Risch and Gruntz claims now match `docs/DESIGN.md`, new capabilities are listed, test counts are current; section 2.1 was re-checked and stands); the solver's real-versus-complex convention a typed outcome (done for the `solve` command, 19 Nov 2026: real/non-real counts and a principal-branch marker; a typed `SolveOutcome` carrying them is still to do); identity results that name excluded points (done for simplification, 19 Nov 2026: `simplify_with_conditions`; shown by the CLI, the JSON API and the notebook's domain info); `missing_docs` as a lint (ratchet started 19 Nov 2026: denied in `urae`, `cat-tangent` and `cat-numbers`; 2,500 items remain, in `algebra-engine` 843, `urae-notebook` 630, `algebra-core` 599, `algebra-advanced` 116, `cat-core` 115 and the rest under 50 each; each crate is denied as it reaches zero) | Every public feature works or errors; `ROADMAP.md`, README and ledger agree; the GUI web basics (viewport, `100dvh`, local KaTeX) shipped |
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
4. ~~Exact values in both surreal engines~~ (done, 19 Nov 2026); a shared Conway normal form for true transfinite numbers remains design work (M3).
5. The rest of `F5` and `F10`, then the verification pass below.
6. The verification pass: Galois groups of degree 5 and above (`A15`), Tracks `E`, `F1–F3`, `F9`, `F11–F16`, `G–J`; stack depth and the FFI and WASM boundaries of Track `E` (the production panic recount is done).
7. Then M2.

**Usability probe loop (running since 20 Nov 2026).** Alongside the queue above, each round feeds the engine a batch of
ordinary inputs from one field (calculus, linear algebra, number theory, solving, units, statistics, ...), reads every
answer against what a mathematician expects, and treats three outcomes as defects in this order: a wrong answer, an
input echoed back unevaluated, an error for something that should work. Each defect gets a test with an independent
oracle (a numerical value, a defining identity, a brute-force count, published digits), then a ledger entry
(`R14`-`R104` so far). It found more wrong answers in a day than the audit documents did, and most were in the plain
spellings a new user types first. The open list, by field:

- **Calculus and series.** Expansions at infinity of functions that are not power series in 1/x (`atan(x)`,
  `sqrt(x^2+1)/x`); integrals with non-linear arguments, `sqrt` of cubics and irreducible cubic, quartic or sextic
  denominators (`1/(x^3+x+1)` raises an error); periodic inequalities; `d/dx x^2 + 1` without brackets; the
  numerator form `∂²f/∂x∂y`; total derivatives; `converges` and `radius_of_convergence` of a sum; residues at symbolic
  points and of irrational value.
- **Special functions.** Bessel `Y`, `K` and non-integer order, `polygamma` at negative `x`, `f_quantile` and discrete
  quantiles, `poisson_cdf` in closed form, svd factors `U` and `V` (singular values exist).
- **Numbers and algebra.** Roots of sums (`sqrt(3 + 2 sqrt 2)`), `4^(1/4)` to `sqrt(2)`, symbolic roots
  (`sqrt(x^2*y)`), `exp(x)/exp(y)`, `oo * 0`, `lcm` and `gcd` of multivariate polynomials, `acos(1/sqrt 2)` as a
  plain expression, binomial roots for n = 5, 7, 9, 10, Galois groups of degree 5 and up, `groebner`, the Jordan transformation matrix and
  irrational eigenvalues (`R93` does the form for rational spectra), `schur`, symbolic `sqrtm` and `logm` (`hessenberg` is done, `R94`; numeric `sqrtm`, `R95`), a simplified `qr`. Done (`R82`): pseudoinverse, least squares, Gram-Schmidt, adjugate, `expm`,
  `cond` and the matrix predicates on rational matrices.
- **Solving.** General solutions of trigonometric equations (`+ 2 pi k`), numerical values beside a `RootOf`,
  inequalities with a nonlinear objective (KKT), integer and quadratic programs, `extrema` of several variables.
- **ODEs, transforms and recurrences.** Done: first-order separable, linear, Bernoulli, homogeneous, exact and
  integrating-factor equations; constant-coefficient and Euler second-order equations; homogeneous and forced linear
  systems by the inverse Laplace transform; Clairaut equations (lines and singular envelope); Riccati equations with a particular solution `c x^k` (`R96`); d'Alembert equations (`R105`). Missing: other Riccati particular solutions, other integrating factors, an
  explicit `tan` for `atan(y) = ...`, nonlinear systems, time-varying coefficients, forcing with a non-rational transform,
  irreducible cubic factors in the inverse Laplace transform, the inverse Fourier transform, repeated complex poles and left-sided sequences in the inverse Z transform (real and simple complex poles are done, `R99`), Z-transform of
  `sin`/`cos`, boundary value problems, systems for `rk4`, adaptive steps, variable-coefficient and nonlinear
  recurrences, exact `fft`.
- **Statistics, probability and finance.** Done: the inference commands (`t_test`, `chi2_test`, `anova`, `fisher_exact`,
  correlation, `kendall`, `mann_whitney`, `wilcoxon`, `kruskal`), `regress`, `mregress`, `logit`, `poisson_regress` and `predict`. Likelihood-ratio tests for logistic and Poisson models are done (`R97`). Missing:
  offsets, overdispersion, AIC/BIC and other generalised linear models, weighted regression, residual diagnostics,
  `geometric_cdf`, sentence forms such as
  `P(X=3) binomial n=10 p=0.5`, joint and conditional distributions from tables; amortisation with extra payments,
  `xirr` with dates, bond pricing.
- **Discrete, logic and sets.** Done: weighted and directed graphs (shortest paths, spanning trees, flows,
  components, topological order), Euler and Hamilton paths, maximum matching. Isomorphism and automorphism counts are done (`R91`). Planarity is done for small graphs (`R103`). The assignment problem (bipartite weighted matching) is done (`R104`). Missing: weighted matching on general graphs, `pascal_row`,
  `partitions_list`, normal forms and simplification of formulas, quantifiers, `~a` and `base_convert` in arbitrary
  bases, `multiset_permutations`.
- **Input forms.** Quantified formulas, LaTeX tables, `align` and `\text{}` in formulas, English with several clauses
  (`sum of 1 to 10`, `area of circle radius 3`), `sin x cos x`, unit arithmetic across dimensions (`3 m * 2 s`),
  `|x| + 1`, the decimal tail for commands typed with decimals (`integrate(sin(x), x, 0, 0.5)`).
- **Structure.** The two parser tables of command names (`parser.rs` and `numeric_commands.rs`) must stay in step; one
  table would remove a class of mistakes. Ten command hooks at the top of `UraeSession::execute_line` are one ordered
  registry (`COMMAND_HOOKS` in `session.rs`); linear programs and Lagrange multipliers, element assignment and indexing,
  matrix variables and the `input_text` rewrites still sit inline in an order that matters and should join it. Numbers
  beyond 64 bits travel through the simplifier's e-graph as symbols named by their text (fixed on the way out in `R77`;
  a digit bound and a real big-number node would remove the class). The `missing_docs` backlog.
- **Process.** The unreleased work is a stack of pull requests waiting for the `v0.7.0` release PR to be merged; the
  ledger and changelog entries of one pull request sit next to those of the next, so the stack merges in order or not
  at all. A new item should not start from `dev` while the stack is open.

**What the loop taught (so far).** Wrong answers came almost only from code that reinterprets input: an English or
shorthand rewrite, a retry after a parse failure, or an alias that shadowed an existing function. Each such rewrite
needs a guard (never retry across a bare function name or a multi-letter word, never join two numbers) and a test that
pins what it must refuse as well as what it accepts. The cheap, high-yield probes are the spellings a first user types:
`d/dx x^3`, `2 x`, `0xFF`, `5 km to m`, `sin(0.5)`.
A second cheap check is an audit that substitutes every returned solution back into its equation at several points (
ODE solutions, roots, antiderivatives by differentiation): it found wrong answers in the standard ODE solver that no input-spelling probe
had touched (`R75`, `R76`). Both checks belong in the test suite as oracles, not only in the loop.

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
