# Cosmic C frontend capability matrix

> Historical initial audit. The lowering/execution boundaries in this table
> are superseded by [LOWERING_TODO.md](LOWERING_TODO.md) and the active
> [PORTABILITY_TODO.md](PORTABILITY_TODO.md). Do not use it as current status.

This matrix records frontend evidence separately from SIA lowering evidence.
“Supported” means the parser/preprocessor has a deterministic test in the
maintained workspace. It does not mean that the construct can be lowered to
COSMIC-SIA.

| Area | Construct | Classification | Evidence / boundary |
| --- | --- | --- | --- |
| Lexical | identifiers, keywords, integer/character/string literals, operators, comments | Supported | Parser and lexer unit tests; integer literals only reach the current SIA path. |
| Lexical | floating-point literals | Explicitly rejected for SIA | `saltwater-sia` rejects them before backend lowering. |
| Preprocessor | object-like macros | Supported | Deterministic fixture and parser tests. |
| Preprocessor | function-like macros | Supported | Deterministic fixture and parser tests. Variadic macros remain deferred. |
| Preprocessor | `#if`, `#elif`, `#else`, `#ifdef`, `#ifndef`, `defined` | Supported | Integer-expression and macro-selection tests. |
| Preprocessor | `#define` / `#undef` command-line definitions | Supported at parser API | `Opt.definitions` and source `#undef` are tested; CLI flags remain a driver task. |
| Preprocessor | quoted local `#include` | Supported | Checked-in fixture proves filename-relative lookup. |
| Preprocessor | system include search, `#pragma once`, `#line`, `#warning` | Parser-only / compatibility | Existing parser tests; no Cosmic sysroot contract yet. |
| Preprocessor | token pasting, stringification, variadic macros | Broken or deferred | No Cosmic profile evidence; must remain out of the supported claim. |
| Declarations | `char`, `short`, `int`, `long`, signedness, `_Bool`, pointers | Parser-only or partially lowerable | Parser accepts more than the current SIA lowering supports. |
| Declarations | structs, unions, enums, typedefs, qualifiers, storage classes | Parser-only / not yet lowerable | Frontend infrastructure exists; layout/ABI and object lowering are deferred. |
| Statements | compound blocks, declarations, expression statements, explicit `return` | Partially supported | Simple integer functions are covered by SIA tests. |
| Statements | `if`, loops, `switch`, labels, `goto`, `break`, `continue` | Parsed but not lowerable | M3 is blocked by SIA32 comparison/boolean lowering and control-flow work. |
| Expressions | integer literals, locals, assignment, `+ - & | ^ << >>`, unary integer operations | Supported in current SIA profile | `saltwater-sia` CLIF and emitted-byte tests. |
| Expressions | comparisons, logical operators, conditional expressions, calls | Parsed or semantically represented; not lowerable | No direct SIA fallback; comparisons await Cranelift `icmp`. |
| Initializers | scalar initialized SSA locals | Supported in current SIA profile | Existing SIA tests. |
| Initializers | aggregates, strings, compound literals, static constants | Parser-only / not yet lowerable | Requires memory, global data, and object-format work. |
| Target boundary | float/double, I64/`long long`, varargs, TLS, atomics | Explicitly deferred/rejected | No support or fallback is permitted. |
| Target boundary | globals, address-taken locals, loads/stores, objects/linking | Explicitly deferred | Requires M3.3/M5 contracts. |
| Execution | COSMIC-SIA bytes and `CallPlan` decoding | Supported as encoding evidence | Does not prove execution. |
| Execution | real Lighting board execution | Externally blocked | Waiting for Lighting’s public board-call runner. |

The matrix is intentionally conservative: a parser result is not a compiler
claim until the Cranelift SIA32 path and its tests prove the generated form.

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

### Current literal and initializer follow-up

Supersedes historical wide-string and SIA long-double unsupported entries:
SIA `long double` is a distinct type with binary64 precision/layout; AMD64 still
rejects its unsupported ABI. Shared target-aware nondecimal literal typing,
exact hexadecimal subnormal rounding, typed Inf/empty-payload NaN builtin
constants, pasted function-macro rescan, whole string-array brace initialization,
array decay before `->`, and UTF32 wide strings are covered by fresh focused
regressions. Adjacent narrow/wide strings concatenate to a wide result; inferred
bounds count elements. wchar.h declarations do not imply wide libc
implementations. Exact details, upstream cases and architectural execution
proof are in FRONTEND_PORTABILITY_STATUS.md. This is a bounded compatibility
slice, not a C conformance certification.

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
