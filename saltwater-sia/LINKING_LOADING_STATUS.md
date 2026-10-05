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
