# Recorded laptop data

The preparation and result JSON files retain compiler/source/runner hashes,
validation checksums, commands, raw samples and order, machine metadata and
median/quartile statistics. Assembly files retain the actual optimized Cosmic
and GCC objects' disassembly. Full command logs, binaries and objects remain in
the original `/tmp` run directory named in the JSON. Rebuild using the scripts
in the parent directory; a new run must use a new output directory.

The final SSA/compiler snapshot is recorded in `2026-10-05-ssa-prepare.json`
and `2026-10-05-ssa-results.json`, with Cosmic/GCC O2 disassembly. It shares
unchanged workload/runner hashes, policy and reference tools with the saved
pre-SSA run; machine-window drift remains visible in the comparison report.
All prior result files remain intact.
