# System V outgoing variadic vector count

Cranelift ArgumentPurpose::SystemVVariadicCount marks one final unextended i32
argument. The x86-64 SystemV ABI assigns it to EAX without consuming a normal
GP/SSE register or stack slot. Callers supply the required vector-register
count in 0..8. Invalid position/type/return/convention and all other architectures
are rejected. IR display/parse spelling is sysv_varargs.

This hook supplies the ABI count for ordinary typed outgoing scalar variadic
calls. It does not implement default promotions, aggregate classification,
incoming register saves or va_list traversal; those remain frontend/runtime
responsibilities. Cosmic's frontend supplies promotions and the native backend
computes the count including fixed floating arguments, capped at eight.

Primary contract: [x86-64 psABI](https://gitlab.com/x86-psABIs/x86-64-ABI/-/blob/master/x86-64-ABI/low-level-sys-info.tex), variable argument lists.

Patch applies to published backend commit 923bc3927b328d0e1131c40a3a74e11b3957d6ab.
Backend 233 unit tests pass, including exhausted GP/SSE placement invariance,
invalid signatures and explicit SIA rejection. Eight native GCC13/Clang18
interop combinations pass at Cosmic O0/O2 and reference O0/O2, checking exact
AL counts, promotions, stack arguments, indirect calls and snprintf. Evidence:
/tmp/cosmic-varargs-backend-full.log and
/tmp/cosmic-varargs-local-interop/summary.json.

Published on the approved isolated backend branch as commit
`951ced55b67cc3b2cc76a04b1105a43e8aaf688d`; maintained compiler manifests pin this exact revision.
Object-writer tests also pass (7 library and 16 integration tests).
