#!/usr/bin/env python3
"""Unchanged integer SoftFloat3e oracle for binary64 and width/I64 conversions."""
import argparse,ctypes,hashlib,json,random,subprocess
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,required=True);p.add_argument('--random-cases',type=int,default=10000);a=p.parse_args()
root=Path(__file__).resolve().parent;out=a.output.resolve();out.mkdir(parents=True,exist_ok=False)
manifest=json.loads((root/'upstream/SHA256.json').read_text())
for n,h in manifest.items():
 if hashlib.sha256((root/'upstream'/n).read_bytes()).hexdigest()!=h:p.error('upstream hash mismatch: '+n)
closure=json.loads((root/'BINARY64_CLOSURE.json').read_text())
files=closure['files']+['s_roundPackToF32.c','s_normRoundPackToF32.c','s_normSubnormalF32Sig.c','s_shiftRightJam32.c','s_countLeadingZeros32.c','s_countLeadingZeros8.c','s_shortShiftRightJam64.c','softfloat_state.c','ARM-VFPv2/softfloat_raiseFlags.c']
cmd=['cc','-std=c99','-O2','-fPIC','-shared','-DINLINE_LEVEL=2','-I'+str(root/'upstream'),'-I'+str(root/'upstream/include'),'-I'+str(root/'upstream/ARM-VFPv2'),*[str(root/'upstream'/n) for n in files],str(root/'oracle64.c'),str(root/'binary64.c'),str(root/'context.c'),str(root/'binary64-test-wrapper.c'),'-o',str(out/'oracle.so')]
subprocess.run(cmd,check=True);lib=ctypes.CDLL(str(out/'oracle.so'));U=ctypes.c_uint;Q=ctypes.c_ulonglong
class Context(ctypes.Structure):_fields_=[('flags',U),('rounding',U)]
ref=lib.cosmic_sf_reference_binary64;ref.argtypes=[U,Q,Q,ctypes.POINTER(U)];ref.restype=Q
fn=lib.cosmic_sf_test_binary64;fn.argtypes=[ctypes.POINTER(Context),U,Q,Q];fn.restype=Q
edges64=[0,1,2,0x8000000000000000,0x000fffffffffffff,0x0010000000000000,0x0010000000000001,0x3fe0000000000000,0x3fefffffffffffff,0x3ff0000000000000,0x3ff0000000000001,0x4000000000000000,0xbff0000000000000,0x7fefffffffffffff,0xffefffffffffffff,0x7ff0000000000000,0xfff0000000000000,0x7ff8000012345678,0x7ff0000012345678,0xfff8000012345678,0xfff0000012345678,0x43e0000000000000,0xc3e0000000000000,0x43f0000000000000,0x41e0000000000000,0xc1e0000000000000]
edges32=[0,1,2,0x80000000,0x007fffff,0x00800000,0x00800001,0x3f000000,0x3f7fffff,0x3f800000,0x3f800001,0xbf800000,0x7f7fffff,0x7f800000,0xff800000,0x7fc12345,0x7f812345,0xff812345,0x5f000000,0xdf000000,0x5f800000]
ints=[0,1,2,0x7fffffff,0x80000000,0xffffffff,0x100000000,0x7fffffffffffffff,0x8000000000000000,0xffffffffffffffff,0x20000000000001,0x1000001]
anchors={0:[(0x3ff0000000000000,0x3ff0000000000000,0x4000000000000000,0),(0x3ff0000000000000,0x3ca0000000000000,0x3ff0000000000000,1)],1:[(0x3ff0000000000000,0x3ff0000000000000,0,0)],2:[(1,0x3fe0000000000000,0,3),(0,0x7ff0000000000000,0x7ff8000000000000,16)],3:[(0x3ff0000000000000,0,0x7ff0000000000000,8)],7:[(0x3f800000,0,0x3ff0000000000000,0)],8:[(0x3ff0000000000000,0,0x3f800000,0)]}
seed=20261006;rng=random.Random(seed);vectors=[];counts=[]
for op in range(21):
 edges=edges32 if op in [7,9,10] else ints if op>=15 else edges64
 count=0;flags_seen=set()
 def check(x,y,expected=None,guest=False):
  global count
  f=U();r=ref(op,x,y,ctypes.byref(f));ctx=Context(0);v=fn(ctypes.byref(ctx),op,x,y)
  assert(v,ctx.flags)==(r,f.value),(op,hex(x),hex(y),hex(v),hex(r),ctx.flags,f.value)
  if expected is not None:assert(r,f.value)==expected,('anchor',op,hex(r),f.value,expected)
  count+=1;flags_seen.add(f.value)
  if guest:vectors.append([op,x,y,r,f.value])
 for x,y,r,f in anchors.get(op,[]):check(x,y,(r,f),True)
 for x in edges:
  for y in (edges64 if op<7 else [0]):check(x,y,guest=(op>=7 or y in [0,0x3ff0000000000000,0x7ff0000000000000,0x7ff0000012345678]))
 for i in range(a.random_cases):check(rng.getrandbits(32 if op in [7,9,10,19,20] else 64),rng.getrandbits(64),guest=i<8)
 counts.append({'opcode':op,'name':closure['public'][op],'cases':count,'flags':sorted(flags_seen)})
lines=['/* Generated unchanged SoftFloat3e integer-oracle vectors. */','struct cosmic_sf_vector64 { unsigned int op; unsigned long long a,b,result; unsigned int flags; };','static const struct cosmic_sf_vector64 cosmic_sf_vectors64[] = {']
lines += ['    {%du,0x%016xULL,0x%016xULL,0x%016xULL,%du},'%tuple(v) for v in vectors];lines+=['};','#define COSMIC_SF_VECTOR64_COUNT %du'%len(vectors)]
(out/'binary64-vectors.h').write_text('\n'.join(lines)+'\n')
report={'status':'passed','seed':seed,'random_cases_per_operation':a.random_cases,'operations':counts,'guest_vectors':len(vectors),'command':cmd,'upstream_sources':manifest,'runtime_sources':{str(f.relative_to(root)):hashlib.sha256(f.read_bytes()).hexdigest() for f in [root/'binary64.c',root/'binary32.h',root/'context.c',root/'context.h',root/'oracle64.c',root/'binary64-dispatch-test.h',root/'upstream/platform.h',*sorted((root/'adapted').glob('*.inc'))]},'vector_sha256':hashlib.sha256((out/'binary64-vectors.h').read_bytes()).hexdigest()}
(out/'summary.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'status':'passed','cases':sum(c['cases'] for c in counts),'guest_vectors':len(vectors)}))
