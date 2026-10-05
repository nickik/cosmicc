#!/usr/bin/env python3
"""Hash-verified all-case native differential inventory; consensus is evidence, not an oracle."""
import argparse, collections, hashlib, json, os, platform, shutil, signal, subprocess
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('source',type=Path);p.add_argument('output',type=Path)
p.add_argument('--compiler',type=Path,required=True)
p.add_argument('--snapshot',type=Path,help='existing immutable compiler snapshot to reuse without copying')
p.add_argument('--gcc',default='gcc-13');p.add_argument('--clang',default='clang-18')
p.add_argument('--timeout',type=float,default=5)
a=p.parse_args()
if a.timeout<=0:p.error('timeout must be positive')
lock=Path(__file__).with_name('c-testsuite.lock.json');manifest=json.loads(lock.read_text())
source=a.source.resolve();out=a.output.resolve()
if source==out or source in out.parents:p.error('output must be outside source')
for item in manifest['files']:
 f=source/item['path']
 if not f.is_file() or hashlib.sha256(f.read_bytes()).hexdigest()!=item['sha256']:p.error('source hash mismatch '+item['path'])
actual={f.name for f in (source/'tests/single-exec').glob('*.c')}
if actual!={c['name'] for c in manifest['cases']}:p.error('source inventory mismatch')
out.mkdir(parents=True,exist_ok=False)
if a.snapshot:
 snapshot=a.snapshot.resolve()
 if not snapshot.is_file():p.error('snapshot must be an existing compiler file')
 if hashlib.sha256(snapshot.read_bytes()).digest()!=hashlib.sha256(a.compiler.resolve().read_bytes()).digest():p.error('snapshot/compiler hash mismatch')
 (out/'cosmic-snapshot').symlink_to(snapshot)
