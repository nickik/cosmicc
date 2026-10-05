# Pinned Csmith differential run on SIA — 2026-10-05

The 32 deterministic seeds from Csmith revision
`0cdc710315cfee9035e22ef4363ca479270d1934` were generated with the profile in
`tools/compatibility/csmith.lock.json`. Each GCC 13.3 reference compiled and
ran to produce a checksum. Cosmic's SIA result was checked against that checksum
and executed in the LightingMachine simulator with the integer-only soft-float
runtime closure.

| Outcome | Seeds |
| --- | ---: |
| SIA compiled and checksum matched | 19 |
| Cosmic compile rejected | 13 |
| GCC reference compile/run failure | 0 |
| SIA execution failure/checksum mismatch | 0 |

The 13 compile rejections cluster around global scalar initializers that Cosmic
does not yet accept as link-time constants, pointer-comparison qualifier types,
and one pointer implicit-conversion case. The raw JSON records the seed list,
source hashes, checksums, exact toolchain/runtime hashes and per-case timings.
Generated source and complete logs remain in
`/tmp/cosmic-csmith-sia-20261005-final`; re-running the pinned generator and
same seed/profile reproduces the sources. This small integer-focused random
profile is regression evidence, not a C conformance pass.
