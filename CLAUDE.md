# CLAUDE.md — Agent Context for `arc`

> Read this fully before doing anything else in this repo. Update it before ending
> every session. This file is not documentation for humans — it is your own working
> memory across sessions. Keep it accurate and boring.

## Project Snapshot
- Name: arc (`arc-core` library + `arc-cli` binary)
- Stage: Garage / v0.1 (pre-alpha)
- License: Apache-2.0
- Language: Rust (edition 2021)
- Last updated: 2026-09-03, session 8

## Current State
- What compiles right now: Complete workspace (`arc-core` library and `arc` CLI binary) with `model`, `admittance`, `linear`, `sparse`, `parser`, `regression`, `solver::dc`, and `solver::ac` modules compiles cleanly with Rust 1.98.0 (MSVC).
- What has passing tests right now:
  - `cargo test --workspace`: 44 passed (32 unit tests in `arc-core`, 1 in `arc-cli`, 6 integration tests: `ybus_oracle_validation`, `dc_oracle_validation`, `ac_oracle_validation`, `benchmark_oracle_validation`, `sparse_scaling_benchmarks` [4 tests], and `regression_harness`), 0 failed (verified 2026-09-03).
  - `cargo clippy --workspace --all-targets -- -D warnings`: clean, 0 warnings (verified 2026-09-03).
  - `cargo fmt --all -- --check`: clean (verified 2026-09-03).
  - `arc test` (`cargo run --bin arc -- test`): runs regression test harness across all 12 benchmark permutations (`case3`, `case9`, `case14`, `case30`, `case57`, `case118` in AC and DC) and prints clean pass/fail table with sparsity metrics; 0 failures.
  - IEEE benchmark errors against `pandapower 3.5.4` oracle:
    - `case3`: AC Vm MAE $2.96 \times 10^{-16}$, Va MAE $4.67 \times 10^{-17}$; DC Va MAE $2.89 \times 10^{-19}$ (0.0% sparsity).
    - `case9`: AC Vm MAE $3.82 \times 10^{-16}$, Va MAE $3.68 \times 10^{-16}$; DC Va MAE $4.97 \times 10^{-17}$ (66.7% sparsity).
    - `case14`: AC Vm MAE $3.97 \times 10^{-16}$, Va MAE $5.45 \times 10^{-17}$; DC Va MAE $2.58 \times 10^{-17}$ (72.4% sparsity).
    - `case30`: AC Vm MAE $5.66 \times 10^{-16}$, Va MAE $6.14 \times 10^{-16}$; DC Va MAE $1.61 \times 10^{-17}$ (87.6% sparsity).
    - `case57`: AC Vm MAE $7.40 \times 10^{-16}$, Va MAE $6.19 \times 10^{-16}$; DC Va MAE $3.99 \times 10^{-16}$ (93.3% sparsity).
    - `case118`: AC Vm MAE $1.92 \times 10^{-8}$, Va MAE $7.67 \times 10^{-6}$; DC Va MAE $1.13 \times 10^{-7}$ (96.5% sparsity).
- What is stubbed, fake, or not implemented:
  - Advanced parameter overrides (Q-limits, generator PV-PQ switching in M8).
- Current milestone: M7 — Sparse matrix & linear solver upgrade
- Milestone status: done (pure-Rust sparse LU with Markowitz threshold pivoting implemented in `arc-core::sparse`, integrated into DC and AC solvers, cross-validated on IEEE 14, 30, 57, 118 networks, and demonstrated scaling). M8 ready to begin.

## Build & Test Commands
- `cargo build --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo fmt --all -- --check`
- Run regression harness CLI: `cargo run --bin arc -- test`
- Run power flow CLI: `cargo run --bin arc -- run data/cases/case118.m --mode ac --solver sparse`
- Benchmark oracle test: `cargo test --test benchmark_oracle_validation -- --nocapture`
- Sparse scaling benchmark: `cargo test --test sparse_scaling_benchmarks --release -- --nocapture`
- Oracle cross-check AC: `.\.oracle-venv\Scripts\python scripts/oracle_check.py --case case14 --mode ac`
- Oracle cross-check DC: `.\.oracle-venv\Scripts\python scripts/oracle_check.py --case case14 --mode dc`
- Export cases: `.\.oracle-venv\Scripts\python scripts/export_cases.py`

