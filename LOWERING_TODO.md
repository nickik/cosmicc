# Cosmic C SIA32 Lowering TODO

This checklist tracks lowering work that remains after the ext2-compile-compat merge. Each item is intended to be implemented and validated as an individual coherent commit.

## Global and static data

- [x] **L1 — Global data definitions:** lower non-function top-level object definitions into COSMIC-SIA data objects.
- [x] **L2 — Global references:** lower reads and writes of global variables.
- [x] **L3 — Global addresses:** lower `&global` and code references to global symbols with relocations.
- [x] **L4 — Static locals:** allocate function-local `static` objects in static storage and reference them from functions.
- [x] **L5 — String literals:** emit string literals as read-only data objects and produce address relocations.
- [x] **L6 — Scalar global initializers:** serialize integer, enum, pointer/null and other supported scalar initializers.
- [x] **L7 — Aggregate globals:** serialize global arrays, structs and unions, including nested initializers.
- [x] **L8 — Data relocations:** support pointers/function addresses embedded in data and data-to-data/data-to-code symbol relocations.
- [x] **L9 — External calls:** emit unresolved external function symbols/relocations for declared-but-not-defined callees.

## Floating point

- [x] **L10 — `float` type:** represent and lower C `float` values.
- [x] **L11 — `double` type:** represent and lower C `double` values.
- [x] **L12 — FP arithmetic:** lower floating `+`, `-`, `*`, and `/` plus unary negation to CLIF. Frontend/CLIF lowering is complete; native SIA32 emission remains blocked by the Cranelift SIA32 backend rejecting `f32`/`f64` SSA values.
- [x] **L13 — FP comparisons:** lower floating equality and ordered comparisons to C integer booleans via CLIF `fcmp`. Frontend/CLIF lowering is complete; native SIA32 emission remains blocked by the backend's lack of `f32`/`f64` SSA support.
- [x] **L14 — FP conversions:** lower signed/unsigned integer ↔ float/double plus float ↔ double conversions to CLIF. Native SIA32 emission remains blocked by the backend's lack of `f32`/`f64` SSA support.
- [x] **L15 — FP ABI:** lower `float`/`double` parameters, returns, direct-call signatures, and indirect-call signatures to CLIF `f32`/`f64`. Native SIA32 ABI emission remains blocked by the backend's lack of FP SSA/register support.

## Aggregate values and arrays

