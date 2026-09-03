# ARC — Garage Version Build Directive
### A working brief for Claude Code · v0.1 "garage release"

Paste this entire document as your first message to Claude Code, in an empty (or nearly
empty) directory you intend to use for this project. It is a complete operating brief,
not a feature request — follow it in order.

---

## 0. WHAT YOU ARE BUILDING AND WHY

You are bootstrapping the first real, working version of **arc**: an open-source
(Apache-2.0), Rust-based, deterministic power flow kernel for grid interconnection
studies. The long-term thesis (you don't need to build any of this yet, it's context):
U.S. generator interconnection queues are backlogged by well over 2,000 GW, FERC Order
2023 now legally requires "first-ready, first-served" cluster studies with financial
penalties for missed deadlines, and every study is currently re-run by hand against
1980s-era proprietary solvers with no version control and no reproducibility between
adversarial parties (utility, developer, consultant, regulator). The end-state product
is a neutral, git-diffable, deterministically reproducible substrate that all of those
parties can trust *because* it's open and inspectable, not despite it.

None of that end-state matters for what you build today. It exists here so you
understand *why* determinism, correctness, and auditability outrank feature breadth or
speed at every single decision point in this project. A fast power flow solver that
gives a slightly different answer on two different machines is worse than useless for
this product — it's actively disqualifying.

**The garage-version ethos (read this twice):** Linux v0.01 didn't support networking,
had no filesystem beyond what it needed to boot, and ran on one architecture. It was
real, it compiled, and every line in it did something correct. That is the bar here.
Working, correct, tested, and narrow beats impressive-looking and unverified every
time. If you ever feel the pull to scaffold something broad and half-finished because
it "shows the vision," stop — that instinct is wrong for this project specifically.

---

## 1. MANDATORY FIRST TASK: PRIOR ART SURVEY — DO THIS BEFORE WRITING SOLVER CODE

Before designing or implementing anything, spend real, actual effort (not a memory
dump) evaluating existing tools. This is not a formality — it directly determines
whether you write a solver from scratch or build on/around an existing one, and that
decision should be evidence-based, not assumed.

Known relevant prior art to actually go check (confirm current state yourself — do not
trust any version numbers or benchmark claims below without verifying):

- **RustPower** (`chengts95/rustpower` on GitHub, `rustpower` on PyPI/crates ecosystem)
  — an existing, actively developed Rust power flow crate using an ECS (Bevy)
  architecture, with KLU and `faer` sparse solver backends, that publishes its own
  benchmark comparisons against LightSim2Grid and pandapower. `cargo add rustpower` (or
  clone the repo) in a scratch directory, run its examples, and actually observe what
  it does and doesn't do.
- **`powers`** (crates.io) — a direct Rust port of MATPOWER (BSD-3 licensed), with
  companion crates for OPF/CPF/PTDF. Check what's actually implemented vs. advertised.
- **`qsim`** (docs.rs) — a smaller Rust power grid modeling crate with DC/AC solvers
  and Rayon parallelism.
- **PowSyBl / powsybl-open-loadflow** (Linux Foundation Europe / RTE) — mature, Java,
  production-deployed at multiple European TSOs, full Newton-Raphson AC on KLU. Not
  Rust, and CGMES/Europe-oriented rather than PSS/E/US-oriented — but read its
  documentation to understand what a *production-grade* open power flow engine
  actually has to handle (voltage control priority, tap changers, distributed slack,
  etc.) so v0.1's scope decisions are informed, even though v0.1 won't implement most
  of it yet.
- **pandapower** (Python, `pip install pandapower`) — the tool you will use as your
  numerical oracle throughout this project (see §4). Not a Rust competitor, but your
  most important dependency for correctness.

Write up findings as `docs/adr/0001-prior-art-survey.md` (see §3 for the ADR format).
The ADR must answer explicitly: *do we implement our own Newton-Raphson solver core,
or build arc's differentiated layers (case model, IR, CLI, oracle-validated
correctness harness) on top of an existing crate like RustPower or `powers`?* Either
answer is fine. What's not fine is skipping this step and guessing.

Default recommendation if you want one and don't have strong evidence either way:
implement a minimal from-scratch solver for v0.1 anyway, specifically *because* full
ownership of the numerical core is what lets later phases (custom GridIR integration,
warm-start factorization tied to model diffs, GPU batching) be built without fighting
someone else's architecture — but treat this as a default to override, not a
foregone conclusion, and say so explicitly in the ADR along with what would change
your mind.

