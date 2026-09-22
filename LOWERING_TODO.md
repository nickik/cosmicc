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
- [x] **L12 — FP arithmetic:** lower floating `+`, `-`, `*`, and `/` (and unary negation where applicable).
- [ ] **L13 — FP comparisons:** lower floating equality and ordered comparisons to C integer booleans.
- [ ] **L14 — FP conversions:** lower integer ↔ float/double and float ↔ double conversions.
- [ ] **L15 — FP ABI:** support floating-point parameters, return values, direct calls, and indirect calls according to the SIA32 ABI.

## Aggregate values and arrays

- [ ] **L16 — Aggregate value copy:** lower whole struct/union assignment such as `a = b`, including overlap-safe semantics where required.
- [ ] **L17 — Aggregate returns:** lower functions returning structs/unions by value according to the SIA32 ABI.
- [ ] **L18 — Aggregate arguments:** lower structs/unions passed by value according to the SIA32 ABI.
- [ ] **L19 — Aggregate scalar/brace-elided initialization:** support remaining legal aggregate initializer shapes not handled by the current recursive initializer-list path.
- [ ] **L20 — Designated aggregate initialization:** support designated array/member initializers if/when represented by the frontend HIR.
- [ ] **L21 — Non-fixed arrays:** lower supported variable-length stack arrays; diagnose genuinely incomplete object types separately.

## Complete partial lowering families

- [ ] **L22 — Cast completion:** audit every frontend cast HIR shape; implement remaining legal integer, pointer, function-designator, aggregate-address, and newly supported FP conversions. Reject only invalid C conversions.
- [ ] **L23 — Assignment completion:** audit every assignable HIR lvalue; implement remaining unusual lvalue shapes and integrate aggregate-by-value assignment.
- [ ] **L24 — Increment/decrement completion:** audit all legal scalar lvalue forms for `++`/`--`, including wrapped/address-taken forms and correct width/pointer scaling.
- [ ] **L25 — Address-of completion:** lower addresses of globals/statics/functions and every legal frontend lvalue shape; retain `&*p == p`.
- [ ] **L26 — Aggregate initializer completion:** audit array/struct/union nesting, brace elision, zero-fill, excess-element diagnostics, unions and designated forms.
- [ ] **L27 — Control-flow completion:** enumerate every `StmtType` variant and remove the generic control-flow fallback for all legal statements.
- [ ] **L28 — Binary-operator completion:** enumerate every `BinaryOp` and legal operand category, including integer, pointer and FP cases, with correct promotions/conversions.
- [ ] **L29 — Type lowering completion:** enumerate every frontend `Type` variant and give each an explicit SIA32 representation, address-only treatment, ABI treatment, or deliberate diagnostic; remove accidental fallthrough to generic `ir_type()` failures.

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
