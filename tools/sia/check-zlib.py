#!/usr/bin/env python3
"""Build unchanged Z_SOLO C objects, checked-link, and execute on LightingMachine."""
import argparse, hashlib, json, pathlib, shutil, subprocess
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('source',type=pathlib.Path)
p.add_argument('output',type=pathlib.Path)
p.add_argument('--compiler',type=pathlib.Path,default=pathlib.Path('target/debug/cosmicc'))
p.add_argument('--linker',type=pathlib.Path,default=pathlib.Path('target/debug/cosmic-link'))
p.add_argument('--board',action='store_true')
a=p.parse_args(); out=a.output.resolve(); out.mkdir(exist_ok=False)
root=pathlib.Path(__file__).resolve().parents[2]
compiler=out/'compiler-snapshot'; shutil.copy2(a.compiler.resolve(),compiler)
linker=out/'linker-snapshot'; shutil.copy2(a.linker.resolve(),linker)
report={'compiler_sha256':hashlib.sha256(compiler.read_bytes()).hexdigest(),'linker_sha256':hashlib.sha256(linker.read_bytes()).hexdigest(),'sources':{},'commands':[],'status':'running'}
def run(cmd,name,timeout=40,cwd=None):
    try:
        r=subprocess.run([str(x) for x in cmd],capture_output=True,text=True,timeout=timeout,cwd=cwd)
        (out/(name+'.log')).write_text(r.stdout+r.stderr)
        report['commands'].append({'command':[str(x) for x in cmd],'returncode':r.returncode})
        if r.returncode: raise RuntimeError(name+' failed; see '+str(out/(name+'.log')))
    finally: (out/'summary.json').write_text(json.dumps(report,indent=2)+'\n')
try:
    objects=[]
    for name in 'guest adler32 crc32 deflate infback inffast inflate inftrees trees zutil'.split():
        src=root/'tools/sia/zlib-roundtrip.c' if name=='guest' else a.source.resolve()/(name+'.c')
        report['sources'][name]=hashlib.sha256(src.read_bytes()).hexdigest()
        obj=out/(name+'.sia'); objects.append(obj)
        run([compiler,'-D','Z_SOLO=1','-I',a.source.resolve(),src,'-o',obj],name)
    run([linker,'--base','0x10000','--entry','main','--max-bytes','0x60000','--container','-o',out/'image.csia',*objects],'link')
    report['objects_sha256']={obj.name:hashlib.sha256(obj.read_bytes()).hexdigest() for obj in objects}
    report['image_sha256']=hashlib.sha256((out/'image.csia').read_bytes()).hexdigest()
    run(['cargo','run','--offline','--quiet','--bin','multi-object-native','--',*(['--board',*objects] if a.board else ['--image',out/'image.csia'])],'execute',timeout=600 if a.board else 60,cwd=root/'tools/l21-execution')
    report['status']='passed'
except Exception as e:
    report['status']='failed'; report['error']=str(e); raise
finally: (out/'summary.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'status':report['status'],'objects':len(objects),'board':a.board}))
