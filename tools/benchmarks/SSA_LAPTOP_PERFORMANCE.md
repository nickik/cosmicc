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
| cosmic-O0 | 3.73 [3.56, 4.12] | 15.97 [15.35, 16.24] | 177.26 [173.71, 177.52] |
| cosmic-O2 | 3.65 [3.47, 4.28] | 15.26 [14.65, 15.60] | 193.00 [187.61, 193.30] |
| gcc-O0 | 16.46 [15.57, 18.45] | 16.06 [15.37, 16.47] | 323.01 [322.64, 327.34] |
| gcc-O2 | 30.08 [29.62, 31.07] | 15.78 [15.53, 15.97] | 1122.20 [1113.91, 1123.31] |
| clang-O0 | 30.53 [29.79, 32.26] | 15.21 [14.92, 17.04] | 384.97 [384.16, 392.78] |
| clang-O2 | 43.96 [43.12, 45.89] | 15.38 [15.14, 16.06] | 1136.70 [1126.92, 1138.04] |

Zlib timings are the sum of nine individual Z_SOLO translation-unit compilations per sample. They measure compilation only. The separate correctness roundtrip gate, rather than these compilation timings, establishes algorithm execution; no host zlib substitutes for Cosmic algorithms there.

## Native kernel execution time

| Compiler/mode | Hash | Branch/state | Integer division | Matrix |
| --- | ---: | ---: | ---: | ---: |
| cosmic-O0 | 72.69 [72.65, 72.70] | 390.59 [390.49, 390.95] | 252.17 [251.77, 253.31] | 126.81 [126.64, 127.77] |
| cosmic-O2 | 72.45 [72.34, 73.14] | 285.47 [284.68, 286.91] | 250.78 [250.31, 252.90] | 71.69 [71.57, 73.09] |
| gcc-O0 | 90.58 [89.81, 91.12] | 332.07 [331.09, 334.47] | 254.57 [254.42, 257.92] | 136.78 [136.57, 138.65] |
| gcc-O2 | 71.67 [71.53, 71.80] | 239.63 [239.41, 240.01] | 131.72 [131.62, 132.02] | 36.57 [36.55, 36.61] |
| clang-O0 | 305.60 [305.36, 306.14] | 615.54 [615.34, 620.25] | 266.28 [265.85, 269.69] | 135.23 [134.81, 139.55] |
| clang-O2 | 71.48 [71.43, 71.60] | 120.65 [120.48, 121.25] | 134.62 [134.31, 134.81] | 30.98 [30.92, 31.15] |

## Optimized ratios

Ratios are Cosmic-O2 time divided by the comparison compiler-O2 time; below 1 means Cosmic took less time. These ratios apply to these specific workloads and optimization configurations.

| Workload | Cosmic/GCC O2 | Cosmic/Clang O2 |
| --- | ---: | ---: |
| compile | 0.12× | 0.08× |
| zlib | 0.17× | 0.17× |
| hash | 1.01× | 1.01× |
| branch | 1.19× | 2.37× |
| divide | 1.90× | 1.86× |
| matrix | 1.96× | 2.31× |

## Reproducibility and interpretation

Raw measurements: `/tmp/cosmic-laptop-benchmark-ssa-final/results.json`. Runner/kernel/harness source hashes, compiler binary hashes, prepared executable/object hashes, sample order, commands/logs, CPU affinity, governor, boost, frequency snapshots and start/end load averages are retained in JSON and the neighboring preparation artifact. Cosmic snapshot SHA-256: `60e33b787f7d6350ad66d4246e940a71648780908719448234af2193c9639835`.

These small hot-cache kernels are not a general application benchmark. Native codegen passes, frontend lowering, target ABI work, process startup, machine state and workload choice all affect results. The numbers do not establish one universal Cranelift/LLVM runtime or compilation ratio. See [published comparisons](PUBLISHED_COMPARISONS.md) for studies with distinct workloads and versions.
