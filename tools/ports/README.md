# Upstream portability probes

These probes preserve upstream source and use Cosmic's owned target headers.
They distinguish per-TU compilation from linked/native application acceptance.

## zlib 1.3.2

Source: https://zlib.net/fossils/zlib-1.3.2.tar.gz
Archive SHA-256: `bb329a0a2cd0274d05519d61c667c062e06990d72e125ee2dfa8de64f0119d16`.
Verify that hash before extraction. Run from the cosmicc directory:

```sh
python3 tools/ports/probe-zlib.py /tmp/zlib-1.3.2 /tmp/cosmicc-zlib-probe
```

The selected upstream `Z_SOLO` configuration probes nine deflate/inflate/checksum
core files. It excludes gzip file I/O and convenience wrappers; callbacks for
allocation remain required. No host configure or host library is used.
`summary.json` records each source hash, exact compiler command, outcome and
first diagnostics. Full diagnostics are separate logs. Exit zero means the
inventory completed, not that all files compiled.

The CRC wrapper provides a small concrete acceptance fixture:

```sh
target/debug/cosmicc -D Z_SOLO=1 -I /tmp/zlib-1.3.2 tools/ports/zlib-crc32.c -o /tmp/crc.sia
target/debug/cosmic-link --base 0x10000 --entry main --max-bytes 0x40000 -o /tmp/crc.bin /tmp/crc.sia
# Copy the entry hex value printed above (omit the 0x prefix):
cargo run --manifest-path tools/l21-execution/Cargo.toml --bin zlib-crc-native --locked -- /tmp/crc.bin ENTRY_HEX 0
```

The old pinned backend fails the return path. The isolated fix passes the expanded
fixture on both machine paths; see REAL_SOFTWARE_STATUS.md and
../backend-patches/README.md for the dependency limitation. To run the expanded
known-vector and boundary fixture with a corrected backend:

```sh
cargo run --manifest-path tools/l21-execution/Cargo.toml --bin zlib-crc-native --locked -- --compile /tmp/zlib-1.3.2
# With the documented Mainboard bridge configured, append --board.
```

The native tool currently uses its existing sibling development patches.

