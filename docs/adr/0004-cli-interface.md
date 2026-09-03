# ADR-0004: Command-Line Interface (CLI) Ergonomics and Output Formats

## Status
Accepted (Milestone 8)

## Context
Milestone 8 requires providing a clean, minimal command-line interface:
`arc solve <case-file>`
that prints bus voltages, phase angles, and convergence status.

As `arc` evolves toward automated cluster studies and batch regression pipelines, the CLI needs:
1. A clear command hierarchy conforming to the garage specification (`arc solve`).
2. Deterministic, human-readable terminal table output as well as structured JSON output for downstream consumers and pipelines.
3. Configurable formulation modes (AC Newton-Raphson vs. DC linear) and linear solver backends (Sparse Markowitz LU vs. Dense Gaussian elimination).
4. Convergence controls (tolerance, max iterations) with sensible defaults.
5. Predictable exit codes (0 for converged solve, 1 for failure or divergence) suitable for scripting and CI pipelines.

## Decision
We implement the CLI in `arc-cli`:
- **Primary Subcommand**: `arc solve <FILE>` (with `arc run <FILE>` retained as an alias).
- **Arguments**:
  - `FILE`: Path to a MATPOWER `.m` or `arc` Grid `.json` file.
  - `--mode` / `-m`: Formulation mode (`ac` [default] or `dc`).
  - `--solver` / `-s`: Linear equation solver backend (`sparse` [default] or `dense`).
  - `--tolerance` / `-t`: Convergence tolerance in per-unit (default: `1e-8`).
  - `--max-iterations` / `-i`: Maximum iteration limit (default: `30`).
  - `--format` / `-f`: Output format (`table` [default] or `json`).
- **Regression Subcommand**: `arc test` to execute the automated numerical regression harness against the pandapower oracle across all benchmark cases.
- **Info Subcommand**: `arc info` to display kernel version and capabilities.

## Consequences
- Single unified entry point `arc solve <case-file>` works out-of-the-box for IEEE benchmark cases.
- Machine-readable JSON output enables piping directly into `jq`, Python, or web dashboards.
- Predictable Unix process semantics with nonzero exit codes on model or convergence errors.
