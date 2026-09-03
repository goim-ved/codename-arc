# arc

> An open-source, deterministic power flow kernel for grid interconnection studies.
> Garage stage. Not production-ready. Do not use for real grid decisions.

## What this is

When a new solar farm, wind plant, or battery facility wants to connect to the electric power grid, engineers must simulate the physics of the high-voltage transmission network to ensure the addition will not overload transmission lines or cause voltages to collapse. Today, over 2,000 gigawatts of planned clean energy projects are stuck waiting in U.S. interconnection queues. Under FERC Order 2023, grid operators are required to run cluster studies with strict deadlines and financial penalties for delay, yet these studies are still performed using closed-source, decades-old software where results cannot be verified independently by developers, utilities, or regulators.

`arc` is an open-source, high-performance power flow kernel built in Rust. It is engineered from first principles for auditability and bit-for-bit determinism: two independent parties running the same grid case on different computers will always get the exact same numerical result.

## Status

🚧 Garage stage (v0.1-in-progress). Current one-line truth: Milestone 7 complete (pure-Rust sparse matrix & linear solver upgrade with Markowitz threshold pivoting, validated across IEEE 14, 30, 57, 118 cases; 44 passing tests); Milestone 8 (CLI ergonomics & parameter overrides) ready to start.

## Try it

Once the Rust toolchain is installed:

```bash
# Build the workspace
cargo build --workspace

# Run full test suite across workspace (44 passing tests)
cargo test --workspace

# Run automated numerical regression harness across all 6 IEEE benchmark cases (12 evaluations)
cargo run --bin arc -- test

# Solve power flow on an IEEE benchmark case using the sparse solver
cargo run --bin arc -- run data/cases/case118.m --mode ac --solver sparse

# Run sparse vs dense scaling benchmarks
cargo test --test sparse_scaling_benchmarks --release -- --nocapture

# Run the pandapower numerical oracle
python scripts/oracle_check.py --case case14 --mode ac
```

## Roadmap

Development proceeds strictly milestone-by-milestone, with each milestone verified against the numerical oracle before the next begins:

- **M0 — Repo Scaffold & Prior Art Survey**: Cargo workspace, Apache-2.0 license, CI, ADR-0001, oracle environment. *(Done)*
- **M1 — Core Data Model**: `Bus`, `Branch`, `Generator`, `Load` types with explicit per-unit conventions. *(Done)*
- **M2 — Y-bus Admittance Matrix Builder**: Hand-derivation and unit tests for 3-bus network admittance. *(Done)*
- **M3 — DC Power Flow (Linear, Dense)**: Linear $B\theta = P$ solve cross-validated against pandapower. *(Done)*
- **M4 — AC Power Flow (Newton-Raphson, Dense)**: Polar Newton-Raphson solver cross-validated against oracle at $10^{-6}$ p.u. tolerance. *(Done)*
- **M5 — Standard Test Case Support**: IEEE 9-bus and 14-bus case loading and solving. *(Done)*
- **M6 — Automated Numerical Regression Test Harness**: Automated suite running in CI and CLI across all cases. *(Done)*
- **M7 — Sparse Matrix & Linear Solver Upgrade**: Pure-Rust sparse LU with Markowitz threshold pivoting, tested on IEEE 14, 30, 57, 118 networks. *(Done)*
- **M8 — CLI & Ergonomics**: Full `arc solve <case-file>` interface and parameter overrides. *(Next)*
- **M9 — Determinism & Benchmark Baseline**: Bit-diff testing in CI and Criterion performance baseline.
- **M10 — v0.1 Garage Release**: Tagged v0.1.0 release note and frozen benchmark.

## Prior art & attribution

This project was built after evaluating pandapower, PowSyBl, RustPower, and the `powerio`/`qsim`/`powers` Rust crates. See [docs/adr/0001-prior-art-survey.md](docs/adr/0001-prior-art-survey.md) for what we found and why we made the choices we made.

## License

Apache-2.0
