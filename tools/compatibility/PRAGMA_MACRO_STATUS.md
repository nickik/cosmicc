# Macro save/restore pragma extension

The maintained preprocessor implements `#pragma push_macro("name")` and
`#pragma pop_macro("name")` as an extension. Each macro name has its own stack;
saving an undefined name records an explicit undefined state. Restoration
preserves object-like, empty and function-like definitions, including their
parameter lists and variadic metadata. A restored undefined state removes the
current definition.

Directive tokens are read without macro expansion, so defining macros named
`push_macro` or `pop_macro` does not change pragma recognition. The argument must
be a parenthesized string containing an ASCII C identifier, with no trailing
tokens. Malformed recognized directives produce a preprocessing diagnostic.
An unmatched pop leaves the current definition unchanged and uses the existing
ignored-pragma warning. Other unimplemented pragmas retain their prior warning
and ignore behavior; this does not implement `_Pragma` or `#pragma once`.

Three focused unit tests pass: nested restoration despite macros with pragma
names; restoration of undefined, empty and function-like states; and malformed
argument rejection. The complete parser library unit suite also passes:
140 tests, zero failures. Native corpus case 00206 previously exposed ignored
save/restore behavior; final unchanged-corpus execution passes at Cosmic O0/O2 with snapshot
60e33b78, as recorded in [the differential report](DIFFERENTIAL_STATUS.md).
