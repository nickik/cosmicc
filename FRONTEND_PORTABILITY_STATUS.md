# Frontend portability slice — 2026-10-04

Implemented outer incomplete-array completion before declaration publication.
The inferred bound comes from normalized HIR aggregate entries, including
brace elision and designated holes, or from the string literal byte count
including its terminator. Later `sizeof` now sees a complete type.

Compound assignment now declares its hidden pointer temporary without an
initializer and captures the lvalue address in a comma expression at the
actual evaluation point. Nested statement parsing preserves the parent
statement's pending temporaries. Previously a condition's declaration could
land inside its body, and an initializer hoisted outside a loop could capture
the address only once. Address-of lowering preserves pointer-to-lvalue Noop
nodes so `*pick() += 1` evaluates the pointer-returning call correctly.

Regression coverage compiles array designators, brace-elided multidimensional
arrays, strings, if/while/for conditions, short circuiting, and pointer-returning
calls. `tools/l21-execution/src/bin/frontend-native.rs` executes initialized
array sizeof (40 bytes total), repeated side-effecting lvalue address capture,
short-circuit suppression, and compound-assignment loop conditions/updates.
It passed on **LightingMachine architectural execution**, 791 guest instructions,
zero return, restored stack, and no architectural trap. No SoftwareCpuBoard /
MainboardFPGA execution claim is made for this slice. The runner also exposes
`--board`, but that mode was not run. Local sibling dependency patches apply.

Validation: `scripts/check-supported.sh` passed: 133 parser tests, 19
preprocessor tests, 143 lowering/image tests, five doctests (two ignored).
Release compiler build passed. Logs are `/tmp/frontend-supported.log`,
`/tmp/frontend-release.log`, and `/tmp/frontend-native.log`.

## Unmodified e2fsprogs 1.47.2 probe

All 107 `lib/ext2fs/*.c` translation units were probed independently, with the
existing target-owned headers and generated SIA32 ext2 type/config headers.
31 compiled; 76 failed. This includes optional programs/test files and is a
broad compilation inventory, not a selected filesystem dependency closure.
Results: `/tmp/frontend-ext2/all/summary.tsv`, per-file logs alongside it.

- `namei.c` now compiles, resolving the generated Register tmp failure.
- `block.c` terminates promptly at unsupported `imul.i64`; no timeout occurred
  with the five-second per-file probe budget in this run.
- `fileio.c` reaches unsupported `udiv.i64`.
- `openfs.c` still requires target `strtoul` and `strchr` declarations.

Remaining section 3 work: runtime/sysroot declarations and implementations,
additional valid frontend/preprocessor patterns, and a linked, selected ext2
read/write dependency closure. No ext2 application execution is established by
this inventory.

## Current frontend slice: object literals and valid C99 declarations (2026-10-05)

The published backend pin `951ced55b67cc3b2cc76a04b1105a43e8aaf688d`
was used without local dependency overrides. Parser source is frozen for the
integration build. This entry supersedes the initial matrix for these features.

Implemented and tested for both explicit ILP32/SIA32 and LP64/amd64 models:

- Scalar and aggregate compound literals, fixed and initializer-inferred arrays,
  file-scope static storage and block-scope storage with initialization at each
  evaluated occurrence. Nested member initialization and zero fill are covered.
- Anonymous struct/union storage and promoted member access with retained nested
  layout; duplicate promoted member names are diagnosed.
- Macro argument prescan before replacement, inherited recursive-macro inhibition,
  and macro-expanded `#line` with `__LINE__`/`__FILE__` mapping.
- Wide character constants with UTF-8 source characters, standard escapes, octal,
  hexadecimal and universal escapes; these have signed 32-bit `int` type on both
  targets. Invalid universal characters and overflowing values are diagnosed.
- Scoped typedef/ordinary-name disambiguation, including block and loop scopes,
  and C99 array parameter qualifiers/static bounds/prototype `[*]` constraints.
