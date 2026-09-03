# arc

> An open-source, deterministic power flow kernel for grid interconnection studies.
> Garage stage. Not production-ready. Do not use for real grid decisions.

## What this is

When a new solar farm, wind plant, or battery facility wants to connect to the electric power grid, engineers must simulate the physics of the high-voltage transmission network to ensure the addition will not overload transmission lines or cause voltages to collapse. Today, over 2,000 gigawatts of planned clean energy projects are stuck waiting in U.S. interconnection queues. Under FERC Order 2023, grid operators are required to run cluster studies with strict deadlines and financial penalties for delay, yet these studies are still performed using closed-source, decades-old software where results cannot be verified independently by developers, utilities, or regulators.

`arc` is an open-source, high-performance power flow kernel built in Rust. It is engineered from first principles for auditability and bit-for-bit determinism: two independent parties running the same grid case on different computers will always get the exact same numerical result.

## Status

🚧 Garage stage (v0.1-in-progress). Current one-line truth: Repository scaffold initialized (M0); pandapower numerical oracle verified; core data models and solvers pending toolchain verification (M1+).

## Try it

Once the Rust toolchain is installed:

```bash
# Build the workspace
cargo build --workspace

# Run tests
cargo test --workspace

# Run the pandapower numerical oracle (requires Python virtualenv in .oracle-venv)
python scripts/oracle_check.py --case case3 --mode ac
```

## Roadmap

Development proceeds strictly milestone-by-milestone, with each milestone verified against the numerical oracle before the next begins:

- **M0 — Repo Scaffold & Prior Art Survey**: Cargo workspace, Apache-2.0 license, CI, ADR-0001, oracle environment. *(Current)*
- **M1 — Core Data Model**: `Bus`, `Branch`, `Generator`, `Load` types with explicit per-unit conventions.
- **M2 — Y-bus Admittance Matrix Builder**: Hand-derivation and unit tests for 3-bus network admittance.
- **M3 — DC Power Flow (Linear, Dense)**: Linear $B\theta = P$ solve cross-validated against pandapower.
- **M4 — AC Power Flow (Newton-Raphson, Dense)**: Polar Newton-Raphson solver cross-validated against oracle at $10^{-6}$ p.u. tolerance.
- **M5 — Standard Test Case Support**: IEEE 9-bus and 14-bus case loading and solving.
- **M6 — Formal Oracle Integration Harness**: Automated test suite diffing against frozen oracle fixtures.
- **M7 — Sparse Solver**: Transition dense linear solve to high-performance sparse factorization (`faer`/sparse).
- **M8 — CLI**: `arc solve <case-file>` interface.
- **M9 — Determinism & Benchmark Baseline**: Bit-diff testing in CI and Criterion performance baseline.
- **M10 — v0.1 Garage Release**: Tagged v0.1.0 release note and frozen benchmark.

## Prior art & attribution

This project was built after evaluating pandapower, PowSyBl, RustPower, and the `powerio`/`qsim`/`powers` Rust crates. See [docs/adr/0001-prior-art-survey.md](docs/adr/0001-prior-art-survey.md) for what we found and why we made the choices we made.

## License

Apache-2.0
