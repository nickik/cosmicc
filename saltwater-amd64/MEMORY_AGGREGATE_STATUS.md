Current continuation: incoming stdarg and natural INTEGER/SSE aggregate parameters/returns are now implemented and independently executed. See STDARG_STATUS.md and REGISTER_AGGREGATE_STATUS.md; historical limits below describe the earlier gate.

# System V MEMORY-class aggregate parameters

The [AMD64 psABI](https://gitlab.com/x86-psABIs/x86-64-ABI/-/blob/master/x86-64-ABI/low-level-sys-info.tex)
passes aggregates classified MEMORY in stack argument storage. This slice handles
structs/unions larger than16 bytes, with natural alignment at most8 and recursively
supported scalar/pointer/fixed-array fields. Vectors, x87/long double, flexible
arrays, small register-class aggregates and aggregate returns remain diagnosed.
It is a bounded parameter classifier, not full aggregate ABI coverage.

`aggregate.rs` emits Cranelift I64 address values with
`ArgumentPurpose::StructArgument(round_up(size,8))`. The maintained x64 backend
copies argument bytes into outgoing stack storage without consuming ordinary
GP/SSE argument registers. Callers snapshot each aggregate rvalue into an aligned
temporary during argument evaluation. Non-eight-byte sizes reserve and zero
padding so the backend's rounded memcpy cannot read past the original object.
Callees receive the incoming stack object's address, then copy the actual C size
into their own local storage. They never store the incoming pointer as the value
of the aggregate parameter. Fixed and variadic arguments use the same path.
Variadic aggregate fields do not count as vector-register arguments.

All eight Cosmic O0/O2 × GCC13/Clang18 O0/O2 interop combinations pass, alongside
four host-only fixture references. Fixtures cover multiple32-byte mixed integer/
double aggregates,17-byte character aggregates padded to24, direct/indirect calls,
fixed and ellipsis arguments, pointers and scalar arguments interleaved with
stack copies. Host entry captures AL2 for fixed+unnamed scalar doubles; aggregate
FP fields contribute no vector count. Host volatile mutations and Cosmic local
mutations leave caller originals unchanged, establishing by-value behavior.

The unchanged c-testsuite case00140 now compiles, links and executes at O0/O2/Os.
Ten AMD64 crate tests pass, including explicit small-aggregate and return
rejection gates. Evidence:

- /tmp/cosmic-published-951-memory-final/summary.json
- /tmp/cosmic-published-951-case00140/summary.json
- /tmp/cosmic-published-951-native-tests.log

Final eight-combination interoperability and ten-test crate gates use published
backend951ced55b67cc3b2cc76a04b1105a43e8aaf688d.
They are native AMD64 execution, not Lighting evidence.

Reproduce from cosmicc with a new output directory:

```sh
python3 tools/amd64/check-aggregate-memory.py --compiler target/debug/cosmicc \
  --output /tmp/memory-aggregate-new
```

Final operator-corrected release acceptance also passes: `/tmp/cosmic-aggregate-operators-final/summary.json`.
Compiler SHA256 `60e33b787f7d6350ad66d4246e940a71648780908719448234af2193c9639835`;
raw report retained in `tools/compatibility/results/`.
