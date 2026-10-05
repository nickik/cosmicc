#!/usr/bin/env python3
"""Compile unmodified Z_SOLO algorithms with Cosmic; link and run without host zlib."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('source', type=Path)
p.add_argument('output', type=Path)
p.add_argument('--compiler', type=Path, default=Path('target/debug/cosmicc'))
p.add_argument('--optimization', choices=['O0', 'O2', 'Os'], default='O0')
a = p.parse_args()
source, out = a.source.resolve(), a.output.resolve()
out.mkdir(parents=True, exist_ok=False)
compiler = out / 'compiler-snapshot'
shutil.copy2(a.compiler.resolve(), compiler)
report = {'target': 'x86_64-unknown-linux-gnu', 'compiler_sha256': hashlib.sha256(compiler.read_bytes()).hexdigest(), 'optimization': a.optimization, 'commands': [], 'evidence': 'native amd64; not Lighting'}
def run(command, name):
    if command[0] == str(compiler): command.insert(1, '-'+a.optimization)
    try:
        result = subprocess.run(command, capture_output=True, text=True, timeout=30)
        (out/(name+'.log')).write_text(result.stdout+result.stderr)
        report['commands'].append({'command': command, 'returncode': result.returncode, 'log': name+'.log'})
        return result.returncode == 0
    except subprocess.TimeoutExpired:
        (out/(name+'.log')).write_text('exceeded 30-second budget\n')
        report['commands'].append({'command': command, 'status': 'timeout'})
        return False
    finally:
        (out/'summary.json').write_text(json.dumps(report, indent=2)+'\n')
objects = []
failed = []
report['sources'] = {}
for unit in 'adler32 crc32 deflate infback inffast inflate inftrees trees zutil'.split():
    path = source/(unit+'.c')
    report['sources'][unit] = hashlib.sha256(path.read_bytes()).hexdigest()
    obj = out/(unit+'.o')
    if run([str(compiler), '--target', 'amd64', '-c', '-D', 'Z_SOLO=1', '-I', str(source), str(path), '-o', str(obj)], unit):
        objects.append(obj)
    else:
        failed.append(unit)
report['compiled'] = len(objects)
report['failed'] = failed
if failed:
    report['status'] = 'compilation-failed'
else:
    harness = out/'roundtrip.c'
    harness.write_text(r'''
#include "zlib.h"
#include <stdlib.h>
#include <string.h>
static voidpf allocate(voidpf opaque, uInt count, uInt size) {
    (void)opaque; return calloc(count, size);
}
static void release(voidpf opaque, voidpf address) { (void)opaque; free(address); }
static voidpf refuse(voidpf opaque, uInt count, uInt size) {
    (void)opaque; (void)count; (void)size; return Z_NULL;
}
int main(void) {
    unsigned char input[4096], packed[8192], output[4096];
    unsigned int lengths[] = {0,1,31,4096};
    int levels[] = {0,1,6,9};
    unsigned int i, l, n;
    for (i=0;i<4096;i++) input[i]=(unsigned char)((i*37U)^(i>>5));
    for (l=0;l<4;l++) for (n=0;n<4;n++) {
        z_stream c={0}, d={0};
        unsigned long bytes;
        c.zalloc=allocate; c.zfree=release;
        if (deflateInit(&c,levels[l]) != Z_OK) return 1;
        c.next_in=input; c.avail_in=lengths[n];
        c.next_out=packed; c.avail_out=sizeof packed;
        if (deflate(&c,Z_FINISH) != Z_STREAM_END) return 2;
        bytes=c.total_out;
        if (deflateEnd(&c) != Z_OK) return 3;
        d.zalloc=allocate; d.zfree=release;
        if (inflateInit(&d) != Z_OK) return 4;
        d.next_in=packed; d.avail_in=(uInt)bytes;
        d.next_out=output; d.avail_out=sizeof output;
        if (inflate(&d,Z_FINISH) != Z_STREAM_END) return 5;
        if (d.total_out != lengths[n] || memcmp(input,output,lengths[n])) return 6;
        if (inflateEnd(&d) != Z_OK) return 7;
    }
    { z_stream c={0}; c.zalloc=refuse; c.zfree=release;
      if (deflateInit(&c,6) != Z_MEM_ERROR) return 8; }
    return 0;
}
''')
    exe = out/'roundtrip'
    # cc only compiles the harness and links Cosmic's nine algorithm objects.
    # No -lz or host zlib source/library is used.
    if run(['cc', '-D', 'Z_SOLO=1', '-I', str(source), str(harness), *map(str,objects), '-o', str(exe)], 'link'):
        report['status'] = 'passed' if run([str(exe)], 'execute') else 'execution-failed'
    else:
        report['status'] = 'link-failed'
(out/'summary.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'compiled':len(objects), 'failed':failed, 'status':report['status']}))
raise SystemExit(0 if report['status']=='passed' else 1)
