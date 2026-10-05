# Scalar local SSA lowering

The amd64 backend promotes nonvolatile automatic/register scalar objects and
parameters to Cranelift frontend `Variable` values when their storage does not
escape anywhere in the function. Arithmetic and pointer scalar types are
eligible. Arrays, structures, unions, static/extern objects, volatile objects,
and address-taken scalar objects retain memory storage. The analysis examines
all statements and initializer expressions before lowering; it uses Symbol
identity, so shadowed names remain distinct.

Scalar reads, assignments and postincrements use `use_var`/`def_var`, with the
same width conversions as memory storage. Cranelift constructs block parameters
for merges and loop backedges; all blocks are sealed after the complete function
has been emitted, including forward labels and switch dispatch. Simple compound
and prefix updates are lowered directly by the parser; complex lvalues still
capture an address to ensure a single evaluation.

This is conservative promotion rather than alias analysis or memory-to-register
optimization of escaped objects. A pointer local can itself be promoted while
loads/stores through its pointer continue to access memory. Unexpected access to
promoted object storage is a compilation error, avoiding an incoherent shadow
stack copy. Uninitialized scalar reads retain C's undefined-behavior boundary.

Validation so far: the `scalar_ssa_preserves_control_flow_and_escaped_storage`
unit test compiles, links with the system C driver, and executes at None, Speed,
and SpeedAndSize. It covers while/do/for, continue/break, goto backedges, switch,
ternary and short-circuit side effects, shadowing, escaped alias writes, volatile
updates, narrow wrapping increments, FP updates, and pointer dereferences. All
10 amd64 crate tests pass against published backend pin 951ced55, with no local
dependency override. The final O2 native acceptance gate passes. Final six-way
220-case native inventory passes 199 cases at both Cosmic O0/O2, including
146/147 reviewed cases. The remaining reviewed case 00144 is an annotated
uninitialized-read/qualification boundary. All 21 full-corpus failures are
compile diagnostics; there are no compiler crashes, native crashes, linker
failures, output mismatches or timeouts. Final compiler snapshot is
`60e33b787f7d6350ad66d4246e940a71648780908719448234af2193c9639835`.
See [full interpretation](../compatibility/DIFFERENTIAL_STATUS.md).

`objdump -dr` of `tools/benchmarks/kernels.c` compiled with `-O2` shows
`bench_hash` keeping input/count/index/hash in registers; its loop has no stack
loads/stores. It reads each input word and uses a RIP-relative multiplication
constant. This is assembly evidence of removal of the observed local-spill
problem, not a new runtime measurement. Source-frozen timing is recorded in
[the full laptop report](../benchmarks/SSA_LAPTOP_PERFORMANCE.md) and
[the baseline comparison](../benchmarks/SSA_COMPARISON.md). The raw hash-runtime
speedup is 3.17× at O2, but unchanged references also ran 1.19–1.27× faster in the
new window; the comparison does not isolate SSA from other compiler or machine
changes.

The integrated scalar variadic helper also passes eight native interoperability
combinations; its separate status is documented by the variadic worker.
