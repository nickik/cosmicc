# Cosmic C portability and runtime work

Current SIA continuation: I64 arithmetic, binary32/binary64 software arithmetic,
float/integer conversions, aggregate arguments/returns and incoming/outgoing
variadic calls are implemented. **146/147 reviewed cases execute successfully**
with explicit guest runtime objects. Production RTL CPU/mainboard fixtures pass
for binary64 and cross-TU aggregate/varargs behavior; this is not physical FPGA
or complete-system qualification. AMD64 incoming stdarg and INTEGER/SSE/MEMORY aggregate calls now pass
GCC/Clang interoperability checks; its full inventory is maintained separately.

This is the active checklist for making real C software run on Lighting.
Historical TODO.md describes the initial compiler and is not current status.
Evidence is distinguished as source/CLIF, emitted SIA, Lighting architectural
execution, or SoftwareCpuBoard/MainboardFPGA execution. Never equate these.

## Next SIA implementation order

The five frontend failures are closed. I64, binary32/binary64 and all scalar
integer/float width conversions have normal compiler lowering and integer-only
runtime calls. Aggregate snapshots/hidden returns and packed variadic arguments
are documented in [SIA ABI](tools/sia/AGGREGATE_VARIADIC_ABI.md).

1. Add startup/sysroot, archive extraction and standard runtime dependencies
   for larger real-software gates. The checked static multi-object linker works.
2. Broaden the implemented decimal/fenv/libm corpus and production RTL coverage.
3. Define cross-language ABI interoperability and version compatibility explicitly;
   current aggregates/varargs use a private Cosmic convention.
4. Broaden zlib and ext2 dependency closures, then SQLite in-memory.
5. Expand production RTL coverage beyond these bounded fixtures and qualify
   full ROM/RAM/MMIO boot; physical FPGA qualification remains separate.

## 1. Integer and ABI prerequisites

- [x] Complete SIA I64 variable shifts, multiply, divide and remainder.
- [x] Validate signed division including negative operands and INT64_MIN;
  define divide-by-zero and overflowing signed division diagnostics/traps.
- [ ] Exercise mixed I32/I64 arguments, register exhaustion, stack arguments,
  returns, recursion, indirect calls, and spills on the composed board path.
- [x] Add target-native integer lowering where operations need software;
  route unsupported CLIF operations through the normal backend, without a
  private instruction encoder or host arithmetic fallback.

## 2. Software floating point (SIA has no FPU)

- [ ] Freeze a shared C/Forge/Rust soft-float ABI: IEEE binary32 in one word,
  binary64 in the existing little-endian I64 pair convention, stack alignment,
  mixed argument placement, returns, and indirect calls.
- [x] Define rounding (initially nearest, ties to even), NaN propagation,
  subnormals, signed zero, infinities, conversion range behavior, and exception
  state. If exception flags are exposed they must be per task, not shared globals.
- [x] Port and retain licensing for an established integer-only implementation
  (candidate: Berkeley SoftFloat 3e). Audit its integer prerequisite closure.
- [x] Add binary32/binary64 legalization that replaces FP SSA values with integer bit
  representations and arithmetic/comparison/conversion operations with calls
  to integer-ABI helpers before SIA lowering. Cover blocks, loads/stores,
  constants, direct/indirect calls and signatures, including float/I64 casts.
- [x] Support float/double static initializers and constant folding with target
  semantics, including deterministic decimal-to-binary conversion.
- [x] Test bit-exact arithmetic/conversions against upstream integer vectors on
  LightingMachine and ordinary C arithmetic/ABI fixtures on production RTL.
- [ ] Run the full vector corpus and broader software on production RTL.
- [x] Add integer-only decimal parsing/formatting, C fenv and C99 real math
  families; SIA long double uses binary64 precision. SIA varargs/default
  float-to-double promotion is implemented.
- [ ] Qualify larger numerical software, non-C locales, wide/FILE text I/O,
  and complete-system/physical FPGA execution.

## 3. Frontend compatibility