---

## 2. ANTI-HALLUCINATION PROTOCOL — NON-NEGOTIABLE, APPLIES TO EVERY SESSION

Power flow code is exactly the kind of thing that *looks* plausible when wrong — a
solver can converge to a self-consistent but physically incorrect answer, and nobody
will notice by eyeballing the code. Follow these rules without exception:

1. **Never state a numerical result you have not actually generated by running code.**
   If you write "the 3-bus case converges to V=1.02∠-2.1°" in documentation or a commit
   message, that number must have come from an actual `cargo test` or `cargo run`
   output you just produced, not from memory or a plausible-looking guess.
2. **Every physics claim gets cross-validated against pandapower before it's trusted.**
   Set up a Python virtual environment (`python -m venv .oracle-venv`,
   `pip install pandapower`) early, and write a small script
   (`scripts/oracle_check.py`) that takes a case, solves it in pandapower, and dumps
   bus voltages/angles as JSON. Every new arc capability gets diffed against this
   oracle within a numerical tolerance (start with 1e-6 per-unit on voltage magnitude
   and angle) before you consider it correct. Do not hand-derive "expected" values
   from textbook memory and hardcode them as test fixtures — generate them from the
   oracle, and record in the test file which oracle run produced them and when.
3. **Show your work in every progress update.** When you update CLAUDE.md or
   README.md to say something works, paste the actual terminal output (or a
   faithful summary of it) that proves it, not just an assertion.
4. **If you're not sure whether a Rust crate, API, or method exists or behaves a
   certain way, check — `cargo doc`, docs.rs, or a quick isolated test script — before
   writing code that depends on the assumption.** Do not guess crate APIs.
5. **Determinism is a testable property, not a hope.** Any code path whose output
   order could depend on `HashMap` iteration order, floating-point reduction order
   across threads, or wall-clock timing must be called out and fixed (prefer
   `BTreeMap`/`IndexMap`, fixed iteration order, single-threaded by default in v0.1).
   Add a CI step that runs the solver twice on the same input and byte-diffs the
   output.
6. **Never mark a milestone complete in README.md or CLAUDE.md unless its tests are
   green in an actual `cargo test` run you just performed.** "Should work" is not
   "works."

---

## 3. THE DOCUMENTATION SYSTEM — TWO PARTS, STRICTLY SEPARATE PURPOSES

You will maintain exactly two living documents, plus a lightweight decision log that
feeds both. Do not conflate their purposes.

### 3a. `README.md` — for human viewers

Audience: anyone landing on this repo cold — a collaborator, a future contributor, the
project owner showing someone else. Plain English, no internal implementation detail
dumps, honest about what stage this is.

Template to create now and keep current:

```markdown
# arc

> An open-source, deterministic power flow kernel for grid interconnection studies.
> Garage stage. Not production-ready. Do not use for real grid decisions.

## What this is
[1-2 paragraphs, plain English, no jargon a non-power-engineer can't follow]

## Status
🚧 Garage stage (v0.1-in-progress). Current one-line truth: <e.g. "solves a 3-bus DC
power flow correctly and deterministically; AC Newton-Raphson not yet implemented.">

## Try it
[exact commands, kept current — if they don't work, this section is wrong]

## Roadmap
[link to milestones in CLAUDE.md's structure, kept high-level here]

## Prior art & attribution
This project was built after evaluating pandapower, PowSyBl, RustPower, and the
`powers`/`qsim` Rust crates. See docs/adr/0001-prior-art-survey.md for what we found
and why we made the choices we made.

## License
Apache-2.0
```

### 3b. `CLAUDE.md` — for you, Claude Code, and only you

This is Claude Code's real, built-in memory mechanism: it is read automatically at the
start of every session in this repo. Treat it as ground truth about the repo's current
state, not as marketing copy, and not as a duplicate of the README. If a future session
of you reads only this file and nothing else, it should be able to know exactly what
exists, what's tested, what's broken, and what to do next — with no guessing.

Template to create now and update **every session, without exception**:

