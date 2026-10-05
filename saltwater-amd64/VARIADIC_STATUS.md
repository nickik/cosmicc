Current continuation: incoming stdarg and natural INTEGER/SSE aggregate parameters/returns are now implemented and independently executed. See STDARG_STATUS.md and REGISTER_AGGREGATE_STATUS.md; historical limits below describe the earlier gate.

# Scalar outgoing System V variadic ABI

The public [AMD64 psABI source](https://gitlab.com/x86-psABIs/x86-64-ABI/-/blob/master/x86-64-ABI/low-level-sys-info.tex)
requires the caller's AL to describe vector-register argument use, bounded by8.
Integer and SSE register allocation proceeds independently; further scalar
arguments use normal aligned System V stack slots. C default promotions turn
float into double and narrow integer types into int before classification.

`variadic.rs` constructs a call-site signature containing all actual arguments,
including promoted unnamed arguments. It appends a target-owned I32
`SystemVVariadicCount` special argument; Cranelift's x64/SystemV ABI binds that
argument to RAX without consuming an ordinary integer argument slot. Calls use
the call-site signature through indirect CLIF calls, including named targets,
so a function's fixed prototype does not constrain unnamed argument placement.
This reuses native register allocation and call lowering; there is no handwritten
instruction thunk or host substitute for generated C execution.

Supported classes are scalar integers/pointers and float/double fixed arguments,
with promoted scalar unnamed arguments. Supported MEMORY-class aggregates (>16 bytes, naturally laid out scalar/array
fields) now pass by value on the stack; see MEMORY_AGGREGATE_STATUS.md. Small
register-class aggregates, aggregate returns, x87/long double and vectors remain
diagnosed. Cosmic variadic definitions using only named
parameters use the normal fixed-parameter ABI and ignore unnamed arguments.
The parser's `va_start`/`va_arg` family is still unsupported. Real incoming
variadic support requires target register-save-area capture, named-parameter
GP/FP offsets and an overflow-stack cursor; ordinary fixed parameter handling
does not establish that support.

`tools/amd64/check-variadic.py` records compiler/source hashes and tests Cosmic
O0/O2 against GCC13/Clang18 O0/O2. Host fixed callers enter a Cosmic fixed adapter,
which calls host stdarg functions directly and through a function pointer.
Fixtures check entry AL0/1/7/8 and9(capped8), GP and SSE register exhaustion,
interleaved stack arguments, narrow signed/unsigned promotions, pointers, a
fixed float plus unnamed double, and snprintf formatting. Host variadic callers also enter Cosmic variadic
definitions with fixed float/double/int parameters and ignored unnamed register/
stack arguments. Host-only reference
fixtures pass under both compilers. All eight native Cosmic/host combinations and four host-only reference
combinations pass against published backend951ced55b67cc3b2cc76a04b1105a43e8aaf688d.
Evidence and source/compiler hashes:
/tmp/cosmic-published-951-varargs-final/summary.json. The three focused variadic
tests pass within the ten-test AMD64 crate gate. These results are host-native
x86-64 ABI evidence, not Lighting execution.

The general native ABI/linking gate also passes at O0/O2/Os against that published
pin: /tmp/cosmic-published-951-abi-O0/summary.json,
/tmp/cosmic-published-951-abi-O2/summary.json and
/tmp/cosmic-published-951-abi-Os/summary.json. This includes narrow integer and
mixed integer/FP register/stack interoperability, PIC/PIE and archive links.

Final operator-corrected release acceptance also passes: `/tmp/cosmic-variadic-operators-final/summary.json`.
Compiler SHA256 `60e33b787f7d6350ad66d4246e940a71648780908719448234af2193c9639835`;
raw report retained in `tools/compatibility/results/`.
