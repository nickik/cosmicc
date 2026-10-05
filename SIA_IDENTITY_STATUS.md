# SIA C identity and narrow conversion acceptance

The SIA lowering now coalesces same-translation-unit declarations by C
identifier, preferring initialized definitions over tentative declarations and
storage definitions over declaration-only externs. All associated HIR symbols
refer to one artifact index and one object. Block-scope function declarations
allocate no local storage; calls and function addresses resolve the existing
translation-unit identifier. Falling off `int main` returns zero. Other
non-void functions retain the explicit-return requirement.

Narrow-to-narrow integer conversions widen through the native 32-bit word and
then reduce to the destination width. This avoids unsupported `uextend.i16`
while preserving signed and unsigned conversions. No backend dependency patch
was required for this change.

Five unchanged upstream reviewed cases compiled, linked and executed on
LightingMachine architectural execution with zero return, restored stack and
no trap: 00051 (59 instructions), 00078 (71), 00096 (33), 00136 (2), 00128 (942).
Evidence: `/tmp/cosmic-sia-five-native.log`. Case 00096's upstream predicate does
not enforce its initial value strongly, so an independent defined fixture
(`tools/compatibility/fixtures/sia-identity.c`) checks `x==3`, a block-scoped function declaration call, nonzero unsigned-byte
to unsigned-short widening, and actual implicit main return. It passed 129
instructions: `/tmp/cosmic-sia-identity-defined.log`.

The reusable runner is `tools/l21-execution/src/bin/sia-source-native.rs`:

```
cargo run --manifest-path tools/l21-execution/Cargo.toml --bin sia-source-native --offline -- /path/to/case.c
```

This is guest instruction execution, not host C execution. The optional board
mode was not used for these tests; no composed board acceptance is claimed.

Ordinary I64 multiply, divide and shifts now dispatch to the integer-only
legalization helpers implemented in `saltwater-sia/integer.rs`, retaining the
existing I32 path. Their broader defined arithmetic/ABI execution evidence is
recorded by the parent worker separately. Full SIA crate tests passed 154 tests:
`/tmp/cosmic-sia-five-plus-i64-gate.log` (published backend pin951, no overrides).

## Defined I64 ABI stress and division policy

`tools/sia/i64-abi-stress.c` independently checks twelve I64 arguments (register
pair exhaustion and fixed stack arguments), sixteen alternating I32/I64
arguments (pair alignment and exhaustion at a mixed boundary), indirect I64
calls, negative signed I64 stack arguments, I64 return values, and sixteen
values held live across nested calls. The latter pressures caller/callee
preservation and register spilling; this report does not infer exact spill
counts from source. All arithmetic is defined unsigned modular arithmetic or
bounded signed arithmetic. GCC13 and Clang18 each passed at O0 and O2.

The unchanged Cosmic SIA fixture compiled, linked and executed successfully on
LightingMachine, returning zero with restored stack and no architectural trap.
Execution log: `/tmp/cosmic-sia-i64-abi-stress.log`. No compiler/backend fix was
needed for this slice; it uses the currently published backend pin and public
compile/link APIs. Run it through the source runner above.

`tools/l21-execution/src/bin/sia-i64-traps.rs` separately checks the compiler's
chosen trap policy for division by zero and signed MIN/-1 division. These
inputs have undefined C behavior and are **not** differential conformance
cases. Both emitted architectural TRAP cause3 before returning: zero divisor
55 instructions, signed overflow81. Log: `/tmp/cosmic-sia-i64-traps.log`.
Run with `cargo run --manifest-path tools/l21-execution/Cargo.toml --bin
sia-i64-traps --offline`. These are architectural execution results; composed
board trap propagation and external cross-compiler SIA ABI interoperability
remain separate acceptance work.
