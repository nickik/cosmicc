# Cosmic C 0.13 inter-project Git integration

Only exact Git revisions between nickik projects are being updated. Registry
package versions, sources and checksums remain identical to the 0.13 baseline;
Rust remains unchanged.

| Dependency | Previous revision | Candidate revision | Status |
| --- | --- | --- | --- |
| nickik/crainlift | `951ced55b67cc3b2cc76a04b1105a43e8aaf688d` | `5c3dc85e79c6502e83e3210a9a29b5aabbc0d441` | Published feature branch; compiler gate and AMD64/SIA architectural validation passed |
| nickik/LightingSimulation | `93a565c687f61d4e758af81bbde59ce5c4770996` | `7a0c16aff324f678a72a05df163c208f8031914f` | Tested local feature candidate; publication pending |

The backend candidate starts from the newer published fork main revision
`b952544cad78316aa691661e0831087e40b92d3b`. It retains newer SIA narrow
comparison and wide division changes while restoring Cosmic's System V
variadic-count support and frame regression test. Its 233 unit tests and 20
focused integration tests passed. Compiler consumer validation also passed:
full supported gate; AMD64 211/220 at O0 and O2 with unchanged outcomes versus
the baseline and three bidirectional GCC/Clang ABI gates; SIA 146/146 accepted
reviewed cases executed without failures (12,269,799 instructions); cross-TU
aggregate, indirect-call, I64/double variadic and va_copy regression.

Compiler SHA-256: `53e8d148f14fd2aaf072eff3c6613a7255ae1ac2e14d95d707788b567379a8cf`.
The I64 arithmetic fixture also passes both LightingMachine and production
CPU/mainboard RTL (650,691 guest instructions in each).
The full SIA inventory uses LightingMachine. Generic selection and cross-TU
ABI also pass current production CPU/mainboard Bluesim bridges. Rebuilding
from published hardware Git candidates and removing the sibling Lighting
override remain pending; these RTL checks currently use local sources.

The latest published LightingSimulation revision alone uses SoftwareCpuBoard.
Replacing the previous local hardware execution integration with that revision
would invalidate production CPU claims. A minimal hardware adapter candidate
therefore needs a published RealCard memory validation handshake, a compatible
SIA Git pin, and independently built CPU/mainboard bridges. Prebuilt bridges
are not sufficient evidence of a reproducible Git dependency closure.

Local compiler/runtime integration is committed separately from candidate Git
pin changes. Shared remote main/master publication remains pending review.

Matching LightingChips candidate:
`6d4a03e1ae31a008685cad68072ad1a652c62a85`, based on
`b4f4cee0524ebbfa61a5a9049f27755e05f6492e`. Both Chips and Simulation
candidates use branch `cosmicc-hardware-cpu-integration` in their own
repositories. Their exact patches and receipts are saved in
`tools/dependency-integration/lighting-hardware-candidates/`.
Six direct RTL gates, six configured hardware adapter tests (no skips), and
ten independent reference tests pass. The new compiler Generic fixture
also passes the freshly built candidate bridge pair. Simulation updates its
own SIA pin to `ae9d5771a788d374c7be86e77da3724252b5fd06`; that revision
changes only repository metadata and Cargo.lock, with no code change.
Publish the RealCard candidate before the Simulation Gitlink can be fetched.
The root sibling override is retained until the exact hardware Git commits
are published and a clean Git-only dependency run succeeds.
The tested RealCard candidate is
`b09117c762a36f6d25256fc3d0de3c212ada2d61`, based on
`c0b02d978b16aaf4dccc9310d5f1971a696436a9`. Its fresh Bluesim gates pass
11 validation cases, 8 qualified-read cases and all ten byte-enable masks.
It preserves side-effect-free validation and rejects unsupported qualified
MMIO accesses. The bounded responder tests do not replace concrete backend
integration tests. Review patch and receipt are saved in
`tools/dependency-integration/`.

Automatic approval review rejected publication of this exact candidate to
`nickik/rax-plio-qdx:cosmic-lighting-validation`, requiring explicit approval
for that code and destination. No alternate publication path was used.

Physical FPGA execution is unverified.
