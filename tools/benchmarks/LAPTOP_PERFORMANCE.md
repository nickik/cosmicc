# Laptop native performance measurement

These are local whole-compiler and native-program measurements, not a backend-only Cranelift-versus-LLVM experiment.

Machine: AMD Ryzen 7 PRO 7840U w/ Radeon 780M Graphics; Linux-7.1.5-76070105-generic-x86_64-with-glibc2.39. Timed processes and children were pinned to CPU 0. Governor: powersave; boost: 1. No system settings were changed. Laptop frequency and background OS work remain uncontrolled.

Versions: cosmicc 0.12.0 (SIA32, amd64 Linux); gcc-13 (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0; Ubuntu clang version 18.1.3 (1ubuntu1).

Cosmic compiler was a Rust release build. Generated C code was explicitly measured twice: `-O0` selects Cranelift `none`, and `-O2` selects Cranelift `speed`. GCC/Clang levels select different pipelines; names do not imply equivalent transformations. All kernel objects use PIC; no generated-program LTO or host-specific architecture tuning was enabled. Cosmic enables CLIF verification in both modes, so compile timing includes those checks and is not bare Cranelift code generation. The same system-GCC-O2 harness supplies timing and input data for every variant.

Each kernel uses 20,000 outer calls; hash, branch and division each perform 4,096 inner steps per call, and matrix multiplication uses 16×16 matrices. Inputs are deterministic, integer overflow is unsigned or excluded by bounded signed inputs, and every warmup and measured program checksums against the GCC-O2 reference. Harness setup and output occur outside kernel timers.

Each phase used one untimed warmup per variant, then 7 compile/link repetitions, 7 runtime repetitions and 3 zlib repetitions. Variant order was shuffled each round with seed 20261005. Phases were serial. Entries below are **median milliseconds [Q1, Q3]**; raw samples also retain minima/maxima.

## Source-to-object and linking time

Compilation includes process startup, preprocessing, parsing/lowering, optimization, object emission and writing the object. It excludes linking. Link timing separately uses the same GCC driver and prepared harness object. Small translation-unit timings include substantial fixed startup overhead.

| Compiler/mode | Kernel source → object | Link executable | Nine zlib units → objects |
| --- | ---: | ---: | ---: |
| cosmic-O0 | 4.36 [4.13, 4.62] | 19.62 [18.94, 20.99] | 201.47 [192.72, 207.80] |
| cosmic-O2 | 4.24 [4.07, 4.53] | 18.75 [18.42, 19.58] | 204.77 [202.95, 215.30] |
| gcc-O0 | 21.34 [20.63, 21.42] | 19.08 [18.62, 19.93] | 388.85 [387.65, 390.67] |
| gcc-O2 | 38.94 [37.40, 39.36] | 19.46 [18.90, 19.96] | 1339.36 [1328.89, 1502.01] |
| clang-O0 | 36.93 [35.81, 37.16] | 19.86 [19.13, 20.14] | 424.11 [401.77, 433.09] |
| clang-O2 | 55.66 [55.06, 58.15] | 18.87 [18.59, 20.53] | 1305.23 [1279.81, 1388.38] |

Zlib timings are the sum of nine individual Z_SOLO translation-unit compilations per sample. They measure compilation only. The separate correctness roundtrip gate, rather than these compilation timings, establishes algorithm execution; no host zlib substitutes for Cosmic algorithms there.

## Native kernel execution time

| Compiler/mode | Hash | Branch/state | Integer division | Matrix |
| --- | ---: | ---: | ---: | ---: |
| cosmic-O0 | 229.45 [220.74, 229.78] | 734.05 [714.10, 777.02] | 313.19 [305.53, 313.44] | 210.09 [206.46, 215.42] |
| cosmic-O2 | 229.70 [217.94, 231.17] | 416.99 [411.21, 427.98] | 305.70 [300.52, 307.01] | 137.36 [135.14, 140.34] |
| gcc-O0 | 103.49 [103.43, 107.36] | 387.94 [383.11, 401.27] | 298.98 [294.52, 305.12] | 164.78 [158.64, 171.85] |
| gcc-O2 | 91.25 [86.97, 96.02] | 296.74 [289.29, 302.23] | 164.27 [157.47, 165.99] | 44.38 [42.92, 45.29] |
| clang-O0 | 372.83 [365.49, 377.47] | 738.53 [729.91, 764.72] | 320.27 [317.33, 328.44] | 162.89 [162.63, 167.38] |
| clang-O2 | 84.97 [82.94, 88.77] | 150.05 [142.84, 152.01] | 168.98 [159.54, 171.14] | 38.96 [37.49, 43.49] |

## Optimized ratios

Ratios are Cosmic-O2 time divided by the comparison compiler-O2 time; below 1 means Cosmic took less time. These ratios apply to these specific workloads and optimization configurations.

| Workload | Cosmic/GCC O2 | Cosmic/Clang O2 |
| --- | ---: | ---: |
| compile | 0.11× | 0.08× |
| zlib | 0.15× | 0.16× |
| hash | 2.52× | 2.70× |
| branch | 1.41× | 2.78× |
| divide | 1.86× | 1.81× |
| matrix | 3.10× | 3.53× |

## Reproducibility and interpretation

Raw measurements: `/tmp/cosmic-laptop-benchmark-final2/results.json`. Runner/kernel/harness source hashes, compiler binary hashes, prepared executable/object hashes, sample order, commands/logs, CPU affinity, governor, boost, frequency snapshots and start/end load averages are retained in JSON and the neighboring preparation artifact. Cosmic snapshot SHA-256: `7e7f64f3bd7f6bf2794b2b82d2639f7adfc7dd09fd24cab66652fbff4fc631d3`.

These small hot-cache kernels are not a general application benchmark. Native codegen passes, frontend lowering, target ABI work, process startup, machine state and workload choice all affect results. The numbers do not establish one universal Cranelift/LLVM runtime or compilation ratio. See [published comparisons](PUBLISHED_COMPARISONS.md) for studies with distinct workloads and versions.


Recorded data is retained in the adjacent `results/` directory. Full command logs
and compiler snapshots remain in the original `/tmp` run directory.

The recorded hash-loop assembly shows a concrete lowering difference: Cosmic O2
repeatedly loads/stores the running hash, index and parameters through stack
slots; GCC O2 keeps the loop state in registers. This is an observed difference,
not a complete attribution of the timing gap. Promoting suitable scalar locals
into SSA is a next implementation target, with address-taking, aliasing and
volatile behavior preserved. Disassemblies are retained alongside the data.

Compared with the published studies, this whole-C pipeline has a larger compile
advantage and a larger runtime penalty on these kernels. The historical
WebAssembly study summarized by Cranelift used older engines; the CGO 2024
query-JIT study found only a 20–35% compilation advantage. Workloads, frontends,
optimization policies and measured stages differ. Our figures do not reproduce
or contradict either study's experiment; see the primary sources in
[PUBLISHED_COMPARISONS.md](PUBLISHED_COMPARISONS.md).