- [x] Fix qualifier placement in declarations and permitted pointer conversions;
  allow adding pointee const/volatile, reject removing them and unsafe T**
  conversions, preserve pointer-object const, and check writes through const.
- [x] Lower offsetof-style member addresses, including nested members/arrays,
  without loading through null pointers; validate target layout. Static constant
  expression acceptance and a target offsetof header remain separate work.
- [x] Add target-owned limits.h, strtoul/strchr declarations and ERANGE;
  compile width/extrema assertions and verify emitted boundary bytes.
  These declarations do not provide standard runtime implementations.
- [ ] Complete target-owned timeval and standard strtoul/strchr implementations
  with error policy; versioned cosmic_* helpers remain separate interfaces.
- [x] Infer incomplete-array bounds before symbol publication, including
  aggregate designators, brace elision and string terminators; subsequent
  sizeof passes the focused LightingMachine regression.
- [x] Diagnose the observed block.c timeout: the broad unmodified TU probe
  terminates at unsupported imul.i64 rather than timing out. Integer/backend
  completion remains tracked above; this is not block.c runtime acceptance.
- [x] Fix namei.c's generated register-storage tmp: preserve parent expression
  declarations, capture addresses at expression evaluation time, and preserve
  pointer-to-lvalue nodes. Unmodified namei.c compiles; focused compound
  assignment/loop/short-circuit regressions pass on LightingMachine.
- [x] Preserve sibling function-macro hide sets for repeated nested DO2/DO4/DO8
  expansions; handle qualified pointer comparisons/differences, array decay,
  conditional qualifier unions, and integer offsets in prefix pointer ++/--.
- [ ] Re-run the unmodified e2fsprogs 1.47.2 dependency closure; track actual
  diagnostics instead of treating individual compiled files as an ext2 port.

## 4. Linking and loading

- [x] Add deterministic bounded layout and checked Abs4 code/data relocation
  resolution for a single COSMIC-SIA bundle, rejecting unresolved imports.
- [x] Validate linked calls, global pointers and function pointers on Lighting.
- [x] Record explicit TU-local bindings in v4 bundles; namespace static/string
  definitions by input ordinal and resolve cross-TU external references.
  Two serialized TUs execute on architectural and composed-board Lighting.
- [x] Add multi-object cosmic-link with checked relocations and optional CSIAIMG
  transport metadata; diagnose duplicate strong/tentative definitions.
- [ ] Add weak/common coalescing, archive extraction and protected executable ABI.
- [ ] Integrate Cosmic's protected loader, read-only mappings, capability import
  resolution, startup/termination and malformed-image rejection.

## 5. Freestanding runtime

- [x] Add integer-only memory routines and soft-float bit primitives with
  emitted-SIA tests and native linked execution coverage.
- [x] Implement namespaced target-native allocation with alignment/exhaustion,
  realloc preservation, zero-size policy, free/reuse/coalescing and calloc.
  Standard malloc/free bindings and task-owned allocation policy remain separate.
- [ ] Implement string routines, errno/error policy, abort/exit and CRT startup.
- [ ] Ship a versioned sysroot and test that no host headers/symbols leak in.
- [x] Specify SIA varargs; printf and general hosted-library coverage remain open.

## 6. Real software acceptance

- [ ] zlib: compress/decompress and upstream correctness tests on Lighting.
- [ ] ext2: native lookup/read/create/write/close/flush through QDX-B; independent
  byte comparison and e2fsck, including error paths and persistence failures.
- [ ] SQLite: in-memory tests first, then Cosmic VFS persistence and locking.
- [ ] Lua: numeric/runtime prerequisites, interpreter tests and Cosmic I/O.

## Validation

Run scripts/check-supported.sh and the release compiler build for compiler
changes. New linked/runtime behavior must also execute through Lighting;
record which machine path was used. Existing sibling backend edits are not
owned by this checklist and must not be overwritten. Published Git pins and
clean-checkout validation are required before release claims.

## First implementation slice — 2026-10-04

- Added Artifact::link_image and cosmic-link: explicit base/entry/byte budget,
  internal Abs4 relocation resolution, four-byte function/literal-pool alignment,
  region permissions metadata, and rejection of undefined symbols, duplicate
  definitions, overlapping/out-of-range relocations and overflowing addresses.
