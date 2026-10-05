# SIA software floating-point runtime

Current status: task-bound fenv, four rounding modes, binary32/binary64 arithmetic,
C99 real math interfaces and binary64-alias long double are implemented and
pass recorded guest and production-board integration. See
[environment/math evidence and accuracy limits](FENV_MATH_STATUS.md),
[decimal text I/O](../decimal/README.md), and
[compiler/production-board integration](../../SIA_FLOAT_LANGUAGE_STATUS.md).
The implicit environment uses an explicit scheduler binding and a static boot
fallback; each task must bind initialized guest-owned storage.

Historical arithmetic evidence, 2026-10-05: integer-only binary32 add/subtract/multiply/divide are implemented.
Addition passes 754 guest vectors; sub/mul/div pass 521 vectors on LightingMachine
architectural execution. All four match unchanged upstream SoftFloat output bits
and flags over the recorded integer-reference samples. Binary64 arithmetic,
comparisons and binary32/64/I64 conversions are also implemented and pass
2,105,005 integer-reference comparisons plus 1,169 LightingMachine guest vectors;
see [binary64 status](BINARY64_STATUS.md).
The licensed arithmetic is adapted from SoftFloat 3e with explicit task context;
see [adaptation and evidence](ADAPTATION.md). Compiler source-FP legalization and
composed-board acceptance have [separate integration evidence](../../SIA_FLOAT_LANGUAGE_STATUS.md).
A shared C/Forge/Rust ABI freeze still requires cross-language evidence.
SIA has no FPU.

## Current C runtime boundary

Use unsigned 32-bit raw bits for binary32 and unsigned long long raw bits for
binary64. Binary64 memory uses low word then high word, preserving the existing
little-endian integer convention. Every arithmetic helper takes an explicit guest
`cosmic_sf_context *`. Current interfaces include:

```c
unsigned int cosmic_sf_add32(cosmic_sf_context *, unsigned int, unsigned int);
unsigned long long cosmic_sf_add64(cosmic_sf_context *,
    unsigned long long, unsigned long long);
```

These scalar interfaces are validated through SIA's integer ABI; binary32.h and
BINARY64_STATUS.md record all actual declarations. Normal integer
ABI argument allocation and indirect calls apply. Verify mixed arguments,
register exhaustion, stack alignment and returns across all three languages
before adopting any by-value binary64 ABI. Context storage must be task-owned;
shared process globals and save/restore of global SoftFloat state are not safe
under preemption. Context objects require valid aligned guest addresses;
simultaneous use of one context requires caller synchronization.

Implemented semantic policy: binary32/binary64, task-selected nearest ties-to-even,
toward-zero, downward or upward rounding (nearest at initialization),
gradual underflow (no flush-to-zero), tininess after rounding, signed zero and
infinity preservation, quiet bit set on propagated signaling NaNs with invalid
raised. Upstream ARM-VFPv2 specialization's payload behavior and default
NaN bits are preserved and covered by recorded vectors. Negate/abs/copy
preserve payloads and do not raise flags. Exception flags accumulate per context;
no arithmetic traps initially. Comparisons distinguish quiet equality from
signaling ordered comparisons; unordered must never become integer ordering.
Integer conversions use round-toward-zero for C casts. Out-of-range and NaN
conversion raises invalid and uses documented upstream specialization results;
C's undefined conversion cases are not promised portable numeric values.
SIA long double uses binary64 precision and storage with a distinct C type.
Decimal conversion and memory formatting have [separate implementation evidence](../decimal/README.md);
C99 real math and software fenv have [current contracts](FENV_MATH_STATUS.md).

## Audited upstream and prerequisite closure

