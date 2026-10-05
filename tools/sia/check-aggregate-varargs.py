#!/usr/bin/env python3
"""Compile the aggregate/varargs fixture and execute its exact checked SIA image."""
import argparse, hashlib, json, pathlib, subprocess
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('output',type=pathlib.Path)
p.add_argument('--compiler',type=pathlib.Path,default=pathlib.Path('target/debug/cosmicc'))
p.add_argument('--linker',type=pathlib.Path,default=pathlib.Path('target/debug/cosmic-link'))
p.add_argument('--board',action='store_true')
a=p.parse_args();root=pathlib.Path(__file__).resolve().parents[2];out=a.output.resolve();out.mkdir(exist_ok=False)
compiler=a.compiler.resolve();linker=a.linker.resolve();src=root/'tools/sia/aggregate-varargs.c'
report={'compiler_sha256':hashlib.sha256(compiler.read_bytes()).hexdigest(),'linker_sha256':hashlib.sha256(linker.read_bytes()).hexdigest(),'source_sha256':hashlib.sha256(src.read_bytes()).hexdigest(),'commands':[],'status':'running'}
def run(cmd,name,cwd=root,timeout=90):
 r=subprocess.run([str(x) for x in cmd],capture_output=True,text=True,cwd=cwd,timeout=timeout)
 (out/(name+'.log')).write_text(r.stdout+r.stderr)
 report['commands'].append({'command':[str(x) for x in cmd],'returncode':r.returncode})
 if r.returncode:raise RuntimeError(name+' failed')
try:
 run([compiler,src,'-o',out/'guest.sia'],'compile')
 run([linker,'--base','0x10000','--entry','main','--max-bytes','0x60000','--container','-o',out/'image.csia',out/'guest.sia'],'link')
 run(['cargo','run','--offline','--quiet','--bin','multi-object-native','--',*(['--board',out/'guest.sia'] if a.board else ['--image',out/'image.csia'])],'execute',root/'tools/l21-execution',600 if a.board else 90)
 report['image_sha256']=hashlib.sha256((out/'image.csia').read_bytes()).hexdigest()
 report['status']='passed'
finally:(out/'summary.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report))
