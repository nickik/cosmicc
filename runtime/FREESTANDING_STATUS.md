# Freestanding runtime v1 foundation — 2026-10-04

Implemented target-owned `include/cosmic_runtime_v1.h`, `allocator.c` and
`strings.c`. The header exposes namespaced interfaces and an ABI version;
it is the first header of a prospective sysroot, not a complete C sysroot.
No host includes, callbacks, pointers or floating-point execution are used.

## Allocation contract

Caller supplies valid writable guest memory and a writable arena object outside
that buffer. SIA32 uses eight-bit bytes, 32-bit unsigned int and guest pointers.
Initialization aligns the start upwards to eight bytes and rejects an address
range that wraps. Allocation aligns results to eight bytes, includes checked
metadata/padding overhead, and returns null on zero size, overflow or exhaustion.
A failed allocation leaves existing contents unchanged.

The arena now supports first-fit free-block reuse, splitting, adjacent free-block
coalescing and trailing-space reclamation. Eight-byte private headers record each
block's requested size and aligned span. Free validates exact allocation identity
and the complete metadata chain before mutation; null free succeeds, invalid
interior/outside pointers and double free return 1. Reset invalidates every pointer.
Callers must serialize shared arenas and must not overwrite headers or arena state.
As with conventional allocators, a stale pointer cannot be distinguished from a
new allocation reusing the same address. Error checks are not memory isolation.

Realloc validates identity, preserves min(old size,new size) bytes and keeps the
old allocation live on failure. Growth can stay in existing padding or allocate,
copy and free; shrinking retains the block span for later reuse. Realloc(null,n)
allocates. Realloc(p,0) frees p and returns null. calloc checks count*size overflow
before allocating and clears all returned bytes. Its zero-size result is null.
The capacity and all metadata are target-owned; no host allocation serves guest
calls. `used` is the high-water block extent, not a sum of live requested bytes.

These interfaces support zlib-style callbacks: opaque points to cosmic_arena,
zalloc calls cosmic_arena_calloc(opaque,count,size), zfree calls
cosmic_arena_free(opaque,p). A callback can record invalid-free status in task-owned
state. Standard malloc/free symbols, capability-backed allocation and CRT startup
remain separate integration work. Eight-byte alignment covers the supported
32/64-bit scalar target policy; future over-aligned objects need a separate API.

## Strings and unsigned conversion

strlen, strnlen, strcmp, strncmp, strchr and strrchr have namespaced entry points.
Ordering uses unsigned character values; search includes the terminating zero
and converts the search character modulo 256. Inputs must designate valid buffers
with terminators within accessible memory (or the bounded routine's limit).

cosmic_strtoul32(text,end,base,status) parses ASCII whitespace/sign, base 2..36 or
autodetected octal/decimal/hexadecimal, and returns unsigned 32-bit values. Optional
end and status pointers allow callers to keep errors per task. Status values are
0=success, 1=no digits, 2=range overflow, 3=invalid base (header macros provided).
No digits or invalid base leaves end at the original input; overflow returns
UINT32_MAX and consumes the complete valid digit sequence. A representable negative
magnitude is negated modulo 2^32. Prefix 0x is consumed only when a hex digit
follows; 0b is not a supported prefix. No locale or host errno is consulted.

Root coordinates standard parser strchr/strtoul declarations. A future standard
strtoul wrapper on the confirmed 32-bit unsigned-long target can forward the result
and translate range status into task-local errno; that wrapper is not implemented.
The v1 header does not impersonate stdlib.h/string.h or host size_t. abort/exit,
CRT startup, standard runtime names, varargs, printf and full sysroot remain pending.

## Validation

```sh
cargo run --manifest-path cosmicc/tools/l21-execution/Cargo.toml --locked \
  --bin runtime-allocator-native
PATH=/path/to/bluespec/bin:$PATH \
LIGHTING_MAINBOARD_M6_MULTISLOT_BRIDGE=/path/to/mainboard-multislot-bridge \
  cargo run --manifest-path cosmicc/tools/l21-execution/Cargo.toml --locked \
  --bin runtime-allocator-native -- --board
```

Expanded guest passed on LightingMachine in 987,013 instructions, zero return and
restored stack. It exercises alignment, grow/shrink preservation, failed growth,
overflow/exhaustion, interior and stale pointers, free/double-free/null-free,
adjacent coalescing, split reuse, tail reclamation, calloc zero-fill and overflow,
zero-size realloc/free and reset. A 96-step eight-slot allocation/reallocation/free
stress scenario verifies all live payloads and pairwise non-overlap after each step,
then frees every allocation and requires used=0. String tests cover bounded scans,
unsigned ordering, matches, missing matches and terminating-zero search. Numeric
conversion tests cover maximum/overflow, signs, bases 2/8/10/16/36, partial prefix,
no digits, invalid base and end-pointer/null-output behavior.

Before the stress extension, the expanded fixture passed both LightingMachine and
SoftwareCpuBoard → MainboardFPGA → RAM in 43,248 instructions. The earlier monotonic
allocator slice passed both in 7,025; it has been superseded by free/reuse semantics.
The full composed-board stress run subsequently passed in 987,013 guest
instructions, zero result, restored stack and nonzero backend transactions.
It completed within the ten-minute wallclock bound. Both machine paths therefore
passed the same full source fixture; no host runtime services guest calls.
All execution uses compiler-produced linked SIA. Existing local sibling Cargo
patches remain prerequisites; this is not clean published-pin release acceptance.

Logs: /tmp/cosmic-runtime-allocator-reuse.log,
/tmp/cosmic-runtime-allocator-reuse-board.log,
/tmp/cosmic-runtime-allocator-stress-board.log.
