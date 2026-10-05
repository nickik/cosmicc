# Native differential C compatibility evidence

This is an executable compatibility inventory, not ISO C conformance certification.
The immutable upstream corpus contains 220 cases; 147 belong to the reviewed
freestanding profile and 73 remain outside it. GNU11 reference compilation
preserves legacy C89 forms. The corpus and profile are unchanged from the prior
October 5 baseline; new source-review annotations do not remove cases.

Current source-frozen debug matrix on 2026-10-05:

| Compiler | Full corpus | Reviewed profile | Outside profile |
|---|---:|---:|---:|
| Cosmic O0 | 211/220 | 146/147 | 65/73 |
| Cosmic O2 | 211/220 | 146/147 | 65/73 |
| GCC 13.3.0 O0/O2 (each) | 220/220 | 147/147 | 73/73 |
| Clang 18.1.3 O0/O2 (each) | 220/220 | 147/147 | 73/73 |

Every pass means successful compile/link, zero native exit and exact upstream
combined stdout/stderr output. Cosmic's nine remaining outcomes are compile
rejections. No accepted case has a link, execution or output failure. Remaining
cases are 00095, 00144, 00170, 00204, 00209, 00210, 00213, 00214, 00216.
They include diagnosed pointer/qualifier constraints, GNU extensions, genuine
long double/x87 ABI support, while valid C11 `_Generic` now passes.
Reference agreement does not establish that constraint-violating or undefined
cases are valid ISO C. Case 00144 additionally reads an uninitialized local.

Independent defined fixtures validate the fixes that those cases helped expose:
logical/comparison result types are C int; shifts use independent integer
promotions and retain the promoted left operand's type; FP-to-narrow-integer
casts lower through supported wider conversions. Macro push/pop restoration has
focused tests and unchanged case 00206 now passes at both Cosmic modes. The
[pragma extension status](PRAGMA_MACRO_STATUS.md) gives its exact boundaries.

The native continuation includes incoming scalar and aggregate `va_list`
consumption, naturally laid out small register-class aggregate parameters and
returns, and MEMORY-class aggregate returns. Bidirectional GCC/Clang interop
passes at both Cosmic optimization modes. Exact classes and limits are recorded
in the [native continuation status](../../saltwater-amd64/CONTINUATION_MATRIX_STATUS.md).

Current full report: `/tmp/cosmic-native-generic-final-220/summary.json`.
Compiler SHA256: `ff5de80019013a5315083698fc3a09290f787ffcb3be7145abbcd3a385ec5f0e`.
Durable records are `../amd64/results/2026-10-05-native-generic-220.json.gz`
and its TSV. The earlier release matrix passed 199/220 with compiler SHA256
`60e33b787f7d6350ad66d4246e940a71648780908719448234af2193c9639835`;
its `results/2026-10-05-differential-ssa-final.*` records remain historical evidence.
The system linker/libc supplies hosted dependencies. This is native AMD64
evidence, not Lighting or owned-runtime acceptance.

Separate final SIA evidence: the reviewed 147-case profile compiled 137 cases;
all 137 passed LightingMachine architectural execution, totaling 11,311,259 guest
instructions. This is architectural simulation, not native AMD64 or composed
board evidence. The separate SIA reports are maintained by the SIA workflow.

Own-project Git upgrade replay: Cosmic 0.13 with backend 5c3dc85 retains 211/220 at both modes with zero outcome differences and all native ABI gates passing. See [upgrade evidence](../../saltwater-amd64/GIT_UPGRADE_STATUS.md). Third-party packages and Rust are unchanged.
