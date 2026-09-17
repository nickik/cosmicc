# Cosmic C roadmap

Cosmic C is the maintained, Rust-written C compiler for Cosmic OS.  Its production route is:

```text
C source → preprocessor → typed HIR → CLIF → Cranelift SIA32
         → Cosmic relocatable object → Cosmic image/module → LightingSimulation
```

The first merged implementation proves the beginning of that route: it parses C and emits native SIA32 instruction bytes through the real Cranelift SIA backend.  It is not yet a generally usable C compiler or a loadable Cosmic object producer.

## Product definition

### First-class target

The first supported target is a **freestanding ILP32 Cosmic C profile**:

- 8-bit bytes; little-endian 16- and 32-bit integers; 32-bit pointers and `long`.
- SIA32 ABI: `r0` is zero; `r1–r6` carry scalar arguments; `r1` is the scalar result; 8-byte stack alignment; no red zone.
- Target label is initially `sia32-unknown-none`, until the Cosmic object format is specified; then promote the public user-facing target to `sia32-unknown-cosmic`.
- The compiler runs on a build host.  Generated code, objects, and images are always SIA32—never host executables, a JIT result, or host-linker output.
- The supported language baseline is C11 core plus the C99/C17 features Cosmic needs.  “First class” means a documented profile with predictable diagnostics, not an unqualified claim of full hosted C conformance.

### Explicit non-goals for the initial profile

- **No floating point**: `float`, `double`, FP literals, FP member types, and FP conversions fail before CLIF generation.  Do not silently lower or emulate them.
- No 64-bit scalar ABI/`long long`, varargs, TLS, unwinding, dynamic linking, hosted libc, SIMD, or C23 work until the target contracts exist.
- No alternate direct SIA encoder.  Use `nickik/crainlift`’s `LowerBackend → ISLE → MachInst → VCode → regalloc2 → emitter` pipeline.

### Evidence hierarchy

| Evidence | What it proves | What it does **not** prove |
| --- | --- | --- |
| Frontend/unit test | parsing, semantics, diagnostics, HIR/CLIF shape | SIA execution |
| Backend byte/object test | SIA encoding, relocations, object structure | whole-program behavior |
| Host differential test | defined-C integer behavior | Cosmic ABI or SIA hardware behavior |
| LightingSimulation test | real SIA/board-path execution | language feature coverage by itself |
| Cosmic boot/module test | usable OS integration | broad compiler conformance |

A milestone is complete only when its stated evidence is present.

## Current state

### Completed in PR #1

- [x] Rename the main package and compiler binary to `cosmicc`.
- [x] Establish a dedicated `saltwater-sia` crate that lowers typed C HIR through the current `nickik/crainlift` SIA32 backend.
- [x] Pin that lowering path to a known compatible Cranelift revision.
- [x] Accept only `sia32-unknown-none` and use the initial ILP32 layout (32-bit pointer and `long`).
- [x] Compile simple integer functions with scalar parameters, explicit return, initialized SSA locals, local assignment, unary negation/not, `+ - & | ^ << >>`.
- [x] Emit word-aligned native SIA instruction bytes and package them in the temporary `COSMIC-SIA` code bundle.
- [x] Add a strict `COSMIC-SIA` decoder and an explicit SIA32 register/call plan for an external Lighting executor; this validates the loading boundary without executing code on the host.
- [x] Reject float/double declarations, nested float-containing types, and float literals—including casts of float literals—before backend lowering.
- [x] Add focused CI: formatting, SIA-path tests, and compiler command check.
- [x] Merge the initial path into `master` as `5c2586c`.

### Still intentionally incomplete

