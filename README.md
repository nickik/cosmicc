# Cosmic C

Cosmic C is the maintained Rust C compiler for the Cosmic operating system.

Its production path is deliberately direct:

```text
C99 source → preprocessor → parser → typed HIR → CLIF → Cranelift SIA32 → Cosmic object/image
```

Cosmic C reuses the SIA32 backend in [nickik/crainlift](https://github.com/nickik/crainlift). It does not use LLVM. The compiler runs on the build host initially and produces freestanding `sia32-unknown-cosmic` objects for Cosmic.

## Initial target

The first supported target is a freestanding C99 integer/pointer profile:

- 32-bit little-endian pointers and data model
- ordinary functions, globals, control flow, scalar loads/stores, and integer arithmetic
- fixed SIA ABI: `r1–r6` argument registers, `r1` result, 8-byte stack alignment
- explicit Cosmic platform imports only; no hosted libc or host linker

The initial SIA32 backend deliberately rejects floating point, I64/`long long`, target varargs, TLS, unwinding, SIMD, and tail calls. Cosmic C must diagnose every unsupported construct clearly rather than emit incorrect code.

## Status

The C frontend, preprocessor, typed HIR, and Cranelift-oriented code-generation structure are present. Current work modernizes the Cranelift integration and replaces host-x86-specific linking/output with the Cosmic SIA32 target.

The authoritative roadmap is [TODO.md](TODO.md).

## Validation

Every SIA-facing change requires focused frontend tests and SIA backend tests. A milestone is complete only when it produces real SIA32 code through `nickik/crainlift`; execution claims additionally require the corresponding proof in LightingSimulation.

## Development

```sh
cargo test --workspace
cargo run -- --help
```

The command-line interface and package layout may change while the compiler is retargeted.

## License

Cosmic C is distributed under the BSD 3-Clause License. See [LICENSE.txt](LICENSE.txt).
