#!/usr/bin/env python3
"""Check unused VLA bound effects and fixed-array storage across forward goto."""
import argparse,hashlib,json,pathlib,shutil,subprocess
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--compiler',type=pathlib.Path,required=True);p.add_argument('--output',type=pathlib.Path,required=True);a=p.parse_args()
out=a.output.resolve();out.mkdir(exist_ok=False,parents=True);source=pathlib.Path(__file__).resolve().parent/'unused-vla.c';compiler=out/'compiler-snapshot';shutil.copy2(a.compiler.resolve(),compiler)
report={'compiler_sha256':hashlib.sha256(compiler.read_bytes()).hexdigest(),'source_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'commands':[],'status':'running'}
try:
 for cc in [compiler,'gcc-13','clang-18']:
  for opt in ['O0','O2']:
   name=('cosmic' if cc==compiler else cc)+'-'+opt;exe=out/name
   commands=[[cc,*(['--target','amd64'] if cc==compiler else []),'-'+opt,source,'-o',exe],[exe]]
   for stage,cmd in zip(['compile','execute'],commands):
    r=subprocess.run([str(x)for x in cmd],capture_output=True,text=True,timeout=20);(out/(name+'-'+stage+'.log')).write_text(r.stdout+r.stderr);report['commands'].append({'command':[str(x)for x in cmd],'returncode':r.returncode})
    if r.returncode:raise RuntimeError(name+' '+stage+' failed')
 report['status']='passed'
finally:(out/'summary.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'status':report['status']}))
