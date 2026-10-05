#!/usr/bin/env python3
"""Independent C TUs: reference native execution and exact Cosmic SIA objects."""
import argparse, hashlib, json, os, shutil, subprocess
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('output',type=Path)
p.add_argument('--compiler',type=Path,default=Path('target/debug/cosmicc'))
p.add_argument('--board',action='store_true')
a=p.parse_args()
root=Path(__file__).resolve().parents[2]
out=a.output.resolve(); out.mkdir(parents=True,exist_ok=False)
compiler=out/'compiler-snapshot'; shutil.copy2(a.compiler.resolve(),compiler)
sources=[root/'tools/sia'/('abi-cross-tu-'+n+'.c') for n in ['caller','callee']]
report={'compiler_sha256':hashlib.sha256(compiler.read_bytes()).hexdigest(),'sources':{str(s):hashlib.sha256(s.read_bytes()).hexdigest() for s in sources},'commands':[],'evidence':'production CPU/mainboard Bluesim' if a.board else 'LightingMachine'}
def run(name,cmd,timeout=120):
 r=subprocess.run([str(x) for x in cmd],cwd=root,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=timeout)
 (out/(name+'.log')).write_bytes(r.stdout)
 report['commands'].append({'name':name,'argv':[str(x) for x in cmd],'returncode':r.returncode})
 (out/'summary.json').write_text(json.dumps(report,indent=2)+'\n')
 if r.returncode: raise RuntimeError(f'{name} failed: see {out/name}.log')
for cc in ['gcc','clang']:
 for opt in ['O0','O2']:
  exe=out/(cc+'-'+opt)
  run(cc+'-'+opt+'-compile',[cc,'-'+opt,*sources,'-o',exe])
  run(cc+'-'+opt+'-execute',[exe])
objects=[]
for src in [*sources,*[root/'runtime/softfloat'/n for n in ['context.c','binary32-add.c','binary32-convert.c','binary64.c']]]:
 obj=out/(src.stem+'.sia'); run(src.stem+'-compile',[compiler,src,'-o',obj]); objects.append(obj)
report['objects']={str(s):hashlib.sha256(s.read_bytes()).hexdigest() for s in objects}
cmd=['cargo','run','--manifest-path',root/'tools/l21-execution/Cargo.toml','--bin','multi-object-native','--offline','--',*objects]
if a.board: cmd.append('--board')
run('sia-execute',cmd,timeout=600)
report['status']='passed'; (out/'summary.json').write_text(json.dumps(report,indent=2)+'\n')
print('PASS cross-TU aggregate return, indirect call, variadic I64/double promotions and va_copy; '+report['evidence'])
