#!/usr/bin/env python3
"""Pinned unmodified c-testsuite slice: Cosmic amd64 compile, cc link, native execute."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('source',type=Path)
p.add_argument('output',type=Path)
p.add_argument('--compiler',type=Path,required=True)
p.add_argument('--limit',type=int,default=32)
p.add_argument('--cc',default='cc')
a=p.parse_args()
if a.limit < 1: p.error('limit must be positive')
manifest=json.loads((Path(__file__).resolve().parents[1]/'compatibility/c-testsuite.lock.json').read_text())
source=a.source.resolve()
for item in manifest['files']:
    path=source/item['path']
    if not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest()!=item['sha256']:
        p.error('pinned source mismatch: '+item['path'])
out=a.output.resolve()
if source==out or source in out.parents: p.error('output must be outside upstream source tree')
out.mkdir(parents=True,exist_ok=False)
compiler=out/'compiler-snapshot'
shutil.copy2(a.compiler.resolve(),compiler)
rows=[]
report={'metadata':{'suite_commit':manifest['commit'],'compiler_sha256':hashlib.sha256(compiler.read_bytes()).hexdigest(),
                   'target':'x86_64-unknown-linux-gnu','policy':'first selected freestanding cases in pinned manifest order',
                   'evidence':'native amd64 execution; no Lighting or hosted-libc completeness claim'},'results':rows}
for case in [c for c in manifest['cases'] if c['selected']][:a.limit]:
    name=case['name']
    path=source/'tests/single-exec'/name
    row={'name':name,'tags':case['tags'],'commands':[]}
    rows.append(row)
    stages=[('compile',[str(compiler),'--target','x86_64-unknown-linux-gnu','-c',str(path),'-o',str(out/(name+'.o'))]),
            ('link',[a.cc,'-no-pie',str(out/(name+'.o')),'-o',str(out/(name+'.exe'))]),
            ('execute',[str(out/(name+'.exe'))])]
    for stage,command in stages:
        try:
            result=subprocess.run(command,stdout=subprocess.PIPE,stderr=subprocess.STDOUT if stage=='execute' else subprocess.PIPE,timeout=10)
            (out/(name+'.'+stage+'.log')).write_bytes(result.stdout+(result.stderr or b''))
            row['commands'].append({'stage':stage,'command':command,'returncode':result.returncode})
            if result.returncode != 0:
                row['status']=stage+'-failure'
                diagnostic=(result.stdout+(result.stderr or b'')).decode(errors='replace')
                row['diagnostic']=diagnostic.splitlines()[:5]
                if stage=='compile':
                    if result.returncode<0 or 'panicked at' in diagnostic: category='compiler-crash'
                    elif 'Verifier errors' in diagnostic or 'AMD64 lowering' in diagnostic: category='amd64-backend'
                    elif 'not found' in diagnostic and 'file ' in diagnostic: category='missing-header'
                    elif any(x in diagnostic for x in ['invalid program:', 'invalid syntax:', 'invalid macro:']): category='frontend-diagnostic'
                    else: category='compile-other'
                elif stage=='link': category='unresolved-symbol' if 'undefined reference' in diagnostic else 'link-other'
                else: category='execution-crash' if result.returncode<0 else 'execution-exit-mismatch'
                row['category']=category
                break
            if stage=='execute' and result.stdout!=(path.with_name(name+'.expected')).read_bytes():
                row['status']='output-mismatch'; row['category']='output-mismatch'; break
        except subprocess.TimeoutExpired:
            row['status']=stage+'-timeout'; row['category']=stage+'-timeout'; break
    else: row['status']='passed'; row['category']='passed'
    report['summary']=dict(Counter(r['status'] for r in rows))
    report['categories']=dict(Counter(r['category'] for r in rows))
    (out/'summary.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report['summary'],sort_keys=True))
raise SystemExit(0 if all(r['status']=='passed' for r in rows) else 1)
