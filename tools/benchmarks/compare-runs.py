#!/usr/bin/env python3
"""Compare completed same-workload measurements with the saved pre-SSA baseline."""
import argparse
import json
import os
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('baseline', type=Path)
parser.add_argument('current', type=Path)
parser.add_argument('output', type=Path)
a = parser.parse_args()
before, after = [json.loads(p.read_text()) for p in (a.baseline, a.current)]
for report in (before, after):
    if report.get('status') != 'passed':
        parser.error('both measurements must have passed all checksum checks')
x, y = before['prepare'], after['prepare']
for field in ('sources', 'iterations', 'reference_checksums', 'system_component_hashes'):
    if x[field] != y[field]:
        parser.error('incompatible prepared measurements: ' + field)
for tool in ('gcc', 'clang'):
    if x['tool_hashes'][tool] != y['tool_hashes'][tool]:
        parser.error('reference compiler changed: ' + tool)
for field in ('repeats', 'random_seed'):
    if before[field] != after[field]:
        parser.error('measurement policy changed: ' + field)
for field in ('cpu_model', 'platform', 'measurement_cpu', 'boost'):
    if before['machine_start'][field] != after['machine_start'][field]:
        parser.error('machine configuration changed: ' + field)
if before['machine_start']['cpufreq']['scaling_governor'] != after['machine_start']['cpufreq']['scaling_governor']:
    parser.error('governor changed')

def median(r, phase, variant, kernel=None):
    s = r['statistics'][phase][variant]
    return (s[kernel] if kernel else s)['median_seconds']

lines = ['# Native performance after scalar SSA lowering', '',
         'The same kernels and harness were measured again after scalar SSA lowering and the accompanying compiler changes. This is a whole-compiler comparison; it does not isolate SSA from other changes in the new snapshot.', '',
         f"Baseline compiler SHA-256: `{x['compiler_sha256']}`. New compiler SHA-256: `{y['compiler_sha256']}`.", '',
         'Workload/runner hashes, reference compiler and linker hashes, checksums, iteration counts, repetition policy, CPU model, affinity CPU, governor and boost match. Frequency and background OS activity remain uncontrolled. Values below are median milliseconds; speedup means baseline time divided by new time.', '',
         '| Cosmic mode/workload | Baseline ms | New ms | Baseline/new |',
         '| --- | ---: | ---: | ---: |']
for mode in ('cosmic-O0', 'cosmic-O2'):
    for phase, kernel in [('compile', None), ('link', None), ('zlib', None)] + [('runtime', k) for k in ('hash', 'branch', 'divide', 'matrix')]:
        old, new = [median(r, phase, mode, kernel) for r in (before, after)]
        lines.append(f'| {mode} {kernel or phase} | {old*1000:.2f} | {new*1000:.2f} | {old/new:.2f}× |')
lines += ['', '## New optimized execution against GCC and Clang', '',
          '| Kernel | Cosmic/GCC O2 time | Cosmic/Clang O2 time |', '| --- | ---: | ---: |']
for k in ('hash', 'branch', 'divide', 'matrix'):
    cosmic = median(after, 'runtime', 'cosmic-O2', k)
    lines.append(f"| {k} | {cosmic/median(after, 'runtime', 'gcc-O2', k):.2f}× | {cosmic/median(after, 'runtime', 'clang-O2', k):.2f}× |")
lines += ['', '## Reference drift between measurement windows', '',
          'These baseline/new ratios for unchanged reference compilers help assess machine/window variation; they do not normalize the Cosmic numbers.', '',
          '| Kernel | GCC O2 baseline/new | Clang O2 baseline/new |', '| --- | ---: | ---: |']
for k in ('hash', 'branch', 'divide', 'matrix'):
    ratios = [median(before, 'runtime', t, k)/median(after, 'runtime', t, k) for t in ('gcc-O2', 'clang-O2')]
    lines.append(f'| {k} | {ratios[0]:.2f}× | {ratios[1]:.2f}× |')
lines += ['', 'Cosmic O0 uses Cranelift `none`; O2 uses `speed`. GCC/Clang optimization levels select different pipelines. Rust release mode optimizes the compiler executable. Source-to-object, linking, and native runtime remain separate measurements; zlib here measures compilation only. The standalone measured report includes quartiles and the raw JSON retains samples and order.', '',
          f"Baseline data: [recorded JSON]({os.path.relpath(a.baseline.resolve(), a.output.resolve().parent)}). New data: [recorded JSON]({os.path.relpath(a.current.resolve(), a.output.resolve().parent)}). See [published comparisons](PUBLISHED_COMPARISONS.md) for distinct Cranelift/LLVM studies."]
a.output.write_text('\n'.join(lines) + '\n')
