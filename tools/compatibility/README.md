# Established C compatibility baseline

Use the pinned [c-testsuite](https://github.com/c-testsuite/c-testsuite) single-exec
corpus before adding a larger GCC torture matrix. It provides individual case
standard/portability/libc tags, expected output, and a main/zero-exit interface.
[GCC torture](https://gcc.gnu.org/onlinedocs/gccint/Testsuites.html) is an appropriate
next layer, with compile-only and execute directories and a larger DejaGnu/options
matrix. Neither suite by itself certifies ISO C conformance.

## Immutable source

Commit: `5c7275656d751de0e68b2d340a95b5681858ed07`

Archive: [pinned GitHub source](https://codeload.github.com/c-testsuite/c-testsuite/tar.gz/5c7275656d751de0e68b2d340a95b5681858ed07)

Archive SHA-256:
`010008bf4b5671f947ae7e8d693a1c959a4a4d795fa7227c0abb1f95de45f7e7`

Download and extract outside the repository, verifying the archive hash first.
No upstream source is vendored, patched, concatenated or rewritten. The lock
file checks every case, tags, expected-output file and source/license README
against SHA-256 values; source mismatch or additional cases abort before testing.
Upstream individual test licenses are described through tests/LICENSE and their
origin metadata; retain the upstream tree and those files when redistributing.

## Selection

The initial tagged inventory has 157 portable C89/C99/C11-tagged cases without
`needs-libc`. Those tags are imperfect. The default reviewed freestanding profile
has **147 cases**, with 73 exclusions across the full 220-case corpus. The lock
file records every exclusion and its reason, including untagged strlen/printf,
hosted output, GNU attributes/statement expressions/builtins, and function-pointer
conversion to void*. Actual unsupported ISO features, including floating point,
anonymous members and compound literals, remain visible as compiler failures.

C89 legacy forms remain in the corpus. Cosmic has no strict `-std=c11` mode;
this is a C11-or-earlier feature envelope and compatibility baseline, not a strict
standard-conformance result. Case acceptance alone does not prove a test is free
of undefined behavior or that its assertion coverage is exhaustive.

## Compilation inventory

Run from the cosmicc directory, with a **fresh empty output directory**:

```sh
python3 tools/compatibility/probe-c-testsuite.py \
  /tmp/c-testsuite-5c7275656d751de0e68b2d340a95b5681858ed07 \
  /tmp/cosmicc-compat-run --compiler target/debug/cosmicc --timeout 5 --jobs 4
```

The runner verifies source integrity, snapshots/hashes one compiler binary, and
records its version, commands, source hashes, diagnostics and outcomes. It bounds
each compiler subprocess and kills its process group on timeout. JSON/TSV and
per-case logs remain in the output directory. Default maximum compile wall budget
is roughly 185 seconds for 147 cases with four workers. No generated host C program
is executed. Zero runner exit means the inventory completed, not all cases passed.

`--inventory-only` verifies source and selection without compiling. `--profile
tagged` exposes the original 157-case tag selection; its results must not be
presented as the reviewed freestanding baseline. Diagnostic categories identify
an observed boundary, not an automatic judgment of standards correctness.

## Architectural execution

```sh
cargo run --manifest-path tools/l21-execution/Cargo.toml \
  --bin compatibility-native --locked -- /tmp/cosmicc-compat-run \
  > /tmp/cosmicc-compat-native.tsv
```

For programs using ordinary C binary32, append serialized guest runtime objects
(context.sia, binary32-add.sia, binary32-convert.sia and binary64.sia) after the inventory
directory. The runner links these explicit objects through the same checked
linker; it does not provide host floating-point callbacks. The
`sia-float-language` runner builds these four objects from the runtime sources.

The inventory supplies candidates only when compiled, expected output is empty,
and main has no arguments. The driver reads the exact emitted bundle, links main,
loads a fresh LightingMachine, and executes guest SIA instructions. It checks zero
return, stack restoration, architectural traps and PC staying in executable image
regions. Per-case budget is 20 million instructions. It supplies no host library
callbacks, argc/env, hosted I/O or output substitution. Unresolved imports, traps,
nonzero returns and budget exhaustion remain explicit failures. TSV output is an
inventory; process exit zero is not a suite-pass signal.

This path is **LightingMachine architectural execution**. It does not establish
SoftwareCpuBoard/MainboardFPGA execution, protected Cosmic loader behavior, or
hosted runtime compatibility. The current runner dependency configuration uses
the local LightingSimulation checkout; preserve that qualification in reports.

See [C_COMPATIBILITY_STATUS.md](../../C_COMPATIBILITY_STATUS.md) for measured results
and the first actionable compiler boundaries.

## Native differential matrix

Run all **220** immutable cases, including cases outside the reviewed profile:

```sh
python3 tools/compatibility/differential-c-testsuite.py \
  /tmp/c-testsuite-5c7275656d751de0e68b2d340a95b5681858ed07 \
  /tmp/cosmic-differential-final --compiler target/release/cosmicc \
  --gcc gcc-13 --clang clang-18 --timeout 5
```

This executes Cosmic, GCC and Clang at O0 and O2 on native x86-64. Every stage
has a process-group timeout. It records source/lock/runner/compiler SHA-256,
compiler versions, commands, stderr and stdout, exit codes and upstream expected
output matches in `summary.json`. Each reference uses explicit GNU11 with legacy
implicit-int/declaration diagnostics disabled, allowing the upstream C89 corpus
and GNU extensions to remain visible. Cosmic has no equivalent dialect selection.

Reviewed-profile and outside-profile counts stay separate. A Cosmic gap with
reference consensus is actionable evidence; acceptance of excluded extensions
is not required for the freestanding profile. Disagreement with expected output,
compiler disagreement or optimization-sensitive behavior needs source review
before blaming a compiler: shared GCC/Clang behavior does not establish that a
program has defined ISO C behavior. Passing rows mean zero exit and exact upstream
output, not mere compilation. This runner uses the system linker/libc and makes
no Lighting execution or owned-runtime completeness claim. Exit zero means a
completed inventory; inspect the report for failed cases.

Next layers remain explicit work items:

- Pin a GCC torture execute/compile subset with its options and target requirements;
  record unsupported extensions separately from required language features.
- Add pinned Csmith generated programs and generator seeds with defined-behavior
  safeguards, GCC/Clang O0/O2 checksums and minimized disagreement reproducers.
- Review optimization-sensitive disagreements for undefined or implementation
  defined behavior before assigning compiler defects.
- Keep native host-libc acceptance separate from SIA architectural execution and
  owned-runtime acceptance. Never infer one target's pass from another's result.

## GCC torture execute suite on SIA

The GCC `gcc.c-torture/execute` corpus is a larger regression layer. GCC's
internals manual describes torture tests as regressions run across option
combinations and the execute cases as tests that should compile, link and run
([GCC testsuite manual](https://gcc.gnu.org/onlinedocs/gccint/Testsuites.html)).
This is not an ISO conformance certificate. The SIA runner compares each selected
program's GCC 13.3 native exit status with a real LightingMachine execution
through the checked CSIAIMG loader. The simulator recognizes C runtime
`abort` and `exit` entries as process termination; integer-only binary32/binary64
runtime objects are linked for floating-point cases. ELF ET_REL and ET_EXEC
have separate inspection/link/load acceptance tests. The runner does not
substitute host execution for SIA execution.

The source is pinned to the clean `rust-lang/gcc` checkout at
`6f155cc3f5a2dff33afe6cc3ed6c2e0e605ae6a3`. The initial broad profile chooses
all direct `.c` execute tests with UTF-8 source, no DejaGNU directive, no
`#include`, and a `main` token. This keeps the runner independent of the GCC
DejaGNU harness, target flags and hosted headers; unsupported cases remain
counted in the exclusion totals. The report stores every selected source hash,
the compiler/linker hashes, command logs, per-case timings and outcomes.

```sh
python3 tools/compatibility/gcc-torture-sia.py \
  /path/to/rust-lang-gcc /tmp/cosmic-gcc-torture-run \
  --limit 1217 --timeout 10
```

The bounded execution runner stops a testcase after 10 seconds or its
architectural instruction budget. A GCC pass with a Cosmic compile/link/run
failure is a compatibility gap to investigate; execution mismatches still
need source review for undefined or implementation-defined behavior. The
filtered profile is a repeatable regression suite, not proof of C compliance.

The first pinned run and exact per-case outcomes are retained in
[the 2026-10-05 report](results/2026-10-05-gcc-torture-sia/README.md). It passed
587 of 1,216 selected candidates on LightingMachine. Treat every remaining
GCC-accepted failure as a triage item: the corpus includes GNU extensions and
some target-sensitive or undefined programs, and passing this filter is not a
standard-conformance score.
