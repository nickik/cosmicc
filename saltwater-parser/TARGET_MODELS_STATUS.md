# Explicit target data models

`Opt.target` selects `TargetDataModel::Sia32` (the default ILP32 model) or
`TargetDataModel::Amd64` (LP64) before preprocessing and semantic analysis.
Analyzer and preprocessor instances hold their own target; repeated compilation
with different targets does not rely on mutable global state or host headers.

Backend consumers use `Type::{sizeof_for,alignof_for,can_represent_for}`,
`StructType::offset_for` and `Expr::const_fold_for`. Legacy layout/folding APIs
retain the SIA default. Long and pointer widths, integer promotions, pointer
scaling, sizeof results and constant shift widths follow the selected model.
L/UL suffixes retain their long type separately from LL/ULL. AMD64 enum storage
is four bytes. Padded struct member offsets align the member before returning its
offset; native union sizes include trailing alignment padding.

AMD64 has owned LP64 overrides for limits.h, stdint.h, stddef.h and sys/types.h.
Architecture and size predefines match the selected model. Both sysroots are
freestanding. These declarations do not constitute a full host libc ABI.
AMD64 long double is rejected during semantic analysis, including declaration,
typedef, signature and sizeof contexts, because its ABI/layout is unrepresented.
`max_align_t` currently describes supported scalar types (maximum alignment 8).
Aggregate calling conventions and variadic ABI support remain backend concerns.
Nondecimal unsuffixed integer literal candidate selection and floating suffix
precision retain existing frontend limitations; this is not a conformance claim.

Validation on 2026-10-05:

- `cargo test -p saltwater-parser`: 135 unit tests, 19 preprocessing tests,
  initially 3 target-model tests and 5 doctests passed (2 doctests ignored).
- `cargo test -p saltwater-parser --test target_models --offline`: all 4 tests
  passed after adding the long-double rejection regression.
- `scripts/check-supported.sh`: passed, including SIA tests, workspace build
  and workspace check. The final long-double-only change subsequently passed
  the focused target-model suite.

Tests interleave AMD64/SIA/AMD64 compilation, check owned header boundary values,
size/alignment, long literals, arithmetic promotion, wide shifts, padded union
sizes and exact member offsets. These are parser/semantic results; no native
execution claim is made here.