- Added target-native runtime/primitives.c (memory operations, IEEE bit
  classification/sign handling, sticky shifts, wide multiply and unsigned
  word-pair division). These are explicit helper calls, not compiler-wired I64
  lowering or complete IEEE arithmetic.
- Fixed declaration/pointee qualifier placement, typedef qualifier preservation,
  pointer-conversion direction, unsafe qualifier removal, const lvalue writes,
  global function-pointer call value loading, and null-based member addressing.
- Fixed array-member decay: its pointer denotes inline storage and must not be
  loaded as though the member were a pointer-valued slot.
- Supported workspace gate: 133 parser, 19 preprocessing, 141 lowering/image
  tests, five doctests passed; two historical doctests ignored.
- CLI smoke test compiled and linked direct calls plus global data.
- The unmodified e2fsprogs 1.47.2 alloc.c and bb_inode.c compile. fileio.c now
  reaches unsupported udiv.i64; namei.c reaches the generated tmp issue above.
  These are translation-unit probes, not an ext2 runtime acceptance result.

Execution results and exact limitations are recorded in runtime/README.md.

Final linked-runtime acceptance passed on both LightingMachine and the public
SoftwareCpuBoard → MainboardFPGA → RAM path: 466,209 guest instructions,
zero exit status and exact stack restoration. This includes nested/array member
offsets and const-pointer reassignment. The release builds of cosmicc and
cosmic-link passed. Logs: /tmp/cosmicc-portability-supported.log,
/tmp/cosmicc-portability-release-final.log, /tmp/cosmicc-runtime-native-final.log,
and /tmp/cosmicc-runtime-board-final.log. The native tool uses its existing
sibling development patches; this is local acceptance, not a published-pin
clean-checkout or remote-CI result.


## Frontend and target-header follow-up — 2026-10-04

- Supported frontend follow-up gate passed: 135 parser, 19 preprocessing,
  148 lowering/image tests and five doctests (two ignored), recorded in
  /tmp/frontend-zlib-supported.log. Focused array/compound-assignment execution
  passed on LightingMachine in 791 guest instructions; this follow-up does not
  claim composed-board execution for the new expression compatibility cases.
- Unmodified e2fsprogs 1.47.2 broad inventory: 31 of 107 ext2fs TUs compiled.
  This includes optional/test files, not a selected linked dependency closure.
- Unmodified zlib now passes its previous nested-macro and pointer-expression
  frontend boundaries. Fresh ten-second TU inventory: crc32/zutil emit SIA;
  infback/inflate reach a cached backend branch-range assertion; five larger
  core TUs time out. Independent semantic probes for adler32/deflate/inftrees
  complete promptly without errors. Backend follow-up results belong to the
  integer/linking evidence, not frontend/application acceptance.
- New parser/tests/fixtures/target_limits.c compiles using target-owned built-in
  headers (also verified with an explicit include path). Compile-time width/extrema assertions and ten emitted little-endian
  boundary objects pass; strchr/strtoul declarations produce expected unresolved
  relocations. /tmp/cosmicc-target-limits-verify.log records the byte checks.
  No standard runtime implementation or native header-fixture execution is
  claimed. Current sysroot has no sys/time.h/timeval declaration.

Details: FRONTEND_PORTABILITY_STATUS.md and
saltwater-parser/FRONTEND_ZLIB_STATUS.md. Counts above capture their validation
runs; coordinated backend work may add further tests afterward.

## Compatibility continuation — local evidence

- Fixed nested macro expansion, qualified pointer expressions, prefix pointer
  increments, narrow casts/returns/local initializers, and static array-designator
  relocation identity. Workspace validation: 135 parser, 19 preprocessing,
  149 lowering/image tests and five doctests pass (two historical ignores).
- Expanded frontend guest regression passes both Lighting paths: 1,128
  instructions covering macros, pointer difference/decrement, narrow locals,
  initialized arrays and compound-assignment evaluation.
