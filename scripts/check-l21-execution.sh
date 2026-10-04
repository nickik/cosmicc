#!/usr/bin/env sh
# Uses the exact Git-pinned LightingSimulation dependency, without sibling checkouts.
set -eu
COSMICC_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
LIGHTING_ROOT=$(cargo metadata --manifest-path "$COSMICC_ROOT/tools/l21-execution/Cargo.toml" --locked --format-version 1 | python3 -c 'import json,sys,pathlib; data=json.load(sys.stdin); print(next(str(pathlib.Path(p["manifest_path"]).parent) for p in data["packages"] if p["name"] == "lighting-simulation"))')
if [ -z "${LIGHTING_MAINBOARD_M6_MULTISLOT_BRIDGE:-}" ]; then
    BSC=$(command -v "${BSC:-bsc}")
    PATH="$(dirname -- "$BSC"):$PATH"
    export PATH
    BUILD="$COSMICC_ROOT/tools/l21-execution/target/mainboard"
    ROOT="$LIGHTING_ROOT/vendor/rax-plio-qdx/hardware"
    SEARCH="+:$ROOT/qli/bluespec:$ROOT/qic/bluespec:$ROOT/pti/bluespec:$ROOT/plio-tx/bluespec:$ROOT/plio-rax-host/bluespec:$ROOT/memory-controller/bluespec:$ROOT/mainboard-fpga/bluespec:$LIGHTING_ROOT/hardware/m6"
    mkdir -p "$BUILD"
    "$BSC" +RTS -K8M -RTS -u -sim -p "$SEARCH" -bdir "$BUILD" -simdir "$BUILD" -info-dir "$BUILD" -g mkTbMainboardMultiSlotBridge "$LIGHTING_ROOT/hardware/m6/TbMainboardMultiSlotBridge.bsv"
    "$BSC" +RTS -K8M -RTS -sim -p "$SEARCH" -bdir "$BUILD" -simdir "$BUILD" -e mkTbMainboardMultiSlotBridge -o "$BUILD/mainboard-multislot-bridge"
    export LIGHTING_MAINBOARD_M6_MULTISLOT_BRIDGE="$BUILD/mainboard-multislot-bridge"
fi
cargo run --manifest-path "$COSMICC_ROOT/tools/l21-execution/Cargo.toml" --locked
