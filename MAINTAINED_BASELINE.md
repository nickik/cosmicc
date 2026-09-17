# Maintained Cosmic C baseline

Cosmic C's supported product is the `cosmicc` command and its direct
`C -> typed HIR -> CLIF -> Cranelift SIA32 -> COSMIC-SIA` path.  The compiler
runs on the build host, but generated code is only SIA32.  Native bytes or a
decoded `CallPlan` are compilation evidence, not execution evidence.

## Supported toolchain and commands

The supported Rust toolchain is **1.98.1**, declared in
[`rust-toolchain.toml`](rust-toolchain.toml).  `Cargo.lock` records the pinned
Cranelift SIA32 revision; every supported command uses `--locked` so a lockfile
change is deliberate.

| Purpose | Command |
| --- | --- |
| Formatting | `cargo fmt --all -- --check` |
| C-to-SIA tests | `cargo test -p saltwater-sia --locked` |
| Compiler command | `cargo check --bin cosmicc --locked` |
| Workspace validation | `cargo check --workspace --locked` |
| Complete supported gate | `sh scripts/check-supported.sh` |
| Release compiler build | `cargo build --release --bin cosmicc --locked` |

`cargo test --workspace` is intentionally not a supported gate yet.  Its
inherited host-runner, JIT, varargs, system-`cc`, and benchmark sources are
retained below for provenance but do not describe the Cosmic SIA profile.

## Inventory

| Location | Classification | Status |
| --- | --- | --- |
| `src/cosmicc.rs`, root `cosmicc` package | Active Cosmic C implementation | Supported compiler command; sole root Cargo binary target. |
| `saltwater-sia/` | Active Cosmic C implementation | Supported lowering, bundle, CallPlan, and byte tests. |
| `saltwater-parser/` | Retained parser/frontend infrastructure | Required by `saltwater-sia`; retained while its broader C surface is audited. Its historical optional `jit` compatibility switch is not enabled or supported by Cosmic C. |
| `src/main.rs` | Retired host/JIT functionality | Historical Saltwater driver, explicitly not a Cargo target. |
| `saltwater-codegen/` | Retired host/JIT functionality | Historical x86/object/JIT code; neither workspace member nor Cosmic C dependency. |
| `tests/`, `benches/`, `fuzz/`, `minimizer/` | Test-only compatibility/reference code | Excluded from the maintained Cargo target discovery; they are not SIA acceptance evidence. |
| Historical Saltwater/RCC/`swcc` text in `CHANGELOG.md`, `FAQ.md`, and `IMPLEMENTATION_DEFINED.md` | Retained historical documentation | Not current Cosmic C product documentation; review before re-enabling any behavior. |

The current repository history does not preserve a single, independently
verified upstream Saltwater commit for its full imported baseline.  BSD-3-Clause
attribution is retained in [`LICENSE.txt`](LICENSE.txt); identifying that exact
upstream baseline remains an M0 task.

## Acceptance boundary

Cosmic C does not use LLVM, a direct SIA encoder, a host executable, a host
JIT, or a reference interpreter as target acceptance.  Floating-point C stays
rejected before CLIF lowering.  Real execution requires the public Lighting
board-call API described in [`TODO.md`](TODO.md); until that API exists, tests
may prove compilation and byte encoding only.
