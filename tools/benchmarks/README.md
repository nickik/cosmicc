# Native laptop measurements

Use a **release-built Cosmic compiler**, then prepare six independently generated
PIC variants: Cosmic/GCC13/Clang18, each at explicit O0 and O2. Cosmic O0 means
Cranelift none; O2 means Cranelift speed. Rust release optimization builds the
compiler executable; it does not select optimization of compiled C programs.

```sh
python3 tools/benchmarks/compare.py --phase prepare \
  --compiler target/release/cosmicc --output /tmp/cosmic-perf-new \
  --zlib /path/to/zlib-1.3.2
# Coordinate a quiet window before starting the measured phase.
python3 tools/benchmarks/compare.py --phase measure \
  --compiler target/release/cosmicc --output /tmp/cosmic-perf-new \
  --zlib /path/to/zlib-1.3.2
python3 tools/benchmarks/render-report.py /tmp/cosmic-perf-new/results.json \
  tools/benchmarks/LAPTOP_PERFORMANCE.md
```

Preparation compiles, system-links and verifies matching checksums. Measurement
verifies the immutable Cosmic snapshot, system compiler hashes/versions, runner
and workload hashes, and prepared object/executable hashes. It pins its own
process/children to one permitted CPU; it changes no governor/boost/system setting.
Machine/affinity/frequency/versions and raw samples are recorded.

Compilation, system linking and native execution are separate phases. Compilation
is end-to-end source-to-object, including startup and object I/O; the nine zlib
units form a separate compile-only workload. Every phase warms up each variant,
then shuffles variant order per repetition with a fixed seed. Reports use medians
and inclusive quartiles, retaining raw samples, minima/maxima and order. No LTO,
host tuning or compiler-result substitution is used. Timed kernel code contains
no host calls; the identical system-built harness supplies input and clocks.

The portable kernels exercise unsigned hash arithmetic, unpredictable branches,
64-bit division/remainder, and bounded signed matrix products. They are small
hot-cache kernels, not universal application coverage. The benchmark does not
execute on Lighting or measure SIA software arithmetic. Separate zlib runtime
acceptance validates Cosmic algorithms; compiling zlib here proves only compile
success. [Published comparisons](PUBLISHED_COMPARISONS.md) describe different
Cranelift/LLVM workloads and configurations and must not be relabeled as these
laptop results.

## Saved pre-SSA laptop run

[Measured report](LAPTOP_PERFORMANCE.md): Ryzen 7 PRO 7840U, GCC 13.3.0,
Clang 18.1.3 and source-frozen Cosmic release 7e7f64f3. At O2, Cosmic compiled
nine zlib units in 205 ms versus GCC 1339 ms/Clang 1305 ms; the four native kernels
required 1.41–3.10× GCC's and 1.81–3.53× Clang's execution time. All checksums matched.
These are the specified source-to-object workloads and kernels, not universal
compiler/backend ratios; the report retains modes, medians and quartiles.

## Final SSA/compiler run

The source-frozen compiler `60e33b78` uses the identical kernels, harness and
measurement runner. [Full measured report](SSA_LAPTOP_PERFORMANCE.md) retains
all six modes and quartiles; [baseline comparison](SSA_COMPARISON.md) retains
raw before/after times and unchanged-reference drift. At O2, Cosmic hash runtime
is 72.45 ms, versus GCC 71.67 ms and Clang 71.48 ms. Branch, division and matrix
remain 1.19–1.96× GCC and 1.86–2.37× Clang times for these workloads. Nine zlib
units compile in 193 ms versus GCC 1122 ms and Clang 1137 ms. All checksums match.

Cosmic O2 raw speedups over the saved snapshot range from 1.22× to 3.17×, but
unchanged reference runtimes also improved 1.19–1.27× between windows. Frequency
and OS activity were uncontrolled; the old/new numbers do not isolate SSA alone.
Actual hash disassembly now has no loop stack loads/stores. Durable new results,
preparation and disassembly are in `results/2026-10-05-ssa-*`; prior files remain.

To regenerate the before/after report after another compatible run:

```sh
python3 tools/benchmarks/compare-runs.py \
  tools/benchmarks/results/2026-10-05-results.json \
  /tmp/cosmic-perf-new/results.json tools/benchmarks/SSA_COMPARISON.md
```
