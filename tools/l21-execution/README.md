# SIA production RTL execution gate

From a clean Cosmic C checkout, run:

```sh
BSC=/path/to/bsc sh scripts/check-l21-execution.sh
```

The gate resolves LightingSimulation from its exact Cargo Git revision and
builds fresh CPU and mainboard Bluesim bridges using the exact LightingChips
revision in `hardware-dependencies.json`. No sibling checkout or Cargo patch
is required. GitHub read access, Git, Python, Cargo and BSC are required. The
validated BSC release is 2026.01. Its bin directory and BLUESPECDIR are set for
both building and executing the bridges.

Explicit `LIGHTING_CPU_BOARD_BRIDGE` and
`LIGHTING_MAINBOARD_M6_MULTISLOT_BRIDGE` paths may reuse independently built
compatible bridges. Missing paths are built; no software CPU is substituted.

## Exact Git dependencies

- Cranelift: `5c3dc85e79c6502e83e3210a9a29b5aabbc0d441`.
- LightingSimulation: `7a0c16aff324f678a72a05df163c208f8031914f`.
- LightingChips: `6d4a03e1ae31a008685cad68072ad1a652c62a85`.
- RealCard (Simulation submodule): `b09117c762a36f6d25256fc3d0de3c212ada2d61`.
- SIA (Simulation dependency): `ae9d5771a788d374c7be86e77da3724252b5fd06`.

These are updates between nickik repositories. Registry dependencies and Rust
remain unchanged. In an SSH-authenticated environment, use:

```sh
CARGO_NET_GIT_FETCH_WITH_CLI=true \
GIT_CONFIG_COUNT=1 \
GIT_CONFIG_KEY_0=url.ssh://git@github.com/.insteadOf \
GIT_CONFIG_VALUE_0=https://github.com/ \
BSC=/path/to/bsc sh scripts/check-l21-execution.sh
```

## Acceptance boundary

The native gate sends compiler bytes to the production CPU/MMU/module RTL and
real MainboardFPGA bridge. It checks VLA allocation/cleanup and preserved outer
scopes, all signed/unsigned comparison operators, final stack restoration,
mainboard RAM transactions, and bounded diagnostics for loops and faults.
Hardware dependency gates additionally verify validation causes no storage
access or mutation and qualified MMIO is rejected before target access.

Raw prior and release validation results are retained in
`tools/sia/results/` and `tools/dependency-integration/`. Bluesim RTL execution
is separate from physical FPGA timing, pin constraints and board bring-up.
The full Cosmic boot and physical FPGA qualification remain unverified.