- [x] **L16 — Aggregate value copy:** lower whole struct/union assignment such as `a = b`; copy the complete object representation byte-wise so padding, odd sizes, and conservative alignment are handled without scalar-width assumptions.
- [x] **L17 — Aggregate returns:** lower struct/union returns by value through the SIA32 hidden return-address ABI, including local, nested-call, union, and odd-sized aggregate results.
- [x] **L18 — Aggregate arguments:** lower structs/unions passed by value through caller-owned ABI copy slots and callee-local copies, preserving C by-value isolation for struct/union parameters including odd-sized objects.
- [x] **L19 — Aggregate scalar/brace-elided initialization:** audited the frontend-normalized recursive initializer path and cover brace-elided nested struct/array forms, recursively lower scalar sub-braces inside aggregate elements, and cover partial nested initialization and zero-fill behavior.
- [x] **L20 — Designated aggregate initialization:** AST/parser support member/index designators and nested paths; analyzer normalizes them into positional HIR with explicit sparse `Initializer::Zero` gaps, continuation resumes after the designated subobject, and repeated/backward designators replace prior values with last-initializer-wins semantics. Local and global SIA32 lowering regressions cover sparse arrays and struct member designators.
- [x] **L21 — Non-fixed arrays:** frontend and SIA32 backend now preserve and lower integral runtime bound expressions, including identifier and arithmetic forms such as `int a[n]` and `int a[n + 3]`, through aligned `stack_alloc_dynamic` storage. `sizeof(VLA)` lowering is now implemented for direct variable-length array types by evaluating the preserved runtime bound and multiplying by the element size. Multidimensional VLA addressing now covers runtime outer bounds with fixed inner dimensions and adds runtime inner stride lowering for `int a[n][m]` pointer arithmetic. Nested VLA allocation now recursively computes runtime total sizes for fully runtime shapes such as `int a[n][m]`; runtime inner pointer strides are preserved by the analyzer. Scope-lifetime reclamation is now implemented for normally exited compound scopes: Cosmic C tracks dynamic allocation sizes and emits generic `stack_free_dynamic`, with SIA32 lowering restoring r13 by the matching byte count. Return-path cleanup is now implemented by releasing all outstanding dynamic VLA allocations before scalar, void, and aggregate returns. `break` and `continue` now release dynamic VLA allocations from every scope exited by the transfer before branching to their loop/switch targets. `goto` now records lexical VLA depth for labels, releases allocations when transferring out of VLA scopes, preserves allocations for same-depth transfers, and rejects transfers into scopes with active VLAs. Non-local cleanup lowering and local execution validation are complete. `scripts/check-l21-execution.sh` passes 15 VLA cases through the real SoftwareCpuBoard → MainboardFPGA → RAM path, checking return values, intermediate allocation-address reuse, outer-VLA lifetime, and exact final r13 restoration for normal exit, return, break, continue, and goto. It also passes 48 comparison cases, bounded-failure diagnostics, and rejection of goto into an active VLA scope. Local completion checks passed on 2026-10-04 (formatting, workspace tests, and build); the ext2 probe boundary is recorded below. Coordinated backend and Lighting fixes are landed on remote main and exact Git pins replace all local overrides; clean-checkout validation is recorded below. See `tools/l21-execution/README.md`. Incomplete `T a[]` remains distinct.

## Complete partial lowering families

- [x] **L22 — Cast completion:** audited frontend `ExprType::Cast`; lower integer width/signedness, pointer/integer and function-designator address casts, aggregate-address decay shapes, FP conversions, and cast-to-void while preserving operand side effects. Invalid non-scalar/float-pointer/void-source casts remain frontend diagnostics.
- [x] **L23 — Assignment completion:** audited assignable HIR shapes; direct locals/globals retain optimized handling, aggregate-by-value assignment uses object-copy lowering, and all remaining scalar dereference/member/index/wrapped lvalues lower through the common `compile_lvalue_address` path with assignment values preserved for chaining.
- [x] **L24 — Increment/decrement completion:** audited post-`++`/`--` HIR lowering; direct SSA locals retain the fast path while globals, stack/address-taken locals, dereferences, members, array/index pointer arithmetic, and wrapped legal lvalues store through the common lvalue-address path with declared-width stores and pointer-size scaling.
- [x] **L25 — Address-of completion:** audited `StaticRef`/address HIR; globals, statics, functions, locals, nested members, array/index lvalues and wrapped legal lvalues use the common address path, while dereference operands preserve the C identity `&*p == p` without a load.
- [x] **L26 — Aggregate initializer completion:** audited local/global array/struct/union recursion, brace elision, scalar sub-braces, partial/zero-filled objects, unions, and excess-element diagnostics. Designated forms are implemented by L20: the analyzer normalizes member/index designators into positional HIR, including sparse zero-fill and last-initializer-wins behavior.
- [x] **L27 — Control-flow completion:** exhaustively enumerate every current `StmtType` variant and audit lowering for compound/decl/expr/return, if, while/do/for, switch/case/default/fallthrough, goto/labels, break and continue. No generic legal-statement fallback remains; an exhaustive statement-kind guard forces future HIR variants to be handled explicitly.
- [x] **L28 — Binary-operator completion:** exhaustively audit every current `BinaryOp`; cover integer arithmetic/div/rem/bitwise/shifts/comparisons with promotions and signedness, short-circuit logical operators, assignment, pointer add/sub/difference/comparisons, and FP arithmetic/comparisons. An exhaustive enum regression forces future operators to be classified explicitly. Pointer arithmetic HIR now preserves the integer index domain instead of casting the index to pointer type before scaling, and pointer-pointer subtraction is typed as a signed integer (`long`/ptrdiff representation) rather than incorrectly retaining pointer type.
- [x] **L29 — Type lowering completion:** `ir_type()` now exhaustively enumerates every frontend `Type` variant: integer/enum/pointer/function scalars map explicitly, FP maps to CLIF FP types, arrays/structs/unions are explicitly address-only/aggregate-ABI types, and void/va_list/error receive deliberate diagnostics. No wildcard/generic type-lowering fallback remains.

