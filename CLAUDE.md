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
- What compiles right now: Repository workspace files, `arc-core` library, and `arc-cli` binary scaffolding created; compilation blocked on host machine lacking `cargo`/`rustc` in PATH.
- What has passing tests right now:
  - Python oracle runner `scripts/oracle_check.py` successfully runs AC and DC power flows in `.oracle-venv` on pandapower 3.5.4 (verified 2026-09-03).
  - Rust unit tests (`arc-core` and `arc-cli` scaffold tests) awaiting host `cargo test`.
- What is stubbed, fake, or not implemented:
  - Grid data models (`Bus`, `Branch`, `Generator`, `Load` in `arc-core/src/model.rs`) not yet created (M1).
  - Admittance builder (`Ybus`), DC solver, and Newton-Raphson AC solver not yet created (M2-M4).
- Current milestone: M0 — Repo scaffold and prior art survey
- Milestone status: in progress (Prior art survey done as ADR-0001; scaffold files created; python oracle installed and verified; host cargo/rustc installation pending).

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
- Host environment currently lacks `cargo` and `rustc` in PATH. Need Rust installed via `rustup` on Windows before `cargo test --workspace` can be executed locally.

## Next Steps
1. User installs Rust toolchain via `rustup-init.exe` on Windows and verifies `cargo --version`.
2. Run `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, and `cargo fmt --check` to officially close out M0.
3. Begin Milestone 1 (M1): Implement `Bus`, `Branch`, `Generator`, `Load` in `arc-core/src/model.rs` with per-unit conventions and unit tests.

## Session Log (append-only, newest entry at top — never delete history)
### Session 1 — 2026-09-03
- Did:
  - Confirmed working directory `e:\products\arc` is clean and appropriate.
  - Checked Rust toolchain (`cargo`, `rustc` not found on system PATH; documented Windows installation steps).
  - Conducted prior-art survey across RustPower (`chengts95/rustpower`), `powerio`/`powers`/`qsim`, PowSyBl / `powsybl-open-loadflow`, and pandapower.
  - Wrote and accepted `docs/adr/0001-prior-art-survey.md` establishing from-scratch solver core decision.
  - Set up Python 3.12 `.oracle-venv` and installed `pandapower` 3.5.4.
  - Implemented `scripts/oracle_check.py` for automated AC and DC power flow reference generation.
  - Scaffolded Cargo workspace (`arc-core`, `arc-cli`), `LICENSE`, `.gitignore`, and `.github/workflows/ci.yml`.
  - Initialized `README.md` and `CLAUDE.md`.
- Verified via:
  - `.\.oracle-venv\Scripts\python scripts/oracle_check.py --case case3 --mode ac` →
    ```json
    {
      "metadata": {
        "case": "case3",
        "mode": "AC",
        "converged": true,
        "pandapower_version": "3.5.4",
        "timestamp": "2026-09-03T04:45:03.152860+00:00"
      },
      "buses": {
        "0": { "bus_id": 0, "vm_pu": 1.0, "va_degree": 0.0, "va_rad": 0.0, "p_mw": 9.40171614, "q_mvar": 63.53310725 },
        "1": { "bus_id": 1, "vm_pu": 1.0066893215, "va_degree": -0.3823202861, "va_rad": -0.0066727478, "p_mw": 40.0, "q_mvar": 20.0 },
        "2": { "bus_id": 2, "vm_pu": 1.02, "va_degree": -0.0117374675, "va_rad": -0.0002048575, "p_mw": -50.0, "q_mvar": -85.32795883 }
      }
    }
    ```
  - `.\.oracle-venv\Scripts\python scripts/oracle_check.py --case case3 --mode dc` →
    ```json
    {
      "metadata": {
        "case": "case3",
        "mode": "DC",
        "converged": true,
        "pandapower_version": "3.5.4",
        "timestamp": "2026-09-03T04:44:54.764647+00:00"
      },
      "buses": {
        "0": { "bus_id": 0, "vm_pu": 1.0, "va_degree": 0.0, "va_rad": 0.0, "p_mw": 10.0, "q_mvar": NaN },
        "1": { "bus_id": 1, "vm_pu": 1.0, "va_degree": -0.2291831181, "va_rad": -0.004, "p_mw": 40.0, "q_mvar": NaN },
        "2": { "bus_id": 2, "vm_pu": 1.02, "va_degree": 0.3437746771, "va_rad": 0.006, "p_mw": -50.0, "q_mvar": NaN }
      }
    }
    ```
- Did not do / deliberately deferred:
  - Local `cargo test` execution deferred until Rust toolchain is installed on Windows host.
  - Milestone 1 (`model.rs`) deferred until M0 toolchain verification is complete.
- Next session should start with:
  - Confirm `cargo --version` after toolchain installation.
  - Run `cargo test --workspace` to verify M0 green.
  - Start M1 (core data model).
