# Native performance after scalar SSA lowering

The same kernels and harness were measured again after scalar SSA lowering and the accompanying compiler changes. This is a whole-compiler comparison; it does not isolate SSA from other changes in the new snapshot.

Baseline compiler SHA-256: `7e7f64f3bd7f6bf2794b2b82d2639f7adfc7dd09fd24cab66652fbff4fc631d3`. New compiler SHA-256: `60e33b787f7d6350ad66d4246e940a71648780908719448234af2193c9639835`.

Workload/runner hashes, reference compiler and linker hashes, checksums, iteration counts, repetition policy, CPU model, affinity CPU, governor and boost match. Frequency and background OS activity remain uncontrolled. Values below are median milliseconds; speedup means baseline time divided by new time.

| Cosmic mode/workload | Baseline ms | New ms | Baseline/new |
| --- | ---: | ---: | ---: |
| cosmic-O0 compile | 4.36 | 3.73 | 1.17× |
| cosmic-O0 link | 19.62 | 15.97 | 1.23× |
| cosmic-O0 zlib | 201.47 | 177.26 | 1.14× |
| cosmic-O0 hash | 229.45 | 72.69 | 3.16× |
| cosmic-O0 branch | 734.05 | 390.59 | 1.88× |
| cosmic-O0 divide | 313.19 | 252.17 | 1.24× |
| cosmic-O0 matrix | 210.09 | 126.81 | 1.66× |
| cosmic-O2 compile | 4.24 | 3.65 | 1.16× |
| cosmic-O2 link | 18.75 | 15.26 | 1.23× |
| cosmic-O2 zlib | 204.77 | 193.00 | 1.06× |
| cosmic-O2 hash | 229.70 | 72.45 | 3.17× |
| cosmic-O2 branch | 416.99 | 285.47 | 1.46× |
| cosmic-O2 divide | 305.70 | 250.78 | 1.22× |
| cosmic-O2 matrix | 137.36 | 71.69 | 1.92× |

## New optimized execution against GCC and Clang

| Kernel | Cosmic/GCC O2 time | Cosmic/Clang O2 time |
| --- | ---: | ---: |
| hash | 1.01× | 1.01× |
| branch | 1.19× | 2.37× |
| divide | 1.90× | 1.86× |
| matrix | 1.96× | 2.31× |

## Reference drift between measurement windows

These baseline/new ratios for unchanged reference compilers help assess machine/window variation; they do not normalize the Cosmic numbers.

| Kernel | GCC O2 baseline/new | Clang O2 baseline/new |
| --- | ---: | ---: |
| hash | 1.27× | 1.19× |
| branch | 1.24× | 1.24× |
| divide | 1.25× | 1.26× |
| matrix | 1.21× | 1.26× |

Cosmic O0 uses Cranelift `none`; O2 uses `speed`. GCC/Clang optimization levels select different pipelines. Rust release mode optimizes the compiler executable. Source-to-object, linking, and native runtime remain separate measurements; zlib here measures compilation only. The standalone measured report includes quartiles and the raw JSON retains samples and order.

Baseline data: [recorded JSON](results/2026-10-05-results.json). New data: [recorded JSON](results/2026-10-05-ssa-results.json). See [published comparisons](PUBLISHED_COMPARISONS.md) for distinct Cranelift/LLVM studies.
