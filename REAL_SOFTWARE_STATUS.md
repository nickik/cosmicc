# Real software acceptance — 2026-10-05

## zlib: reproducible unmodified-source probe

Fetched upstream zlib 1.3.2 from its official fossil URL and checked its archive
SHA-256 against the official site:
`bb329a0a2cd0274d05519d61c667c062e06990d72e125ee2dfa8de64f0119d16`.
The source stays in `/tmp/zlib-1.3.2`; it is not vendored or modified.
`tools/ports/probe-zlib.py` records commands and source hashes for the upstream
Z_SOLO core configuration, using no host configure/header/library substitution.

The original pinned-backend inventory compiled only crc32.c and zutil.c.
Frontend and lowering fixes now pass the former macro, pointer, narrowing and
static-initializer boundaries. The old backend then exposed two real failures:
reserved frame-pointer corruption on nested returns, and nonterminating backward
branch veneers. The local isolated backend fix is retained in
`tools/backend-patches/sia32-frame-and-branches.patch`.

With that patch applied to published backend base
`9edf4ede3728b51c2abf21ce4686bd4ed0a391ae`, all nine core translation units
compile: adler32, crc32, deflate, infback, inffast, inflate, inftrees, trees and
zutil. Automatic signed/unsigned I64 remainder now uses integer-only restoring
legalization; deflate passes after narrow-local initializer coercion. Inventory: `/tmp/cosmic-zlib-auto-rem/summary.json`
(the console log is `/tmp/cosmic-zlib-auto-rem.log`). These are individual objects,
not a linked compression port.

The expanded CRC fixture includes unmodified upstream crc32.c. It checks empty
input, known vectors, incremental updates and patterned data around eight-byte
and 256-byte boundaries. It passes on **LightingMachine** and the public
**SoftwareCpuBoard → MainboardFPGA → RAM** path: 26,430 guest instructions,
zero result and exact stack/frame-pointer restoration with the isolated backend.
Logs: `/tmp/crc-isolated-backend.log`, `/tmp/crc-isolated-board.log`.
The ordinary compiler/CLI linked known-vector wrapper also passes architectural
execution (388 instructions; `/tmp/crc-ordinary-native.log`). No host CRC callback
is used to compute guest results.

The isolated backend commit is `5df9a089753b520a47251c635e99be4466b4e7f6`.
Its 40 SIA tests pass. The user approved publication; the fix is now on the
cosmicc-frame-preservation branch at nickik/crainlift, and the product manifests
and lockfile pin that exact fetchable revision. Published-pin workspace validation passes: 135 parser, 19 preprocessing,
150 lowering/image tests and five doctests (two historical ignores). The native
execution tool now uses the published backend pin; its Lighting adapter retains
its existing sibling patch. CRC passes both Lighting paths at 26,430 instructions.

Initial dependency checklist (superseded by the linked acceptance below):

- Resolve remaining native I64 arithmetic and core compilation boundaries.
- Support multiple translation units with TU-local/static identity; the current
  one-bundle linker cannot merge the separate emitted objects.
- Z_SOLO requires guest allocation/free callbacks supplied in z_stream. Use a
  checked target arena/allocator and exercise allocation failures; it deliberately
  removes zlib's default hosted allocation and gzip file APIs.
- Link zlib's own zmemcpy/zmemcmp/zmemzero from zutil.c and its internal helpers.
  The integer-only compression core does not require software floating point.
- Run a guest round trip and upstream algorithm tests on Lighting before claiming
  a zlib port. Gzip file operations separately require Cosmic file I/O and error
  handling.

## Other acceptance targets

The e2fsprogs 1.47.2 broad TU inventory and current diagnostics are in
FRONTEND_PORTABILITY_STATUS.md. Before acceptance, select the actual ext2
read/write dependency closure, resolve all imports, add target allocation/error
and block-device adapters, then verify persistence using independent byte
comparison and e2fsck. 31 compiled TUs do not constitute an ext2 port.

SQLite's first bounded goal remains in-memory operation: signed I64 and numeric
conversions, allocation, string/error/runtime functions, linking, and an explicit
threading/locking policy precede persistence. Cosmic VFS read/write/truncate,
sync and locking are later requirements. No SQLite source probe or native test
was performed in this slice.

Lua requires a chosen lua_Integer/lua_Number ABI, software floating point for
its usual numeric configuration, math functions, allocation, setjmp/longjmp and
error unwinding, string/runtime functions, and Cosmic I/O for a hosted shell.
No Lua source probe or native test was performed in this slice. These are
prerequisite plans rather than measured compiler diagnostics.

Official reference sources: https://zlib.net/ , https://www.sqlite.org/vfs.html ,
https://www.lua.org/manual/5.4/ .

## amd64 zlib execution — 2026-10-05

All nine unmodified upstream Z_SOLO core files emit standard ELF64 through the
maintained amd64 target, link using system cc and execute successfully. Sixteen
compression/decompression round trips cover four levels (0/1/6/9) and four
lengths (0/1/31/4096), with exact byte comparison and an allocation-refusal check.
Only the test harness is compiled by system cc; algorithms are Cosmic C output
and no host zlib library is linked. Reproducible gate: tools/amd64/check-zlib.py.
Evidence: /tmp/cosmic-amd64-zlib-index-fixed/summary.json, with immutable compiler
snapshot and source hashes.

This acceptance found and fixed a shared frontend narrow-index scaling bug:
unsigned-char index times four wrapped before address addition. Indices are now
converted to pointer width before scaling. The 256-entry/signed-negative-index
regression passes both SIA execution paths (22,268 instructions). SIA linked acceptance is recorded below; full upstream and hosted file-I/O
coverage remains separate.


## SIA linked zlib execution — 2026-10-05

All nine unchanged zlib 1.3.2 Z_SOLO sources plus a guest harness now compile as
separate v4 objects and link with cosmic-link. The exact emitted CSIAIMG container
is decoded and executed by LightingMachine. One 31-byte level 1 roundtrip, with
windowBits 9 and memLevel 1, passes in 891,924 guest instructions, with zero result,
no architectural trap and restored stack. A bounded guest RAM allocator and
C zeroing loops supply the callbacks; no host allocator or host zlib performs
algorithm work. Reproduce with tools/sia/check-zlib.py; current evidence:
/tmp/cosmic-sia-zlib-source-frozen/summary.json and
/tmp/cosmic-sia-zlib-source-frozen-container-replay.log.

This establishes a linked target-native compression slice, not the full upstream
zlib suite, gzip file I/O, protected loading, or composed-board zlib execution.
The two-TU linker fixture separately passes both Lighting paths in 153 instructions.

The final release amd64 compiler also passes all 16 roundtrips and allocation
refusal at explicit Cranelift speed (`-O2`), plus native ELF/ABI/driver checks.
Evidence: /tmp/cosmic-zlib-source-frozen-o2/summary.json and
/tmp/cosmic-native-source-frozen-o2/summary.json. Compiler SHA-256:
`7e7f64f3bd7f6bf2794b2b82d2639f7adfc7dd09fd24cab66652fbff4fc631d3`.
