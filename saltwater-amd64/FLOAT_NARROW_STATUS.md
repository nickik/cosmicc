# Native floating-to-narrow integer conversion

Unchanged c-testsuite00175 previously panicked in x64 instruction emission:
valid float/double conversions to char produced fcvt_to_sint with I8 destination,
but native scalar conversion instructions support32/64-bit integer destinations.

Native lowering now converts float/double to I32 before reducing to I8/I16.
Explicit typed C casts preserve destination signedness through fcvt_to_sint or
fcvt_to_uint. The generic narrow coercion path also uses the supported I32 width.
C truncation toward zero is retained for representable destinations; this does
not define behavior for out-of-range floating-to-integer C conversions.

`tools/amd64/float-narrow.c` covers dynamically supplied signed/unsigned char and
short conversions, fractional truncation, negative values, boundary values,
local initialization, return and call coercion. Four GCC13/Clang18 O0/O2
references and Cosmic O0/O2/Os all pass. The unchanged upstream00175 also produces
matching stdout across those seven combinations. A focused emission regression
test passes; the AMD64 crate now contains11 tests.

Evidence uses the final debug compiler against published backend
951ced55b67cc3b2cc76a04b1105a43e8aaf688d:

- /tmp/cosmic-published-951-float-narrow-final/summary.json
- /tmp/cosmic-published-951-case00175/summary.json
- /tmp/cosmic-published-951-float-narrow-focused.log

Reproduce with a new output directory:

```sh
python3 tools/amd64/check-float-narrow.py --compiler target/release/cosmicc \
  --output /tmp/float-narrow-new
```

This is native AMD64 floating-point acceptance; it does not establish SIA floating
point support. The parent records release-snapshot and complete differential
matrix gates separately.
