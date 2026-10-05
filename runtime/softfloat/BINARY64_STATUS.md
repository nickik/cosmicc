# Integer-only binary64 and width/I64 conversion runtime

`binary64.c` implements 21 context-first raw-bit helper entry points declared in
`binary32.h`. Binary64 arguments/results use unsigned long long raw bits; integer
arguments/results preserve their declared signed/unsigned width. Arithmetic
supports add, subtract, multiply and divide; comparisons support quiet equality
and signaling less-than/less-or-equal. Width conversion supports binary32↔64.
Integer conversions cover binary32↔signed/unsigned64 and binary64↔signed/unsigned
32/64. Float-to-integer conversion uses minMag (toward zero), raises inexact for
fraction loss, and uses ARM-VFPv2 invalid-result saturation/NaN conventions.
C out-of-range casts retain their undefined-behavior boundary.

The runtime inherits SoftFloat 3e arithmetic, normalization, rounding, NaN and
conversion code. Original files/notices and SHA256 hashes are in `upstream/`.
`BINARY64_CLOSURE.json` records the closure and public symbol mapping;
`adapt-binary64.py` verifies the mechanical adaptations. These replace aggregate
float wrappers with raw bits, thread explicit context through every reachable
state access, preserve exact integer conversions with a fixed exact=true policy,
and change normalization struct returns into output pointers. Arithmetic itself
is preserved. No host FP execution or newly invented IEEE arithmetic supplies
runtime results.

Configuration: task-selected rounding (initial nearest ties-even); gradual underflow; tininess after rounding;
ARM-VFPv2 signaling/quiet NaNs, payload propagation, default NaN and invalid
conversion results. Internal globals are limited to immutable lookup tables.
Each context belongs to its task; concurrent sharing requires synchronization.
The normal C integer ABI transports binary64/I64 values on SIA. This replaces the
earlier provisional low/high/output-pointer proposal for these tested C helpers,
but does not freeze a C/Forge/Rust ABI or establish cross-language FP interop.

Multiplication uses the upstream non-FAST_INT64 four-word mul64To128M product.
Division uses upstream approximate reciprocal refinement and exact integer
remainder correction. Their I64 operations execute through real SIA integer
lowering/runtime. Neither requires an FPU. Shared pure binary32 normalization/
rounding helpers are TU-local copies needed by width/I64 conversions; compile
binary64.c independently alongside context.c and the binary32 modules.

## Recorded validation

- Unchanged SoftFloat3e ARM-VFPv2 integer oracle: 2,105,005 output-bit and fresh
  exception-flag comparisons across all 21 helpers, including 100,000 seeded
  random cases per helper, edge-pattern pairs, integer boundaries and transparent
  manual anchors. No mismatch. Reference platform uses generic C integer
  primitives, without host FP or intrinsic/int128 substitution.
- LightingMachine architectural guest: 1,169 reference vectors across all 21
  helpers plus independent-context and sticky-flag checks. Zero return, restored
  stack and no trap, after 1,176,594 guest instructions. A 10-million budget was
  provided; the completed run used fewer than two million instructions.
- GCC integer fixture and Cosmic SIA object compilation both pass.
- Original/adapted provenance verification passes for both 32- and 64-bit closures.

Durable JSON, guest source/vector/runner hashes, exact oracle commands, source
hashes and logs: `results/2026-10-05-binary64-*`. Full temporary logs and integer
oracle are under `/tmp/cosmic-softfloat-binary64-final*`.

Reproduce with a fresh output directory:

```sh
python3 runtime/softfloat/adapt-binary64.py
python3 runtime/softfloat/check-binary64.py --random-cases 100000 \
  --output /tmp/cosmic-sf64-new
cc -E -P runtime/softfloat/binary64-test.c -o /tmp/cosmic-sf64-guest.c
cargo run --manifest-path tools/l21-execution/Cargo.toml \
  --bin sia-source-native -- /tmp/cosmic-sf64-guest.c --max-instructions 10000000
```

The raw runtime tests establish this bounded helper slice. Compiler lowering of
actual float/double C expressions and composed-board execution have
[separate integration evidence](../../SIA_FLOAT_LANGUAGE_STATUS.md). This is not full TestFloat coverage, IEEE/C
conformance certification, libm, decimal conversion, long double, or a hosted
fenv contract. Other arithmetic such as remainder/square root remains outside
this runtime slice. [Upstream documentation](https://www.jhauser.us/arithmetic/SoftFloat-3/doc/SoftFloat-source.html)
and the [adaptation notes](ADAPTATION.md) describe the inherited implementation.

The current eight-byte context and dynamic rounding extension have
[separate four-mode validation and math documentation](FENV_MATH_STATUS.md).
The original results above remain evidence for their recorded source snapshots.