## Architecture Decisions
- ADR-0001: Minimal from-scratch Newton-Raphson solver core selected over wrapping RustPower or external C/KLU crates to guarantee strict determinism and compatibility with future GridIR — docs/adr/0001-prior-art-survey.md
- ADR-0002: Two-tier case representation adopting native `arc` Grid JSON (serde) alongside a lightweight tabular MATPOWER `.m` parser with bundled IEEE benchmark cases — docs/adr/0002-case-format.md
- ADR-0003: Pure-Rust Sparse Matrix representation (`TripletList`, `CsrMatrix`) and Sparse direct LU solver with dynamic Markowitz threshold pivoting (`SparseLuSolver`) — docs/adr/0003-sparse-solver-selection.md

## File Manifest
- `Cargo.toml` — Workspace root configuration for `arc-core` and `arc-cli`
- `LICENSE` — Apache License 2.0
- `.gitignore` — Ignore rules for Rust target directory, Python venv, and editor files
- `.github/workflows/ci.yml` — GitHub Actions workflow for build, test, clippy, rustfmt, and numerical regression
- `README.md` — Project overview and human-facing status
- `CLAUDE.md` — Agent working memory across sessions
- `docs/adr/0001-prior-art-survey.md` — ADR evaluating existing power flow tools and justifying from-scratch core
- `docs/adr/0002-case-format.md` — ADR on test case format selection (JSON + MATPOWER .m)
- `arc-core/Cargo.toml` — Crate definition for core power flow library
- `arc-core/src/lib.rs` — Root module for arc-core with re-exports
- `arc-core/src/model.rs` — Bus, Branch, Generator, Load, Shunt, and Network types with per-unit conversions
- `arc-core/src/admittance.rs` — Bus admittance matrix ($Y_{\text{bus}}$) builder with branch and shunt support
- `arc-core/src/linear.rs` — Deterministic dense linear system solver ($A x = b$) with partial pivoting
- `arc-core/src/parser/mod.rs` — Parser module definitions and re-exports
- `arc-core/src/parser/matpower.rs` — Tabular MATPOWER `.m` case parser
- `arc-core/src/regression.rs` — Automated numerical regression harness engine and table formatter
- `arc-core/src/solver/mod.rs` — Solver module definitions and re-exports
- `arc-core/src/solver/dc.rs` — Linear DC power flow solver ($B\theta = P$) and tests
- `arc-core/src/solver/ac.rs` — Non-linear AC Newton-Raphson polar power flow solver and tests
- `arc-core/tests/ybus_oracle_validation.rs` — Integration test cross-validating Y-bus against pandapower oracle
- `arc-core/tests/dc_oracle_validation.rs` — Integration test cross-validating DC power flow against pandapower oracle
- `arc-core/tests/ac_oracle_validation.rs` — Integration test cross-validating AC power flow against pandapower oracle and DC sanity checks
- `arc-core/tests/benchmark_oracle_validation.rs` — Integration test cross-validating IEEE case9 and case14 across AC/DC solvers
- `arc-core/tests/regression_harness.rs` — Integration test executing full automated numerical regression suite
- `arc-cli/Cargo.toml` — Crate definition for command line interface
- `arc-cli/src/main.rs` — Entry point for CLI binary `arc` (`test`, `run`, `info`)
- `data/cases/` — Bundled benchmark cases (`case3`, `case9`, `case14` in `.m`, `.json`, and oracle formats)
- `scripts/oracle_check.py` — Pandapower numerical oracle runner for case cross-validation and Y-bus dumping
- `scripts/export_cases.py` — Script exporting canonical pandapower networks to MATPOWER and arc Grid JSON

