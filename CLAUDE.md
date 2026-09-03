# CLAUDE.md — Agent Context for `arc`

> Read this fully before doing anything else in this repo. Update it before ending
> every session. This file is not documentation for humans — it is your own working
> memory across sessions. Keep it accurate and boring.

## Project Snapshot
- Name: arc (`arc-core` library + `arc-cli` binary)
- Stage: Garage / v0.1 (pre-alpha)
- License: Apache-2.0
- Language: Rust (edition 2021)
- Last updated: 2026-09-03, session 1

## Current State
- What compiles right now: Complete workspace (`arc-core` library and `arc-cli` binary) compiles cleanly with Rust 1.98.0 (MSVC).
- What has passing tests right now:
  - `cargo test --workspace`: 2 passed (1 in `arc-core`, 1 in `arc-cli`), 0 failed (verified 2026-09-03).
  - `cargo clippy --workspace --all-targets -- -D warnings`: clean, 0 warnings (verified 2026-09-03).
  - `cargo fmt --all -- --check`: clean (verified 2026-09-03).
  - Python oracle runner `scripts/oracle_check.py` AC & DC solves on `case3` match pandapower 3.5.4 in `.oracle-venv` (verified 2026-09-03).
- What is stubbed, fake, or not implemented:
  - Grid data models (`Bus`, `Branch`, `Generator`, `Load` in `arc-core/src/model.rs`) not yet created (M1).
  - Admittance builder (`Ybus`), DC solver, and Newton-Raphson AC solver not yet created (M2-M4).
- Current milestone: M0 — Repo scaffold and prior art survey
- Milestone status: done (verified via `cargo test --workspace`, `cargo clippy`, `cargo fmt`, and oracle execution). M1 ready to begin upon user confirmation.

## Build & Test Commands
- `cargo build --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo fmt --all -- --check`
- Oracle cross-check AC: `.\.oracle-venv\Scripts\python scripts/oracle_check.py --case case3 --mode ac`
- Oracle cross-check DC: `.\.oracle-venv\Scripts\python scripts/oracle_check.py --case case3 --mode dc`

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
- `arc-core/src/lib.rs` — Root module for arc-core with base test
- `arc-cli/Cargo.toml` — Crate definition for command line interface
- `arc-cli/src/main.rs` — Entry point for CLI binary
- `scripts/oracle_check.py` — Pandapower numerical oracle runner for case cross-validation

## Known Issues / Gaps
- None for M0. Environment is fully configured with Rust 1.98.0 and Python 3.12 (.oracle-venv).

## Next Steps
1. User confirms starting Milestone 1 (M1).
2. Implement `Bus`, `Branch`, `Generator`, `Load` in `arc-core/src/model.rs` with explicit per-unit conventions.
3. Add unit tests for component construction and per-unit conversions; verify with `cargo test`.

## Session Log (append-only, newest entry at top — never delete history)
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