- Preserved isolated backend frame/long-branch fixes as a reviewable patch under
  tools/backend-patches. 40 SIA tests pass. Expanded upstream zlib CRC passes
  both Lighting paths: 26,430 instructions and restored SP/FP. Publishing the
  dependency revision awaits explicit approval; product pins are unchanged.
- Eight of nine unmodified Z_SOLO core TUs compile with those local fixes.
  adler32 still requires signed I64 remainder; multi-TU linking and a native
  compression/decompression round trip remain open.
- Allocator/string helpers pass the full 96-step allocator stress on both
  Lighting paths: 987,013 instructions, zero result and restored stack.
  Includes free/reuse, coalescing, realloc, calloc overflow and conversion
  boundaries; see runtime/FREESTANDING_STATUS.md.

Final continuation validation: formatting and diff checks pass; the isolated
patched-backend workspace tests pass (135/19/149 plus five doctests), and both
release binaries build successfully. Logs: /tmp/cosmic-isolated-supported.log,
/tmp/cosmic-isolated-release.log. The tracked lockfile was restored to the
published dependency pins after explicit local-override validation.

## Published backend and full zlib compile — 2026-10-05

- Approved backend fixes published to nickik/crainlift branch
  cosmicc-frame-preservation, exact revision 5df9a089753b520a47251c635e99be4466b4e7f6.
  Manifests/lockfiles use that revision; native tool's backend sibling overrides
  removed. Lighting remains its explicitly documented local adapter.
- Added automatic signed/unsigned I64 remainder legalization through normal
  integer CLIF operations, with zero-divisor and signed overflow traps.
- All nine unmodified zlib 1.3.2 Z_SOLO core files compile. This is a compilation
  gate, not linked compression/decompression acceptance.
- Published-pin supported gate passes: 135 parser, 19 preprocessing, 150 lowering
  tests and five doctests. CRC passes both Lighting paths at 26,430 instructions.
- Broader pinned C suite baseline and actual output/target audit are recorded
  in tools/compatibility and TARGET_OUTPUT_STATUS.md. ELF output and amd64/
  aarch64 frontend/backend/ABI integration remain product work.

## Next compatibility and output priorities

- [ ] Fix block-scope external function declaration identity (upstream
  c-testsuite 00078), preserving compatible declarations and genuine imports.
- [ ] Coalesce compatible tentative global definitions (00096/00136), rejecting
  conflicting strong definitions instead of producing duplicate bundle symbols.
- [ ] Complete ordinary C aggregate-value ABI and remaining narrow conversions.
- [ ] Integrate relocatable ELF emission into Cosmic C, with an explicit SIA
  machine/relocation contract and linker/loader acceptance. Current C bundles
  are not ELF; shared-backend object tests alone do not prove product support.
- [x] Add initial amd64 target selection, LP64 scalar ABI and ELF emission,
  with the reviewed native compile/execution suite recorded below.
- [ ] Add aarch64 target/layout/ABI/object integration and acceptance.

Broader reviewed c-testsuite baseline: 147 selected, 73 explicitly excluded;
127 compile and all 127 pass LightingMachine architectural execution, with
no traps and restored stacks (8,044,764 total guest instructions). Remaining
20 compile failures are recorded by category and case; no full ISO C conformance
or composed-board suite pass is claimed. Automatic I64 remainder validates
178 signed/unsigned vectors architecturally and 16 boundary vectors on the
composed board (85,144 instructions); zero/overflow traps are separately checked.

Final published-pin verification: scripts/check-supported.sh and optimized builds
of cosmicc/cosmic-link pass. Native tools formatting passes. Logs:
/tmp/cosmic-published-supported.log and /tmp/cosmic-published-release.log.

## amd64 implementation — 2026-10-05

- [x] Add explicit AMD64 LP64 frontend/layout/header model, preserving SIA ILP32.
- [x] Emit standard ELF64 x86-64 relocatable objects through current Cranelift.
- [x] Integrate maintained --target amd64 / x86_64-unknown-linux-gnu CLI,
  compile-only objects and multi-source/object/archive linking through existing CC.
