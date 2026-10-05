#!/usr/bin/env python3
"""Render measured JSON with explicit phases, optimization labels, dispersion and limits."""
import argparse
import json
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('results',type=Path)
p.add_argument('output',type=Path)
a=p.parse_args()
r=json.loads(a.results.read_text())
if r.get('status')!='passed': p.error('only completed validated measurement reports can be rendered')
prepare=r['prepare']; machine=r['machine_start']; stats=r['statistics']
labels=['cosmic-O0','cosmic-O2','gcc-O0','gcc-O2','clang-O0','clang-O2']
def cell(s): return f"{s['median_seconds']*1000:.2f} [{s['q1_seconds']*1000:.2f}, {s['q3_seconds']*1000:.2f}]"
lines=['# Laptop native performance measurement', '',
       'These are local whole-compiler and native-program measurements, not a backend-only Cranelift-versus-LLVM experiment.', '',
       f"Machine: {machine['cpu_model']}; {machine['platform']}. Timed processes and children were pinned to CPU {machine['measurement_cpu']}. Governor: {machine['cpufreq']['scaling_governor']}; boost: {machine['boost']}. No system settings were changed. Laptop frequency and background OS work remain uncontrolled.", '',
       'Versions: '+ '; '.join(prepare['versions'].values())+'.', '',
       'Cosmic compiler was a Rust release build. Generated C code was explicitly measured twice: `-O0` selects Cranelift `none`, and `-O2` selects Cranelift `speed`. GCC/Clang levels select different pipelines; names do not imply equivalent transformations. All kernel objects use PIC; no generated-program LTO or host-specific architecture tuning was enabled. Cosmic enables CLIF verification in both modes, so compile timing includes those checks and is not bare Cranelift code generation. The same system-GCC-O2 harness supplies timing and input data for every variant.', '',
       f"Each kernel uses {prepare['iterations']:,} outer calls; hash, branch and division each perform 4,096 inner steps per call, and matrix multiplication uses 16×16 matrices. Inputs are deterministic, integer overflow is unsigned or excluded by bounded signed inputs, and every warmup and measured program checksums against the GCC-O2 reference. Harness setup and output occur outside kernel timers.", '',
       f"Each phase used one untimed warmup per variant, then {r['repeats']['compile']} compile/link repetitions, {r['repeats']['runtime']} runtime repetitions and {r['repeats']['zlib']} zlib repetitions. Variant order was shuffled each round with seed {r['random_seed']}. Phases were serial. Entries below are **median milliseconds [Q1, Q3]**; raw samples also retain minima/maxima.", '',
       '## Source-to-object and linking time', '',
       'Compilation includes process startup, preprocessing, parsing/lowering, optimization, object emission and writing the object. It excludes linking. Link timing separately uses the same GCC driver and prepared harness object. Small translation-unit timings include substantial fixed startup overhead.', '',
       '| Compiler/mode | Kernel source → object | Link executable | Nine zlib units → objects |',
       '| --- | ---: | ---: | ---: |']
for name in labels:
    z=stats.get('zlib',{}).get(name)
    lines.append(f"| {name} | {cell(stats['compile'][name])} | {cell(stats['link'][name])} | {cell(z) if z else 'not measured'} |")
lines+=['', 'Zlib timings are the sum of nine individual Z_SOLO translation-unit compilations per sample. They measure compilation only. The separate correctness roundtrip gate, rather than these compilation timings, establishes algorithm execution; no host zlib substitutes for Cosmic algorithms there.', '',
        '## Native kernel execution time', '',
        '| Compiler/mode | Hash | Branch/state | Integer division | Matrix |',
        '| --- | ---: | ---: | ---: | ---: |']
for name in labels:
    lines.append('| '+name+' | '+' | '.join(cell(stats['runtime'][name][k]) for k in ['hash','branch','divide','matrix'])+' |')
lines+=['', '## Optimized ratios', '',
        'Ratios are Cosmic-O2 time divided by the comparison compiler-O2 time; below 1 means Cosmic took less time. These ratios apply to these specific workloads and optimization configurations.', '',
        '| Workload | Cosmic/GCC O2 | Cosmic/Clang O2 |', '| --- | ---: | ---: |']
for phase,key in [('compile',None),('zlib',None)]+[('runtime',k) for k in ['hash','branch','divide','matrix']]:
    if not stats.get(phase): continue
    def median(name):
        s=stats[phase][name];return (s[key] if key else s)['median_seconds']
    lines.append(f"| {key or phase} | {median('cosmic-O2')/median('gcc-O2'):.2f}× | {median('cosmic-O2')/median('clang-O2'):.2f}× |")
lines+=['', '## Reproducibility and interpretation', '',
        f"Raw measurements: `{a.results.resolve()}`. Runner/kernel/harness source hashes, compiler binary hashes, prepared executable/object hashes, sample order, commands/logs, CPU affinity, governor, boost, frequency snapshots and start/end load averages are retained in JSON and the neighboring preparation artifact. Cosmic snapshot SHA-256: `{prepare['compiler_sha256']}`.", '',
        'These small hot-cache kernels are not a general application benchmark. Native codegen passes, frontend lowering, target ABI work, process startup, machine state and workload choice all affect results. The numbers do not establish one universal Cranelift/LLVM runtime or compilation ratio. See [published comparisons](PUBLISHED_COMPARISONS.md) for studies with distinct workloads and versions.']
a.output.write_text('\n'.join(lines)+'\n')
