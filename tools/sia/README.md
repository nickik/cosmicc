# SIA multi-object acceptance

`check-zlib.py SOURCE NEW_OUTPUT` compiles all nine unchanged zlib 1.3.2 Z_SOLO
core translation units and `zlib-roundtrip.c`, invokes cosmic-link, decodes its
exact CSIAIMG output and executes target-native code on LightingMachine. It
records source/compiler/linker hashes and logs. Run from cosmicc after building
cosmicc and cosmic-link. Cargo builds the public Lighting runner offline.

The guest uses level 1, windowBits 9, memLevel 1 and 31 bytes to keep initialization
bounded. Guest callbacks allocate from reserved RAM 0x80000..0xd0000 and zero it
with C loops. This is one roundtrip, not the full upstream zlib tests or hosted
libc. Full default-window diagnostic execution separately passed 7,637,910
instructions; the durable default gate passes 891,924 instructions.

Optional `--board` explicitly relayouts the objects with main at image base for
the public board-call transport. It requires LIGHTING_MAINBOARD_M6_MULTISLOT_BRIDGE
and Bluespec PATH; its result must be reported separately from exact CLI-image
architectural acceptance. A smaller two-TU static/string/call fixture is available
through `cargo run --offline --bin multi-object-native [-- --board]` in
`tools/l21-execution`; both paths passed 153 instructions.

## Aggregate, variadic and software floating-point acceptance

`check-abi-cross-tu.py NEW_OUTPUT` compiles separate caller/callee translation
units and four integer-only floating-point runtime objects, verifies GCC/Clang
O0/O2 references, then executes the exact Cosmic objects. `--board` selects
production CPU/mainboard RTL through the configured public bridges. Final
architecture and production checks both pass 20,706 instructions. The ordinary
double fixture passes 72,249 production instructions; focused aggregate/varargs
passes 3,877. Source/compiler/object/bridge hashes and logs are in results/.
See [ABI contract](AGGREGATE_VARIADIC_ABI.md) and
[float scope](../../SIA_FLOAT_LANGUAGE_STATUS.md). Physical FPGA and full Cosmic
boot qualification remain separate.
