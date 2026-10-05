# Tested local hardware dependency candidates

These are reviewable feature commits, not published dependencies or merges.

| Repository | Published base | Local tested candidate | Intended feature destination |
|---|---|---|---|
| nickik/LightingSimulation | 83a6acf8421356473a61336b0d58a504f7b40da8 | 7a0c16aff324f678a72a05df163c208f8031914f | github.com/nickik/LightingSimulation, refs/heads/cosmicc-hardware-cpu-integration |
| nickik/LightingChips | b4f4cee0524ebbfa61a5a9049f27755e05f6492e | 6d4a03e1ae31a008685cad68072ad1a652c62a85 | github.com/nickik/LightingChips, refs/heads/cosmicc-hardware-cpu-integration |

LightingSimulation pins published SIA ae9d5771a788d374c7be86e77da3724252b5fd06 and local RealCard b09117c762a36f6d25256fc3d0de3c212ada2d61. RealCard must be published first to make the Lighting Gitlink fetchable. Its feature publication was rejected by automatic approval review for missing exact-destination authorization. Neither candidate was pushed, and no main branch was merged.

Six freshly compiled direct RTL checks PASS. Six configured HardwareCpuBoard/mainboard integration tests PASS with no SKIP: access classes/MMIO rejection, bounded nonmutating validation, updating/grouped native transfers, Forge compiled guest workload, and six translated fixtures compared against the Rust reference. Ten encoding/assembler/software-board reference checks PASS on the new SIA pin. The parent additionally ran its current Cosmic Generic guest through this fresh bridge pair: PASS595.

The initial configured test log records a missing bluetcl PATH launch failure. The final replay exports BSC/bin PATH and BLUESPECDIR and passes unchanged assertions. Both logs are retained. No compiled bridge was reused for the fresh candidate acceptance.

Patches, receipt.json, build logs and individual directed RTL logs are retained here. receipt.json records commit bases and SHA256 evidence hashes. Reproduction scripts are in the candidate patches. Working paths: /tmp/lighting-hardware-integration-20261005 and /tmp/lighting-chips-cpu-integration-20261005. Both working trees are clean. Shared dirty source/submodules were preserved; unrelated DMA/QDX/Cosmic M29/M30 and autonomous storage/system closures were excluded. Third-party crate versions and Rust were not upgraded.

This is production RTL simulation acceptance. Physical FPGA part, pin constraints, timing closure and actual board bring-up remain separate prerequisites.

## Published clean Git acceptance

Both exact feature candidates are now published as LightingChips PR9 and LightingSimulation PR38; RealCard exactb091 is published as PR43. The earlier rejection was resolved by root with verified repository/ownership evidence. Clean recursive GitHub clones fetched the exact dependency tuple without local overrides. Six directed RTL gates, fresh CPU/mainboard bridge builds, six configured hardware tests (zero skips), and ten reference tests all passed. Clean logs and hashes are in clean-git/receipt.json. PRs remain draft until root coordinates compiler clean-pin acceptance; no main merge performed by this worker.

## Merge completion

After user-authorized clean hardware acceptance, PR9 merged as 9599b87e4c2b3412a31c2e5b4d44cad2fcf42817. RealCard PR43 merged first (df199a1b80dd55596ef48f9c6941962b8dbc79a8), then LightingSimulation PR38 merged as 60722e6f755a2707b4d86455d3fcab6a28278018. Expected-head SHA guards matched the exact tested candidates and base SHAs were unchanged. publication.json records the final remote state; earlier local-only/draft statements above describe historical stages.