```markdown
# CLAUDE.md — Agent Context for `arc`

> Read this fully before doing anything else in this repo. Update it before ending
> every session. This file is not documentation for humans — it is your own working
> memory across sessions. Keep it accurate and boring.

## Project Snapshot
- Name: arc (`arc-core` library + `arc-cli` binary)
- Stage: Garage / v0.1 (pre-alpha)
- License: Apache-2.0
- Language: Rust (state edition/toolchain version once pinned)
- Last updated: <date>, session <N>

## Current State
- What compiles right now: ...
- What has passing tests right now (be specific — which tests, last run when): ...
- What is stubbed, fake, or not implemented: ...
- Current milestone: M<N> — <name> (see §5 of the original build directive, or restate
  the milestone list here once established)
- Milestone status: not started / in progress / blocked (why) / done (verified how)

## Build & Test Commands
- `cargo build --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cargo fmt --check`
- Oracle cross-check: `python scripts/oracle_check.py <case>` then compare
- (Add any other command a fresh session needs to verify the repo's actual state)

## Architecture Decisions
- ADR-0001: <one-line summary> — docs/adr/0001-prior-art-survey.md
- (append as ADRs are added; full reasoning lives in docs/adr/, this is just an index)

## File Manifest
(One line per source file, updated as files are added/changed — purpose, not content)
- `arc-core/src/model.rs` — Bus/Branch/Generator/Load types, per-unit constants
- ...

## Known Issues / Gaps
- ...

## Next Steps
(The literal next thing to do, in order — a future session should be able to start
working immediately from this list without re-deriving it)
1. ...
2. ...

## Session Log (append-only, newest entry at top — never delete history)
### Session N — <date>
- Did: ...
- Verified via: `cargo test ...` → <actual output, not paraphrase>
- Did not do / deliberately deferred: ...
- Next session should start with: ...
```

### 3c. `docs/adr/NNNN-title.md` — the decision log feeding both

Short architecture decision records: context, decision, why, what would change it.
Every non-trivial choice (solver crate vs. from-scratch, dense-before-sparse, case
format order, etc.) gets one. This is what keeps CLAUDE.md's "Architecture Decisions"
section from becoming a wall of unexplained assertions.

---

## 4. TECHNICAL SCOPE FOR v0.1 — ONE MILESTONE AT A TIME

Work through these in order. **Do not start milestone N+1 until milestone N's tests
pass and both README.md and CLAUDE.md are updated.** If you're tempted to jump ahead
because "it's related," don't — note the idea in CLAUDE.md's Known Issues/Next Steps
instead and stay on the current milestone.

- **M0 — Repo scaffold.** Cargo workspace with `arc-core` (library) and `arc-cli`
  (binary) crates. `LICENSE` (Apache-2.0). `.gitignore`. GitHub Actions CI running
  `cargo build`, `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check` on
  every push. Initialize README.md and CLAUDE.md from the templates above, even though
  most sections will say "not started yet." Complete the prior-art survey (§1) as part
  of this milestone, before M1.

