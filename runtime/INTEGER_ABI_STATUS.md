# Integer runtime and ABI first slice — 2026-10-04

`integer64.c`, included after `primitives.c`, adds two explicit I32-only helpers:

- `cosmic_mul64`: the low 64 bits of a full word-pair product, valid for both
  unsigned arithmetic and two's-complement bit patterns. Cross-products use
  unsigned I32 multiplication and intentionally discard bits above bit 63.
- `cosmic_sdivmod64`: signed quotient/remainder truncating toward zero. It uses
  unsigned magnitudes and the existing unsigned restoring divider, including
  correct INT64_MIN magnitude handling. Remainder follows the numerator sign.
  Status 1 means zero divisor; status 2 means INT64_MIN/-1 overflow. Failures
  preserve both outputs. This is an explicit checked helper policy, not a claim
  that general compiled C division now traps or calls the helper automatically.

Operands are low/high I32 words; outputs are distinct valid guest objects.
No native I64 C expressions, host headers, private instruction encoding, host
arithmetic service, or presumed I64 function return convention are required.

## Focused validation

`tools/l21-execution/src/bin/integer64-native.rs` generates fixed expected data
on the host, then compiles and links a C guest using the regular compiler APIs.
The guest checks 89 signed division/remainder combinations (negative operands,
INT64_MIN, INT64_MAX, word-boundary magnitudes and zero numerator), 100 modulo
products, and preservation of sentinel outputs for both error statuses.

Architectural LightingMachine execution passed: 1,095,292 guest instructions,
zero return status, no architectural trap, and exact stack restoration.
Log: `/tmp/cosmic-integer64-native.log`.

Reproduce from the repository root:

```sh
cargo run --manifest-path cosmicc/tools/l21-execution/Cargo.toml --locked --bin integer64-native
PATH=/tmp/cosmicc-l21-bsc/bin:$PATH \
LIGHTING_MAINBOARD_M6_MULTISLOT_BRIDGE="$PWD/cosmicc/tools/l21-execution/target/mainboard/mainboard-multislot-bridge" \
 cargo run --manifest-path cosmicc/tools/l21-execution/Cargo.toml --locked --bin integer64-native -- --board
```

The board mode uses the public SoftwareCpuBoard → MainboardFPGA → RAM runner.
Its acceptance result is recorded below when available. The execution manifest
uses existing local sibling patches; this is not clean published-pin validation.

## Audit and remaining coverage

The current backend ABI assigns two-word values to aligned register pairs, uses
8-byte alignment for spilled argument pairs, and makes subsequent arguments
stack-only once a value exhausts argument registers (`sia32/abi.rs`). Existing
backend work contains I64 arithmetic/bitwise, extension/reduction, and shift
lowering. These observed rules are not full mixed-call or spill execution proof.
The Cosmic compiler's existing `unsupported_i64_division_retains_backend_diagnostic`
test explicitly expects unsupported `udiv.i64` rather than automatic runtime
legalization. This slice does not change that boundary.

Still needed: wire unsupported CLIF operations through the normal backend;
choose generated division trap semantics; test mixed I32/I64 arguments,
register exhaustion, stack arguments, direct/indirect I64 returns, recursion and
spills on the composed board. Preserve concurrent sibling backend changes.

Composed-board acceptance passed: 1,095,292 guest instructions, zero result,
and restored stack through SoftwareCpuBoard → MainboardFPGA → RAM.
Log: `/tmp/cosmic-integer64-board.log`. The initial missing-bluetcl launch
failed before execution; the successful rerun used the installed Bluespec PATH.

Composed-board acceptance now PASSED: SoftwareCpuBoard → MainboardFPGA → RAM,
1,095,292 guest instructions, zero result and restored stack. Log:
`/tmp/cosmic-integer64-board.log`. This supersedes the pending note above.