## Validation / completion criteria

L1–L9 were reconciled after the completion pass: their implementations and focused regressions are already present on `master` (global/static data objects, symbol references/addresses, strings, initializers, data relocations, and unresolved external calls).

Each lowering item should land as its own commit with focused regression tests. Before marking an item complete:

- [x] `cargo fmt --all -- --check`
- [x] `cargo test --workspace`
- [x] `cargo build`
- [x] Add positive tests for the new lowering.
- [x] Add negative/diagnostic tests where the C construct is invalid or intentionally unsupported.
- [x] Re-run the e2fsprogs/ext2 compile probe and record the next concrete boundary.
- [x] Avoid weakening existing float rejection until the floating-point slices themselves are implemented.
- [x] Avoid host-only fallback lowering; generated code must remain on the C → HIR → CLIF → Cranelift SIA32 → COSMIC-SIA path.

## L21 local completion evidence — 2026-10-04

`cargo fmt --all -- --check`, `cargo test --workspace`, and `cargo build`
passed. Workspace tests: 131 parser unit tests, 15 preprocessor profile tests,
132 lowering tests, and 5 doctests passed (2 existing doctests ignored).
The native gate passed all 15 VLA cases, 48 comparison cases, bounded execution
and architectural-fault diagnostics, and rejection of a goto into an active VLA
scope. Nested VLAs and repeated loop allocations verify intermediate address
reuse as well as final stack restoration. Float rejection is retained; execution
uses the real SIA32/mainboard path.

The historical ext2 probe source/configuration was unavailable locally, so the
probe was reconstructed against the unmodified upstream e2fsprogs **1.47.2**
release. Archive SHA-256:
`08242e64ca0e8194d9c1caad49762b19209a06318199b63ce74ae4ef2d74e63c`.
Reproduce after `cargo build` with:

```sh
sh scripts/probe-ext2.sh /path/to/e2fsprogs-1.47.2 /tmp/cosmicc-ext2-probe
cat /tmp/cosmicc-ext2-probe/compile.log
```

The script requires Python 3 and `compile_et`; it generates SIA32-width type
headers and a minimal target configuration, without host libc headers or host
code execution. This is a translation-unit compatibility probe, not a complete
ext2 library build or validated OS port.

Next actual boundary: `lib/ext2fs/alloc.c:36`, GNU named variadic macro
`# define dbg_printf(f, a...)`. Compilation exits 2 with `invalid macro: missing
',' or ')' in macro parameter list`, followed by `expected ',' or ')', got ...`,
`expected identifier or ')', got )`, and cascading undeclared `dbg_printf`
diagnostics. No ext2 source rewrite was used to bypass this failure.

- [x] Land coordinated backend/Lighting changes, replace local Cargo overrides
  with exact merged Git pins, and update the supported CI gate to workspace tests
  and compiler build. Native validation requires Bluespec and repository read access.

## L21 integration acceptance — 2026-10-04

Backend fixes landed on remote `crainlift/main` at
`8ecc42343f0f467b5d0a445ad97f9c4a77590b3f`; Lighting fixes landed on remote
`LightingSimulation/main` at `93a565c687f61d4e758af81bbde59ce5c4770996`.
Cosmic C pins those revisions directly, with no local Cargo patches or sibling
paths. The parser target lexicon uses the same backend revision.

