# Cosmic C 0.13 inter-project Git integration

Only exact Git revisions between nickik projects are being updated. Registry
package versions, sources and checksums remain identical to the 0.13 baseline;
Rust remains unchanged.

| Dependency | Previous revision | Candidate revision | Status |
| --- | --- | --- | --- |
| nickik/crainlift | `951ced55b67cc3b2cc76a04b1105a43e8aaf688d` | `5c3dc85e79c6502e83e3210a9a29b5aabbc0d441` | Published feature branch; compiler gate and AMD64/SIA architectural validation passed |
| nickik/LightingSimulation | `93a565c687f61d4e758af81bbde59ce5c4770996` | Pending hardware integration candidate | Existing sibling override must be removed only after published hardware adapter validation |

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
Physical FPGA execution is unverified.
