# L21 native execution validation

Run from the Cosmic C checkout:

```sh
BSC=/path/to/bsc sh scripts/check-l21-execution.sh
```

Alternatively put the Bluespec `bin` directory on PATH and supply
`LIGHTING_MAINBOARD_M6_MULTISLOT_BRIDGE` to reuse a built bridge. The script
fails if the real bridge is unavailable; it never substitutes a host executor.
The compiler release used locally was BSC 2026.01, archive SHA-256
`9da36623e301ae14ba5e670cb98d11c99277faa915025ccb7615de3ec14002c3`.

## Pinned dependencies

The gate resolves LightingSimulation from its exact Git revision using Cargo
metadata, including its pinned hardware submodule. No sibling checkout or
local Cargo override is needed. Backend revision:
`3cf7afcb6e771e0de539f5cb9bf413574a8f6963`; Lighting revision:
`93a565c687f61d4e758af81bbde59ce5c4770996`. Both landed on remote main.
The parser and lowering crate use the same backend/target-lexicon revision.
Fetching Lighting and its submodule requires repository read access. For an
SSH-authenticated environment where HTTPS credentials are unavailable, use:

```sh
CARGO_NET_GIT_FETCH_WITH_CLI=true \
GIT_CONFIG_COUNT=1 \
GIT_CONFIG_KEY_0=url.ssh://git@github.com/.insteadOf \
GIT_CONFIG_VALUE_0=https://github.com/ \
BSC=/path/to/bsc sh scripts/check-l21-execution.sh
```

The supported CI gate now runs workspace tests and a compiler build. The native
gate additionally requires Bluespec and Lighting repository read access.

The Lighting change adds a dependency-neutral native board-call API. It loads
real compiler bytes into board RAM, initializes architectural registers, and
clocks SoftwareCpuBoard → MainboardFPGA → backend. It stops at the explicit
return PC before fetching another instruction, with instruction/cycle limits
and bounded code/register/CAUSE/EPC/BADADDR/mainboard diagnostics.

The backend change routes CLIF comparisons through canonical 0/1 lowering,
preserves right operands when a two-address result reuses their register, and
combines dynamic-stack cleanup and symbol-address support with the stable
frame-base branch. Cosmic C fixes duplicated HIR index scaling, compound
assignment's address-initialized temporaries, preallocates addressed local
storage before loops/branches, loads indexed post-increment operands, and
retains outer cleanup metadata while compiling conditional returns.

## Verified locally on 2026-10-04

- 15 VLA execution cases: normal/repeated and nested scopes, nested/conditional
  returns, continue, break, goto out and within a scope, parent-VLA preservation,
  multidimensional access, and runtime sizeof.
- Every VLA case checks the C result and final r13 equals initial r13
  (`0x000f0000`); repeated scopes compare allocation addresses to detect leaks
  before the function epilogue. Each observes stack growth and mainboard RAM
  traffic.
- 48 signed/unsigned comparison cases verify all six C comparison operators.
- A deliberately non-returning function verifies bounded failure diagnostics.
- An unaligned native load verifies architectural fault diagnostics, including BADADDR.
- A goto into an active VLA scope is rejected during compilation.
- Cosmic C completion checks: `cargo fmt --all -- --check`,
  `cargo test --workspace` (131 parser, 15 preprocessor, 132 lowering tests,
  5 doctests passed; 2 existing doctests ignored), and `cargo build` pass.
- Lighting SoftwareCpuBoard regressions: three tests pass.

The integration also passed from an independent clean clone, using these
merged Git pins and a freshly built bridge. Remote CI results have not been
verified. See `LOWERING_TODO.md` for clean-checkout acceptance evidence.
The reconstructed e2fsprogs 1.47.2 probe was rerun separately using
`scripts/probe-ext2.sh`; it fails on the GNU named variadic macro in
`lib/ext2fs/alloc.c:36`. See `LOWERING_TODO.md` for inputs and reproduction.
