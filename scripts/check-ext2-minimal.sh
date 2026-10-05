#!/bin/sh
# Creates a disposable image; only the native guest writes the final payload.
set -eu
COSMICC_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
OUTPUT=${1:-$(mktemp -d /tmp/cosmicc-ext2-minimal.XXXXXX)}
mkdir -p "$OUTPUT"
IMAGE="$OUTPUT/disk.img"
if [ -e "$IMAGE" ]; then echo "Refusing to replace $IMAGE" >&2; exit 2; fi
cat > "$OUTPUT/mke2fs.conf" <<'CONFIG'
[defaults]
base_features = none
blocksize = 1024
inode_size = 128
inode_ratio = 8192
[fs_types]
ext2 = {
features = none
}
CONFIG
truncate -s 262144 "$IMAGE"
MKE2FS_CONFIG="$OUTPUT/mke2fs.conf" mke2fs -q -F -t ext2 -b 1024 -I 128 -N 128 -m 0 -O none "$IMAGE"
python3 - "$OUTPUT" <<'PY'
from pathlib import Path
import sys
out = Path(sys.argv[1])
(out / 'original.txt').write_bytes(b'A' * 32)
(out / 'expected.txt').write_bytes(b'Hello from Lighting via QDX-B!!\n')
PY
debugfs -w -R "write \"$OUTPUT/original.txt\" /note.txt" "$IMAGE" > "$OUTPUT/fixture.log" 2>&1
e2fsck -fn "$IMAGE" > "$OUTPUT/before-fsck.log" 2>&1
cargo run --manifest-path "$COSMICC_ROOT/tools/l21-execution/Cargo.toml" --locked --bin ext2-minimal -- "$IMAGE"
debugfs -R "dump /note.txt \"$OUTPUT/actual.txt\"" "$IMAGE" > "$OUTPUT/verify.log" 2>&1
cmp "$OUTPUT/expected.txt" "$OUTPUT/actual.txt"
e2fsck -fn "$IMAGE" > "$OUTPUT/after-fsck.log" 2>&1
printf 'PASS independent ext2 file bytes and e2fsck: %s\n' "$IMAGE"
