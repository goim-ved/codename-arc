# CLAUDE.md — Agent Context for `arc`

> Read this fully before doing anything else in this repo. Update it before ending
> every session. This file is not documentation for humans — it is your own working
> memory across sessions. Keep it accurate and boring.

## Project Snapshot
- Name: arc (`arc-core` library + `arc-cli` binary)
- Stage: Garage / v0.1 (pre-alpha)
- License: Apache-2.0
- Language: Rust (edition 2021)
- Last updated: 2026-09-03, session 3

## Current State
- What compiles right now: Complete workspace (`arc-core` library and `arc-cli` binary) with `model` and `admittance` modules compiles cleanly with Rust 1.98.0 (MSVC).
- What has passing tests right now:
  - `cargo test --workspace`: 16 passed (14 unit tests in `arc-core`, 1 in `arc-cli`, 1 integration test `ybus_oracle_validation`), 0 failed (verified 2026-09-03).
  - `cargo clippy --workspace --all-targets -- -D warnings`: clean, 0 warnings (verified 2026-09-03).
  - `cargo fmt --all -- --check`: clean (verified 2026-09-03).
  - Python oracle runner `scripts/oracle_check.py --case case3 --dump-ybus` matches Rust `YBus` values to $10^{-9}$ precision (verified 2026-09-03).
- What is stubbed, fake, or not implemented:
  - DC linear power flow solver not yet created (M3).
  - AC Newton-Raphson polar power flow solver not yet created (M4).
- Current milestone: M2 — Y-bus admittance matrix builder
- Milestone status: done (verified via hand derivation and oracle cross-validation integration test). M3 ready to begin.

## Build & Test Commands
- `cargo build --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo fmt --all -- --check`
- Oracle cross-check AC: `.\.oracle-venv\Scripts\python scripts/oracle_check.py --case case3 --mode ac`
- Oracle cross-check DC: `.\.oracle-venv\Scripts\python scripts/oracle_check.py --case case3 --mode dc`
- Oracle dump Y-bus: `.\.oracle-venv\Scripts\python scripts/oracle_check.py --case case3 --dump-ybus`

## Architecture Decisions
- ADR-0001: Minimal from-scratch Newton-Raphson solver core selected over wrapping RustPower or external C/KLU crates to guarantee strict determinism and compatibility with future GridIR — docs/adr/0001-prior-art-survey.md

## File Manifest
- `Cargo.toml` — Workspace root configuration for `arc-core` and `arc-cli`
- `LICENSE` — Apache License 2.0
- `.gitignore` — Ignore rules for Rust target directory, Python venv, and editor files
- `.github/workflows/ci.yml` — GitHub Actions workflow for build, test, clippy, and rustfmt
- `README.md` — Project overview and human-facing status
- `CLAUDE.md` — Agent working memory across sessions
- `docs/adr/0001-prior-art-survey.md` — ADR evaluating existing power flow tools and justifying from-scratch core
- `arc-core/Cargo.toml` — Crate definition for core power flow library
- `arc-core/src/lib.rs` — Root module for arc-core with re-exports
- `arc-core/src/model.rs` — Bus, Branch, Generator, Load, and Network types with per-unit conversions
- `arc-core/src/admittance.rs` — Bus admittance matrix ($Y_{\text{bus}}$) builder and tests
- `arc-core/tests/ybus_oracle_validation.rs` — Integration test cross-validating Y-bus against pandapower oracle
- `arc-cli/Cargo.toml` — Crate definition for command line interface
- `arc-cli/src/main.rs` — Entry point for CLI binary
- `scripts/oracle_check.py` — Pandapower numerical oracle runner for case cross-validation and Y-bus dumping

## Known Issues / Gaps
- None for M2. Y-bus construction verified against hand-calculated ground truth and external oracle.

## Next Steps
1. Begin Milestone 3 (M3): Implement linear DC power flow ($B\theta = P$).
2. Implement dense LU or Gaussian elimination solve on $B_{\text{bus}} \theta = P$ for non-slack buses.
3. Cross-validate DC voltage angles and branch flows against `scripts/oracle_check.py --case case3 --mode dc`.

## Session Log (append-only, newest entry at top — never delete history)
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
