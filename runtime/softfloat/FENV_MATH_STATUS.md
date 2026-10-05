# SIA software environment and math runtime

Context ABI version 2: the environment is now `struct { unsigned int flags; unsigned int rounding; }`
(eight guest bytes). Initialize it with `cosmic_sf_init` before use. Mode values
match SoftFloat: nearest ties-even 0, toward zero 1, toward negative infinity 2,
toward positive infinity 3. Arithmetic and integer-to-float/format conversions
read the supplied context's mode; C float-to-integer casts still truncate toward
zero independently of it. Tininess remains after rounding, with gradual
underflow and the existing ARM-VFPv2 NaN specialization.

`cosmic_sf_current_context()` supplies the compiler's implicit environment.
`cosmic_sf_bind_context(next)` returns the previous environment; null selects a
zero-initialized boot environment. A scheduler must bind the next task's guest
context before executing that task and restore the previous binding on return.
This hook is a single-machine binding, not TLS, an automatic scheduler hook, or
safe concurrent sharing. Each independently executing machine needs its own
runtime instance; callers must serialize shared context access.

`fenv.c` implements clear/get/raise/set/test exception flags, get/set rounding,
get/set/hold/update environment. Exceptions are sticky and nontrapping;
`feholdexcept` saves state and clears flags, while `feupdateenv` restores the
saved environment and merges raised flags. Unknown rounding values return
nonzero without changing state. `FE_DFL_ENV` restores nearest and clear flags.
`FLT_ROUNDS` uses `cosmic_sf_flt_rounds`, mapping the runtime mode to the C macro's
required numbering. No host fenv callback is used by guest code.

## Core math slice

- `sqrtf/sqrt/sqrtl` use licensed unchanged SoftFloat square-root algorithms,
  adapted only for raw-bit parameters/results and explicit context. Reciprocal
  square-root tables/refinement are preserved under `upstream/` and `adapted/`.
- `fmod`, `floor`, `ceil`, `trunc`, `round`, `fabs`, `copysign`, `frexp`, `scalbn`
  and `ldexp` have float/double/long-double wrappers. Remainder uses exact integer
  significand subtraction; integral rounding uses bit masks; scale uses the
  existing upstream round-pack and its exception policy. `floor/ceil/trunc/round`
  do not raise inexact for ordinary finite inputs. Signaling NaNs raise invalid
  where arithmetic semantics require it; fabs/copysign are pure bit operations.
- Classification macros evaluate their operand once and distinguish NaN,
  infinity, zero, subnormal and normal. They do not raise exception flags.
- SIA long double has the compiler's documented binary64 representation and
  precision. This does not supply an extended-precision format.

## Transcendental slice

