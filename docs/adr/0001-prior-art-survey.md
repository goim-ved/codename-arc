# ADR-0001: Prior Art Survey and Solver Core Architecture

- **Status:** Accepted
- **Date:** 2026-09-03
- **Deciders:** arc project bootstrap team
- **Consulted:** Claude Code / Antigravity

## Context and Problem Statement

`arc` is an open-source (Apache-2.0), Rust-based, deterministic power flow kernel for grid interconnection studies (FERC Order 2023). For adversarial multi-party environments (transmission providers, developers, consultants, regulators), numerical correctness, auditability, and bit-for-bit determinism outrank raw simulation speed or broad feature scaffolding.

Before writing solver code for v0.1, we must evaluate existing tools in the power systems and Rust ecosystems to answer a fundamental architectural question:
**Do we implement our own minimal Newton-Raphson solver core from scratch, or do we build arc's differentiated layers (data model, GridIR, CLI, oracle validation harness) on top of an existing crate such as RustPower, powers/powerio, or qsim?**

## Prior Art Evaluated

### 1. RustPower (`chengts95/rustpower`)
- **Overview:** An actively developed Rust power flow calculation crate utilizing an Entity-Component-System (ECS) architecture. Supports Newton-Raphson AC power flow and time-series simulations, with sparse backends including RSparse and KLU.
- **Strengths:**
  - High performance and memory efficiency with zero-allocation design patterns.
  - Native support for pandapower JSON networks, external grids, and transformers.
  - Active benchmarks against LightSim2Grid and pandapower.
- **Drawbacks & Risks for arc:**
  - **Architecture Coupling:** The ECS paradigm organizes grid elements into component storages optimized for game-engine-style iteration. This creates impedance mismatch with Git-diffable, hierarchical, content-addressable grid models (GridIR) and Merkle DAG representations planned for future arc phases.
  - **Determinism & Dependencies:** Relying on KLU involves native C/SuiteSparse linkages which complicate static cross-compilation and introduce potential machine-specific floating-point or ordering variances.
  - **Control over Internal Solver State:** Implementing diff-informed warm starts, exact Jacobian inspection, and deterministic floating-point reduction order is harder when wrapped around a third-party engine.

### 2. `powerio` / `powers` / `qsim` Ecosystem
- **`powerio` / `powerio_matrix`:** Robust Rust crates focused on power system data I/O (MATPOWER, PSS/E, PowerModels) and sparse admittance matrix construction (Y-bus, B', B''). While valuable as a future reference for parser implementations, `powerio` is primarily a model transformation/graph projection toolkit rather than an audited solver engine.
- **`qsim` / `qsim-solvers`:** A modular power grid analysis framework divided into `qsim-core`, `qsim-elements`, `qsim-solvers`, and `qsim-io`. AC/DC solvers are present, but the project is early in development, lacks widespread validation against standard IEEE benchmarks, and has low community adoption.
- **`oxigrid`:** A broader crate covering OPF, small-signal stability, and dynamics. Its wide scope conflicts with the v0.1 "garage release" mandate: narrow, fully tested, and transparent.

### 3. PowSyBl / `powsybl-open-loadflow` (LF Energy / RTE)
- **Overview:** Production-grade, Java-based open-source framework deployed at European TSOs (e.g., RTE). Implements full Newton-Raphson AC power flow with KLU, voltage control priority, tap changers, and distributed slack.
- **Relevance:**
  - Not a direct candidate for arc's engine due to language (Java/JVM) and European CGMES/CIM focus rather than US PSS/E / MATPOWER interconnection workflows.
  - Serves as an indispensable reference for what a production power flow engine eventually requires: PV-to-PQ bus switching on reactive limits, transformer phase/voltage tap regulation, slack bus distribution, and multi-island management.

### 4. pandapower (Python)
- **Overview:** Established Python power system modeling tool based on PyPower and SciPy/LightSim2Grid.
- **Role in arc:** Serves as our primary external **numerical oracle**. Arc will not compete with pandapower's modeling breadth in v0.1; instead, every physics claim, admittance calculation, and power flow solution in arc will be validated against pandapower outputs at strict numerical tolerances ($10^{-6}$ p.u.).

## Decision

**We will implement a minimal, from-scratch Newton-Raphson solver core in `arc-core` for v0.1.**

Specifically:
1. **Ownership of the Numerical Pipeline:** Implementing the core linear DC (`Bθ = P`) and non-linear AC (polar Newton-Raphson) algorithms in `arc-core` ensures complete control over:
   - Floating-point reduction order and tie-breaking (guaranteeing cross-platform determinism).
   - Exact bus/branch indexing using ordered maps (`BTreeMap` / index mappings).
   - Jacobian formulation and mismatch calculation.
2. **Dense First, Sparse in M7:** We will begin with dense linear algebra for the 3-bus and small test cases (M3, M4) to eliminate third-party solver bugs while verifying math against the oracle. In M7, we will introduce a sparse backend (evaluating pure-Rust `faer` sparse solvers vs `sprs`).
3. **Reference Prior Art for I/O and Advanced Features:** When parsing standard MATPOWER cases in M5 and designing advanced control loops in future versions, we will draw directly on designs from `powerio` and `powsybl-open-loadflow`.

## Consequences

### Positive
- **Guaranteed Determinism:** Zero hidden thread races, iteration jitter, or non-deterministic sparse reordering heuristics.
- **No Unwanted Dependencies:** Clean, minimal dependency graph without C runtime/SuiteSparse compilation friction on Windows/Linux/macOS.
- **Direct Path to GridIR:** Clean data structures that can directly integrate with content-addressed diffing and incremental factorizations in later versions.

### Negative / Trade-offs
- **Initial Engineering Overhead:** Requires writing the admittance builder, mismatch equations, Jacobian assembly, and Newton-Raphson loop directly.
- **Temporary Lack of Advanced Controls:** Tap changers, phase shifters, and Q-limit bus switching are deferred to post-v0.1 milestones.

## What Would Change This Decision
We would revisit integrating or wrapping an existing engine (such as `rustpower` or a pure-Rust sparse core) if:
1. Pure-Rust sparse linear algebra in M7 proves insufficient to solve IEEE 14-bus, 30-bus, or 118-bus cases within acceptable performance boundaries without native C KLU bindings.
2. An existing Rust crate achieves verified, reproducible bit-for-bit determinism across all platforms and adopts a decoupled, non-ECS storage model compatible with arc's GridIR.
