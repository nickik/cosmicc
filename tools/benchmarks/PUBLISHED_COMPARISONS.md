# Published Cranelift / LLVM comparisons

These are external results, not measurements of Cosmic C or this laptop.
Our full C compilation path includes preprocessing, semantic analysis, IR
construction, backend work, object writing, and process startup. A backend-only
JIT comparison cannot predict that whole-path time.

| Evidence | Workload and versions | Reported result | Applicability |
| --- | --- | --- | --- |
| [Xu and Kjolstad, Copy-and-Patch Compilation (2021)](https://arxiv.org/html/2011.13127v2) | WebAssembly CoreMark / PolyBench; Wasmer 1.0, Wasmtime 0.26, Chrome 81, WAVM | The [Cranelift project summary](https://cranelift.dev/) describes about an order-of-magnitude faster code generation than an LLVM system, with roughly 14% slower execution. | Historical WebAssembly engine comparisons, including frontend/runtime differences. The paper omitted Wasmer LLVM from final charts because WAVM performed better. This is not a native C frontend comparison or today's backend version. |
| [Engelke and Schwarz, CGO 2024](https://conf.researchr.org/details/cgo-2024/cgo-2024-main-conference/9/Compile-Time-Analysis-of-Compiler-Frameworks-for-Query-Compilation) | Database query JIT backends in Umbra; paper reports 6,678 functions from TPC-DS and 20 measurement repetitions | Cranelift compiled 20–35% faster than the studied LLVM configuration and ran similarly to unoptimized LLVM; LLVM achieved the highest execution performance. | Different workload, IR construction, optimization policy and tuned LLVM configuration. Much smaller compilation advantage than the historical WebAssembly result. [Authors' reproducibility artifact](https://zenodo.org/records/8256565). |

The numerical differences are evidence that workload and compiler configuration
matter. Neither study justifies a universal “Cranelift is N times faster” claim.
The laptop report must state optimization mode, measured stages, correctness
checks, repeated samples, versions, CPU, and workload separately.

Cosmic `-O0` selects Cranelift `none`; `-O1` and `-O2` select `speed`, and `-Os`
selects `speed_and_size`. These names do not imply the same optimization passes
or generated-code quality as GCC/Clang levels. In particular Cosmic's frontend
and memory-based lowering can dominate both compile time and runtime.

For a future backend-isolated experiment, produce equivalent IR with the same
semantics and measure only backend compilation; keep that experiment distinct
from the user-facing source-to-object measurements and native program timings.
