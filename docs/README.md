# URAE documentation map

Three maintained documents say what the project is, where it is going and what is wrong. Everything else is a
specification, a reference or historical evidence.

```text
docs/
  PLANNING.md      the master plan: direction, milestones, work queue, process
  DESIGN.md        invariants, architecture as built, subsystem designs, the target and how it grows
  LEDGER.md        status and evidence for every known stub, defect and oversimplification
  specs/           specifications and mathematical references (not status reports)
    ARCHITECTURE_SPEC.md       the long-range target: eight waves of numerics, CAD, multiphysics, AI, certified arithmetic
    GUI_DESIGN_PLAN.md         the GUI rework
    GEAR_PROFILE_CURVES.md     the mathematics behind the gear generator
    symbolic/                  lemma mining and MCTS proof guidance
    physics_informed_benchmarks/   the physics-informed benchmark suite
  audits/          the reports the ledger was built from (historical; not maintained in place)
    CODEBASE_AUDIT_AND_REMEDIATION_PLAN.md   the catalogue: A1-A16, B1-B20, C1-C17, Tracks D-J
    AUDIT-2026-11-15.md                      the first quality audit
    AUDIT-PANICS-AND-UNWRAPS.md              the inventory of production panic sites
  katex-header.html  rustdoc header that loads KaTeX
```

Outside this folder: [`../ROADMAP.md`](../ROADMAP.md) (what each of the 31 phases was meant to deliver, with a status
per phase), [`../CHANGELOG.md`](../CHANGELOG.md), [`../CONTRIBUTING.md`](../CONTRIBUTING.md) and
[`../DOCUMENTATION_STANDARDS.md`](../DOCUMENTATION_STANDARDS.md).

## Which document answers which question

| Question | Read |
| :--- | :--- |
| What is URAE and how is it built? | [`DESIGN.md`](DESIGN.md) |
| What is planned, and in what order? | [`PLANNING.md`](PLANNING.md) |
| Is this behaviour a known defect, and is it fixed? | [`LEDGER.md`](LEDGER.md) |
| What did the original audit say about it? | [`audits/`](audits) (only when the ledger entry is not enough) |
| What is the long-range target for numerics, CAD and simulation? | [`specs/ARCHITECTURE_SPEC.md`](specs/ARCHITECTURE_SPEC.md) |
| What was phase N meant to deliver? | [`../ROADMAP.md`](../ROADMAP.md) |

## Rules for keeping it true

1. **One home for each fact.** A status lives in the ledger; the plan, the roadmap and the README link to it and do not
   copy it. A number that drifts (test counts, line counts) is stated once, with a date, or not at all.
2. **Precedence.** Where documents disagree: the code, then the ledger, then the plan, then the design, then the
   specifications and audits.
3. **A specification is not a status report.** `specs/` documents describe intended scope. Their statements about what
   exists are checked only when someone updates them; the ledger and the design say what is built.
4. **Audits are evidence, not state.** Nothing in `audits/` is updated when a defect is fixed. Its banner points at the
   ledger.
5. **Update in the same pull request.** A change that fixes or finds a defect edits its ledger entry and the
   `[Unreleased]` section of the changelog; a change that moves a milestone edits the plan.
6. **Links are relative.** No `file:///` or machine-specific paths; links to source use paths relative to the file
   (`../crates/...`) so they work on GitHub and in a clone.
7. **Ids are permanent.** A ledger id is never reused for a different item; if one has to change, the ledger keeps a note saying what replaced it (`R1`–`R13` are the only renamed ids).
