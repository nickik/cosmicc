# Actual targets and output formats — 2026-10-05

The maintained Cosmic C driver now supports SIA32 and amd64 Linux. It uses a new
maintained native backend; retired src/main.rs remains excluded from Cargo.
Cranelift architecture availability alone does not establish a Cosmic C target.

| Target | Compiler output | Executable linking |
| --- | --- | --- |
| sia32-unknown-none (default) | COSMIC-SIA bundle, including with a .o filename | cosmic-link produces raw RAM bytes or CSIAIMG container |
| x86_64-unknown-linux-gnu / amd64 | ELF64 little-endian x86-64 ET_REL with -c | Maintained driver invokes system cc for native ELF executable |
| aarch64 | Not implemented | No supported Cosmic C path |

## Verified amd64 acceptance

The tools/amd64/check-native.py gate passed with compiler SHA-256
`60e33b787f7d6350ad66d4246e940a71648780908719448234af2193c9639835`.
Explicit Cranelift speed (`-O2`) evidence: /tmp/cosmic-native-operators-final/summary.json and per-command logs.
Actual objects were ELF64 ET_REL, machine 62, flags 0; readelf recognized standard
R_X86_64_GOTPCREL relocations. External system-cc non-PIE linking executed
successfully. The Cosmic driver compiled two C inputs with a system-built harness
object, linked using default PIE, and executed; ELF ET_DYN was verified. A further
driver link resolved a helper from an ar archive through -L/-l and executed.
Missing-helper linkage failed as expected.

Coverage: LP64 sizes; mixed integer/pointer arguments exceeding register capacity;
64-bit division/remainder/shifts; bidirectional system-C interoperability; indirect
calls; arrays/global state; separate translation units with matching static names;
narrow signed/unsigned arguments and returns; native double arithmetic/returns
with nine floating arguments and mixed calls. The independent unchanged
reviewed-target fixture also passed aggregate padding, static locals,
member/array-address relocations, pointer differences, casts, 64-bit integer/double
conversions, NaN truth/comparisons and negative-zero truth.

The complete 220-case pinned c-testsuite corpus was compared with GCC 13.3.0
and Clang 18.1.3 at O0/O2. Cosmic now passes 211/220 at both optimization modes;
the reviewed freestanding profile passes 146/147. Both references pass 220/220
at both levels. Cosmic's remaining 21 outcomes are compile rejections; no
compiler/native crashes, link failures or output mismatches remain. Three corpus
cases have identified undefined-behavior/constraint boundaries; two still count
as raw expected-output passes. Reference acceptance does not prove valid ISO C.
Source and compiler hashes, commands and every result are retained in
/tmp/cosmic-differential-ssa-varargs-final2/summary.json and durable compressed
JSON/TSV. See [compatibility interpretation](tools/compatibility/DIFFERENTIAL_STATUS.md).

Real zlib 1.3.2 acceptance also passed with the same compiler snapshot. Nine
unmodified Z_SOLO translation units were Cosmic-compiled, system-linked to a
host C harness, and executed 16 compress/decompress roundtrips (levels 0/1/6/9,
lengths 0/1/31/4096), comparing exact output bytes. The algorithm implementations
came from Cosmic-produced objects; no host zlib serviced compression/decompression.
The host harness supplies allocation callbacks and native process startup. This
is native-host library acceptance, not a Lighting zlib port or full upstream
zlib test suite. Evidence: /tmp/cosmic-native-zlib-operators-final/summary.json.
The final native ABI, full differential matrix and optimized zlib gates share
the compiler hash above, after the scalar SSA, variadic/aggregate parameter, operator-type,
FP-to-narrow-cast and pragma fixes.