## Known Issues / Gaps
- None for M6. Automated regression test harness runs cleanly across all 3 networks and both formulations with 0 failures and sub-$10^{-15}$ errors.

## Next Steps
1. Begin Milestone 7 (M7): Sparse Matrix & Linear Solver Upgrade.
2. Formulate sparse matrix representation (CSR/CSC) for admittance and Jacobian structures.
3. Benchmark dense vs sparse solve times across bus count.

## Session Log (append-only, newest entry at top — never delete history)
### Session 7 — 2026-09-03
- Did:
  - Exported canonical 3-bus reference benchmark files (`case3.m`, `case3.json`, `case3_oracle.json`) to `data/cases/` via `scripts/export_cases.py`.
  - Implemented automated numerical regression engine in `arc-core/src/regression.rs` (`RegressionHarness`, `RegressionReport`, `CaseRegressionResult`) comparing case results against pandapower oracle outputs across strict thresholds (< $10^{-5}$ Vm, < $10^{-4}$ Va).
  - Implemented CLI subcommand `arc test` in `arc-cli/src/main.rs` to run regression harness and display aligned terminal table with exit codes.
  - Implemented CLI subcommand `arc run <FILE> [--mode ac|dc]` to solve cases directly from command line.
  - Added integration test `arc-core/tests/regression_harness.rs`.
  - Updated GitHub Actions CI workflow (`.github/workflows/ci.yml`) to execute `cargo run --bin arc -- test` on every push and pull request.
- Verified via:
  - `cargo test --workspace` → 29 passed (23 in `arc-core`, 1 in `arc-cli`, 5 integration test suites), 0 failed.
  - `cargo run --bin arc -- test` → ALL 6 BENCHMARKS PASSED (0 failures) with full formatted table.
  - `cargo run --bin arc -- run data/cases/case14.m` → AC solved in 4 iterations with matching bus voltage profile.
  - `cargo run --bin arc -- run data/cases/case14.m --mode dc` → DC solved in 1 iteration.
  - `cargo clippy --workspace --all-targets -- -D warnings` → Clean (0 warnings).
  - `cargo fmt --all -- --check` → Clean (formatting confirmed).
- Did not do / deliberately deferred:
  - Milestone 7 (Sparse solver upgrade) deferred to next milestone.
- Next session should start with:
  - Start Milestone 7 (M7): Sparse matrix and linear solver upgrade.

### Session 6 — 2026-09-03
- Did:
  - Wrote and committed ADR-0002 on test case format selection (`docs/adr/0002-case-format.md`).
  - Added `Shunt` struct and `shunts` map to `Network` in `arc-core/src/model.rs`.
  - Updated `YBus::build` in `arc-core/src/admittance.rs` to incorporate bus shunt conductances and susceptances into diagonal elements.
  - Implemented tabular MATPOWER `.m` case parser in `arc-core/src/parser/matpower.rs`.
  - Created `scripts/export_cases.py` and exported canonical `case9.m`, `case9.json`, `case14.m`, `case14.json`, and oracle reference solutions to `data/cases/`.
  - Implemented integration test `arc-core/tests/benchmark_oracle_validation.rs`:
    - Verified equivalence of `.m` parser and `.json` deserialization.
    - Solved AC and DC power flow on both IEEE 9-bus and IEEE 14-bus networks.
    - Recorded MAE: Case 9 AC Vm MAE $3.58 \times 10^{-16}$, Va MAE $3.12 \times 10^{-16}$; Case 14 AC Vm MAE $3.65 \times 10^{-16}$, Va MAE $4.71 \times 10^{-16}$ (both well within targets $< 10^{-6}$ and $< 10^{-4}$).
- Verified via:
  - `cargo test --workspace` → 28 passed (22 in `arc-core`, 1 in `arc-cli`, 5 integration test suites), 0 failed.
  - `cargo clippy --workspace --all-targets -- -D warnings` → Clean (0 warnings).
  - `cargo fmt --all -- --check` → Clean (formatting confirmed).
