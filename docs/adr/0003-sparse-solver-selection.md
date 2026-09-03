# ADR-0003: Sparse Matrix Representation and Linear Solver Selection

## Status
Accepted (Milestone 7)

## Context
In Milestones 3 and 4, `arc` implemented linear DC power flow and polar AC Newton-Raphson power flow using a deterministic dense linear solver with partial pivoting (`arc-core/src/linear.rs`).

While dense Gaussian elimination was sufficient to establish analytical correctness on 3-bus, 9-bus, and 14-bus test cases, it scales with:
- **Memory**: $\mathcal{O}(N^2)$ storage.
- **Computation**: $\mathcal{O}(N^3)$ arithmetic operations per Newton-Raphson iteration.

In real-world power transmission grids, each bus is typically connected to only 2 to 4 neighboring buses regardless of total grid size. Consequently, the bus admittance matrix $Y_{\text{bus}}$, the DC susceptibility matrix $B_{\text{bus}}$, and the AC Newton-Raphson Jacobian matrix $J$ are extremely sparse:
- 14-bus grid: ~27% non-zero entries.
- 30-bus grid: ~12% non-zero entries.
- 57-bus grid: ~5% non-zero entries.
- 118-bus grid: ~3.5% non-zero entries.
- 1000+ bus grids: < 0.5% non-zero entries.

To scale `arc` to standard transmission systems, Milestone 7 mandates upgrading the linear solver to a sparse matrix formulation while maintaining strict numerical determinism and matching oracle solutions to machine precision.

## Evaluated Alternatives

### Option 1: `sprs`
- **Pros**: Established pure-Rust sparse matrix library providing CSR and CSC formats.
- **Cons**: Built-in direct factorization is limited to Cholesky / LDL for symmetric positive-definite systems. Unsymmetric sparse direct LU (required for the AC power flow Jacobian) is only available through foreign C bindings to SuiteSparse/UMFPACK, which violates `arc`'s directive of zero external C/Fortran toolchain dependencies.

### Option 2: `faer` (Sparse LU)
- **Pros**: Pure-Rust, SIMD-accelerated, high performance on very large dense/sparse benchmarks.
- **Cons**: The sparse LU API (`faer::sparse::linalg::lu`) is a low-level, multi-stage interface requiring manual workspace layout calculation (`StackReq`) and `dyn-stack` memory management. Furthermore, pulling in full `faer` introduces over 125 transitive dependencies (including parser generators, regex engines, and threading runtimes), bloating compilation time by 20+ seconds and adding significant complexity to the lightweight garage core.

### Option 3: Pure-Rust Sparse LU with Markowitz Threshold Pivoting (Selected)
- **Pros**:
  - **Zero External Dependencies**: Implemented directly in `arc-core` using standard Rust data structures.
  - **Deterministic**: Exact arithmetic with deterministic pivot tie-breaking.
  - **Tailored for Power Systems**: Employs the classic Markowitz criterion:
    $$\min_{i, j} (r_i - 1)(c_j - 1)$$
    subject to threshold numerical stability ($|a_{ij}| \ge u \cdot \max_k |a_{kj}|$ with threshold parameter $u \approx 0.1$). This approach is identical to the core strategy of KLU (specifically designed for circuit simulation and power flow), minimizing fill-in while preventing numerical instability.
  - **Fast Compilation**: Zero additional dependencies; compiles in under 1 second.
  - **Portability**: 100% safe Rust, compatible with MSVC, Linux, macOS, and WebAssembly targets.

## Decision
We select **Option 3**:
1. Implement a clean, coordinate-based sparse matrix representation (`TripletList`) and Compressed Sparse Row (`CsrMatrix`) in `arc-core/src/sparse/`.
2. Implement a pure-Rust sparse LU solver with Markowitz threshold pivoting (`SparseLuSolver`).
3. Integrate the sparse solver into both `DCPowerFlow` ($B_{\text{bus}} \theta = P$) and `ACPowerFlow` (sparse Jacobian $J \Delta x = -\Delta F$).
4. Retain the dense solver behind an option (`SolverKind::Dense` vs `SolverKind::Sparse`) to allow continuous cross-verification and benchmarking.

## Consequences
- `arc` gains $\mathcal{O}(N^{1.2} - N^{1.4})$ scaling on sparse transmission networks.
- Memory consumption drops from dense $\mathcal{O}(N^2)$ to sparse $\mathcal{O}(N + \text{nnz})$.
- Zero additional third-party dependencies or C compiler requirements.
- Existing tests continue to pass with identical numerical results.