- [x] Validate System V scalar integer/floating ABI, stack arguments, narrow
  parameters/results, callbacks, globals, static locals and data relocations.
- [x] Validate GNU ld and LLVM lld on emitted objects and native executables.
- [x] Run complete reviewed 147-case amd64 baseline: 135 native passes,
  11 existing frontend diagnostics and one aggregate/variadic ABI limitation.
- [x] Fix narrow-index pointer scaling in the shared frontend; scale in pointer
  width before multiplication instead of overflowing unsigned-char offsets.
- [ ] Complete variadic/aggregate-by-value ABI and remaining frontend failures.
- [ ] Port native startup/runtime to Cosmic OS; present executable driver is Linux.
- [ ] Add aarch64 target using the same maintained target/object architecture.

Current backend exact revision `951ced55b67cc3b2cc76a04b1105a43e8aaf688d`
adds checked x86-64 System V variadic vector-count placement to the earlier
object-writer and SIA frame fixes. It is published on the approved isolated
`cosmicc-frame-preservation` backend branch. See
[backend contract](tools/backend-patches/X64_VARIADIC_COUNT.md).

Final amd64 acceptance: all nine unmodified zlib units compile/link and 16 native
round trips plus allocation refusal pass without host zlib. Final reviewed suite
rerun remains 135/147 native passes. Supported gate: 136 parser, 19 preprocessing,
4 target-model, 4 amd64 and 150 SIA tests plus five doctests pass; two historical
ignores. Release compiler/image-linker builds pass, as does a release-built
multi-source amd64 executable. Both SIA paths pass expanded index regression
(22,268 instructions). Logs: /tmp/cosmic-amd64-supported-index-final.log,
/tmp/cosmic-amd64-release-index-final.log; native evidence is in
TARGET_OUTPUT_STATUS.md. Remaining ABI/frontend gaps are explicitly recorded.

## Differential compatibility and performance evidence

- [x] Pin and hash the complete 220-case c-testsuite corpus; retain explicit
  reviewed freestanding (147) and outside-profile (73) denominators.
- [x] Add bounded native compile/link/execute comparisons against GCC 13 and
  Clang 18 at O0/O2, checking upstream combined output and zero exit, preserving
  immutable compiler snapshots, hashes, commands and per-case diagnostics.
- [x] Fix eager folding of unused logical/conditional operands; preprocessing
  short-circuit expressions now pass both target-model regressions.
- [x] Add explicit amd64 Cranelift none/speed/speed_and_size modes; default none.
- [x] Add checksum-validated native kernels and separate source-to-object and
  system-link measurements, with warmups, shuffled repetitions, CPU affinity,
  machine metadata and raw samples. See tools/benchmarks/README.md.
- [ ] Complete amd64 variadic System V ABI: it is the largest observed hosted
  suite boundary. Add va_list/default promotions and meaningful ABI interop gates.
- [x] Close the observed anonymous-member, compound-literal and wide-character
  constant frontend gaps; remaining wide strings, anonymous designators and
  target dynamic VLA allocation remain explicitly tracked in frontend status.
- [ ] Add a pinned GCC torture compile/execute matrix and defined-behavior Csmith
  differential corpus. Source review, optimization agreement and expected-output
  assertions complement each other; GCC/Clang agreement is not an infallible oracle.
- [ ] Expand native runtime measurements to representative full applications and
  broader inputs; small hot-cache kernels do not establish universal performance.
- [ ] Add backend-only timing with equivalent IR if isolating Cranelift vs LLVM;
  published WebAssembly/query JIT results are separate evidence, not laptop C results.


Current source-frozen validation (2026-10-05): 136 parser, 19 preprocessing,
5 target-model, 4 amd64 and 153 SIA tests plus 5 doctests pass (2 historical
ignores). Release compiler and image-linker builds pass. Full native differential
matrix: Cosmic O0/O2 each 138/220, reviewed 136/147; GCC 13 / Clang 18 O0/O2
all 220/220. SIA reviewed profile: 128/147 compile and all 128 pass architectural
execution. Source-frozen native optimized zlib passes 16 roundtrips; exact SIA
CLI container passes one bounded roundtrip. Current reports and interpretation:
TARGET_OUTPUT_STATUS.md and tools/compatibility/DIFFERENTIAL_STATUS.md.