- Runtime VLA `sizeof` remains an expression during folding. This is semantic
  coverage; dynamic VLA allocation on amd64 remains a separate backend boundary.
- Compound assignment to a plain identifier avoids an unnecessary address
  temporary; complex lvalues retain one evaluation of their address.

Validation: `cargo test -p saltwater-parser --offline` passed 137 unit tests,
19 preprocessor tests, 10 target-model tests, and five doctests (two ignored).
Log: `/tmp/cosmic-frontend-next-parser-final.log`.

The maintained `frontend-native` fixture passed on **LightingMachine architectural
execution**, 29,789 guest instructions, return zero, restored stack, no trap.
Log: `/tmp/cosmic-frontend-next-lighting-final.log`. This run did not use the
composed SoftwareCpuBoard/MainboardFPGA path.

The same independent fixture and unmodified upstream c-testsuite cases `00098`,
`00129`, `00162` compiled, linked and executed with exact expected output at
amd64 `-O0`, `-O2`, and `-Os`: 12 successful runs. Evidence:
`/tmp/cosmic-frontend-final-native/summary.json`. These focused results do not
replace the full differential-suite denominator or establish ISO conformance.

Remaining bounds: promoted anonymous-member designators and non-first union
member initialization need further work; wide strings and incoming variadic C
function bodies remain unsupported. `#line` affects predefined macros while
source diagnostics retain physical locations. VLA compound literal types are
correctly rejected. The upstream macro-paste case `00141` reads uninitialized
variables, so the independent initialized macro fixture supplies the defined
behavior evidence for that fix.

### Final operator-type correction

The full matrix exposed that logical/comparison results were represented as
`_Bool` rather than C `int`, and shifts used the usual arithmetic conversions
rather than independent integer promotions. Both are corrected generally,
including compound shifts. Internal condition truth values retain `_Bool`.
Unmodified case `00178` motivated the first fix, but its printf format does not
match `sizeof`'s type; case `00200` also includes undefined negative left shifts.
The maintained independent tests therefore use compile-time type assertions and
positive unsigned right shifts with a 64-bit count to provide defined evidence.

Published-pin parser gates passed: 137 unit, 19 preprocessor, 12 target-model
and five doctests (two ignored), `/tmp/cosmic-c-operator-types-parser.log`.
The expanded LightingMachine fixture passed 35,100 instructions with zero return,
restored stack and no trap: `/tmp/cosmic-c-operator-types-lighting.log`.
This supersedes the 29,789-instruction fixture result above. No composed board
execution was run for this slice. Final native matrix evidence is supplied by
its separately captured compiler snapshot.

### Binary32 integration follow-up

The frontend now preserves `f`/`F` literal types, parses decimal and hexadecimal
binary32 literals directly at that precision, and rounds float-typed folded
operations/casts to binary32. Long-double literal spelling is explicitly
rejected rather than silently becoming double. Both target models have type
and rounding regressions. Parser gates now pass 140 unit, 19 preprocessor,
13 target-model tests and five doctests (two ignored).

SIA binary32 software transport and actual C execution are detailed in
[SIA_FLOAT_LANGUAGE_STATUS.md](SIA_FLOAT_LANGUAGE_STATUS.md): 32,419 guest
instructions with three explicitly linked runtime objects. This supersedes
historical statements that every SIA floating value is rejected; binary64 and
float/I64 conversion support remain bounded diagnostics.

### Binary64 follow-up

C double operations, binary32/binary64 width casts, and signed/unsigned I64
floating conversions now use integer-only SIA runtime helpers. The current
scope and actual C execution evidence are in
[SIA_FLOAT_LANGUAGE_STATUS.md](SIA_FLOAT_LANGUAGE_STATUS.md), superseding the
previous binary64/I64 diagnostic boundary. Long-double declarations are now
explicitly rejected on SIA as well as AMD64; the former silent double alias
would otherwise become observable wrong layout/precision after this change.

