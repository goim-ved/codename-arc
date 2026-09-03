# ADR-0002: Test Case Format Selection for arc v0.1

## Status
Accepted

## Date
2026-09-03

## Context
Milestone 5 requires loading and solving standard benchmark power flow test cases, specifically IEEE 9-bus (`case9`) and IEEE 14-bus (`case14`), and establishing cross-validation against the reference numerical oracle (`pandapower 3.5.4`).

Historically, power flow research and industry tools rely on the MATPOWER `.m` case file format developed by Cornell University / PSERC. A MATPOWER case file is an executable MATLAB script defining a struct `mpc` with fields `baseMVA`, `bus` matrix, `gen` matrix, `branch` matrix, and optional cost and generator capability curve matrices.

Parsing full arbitrary MATLAB syntax in a production Rust kernel introduces severe dependencies, fragile grammar handling (comments, nested cell arrays, dynamic variable scopes), and high maintenance overhead for a v0.1 garage release. Conversely, relying exclusively on proprietary or non-standard bespoke formats reduces accessibility for power systems researchers accustomed to MATPOWER.

## Decision
We adopt a **two-tier case representation architecture**:

1. **Native Structured Exchange Format (`arc` Grid JSON)**:
   - Complete `Network` data model natively serializable and deserializable via `serde_json`.
   - Explicit typed schemas for buses, branches (lines and transformers), generators, loads, and bus shunts.
   - Guaranteed deterministic field ordering and lossless float representation.

2. **Lightweight MATPOWER Matrix Parser (`arc-core/src/parser/matpower.rs`)**:
   - A deterministic, regex/token-driven parser targeting the standard tabular matrix blocks (`mpc.baseMVA`, `mpc.bus`, `mpc.gen`, `mpc.branch`).
   - Parses standard IEEE/MATPOWER benchmarks directly without requiring a MATLAB runtime or heavy AST parser.
   - Automatically maps MATPOWER 1-based indexing, bus types (1: PQ, 2: PV, 3: Slack, 4: Isolated), branch $\pi$-model parameters, off-nominal transformer tap ratios, phase angle shifts, and bus shunt admittances ($G_s, B_s$).

3. **Bundled Case Repository (`data/cases/`)**:
   - Canonical benchmark cases (`case9.m`, `case9.json`, `case14.m`, `case14.json`) bundled directly in the repository for repeatable, offline test harness execution.

## Consequences
### Positive
- Direct interoperability with widely available MATPOWER cases without pre-conversion steps.
- Zero external native C or MATLAB runtime dependencies.
- Native JSON format provides clean integration with modern Web APIs, CLI tools, and future GridIR content addressing.
- Full support for bus shunts ($G_s, B_s$) and off-nominal tap/phase-shifting transformers required by `case14`.

### Negative / Trade-offs
- The MATPOWER parser does not execute MATLAB expressions (e.g. arithmetic inside matrix definitions like `10/3`); numbers in matrices must be numeric literals. Standard MATPOWER distribution cases already satisfy this requirement.
