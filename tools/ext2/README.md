# Native ext2 integration

## Current evidence

The e2fsprogs 1.47.2 `alloc.c` GNU named variadic macro blocker is fixed.
Standard `__VA_ARGS__`, named argument tails, rescanning, empty arguments,
GNU omitted-argument comma elision, and invalid parameter placement are tested.
Target-owned `fcntl.h` declarations and `atexit` declaration remove the next
header/declaration failures. `alloc.c`, `alloc_sb.c`, `alloc_stats.c`,
`alloc_tables.c`, and `atexit.c` compile to COSMIC-SIA bundles.

Compilation alone does not validate the on-disk ABI. The frontend currently
maps `long long` to `Type::Long`, whose SIA32 width is four bytes. e2fsprogs
`ext2_types.h` selects `unsigned long long` for `__u64`. The native preflight
checks __u64/long/pointer widths and exits 2 unless they are 8/4/4:

```sh
LIGHTING_MAINBOARD_M6_MULTISLOT_BRIDGE=/path/to/mainboard-multislot-bridge \
  cargo run --manifest-path tools/l21-execution/Cargo.toml --locked --bin ext2-abi
```

Bluespec's bin directory must be on PATH. Build the bridge with
`scripts/check-l21-execution.sh` first. This probe runs actual SIA32 bytes
through SoftwareCpuBoard/MainboardFPGA/RAM and does not open a disk image.

Broader source probes also expose offsetof member-address lowering failures,
const-qualified pointer assignment errors, missing timeval/limits declarations,
and a 30-second compilation timeout on `block.c`. These are compatibility
issues, independent of the ABI preflight.

## Required runtime boundary

- Target-native malloc/calloc/realloc/free with checked sizes, alignment, and
  exhaustion; never return host pointers into guest memory.
- Target-native memory/string routines operating on guest addresses.
- Target-owned errno and ext2 error returns; bounded exit/abort diagnostics.
  `atexit` registration must be implemented or the unused module excluded from
  the linked dependency closure; declarations alone are not implementations.
- An ext2 `io_manager` backed by guest-visible QDX-B commands. Convert ext2
  filesystem blocks into 512-byte sector transfers with checked multiplication
  and namespace bounds. Handle partial-block reads/writes and propagate device
  completion errors. Flush must wait for actual backend persistence.
- Resolve function/global/data relocations in a guest image before execution.
  `Artifact::prepare_integer_call` currently rejects relocated functions;
  externs must link to target runtime symbols, never host callbacks.

## Existing storage implementation

LightingSimulation already supports `RaxPhysicalQdxBDevice::with_backends`.
The vendored QDX-B `FileDisk::open(path, 512, read_only)` implements BlockBackend
using little-endian words, bounds checks, and sync_data on flush. Reuse it with
an explicitly supplied disposable image. `LightingBoardMachine::attach_card`
provides the card attachment boundary. The native board-call helper currently
attaches only the CPU; it must accept/configure the storage card before reset.
Use the established slot/profile protocol and registers, rather than inventing
an ext2-specific MMIO ABI or intercepting guest block requests on the host.

## Acceptance still to implement

- [ ] Correct frontend 64-bit integer types/layout and native SIA32 arithmetic,
  comparisons, conversions, calls, loads/stores; pass the ABI preflight.
- [ ] Compile the ext2 read/write dependency closure and resolve all externs.
- [ ] Native allocator/memory/error runtime and relocatable image loader.
- [ ] Attach file-backed QDX-B storage to the native board runner.
- [ ] Create a disposable ext2 fixture with a known file using mke2fs/debugfs.
- [ ] Native open/read verifies its complete expected bytes.
- [ ] Native create/write/close persists a second file and flushes the device.
- [ ] Independent debugfs byte comparison and e2fsck verify the resulting image.

No native filesystem operation or successful file integration is claimed yet.
