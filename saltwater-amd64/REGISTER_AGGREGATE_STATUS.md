# AMD64 aggregate ABI acceptance

abi.rs classifies naturally laid out scalar/array-field structures and unions. Objects of at most 16 bytes use INTEGER or SSE eightbyte classes, with INTEGER dominating mixed fields. A register shortage rolls the whole aggregate back to stack transport, preserving remaining registers for later arguments. Large objects use a hidden return destination in RDI and return that address in RAX. These conventions follow the [AMD64 psABI](https://gitlab.com/x86-psABIs/x86-64-ABI/-/blob/master/x86-64-ABI/low-level-sys-info.tex).

A shared plan controls import/definition signatures, callers, callees, variadic vector counts, register-save cursors and returns. Register chunks preserve object bits through I64/F64 transports. Padded snapshots prevent overreads for odd-sized objects and preserve argument values during evaluation. Callees reconstruct independent parameter storage. Mixed-class va_arg reconstructs a temporary from GP/SSE save slots or takes the entire object from the stack when either bank lacks capacity.

The register gate passed eight Cosmic O0/O2 × GCC13/Clang18 O0/O2 combinations and four host-only references: /tmp/cosmic-native-register-storage-final/summary.json. It checks three-byte objects, single/four-float structures, double pairs, mixed return ordering, nested array fields, union merging, GP/SSE exhaustion rollback, direct/indirect calls, callbacks and aggregate va_arg in both directions.

The MEMORY return gate passed the same eight/four combinations: /tmp/cosmic-native-memory-return-storage-final/summary.json. It checks copied parameters, hidden destinations, indirect/callback returns, seven named GP arguments and a variadic aggregate-returning definition.

Limits: natural alignment at most eight; supported fields are scalar integers, pointers, float/double, nested aggregates and fixed arrays. Long double/x87, vectors, packed/overaligned objects and flexible-array ABI classes remain diagnosed. Frontend bit-fields currently warn that widths are ignored; this evidence does not establish compatible bit-field layout. No complete System V ABI or C extension claim is made.

Both final gates passed compiler SHA256 f56966383461ef5bc0867d5347d9369fcbc93c7b399d76242b73e54062fddcad against published backend951ced55b67cc3b2cc76a04b1105a43e8aaf688d. The native crate passes 19 tests; the general ABI O2 gate also passes.
