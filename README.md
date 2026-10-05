# Cosmic C

Cosmic C is the maintained Rust C compiler for the Cosmic operating system.

Its production path is deliberately direct:

```text
C source → target-aware frontend → typed HIR → CLIF → Cranelift
                                                      ├ SIA32 → COSMIC-SIA bundle
                                                      └ amd64 → ELF64 object → system linker
```

Cosmic C reuses the SIA32 backend in [nickik/crainlift](https://github.com/nickik/crainlift). It does not use LLVM. The compiler runs on the build host and targets the freestanding SIA32 Cosmic ABI. SIA output uses versioned `COSMIC-SIA` objects and a bounded static image linker; protected Cosmic loader integration remains open.

## Current targets and status

The default target is little-endian SIA32 ILP32: 32-bit pointers and long, 64-bit
long long, integer argument registers r1–r6, and an eight-byte-aligned stack.
Integer control flow, pointers/memory, globals, aggregates, direct/indirect
calls and VLA cleanup have lowering support and focused execution evidence.
I64 arithmetic has integer legalization and bounded pair/stack ABI execution checks.

SIA has no FPU. C float and double use integer-bit representations and
explicitly linked, integer-only arithmetic/comparison/conversion helpers.
Aggregate arguments/returns and stdarg use documented private Cosmic SIA
conventions. The reviewed profile passes 146/147 cases on LightingMachine;
ordinary double and cross-TU ABI fixtures also pass production CPU/mainboard RTL
in Bluesim. See [compatibility evidence](C_COMPATIBILITY_STATUS.md).
SIA long double uses binary64 precision. C fenv, four rounding modes, C99 real
math families and C-locale decimal memory I/O now have integer-only runtime
implementations and guest execution checks. Broader runtime and physical FPGA
qualification remain open. See [decimal runtime](runtime/decimal/README.md) and
[software floating point](runtime/softfloat/README.md).

The active implementation checklist is [PORTABILITY_TODO.md](PORTABILITY_TODO.md).
[LOWERING_TODO.md](LOWERING_TODO.md) records recent evidence; the early
[TODO.md](TODO.md) roadmap is historical and contains superseded blockers.

A bounded static image linker resolves Abs4 relocations across version 4 objects,
including calls, global addresses and data/function pointers. Per-object private
statics and strings retain their own identity. Its command is:

```sh
cargo run --locked --bin cosmic-link -- --base 0x10000 --entry main \
  --max-bytes 0x40000 -o program.bin main.sia helper.sia
```

It emits raw SIA RAM bytes and prints load/entry addresses and region metadata.
Archive extraction, weak/common symbol coalescing, capability imports and the
protected Cosmic loader remain outstanding. Function starts preserve four-byte literal-pool alignment.

[Runtime foundations](runtime/README.md) provide integer-only memory operations,
software-float bit primitives and wide-integer arithmetic building blocks.
The [soft-float runtime](runtime/softfloat/README.md) supplies compiler helper
dependencies; these foundations are not a full libc.

## Validation

Every SIA-facing change requires focused frontend tests and SIA backend tests. A milestone is complete only when it produces real SIA32 code through `nickik/crainlift`; execution claims additionally require the corresponding proof in LightingSimulation.

`COSMIC-SIA` byte tests and `CallPlan` decoding prove compilation and encoding,
not execution. Cosmic C must not use a host executable, JIT, direct SIA
encoder, or reference interpreter as a substitute for real board evidence.

## Development

Cosmic C is currently maintained with Rust **1.98.1**, selected automatically
by [`rust-toolchain.toml`](rust-toolchain.toml). The supported command matrix
is:

```sh
cargo fmt --all -- --check
cargo test -p saltwater-sia --locked
cargo check --bin cosmicc --locked
cargo check --workspace --locked
sh scripts/check-supported.sh
cargo build --release --bin cosmicc --locked
```

The full supported gate is `sh scripts/check-supported.sh`; CI runs it and also
checks the release compiler build. Workspace tests cover the maintained compiler,
parser and SIA lowering; historical host/JIT runner sources remain outside the
supported Cargo target set. See [MAINTAINED_BASELINE.md](MAINTAINED_BASELINE.md).

To compile a supported integer-only C function:

```sh
printf 'int main(void) { int x = 4; return (x << 2) + 3; }\n' \
  | cargo run --locked -- -o demo.sia -
```

`cosmicc` defaults to `--target sia32-unknown-none`. Use `--target amd64` for the
Linux System V native target described below. SIA ELF and protected Cosmic image
loading remain separate integration work.

The default SIA32 profile remains freestanding. Floating-point programs must
link the four documented software runtime objects. Native amd64 execution is validated
separately; it does not substitute for SIA32 execution evidence.

## License

Cosmic C is distributed under the BSD 3-Clause License. See [LICENSE.txt](LICENSE.txt).

## amd64 Linux target

The maintained CLI integrates current Cranelift ELF object emission for
`x86_64-unknown-linux-gnu` (`amd64` alias). The frontend uses an explicit LP64
model: 64-bit pointers/long/size_t and 32-bit int, independent of the build host.
The initial ABI covers System V scalar parameters/results and native integer
and floating-point values. Unimplemented ABI cases must produce diagnostics.

```sh
# Produce a real ELF64 relocatable object.
cargo run --locked -- --target amd64 -c program.c -o program.o
# Compile and link sources plus existing objects/libraries using the C driver.
cargo run --locked -- --target amd64 program.c helper.c -o program
# Precompiled objects and -L/-l are supported in amd64 linking mode.
cargo run --locked -- --target amd64 program.o -L ./lib -l helper -o program
```

`CC` selects the existing amd64 C driver (default `cc`). It supplies startup
objects, libc and linker defaults; GNU ld or LLVM lld performs ELF linking.
No new amd64 linker is required. `CC` is an executable path, not a shell command.
The Linux startup/runtime integration is separate from Cosmic OS startup.

`-O0` selects Cranelift `none` (the default), `-O1`/`-O2` select `speed`, and
`-Os` selects `speed_and_size`. These options currently apply to amd64 and do
not imply the same optimization pipeline as GCC or Clang. Use a release-built
Cosmic executable when measuring compiler speed.

The [differential compatibility runner](tools/compatibility/README.md) checks
all 220 pinned upstream cases against GCC and Clang at O0/O2. The
[laptop benchmark tools](tools/benchmarks/README.md) measure validated native
kernels and source-to-object compilation separately from linking. See also
[published backend comparisons](tools/benchmarks/PUBLISHED_COMPARISONS.md).
See [amd64 acceptance tools](tools/amd64/README.md) and
[TARGET_OUTPUT_STATUS.md](TARGET_OUTPUT_STATUS.md) for evidence and limits.