Laptop measurement completed with the same release snapshot as the full matrix:
see tools/benchmarks/LAPTOP_PERFORMANCE.md and retained results JSON. At O2,
zlib source-to-object compilation is about 6.4–6.5 times faster than GCC/Clang;
the four native kernels are 1.4–3.1 times slower than GCC and 1.8–3.5 times
slower than Clang. These are workload-specific whole-compiler measurements.

- [x] Promote non-address-taken scalar locals/parameters into Cranelift SSA,
  preserving volatile semantics and address-taking/alias behavior. The measured
  Cosmic hash loop repeatedly accesses stack slots; GCC keeps its running hash,
  cursor and multiplier in registers. Re-run identical correctness and timing
  gates after changes; do not attribute this frontend lowering cost universally
  to Cranelift. Then inspect loop transformations and vectorization separately.

## Current implementation continuation — 2026-10-05

The results above retain their original compiler snapshots. Final validation for
the following implementation is being recorded separately.

- [x] Promote suitable native scalar locals and parameters through Cranelift SSA;
  retain storage for volatile, address-taken and aggregate objects. CFG sealing
  handles branch/loop joins, switch, goto and short-circuit expressions.
- [x] Implement outgoing native System V variadic calls, default promotions,
  register/stack exhaustion, direct/indirect calls and checked AL vector counts.
- [x] Accept native variadic definitions that use only their named parameters.
- [x] Implement native MEMORY-class aggregate parameters larger than 16 bytes
  with supported natural layouts/alignment, padded caller snapshots and callee
  by-value copies. Register-class aggregates and aggregate returns remain below.
- [x] Implement anonymous members and file/block compound literals; preserve
  runtime initializer evaluation, inferred array bounds and lvalue behavior.
- [x] Correct macro argument prescan and macro-expanded #line semantics.
- [x] Implement 32-bit wide character constants, scoped typedef name
  disambiguation and C99 array-parameter qualifiers/static/prototype [*].
- [x] Normalize postfix _Bool increments/decrements on both targets; compare
  promoted locals and escaped/global/aggregate storage with GCC and Clang.
- [ ] Complete incoming va_list/va_start/va_arg/va_copy/va_end, register save area
  and overflow-argument traversal; current named-only definitions do not prove it.
- [ ] Implement <=16-byte register-class aggregate arguments and aggregate returns.
- [ ] Define and implement SIA variadic ABI separately; native System V support
  does not establish SIA varargs.
- [x] Re-run immutable compatibility matrix for the final compiler snapshot;
  retain raw results and reference-window drift checks for the benchmark below.

Implementation details: [SSA](tools/amd64/SSA_STATUS.md),
[variadic ABI](saltwater-amd64/VARIADIC_STATUS.md),
[MEMORY aggregates](saltwater-amd64/MEMORY_AGGREGATE_STATUS.md).

Published-pin integration gate passes: formatting, 137 parser unit tests,
19 preprocessor tests, 10 target-model tests, 10 AMD64 tests, 153 SIA tests
and five doctests (two historical ignores), maintained debug build and workspace
check. Backend revision is `951ced55b67cc3b2cc76a04b1105a43e8aaf688d`.
Corrected SIA Bool fixture passes 2,503 architectural instructions. The current
Lighting board API requires `LIGHTING_CPU_BOARD_BRIDGE`; no production RTL
bridge is configured, so this fixture has architectural evidence only.
Logs: `/tmp/cosmic-ssa-varargs-supported-final2.log` and the Bool status report.

Final release snapshot SHA256:
`a223f80c4c6bf9ecc0933e63073d15201dd4c4820dee98374c1f008752c2a383`.
SIA reviewed profile: 137/147 compile, all 137 execute successfully in
8,044,273 LightingMachine instructions. Native O2 zlib passes 16 roundtrips and
allocator refusal; SIA linked-container bounded roundtrip passes 875,907
instructions. Retained JSON/TSV files: `tools/compatibility/results/2026-10-05-*ssa-varargs*`.

