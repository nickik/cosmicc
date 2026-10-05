# zlib frontend compatibility slice — 2026-10-04

Changes stay in lex/replace.rs, lex/cpp.rs, and analyze/expr.rs, with parser
regressions in those modules.

- Preserve each untouched pending token's macro hide set when a function macro
  consumes a prefix. Previously the active DO8 invocation's suppression leaked
  to its sibling DO8/DO4/DO2 invocations, leaving their names in the C stream.
- Compare/subtract pointers to the same immediate pointee type while allowing
  different const/volatile qualifiers. Nested pointee types remain checked.
- Decay arrays before pointer difference checks; the resulting type is signed
  target ptrdiff_t, not a pointer.
- Compute a conditional pointer's common type with the union of immediate
  qualifiers. Object/void pointer conditionals produce qualified void pointers;
  null pointer constants inherit the other operand's type. Unrelated char*
  and int* branches are rejected instead of silently converting them.
- Keep the RHS of prefix ++/-- integer-typed. The previous cast of one to a
  pointer produced invalid pointer+pointer operations in valid ++p/--p code.
- Preserve subtraction for p - integer instead of emitting addition.

Meaningful regressions cover all eight nested DO8 expansions, qualified
comparison/difference, array decay, conditional qualifier union and qualifier
removal rejection, void/null cases, unrelated pointer rejection, and pointer
prefix/compound operations. Existing self/mutually recursive macro tests remain
passing. The supported workspace gate passed: 135 parser tests, 19
preprocessing tests, 148 lowering/image tests, five doctests (two ignored).
Log: /tmp/frontend-zlib-supported.log. Debug compiler build passed. These new
expression changes have compile/HIR evidence; no new Lighting native execution
claim is made by this report.

## Fresh unmodified-source evidence

Upstream zlib 1.3.2 remains unchanged at /tmp/zlib-1.3.2. The existing
`tools/ports/probe-zlib.py` was re-run with its 30-second TU budgets during this
slice; logs: /tmp/cosmicc-zlib-probe-next2. A final parallel inventory with
10-second TU budgets after all expression fixes is recorded in
/tmp/frontend-zlib-latest/summary.json and per-file logs. Concurrent lowering
work added temporary stage diagnostics to the binary used for that inventory.

crc32 and zutil emit SIA. infback and inflate now reach a cached SIA backend branch displacement assertion
(`label.rs:75`, range -64..=63);
adler32, deflate, inffast, inftrees and trees exceed the ten-second budget. The
previous undeclared DO2/DO4/DO8 and qualified-pointer/frontend diagnostics are
resolved. This is progress through the frontend, not a complete zlib port.

Preprocessing adler32 and deflate finishes promptly. Direct freshly-built
`saltwater_parser::check_semantics` probes of their Cosmic-preprocessed source
complete with 91 and 141 declarations respectively and no errors. A probe of
/tmp/cosmic-inftrees-expanded.c completes with 96 declarations and no errors.
The inftrees timeout therefore occurs after semantic analysis. Probe source:
/tmp/frontend-semantic-probe.rs; logs: /tmp/frontend-adler-semantics.log,
/tmp/frontend-deflate-semantics.log, /tmp/frontend-inftrees-semantics.log.
