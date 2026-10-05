# Contributing to Cosmic C

Cosmic C has a freestanding SIA32 target and an AMD64 Linux target. The maintained product path is
`C -> typed HIR -> CLIF -> Cranelift SIA32 -> COSMIC-SIA`. Contributions must
preserve that path and its explicit unsupported-feature diagnostics.

## Before submitting a change

Use the Rust **1.98.1** toolchain selected by `rust-toolchain.toml`, then run:

```sh
sh scripts/check-supported.sh
cargo build --release --bin cosmicc --locked
```

The script runs formatting, workspace tests, the compiler build,
and a check of every maintained workspace package. A lockfile
change must be intentional: supported commands use `--locked`.

Add focused tests in `saltwater-sia` for a change to C lowering, CLIF shape,
bundle decoding, or emitted SIA bytes. State precisely what the test proves.
Byte-generation and `CallPlan` tests do **not** prove code execution.

## Acceptance rules

- SIA acceptance must execute emitted guest instructions on Lighting architectural
  or production RTL paths, without host runtime callbacks or a private encoder.
  AMD64 acceptance uses native output and GCC/Clang differential and ABI checks.
- Do not add an LLVM product pipeline. SIA has no FPU: floating-point support
  must use documented software legalization and integer-only target runtime
  helpers. Until implemented, retain explicit unsupported emission diagnostics.
- Keep SIA and AMD64 output/ABI contracts separate. SIA uses the versioned
  COSMIC-SIA bundle; AMD64 emits ELF64 and uses the system linker.
- State which Lighting machine path supplied execution evidence. Production board
  acceptance uses HardwareCpuBoard/MainboardFPGA RTL; architectural
  LightingMachine execution is a separately labelled integration result.
- Use [PORTABILITY_TODO.md](PORTABILITY_TODO.md) for the active portability work.

## Retained historical material

The repository still contains Saltwater-derived parser sources and retired
host-codegen, runner, fuzzing, benchmark, and minimizer material. These are
not default Cargo targets and must not be re-enabled accidentally. The exact
classification is in [`MAINTAINED_BASELINE.md`](MAINTAINED_BASELINE.md).

For substantial work, start from a TODO milestone, describe the required
evidence, and keep the pull request limited to one coherent compiler slice.
