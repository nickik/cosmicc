# Linking and loading first slice — 2026-10-04

Added an opt-in versioned **local transport container**, not Cosmic's protected
executable contract. `cosmic-link --container` writes it; the default remains
raw relocated RAM bytes. `LinkedImage::to_image_bytes`, `from_image_bytes` and
`validate_image` expose checked transport through the existing public type.

## Format v1

All integers are little-endian u32. The header is eight bytes `CSIAIMG\0`,
followed by version=1, base, entry, payload length, region count and symbol count.
Each region stores address, size, flags (bit 0 read-only, bit 1 executable), then
name length and UTF-8 name. Each symbol stores address, name length and UTF-8
name. Relocated payload follows. Symbols serialize in BTreeMap order; regions
retain deterministic linker layout order. Unknown versions/flags and trailing
bytes are rejected. No external imports, relocation processing, compression,
cryptographic authenticity or checksum are implied by this transport format.

Decoding requires explicit independent encoded-file and guest-payload budgets.
Before allocation it rejects impossible metadata counts. Bounds-checked reads
reject truncation and invalid names; validation rejects empty payloads,
misaligned bases/entries, overflowing address ranges, region bounds, duplicate
region/symbol names, overlapping nonempty regions, writable executable regions,
and entries outside executable storage. Permissions remain intended metadata.
Parsing never maps guest memory or executes guest instructions.

## Verification

Three focused tests pass, covering a compiler-produced image round-trip, deterministic serialization, independent budgets,
every-byte truncation, trailing data, bad version/count/flags/entry, duplicate
regions and invalid executable/address metadata. Log:
`/tmp/cosmic-image-format.log`. These are host-side codec tests; no protected
loader execution claim is made. Optimized cosmic-link release build passed; log /tmp/cosmic-link-format-release.log.

## Multi-object static linking — 2026-10-05

COSMIC-SIA bundle v4 stores an explicit set of TU-local definitions before its
function/data tables. Compiler metadata comes from C storage classes, static
locals and string pools. File-scope static prototypes and definitions share a
name within their TU. Linker input ordinals scope these definitions; relocation
references to locals remain in the originating TU. External definitions retain
C linkage names. Synthesized local names cannot collide with external imports.
No static or string identity is inferred from name prefixes.

`Artifact::link_images` and `cosmic-link ... A.sia B.sia` resolve checked Abs4
relocations across TUs using deterministic layout. Duplicate external definitions,
unresolved references, overflowing addends, invalid metadata/layout and missing
entry fail. Tentative definitions allocate zero-filled storage and count as
strong definitions; duplicate common/tentative and weak coalescing are unsupported
and diagnosed. Declaration-only extern objects allocate no storage. Data-only
TUs are supported. v3 bundles remain decodable and single-bundle linkable;
multi-object linking rejects their unknown bindings and requests recompilation.

Nine focused image/codec tests and the full 153-test SIA suite pass, including
the additional missing-static regression. Referenced undefined TU-local symbols
are diagnosed; unused static prototypes allocate no storage. A fixture
with two independently serialized TUs, duplicate static function/data names,
separate string pools and cross-TU calls passes LightingMachine and the composed
board: 153 instructions, zero return, restored stack. Logs:
/tmp/cosmic-multi-object-native.log and /tmp/cosmic-multi-object-board.log.

Default output remains flat RAM; opt-in CSIAIMG v1 is a checked local transport.
Archives, weak/common rules, dynamic imports, ELF input/output, protected loading,
read-only enforcement, capability resolution and process startup remain open.
The separate rust-sia/scripts/sia_link.py consumes private ELF32 machine 0xff53,
REL 0x80 objects/archives; it is not integrated into this bundle linker. System
linker interoperability is not claimed.

## Target-native zlib execution

