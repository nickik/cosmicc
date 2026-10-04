#!/bin/sh
# Probe an extracted upstream e2fsprogs 1.47.2 release with SIA32-owned headers.
# Usage: sh scripts/probe-ext2.sh SOURCE_DIRECTORY OUTPUT_DIRECTORY
set -eu
source_dir=$(realpath "$1")
mkdir -p "$2"
output_dir=$(realpath "$2")
python3 - "$source_dir" "$output_dir" <<'PY'
import sys
from pathlib import Path
source, output = map(Path, sys.argv[1:])
(output / 'ext2fs').mkdir(exist_ok=True)
text = (source / 'lib/ext2fs/ext2_types.h.in').read_text()
# SIA32 widths, deliberately independent of host configure/libc results.
for key, value in {'ASM_TYPES_HEADER': '', 'PUBLIC_CONFIG_HEADER': '',
                   'SIZEOF_INT': '4', 'SIZEOF_SHORT': '2',
                   'SIZEOF_LONG': '4', 'SIZEOF_LONG_LONG': '8'}.items():
    text = text.replace('@' + key + '@', value)
(output / 'ext2fs/ext2_types.h').write_text(text)
(output / 'config.h').write_text('#define HAVE_UNISTD_H 1\n#define HAVE_SYS_STAT_H 1\n#define HAVE_SYS_TYPES_H 1\n#define SIZEOF_TIME_T 4\n')
(output / 'ext2_err.et').write_text(
    (source / 'lib/ext2fs/ext2_err.et.in').read_text()
    .replace('@E2FSPROGS_VERSION@', '1.47.2'))
PY
(cd "$output_dir" && compile_et ext2_err.et)
cp "$output_dir/ext2_err.h" "$output_dir/ext2fs/ext2_err.h"
compiler=${COSMICC:-./target/debug/cosmicc}
"$compiler" -I "$output_dir" -I "$source_dir/lib" \
    -I "$source_dir/lib/ext2fs" -I "$source_dir/lib/et" \
    "$source_dir/lib/ext2fs/alloc.c" -o "$output_dir/alloc.sia" \
    > "$output_dir/compile.log" 2>&1