Official [Netlib fdlibm](https://netlib.org/fdlibm/) sources supply exp, log,
log10, pow, sin, cos, tan, asin, acos, atan, atan2, sinh, cosh, tanh, expm1, log1p,
cbrt, hypot, erf, erfc, lgamma, acosh, asinh, atanh and remainder. Original notices and files are retained in `fdlibm/upstream/`
with `fdlibm/SHA256.json`. `fdlibm/adapt.py` namespaces public functions, fixes
the little-endian 32-bit word interface, expresses signed exponent/word shifts
as unsigned modulo operations, and appends unreachable terminal returns to three
exhaustive switches for the current SIA control-flow checker. Constants,
polynomials and large-argument trigonometric reduction are retained.

These C algorithms execute through compiler software-FP lowering. Target code
never calls the host FPU or host libm. The fdlibm word interface relies on the
SIA compiler's memory reinterpretation support; host reference compilation uses
`-fno-strict-aliasing`. Standard wrappers provide float via binary64 computation
and narrowing, and long double via binary64 aliases. Float results are not
claimed correctly rounded: wider evaluation followed by narrowing can double
round. Transcendental accuracy is inherited from fdlibm, not certified for all
inputs/rounding modes. Directed-rounding operations use the task environment,
but fdlibm's approximation bounds were designed around nearest rounding.

## Recorded checks

`results/2026-10-05-environment-reference.json` records **1,168,968** exact
output-bit and fresh exception-flag comparisons against unchanged integer
SoftFloat3e, covering four modes, all prior binary32/binary64 arithmetic and
conversion helpers, and square root. It generated **1,884** guest vectors.
The reference process alone uses SoftFloat's global state, reset each call;
guest implementations use explicit contexts.

`results/2026-10-05-math-bits-reference.json` records **724,992** finite-input
output-bit comparisons against an independent host libm oracle in four modes
for nine operations in both formats, including signed zero and subnormals.
Host libm appears only in test code. Its flags are not used to certify the
bit-based helpers because C permits different inexact behavior for integral
rounding. Saved reference output generated **1,192** vectors.

`results/2026-10-05-fdlibm-reference.json` records **324** vectors generated from
original untouched fdlibm sources under nearest host binary64 evaluation,
covering 18 functions, edge values, huge trigonometric inputs and seeded random
raw inputs. `fdlibm/test.c` compares guest finite outputs exactly, NaNs by class,
and invalid/divide-by-zero/overflow/underflow flags; inexact is diagnostic because
conversion execution can differ. Reference generation alone is not proof of
guest execution. Guest integration results are recorded separately by the
[compiler integration gate](../../SIA_FLOAT_LANGUAGE_STATUS.md).

All 34 adapted fdlibm TUs and the additional C99 wrapper TU passed actual
Cosmic SIA object compilation. Standard wrapper and guest execution acceptance
must use the compiler's current long-double and persistent-environment lowering.

## Reproduction

```sh
python3 runtime/softfloat/adapt-sqrt.py
python3 runtime/softfloat/check-environment.py --output /tmp/sf-environment-new
python3 runtime/softfloat/check-math-bits.py --output /tmp/sf-math-new
python3 runtime/softfloat/fdlibm/generate-reference.py --output /tmp/fdlibm-new
```

Compile `environment-test.c`, `environment-vectors-test.c`, or `fdlibm/test.c`
with context.c, fenv.c, binary32-add.c, binary32-convert.c, binary64.c,
math-bits.c and math.c. Transcendental fixtures also require
math-transcendental.c, math-extra.c and all `fdlibm/adapted/*.c` translation units.
The C99 real `<math.h>` function families now have implementations and scalar
float/double/binary64-alias long-double interfaces. This does not add complex
math, POSIX Bessel functions, errno/SVID wrappers, trapping
exceptions, or IEEE conformance certification. No missing function is replaced
by a placeholder result.

## Additional C99 functions and accuracy boundaries

`math-extra.c` adds rint/nearbyint, lrint/llrint/lround/llround, remainder/remquo,
nextafter/nexttoward, ilogb/logb, modf, exp2/log2, erf/erfc, tgamma/lgamma,
fmin/fmax/fdim, scalbln and nan. Inverse hyperbolic functions use licensed
fdlibm. Adjacent-value stepping, quotient low bits and integer rounding are
integer-only; remquo returns seven quotient bits with the required sign.
The float nexttoward helper compares against the full binary64 destination
before stepping, preserving direction when narrowing the destination would
erase it. `nearbyint` preserves preexisting inexact and does not raise a new
inexact; signaling NaNs can still raise invalid. `nan` chooses the default
quiet NaN independently of the optional implementation-defined payload string.
Quiet ordered/unordered comparison macros evaluate arguments once and avoid
ordinary ordered comparisons on NaNs.

`exp2` uses exact scalbn for integer powers and fdlibm exp for the reduced
fraction. `log2` recognizes exact powers of two and otherwise uses fdlibm log
and inverse ln2. `tgamma` uses signed exp of fdlibm lgamma, with explicit pole
handling and direction reversal when rounding a negative magnitude. These
composite algorithms can accumulate extra approximation/rounding error and are
not claimed universally correctly rounded. Gamma cancellation near poles and
large arguments remains a particular accuracy boundary. Runtime math reports
exceptions (`math_errhandling == MATH_ERREXCEPT`) without an errno facility.
HUGE_VAL/INFINITY/NAN use compiler constant builtins and raise no flags.

Fused multiply-add is not a separated multiply and add: the preserved licensed
SoftFloat algorithm keeps the full product before a single final rounding.
Binary64 uses the upstream four-word integer primitive closure. `adapt-fma.py`
verifies the mechanical adaptation against original hashes.
`results/2026-10-05-fma-reference.json` records **182,088** FMA/rint cases in four
modes, repeated with fresh and sticky contexts (**364,176** bit-and-flag checks),
including cancellation, overflow cancellation, and subnormal anchors. It
generated **240** guest vectors, tested in both context states by `fma-test.c`.

The expanded bit oracle records **888,576** finite-input comparisons across
eleven operations, both formats and four modes, including remquo output and
quotient low bits/sign and nextafter. **1,432** guest vectors are in
`math-vectors-test.c`. Its stored flags check integer-C vs guest consistency;
independent host-libm certification is for output bits and exponent/quotient,
with exception semantics additionally exercised by targeted C fixtures.
The expanded unchanged fdlibm reference supplies **450** vectors across 25
functions; earlier 324-vector guest evidence remains a separate snapshot.
`c99-math-test.c` exercises every function family, long-double aliases, constant
macros, dynamic rounding, exact fused anchors and mandatory exception flags.
Guest completion is reported by the integration gate, not inferred from
reference generation or object compilation.

## Completed guest and production execution

The frozen runtime passed actual LightingMachine execution with reachable-section
collection and explicit linked runtime objects:

| Fixture | Coverage | Guest instructions |
| --- | --- | ---: |
| `environment-vectors-test.c` | 1,884 all-mode integer SoftFloat bit/flag vectors | 1,891,004 |
| `fma-test.c` | 240 vectors in fresh and sticky contexts; 480 checks | 542,747 |
| `math-vectors-test.c` | 1,432 output/exponent/quotient vectors | 2,172,252 |
| `fdlibm/test.c` | 450 unchanged-reference vectors over 25 functions | 13,296,432 |
| `c99-math-test.c` | C99 function families, scalar aliases and mandatory flags | 554,577 |

The first three logs and source/runner hashes are retained in `results/` as
`2026-10-05-{environment,fma,math}-lighting*`. The root integration run retains
[expanded fdlibm](../../tools/sia/results/2026-10-05-runtime/fdlibm-expanded-architectural.log),
[C99 language fixture](../../tools/sia/results/2026-10-05-runtime/c99-math-architectural.log),
and the [combined summary](../../tools/sia/results/2026-10-05-runtime/summary.json).

The unchanged standard environment fixture also passed the production RTL board
path: **33,779 instructions**, **265,601 cycles**, **38,179 transactions**.
The [production log](../../tools/sia/results/2026-10-05-runtime/environment-production.log)
records that separate physical-model integration gate. These results establish
the recorded inputs and source snapshot; they do not remove the accuracy and
coverage limits stated above.
