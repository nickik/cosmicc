# Cosmic C

Cosmic C is the maintained Rust C compiler for the Cosmic operating system.

Its production path is deliberately direct:

```text
C99 source → preprocessor → parser → typed HIR → CLIF → Cranelift SIA32 → COSMIC-SIA bundle
```

Cosmic C reuses the SIA32 backend in [nickik/crainlift](https://github.com/nickik/crainlift). It does not use LLVM. The compiler runs on the build host and targets the freestanding SIA32 Cosmic ABI. Its initial on-disk output is a small `COSMIC-SIA` code bundle; relocatable Cosmic objects and images are the next output milestone.

## Initial target

The first implemented profile is intentionally small and freestanding:

- 32-bit little-endian pointers and data model
- functions with parameters, explicit returns, initialized SSA locals, and integer arithmetic/bitwise shifts
- fixed SIA ABI: `r1–r6` argument registers, `r1` result, 8-byte stack alignment
- no host linker and no hidden fallback to a host ISA

The initial SIA32 backend deliberately rejects floating point, target varargs, globals, calls, pointer dereferences, control flow, TLS, unwinding, SIMD, and tail calls. Cosmic C must diagnose every unsupported construct clearly rather than emit incorrect code.

## Status

The `cosmicc` command now parses C with the existing frontend and compiles the supported integer subset through Cranelift's production SIA32 path. It writes a `COSMIC-SIA` bundle containing word-aligned native SIA instructions. Floating-point declarations and literals fail before CLIF generation.

The authoritative roadmap is [TODO.md](TODO.md).

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
checks the release compiler build. `cargo test --workspace` is intentionally
not a gate while inherited host-runner/JIT compatibility sources are retained
outside the supported Cargo target set. See
[`MAINTAINED_BASELINE.md`](MAINTAINED_BASELINE.md) for the exact inventory.

To compile a supported integer-only C function:

```sh
printf 'int main(void) { int x = 4; return (x << 2) + 3; }\n' \
  | cargo run --locked -- -o demo.sia -
```

`cosmicc` accepts `--target sia32-unknown-none`; this is the concrete target triple currently understood by the Cranelift SIA backend. The Cosmic-specific object/image target will follow once its object format is defined.

The supported profile remains freestanding and SIA-only: float is rejected
before CLIF lowering, and no LLVM, host execution, JIT acceptance, or direct
SIA encoding is part of the product path.

## License

Cosmic C is distributed under the BSD 3-Clause License. See [LICENSE.txt](LICENSE.txt).