These are bounded executable tests, not C-standard conformance certification.
Native scalar outgoing System V variadic calls and named-only variadic
definitions also pass eight Cosmic/GCC/Clang O0/O2 interoperability combinations,
including AL capture, default promotions, mixed register/stack arguments,
indirect calls and snprintf. Consuming `va_list` remains unsupported. Natural
MEMORY-class by-value parameters larger than 16 bytes pass an independent
eight-combination matrix; small register-class aggregates and aggregate returns
remain unsupported. These gates do not establish a complete hosted sysroot or
shared-library compatibility. These host-native
results do not claim Lighting execution. Native amd64 FP does not implement
software FP for SIA.

## SIA formats and separate private ELF capability

COSMIC-SIA v4 records code/data, explicit TU-local bindings and relocations, not
ELF. cosmic-link resolves checked Abs4 references across independent v4 TUs at
an explicit base/entry; locals and strings are scoped by linker input ordinal.
Duplicate external/common definitions and unresolved imports are diagnosed;
archives and weak/common coalescing are unsupported. Legacy v3 input remains
single-bundle only. Default output is flat guest RAM bytes; --container emits CSIAIMG
version 1 with address/entry/region/symbol metadata. Neither is an ELF executable
or establishes protected Cosmic loading/process startup.

Independent local shared-backend work can emit SIA ELF32 little-endian ET_REL
with project-private machine 0xff53 and REL type0x80 (ABS32). It patches generic
ELF32 structural-writer metadata; no x86 identity/type appears in the final SIA
object. REL stores signed-i32 A in the little-endian patch word; the SIA-aware
linker computes checked unsigned32 S+A. Only Abs4/signed-i32 addends are accepted.
The three focused local crainlift-object tests passed for identity, private REL
negative-addend encoding and overflowing-addend rejection; log:
/tmp/cosmic-target-audit/object-tests.log. This distinct local ELF work is not
included in the minimal published backend slice or exposed through Cosmic C CLI.

The inspected /tmp/sia-panic-probe.o was actual ELF32, private machine 0xff53,
flags 0 and private REL 0x80, with Rust-mangled symbols; it does not prove Cosmic C
ELF emission. GNU ld -r rejected it as generic ELF/wrong format. The separate
rust-sia/scripts/sia_link.py understands the private contract and emits simulator
ROM/RAM images and JSON; its harness starts a selected function, not Cosmic boot.
Its parser does not currently validate e_flags. This audit inspected that path;
it did not rerun full private-ELF loader/execution acceptance.

## Reproducing native checks

From cosmicc:

```sh
python3 tools/amd64/check-native.py --compiler target/debug/cosmicc \
  --output /tmp/cosmic-amd64-native-new
python3 tools/amd64/check-c-testsuite.py /path/to/pinned/c-testsuite \
  /tmp/cosmic-amd64-suite-new --compiler target/debug/cosmicc --limit 147
```

Output directories must be new. See tools/amd64/README.md for evidence boundaries
and primary GCC/GNU ld documentation. The tiny historical cc-link wrapper is not
a substitute for the new target-owned LP64 lowering, native ABI and ELF backend.

## SIA software floating-point and ABI continuation (2026-10-05)

C float uses I32 bits; double uses I64 register pairs. Integer-only runtime
calls implement arithmetic, comparisons and all signed/unsigned 32/64-bit
conversions. Aggregates use copied address parameters/hidden return storage;
varargs use an explicit packed hidden pointer. All consumers must use the same
private Cosmic SIA conventions. See [float scope](SIA_FLOAT_LANGUAGE_STATUS.md)
and [aggregate/variadic ABI](tools/sia/AGGREGATE_VARIADIC_ABI.md).

Production CPU/MMU/module RTL and real MainboardFPGA Bluesim execute the actual
C double fixture in 72,249 guest instructions. Cross-TU aggregate/varargs execution
passes 20,706 instructions; this is bounded RTL acceptance, not physical FPGA
or complete Cosmic boot. COSMIC-SIA/CSIAIMG remains a private object/image
contract, not standard ELF. SIA long double is distinct in C but has binary64
precision; fenv, four rounding modes, C99 real math families and decimal memory
text I/O are implemented. These checks do not qualify physical FPGA execution.
