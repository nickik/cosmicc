# Cosmic C 0.13 inter-project Git integration

Exact Git pins between nickik projects are updated. Registry package versions,
sources and checksums are unchanged; Rust remains 1.98.1. The execution harness
has no sibling Cargo override or external path dependency.

| Dependency | Exact tested revision | Merged PR |
| --- | --- | --- |
| nickik/crainlift | `5c3dc85e79c6502e83e3210a9a29b5aabbc0d441` | #13 |
| nickik/LightingSimulation | `7a0c16aff324f678a72a05df163c208f8031914f` | #38 |
| nickik/LightingChips | `6d4a03e1ae31a008685cad68072ad1a652c62a85` | #9 |
| nickik/rax-plio-qdx | `b09117c762a36f6d25256fc3d0de3c212ada2d61` | #43 |
| nickik/SIA | `ae9d5771a788d374c7be86e77da3724252b5fd06` | Published main; metadata-only update |

The backend replaces `951ced55b67cc3b2cc76a04b1105a43e8aaf688d` with newer
SIA narrow comparisons and wide division while retaining System V variadic
vector-count support. Its 233 unit and 20 focused integration tests pass.
Lighting replaces `93a565c687f61d4e758af81bbde59ce5c4770996` and the local
hardware override with published production CPU/MainboardFPGA integration.
Validation and qualified-read access types traverse the complete hardware path.

Clean GitHub checkouts pass the complete supported compiler gate; six freshly
compiled direct RTL gates; six configured hardware adapter tests without
skips; ten independent reference tests; and the native compiler gate's
15 VLA cases, 48 comparisons, bounded/fault diagnostics and VLA-scope rejection.
The native command builds both bridges from pinned Git sources automatically.
Its source commit, source metadata and fresh bridge hashes are recorded in
`tools/sia/results/2026-10-05-013-clean-release/`.

The upgraded compiler remains at 211/220 native passes at O0/O2 with unchanged
outcomes. GCC/Clang stdarg and aggregate interoperability passes. All 146
accepted reviewed SIA cases execute (12,269,799 instructions). The I64 fixture
passes architectural and production RTL execution (650,691 instructions each).
Generic selection also passes the clean candidate bridge pair. These prior
consumer results are retained separately from clean release acceptance.

Hardware patches, clean-checkout receipts and exact merge SHAs are retained in
`tools/dependency-integration/`. The pins retain tested feature commits after
merges, so the dependency source bytes do not move with main branches.

Physical FPGA part selection, pin constraints, timing closure and actual board
bring-up are unverified. Full Cosmic boot qualification remains separate.
