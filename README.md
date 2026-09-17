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

## Development

```sh
cargo test -p saltwater-sia
printf 'int main(void) { int x = 4; return (x << 2) + 3; }\n' | cargo run -- -o demo.sia -
```

`cosmicc` accepts `--target sia32-unknown-none`; this is the concrete target triple currently understood by the Cranelift SIA backend. The Cosmic-specific object/image target will follow once its object format is defined.

## License

Cosmic C is distributed under the BSD 3-Clause License. See [LICENSE.txt](LICENSE.txt).