else:shutil.copy2(a.compiler.resolve(),out/'cosmic-snapshot')
compilers={'cosmic':str(out/'cosmic-snapshot'),'cosmic-O2':str(out/'cosmic-snapshot'),'gcc':shutil.which(a.gcc),'clang':shutil.which(a.clang),'gcc-O2':shutil.which(a.gcc),'clang-O2':shutil.which(a.clang)}
if not all(compilers.values()):p.error('missing reference compiler')
report={'metadata':{'runner_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'suite_commit':manifest['commit'],'lock_sha256':hashlib.sha256(lock.read_bytes()).hexdigest(),'machine':platform.platform(),'reference_dialect':'gnu11 with legacy implicit-int/function declaration diagnostics disabled','optimizations':'O0 and O2 for Cosmic and references','execution_output':'stdout and stderr merged in subprocess emission order','timeout_seconds_per_stage':a.timeout,'evidence':'native x86-64; reference consensus is not proof of defined behavior','compilers':{}},'results':[]}
for name,path in compilers.items():
 report['metadata']['compilers'][name]={'path':path,'sha256':hashlib.sha256(Path(path).read_bytes()).hexdigest(),'version':subprocess.run([path,'--version'],capture_output=True,text=True,timeout=a.timeout).stdout.strip()}
# Each stage runs in its own process group so timed-out children cannot linger.
def run(command,stem,merge_output=False):
 proc=subprocess.Popen(command,stdout=subprocess.PIPE,stderr=subprocess.STDOUT if merge_output else subprocess.PIPE,start_new_session=True)
 try:stdout,stderr=proc.communicate(timeout=a.timeout); timed=False
 except subprocess.TimeoutExpired:
  os.killpg(proc.pid,signal.SIGKILL);stdout,stderr=proc.communicate();timed=True
 (out/(stem+'.stdout')).write_bytes(stdout);(out/(stem+'.stderr')).write_bytes(stderr or b'')
 return {'command':command,'returncode':proc.returncode,'timeout':timed,'stdout_sha256':hashlib.sha256(stdout).hexdigest(),'stderr_log':stem+'.stderr'},stdout
for case in manifest['cases']:
 name=case['name'];src=source/'tests/single-exec'/name;expected=src.with_name(name+'.expected').read_bytes()
 row={'name':name,'tags':case['tags'],'profile':'reviewed-freestanding' if case['selected'] else 'outside-reviewed-profile','exclusion_reason':case.get('exclusion_reason',case.get('reason')),'source_sha256':hashlib.sha256(src.read_bytes()).hexdigest(),'expected_sha256':hashlib.sha256(expected).hexdigest(),'compilers':{}}
 for compiler,path in compilers.items():
  exe=out/(name+'.'+compiler);obj=out/(name+'.'+compiler+'.o');stem=name+'.'+compiler
  flags=['-std=gnu11','-O2' if compiler.endswith('-O2') else '-O0','-Wno-implicit-int','-Wno-implicit-function-declaration']
  commands=[('compile',[path,'-O2' if compiler.endswith('-O2') else '-O0','--target','x86_64-unknown-linux-gnu','-c',str(src),'-o',str(obj)]),('link',[compilers['gcc'],str(obj),'-lm','-o',str(exe)])] if compiler.startswith('cosmic') else [('compile-link',[path,*flags,str(src),'-lm','-o',str(exe)])]
  record={'stages':[]};row['compilers'][compiler]=record
  for stage,cmd in commands+[('execute',[str(exe)])]:
   result,stdout=run(cmd,stem+'.'+stage,merge_output=stage=='execute');result['stage']=stage;record['stages'].append(result)
   if result['timeout']:record['status']=stage+'-timeout';break
   if result['returncode']!=0:record['status']=stage+'-failure';break
   if stage=='execute':record['status']='expected-pass' if stdout==expected else 'output-mismatch'
  record['expected_match']=record['status']=='expected-pass'
  if not record['expected_match']:
   diagnostic=(out/record['stages'][-1]['stderr_log']).read_text(errors='replace')
   record['diagnostic']=diagnostic.splitlines()[:8]
   record['failure_category']='timeout' if record['stages'][-1]['timeout'] else 'compiler-crash' if 'panicked at' in diagnostic else 'unsupported-aggregate-ABI' if 'aggregate by-value' in diagnostic else 'unsupported-varargs-ABI' if 'variadic System V' in diagnostic else 'unsupported-long-double' if 'long double is unsupported' in diagnostic else 'preprocessor-diagnostic' if 'invalid macro' in diagnostic else 'unsupported-VLA-sizeof' if 'sizeof variable length array' in diagnostic else 'frontend-diagnostic' if 'invalid program' in diagnostic or 'invalid syntax' in diagnostic else 'link-failure' if record['status']=='link-failure' else record['status']
 # Upstream single-exec convention is zero exit plus exact .expected output.
 refs=[row['compilers'][c]['expected_match'] for c in ['gcc','clang','gcc-O2','clang-O2']]
 cosmic=all(row['compilers'][c]['expected_match'] for c in ['cosmic','cosmic-O2'])
 row['source_review_notes'] = {
  '00144.c': ['Reads uninitialized local i; also discards const qualification in pointer assignment. Reference agreement does not make this defined ISO C.'],
  '00178.c': ['Uses printf %d with sizeof results of type size_t; on AMD64 LP64 size_t is unsigned long, so the format/argument mismatch is undefined behavior. A separate defined fixture tests logical-not expression type and sizeof without this mismatch.'],
  '00200.c': ['Some macro-expanded expressions left-shift negative signed values, which is undefined behavior. Positive operand cases still expose an actionable shift-result typing requirement, tested separately without negative shifts.'],
 }.get(name, [])
 row['assessment']='all-match-expected' if cosmic and all(refs) else 'cosmic-gap-reference-consensus' if all(refs) else 'reference-disagreement-or-test-boundary'
 if row['source_review_notes']: row['assessment']='reviewed-undefined-or-constraint-boundary'
 report['results'].append(row)
 report['summary']={'total_cases':len(report['results']),'all_cases':{c:dict(collections.Counter(r['compilers'][c]['status'] for r in report['results'])) for c in compilers},'assessments':dict(collections.Counter(r['assessment'] for r in report['results'])),'profiles':{profile:{c:dict(collections.Counter(r['compilers'][c]['status'] for r in report['results'] if r['profile']==profile)) for c in compilers} for profile in ['reviewed-freestanding','outside-reviewed-profile']}}
 (out/'summary.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report['summary'],sort_keys=True))