Official [SoftFloat interface](https://www.jhauser.us/arithmetic/SoftFloat-3/doc/SoftFloat.html)
and [porting guide](https://www.jhauser.us/arithmetic/SoftFloat-3/doc/SoftFloat-source.html)
were read; official 3e archive downloaded and source inspected locally.
Archive: https://www.jhauser.us/arithmetic/SoftFloat-3e.zip
SHA-256: `21130ce885d35c1fe73fc1e1bf2244178167e05c6747cad5f450cc991714c746`.
The distribution license is retained in UPSTREAM-LICENSE.txt; the bounded addition closure and reference headers are now vendored,
with original hashes/notices retained and adaptation documented.

Release 3e requires 64-bit integer arithmetic, even on targets without FAST_INT64.
Do not confuse that option with a 32-bit-only implementation. Disable host
intrinsic/int128 optimization hooks initially. Supply target-owned stdint/stdbool
including fast integer types, platform.h and explicit endianness. Test integer
shifts, multiplication, division, casts, carry propagation and word-pair ABI
before compiling the complete arithmetic runtime.

A bounded first arithmetic closure is f32_add.c, s_addMagsF32.c,
s_subMagsF32.c, s_roundPackToF32.c, s_normRoundPackToF32.c,
s_shiftRightJam32.c, s_countLeadingZeros32.c plus its lookup table and
specialization NaN/raiseFlags support. Define INLINE_LEVEL >= 1 to avoid
f32_add's optional indirect magnitude-function call. Required headers still
include uint64_t definitions and other declarations; this bounded raw-bit adapted closure now compiles on Cosmic and executes
on LightingMachine. Multiplication/conversions/double
extend the closure into 64-bit operations and must not be declared supported
from binary32 addition results.

Upstream roundPack directly reads roundingMode/detectTininess and directly ORs
exceptionFlags; replacing only raiseFlags is insufficient. Adapt every reachable
state access to explicit context passing, preserving upstream notices and
recording the patch. Read rounding from task context; retain the tininess-after-rounding constant
only after equivalence tests. Keep upstream integer arithmetic intact.

## Implemented slice and checks

context.h/context.c implement initialization, masked sticky flag accumulation,
query and selective clearing on explicit contexts. The current implicit binding
uses a static boot fallback until the scheduler binds task storage; no upstream
SoftFloat global state is used by arithmetic. context-test.c tests independent
contexts, accumulation, clearing and ignored reserved flag bits.

```sh
cosmicc/target/release/cosmicc -o /tmp/cosmic-softfloat-context.sia \
  cosmicc/runtime/softfloat/context-test.c
cc -std=c99 -Wall -Wextra -Werror \
  cosmicc/runtime/softfloat/context-test.c -o /tmp/cosmic-softfloat-context-test
/tmp/cosmic-softfloat-context-test
```

Both commands passed locally. The first is emitted-SIA evidence only; the second
is a host test of integer context bookkeeping, not host floating-point substitution
or Lighting execution. Addition now has separate bit-exact upstream-reference evidence and
LightingMachine guest execution; see ADAPTATION.md. Subtraction/multiplication/division now have separate integer-reference and
LightingMachine evidence in ADAPTATION.md. No full TestFloat or composed-board claim
is made.

## Implemented binary32 addition slice

`binary32.h` exposes `cosmic_sf_add32(ctx,a_bits,b_bits)`. The licensed
SoftFloat3e ARM-VFPv2 closure is retained in upstream/ with SHA256 provenance
and adapted/ with explicit context passing. `adapt-upstream.py` records the
mechanical adaptation. The implementation uses integer-only arithmetic; fixed
task-selected rounding (initial nearest ties-even), tininess after rounding, gradual underflow, signed
zero, infinities and ARM-VFPv2 NaN payload/default policy are preserved. Exception
flags accumulate in caller-owned context, without a global task flag.

`check-add32.py --output NEW_DIRECTORY` verifies original source hashes and
compares result bits/flags against unmodified upstream integer arithmetic:
14 independent anchors, 676 edge pairs and 100,000 seeded random pairs pass.
754 saved vectors execute on LightingMachine in 453,343 instructions, also
checking sticky flags and independent contexts. This is architectural execution,
not production board evidence. Original licensing is preserved in every adapted
source and UPSTREAM-LICENSE.txt.

```sh
python3 runtime/softfloat/check-add32.py --output /tmp/add32-new-check
cc -E -P -I runtime/softfloat runtime/softfloat/add32-test.c -o /tmp/add32-flat.c
cargo run --manifest-path tools/l21-execution/Cargo.toml --offline --quiet \
  --bin sia-source-native -- /tmp/add32-flat.c
```

The raw API transports IEEE bits as unsigned int. Ordinary C binary32 expressions
now lower through these routines; see [C float integration](../../SIA_FLOAT_LANGUAGE_STATUS.md).
Addition, subtraction, multiplication, division, comparisons and 32-bit integer
conversions are implemented. Binary64, float/I64 conversions, decimal I/O and
C fenv remain open. Earlier prerequisite audits describe historical boundaries.

## Binary32 comparisons and integer conversions

`binary32-convert.c` implements equality, ordered less/less-equal, conversions
to signed/unsigned 32-bit integers with truncation toward zero, and conversions
from those integers with task-selected rounding. Equality raises invalid
for signaling NaNs; ordered comparisons raise invalid for any NaN. Both signed
zeros compare equal. Float-to-integer invalid results follow the ARM-VFPv2
saturation/NaN policy; this does not define otherwise undefined out-of-range
C casts. All routines use caller-owned exception state.

`check-convert32.py` compares 705,103 operation results and fresh flags against
unmodified upstream integer code, repeating each with sticky preexisting flags.
5327 saved vectors execute with fresh/sticky contexts on LightingMachine in
6,915,714 instructions. Raw reference metadata and architectural logs are
retained in results/.

```sh
python3 runtime/softfloat/check-convert32.py --output /tmp/convert32-new-check
cc -E -P -I runtime/softfloat runtime/softfloat/convert32-test.c -o /tmp/convert32-flat.c
cargo run --manifest-path tools/l21-execution/Cargo.toml --offline --quiet \
  --bin sia-source-native -- /tmp/convert32-flat.c --max-instructions 10000000
```

The runner has a bounded explicit instruction budget; the default 2 million is
insufficient for this expanded vector fixture. Signed/unsigned 64-bit conversions
and binary64 format conversion are covered by [binary64 validation](BINARY64_STATUS.md).

## Persistent environment and math

The context now contains flags and rounding (eight bytes). Standard fenv APIs,
task binding, basic math and a licensed fdlibm transcendental slice are implemented;
see [current environment/math contract and evidence](FENV_MATH_STATUS.md).
Earlier results above describe their recorded nearest-only source snapshots.
