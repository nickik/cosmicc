#!/usr/bin/env python3
"""GCC byte oracle versus unchanged Z_SOLO SIA objects, archive, loader and simulator."""
import argparse, hashlib, json, pathlib, shutil, subprocess
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('source', type=pathlib.Path)
p.add_argument('output', type=pathlib.Path)
p.add_argument('--compiler', type=pathlib.Path, default=pathlib.Path('target/debug/cosmicc'))
p.add_argument('--linker', type=pathlib.Path, default=pathlib.Path('target/debug/cosmic-link'))
p.add_argument('--reuse-snapshot', action='store_true')
p.add_argument('--board', action='store_true', help='optional separate RTL gate, not required for simulator acceptance')
a = p.parse_args()
root = pathlib.Path(__file__).resolve().parents[2]
source = a.source.resolve()
out = a.output.resolve(); out.mkdir(exist_ok=False)
compiler = a.compiler.resolve() if a.reuse_snapshot else out / 'compiler-snapshot'
linker = a.linker.resolve() if a.reuse_snapshot else out / 'linker-snapshot'
if not a.reuse_snapshot:
    shutil.copy2(a.compiler.resolve(), compiler); shutil.copy2(a.linker.resolve(), linker)
def digest(path): return hashlib.sha256(path.read_bytes()).hexdigest()
report = {'compiler_sha256': digest(compiler), 'linker_sha256': digest(linker), 'sources': {}, 'commands': [], 'status': 'running', 'execution': 'optional RTL' if a.board else 'LightingMachine simulator with dirty RAM and public loader'}
def run(cmd, name, timeout=120, cwd=root):
    cmd = [str(x) for x in cmd]
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout, cwd=cwd)
        (out / (name + '.log')).write_text(r.stdout + r.stderr)
        report['commands'].append({'command': cmd, 'returncode': r.returncode})
        if r.returncode: raise RuntimeError(name + ' failed; see ' + str(out / (name + '.log')))
        return r.stdout
    finally: (out / 'summary.json').write_text(json.dumps(report, indent=2) + '\n')
try:
    names = 'adler32 crc32 deflate infback inffast inflate inftrees trees zutil'.split()
    sources = [source / (n + '.c') for n in names]
    guest = root / 'tools/sia/zlib-roundtrip.c'
    report['sources'] = {str(s): digest(s) for s in [guest, *sources]}
    report['headers'] = {s.name: digest(s) for s in sorted(source.glob('*.h'))}
    run(['gcc', '--version'], 'gcc-version')
    reference = out / 'gcc-zlib-reference'
    run(['gcc', '-O2', '-D', 'Z_SOLO=1', '-D', 'COSMIC_ZLIB_REFERENCE=1', '-I', source, guest, *sources, '-o', reference], 'gcc-reference-build')
    text = run([reference], 'gcc-reference-execute')
    cases = []
    for line in text.splitlines():
        index, length, crc, adler, payload = line.split()
        data = bytes.fromhex(payload)
        assert int(index) == len(cases) and int(length) == len(data) and len(data) <= 2048
        cases.append({'index': int(index), 'length': len(data), 'crc32': int(crc, 16), 'adler32': int(adler, 16), 'bytes': data.hex()})
    assert len(cases) == 8
    report['reference_cases'] = cases
    offsets = []; data = bytearray()
    for case in cases: offsets.append(len(data)); data.extend(bytes.fromhex(case['bytes']))
    header = ''
    for name, values in [('size', [c['length'] for c in cases]), ('offset', offsets), ('crc', [c['crc32'] for c in cases]), ('adler', [c['adler32'] for c in cases])]:
        header += 'static const unsigned int expected_' + name + '[8]={' + ','.join(str(v) + 'u' for v in values) + '};\n'
    header += 'static const unsigned char expected_bytes[' + str(len(data)) + ']={' + ','.join(str(v) for v in data) + '};\n'
    (out / 'zlib-oracle.h').write_text(header)
    objects = []
    for name, src in [('guest', guest), *zip(names, sources)]:
        obj = out / (name + '.sia'); objects.append(obj)
        run([compiler, '-D', 'Z_SOLO=1', '-I', source, '-I', out, src, '-o', obj], name)
    library = out / 'libz-sia.a'
    run(['ar', 'crs', library, *objects[1:]], 'archive')
    run([linker, '--base', '0x10000', '--entry', 'main', '--max-bytes', '0xc0000', '--container', '-o', out / 'image.csia', objects[0], library], 'link')
    report['objects_sha256'] = {obj.name: digest(obj) for obj in objects}
    report['archive_sha256'] = digest(library)
    report['image_sha256'] = digest(out / 'image.csia')
    args = ['--board', *objects] if a.board else ['--image', out / 'image.csia']
    run(['cargo', 'run', '--offline', '--locked', '--quiet', '--bin', 'multi-object-native', '--', *args], 'execute', timeout=600 if a.board else 180, cwd=root / 'tools/l21-execution')
    assert digest(compiler) == report['compiler_sha256'] and digest(linker) == report['linker_sha256'], 'compiler/linker changed during acceptance'
    report['status'] = 'passed'
except Exception as e:
    report['status'] = 'failed'; report['error'] = str(e); raise
finally: (out / 'summary.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps({'status': report['status'], 'cases': len(cases), 'compressed_bytes_compared': len(data), 'board': a.board}))
