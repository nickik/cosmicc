# Cosmic C SIA32 Lowering TODO

This checklist tracks lowering work that remains after the ext2-compile-compat merge. Each item is intended to be implemented and validated as an individual coherent commit.

## Global and static data

- [ ] **L1 — Global data definitions:** lower non-function top-level object definitions into COSMIC-SIA data objects.
- [ ] **L2 — Global references:** lower reads and writes of global variables.
- [ ] **L3 — Global addresses:** lower `&global` and code references to global symbols with relocations.
- [ ] **L4 — Static locals:** allocate function-local `static` objects in static storage and reference them from functions.
- [ ] **L5 — String literals:** emit string literals as read-only data objects and produce address relocations.
- [ ] **L6 — Scalar global initializers:** serialize integer, enum, pointer/null and other supported scalar initializers.
- [ ] **L7 — Aggregate globals:** serialize global arrays, structs and unions, including nested initializers.
- [ ] **L8 — Data relocations:** support pointers/function addresses embedded in data and data-to-data/data-to-code symbol relocations.
- [ ] **L9 — External calls:** emit unresolved external function symbols/relocations for declared-but-not-defined callees.

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
- [ ] **L20 — Designated aggregate initialization:** BLOCKED in lowering: the Saltwater AST/HIR initializer model contains only scalar/list forms and the parser does not parse `.member =` or `[index] =` designators. Frontend parser/HIR support is required before SIA32 lowering can preserve designators.
- [ ] **L21 — Non-fixed arrays:** BLOCKED for true VLAs: the Saltwater frontend currently constant-folds every array bound and substitutes `Fixed(1)` after a non-constant-bound diagnostic; HIR `ArrayType` has only `Fixed`/`Unbounded`, so runtime VLA bounds do not reach SIA32 lowering. Incomplete `T a[]` objects are already completed from initializers or diagnosed separately.

## Complete partial lowering families

- [x] **L22 — Cast completion:** audited frontend `ExprType::Cast`; lower integer width/signedness, pointer/integer and function-designator address casts, aggregate-address decay shapes, FP conversions, and cast-to-void while preserving operand side effects. Invalid non-scalar/float-pointer/void-source casts remain frontend diagnostics.
- [x] **L23 — Assignment completion:** audited assignable HIR shapes; direct locals/globals retain optimized handling, aggregate-by-value assignment uses object-copy lowering, and all remaining scalar dereference/member/index/wrapped lvalues lower through the common `compile_lvalue_address` path with assignment values preserved for chaining.
- [x] **L24 — Increment/decrement completion:** audited post-`++`/`--` HIR lowering; direct SSA locals retain the fast path while globals, stack/address-taken locals, dereferences, members, array/index pointer arithmetic, and wrapped legal lvalues store through the common lvalue-address path with declared-width stores and pointer-size scaling.
- [x] **L25 — Address-of completion:** audited `StaticRef`/address HIR; globals, statics, functions, locals, nested members, array/index lvalues and wrapped legal lvalues use the common address path, while dereference operands preserve the C identity `&*p == p` without a load.
- [x] **L26 — Aggregate initializer completion:** audited local/global array/struct/union recursion, brace elision, scalar sub-braces, partial/zero-filled objects, unions, and excess-element diagnostics. Designated forms remain separately blocked by L20 because frontend AST/HIR does not preserve designators.
- [x] **L27 — Control-flow completion:** exhaustively enumerate every current `StmtType` variant and audit lowering for compound/decl/expr/return, if, while/do/for, switch/case/default/fallthrough, goto/labels, break and continue. No generic legal-statement fallback remains; an exhaustive statement-kind guard forces future HIR variants to be handled explicitly.
- [x] **L28 — Binary-operator completion:** exhaustively audit every current `BinaryOp`; cover integer arithmetic/div/rem/bitwise/shifts/comparisons with promotions and signedness, short-circuit logical operators, assignment, pointer add/sub/difference/comparisons, and FP arithmetic/comparisons. An exhaustive enum regression forces future operators to be classified explicitly. Pointer arithmetic HIR now preserves the integer index domain instead of casting the index to pointer type before scaling, and pointer-pointer subtraction is typed as a signed integer (`long`/ptrdiff representation) rather than incorrectly retaining pointer type.
- [x] **L29 — Type lowering completion:** `ir_type()` now exhaustively enumerates every frontend `Type` variant: integer/enum/pointer/function scalars map explicitly, FP maps to CLIF FP types, arrays/structs/unions are explicitly address-only/aggregate-ABI types, and void/va_list/error receive deliberate diagnostics. No wildcard/generic type-lowering fallback remains.

## Validation / completion criteria

Each lowering item should land as its own commit with focused regression tests. Before marking an item complete:

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo test --workspace`
- [ ] `cargo build`
- [ ] Add positive tests for the new lowering.
- [ ] Add negative/diagnostic tests where the C construct is invalid or intentionally unsupported.
- [ ] Re-run the e2fsprogs/ext2 compile probe and record the next concrete boundary.
- [ ] Avoid weakening existing float rejection until the floating-point slices themselves are implemented.
- [ ] Avoid host-only fallback lowering; generated code must remain on the C → HIR → CLIF → Cranelift SIA32 → COSMIC-SIA path.
