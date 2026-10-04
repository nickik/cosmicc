#!/bin/sh
# Native prerequisites for ext2; this does not yet execute filesystem operations.
set -eu
COSMICC_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
if [ -z "${LIGHTING_MAINBOARD_M6_MULTISLOT_BRIDGE:-}" ]; then
    BSC=$(command -v "${BSC:-bsc}")
    export BSC
    PATH="$(dirname -- "$BSC"):$PATH"
    export PATH
fi
sh "$COSMICC_ROOT/scripts/check-l21-execution.sh"
export LIGHTING_MAINBOARD_M6_MULTISLOT_BRIDGE=${LIGHTING_MAINBOARD_M6_MULTISLOT_BRIDGE:-"$COSMICC_ROOT/tools/l21-execution/target/mainboard/mainboard-multislot-bridge"}
cargo run --manifest-path "$COSMICC_ROOT/tools/l21-execution/Cargo.toml" --locked --bin ext2-abi
cargo run --manifest-path "$COSMICC_ROOT/tools/l21-execution/Cargo.toml" --locked --bin ext2-i64
