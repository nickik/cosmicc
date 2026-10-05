# SIA I64 compiler legalization

Ordinary C signed/unsigned long long multiplication, division, remainder and
variable left/logical-right/arithmetic-right shifts now lower to integer-only
CLIF through the normal published SIA backend. No custom instruction encoder
or host arithmetic services generated execution. Multiplication uses 16-bit
partial products; division/remainder use restoring division; variable shifts
operate on two I32 words. Existing native I64 add/subtract, constant shifts,
extension and word-pair call lowering form the prerequisite closure.

Division is truncated toward zero with dividend-signed remainder. Runtime guards
trap on division by zero and signed INT64_MIN/-1 overflow; successful-path tests
do not establish those architectural fault codes. Out-of-range C shift counts
remain undefined; defined counts 0..63 are covered. Arithmetic signed right shift
is this target's documented implementation-defined behavior.

`tools/sia/i64-operations.c` covers boundary and 16 seeded random pairs, counts
0/1/15/16/31/32/33/47/63, mixed I32/I64 register+stack arguments, pair returns,
recursion and indirect calls. It executes successfully on LightingMachine in
627,896 instructions, with zero result/restored stack/no trap. GCC and Clang O2
execute the same fixture successfully. This is a bounded ABI matrix, not full
ABI conformance or production RTL-board evidence.

Reproduce from cosmicc:

```sh
cargo run --manifest-path tools/l21-execution/Cargo.toml --offline --quiet \
  --bin sia-source-native -- "$PWD/tools/sia/i64-operations.c"
```

Logs: `/tmp/cosmic-sia-i64-ordinary-final.log`; compiler workspace gate:
`/tmp/cosmic-sia-arithmetic-supported.log`. SIA 154 unit tests pass.

## Extended ABI and fault-path execution

`tools/sia/i64-abi-stress.c` passes GCC13/Clang18 at O0/O2 and LightingMachine
in 7,326 instructions: 12 I64 parameters,16 mixed-width parameters crossing register
pair boundaries, signed stack arguments, indirect calls and 16 live I64 values
across nested calls. The separate sia-i64-traps runner observes cause 3 for
division by zero (55 instructions) and signed MIN/-1 (81 instructions). Those are
explicit compiler trap-policy tests, not defined-C conformance inputs.
Evidence: `/tmp/cosmic-sia-i64-abi-stress.log` and
`/tmp/cosmic-sia-i64-traps.log`. Production RTL-board validation remains open.
