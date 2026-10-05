#!/usr/bin/env python3
"""Compile with Cosmic, inspect ELF, link with host cc, execute native x86-64."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import struct
import subprocess

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--compiler', type=Path, required=True)
p.add_argument('--reuse-snapshot', action='store_true', help='use the supplied immutable compiler file without copying')
p.add_argument('--output', type=Path, required=True)
p.add_argument('--cc', default='cc')
p.add_argument('--optimization', choices=['O0', 'O2', 'Os'], default='O0')
a = p.parse_args()
root = Path(__file__).resolve().parent
out = a.output.resolve()
out.mkdir(parents=True, exist_ok=False)
compiler = a.compiler.resolve() if a.reuse_snapshot else out / 'compiler-snapshot'
if not a.reuse_snapshot: shutil.copy2(a.compiler.resolve(), compiler)
report = {'compiler_sha256': hashlib.sha256(compiler.read_bytes()).hexdigest(), 'optimization': a.optimization,
          'target': 'x86_64-unknown-linux-gnu', 'commands': [], 'evidence': 'native host execution; not Lighting'}

def run(command, name, expect_success=True):
    if command[0] == str(compiler): command.insert(1, '-'+a.optimization)
    result = subprocess.run(command, capture_output=True, text=True, timeout=30)
    (out / (name + '.log')).write_text(result.stdout + result.stderr)
    report['commands'].append({'command': command, 'returncode': result.returncode, 'log': name+'.log'})
    (out / 'summary.json').write_text(json.dumps(report, indent=2)+'\n')
    if expect_success and result.returncode != 0:
        raise SystemExit('FAILED '+name+': '+result.stderr)
    return result

for name in ['cosmic', 'helper', 'reviewed-target']:
    obj = out / (name+'.o')
    run([str(compiler), '--target', report['target'], '-c', str(root/(name+'.c')), '-o', str(obj)], 'compile-'+name)
    data = obj.read_bytes()
    if data[:7] != b'\x7fELF\x02\x01\x01' or struct.unpack_from('<HH',data,16) != (1,62):
        raise SystemExit('not ELF64 little-endian x86-64 ET_REL: '+str(obj))
    if struct.unpack_from('<I',data,48)[0] != 0:
        raise SystemExit('unexpected x86-64 ELF flags')
    run(['readelf','-h','-s','-r',str(obj)], 'readelf-'+name)
run([a.cc,'-no-pie',str(root/'harness.c'),str(out/'cosmic.o'),str(out/'helper.o'),'-o',str(out/'native')], 'link')
run([str(out/'native')], 'execute')
# Host harness stays system-built: this isolates bidirectional ABI acceptance.
run([a.cc,'-c',str(root/'harness.c'),'-o',str(out/'harness.o')], 'host-harness')
run([str(compiler),'--target',report['target'],str(root/'cosmic.c'),str(root/'helper.c'),str(out/'harness.o'),'-o',str(out/'driver-pie')], 'driver-pie-link')
exe=(out/'driver-pie').read_bytes()
if exe[:7] != b'\x7fELF\x02\x01\x01' or struct.unpack_from('<HH',exe,16) != (3,62):
    raise SystemExit('driver default is not ELF64 x86-64 PIE (ET_DYN)')
run(['readelf','-h',str(out/'driver-pie')], 'readelf-driver-pie')
run([str(out/'driver-pie')], 'execute-driver-pie')
run(['ar','rcs',str(out/'libhelper.a'),str(out/'helper.o')], 'archive-helper')
run([str(compiler),'--target',report['target'],str(out/'cosmic.o'),str(out/'harness.o'),'-L',str(out),'-l','helper','-o',str(out/'driver-archive')], 'driver-archive-link')
run([str(out/'driver-archive')], 'execute-driver-archive')

run([a.cc,str(root/'reviewed-target-harness.c'),str(out/'reviewed-target.o'),'-o',str(out/'reviewed-native')], 'reviewed-link')
run([str(out/'reviewed-native')], 'reviewed-execute')
missing = run([a.cc,'-no-pie',str(root/'harness.c'),str(out/'cosmic.o'),'-o',str(out/'missing-helper')], 'missing-helper',False)
if missing.returncode == 0 or 'separate_helper' not in missing.stderr:
    raise SystemExit('missing external helper did not produce expected linker rejection')
report['status'] = 'passed'
(out/'summary.json').write_text(json.dumps(report,indent=2)+'\n')
print('PASS amd64 ELF64, system linking, LP64 ABI, native interoperability, multi-TU and missing-symbol rejection')
