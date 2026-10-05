#!/usr/bin/env python3
"""GCC13/Clang18 System V incoming va_list interoperability with Cosmic scalar and MEMORY arguments."""
import argparse, hashlib, json, pathlib, shutil, subprocess
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--compiler',type=pathlib.Path,required=True)
p.add_argument('--output',type=pathlib.Path,required=True)
a=p.parse_args(); out=a.output.resolve(); out.mkdir(parents=True,exist_ok=False)
root=pathlib.Path(__file__).resolve().parent
compiler=out/'compiler-snapshot'; shutil.copy2(a.compiler.resolve(),compiler)
report={'compiler_sha256':hashlib.sha256(compiler.read_bytes()).hexdigest(),'sources':{n:hashlib.sha256((root/n).read_bytes()).hexdigest() for n in ['stdarg-cosmic.c','stdarg-host.c']},'commands':[],'status':'running','scope':'Bidirectional GP/SSE/overflow va_list traversal, va_copy, promoted scalars and MEMORY aggregates'}
def run(cmd,name):
    r=subprocess.run([str(x) for x in cmd],capture_output=True,text=True,timeout=30)
    (out/(name+'.log')).write_text(r.stdout+r.stderr)
    report['commands'].append({'command':[str(x) for x in cmd],'returncode':r.returncode,'log':name+'.log'})
    (out/'summary.json').write_text(json.dumps(report,indent=2)+'\n')
    if r.returncode: raise RuntimeError(name+' failed: '+r.stderr)
try:
    # Check the reference fixture itself at each host compiler/optimization.
    for host in ['gcc-13','clang-18']:
        for host_opt in ['O0','O2']:
            name='reference-'+host+'-'+host_opt
            exe=out/name
            run([host,'-'+host_opt,root/'stdarg-host.c',root/'stdarg-cosmic.c','-o',exe],name+'-link')
            run([exe],name+'-execute')
    for cosmic_opt in ['O0','O2']:
        obj=out/('cosmic-'+cosmic_opt+'.o')
        run([compiler,'-'+cosmic_opt,'--target','amd64','-c',root/'stdarg-cosmic.c','-o',obj],'compile-'+cosmic_opt)
        run(['objdump','-dr',obj],'disassembly-'+cosmic_opt)
        for host in ['gcc-13','clang-18']:
            run([host,'--version'],host+'-version')
            for host_opt in ['O0','O2']:
                name=cosmic_opt+'-'+host+'-'+host_opt
                exe=out/name
                run([host,'-'+host_opt,root/'stdarg-host.c',obj,'-o',exe],name+'-link')
                run([exe],name+'-execute')
    report['status']='passed';report['executed_combinations']=8
except Exception as e:
    report['status']='failed';report['error']=str(e);raise
finally: (out/'summary.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'status':report['status'],'executed_combinations':report.get('executed_combinations',0)}))