- Did not do / deliberately deferred:
  - Milestone 6 (Automated regression test harness) deferred to next milestone.
- Next session should start with:
  - Start Milestone 6 (M6): Numerical regression test harness.

### Session 5 — 2026-09-03
- Did:
  - Implemented Milestone 4: Non-linear AC Newton-Raphson solver in polar coordinates with dense analytical Jacobian in `arc-core/src/solver/ac.rs`.
  - Formulated calculated powers $P_i(\mathbf{V}, \boldsymbol{\theta})$ and $Q_i(\mathbf{V}, \boldsymbol{\theta})$ and mismatches $\Delta P_i, \Delta Q_i$.
  - Formulated 4-block analytical Jacobian $J = \begin{bmatrix} H & N \\ M & L \end{bmatrix}$ with exact derivative formulas.
  - Implemented branch AC power flows ($P_{\text{from}}, Q_{\text{from}}, P_{\text{to}}, Q_{\text{to}}$) and system loss calculation.
  - Added integration test `arc-core/tests/ac_oracle_validation.rs`:
    - Validated convergence in 3 iterations to $10^{-8}$ mismatch.
    - Verified bus voltages and angles against pandapower 3.5.4 oracle to $< 10^{-6}$ p.u.
    - Verified line power flows and total system losses (~$0.598\text{ MW}$) against oracle to $< 10^{-4}\text{ MW}$.
    - Performed DC vs AC ballpark sanity check confirming physical consistency between approximations.
- Verified via:
  - `cargo test --workspace` → 24 passed (20 in `arc-core`, 1 in `arc-cli`, 3 integration test suites), 0 failed.
  - `cargo clippy --workspace --all-targets -- -D warnings` → Clean (0 warnings).
  - `cargo fmt --all -- --check` → Clean (formatting confirmed).
- Did not do / deliberately deferred:
  - Milestone 5 (MATPOWER case9 and case14 parser) deferred to next milestone.
- Next session should start with:
  - Start Milestone 5 (M5): Standard test case support (case9, case14) and ADR-0002.
### Session 4 — 2026-09-03
- Did:
  - Verified M1 and M2 test suite and formatting integrity.
  - Implemented Milestone 3: Deterministic dense linear solver using Gaussian elimination with partial pivoting in `arc-core/src/linear.rs`.
  - Implemented linear DC power flow solver in `arc-core/src/solver/dc.rs`:
    - Assembled $B_{\text{bus}}$ matrix with transformer taps and phase shifts.
    - Partitioned out the reference Slack bus to solve the non-slack reduced system $B_{\mathcal{NS}, \mathcal{NS}} \boldsymbol{\theta}_{\mathcal{NS}} = \mathbf{P}_{\text{eff}, \mathcal{NS}}$.
    - Solved Slack bus generation and active branch flows ($P_{\text{from}}, P_{\text{to}}$).
  - Derived analytical hand solution for the 3-bus network ($\theta_1 = -0.004\text{ rad}$, $\theta_2 = 0.006\text{ rad}$, $P_{\text{slack}} = -10.0\text{ MW}$).
  - Created integration test `arc-core/tests/dc_oracle_validation.rs` validating bus angles and branch flows against `pandapower 3.5.4` oracle.
