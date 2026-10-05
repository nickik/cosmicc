# GCC torture execution on SIA

This is a differential regression run of GCC's `gcc.c-torture/execute` corpus
from the pinned `rust-lang/gcc` source revision
[`6f155cc3f5a2dff33afe6cc3ed6c2e0e605ae6a3`](https://github.com/rust-lang/gcc/commit/6f155cc3f5a2dff33afe6cc3ed6c2e0e605ae6a3).
The GCC internals manual describes the torture tests as compiler regression
tests run under multiple options; they are not an ISO C certification suite
([GCC testsuite documentation](https://gcc.gnu.org/onlinedocs/gccint/Testsuites.html)).

The full source directory contained 1,684 direct `.c` execute tests. The
repeatable profile selected all 1,216 UTF-8 standalone candidates with no
DejaGNU directive, no `#include`, and a `main` token. It excludes 335 cases
whose target/options are controlled by DejaGNU, 132 header/system-library
cases, and one non-UTF-8 source. No selected source was copied into this
repository. Their SHA-256 values and all per-case outcomes are in `summary.json`.

GCC 13.3 compiled and ran the reference cases natively with `-std=gnu89 -O0`.
Cosmic C compiled the same sources for SIA; `cosmic-link` linked the checked
CSIAIMG image with the test runtime and integer-only binary32/binary64 runtime
objects; `multi-object-native` loaded and executed the image on LightingMachine.
The simulator recognizes guest `abort` and `exit` calls as process termination.
No production RTL run is involved.

| Outcome | Cases |
| --- | ---: |
| GCC reference passed and SIA passed | 587 |
| Cosmic compile failed | 528 |
| Cosmic link failed | 11 |
| SIA execution returned failure while GCC passed | 28 |
| GCC compile failed | 29 |
| GCC execution failed | 33 |
| **Total selected** | **1,216** |

The GCC reference passed 1,154 cases; 587 of those passed on SIA. The other
567 expose current compile, link or runtime boundaries. GCC's 62 compile/run
failures are kept separate from the Cosmic comparison. GCC torture includes
GNU and target-sensitive tests, so each reported failure needs source review
before classifying it as a required ISO C behavior or compiler defect. This
run is a compatibility inventory, not a claim that Cosmic C is fully compliant.

The JSON records UTC start/end, exact GCC/source revisions, compiler, linker,
simulator runner, and runtime-source hashes. Per-case logs remain in the local
output directory `/tmp/cosmic-gcc-torture-final-20261005`; the compact summary
is the retained evidence. Re-run using the command in
[`tools/compatibility/README.md`](../../README.md#GCC-torture-execute-suite-on-SIA).
