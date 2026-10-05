# Established C compatibility baseline — 2026-10-05

Current frozen differential release: **211/220 native** at O0/O2;
SIA **146/147 compile and execute**. Earlier sections retain baseline
history. See [final native report](tools/compatibility/DIFFERENTIAL_STATUS.md)
and final operator-corrected evidence below.

The c-testsuite source is pinned at commit
`5c7275656d751de0e68b2d340a95b5681858ed07`. Archive SHA-256 is
`010008bf4b5671f947ae7e8d693a1c959a4a4d795fa7227c0abb1f95de45f7e7`.
The source remains unchanged in /tmp. tools/compatibility contains the per-file
hash/selection lock and a bounded compiler inventory runner. This is a C89/C99/C11
compatibility envelope, not strict ISO C11 certification.

## Reviewed compilation results

147 selected cases out of 220; 73 excluded with explicit reasons. This includes
reviewed corrections to imperfect upstream libc/portability tags. Unsupported
language features are not silently excluded to improve the score.

| Outcome | Count |
| --- | ---: |
| Compiled to SIA bundles | 127 |
| Frontend/source diagnostics | 11 |
| Floating-point boundaries | 3 |
| Backend narrow conversion unsupported | 1 |
| Other lowering/symbol/aggregate boundaries | 5 |
| Crash or compilation timeout | 0 |

Compiler: cosmicc 0.12.0, immutable executable SHA-256
`686783e880b2a3945836eb675bc2a004f7eef08f869fe498dac32663654b7719`.
Ordinary published Cranelift pin `5df9a089753b520a47251c635e99be4466b4e7f6`
was used for the compiler build. Every case had a five-second compile budget.
Results and exact commands: /tmp/cosmicc-c-testsuite-reviewed-final/summary.json,
summary.tsv and per-case logs in the same directory.

## Architectural execution results

**All 127 compiled candidates passed on LightingMachine**, totaling **8,044,764
guest instructions**, with zero return values, restored stacks and no architectural
traps. The prime-count case 00041.c uses 8,032,661 instructions; an initial five
million instruction budget was insufficient, so the maintained bounded budget is
20 million per case. This was budget exhaustion, not a demonstrated compiler bug.

Evidence: /tmp/cosmicc-c-testsuite-reviewed-final-native.tsv and matching .log.
Driver: tools/l21-execution/src/bin/compatibility-native.rs. It reads the emitted
bundles, links main and executes guest instructions without host library callbacks.
All these candidates have no main arguments and empty upstream expected output.
The current native tool uses the local LightingSimulation dependency checkout;
no composed SoftwareCpuBoard/MainboardFPGA acceptance is claimed here.

## Concrete next fixes

1. **00078.c: block-scope function prototype identity.** A local declaration
   `int f1(char *);` inside main refers to the existing external function definition,
   but lowering reports no translation-unit declaration for its call target.
   This is a small actionable symbol-identity fix without runtime dependencies.
2. **00096.c / 00136.c: tentative declarations.** Repeated tentative globals
   produce duplicate bundle definitions rather than one C object.
3. **00128.c: narrow integer conversion closure.** An integer conversion matrix
   reaches unsupported uextend.i16.
4. **00141.c / 00145.c / 00152.c: preprocessing.** Nested token-paste prescan,
   short-circuit #if expressions, and macro-expanded #line directives expose
   concrete frontend boundaries.
5. C11 anonymous members (00046/00050), tag/declaration scopes (00129), wide
   character literals (00098), array parameter qualifiers (00162), compound
   literals (00149/00150), implicit main return/control flow (00051), aggregate
   varargs/ABI (00140), qualified null-pointer conditional behavior (00144), and
   floating point (00113/00119/00123) need individually checked implementation
   slices. The listed diagnostics require standards review before labeling every
   rejection a compiler bug.

These 20 compiler boundaries remain part of the baseline. Case numbers are stable
only together with the pinned suite commit. Wider hosted cases need target runtime
and I/O first; a broader GCC torture inventory is now recorded below as a
regression corpus, not a standards certificate.