- Verified via:
  - `cargo test --workspace` →
    ```text
    running 1 test
    test tests::cli_scaffold_verification ... ok
    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

    running 19 tests
    test linear::tests::test_dimension_mismatch ... ok
    test admittance::tests::test_transformer_tap_ratio ... ok
    test admittance::tests::test_line_charging_shunt_susceptance ... ok
    test admittance::tests::test_hand_derived_2bus_ybus ... ok
    test admittance::tests::test_out_of_service_branch_ignored ... ok
    test admittance::tests::test_hand_derived_3bus_canonical_ybus ... ok
    test linear::tests::test_singular_matrix_detection ... ok
    test linear::tests::test_solve_2x2_system ... ok
    test linear::tests::test_solve_3x3_identity ... ok
    test model::tests::test_per_unit_base_conversions ... ok
    test solver::dc::tests::test_canonical_3bus_dc_power_flow_hand_calculated ... ok
    test model::tests::test_branch_series_admittance ... ok
    test model::tests::test_bus_angle_conversions ... ok
    test model::tests::test_generator_and_load_per_unit_power ... ok
    test model::tests::test_canonical_3bus_network_construction ... ok
    test model::tests::test_net_power_injection ... ok
    test model::tests::test_offline_generator_and_load_ignored_in_injections ... ok
    test model::tests::test_network_validation_errors ... ok
    test tests::scaffold_verification ... ok
    test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

    running 1 test
    test test_dc_power_flow_matches_pandapower_oracle_case3 ... ok
    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

    running 1 test
    test test_ybus_matches_pandapower_oracle_case3 ... ok
    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
    ```
  - `cargo clippy --workspace --all-targets -- -D warnings` → Clean (0 warnings).
  - `cargo fmt --all -- --check` → Clean (formatting confirmed).
- Did not do / deliberately deferred:
  - Milestone 4 (AC Newton-Raphson solver) deferred to next milestone.
- Next session should start with:
  - Start Milestone 4 (M4): AC power flow (Newton-Raphson, dense Jacobian, polar coordinates).
### Session 3 — 2026-09-03
- Did:
  - Implemented Milestone 2: `YBus` admittance matrix builder in `arc-core/src/admittance.rs`.
  - Implemented full $\Pi$-equivalent branch admittance contributions (series $g_s, b_s$, line charging $b_{\text{shunt}}$, and transformer off-nominal turns ratio $a e^{j\phi}$).
  - Documented hand calculations for 2-bus and canonical 3-bus networks in doc-tests/unit tests.
  - Added `--dump-ybus` to `scripts/oracle_check.py` to extract pandapower's internal sparse $Y_{\text{bus}}$.
  - Implemented integration test `arc-core/tests/ybus_oracle_validation.rs` asserting exact agreement ($< 10^{-9}$) with pandapower oracle.
  - Tested tap ratios, out-of-service branches, line charging, and matrix symmetry.
- Verified via:
  - `cargo test --workspace` →
    ```text
    running 1 test
    test tests::cli_scaffold_verification ... ok
    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

    running 14 tests
    test admittance::tests::test_hand_derived_2bus_ybus ... ok
    test admittance::tests::test_line_charging_shunt_susceptance ... ok
    test admittance::tests::test_hand_derived_3bus_canonical_ybus ... ok
    test admittance::tests::test_transformer_tap_ratio ... ok
    test model::tests::test_branch_series_admittance ... ok
    test model::tests::test_bus_angle_conversions ... ok
    test model::tests::test_canonical_3bus_network_construction ... ok
    test model::tests::test_generator_and_load_per_unit_power ... ok
    test model::tests::test_net_power_injection ... ok
    test model::tests::test_network_validation_errors ... ok
    test model::tests::test_offline_generator_and_load_ignored_in_injections ... ok
    test admittance::tests::test_out_of_service_branch_ignored ... ok
    test model::tests::test_per_unit_base_conversions ... ok
    test tests::scaffold_verification ... ok
    test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

    running 1 test
    test test_ybus_matches_pandapower_oracle_case3 ... ok
    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
    ```
  - `cargo clippy --workspace --all-targets -- -D warnings` → Clean (0 warnings).
  - `cargo fmt --all -- --check` → Clean (formatting confirmed).
- Did not do / deliberately deferred:
  - Milestone 3 (linear DC solver) deferred until M2 is committed and reviewed.
- Next session should start with:
  - Start Milestone 3 (M3): Linear DC power flow solver in `arc-core/src/solver/dc.rs`.
