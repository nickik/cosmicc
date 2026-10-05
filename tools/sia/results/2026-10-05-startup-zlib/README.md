# SIA startup/archive/zlib simulator acceptance

Run `python3 tools/sia/check-zlib.py /path/to/zlib-1.3.2 NEW_OUTPUT_DIR` after
building Cosmic C and cosmic-link. `--reuse-snapshot` avoids redundant binary
copies while still recording hashes and checking they did not change.

GCC compiles the same unchanged nine Z_SOLO sources and guest allocator to
generate the oracle. The guest compares all 2,998 compressed bytes and both
checksums over eight cases, plus every round-trip byte and corrupt-header
rejection. The SIA CLI links a real ar archive into a CSIAIMG v2 container;
unused infback is not extracted. The public loader initializes dirty RAM and
sets entry/stack/return state before LightingMachine executes the exact image.

The final run passes after 27,413,176 guest instructions; final SP restoration
is asserted by the runner. Loader and archive error-path tests pass, including
no partial RAM writes, legacy v1 reading and GNU/BSD archive names. Full
compiler supported checks pass; final SIA test count is 161.

This is simulator software acceptance. Protected OS loading, hosted startup,
archive groups, weak/common rules and physical hardware qualification are
separate remaining work. No RTL run was required for this change.