### Current literal, macro-rescan and array-initializer slice

This supersedes earlier long-double rejection statements: SIA now has a distinct
`long double` type with the documented binary64 precision/layout policy, while
AMD64 long-double layout/ABI stays explicitly unsupported. Runtime fenv details
are recorded in SIA_FLOAT_LANGUAGE_STATUS.md.

The shared frontend now chooses hexadecimal/octal integer literal candidate
types according to each target width, including unsigned values above signed
64-bit maximum. Hexadecimal binary32/binary64 floating literals use direct IEEE
round-to-nearest/even conversion, preserving exact subnormals (`0x1p-149f`,
`0x1p-1074`), normal boundaries and ties. Nonzero literals rounding to zero still
receive the existing underflow diagnostic. Zero mantissas with nonzero exponents
are accepted. `__builtin_inf[f/l]()` and `__builtin_nan[f/l]("")` supply typed
constant literals without generating arithmetic or changing fenv flags;
nonempty NaN payload strings remain unsupported.

Function macro replacement now computes the invocation hide set using the
closing parenthesis, allowing pasted function names to rescan correctly.
Character/wide arrays consume string initializers as whole subobjects through
brace elision. An exact bound may omit the terminating null; excess characters
and incompatible element widths remain diagnosed. Array operands decay before
`->` member selection.

Wide strings now preserve UTF-8 source scalars and standard, octal, hexadecimal
and universal escapes as signed 32-bit execution code units on both targets.
Adjacent ordinary/wide strings concatenate into a wide result. Array inference
counts code units instead of bytes. The new wchar.h exposes type/bound/basic
function declarations; it provides no wide libc implementations. Native and
SIA string pools align to four bytes; global/local string array copies retain
member offsets and respect object bounds.

Focused gates: 141 parser unit, 19 preprocessor, 21 target-model and 5 doctests
passed (two ignored) in `/tmp/cosmic-wide-parser-gate.log`. The independent
`tools/sia/frontend-literals-arrays.c` passed GCC13 and Clang18 references and
Cosmic AMD64 compilation/system linking/execution. Its actual LightingMachine
execution passed in **1,000 guest instructions**, with return zero, restored
stack and no trap, in `/tmp/cosmic-frontend-literals-lighting.log`; no composed
production-board execution was performed for this slice. Source SHA256:
`6fba389293f7e455e5b6056a544cd0a55c95ac9d5c1c7f4b48604daabc27d2c8`.

Unchanged pinned upstream cases 00104, 00201, 00205 and 00220 now compile, link
and match exact expected output on AMD64 (`/tmp/cosmic-frontend-five-native`).
Case 00204 now passes the earlier string-initializer obstacle and reaches the
explicit unsupported AMD64 long-double declarations; full acceptance is not
claimed. The independent fixture isolates its valid string initialization
behavior from that target ABI limitation.

## C11 generic selection — 2026-10-05

`_Generic` is implemented on SIA and AMD64. Control expressions receive
lvalue/array/function conversions without integer promotion; top-level
qualifiers are removed, while pointer-pointee qualifiers remain significant.
Only the selected expression reaches executable lowering, preserving its
lvalue/function category. All associations still receive semantic checking.
Duplicate compatible types/defaults, missing matches and incomplete or
variably modified association types are diagnosed. Nominal structure identity
is checked for generic compatibility; this does not replace global structural
equality throughout the older frontend.

Plain char, signed char and unsigned char now have distinct type identities
with existing byte ABI/layout. Ordinary character literals have C int type on
both targets. Three focused acceptance/diagnostic tests run against both target
models; the supported workspace gate passes. The no-I/O language fixture
executes unchanged on Lighting and production CPU/mainboard RTL in 595 guest
instructions. [Retained evidence](tools/sia/results/2026-10-05-generic/summary.json).
