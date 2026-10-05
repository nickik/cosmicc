# Independent AMD64 target review

The focused fixture `reviewed-target.c` is compiled by Cosmic; its separate
`reviewed-target-harness.c` is compiled by the system C compiler and supplies
NaN and negative zero without depending on Cosmic's libc or constant folding.
It checks LP64 sizeof, padded member offsets, data/member relocations, repeated
static-local calls, scaled pointer increment and signed pointer differences,
scalar narrowing/widening and floating conversion, unordered comparisons and
floating truth conversion.

On 2026-10-05 the initial object compilation rejected the valid static pointer
initializer `&values[2]` with `nonconstant static initializer`. Inspection found
that the relocation resolver lacked HIR binary address-addend handling; this
was reported to the backend owner. A temporary diagnostic variant changed only
that initializer to a runtime `values + 2` assignment; all checks passed after
Cosmic object emission, system linking and native x86-64 execution (exit 0).
The backend owner subsequently implemented checked address addends. The unchanged
fixture compiled, linked using system CC default PIE, and executed with exit 0
in the enhanced native gate (`/tmp/cosmic-amd64-native-reviewed/reviewed-*.log`).
The entire enhanced native gate passed.

Reviewed lowering uses explicit AMD64 layout helpers, emits static locals as
local ELF data, performs floating `!=` truth tests (including unordered/NaN),
and diagnoses unsupported aggregate ABI classes. Current scalar outgoing
variadic calls and named-only variadic definitions have separate native
interoperability gates; consuming va_list remains unsupported. MEMORY-class
aggregate parameters larger than 16 bytes also have a separate gate, while small
register-class aggregates and aggregate returns remain unsupported. Parser
semantics reject AMD64 long double. Unused aggregate declarations are allowed;
they do not prove unsupported ABI classes. This review is bounded scalar/native
acceptance, not broad C conformance or Lighting execution.

Reproduce the unchanged fixture after rebuilding:

```
target/debug/cosmicc --target x86_64-unknown-linux-gnu -c tools/amd64/reviewed-target.c -o /tmp/reviewed-target.o
cc -no-pie tools/amd64/reviewed-target-harness.c /tmp/reviewed-target.o -o /tmp/reviewed-target
/tmp/reviewed-target
```
