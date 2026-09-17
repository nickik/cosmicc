# Contributing to Cosmic C

Cosmic C is a freestanding C-to-SIA32 compiler. The maintained product path is
`C -> typed HIR -> CLIF -> Cranelift SIA32 -> COSMIC-SIA`. Contributions must
preserve that path and its explicit unsupported-feature diagnostics.

## Before submitting a change

Use the Rust **1.98.1** toolchain selected by `rust-toolchain.toml`, then run:

```sh
sh scripts/check-supported.sh
cargo build --release --bin cosmicc --locked
```

The script runs formatting, the focused C-to-SIA test suite, the compiler
command check, and a check of every maintained workspace package. A lockfile
change must be intentional: supported commands use `--locked`.

Add focused tests in `saltwater-sia` for a change to C lowering, CLIF shape,
bundle decoding, or emitted SIA bytes. State precisely what the test proves.
Byte-generation and `CallPlan` tests do **not** prove code execution.

## Acceptance rules

- Do not add a host executable, host JIT, reference interpreter, or direct SIA
  encoder as acceptance evidence.
- Do not add LLVM or floating-point support. Floating-point declarations and
  literals must remain rejected before CLIF lowering.
- Keep target output SIA32-only. The host runs the compiler, not the compiled
  program.
- Real execution evidence requires the public Lighting board-call runner. Its
  current external blocker and the Cranelift comparison blocker are documented
  in [`TODO.md`](TODO.md).

## Retained historical material

The repository still contains Saltwater-derived parser sources and retired
host-codegen, runner, fuzzing, benchmark, and minimizer material. These are
not default Cargo targets and must not be re-enabled accidentally. The exact
classification is in [`MAINTAINED_BASELINE.md`](MAINTAINED_BASELINE.md).

For substantial work, start from a TODO milestone, describe the required
evidence, and keep the pull request limited to one coherent compiler slice.
