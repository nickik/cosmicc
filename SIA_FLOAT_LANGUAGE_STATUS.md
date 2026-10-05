# SIA C software floating-point integration

Current C `float` and `double` values travel as integer-register IEEE binary32
and binary64 bits. Scalar parameters/returns, direct/indirect calls, static and
automatic memory, aggregate members, addressed values and stack arguments use
this transport. Arithmetic and all six comparisons dispatch to integer-only
runtime helpers through ordinary external relocations. Negation flips the
appropriate sign bit; postfix/compound updates preserve lvalue capture.

All signed/unsigned 32-bit and 64-bit integer conversions, narrower integer
casts, and float/double width conversions use the corresponding runtime helper.
C `_Bool` conversion tests the complete scalar value: neither signed zero is
true, NaNs are true, and high-only integer/pointer bits are retained. The new
binary64 NaN fixture exposed and fixed the previous integer-to-bool truncation
bug which erased NaN payload bits in the runtime's `isNaNF64UI` predicate.

Each generated helper call fetches `cosmic_sf_current_context()` and uses the
bound task environment: two U32 words, sticky flags and rounding mode. The boot
default is nearest ties-even; nearest/toward-zero/downward/upward modes and
standard exception flags are exposed through the SIA `fenv.h` API. A scheduler
must bind and restore contexts for task switches; there is no automatic TLS.

`#pragma STDC FENV_ACCESS ON`, OFF and DEFAULT are recognized. SIA conservatively
preserves runtime floating operations in all modes: the frontend's ordinary
runtime expression paths do not invoke constant folding, and generated runtime
calls are side-effecting. Required constant contexts and static initializers
still fold at translation-time nearest precision. An independent constant-
operand runtime fixture checks upward rounding/inexact and 0/0 invalid flags.

Long double is a distinct C type on SIA with explicit binary64 precision,
range, eight-byte storage and alignment; `float.h` sets LDBL parameters equal to
DBL parameters. Its arithmetic/conversions use the binary64 helpers. AMD64
long-double declarations/literals/layout/lowering remain explicitly rejected
until an x87 ABI exists. The frontend directly parses binary32 suffixes and
preserves `L` literal type; binary32 folding rounds each typed operation.

SIA variadic default promotions convert float to double; long double retains
its distinct type and eight-byte scalar pack representation. Variadic/aggregate
ABI evidence is maintained separately. Transcendental/library coverage is
listed in the runtime math report, not implied by basic arithmetic support.
Out-of-range floating-to-integer casts remain undefined C behavior; the
runtime's saturation policy is not a C guarantee.

## Actual C execution

Both maintained fixtures invoke C floating operators, not raw-bit API wrappers.
They are compiled separately from context.c, binary32-add.c,
binary32-convert.c, binary64.c and fenv.c and linked with public checked
`Artifact::link_images`. They pass GCC13 and Clang18 at O0/O2.

| Fixture | LightingMachine instructions | Evidence |
| --- | ---: | --- |
| `tools/sia/float-language.c` | 35,643 | `/tmp/cosmic-float-language-current-fenv.log` |
| `tools/sia/float64-language.c` | 74,128 | `/tmp/cosmic-float64-language-current-fenv.log` |
| `tools/sia/bool-conversions.c` | 626 | `/tmp/cosmic-sia-bool-casts-frontend.log` |

All returned zero, restored stack, and produced no architectural trap. The current FP counts supersede earlier results before the task environment integration.
Parent-run production-board results are tracked separately; this table records
architectural LightingMachine evidence only.

Coverage includes arithmetic, all comparisons, finite/narrow and signed/
unsigned integer casts, both FP widths, signed-zero bits/truth, NaN unordered
comparisons/truth, infinity, static/automatic arrays/struct members,
addressed loads/stores, postfix/compound updates, direct/indirect FP calls,
eight float/double arguments, mixed I32/I64 boundaries, and constant rounding.
Binary32 has an independent decimal just above a halfway point bit check;
binary64 narrowing checks nearest ties-even at a representable halfway value.
Runtime oracle/vector coverage is detailed in `runtime/softfloat/README.md`.

Reproduce from the cosmicc workspace:

```
cargo run --manifest-path tools/l21-execution/Cargo.toml --bin sia-float-language --offline -- tools/sia/float-language.c
cargo run --manifest-path tools/l21-execution/Cargo.toml --bin sia-float-language --offline -- tools/sia/float64-language.c
cargo run --manifest-path tools/l21-execution/Cargo.toml --bin sia-source-native --offline -- tools/sia/bool-conversions.c
```

The FP runner writes versioned runtime containers under
`/tmp/cosmic-sia-float-runtime-objects/`. It invokes no host C compiler for guest
execution. Public `compile` emits external helper dependencies; applications
must explicitly provide runtime objects when linking.

Published-pin focused gates passed 140 parser unit tests, 19 preprocessor,
14 target-model, 151 SIA and five doctests (two ignored):
`/tmp/cosmic-longdouble-fenv-gate.log`. Full reviewed-suite, cross-TU
varargs and production-board acceptance are captured by the integration worker
with their own compiler/runtime snapshots.


## Long double and environment execution

`tools/sia/longdouble-fenv-language.c` passed 19,830 LightingMachine instructions
with standard fenv calls, long-double arithmetic/casts/indirect calls, all
rounding state transitions used by the fixture, sticky flags, hold/update
state, and explicit task-context binding. Log:
`/tmp/cosmic-longdouble-fenv-language.log`. GCC13/Clang18 O0/O2 references passed
with `-frounding-math`; target-specific LDBL/FLT_ROUNDS assertions and context
binding are guarded by target header macros. GCC13's FLT_ROUNDS macro was
observed to stay constant under dynamic modes, so that target-specific check is
not treated as a host-reference consensus claim.

`tools/sia/fenv-constant-runtime.c` passed 3,693 architectural instructions:
`/tmp/cosmic-fenv-constant-language.log`. Clang18 O0/O2 and GCC13 O0 passed;
GCC13 O2 returned3 because the constant 0/0 operation did not expose FE_INVALID.
The compiler reference disagreement is recorded, not substituted for the guest
flag/rounding assertions. Both Cosmic fixtures returned zero, restored stack
and had no architectural trap. Parent-run production proofs are separate.