- **M1 — Core data model.** `Bus`, `Branch`, `Generator`, `Load` types in
  `arc-core/src/model.rs`. Per-unit system constants and conventions documented inline
  (state your base MVA convention explicitly — don't leave it implicit). Unit tests
  for basic construction and per-unit conversions only — no solving yet.

- **M2 — Y-bus admittance matrix builder.** Build the bus admittance matrix from a
  tiny hand-constructible 2- or 3-bus network. Unit-test against values you compute
  by hand (show the hand calculation in a code comment or a doc-test) — this is the
  one place in the whole project where hand-derivation is acceptable, precisely
  because it's small enough to actually verify by hand and you need a ground truth
  that doesn't depend on the oracle working yet.

- **M3 — DC power flow (linear, dense).** Solve `Bθ = P` on the same 3-bus system.
  This is deliberately the first *solved* result in the project: one linear solve, no
  iteration, nothing that can silently fail to converge. Cross-check against the
  oracle (pandapower) for the first time here. If this milestone doesn't match the
  oracle, do not proceed — something in M1/M2 is wrong and needs fixing before AC
  power flow (which is much harder to debug) is attempted.

- **M4 — AC power flow, Newton-Raphson, dense Jacobian, polar coordinates.** Same
  3-bus system. This is the real numerical core. Validate against the oracle. Also
  validate that DC and AC results are in the right ballpark relative to each other
  (DC is an approximation of AC — they should be close for a lightly-loaded case,
  which is a useful sanity check independent of the oracle).

- **M5 — Standard test case support.** Parse the MATPOWER case format (or use
  pandapower's exported JSON if that's simpler to start — record the choice as an
  ADR) for IEEE case9 and case14, sourced from either the public MATPOWER GitHub
  repository or pandapower's built-in `pandapower.networks` module. Solve both, cross
  -validate against the oracle.

- **M6 — Formalize the oracle cross-validation harness.** Turn the ad hoc checks from
  M3-M5 into a proper `cargo test` integration test suite that, for a fixed set of
  cases, solves in arc and diffs against a pre-generated oracle output file (checked
  into the repo, regenerated via a documented script, timestamped/versioned so it's
  clear when it was last regenerated against which pandapower version).

- **M7 — Sparse solver.** Swap the dense Jacobian solve for a sparse one, using
  whatever crate the M0 prior-art survey pointed to (`faer`'s sparse solvers, or a
  KLU binding, or `sprs` — decide based on what you actually found, not by default).
  Add a regression test proving sparse and dense give matching results on all
  existing test cases before removing the dense path (or keep both, gated behind a
  feature flag, if that's cheap).

- **M8 — CLI.** `arc solve <case-file>` prints bus voltages/angles and whether the
  solve converged. Minimal, no flags beyond what's needed to run a case.

- **M9 — Determinism and benchmark baseline.** CI step that runs the solver twice on
  the same input and fails if output differs at all. `criterion` benchmark on the
  14-bus case, committed as a baseline for catching future performance regressions —
  not because speed matters yet, but because catching a regression at commit N is
  free and catching it at commit N+200 is a debugging project.

- **M10 — v0.1 "garage release."** Full test suite green, README and CLAUDE.md fully
  current, tag `v0.1.0`. Write a short, honest release note: what it does, what it
  explicitly does not do yet (see Non-Goals below), and what M11+ would be.

**Explicit non-goals for v0.1 — do not build these yet, and say so in README if
anyone might reasonably wonder:** PSS/E RAW/DYR parsing, CIM/CGMES support, GPU
batching, contingency analysis, GridIR/Merkle-DAG content-addressing, the hosted
control plane, multi-party study rooms, any compliance/audit tooling, HELM or any
solver besides Newton-Raphson. All of these are real parts of the long-term product —
none of them belong in the garage version, and pulling any of them forward dilutes
focus on getting the numerical core unquestionably correct first.

---

## 5. SESSION PROTOCOL

**At the start of every session:** Read `CLAUDE.md` in full before doing anything
else. State back, briefly, what you understand the current state and next step to be,
before acting — this catches drift between what the file says and what you're about
to do.

**While working:** One milestone at a time, per §4. Run tests frequently, not just at
the end. If something doesn't match the oracle, stop and investigate rather than
adjusting tolerances to make it pass.

**At the end of every session (or every milestone, whichever comes first):**
1. Run the full test suite and confirm it's actually green (paste real output).
2. Update `CLAUDE.md`: Current State, File Manifest, Next Steps, and a new dated
   Session Log entry, appended, with prior entries left intact.
3. Update `README.md` if the one-line status or Try It section changed.
4. Commit with a message that reflects what actually changed, referencing the
   milestone (e.g. `M3: DC power flow solving 3-bus case, matches oracle to 1e-8`).

---

## 6. YOUR FIRST ACTIONS, RIGHT NOW

1. Confirm you're in the intended working directory for this project. If it's not
   empty and doesn't look like it's meant for this, stop and ask before doing
   anything.
2. Check the Rust toolchain is available (`cargo --version`, `rustc --version`). If
   not, tell me how to install it rather than attempting to install system packages
   yourself.
3. Do the prior-art survey (§1) and write `docs/adr/0001-prior-art-survey.md`.
4. Scaffold M0 per §4, including bootstrapping `README.md` and `CLAUDE.md` from the
   templates in §3.
5. Set up the pandapower oracle environment (§2, rule 2) before writing any solver
   code, so it's ready the moment M3 needs it.
6. Report back with what you found in the prior-art survey and your recommendation
   before starting M1, so we can confirm the from-scratch-vs-build-on-existing-crate
   decision together rather than you committing to it unilaterally.

Go slowly. Correctness and a clean two-document trail of *why* every decision was made
matter more than speed here — that trail is the actual product differentiator later,
not just project hygiene.
