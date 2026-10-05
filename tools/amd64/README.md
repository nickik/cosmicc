# amd64 acceptance tools

These gates compile C using Cosmic C's x86_64-unknown-linux-gnu target, inspect
actual ELF64 little-endian x86-64 relocatable objects, link using the existing
system cc driver, and execute on the native amd64 host. They do not execute SIA
or claim Lighting acceptance. Output directories must be new; the scripts retain
an immutable compiler snapshot/hash, commands and logs.

```sh
python3 tools/amd64/check-native.py --compiler target/debug/cosmicc \
  --output /tmp/cosmic-amd64-native
python3 tools/amd64/check-c-testsuite.py /path/to/pinned/c-testsuite \
  /tmp/cosmic-amd64-suite --compiler target/debug/cosmicc --limit 32
```

check-native checks LP64 sizes, mixed int/long/pointer arguments including stack
arguments, calls from system C to Cosmic C and back, indirect function pointers,
64-bit signed/unsigned arithmetic, signed/unsigned char and short arguments,
narrow returns in both call directions, native double arithmetic/returns including
nine floating arguments and mixed integer/floating calls with stack arguments,
arrays/stride, initialized globals, externs,
separate Cosmic translation units with identically named static globals, and
linker rejection of an absent external helper. It also uses the maintained
compiler driver to link two C inputs with a system-built harness object using
default PIE, verifies ELF ET_DYN, and executes it. A separate driver link resolves
the helper from a static archive through -L/-l and executes that program. readelf symbols/relocations and
ELF header identity/flags are retained. A system-cc-only reference build of the
fixture passed; this verifies the fixture, not Cosmic output acceptance.

The independent reviewed-target fixture adds aggregate padding, static locals,
static member/array-address relocations, pointer difference and casts, 64-bit
integer/double conversions, NaN truth/comparisons and negative-zero truth tests.
It is compiled unchanged and linked to a separate system-built harness.

The c-testsuite gate verifies every file against the maintained pinned inventory,
selects the first --limit freestanding cases in manifest order, and separately
records compilation, system linking and native execution failures. Exit zero and
exact expected stdout are required. The upstream inputs are unmodified. This is
a bounded slice, not whole-suite or strict C-standard conformance.

## System linker integration

The retired Saltwater link helper simply invokes cc(object,-o,output); reusing
that tiny wrapper alone does not restore host target support. Keep the new target's
frontend layout, ABI lowering and ELF object emission in the maintained pipeline.
Use the cc driver for Linux startup objects, libraries and linker selection; direct
ld invocation would require specifying that platform machinery explicitly.
The baseline interoperability link explicitly uses -no-pie. The maintained
driver link separately exercises Linux default PIE using PIC objects and requires
ET_DYN output. Shared-library support needs its own compatible codegen and tests.

Primary documentation: [GCC linking options](https://gcc.gnu.org/onlinedocs/gcc/Link-Options.html)
and [GNU ld options](https://sourceware.org/binutils/docs/ld/Options.html).
GCC documents -c as stopping before linking and -no-pie as selecting a non-PIE
executable; successful native linking still depends on an appropriate target ABI
and ELF relocation implementation. No new linker is needed for ordinary amd64
ELF interoperability.

## Local acceptance — 2026-10-05

The enhanced native gate passed, including the independent reviewed-target fixture,
non-PIE external linking, default-PIE driver linking and archive selection. Logs:
/tmp/cosmic-amd64-native-final-index/. The complete 147-case reviewed freestanding
selection passed 135, with 11 existing frontend diagnostics and one explicit
aggregate-by-value/variadic ABI boundary. All 127 SIA baseline passes were retained;
compiled cases had no linking/execution failures. Reports:
/tmp/cosmic-amd64-suite-147-final-index/summary.json. Both runs used compiler hash
`df3d14cd7ca6e4539c4cbe84495f99f7fc80f3c987dc3f116c6468791975e866`.
See ../../TARGET_OUTPUT_STATUS.md for the current target/output distinction.

## Unmodified zlib native acceptance

```sh
python3 tools/amd64/check-zlib.py /tmp/zlib-1.3.2 /tmp/amd64-zlib-acceptance
```

This builds all nine unmodified upstream Z_SOLO core translation units with
Cosmic C, records their source hashes and a compiler snapshot, then uses system
cc only to compile the harness and link those objects. No host zlib library or
host-compiled zlib algorithm is linked. Sixteen round trips cover levels
0/1/6/9 and input lengths 0/1/31/4096; allocation refusal is separately checked.
The accepted run is /tmp/cosmic-amd64-zlib-index-fixed/summary.json.
These are native amd64/Linux results, not Lighting/SIA compression acceptance.

The same final compiler passed the root's tools/amd64/check-zlib.py gate: nine
unmodified zlib1.3.2 Z_SOLO units and16 exact compress/decompress roundtrips
(levels0/1/6/9; lengths0/1/31/4096). Host C provides allocation callbacks/startup;
compression/decompression executes Cosmic-produced algorithms, with no host zlib.
Report: /tmp/cosmic-amd64-zlib-index-fixed/summary.json. These final native gates
were rerun after the shared narrow-index scaling correction.


Acceptance scripts `check-native.py` and `check-zlib.py` accept
`--optimization O0`, `O2`, or `Os` (default O0). They snapshot the actual compiler
binary and record the selected generated-code policy, so optimized acceptance
can be reproduced without a wrapper script.

`check-variadic.py` tests scalar outgoing System V variadic calls and named-only
variadic definitions at Cosmic O0/O2 against GCC13/Clang18 O0/O2. It captures AL
at host ABI entry and validates default promotions, mixed register/stack args,
callbacks and snprintf. `va_list` consumption remains unsupported.

`check-aggregate-memory.py` uses the same eight-combination matrix for by-value
MEMORY aggregate parameters larger than16 bytes with natural scalar/array layout.
It tests padded17-byte and mixed32-byte objects, fixed and ellipsis arguments,
and mutations that leave caller originals unchanged. Small register-class
aggregates and aggregate returns remain unsupported. Both scripts require a new
`--output` directory and `--compiler` path; results are native AMD64, not Lighting.

Incoming stdarg and aggregate ABI continuation gates:

```sh
python3 tools/amd64/check-stdarg.py --compiler target/debug/cosmicc --output /tmp/stdarg-new
python3 tools/amd64/check-aggregate-register.py --compiler target/debug/cosmicc --output /tmp/register-new
python3 tools/amd64/check-aggregate-return.py --compiler target/debug/cosmicc --output /tmp/return-new
```

Each gate executes eight Cosmic/host optimization combinations plus four GCC/Clang-only references. Supported classes and limits are documented in saltwater-amd64/STDARG_STATUS.md and REGISTER_AGGREGATE_STATUS.md.

check-unused-vla.py checks preserved bound side effects and fixed-array storage across a forward goto at Cosmic/GCC/Clang O0/O2. Referenced dynamic array storage remains unsupported; this is a bounded elimination gate, not general VLA acceptance.
