# _Bool postfix arithmetic

Both native and SIA lowering previously stored raw arithmetic results after
postfix increment/decrement. C converts the updated value back to _Bool:1++
remains1 and0-- becomes1. Both paths now normalize the arithmetic result with
an integer comparison against zero before writing SSA or memory storage, while
returning the original value as required by postfix operators. SIA also preserves
pointer-to-pointee lvalue wrappers and captures nonlocal storage addresses once;
this prevents pointer-sized writes to _Bool objects and repeated evaluation of
side-effecting addresses.

The common tools/amd64/bool-postfix.c fixture checks old expression values,
canonical byte representations, repeated local decrements, escaped locals,
globals, array elements and struct fields. Neighboring storage is checked too.
An immutable pre-fix compiler snapshot failed at CosmicO0 with exit11 (1++
stored2); all four GCC13/Clang18 O0/O2 reference executions passed. Evidence:
/tmp/cosmic-bool-postfix-beforefix/summary.json.

Reproduce corrected native differential acceptance with a new directory:

```sh
python3 tools/amd64/check-bool-postfix.py --compiler target/debug/cosmicc \
  --output /tmp/bool-postfix-new
```

The exact same C source runs through bool-postfix-native in tools/l21-execution
on architectural Lighting or the composed board with --board. Corrected native acceptance passes at O0/O2/Os, with all four GCC13/Clang18
references passing, against published backend951ced55b67cc3b2cc76a04b1105a43e8aaf688d.
Evidence and immutable compiler/source hashes:
/tmp/cosmic-published-951-bool-storage-final/summary.json. The final fixture also
checks that (*pick())++ evaluates a side-effecting address function exactly once.

Architectural Lighting passes the same fixture in2503 instructions with zero
result and restored stack; all153 SIA crate tests pass. Logs:
/tmp/cosmic-published-951-bool-sia-fixed.log and
/tmp/cosmic-published-951-sia-postfix-tests.log. Current board execution is
unavailable: the updated Lighting public API requires LIGHTING_CPU_BOARD_BRIDGE
for its production RTL CPU, and no such bridge is built. The mainboard bridge
alone cannot run this path. No composed-board Bool pass is claimed.

Final operator-corrected release acceptance also passes: `/tmp/cosmic-bool-operators-final/summary.json`.
Compiler SHA256 `60e33b787f7d6350ad66d4246e940a71648780908719448234af2193c9639835`;
raw report retained in `tools/compatibility/results/`.
