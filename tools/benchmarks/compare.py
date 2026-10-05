#!/usr/bin/env python3
"""Serial, affinity-pinned end-to-end compilation and validated native kernel timings."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import random
import shutil
import statistics
import subprocess
import time

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--compiler',type=Path,required=True,help='Cosmic release executable')
p.add_argument('--output',type=Path,required=True)
p.add_argument('--phase',choices=['prepare','measure'],required=True)
p.add_argument('--zlib',type=Path)
p.add_argument('--iterations',type=int,default=20000)
p.add_argument('--compile-repeats',type=int,default=7)
p.add_argument('--runtime-repeats',type=int,default=7)
p.add_argument('--zlib-repeats',type=int,default=3)
p.add_argument('--cpu',type=int)
a=p.parse_args()
root=Path(__file__).resolve().parent
out=a.output.resolve()
units=['adler32','crc32','deflate','infback','inffast','inflate','inftrees','trees','zutil']
allowed=sorted(os.sched_getaffinity(0))
cpu=a.cpu if a.cpu is not None else allowed[0]
if cpu not in allowed: p.error('CPU must be in current permitted affinity set')
if min(a.iterations,a.compile_repeats,a.runtime_repeats,a.zlib_repeats)<1: p.error('counts must be positive')
os.sched_setaffinity(0,{cpu})
rng=random.Random(20261005)
if a.phase=='prepare':
    out.mkdir(parents=True,exist_ok=False)
    shutil.copy2(a.compiler.resolve(),out/'cosmic-snapshot')
else:
    if not (out/'prepare.json').is_file(): p.error('run prepare first')
variants=[]
for tool in ['cosmic','gcc','clang']:
    for level in ['O0','O2']:
        executable=str(out/'cosmic-snapshot') if tool=='cosmic' else shutil.which(tool+'-13' if tool=='gcc' else 'clang-18')
        if not executable: p.error('missing '+tool+' executable')
        variants.append({'name':tool+'-'+level,'tool':tool,'level':level,'executable':executable})

def command(variant,source,object_path,zlib=False):
    result=[variant['executable']]
    if variant['tool']=='cosmic': result+=['--target','amd64']
    else: result+=['-fPIC']
    result+=['-'+variant['level'],'-c',str(source),'-o',str(object_path)]
    if zlib: result+=['-DZ_SOLO=1','-I',str(a.zlib.resolve())]
    return result

command_records=[]
def run(cmd,log,timeout=120):
    start=time.perf_counter_ns()
    r=subprocess.run(cmd,capture_output=True,timeout=timeout)
    elapsed=(time.perf_counter_ns()-start)/1e9
    command_records.append({'command':cmd,'log':str(log),'timeout_seconds':timeout})
    (out/log).write_bytes(r.stdout+r.stderr)
    if r.returncode: raise RuntimeError('command failed: '+repr(cmd)+'; log='+str(out/log))
    return elapsed,r.stdout

def metadata():
    frequency=Path('/sys/devices/system/cpu')/('cpu'+str(cpu))/'cpufreq'
    details={}
    for n in ['scaling_governor','scaling_driver','cpuinfo_min_freq','cpuinfo_max_freq','scaling_cur_freq']:
        path=frequency/n
        details[n]=path.read_text().strip() if path.exists() else None
    return {'platform':platform.platform(),'cpu_model':next((line.split(':',1)[1].strip() for line in Path('/proc/cpuinfo').read_text().splitlines() if line.startswith('model name')),None),
            'logical_cpus':os.cpu_count(),'original_allowed_cpus':allowed,'measurement_cpu':cpu,
            'affinity':sorted(os.sched_getaffinity(0)),'cpufreq':details,
            'boost':Path('/sys/devices/system/cpu/cpufreq/boost').read_text().strip() if Path('/sys/devices/system/cpu/cpufreq/boost').exists() else None,
            'loadavg':os.getloadavg(),'no_system_settings_changed':True}

source_hashes={n:hashlib.sha256((root/n).read_bytes()).hexdigest() for n in ['kernels.c','harness.c','compare.py']}
if a.zlib:
    source_hashes['zlib']={n:hashlib.sha256((a.zlib/(n+'.c')).read_bytes()).hexdigest() for n in units}
tool_hashes={v['tool']:hashlib.sha256(Path(v['executable']).resolve().read_bytes()).hexdigest() for v in variants}
system_component_hashes={}
for component in ['cc1','collect2','ld']:
    path=subprocess.run(['gcc-13','-print-prog-name='+component],capture_output=True,text=True,check=True).stdout.strip()
    resolved=Path(shutil.which(path) or path).resolve()
    system_component_hashes[component]={'path':str(resolved),'sha256':hashlib.sha256(resolved.read_bytes()).hexdigest()}
versions={v['tool']:subprocess.run([v['executable'],'--version'],capture_output=True,text=True,check=True).stdout.splitlines()[0] for v in variants}
if a.phase=='prepare':
    report={'machine':metadata(),'versions':versions,'tool_hashes':tool_hashes,'system_component_hashes':system_component_hashes,'sources':source_hashes,
            'compiler_sha256':hashlib.sha256((out/'cosmic-snapshot').read_bytes()).hexdigest(),
            'cosmic_generated_optimization':{'O0':'Cranelift none','O2':'Cranelift speed (not equivalent optimization pipeline to GCC/Clang O2)'},
            'iterations':a.iterations,'variants':variants,'commands':command_records}
    run(['gcc-13','-O2','-c',str(root/'harness.c'),'-o',str(out/'harness.o')],'harness-build.log')
    for v in variants:
        obj=out/(v['name']+'.o')
        run(command(v,root/'kernels.c',obj),v['name']+'-prepare.log')
        run(['gcc-13',str(out/'harness.o'),str(obj),'-o',str(out/v['name'])],v['name']+'-link-prepare.log')
        _,data=run([str(out/v['name']),str(a.iterations)],v['name']+'-validate.log')
        result=json.loads(data)
        checksums={'validation':result['validation'],'aggregate':{k:x['checksum'] for k,x in result['kernels'].items()}}
        if v['name']=='gcc-O2': report['reference_checksums']=checksums
        report.setdefault('validation',{})[v['name']]=checksums
    if any(x!=report['reference_checksums'] for x in report['validation'].values()): raise RuntimeError('kernel checksum mismatch')
    report['artifact_hashes']={n:hashlib.sha256((out/n).read_bytes()).hexdigest() for n in ['harness.o']+[v['name']+suffix for v in variants for suffix in ['', '.o']]}
    (out/'prepare.json').write_text(json.dumps(report,indent=2)+'\n')
    print('PREPARED all six variants; kernel checksums match GCC O2 reference. No timing comparison yet.')
    raise SystemExit(0)

prepare=json.loads((out/'prepare.json').read_text())
if prepare['system_component_hashes']!=system_component_hashes: p.error('GCC backend/linker components changed')
if prepare['tool_hashes']!=tool_hashes or prepare['versions']!=versions: p.error('compiler binaries/versions changed')
for n,digest in prepare['artifact_hashes'].items():
    if hashlib.sha256((out/n).read_bytes()).hexdigest()!=digest: p.error('prepared artifact changed: '+n)
if prepare['sources']!=source_hashes or prepare['iterations']!=a.iterations: p.error('sources/iterations differ from prepared artifacts')
if prepare['compiler_sha256']!=hashlib.sha256((out/'cosmic-snapshot').read_bytes()).hexdigest(): p.error('compiler snapshot changed')
report={'prepare':prepare,'machine_start':metadata(),'random_seed':20261005,
        'repeats':{'compile':a.compile_repeats,'runtime':a.runtime_repeats,'zlib':a.zlib_repeats},
        'samples':{'compile':{},'link':{},'runtime':{},'zlib':{}},'order':[],'commands':command_records}

def persist(): (out/'results.json').write_text(json.dumps(report,indent=2)+'\n')
def sample(phase,v,index):
    name=v['name']; report['order'].append({'phase':phase,'variant':name,'index':index})
    if phase=='compile':
        elapsed,_=run(command(v,root/'kernels.c',out/(name+'.o')),name+'-compile-'+str(index)+'.log')
    elif phase=='link':
        elapsed,_=run(['gcc-13',str(out/'harness.o'),str(out/(name+'.o')),'-o',str(out/name)],name+'-link-'+str(index)+'.log')
    elif phase=='runtime':
        _,data=run([str(out/name),str(a.iterations)],name+'-runtime-'+str(index)+'.log')
        result=json.loads(data)
        if {'validation':result['validation'],'aggregate':{k:x['checksum'] for k,x in result['kernels'].items()}}!=prepare['reference_checksums']: raise RuntimeError('timed checksum mismatch')
        elapsed={k:x['seconds'] for k,x in result['kernels'].items()}
    else:
        elapsed=0.0
        for unit in units:
            t,_=run(command(v,a.zlib/(unit+'.c'),out/(name+'-'+unit+'.o'),True),name+'-zlib-'+unit+'-'+str(index)+'.log')
            elapsed+=t
    if index>=0: report['samples'][phase].setdefault(name,[]).append(elapsed)
    persist()

for phase,repeats in [('compile',a.compile_repeats),('link',a.compile_repeats),('runtime',a.runtime_repeats),('zlib',a.zlib_repeats)]:
    if phase=='runtime':
        report['runtime_artifact_hashes']={n:hashlib.sha256((out/n).read_bytes()).hexdigest() for n in ['harness.o']+[v['name']+suffix for v in variants for suffix in ['', '.o']]}
    if phase=='zlib' and not a.zlib: continue
    for index in range(-1,repeats):
        order=list(variants); rng.shuffle(order)
        for v in order: sample(phase,v,index)

def stats(values):
    q=statistics.quantiles(values,n=4,method='inclusive') if len(values)>1 else [values[0]]*3
    return {'median_seconds':statistics.median(values),'min_seconds':min(values),'max_seconds':max(values),'q1_seconds':q[0],'q3_seconds':q[2],'samples':len(values)}
report['statistics']={}
for phase,rows in report['samples'].items():
    report['statistics'][phase]={}
    for name,values in rows.items():
        report['statistics'][phase][name]={k:stats([x[k] for x in values]) for k in values[0]} if phase=='runtime' else stats(values)
report['machine_end']=metadata()
report['status']='passed'
persist()
print('MEASURED validated native runtimes and separate compile/link timings: '+str(out/'results.json'))
