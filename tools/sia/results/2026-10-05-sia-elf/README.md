# SIA ELF acceptance — 2026-10-05

`et-rel-readelf.txt` records ELF32 little-endian SIA ET_REL headers, sections,
symbols and relocations, including a regular ar archive member accepted by the
Cosmic linker and independently by `rust-sia/scripts/sia_link.py`.

`et-exec-readelf.txt` records static ELF32 SIA ET_EXEC program headers. The RW
BSS segment has zero file bytes and nonzero memory size. The checked executable
loader loaded and executed this linked program on LightingMachine; see
`et-exec-lightingmachine.log`. This verifies simulator behavior, not a protected
OS loader contract or physical FPGA execution.
