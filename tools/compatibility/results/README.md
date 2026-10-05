# Recorded compatibility data

`2026-10-05-differential.tsv` retains every case, profile, six compiler/mode
outcomes and source-review notes. `2026-10-05-differential.json.gz` contains the
complete original machine-readable report, including source/compiler/runner
hashes, commands, diagnostics and classifications; read it with gzip plus JSON.
The original run logs and immutable compiler binary remain in the `/tmp` path
recorded by the report. `2026-10-05-sia-execution.tsv` is the separate
LightingMachine architectural inventory, not a native AMD64 result.

The final SSA/varargs/operator/pragma run is retained as
`2026-10-05-differential-ssa-final.{json.gz,tsv}` (199/220 native passes at both
Cosmic modes). `2026-10-05-differential-ssa-first.*` retains the intermediate
195/220 inventory and its failures; the original 138/220 baseline remains intact.
The final runner annotates cases 00144, 00178 and 00200 while retaining every
original case and raw pass count.