The first expanded native matrix reached 195/220 (146/147 reviewed), exposing
four newly reachable failures. Fixed them before final acceptance: native FP
conversion to I8/I16 uses an I32 conversion and narrowing; C logical/comparison
operators yield int; shifts independently promote operands and retain the left
operand's type; pragma push_macro/pop_macro preserve nested saved definitions.
Focused operator tests use defined behavior, unlike portions of upstream
00178/00200. Final supported gate now passes 140 parser, 19 preprocessing,
12 target-model, 11 AMD64 and 153 SIA tests, plus five doctests/two ignores.

Final operator-corrected release is
`60e33b787f7d6350ad66d4246e940a71648780908719448234af2193c9639835`.
All 137 SIA compiled reviewed cases pass (11,311,259 architectural instructions);
bounded SIA zlib passes 1,028,362 instructions / 180,172 image bytes. Native ABI,
zlib, variadic, MEMORY aggregates, Bool and narrow FP conversions all pass for
this release. The intermediate snapshot above is retained as historical evidence.

Final native full matrix: Cosmic O0/O2 each 199/220, reviewed 146/147 and
outside-profile 53/73; GCC13/Clang18 O0/O2 each 220/220. All 21 Cosmic failures
are compile rejections; no compiler panic, link failure, execution failure,
output mismatch or timeout. The lone reviewed rejection 00144 retains its
undefined-behavior/constraint annotation. Next priorities are incoming stdarg,
small/return aggregates, remaining valid-C rejections and target-runtime gaps.

- [x] Repeat identical source-to-object, link and runtime laptop measurements
  with the final compiler snapshot; preserve checksums, randomized samples and
  reference-window drift.

[Measured report](tools/benchmarks/SSA_LAPTOP_PERFORMANCE.md) and
[before/after comparison](tools/benchmarks/SSA_COMPARISON.md): O2 hash takes
72.45 ms versus 229.70 ms before, now about 1.01× GCC/Clang. Branch, divide and
matrix take 285.47, 250.78 and 71.69 ms. Unchanged references are also
1.19–1.27× faster between windows, so the observed improvement does not isolate
SSA causality. O2 kernel compilation takes 3.65 ms; nine zlib units take 193.00 ms
versus GCC 1122.20 / Clang 1136.70 ms. Raw JSON and objdump are retained in
`tools/benchmarks/results/2026-10-05-ssa*`.

## Earlier SIA arithmetic snapshot (superseded by current status above)

The five targeted failures are fixed. Reviewed SIA profile now compiles 142/147
and all 142 pass architectural execution (11,312,401 instructions); raw compile
inventory and execution TSV are retained in tools/compatibility/results/.
Ordinary C I64 multiply/divide/remainder/variable shifts and a bounded mixed
register/stack/return/recursive/indirect ABI fixture passes 627,896 instructions.
See [I64 status](saltwater-sia/I64_STATUS.md).

The licensed integer-only binary32 addition runtime is implemented and passes
bit/flag-exact upstream checks and 754 architectural guest vectors.
See [soft-float scope](runtime/softfloat/README.md). Ordinary C FP legalization,
other FP operations, expanded I64 ABI/fault-path testing and production board
execution remain outstanding.

## Current wide floating-point and ABI acceptance

146/147 reviewed SIA cases execute with explicit guest runtime objects.
Binary64 and float/I64 conversions are implemented; 2,105,005 upstream
bit/flag checks pass. Aggregate snapshots/returns and packed stdarg pass across
TUs. Production RTL fixtures pass: double 72,249, cross-TU ABI 20,706,
focused aggregate/varargs 3,877 instructions. See
[current compatibility evidence](C_COMPATIBILITY_STATUS.md) for hashes,
raw files and remaining qualification boundaries. Native O0/O2 differential
replay preserves 199/220; GCC13/Clang18 each pass 220/220.

## 2026-10-05 fenv, numerical runtime and FPGA continuation