Source-integrity rejection was exercised by changing 00001.c in a disposable copy:
the runner rejected its hash before compilation. Log:
/tmp/cosmicc-c-testsuite-integrity.log. Inventory-only validation confirms exactly
147 selected and 73 excluded cases.

Primary sources: [c-testsuite interfaces and tags](https://github.com/c-testsuite/c-testsuite),
[GCC testsuite documentation](https://gcc.gnu.org/onlinedocs/gccint/Testsuites.html).

## Updated target-specific differential inventory (2026-10-05)

The baseline above records its earlier compiler snapshot. The newer all-220
native AMD64 differential matrix, including Cosmic/GCC13/Clang18 at O0 and O2,
is documented in [DIFFERENTIAL_STATUS.md](tools/compatibility/DIFFERENTIAL_STATUS.md).
It keeps the 147 reviewed-profile and 73 outside-profile denominators distinct,
uses exact upstream combined execution output, and records compiler/source hashes.
Reference consensus supports investigation; it does not certify defined ISO C.
Case 00144 has an explicit undefined-behavior/constraint annotation.

The short-circuit #if boundary 00145 is now fixed: unused operands/ternary arms
are not evaluated during constant folding. Narrow array indices now scale in
the target pointer-width domain; this also fixed the unmodified zlib Huffman-table
corruption. Both changes have focused regressions and target-specific execution
checks. The earlier actionable list should be read as historical evidence.

Source-frozen SIA reviewed-profile results: 128/147 compile and all 128 pass
LightingMachine architectural execution, totaling 8,044,783 guest instructions.
Reports: `/tmp/cosmic-sia-source-frozen-suite/summary.json` and
`/tmp/cosmic-sia-source-frozen-execution.tsv`. This establishes architectural
execution only; composed board, protected loader and hosted runtime remain separate.

Final source-frozen native AMD64 matrix: Cosmic O0 and O2 each pass 138/220,
including 136/147 reviewed cases and 2/73 outside-profile cases. GCC 13.3.0 and
Clang 18.1.3 each pass 220/220 at both O0 and O2. Cosmic failures are compile
rejections; no link, execution or output mismatch occurred. Report:
`/tmp/cosmic-differential-source-frozen/summary.json`; compiler SHA256:
`7e7f64f3bd7f6bf2794b2b82d2639f7adfc7dd09fd24cab66652fbff4fc631d3`.

## SSA/variadic continuation — intermediate release snapshot

Compiler SHA256 `a223f80c4c6bf9ecc0933e63073d15201dd4c4820dee98374c1f008752c2a383`.
SIA reviewed profile compiles 137/147 and all 137 pass LightingMachine
architectural execution (8,044,273 guest instructions). Raw compile inventory
and execution TSV are retained in `tools/compatibility/results/2026-10-05-sia-ssa-varargs-*`.
The remaining ten SIA compile rejections are 00051, 00078, 00096, 00113, 00119,
00123, 00128, 00136, 00140 and 00144. This target still lacks complete I64/FP
and aggregate/variadic ABI support; native AMD64 fixes do not establish them.
Current board execution requires an unconfigured production CPU RTL bridge.

The same release passes native O2 ELF/ABI acceptance and the unchanged nine-core
zlib closure with 16 native roundtrips plus allocation refusal. The SIA CLI
checked-linked container roundtrip passes 875,907 architectural instructions
(171,828 image bytes). All are separately recorded; neither is full zlib upstream
acceptance. Durable JSON reports accompany the new compatibility results.

## Final operator-corrected release

SHA256 `60e33b787f7d6350ad66d4246e940a71648780908719448234af2193c9639835`.
All 137/147 compiled SIA reviewed cases pass architectural execution, totaling
11,311,259 guest instructions. SIA bounded zlib linked-container roundtrip passes
1,028,362 instructions, with 180,172 image bytes. Correct C int result typing
changes generated code and counts relative to the intermediate snapshot above.
Native O2 ELF/ABI and zlib acceptance, all eight variadic and eight MEMORY
aggregate interop combinations, Bool O0/O2/Os and float-to-narrow O0/O2/Os
regressions pass with this same release. Durable JSON and SIA execution TSV are
in `tools/compatibility/results/2026-10-05-*-operators-final*`.

Final unchanged native matrix: Cosmic O0/O2 each **199/220** (reviewed 146/147,
outside-profile 53/73), versus GCC13/Clang18 each 220/220. All 21 Cosmic failures
are compile rejections; no accepted program fails linking, execution or expected
output, and no compiler panic/timeout occurs. See final raw differential JSON/TSV
and [current report](tools/compatibility/DIFFERENTIAL_STATUS.md).

## SIA integer, software floating-point and ABI continuation (2026-10-05)

**146/147 reviewed cases compile and all 146 execute** on LightingMachine,
totaling 11,315,694 guest instructions with four explicitly linked integer-only
runtime objects. The remaining 00144 diagnostic rejects a qualifier violation;
the fixture also reads uninitialized locals. Reference compiler acceptance does
not establish defined behavior.

Implemented: I64 arithmetic and pair/stack ABI, binary32/binary64 arithmetic,
comparisons and signed/unsigned 32/64-bit conversions, copied aggregate
arguments/hidden returns, and packed incoming/outgoing variadic arguments with
standard target stdarg macros. A discovered high-bit integer-to-bool conversion
bug was fixed; its independent actual C fixture passes.

Production CPU/MMU/module RTL with real MainboardFPGA in Bluesim passes the
ordinary double fixture (72,249 instructions), cross-TU aggregate/varargs
fixture (20,706), and focused aggregate fixture (3,877). Guest results are zero,
stack restored, and no trap occurs. This is bounded production RTL acceptance,
not physical FPGA or complete Cosmic boot qualification.

The binary64/conversion runtime matches unchanged integer SoftFloat3e in
2,105,005 bit-and-fresh-flag checks; 1,169 saved guest vectors pass on
LightingMachine. Explicit context isolation and sticky accumulation are tested.
Generated ordinary C helper calls discard their per-operation flags: C fenv,
long double, decimal I/O and libm remain open. Aggregate/varargs use private
Cosmic SIA conventions; external ABI interoperability requires agreement.

Full workspace gates pass: parser 140, preprocessor 19, target models 13,
SIA 151, native 11, doctests 5 (two ignored). Native O0/O2 differential replay
retains **199/220**, reviewed **146/147**; GCC13 and Clang18 each pass 220/220
at both optimization levels. All Cosmic native failures are compile rejections.
Native ELF/ABI and eight outgoing variadic interoperability combinations pass.
AMD64 incoming stdarg remains an unvalidated separate prototype.

Compiler snapshot SHA256:
`a779a61517b2ba2e6d39432a1a13081622022451da1bed5de3ad9ef194622f0b`.
Raw compilation/execution/differential/gate evidence:
`tools/compatibility/results/2026-10-05-sia-wide-*`; production logs and
bridge/object/source hashes: `tools/sia/results/2026-10-05-wide-*`.
See [float scope](SIA_FLOAT_LANGUAGE_STATUS.md),
[aggregate/variadic ABI](tools/sia/AGGREGATE_VARIADIC_ABI.md) and
[I64 scope](saltwater-sia/I64_STATUS.md).

## Current runtime-continuation SIA replay (2026-10-05)

The frozen frontend/runtime continuation still compiles and executes **146/147**
reviewed cases: 146 guest passes, 11,315,855 instructions, no traps or execution
failures; case 00144 remains the explicit constraint/undefined-behavior diagnostic.
[Retained inventory and execution](tools/compatibility/results/2026-10-05-sia-fenv-final/summary.json).
The integer-only numerical runtime also passes the full C99 math API fixture
(554,577 instructions) and 450 reference vectors across 25 fdlibm function
families (13,296,432 instructions). These additional fixtures are independent
of the unchanged 147-case denominator; physical FPGA execution is not implied.

## Final C11 generic-selection native replay

The frozen compiler passes **211/220** unchanged upstream expected-output
cases at both O0/O2, with nine compile rejections and no accepted execution
failures. GCC13 and Clang18 each pass 220. `_Generic` case 00219 passes all six
compiler/optimization modes. The 147-case reviewed denominator remains 146
passes; the outside-profile subset is 65/73.
[Native per-case evidence](tools/amd64/results/2026-10-05-native-generic-220.tsv).
AMD64 long-double/x87 interoperability remains unsupported; other recorded
rejections include C constraints and GNU-only language extensions.

Final post-_Generic SIA replay also passes **146/147**: all 146 emitted
candidates execute successfully, totaling 11,315,804 guest instructions.
[Current SIA execution summary](tools/compatibility/results/2026-10-05-sia-generic-final/execution-summary.json).
The separate generic-language fixture passes production RTL in 595 instructions;
post-change decimal and C99 math architectural fixtures pass. Pre-_Generic
numerical production results retain their earlier compiler provenance.

## Pinned GCC torture compatibility inventory (2026-10-05)

The pinned GCC `gcc.c-torture/execute` profile selected 1,216 of 1,684 direct
`.c` cases with standalone sources and a `main`. GCC 13.3 passed 1,154 selected
references; Cosmic compiled, linked and passed 587 on LightingMachine. The
remaining GCC-accepted cases comprise 528 compiler rejections, 11 linker
failures and 28 execution failures. GCC itself rejected 29 and had 33 execution
failures. The cases are useful concrete triage leads, but include GNU extensions
and target-sensitive/undefined programs; these totals are not an ISO C score.

The sources are pinned to `rust-lang/gcc` revision
`6f155cc3f5a2dff33afe6cc3ed6c2e0e605ae6a3`; source hashes, compiler hashes,
per-case status and durations are in the [raw JSON report](tools/compatibility/results/2026-10-05-gcc-torture-sia/summary.json)
and [run notes](tools/compatibility/results/2026-10-05-gcc-torture-sia/README.md).
Execution was on the LightingMachine simulator, not production RTL.

## SIA ELF object and executable validation (2026-10-05)

Cosmic C emits ELF32 little-endian SIA ET_REL objects with relocations and
NOBITS BSS; `cosmic-link` accepts these directly and from standard `ar`
archives. It emits static ELF32 ET_EXEC with PT_LOAD segments and zero-file-size
BSS. `readelf` inspection, the checked executable loader, and 46-instruction
LightingMachine execution passed. The external `rust-sia` linker also accepted
the object/archive input. ELF and suite logs are retained in
[`tools/sia/results/2026-10-05-sia-elf`](tools/sia/results/2026-10-05-sia-elf).
This private SIA ELF machine is not supported by ordinary GNU/LLVM host linkers,
and the simulator result does not establish a protected-OS or physical-board
loader contract.

## Pinned Csmith differential run on SIA (2026-10-05)

The Csmith 2.4.0 generator at pinned revision
`0cdc710315cfee9035e22ef4363ca479270d1934` produced 32 reproducible integer-
focused seeds. All 32 GCC 13.3 references compiled and ran. Nineteen compiled
and matched their checksum on SIA/LightingMachine; Cosmic rejected 13 during
compilation, chiefly nonconstant global scalar initializers and pointer type /
qualifier handling. No SIA runtime mismatch occurred among accepted programs.
The exact seed set, generated-source hashes, checksums, tools and runtime hashes
are in the [raw run report](tools/compatibility/results/2026-10-05-csmith-sia/summary.json)
and [run notes](tools/compatibility/results/2026-10-05-csmith-sia/README.md).

Csmith is a random defined-behavior bug-finding complement, not a standards
conformance score. For a standards-directed claim, the recommended next step is
to select a standard and target profile, then evaluate a licensed conformance
suite. Plum Hall CV-Suite currently lists C11/C17/C23 and documents how to
configure a freestanding test output hook for an emulator/simulator. A full run
requires licensed suite files and a Cosmic-specific configuration; none are
present in this checkout.
