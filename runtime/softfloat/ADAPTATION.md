# SoftFloat 3e binary32 addition adaptation

Upstream archive: <https://www.jhauser.us/arithmetic/SoftFloat-3e.zip>.
SHA256 `21130ce885d35c1fe73fc1e1bf2244178167e05c6747cad5f450cc991714c746`.
The original files used by the reference closure are retained byte-for-byte in
`upstream/`; `upstream/SHA256.json` verifies them. Each inherited implementation
fragment retains its complete upstream copyright and redistribution notice.
The distribution license is also preserved in `UPSTREAM-LICENSE.txt`.

The licensed binary32-add arithmetic closure is adapted as follows:

- Preserve the arithmetic, normalization, sticky-bit shifts and rounding logic.
- Use unsigned 32-bit raw bits instead of a float32_t aggregate wrapper/union.
- Add explicit `cosmic_sf_context *` parameters to stateful helpers, replace
  raiseFlags calls with context accumulation, and redirect direct flag ORs to
  that context. Arithmetic does not use upstream SoftFloat global state. The
  later explicit task binding and static boot fallback are documented separately.
- Initially fixed rounding to nearest ties-even and tininess after rounding. Other rounding
  branches remained inherited and were unreachable in that historical snapshot;
  the current runtime exposes four modes through context rounding. No arithmetic
  traps are enabled. Both signed zeros, gradual underflow and infinities remain.
- Retain ARM-VFPv2 signaling/quiet NaN and payload behavior. A signaling NaN
  raises invalid and is quieted; invalid infinity cancellation returns 0x7fc00000.
- Namespace internal symbols and give helpers/table translation-unit-local
  linkage. Supply only the fixed-width types and macros this closure reaches;
  its arithmetic requires no 64-bit operations despite full SoftFloat 3e's I64
  prerequisite. This does not remove that prerequisite from the full library.

`adapt-upstream.py` verifies the preserved file hashes and reproduces/checks
these mechanical arithmetic-fragment transformations. The public entry point
replaces upstream f32_add's aggregate unpack/pack with raw-bit arguments/return
and retains its sign-directed magnitude dispatch. `binary32.h` documents the
raw-bit ABI; it is not a cross-language ABI freeze or compiler legalization.

`check-add32.py` builds an unmodified upstream ARM-VFPv2 closure and a test-only
reference wrapper using ordinary globals in one single-threaded test process.
Both oracle and adapted arithmetic are integer-only; no host floating-point
operation supplies results. It checks 14 manually specified transparent anchors,
676 edge-pattern pairs and 100,000 seeded random bit-pattern pairs, comparing
both output bits and exception flags. It emits 754 edge/anchor/random guest
vectors. This is an upstream-reference differential sample, not a full TestFloat
run or an IEEE-conformance certificate.

The integer host guest fixture passes. The same 754 vectors plus independent
context/sticky-flag checks pass on LightingMachine architectural execution:
453,343 guest instructions, zero return, restored stack, no trap. Report:
`/tmp/cosmic-softfloat-add32-lighting.log`. No composed-board test has been run
for this slice. Compiler FP source legalization is separate; other arithmetic
and conversions are not established by addition evidence.

Reproduce:

```sh
python3 runtime/softfloat/adapt-upstream.py
python3 runtime/softfloat/check-add32.py --output /tmp/cosmic-sf-new
cc -E -P runtime/softfloat/add32-test.c -o /tmp/cosmic-sf-guest.c
cargo run --manifest-path tools/l21-execution/Cargo.toml \
  --bin sia-source-native -- /tmp/cosmic-sf-guest.c
```

The single-TU guest fixture includes sources for convenience; applications can
instead independently compile/link context.c and binary32-add.c. Contexts must
be valid aligned task-owned guest storage; sharing one concurrently requires
caller synchronization. No allocation or host callback is required.

## Subtraction, multiplication and division extension

The same source module now exports cosmic_sf_sub32/mul32/div32. Subtraction
preserves the upstream f32_sub sign-directed dispatch and original operand NaN
bits; blindly flipping the second input's sign before add would change NaN
payload/sign behavior. Multiplication/division retain upstream integer arithmetic
and the same explicit context/rounding/special-value policy as addition.

The new inherited closure adds normSubnormalF32Sig and shortShiftRightJam64.
The former's internal struct-return ABI is mechanically changed to an output
pointer with the original exponent/significand computations intact. Multiplication
requires uint64_t multiplication and jam-shifting. Division uses upstream's
SOFTFLOAT_FAST_DIV64TO32 configuration: a 64-bit integer numerator, integer
quotient and sticky remainder check. This selects a bounded readily audited
path; it does not claim efficient hardware 64-bit division on SIA. SIA executes
these operations through its actual integer lowering/runtime, without host FP.

An important underflow boundary is explicitly tested: 0x00800000 multiplied by
0x3f7fffff rounds to 0x00800000 and raises underflow plus inexact (flags 3).
This follows upstream's after-rounding policy. Tininess is evaluated using the
destination precision with an unbounded exponent before the final gradual-
underflow rounding, so a final normal result can still raise underflow. The
[upstream FAQ](https://www.jhauser.us/arithmetic/SoftFloat-3/doc/SoftFloat-FAQ.html)
explains this boundary; clearing underflow just because output is normal would
change the required semantics.

`check-arithmetic32.py` validates 302,045 output-bit and fresh-zero-context flag
results against unchanged upstream integer code: 100,000 random pairs per
operation plus edge pairs and manual anchors. Observed flags include exact,
inexact, underflow+inexact, overflow+inexact, divide-by-zero and invalid. The
521-vector guest fixture additionally checks independent contexts and sticky
accumulation across division and multiplication. It passes on LightingMachine:
1,061,772 guest instructions, zero return, restored stack and no trap. The add
fixture was rerun with the expanded module and remains 453,343 instructions.
Durable reference JSON, guest logs, source/runner hashes and counts are in
`results/`. This is architectural simulation, not composed FPGA board proof.

```sh
python3 runtime/softfloat/check-arithmetic32.py --output /tmp/cosmic-sf-arith-new
cc -E -P runtime/softfloat/arithmetic32-test.c -o /tmp/cosmic-sf-arith-guest.c
cargo run --manifest-path tools/l21-execution/Cargo.toml \
  --bin sia-source-native -- /tmp/cosmic-sf-arith-guest.c
```

Comparisons and integer conversions live in the separately maintained
binary32-convert.c slice. These raw-bit API tests do not themselves validate
compiler lowering of C float expressions, binary64, decimal I/O, libm, long
double, complete TestFloat coverage, or cross-language FP ABI interoperability.

The later persistent-environment extension reads rounding from the explicit
context and preserves all four supported upstream modes; see
[FENV_MATH_STATUS.md](FENV_MATH_STATUS.md) for current validation.