- [x] Distinct SIA long double type with binary64 storage/precision and target float.h.
- [x] Task-bindable eight-byte exception/rounding context, all four C rounding
  modes, C fenv save/hold/restore/update and dynamic FLT_ROUNDS. Task switches
  must bind their context explicitly; no scheduler/TLS integration is claimed.
- [x] C99 real math families, exact fused multiply-add and rounding helpers,
  licensed fdlibm transcendental implementation with retained adaptation records.
  Composite exp2/log2/gamma paths have bounded accuracy checks; general correct
  rounding of every transcendental result is not claimed.
- [x] Allocation-free integer-only decimal memory I/O with bounded glibc
  comparisons in four modes: 16,200 parse bit/end checks, 93,196 format byte/length
  checks and 28 truncation checks. See runtime/decimal/results/2026-10-05.
- [x] Opt-in cosmic-link --gc-sections retains relocation-reachable code/data;
  validates malformed relocations and duplicate definitions even in discarded input.
- [x] C99 math language fixture passes 554,577 architectural guest instructions;
  task/fenv language fixture passes production CPU/mainboard RTL in 33,779 guest
  instructions and 265,601 board cycles. See tools/sia/results/2026-10-05-runtime.
- [x] Supported workspace gate passes: 19 AMD64, 141 parser, 19 preprocessor,
  21 target-model, 155 SIA and 5 documentation tests; build/check/format pass.
- [ ] Physical FPGA bitstream/programming, full Cosmic boot and task scheduler
  integration remain separate qualification milestones. LightingSimulation is
  the FPGA-first integration target; board support must remain below its ISA/ABI.

The separate AMD64 continuation now passes 210/220 at O0/O2 with no accepted
program execution failures. Incoming va_list, INTEGER/SSE aggregate arguments
and returns, and MEMORY returns pass GCC13/Clang18 interoperability gates.
C11 _Generic is the next valid frontend gap; AMD64 x87 long-double ABI remains
explicitly unsupported. Constraint-invalid/GNU-only cases retain diagnostics.

The C99 real-math API fixture also passes production CPU/mainboard RTL:
554,528 guest instructions, 4,243,356 board cycles and 609,549 backend memory
transactions, with restored stack and no trap. See c99-math-production.log in
the retained runtime result directory. This is HDL execution in Bluesim, not
a physical board bitstream.

The decimal language fixture passes production CPU/mainboard RTL in 1,440,640
guest instructions and 11,713,002 cycles. The fresh autonomous logical-machine
control/reset qualification also passes; its source/tool/assets/artifact report
is retained under LightingChips/results/2026-10-05-fpga-qualification. Physical
FPGA part/board shell/pin and clock constraints remain absent from the inspected
GitHub and local source inventories.

## C11 generic-selection continuation

- [x] C11 _Generic: type-compatible association selection, expression decay
  and top-level qualifier removal, selected lvalue/function category, and
  control/unselected non-evaluation. Distinct plain/signed/unsigned char type
  identities and C int ordinary-character literals are preserved.
- [x] Duplicate compatible associations/defaults, incomplete/variably modified
  association types and missing matches have focused diagnostics on both targets.
- [x] Unchanged generic-language.c passes architectural and production RTL
  execution in 595 instructions. The post-change decimal/math architectural
  fixtures and full supported gate pass. See tools/sia/results/2026-10-05-generic.
- [ ] AMD64 x87 long-double call/return/storage interoperability remains open.

Final frozen AMD64 replay after _Generic: **211/220** exact upstream-output
passes at both O0/O2; GCC13 and Clang18 each pass 220. Nine compile rejections
remain, with no accepted execution failures. Reviewed profile 146/147; outside
profile 65/73. Compiler SHA-256 ff5de80019013a5315083698fc3a09290f787ffcb3be7145abbcd3a385ec5f0e.
See tools/amd64/results/2026-10-05-native-generic-220.tsv.

The final post-_Generic SIA reviewed replay remains 146/147, with all 146 emitted
candidates passing and 11,315,804 guest instructions. Evidence: tools/compatibility/
results/2026-10-05-sia-generic-final/execution-summary.json.
