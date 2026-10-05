# Target-native runtime foundations

`primitives.c` compiles with Cosmic C into SIA32 instructions. It contains memory
copy/move/set/compare, raw IEEE binary32 sign/classification, binary64
classification, 32/64-bit right-shift-with-sticky-bit, unsigned 32×32→64
multiplication and unsigned 64-bit quotient/remainder using only I32 operations.

These are explicit `cosmic_*` interfaces, not a hosted libc or compiler-wired
soft-float implementation. `cosmic_words64` uses low then high 32-bit words.
The arithmetic interfaces use output pointers so they do not presume an
unverified I64 return ABI. Zero division returns 1 without changing outputs;
quotient and remainder objects must be distinct. Memory calls require valid
guest buffers; memcpy requires non-overlap. Standard const-correct libc
interfaces, allocator, sysroot and automatic builtin wiring remain separate work.

primitives.c supplies bit operations rather than FP arithmetic. The separate
[softfloat runtime](softfloat/README.md) now implements binary32/binary64
arithmetic, comparisons and 32/64-bit conversions, wired through compiler
helper calls. Licensing, reference checks and execution logs are retained there.

## Reproducible validation

From cosmicc:

```sh
sh scripts/check-supported.sh
cargo run --manifest-path tools/l21-execution/Cargo.toml --locked --bin runtime-native
PATH=/path/to/bluespec/bin:$PATH \
LIGHTING_MAINBOARD_M6_MULTISLOT_BRIDGE=/path/to/mainboard-multislot-bridge \
  cargo run --manifest-path tools/l21-execution/Cargo.toml --locked \
  --bin runtime-native -- --board
```

Build the public Bluesim bridge with scripts/check-l21-execution.sh if needed.
The architectural run uses LightingMachine; --board uses the public
production HardwareCpuBoard→MainboardFPGA→RAM runner. Both execute compiler-produced,
linked guest code. Host arithmetic only generates expected test data. Coverage:
50 sticky shifts, 25 wide products, 41 quotient/remainder cases, zero-divisor
output preservation, signed-zero/infinity/NaN classification, odd-sized memory
buffers, both overlap directions, nested/array offsetof-style addresses,
qualified-pointer reassignment and code/data/function-pointer relocations.
Guest exit must return zero and restore the stack. Permissions recorded by the
linker are metadata; this flat RAM test does not prove Cosmic loader protection.

The execution tool currently inherits pre-existing sibling development patches
from its Cargo.toml. A clean, published-pin acceptance run remains required.

## Local acceptance — 2026-10-04

Both execution modes passed the final guest in 466,209 instructions with a zero
result and restored stack. The maintained workspace gate and optimized builds
of cosmicc/cosmic-link passed. Composed-board logs are in
/tmp/cosmicc-runtime-board-final.log; architectural logs are in
/tmp/cosmicc-runtime-native-final.log. This remains local development acceptance
with the execution tool's existing sibling patches, not remote CI evidence.