A fresh independent clone at `/tmp/cosmicc-l21-clean` passed
`cargo fmt --all -- --check`, `cargo test --workspace`, and `cargo build`.
`BSC=/tmp/cosmicc-l21-bsc/bin/bsc sh scripts/check-l21-execution.sh` built its
own MainboardFPGA Bluesim bridge from the Git-pinned Lighting/hardware sources
and passed all 15 VLA cases, 48 comparisons, bounded/fault diagnostics, and
invalid-scope-entry rejection. Git status remained clean after validation.
Dependency sources were Cargo Git checkouts, not the sibling workspace repos.
Backend focused tests passed: 39 unit, 8 encoding, 4 integration, and 13
production tests. Lighting's three SoftwareCpuBoard regressions passed.
The clean compiler reproduces the same ext2 GNU variadic macro boundary
(exit status 2). Logs for this local session are `/tmp/l21-clean-tests.log`,
`/tmp/l21-clean-build.log`, and `/tmp/l21-clean-native.log`.

These are clean-checkout local acceptance results; remote CI has not been
verified. The supported CI script now runs workspace tests and compiler build.
The separate native gate requires BSC and read credentials for Lighting and its
hardware submodule; see the gate README for SSH fetch configuration.

## Native ext2 follow-up — 2026-10-04

GNU named variadic macros and standard __VA_ARGS__ expansion are implemented,
including comma preservation/elision and diagnostic coverage (19 preprocessor
profile tests). Target-owned fcntl and atexit declarations remove the next
header/declaration boundaries. e2fsprogs 1.47.2 alloc.c, alloc_sb.c,
alloc_stats.c, alloc_tables.c, and atexit.c now compile.

Native ABI preflight (`--bin ext2-abi` in the L21 execution tool) executes
compiler bytes through the real board and reports **__u64=4, long=4, pointer=4**,
then exits 2. The frontend maps long long to Type::Long (4 bytes), so ext2's
required 8-byte __u64 and disk structure layouts are not yet correct. Successful
source compilation must not be treated as ext2 execution acceptance. No disk
writes or native file operations have been performed.

Other observed boundaries include offsetof address lowering, const-qualified
pointer assignments, missing timeval/limits declarations, and a 30-second
compile timeout on block.c. The required target runtime, relocatable image
loader, existing file-backed QDX-B integration, and remaining acceptance steps
are tracked in `tools/ext2/README.md`. The filesystem integration remains open.

## Native ext2 continuation — 2026-10-04

The width mismatch is fixed: distinct LongLong types have eight-byte size and
alignment, signedness/rank/promotion support, and preserved LL/ULL literals.
Malformed suffix combinations are rejected. Native width preflight now reports
8/4/4. Eight-byte global initializer contents are covered by a regression.

Backend core `3cf7afcb6e771e0de539f5cb9bf413574a8f6963` is merged and pinned,
with no local Cargo overrides. Native constants, add/subtract carry/borrow,
bitwise operations, constant shifts across word boundaries, comparisons,
extension/reduction and load/store are implemented. The native prerequisite
gate passes 204 cases, including addressed 64-bit compound assignments and
signed narrow loads, plus the existing L21 cases and diagnostic gate.
Frontend HIR now preserves address-of explicitly; stack_load uses the correct
pointer/value argument order and assignment stores retain the original lvalue
width. Backend verifier diagnostics include the actual failing instructions.
Formatting, workspace tests, and build pass. Remote CI is not verified.

Ext2 alloc.c, alloc_sb.c, atexit.c, and badblocks.c compile with corrected widths.
The next allocation-module failures are imul.i64 in alloc_stats.c and udiv.i64
in alloc_tables.c. Other observed boundaries remain offsetof, assignments to
pointers-to-const, and missing strtoul/strchr declarations. File integration is
still incomplete: linking/runtime, full I64 operation coverage, QDX-B attachment,
native open/read/create/write/close, and independent image verification remain.
See `tools/ext2/README.md` and `scripts/check-ext2-native.sh`.