All nine unchanged zlib 1.3.2 Z_SOLO core sources plus a guest harness compile to
v4 bundles, link through the actual cosmic-link CLI and CSIAIMG container, decode
through the public checked image reader, and execute on LightingMachine.
A 31-byte level 1 compress/decompress roundtrip passes with windowBits 9 and
memLevel 1: 891,924 guest instructions, zero result and restored stack. The guest
supplies bounded fixed-RAM allocation callbacks and explicit target-native
zeroing; no host zlib or host allocator services the target. Evidence and source/
compiler/linker hashes: /tmp/cosmic-sia-zlib-source-frozen/summary.json. This is one
architectural roundtrip, not a full zlib upstream suite or composed-board zlib
acceptance. The two-TU board proof above remains the composed-board linker gate.

Reproduce from cosmicc with a new output directory:

```sh
python3 tools/sia/check-zlib.py /path/to/zlib-1.3.2 /tmp/sia-zlib-new
```

The optional --board mode uses the public board-call API and explicitly relayouts
objects to put main first, because that API loads payload at entry. It does not
execute the exact CLI image container. Architectural default mode does.

## Simulator startup, static archives and GCC zlib comparison

`LinkedImage::load_into` validates the complete image/RAM/stack/return-trap
layout before any RAM mutation. It copies relocated storage, clears explicit
zero-fill regions, and returns the aligned stack, entry PC and SIA registers
(r13 stack top; r14 exit trap). The exit trap is an aligned sentinel outside
image and stack. The simulator stops when C returns to it and checks r1 and
stack restoration. This is zero-argument freestanding C startup; hosted
argc/argv, environment, constructors and OS protection are separate work.

CSIAIMG v2 adds region flag bit 2 for writable zero-initialized storage. Fully
zero-valued writable objects without relocations use this storage class,
including explicitly zero-initialized C objects. Code and read-only data cannot
be zero-fill. Nonzero payload bytes in zero-fill regions are rejected. Payload
still includes zero-filled bytes; compact BSS encoding is not implied. The
reader remains compatible with v1. Existing COSMIC-SIA v4 objects are unchanged.

`StaticArchive` reads ordinary Unix ar files with short, GNU long and BSD
embedded names, skipping their symbol indexes. Members must be v4 SIA bundles;
ELF and thin archives are rejected. `cosmic-link` first includes explicit
objects, then selects archive members satisfying unresolved externals, in
archive command order. It repeats within each archive to resolve backwards
dependencies. Archive groups and weak/common coalescing remain unsupported.

```sh
ar crs libhelpers.a helper.sia other.sia
cosmic-link --base 0x10000 --entry main --max-bytes 0xc0000 \
  --container -o program.csia main.sia libhelpers.a
python3 tools/sia/check-zlib.py /path/to/zlib-1.3.2 /tmp/zlib-new-run
```

The zlib gate builds GCC's byte oracle from the same unchanged Z_SOLO sources,
then builds nine SIA library objects and a guest harness. It makes an actual ar
library and links the guest plus that library through the CLI. Lazy extraction
leaves unused infback out. The exact CLI container runs on LightingMachine,
loaded through the public startup API into RAM initially filled with 0xa5.
`main` need not be the first function. The guest uses its own aligned bounded
allocator, with no host C services.

Eight cases cover empty/one-byte input, repetitive and pseudorandom inputs,
levels 0/1/6/9, default/fixed/Huffman/RLE strategies, and corrupt header errors.
Every compressed byte, CRC32 and Adler32 is compared to the GCC oracle; every
decompressed byte is compared to input. Simulator acceptance is the primary
software gate. RTL and physical FPGA qualification are independent checks.

Acceptance: all eight cases pass, with 2,998 compressed bytes compared. The
final simulator run executes 27,413,176 guest instructions from a 503,592-byte
image and restores the initial stack. The complete supported gate and 161 SIA
tests pass. Raw GCC oracle, generated expected bytes, archive selection/layout,
source/binary hashes and execution logs are saved in
`tools/sia/results/2026-10-05-startup-zlib/`.