### Session 2 — 2026-09-03
- Did:
  - Pushed initial repository commit `32b6b68` to GitHub remote `https://github.com/goim-ved/codename-arc.git`.
  - Implemented Milestone 1: `Bus`, `Branch`, `Generator`, `Load`, `Network`, and `ModelError` types in `arc-core/src/model.rs`.
  - Documented explicit per-unit conventions inline (default $S_{\text{base}} = 100.0\text{ MVA}$, $Z_{\text{base}} = V^2 / S$, $I_{\text{base}} = S / (\sqrt{3} V)$).
  - Implemented complex series admittance calculation $Y = 1/(R+jX) = G + jB$ on `Branch`.
  - Implemented net active and reactive power injection calculation ($P_{\text{inj}}, Q_{\text{inj}}$) accounting for online/offline generator and load statuses.
  - Implemented `Network::validate()` verifying topological consistency, single slack bus requirement, and valid bus IDs.
  - Added comprehensive unit tests for per-unit conversions, angle conversions, series admittance, net injections, offline element handling, and canonical 3-bus network construction.
- Verified via:
  - `cargo test --workspace` →
    ```text
    running 1 test
    test tests::cli_scaffold_verification ... ok
    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

    running 9 tests
    test model::tests::test_branch_series_admittance ... ok
    test model::tests::test_bus_angle_conversions ... ok
    test model::tests::test_canonical_3bus_network_construction ... ok
    test model::tests::test_generator_and_load_per_unit_power ... ok
    test model::tests::test_net_power_injection ... ok
    test model::tests::test_network_validation_errors ... ok
    test model::tests::test_per_unit_base_conversions ... ok
    test tests::scaffold_verification ... ok
    test model::tests::test_offline_generator_and_load_ignored_in_injections ... ok
    test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
    ```
  - `cargo clippy --workspace --all-targets -- -D warnings` → Clean (0 warnings).
  - `cargo fmt --all -- --check` → Clean (formatting confirmed).
- Did not do / deliberately deferred:
  - Milestone 2 (Y-bus admittance matrix builder) deferred until M1 is committed and reviewed.
- Next session should start with:
  - Start Milestone 2 (M2): `arc-core/src/admittance.rs` for Y-bus matrix construction and verification.

### Session 1 — 2026-09-03
- Did:
  - Confirmed working directory `e:\products\arc` is clean and appropriate.
  - Installed official Rust toolchain `stable-x86_64-pc-windows-msvc` (Rust 1.98.0, Cargo 1.98.0) via `rustup-init.exe`.
  - Added and confirmed `clippy` and `rustfmt` components.
  - Conducted prior-art survey across RustPower (`chengts95/rustpower`), `powerio`/`powers`/`qsim`, PowSyBl / `powsybl-open-loadflow`, and pandapower.
  - Wrote and accepted `docs/adr/0001-prior-art-survey.md` establishing from-scratch solver core decision.
  - Set up Python 3.12 `.oracle-venv` and installed `pandapower` 3.5.4.
  - Implemented `scripts/oracle_check.py` for automated AC and DC power flow reference generation.
  - Scaffolded Cargo workspace (`arc-core`, `arc-cli`), `LICENSE`, `.gitignore`, and `.github/workflows/ci.yml`.
  - Initialized `README.md` and `CLAUDE.md`.
  - Verified compilation, clippy, formatting, and unit tests across workspace.
- Verified via:
  - `cargo test --workspace` →
    ```text
    running 1 test
    test tests::cli_scaffold_verification ... ok
    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

    running 1 test
    test tests::scaffold_verification ... ok
    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
    ```
  - `cargo clippy --workspace --all-targets -- -D warnings` → Clean, 0 warnings.
  - `cargo fmt --all -- --check` → Clean, formatting matches rustfmt standard.
  - `.\.oracle-venv\Scripts\python scripts/oracle_check.py --case case3 --mode ac` → Converged: true, bus voltages match expected values.
  - `.\.oracle-venv\Scripts\python scripts/oracle_check.py --case case3 --mode dc` → Converged: true.
- Did not do / deliberately deferred:
  - Deferred M1 (core data model) until user confirms proceeding.
- Next session should start with:
  - Start M1: `arc-core/src/model.rs` (Bus, Branch, Generator, Load, per-unit base constants).