- [ ] SIA code has not executed in LightingSimulation.
- [ ] **M4 board-runner interface blocker:** LightingSimulation M6.5 is now merged at [`c51b7f178599c165b83412f683afa1a73df071a5`](https://github.com/nickik/LightingSimulation/tree/c51b7f178599c165b83412f683afa1a73df071a5) and proves `SoftwareCpuBoard → MainboardFPGA → backend` composition. Its public surface exposes low-level `LightingBoardMachine::load_word`, CPU construction, and snapshots, but no stable runner that accepts arbitrary native code bytes, an external register image, an expected return boundary, and bounded execution diagnostics. Cosmic C must not assemble a private test transport around those internals. Minimal required Lighting change: a public, dependency-neutral SIA32 board-call API that loads word-aligned native bytes at an explicit address; initializes PC plus `r1–r6` and `r14`; clocks the composed board to the return boundary/trap/cycle limit; returns `r1`; and reports code, PC, registers, cycles, board-memory transactions, trace, and fault/trap state. `siaemu` and host execution remain non-acceptance evidence.
- [ ] **Blocked comparison/branch slice:** the pinned `nickik/crainlift` revision [`2ae7419c32bbf565c60e04c6730a39610cec4e8b`](https://github.com/nickik/crainlift/tree/2ae7419c32bbf565c60e04c6730a39610cec4e8b) lowers `jump`/`brif`, and SIA has `cmpeq`/`cmplt`/`cmpltu` encodings, but its SIA32 `lower.isle` has no `icmp` (or boolean materialization) rule. Cosmic C must not hand-encode or emulate comparisons. Minimal required upstream capability: I32 signed/unsigned CLIF comparison lowering (`eq`, `ne`, `<`, `<=`, `>`, `>=`) with an explicit canonical integer-boolean result usable by `brif` and C `int` returns. Do not modify that dependency from this repository; repin only after it is provided and proven upstream.
- [ ] There is no relocatable Cosmic object, relocation model, linker, image/module integration, or multi-translation-unit support.
- [ ] Integer control flow, memory, globals, calls, aggregate layout/ABI, and normal preprocessor/driver behavior remain incomplete.
- [ ] The inherited frontend and test baseline still need a full audit; green focused CI is not a full workspace/conformance gate.

## Milestones

## M0 — Maintained compiler baseline

- [ ] Record the exact upstream Saltwater/Saltwater-derived baseline and retain all BSD-3-Clause attribution/notices.
- [ ] Remove remaining stale `saltwater`/`swcc`/`rcc`, legacy, archived, and host-codegen wording from source, package metadata, documentation, examples, and diagnostics.
- [ ] Audit every crate and feature for host-x86, JIT, system-`cc`, old Cranelift, and unsupported-toolchain assumptions.
- [ ] Decide and document the supported Rust toolchain; make `cargo test --workspace` meaningful or explicitly split retired crates from the supported workspace.
- [ ] Make lockfile dependency resolution reproducible against the pinned Cranelift revision.
- [ ] Expand CI into quick formatter/check/test lanes plus a reproducible release-build lane.
- [ ] Add versioning, changelog, issue/PR templates, and a contributor guide that states the SIA acceptance rules.

**Exit:** a clean, reproducible, actively maintained Cosmic C workspace with no ambiguous legacy product identity.

## M1 — Target, ABI, and platform contract

- [x] Initial hard target selection: `sia32-unknown-none`.
- [ ] Publish the canonical SIA32 C ABI jointly with `crainlift`, Cosmic, and LightingSimulation: register classes, argument assignment, return rules, caller/callee saves, stack frames, alignment, aggregate passing, and trap/unwind policy.
- [ ] Specify the Cosmic C data model: signedness of `char`, widths/ranks, integer promotions, enum representation, pointer representation, `size_t`/ptrdiff types, maximum alignment, layout of arrays/structs/unions, bitfield rules, and endian assumptions.
- [ ] Define the freestanding entry/import contract: entry symbols, `__cosmic_*` import namespace, capability handles, startup data, termination/panic behavior, and forbidden host symbols.
- [ ] Define volatile/MMIO semantics and the minimum compiler barriers required by Cosmic drivers.
- [ ] Publish target macros and headers: `__COSMIC__`, SIA architecture macros, widths, endian macros, and feature-test policy.
- [ ] Change the public triple to `sia32-unknown-cosmic` only when its object/module ABI is implemented; keep a compatibility alias only with a clear deprecation period.
- [ ] Add compile-time ABI probes and cross-repository layout assertions.

**Exit:** any Cosmic C translation unit has one authoritative, testable ABI and data-layout interpretation.

## M2 — Frontend and preprocessing correctness

- [ ] Inventory the inherited parser/preprocessor against the required C99/C11 profile; mark each item supported, intentionally rejected, or broken.
- [ ] Complete deterministic preprocessing: `-I`/system include search order, `-D`/`-U`, command-line include files, include guards/`#pragma once`, `#if` integer expressions, token pasting/stringification, variadic macros, and diagnostics with include backtraces.
- [ ] Implement driver-level multiple input files, `-E`, `-S`, `-c`, dependency emission, deterministic output naming, and response files where needed.
- [ ] Support ordinary modern C declarations: typedefs, storage classes, qualifiers, attributes policy, forward declarations, tags, anonymous members only if intentionally adopted, and clear unsupported-attribute diagnostics.
- [ ] Complete C11 static-layout features needed by systems code: `_Static_assert`, `_Alignof`, `_Alignas`, `offsetof`, and a limited `_Generic`.
- [ ] Implement initializers faithfully: zero/default initialization, string/array/aggregate initialization, designators, compound literals, and static constant expressions.
- [ ] Add tests from small, licensed C conformance fragments plus regression tests for every parser/preprocessor defect.
- [ ] Keep unsupported language forms explicit: reject VLAs, varargs, atomics, thread-local storage, and FP constructs with precise source locations until their milestones.

**Exit:** realistic multi-file freestanding C sources parse and diagnose deterministically.

## M3 — Complete integer C lowering

### M3.1 Scalar semantics

- [x] Initial I32 expression lowering and local SSA variables.
- [ ] **Blocked on Cranelift SIA32 `icmp` lowering at `2ae7419c32bbf565c60e04c6730a39610cec4e8b`:** do not implement C comparisons until the backend can produce a canonical integer boolean through its normal MachInst pipeline. The physical SIA compare encodings and `brif` support alone are insufficient because the pinned backend exposes neither comparison lowering nor boolean materialization.
- [ ] Lower all required integer types: signed/unsigned `char`, `short`, `int`, `long`, `_Bool`, enums, and pointers.
- [ ] Implement integer promotions, usual arithmetic conversions, signed/unsigned comparisons, truncation, sign/zero extension, and all required casts.
- [ ] Add division/remainder, multiplication, logical operators, comparisons, conditional expressions, comma expressions, increment/decrement, compound assignment, and `sizeof`.
- [ ] Define and test diagnostics or policy for undefined behavior that affects code generation (division by zero in constants, invalid shifts, overflow assumptions, null dereference in constant contexts).
- [ ] Add CLIF golden tests and SIA byte-shape tests for every scalar operation.

### M3.2 Control flow and lexical scope

- [ ] **Blocked by the same `icmp` capability:** a plain truthy `if` can use existing `brif`, but correct C relational conditions and canonical comparison values require the M3.1 backend prerequisite first. Do not add a comparison-specific fallback or direct SIA encoder in Cosmic C.
- [ ] Lower blocks, scopes, declaration lifetimes, and clean variable mapping.
- [ ] Lower `if`/`else`, `while`, `do`, `for`, `break`, `continue`, `switch`, `case`, `default`, and `goto`.
- [ ] Preserve C sequencing and short-circuit semantics for `&&`, `||`, `?:`, and comma expressions.
- [ ] Handle multiple returns and unreachable code without creating invalid CLIF.
- [ ] Add structured negative diagnostics for constructs not yet enabled.

### M3.3 Memory, objects, and globals

- [ ] Lower address-taken locals to stack slots with correct alignment and lifetime.
- [ ] Lower lvalues, loads/stores, `&`, `*`, array subscripting, member access, and volatile accesses.
- [ ] Implement static/global objects, tentative definitions, linkage, constant data, string literals, and zero-filled storage.
- [ ] Implement struct/union layout, member access, bitfields, aggregate copy/assignment, and aggregate initialization.
- [ ] Add bounds/alignment-aware internal assertions so malformed HIR cannot produce incorrect addressing.

### M3.4 Functions

- [ ] Lower direct functions with prototypes/no-prototype policy, forward declarations, recursion, direct calls, and ABI-accurate parameters/returns.
- [ ] Implement function pointers only after relocations and indirect-call ABI are proven.
- [ ] Implement small aggregate arguments/returns only after the ABI contract and Cranelift support are tested.
- [ ] Reject varargs and incompatible function calls clearly until a target varargs ABI exists.

**Exit:** a nontrivial integer-only C program with control flow, local memory, globals, and direct calls becomes valid SIA32 code.

## M4 — Real SIA execution acceptance

- [x] Define and test strict `COSMIC-SIA` bundle decoding plus an external call plan: code bytes, 2-byte entry address, `r1–r6` arguments, `lr` return boundary, and bounded failure context.
- [ ] Build a minimal bridge from `COSMIC-SIA` function bytes to the LightingSimulation SIA execution harness; do not substitute a host interpreter/JIT.
- [ ] Consume the call plan through Lighting M6.5's public board-call runner once it exists; M6.5 composition alone is insufficient because the merged `c51b7f1` API has no stable arbitrary-code/register/return-boundary invocation seam. Do not count `siaemu` reference-interpreter execution as this result.
- [ ] Execute `int add(int,int)` and prove `add(2,3)=5`, `add(0,0)=0`, and wraparound `add(0xffffffff,1)=0`.
- [ ] Execute representative scalar, branch/loop, stack-local, load/store, and direct-call programs on Lighting.
- [ ] Record PC, registers, memory, trap/fault state, and code bytes on every failure so failures are triageable across Cosmic C, Cranelift, and Lighting.
- [ ] Differential-test defined integer cases against a host reference while treating Lighting results as the only SIA execution proof.
- [ ] Add deterministic randomized and minimized regressions to the Lighting acceptance suite.
- [ ] Make the exact execution suite a required CI/release gate once its simulator dependencies are available.

**Exit:** native code produced by Cosmic C demonstrably executes correctly on the real SIA/Lighting path.

## M5 — Cosmic objects, relocations, and linking

- [ ] Jointly define the Cosmic SIA32 relocatable object format: file/header/version, sections, section flags, alignment, symbols, visibility, COMDAT/weak policy, debug-section policy, and deterministic serialization.
- [ ] Define and implement every relocation needed for direct calls, address constants, globals, string literals, imports, and later function pointers; add positive and negative relocation tests.
- [ ] Emit Cranelift code metadata, traps, stack maps/unwind placeholders, and source-symbol records where useful to Cosmic.
- [ ] Replace `COSMIC-SIA` as the normal compiler output with a documented `.o` object; retain it only as a test artifact if useful.
- [ ] Implement a host-side Cosmic linker/image builder that consumes only Cosmic objects and explicit imports—never the host linker.
- [ ] Add archive/static-library support only after object linking is stable.
- [ ] Verify duplicate/undefined/incompatible symbols, bad section alignment, relocation overflow, and malformed object input fail safely and clearly.
- [ ] Link multiple C objects plus a minimal runtime into a Cosmic module/image; execute its entrypoint in Lighting.

**Exit:** `cosmicc -c` produces loadable Cosmic SIA32 objects and the Cosmic toolchain links them deterministically.

## M6 — Freestanding runtime and headers

- [ ] Ship a versioned `cosmicc-sysroot` with `stdint.h`, `stddef.h`, `stdbool.h`, `limits.h`, `stdalign.h`, `stdnoreturn.h`, and only supported declarations from `string.h`/`stdlib.h`.
- [ ] Define compiler builtins and runtime hooks for `memcpy`, `memmove`, `memset`, `memcmp`, division/modulo helpers if needed, stack probes, and traps; specify ownership and calling convention.
- [ ] Make builtins robust for overlap, zero sizes, alignment, and volatile policy; test them both in isolation and from C.
- [ ] Provide minimal startup/CRT objects that bind to the Cosmic module loader and `__cosmic_*` capabilities.
- [ ] Add a source-level `cosmic.h` for imports, capabilities, error conventions, volatile MMIO helpers, and compiler barriers.
- [ ] Define a stable policy for `assert`, diagnostics, allocation, errno-like state, and file/IO APIs; do not imply hosted POSIX support.
- [ ] Gate `stdarg.h` and `stdatomic.h` on independently specified target varargs/atomic ABIs.

**Exit:** a C component can build against a small, coherent Cosmic sysroot without hidden host dependencies.

## M7 — Cosmic system integration

- [ ] Compile a pure-C freestanding smoke module and load it through the real Cosmic module/image path.
- [ ] Port one bounded user-space service or library that uses imports, heap/runtime hooks, errors, and multi-file linking.
- [ ] Port one capability-safe PLIO/MMIO driver slice using volatile and explicit barriers; validate behavior through the real board path.
- [ ] Exercise C↔Forge ABI calls only after a shared data-layout, symbol, calling convention, ownership, and error contract is documented.
- [ ] Exercise C↔Rust FFI only after the corresponding SIA Rust ABI is independently green.
- [ ] Validate loader rejection, import-capability denial, malformed module handling, and driver fault behavior.
- [ ] Make at least one production Cosmic build consume the released compiler/sysroot rather than a test-only artifact.

**Exit:** Cosmic C is a usable systems-language option for real Cosmic components.

## M8 — Diagnostics, tooling, and quality

- [ ] Preserve file/line/column/macro-expansion context from preprocessor through backend and linker diagnostics.
- [ ] Categorize diagnostics as source, target-profile, ABI, backend, object, linker, or simulator failure; include actionable remediation.
- [ ] Add `--emit=preprocessed,hir,clif,asm,object` debug outputs with stable/reviewable formats where possible.
- [ ] Add source-map/debug-info plan and minimal symbolic backtraces before claiming debugger support.
- [ ] Fuzz lexer, preprocessor, parser, semantic analysis, HIR lowering, object reader, and linker; minimize and retain every regression.
- [ ] Add sanitizer/Miri/UB-oriented checks to the Rust implementation where compatible with the toolchain.
- [ ] Add reproducible-build checks: same input/options/dependency versions yield byte-identical object/image output.
- [ ] Track compilation time, object size, and generated-code size on a small benchmark suite without optimizing ahead of correctness.
- [ ] Maintain a compatibility matrix: language feature × target support × diagnostics × execution evidence.

**Exit:** failures are actionable, releases are reproducible, and regressions are difficult to reintroduce.

## M9 — Release and maintenance

- [ ] Define semantic versioning for compiler, object format, sysroot, and ABI separately where necessary.
- [ ] Produce versioned compiler/sysroot releases and a pinned toolchain manifest for Cosmic builds.
- [ ] Document installation, invocation, target profile, source compatibility, imports, ABI, object format, and troubleshooting.
- [ ] Publish upgrade/migration guidance whenever the object format, target triple, or ABI changes.
- [ ] Maintain a compatibility test corpus and periodically revalidate against current `crainlift`, Cosmic, and LightingSimulation revisions.
- [ ] Establish a policy for accepted C extensions and a public “unsupported by design” list.

**Exit:** Cosmic C can be depended on by other Cosmic repositories with explicit compatibility guarantees.

## Deferred by design

These do not block first-class **integer freestanding** C support:

- Floating point, floating registers, soft-float, and SIMD.
- `long long`/I64 ABI and 64-bit atomics.
- Varargs/full `stdarg.h`.
- C11 threads, atomics, TLS, unwinding, exceptions, stack unwinding metadata, dynamic linking, and a hosted libc.
- VLAs, C23 features, broad GCC extensions, sanitizers for target binaries, optimizations beyond Cranelift defaults, and debug-information formats beyond the initial plan.

Each item needs a separately approved ABI, object/runtime, compiler, and Lighting acceptance plan before it is enabled.

## Recommended next major step

**M4 prerequisite: consume the prepared `COSMIC-SIA` call plan through a public Lighting board-call runner.** M6.5 composition is complete at `c51b7f1`, but Cosmic C needs the explicit arbitrary-code/register/return-boundary invocation API documented above. Once Lighting publishes it, add the Cosmic C-side adapter and execute `add`; until then, do not invent transport, use `siaemu`, or claim M4 complete.
