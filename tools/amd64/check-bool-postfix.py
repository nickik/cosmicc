#!/usr/bin/env python3
"""Differential _Bool postfix arithmetic, canonical storage and old values."""
import argparse,hashlib,json,pathlib,shutil,subprocess
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--compiler',type=pathlib.Path,required=True)
p.add_argument('--output',type=pathlib.Path,required=True)
a=p.parse_args(); out=a.output.resolve();out.mkdir(parents=True,exist_ok=False)
src=pathlib.Path(__file__).resolve().parent/'bool-postfix.c'
compiler=out/'compiler-snapshot';shutil.copy2(a.compiler.resolve(),compiler)
report={'compiler_sha256':hashlib.sha256(compiler.read_bytes()).hexdigest(),'source_sha256':hashlib.sha256(src.read_bytes()).hexdigest(),'commands':[],'status':'running'}
def run(cmd,name):
 r=subprocess.run([str(x) for x in cmd],capture_output=True,text=True,timeout=30)
 (out/(name+'.log')).write_text(r.stdout+r.stderr)
 report['commands'].append({'command':[str(x) for x in cmd],'returncode':r.returncode})
 (out/'summary.json').write_text(json.dumps(report,indent=2)+'\n')
 if r.returncode:raise RuntimeError(name+' failed: '+r.stderr)
try:
 for host in ['gcc-13','clang-18']:
  for opt in ['O0','O2']:
   name=host+'-'+opt;exe=out/name
   run([host,'-'+opt,src,'-o',exe],name+'-build');run([exe],name+'-execute')
 for opt in ['O0','O2','Os']:
  obj=out/(opt+'.o');exe=out/opt
  run([compiler,'-'+opt,'--target','amd64','-c',src,'-o',obj],opt+'-compile')
  run(['cc',obj,'-o',exe],opt+'-link');run([exe],opt+'-execute')
 report['status']='passed'
except Exception as e:report['status']='failed';report['error']=str(e);raise
finally:(out/'summary.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'status':report['status'],'native_optimizations':3,'host_references':4}))
