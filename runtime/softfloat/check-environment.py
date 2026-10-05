#!/usr/bin/env python3
"""All four rounding modes, raw integer SoftFloat oracle; no host FP math."""
import argparse,ctypes,hashlib,json,random,subprocess
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,required=True);p.add_argument('--random-cases',type=int,default=10000);a=p.parse_args()
r=Path(__file__).resolve().parent;o=a.output.resolve();o.mkdir(parents=True,exist_ok=False)
j=json.loads((r/'upstream/SHA256.json').read_text())
for n,h in j.items():assert hashlib.sha256((r/'upstream'/n).read_bytes()).hexdigest()==h,n
names=set(json.loads((r/'BINARY64_CLOSURE.json').read_text())['files'])
names.update(['i32_to_f32.c','ui32_to_f32.c','f32_add.c','f32_sub.c','f32_mul.c','f32_div.c','f32_sqrt.c','f64_sqrt.c','s_approxRecipSqrt32_1.c','s_approxRecipSqrt_1Ks.c','s_addMagsF32.c','s_subMagsF32.c','s_roundPackToF32.c','s_normRoundPackToF32.c','s_normSubnormalF32Sig.c','s_shiftRightJam32.c','s_countLeadingZeros32.c','s_countLeadingZeros8.c','s_shortShiftRightJam64.c','softfloat_state.c','ARM-VFPv2/s_propagateNaNF32UI.c','ARM-VFPv2/softfloat_raiseFlags.c'])
cmd=['cc','-std=c99','-O2','-shared','-fPIC','-DINLINE_LEVEL=2','-DSOFTFLOAT_FAST_DIV64TO32=1','-I'+str(r/'upstream'),'-I'+str(r/'upstream/include'),'-I'+str(r/'upstream/ARM-VFPv2'),*[str(r/'upstream'/n) for n in sorted(names)],*[str(r/n) for n in ['oracle-environment.c','binary32-add.c','binary32-convert.c','binary64.c','context.c','fenv.c','math-bits.c','binary64-test-wrapper.c']],'-o',str(o/'reference.so')]
subprocess.run(cmd,check=True);l=ctypes.CDLL(str(o/'reference.so'));U=ctypes.c_uint;Q=ctypes.c_ulonglong
class Context(ctypes.Structure):_fields_=[('flags',U),('rounding',U)]
ref64=l.cosmic_sf_reference_environment64;ref64.argtypes=[U,U,Q,Q,ctypes.POINTER(U)];ref64.restype=Q
ref32=l.cosmic_sf_reference_environment32;ref32.argtypes=[U,U,U,U,ctypes.POINTER(U)];ref32.restype=U
f64=l.cosmic_sf_test_binary64;f64.argtypes=[ctypes.POINTER(Context),U,Q,Q];f64.restype=Q
sqrt64=l.cosmic_sf_sqrt64;sqrt64.argtypes=[ctypes.POINTER(Context),Q];sqrt64.restype=Q
fs=[]
for n in ['add32','sub32','mul32','div32','sqrt32','from_i32','from_u32']:
 f=getattr(l,'cosmic_sf_'+n);f.restype=U;f.argtypes=[ctypes.POINTER(Context),U]+([] if n in ['sqrt32','from_i32','from_u32'] else [U]);fs.append(f)
edges64=[0,1,0x8000000000000000,0x000fffffffffffff,0x0010000000000000,0x3ff0000000000000,0x3ca0000000000000,0xbff0000000000000,0x7fefffffffffffff,0x7ff0000000000000,0xfff0000000000000,0x7ff8000012345678,0x7ff0000012345678,0x7fffffffffffffff,0xffffffffffffffff]
edges32=[0,1,0x80000000,0x007fffff,0x00800000,0x3f800000,0x33800000,0xbf800000,0x7f7fffff,0x7f800000,0xff800000,0x7fc12345,0x7f812345,0x7fffffff,0xffffffff,0x1000001]
rng=random.Random(20261008);counts=[];vectors=[]
for mode in range(4):
 for width,total,edges in [(32,7,edges32),(64,22,edges64)]:
  for op in range(total):
   pairs=[(x,y) for x in edges for y in (edges if op<4 else [0])]+[(rng.getrandbits(width),rng.getrandbits(width)) for _ in range(a.random_cases)]
   for i,(x,y) in enumerate(pairs):
    flags=U();expected=(ref32 if width==32 else ref64)(mode,op,x,y,ctypes.byref(flags));c=Context(0,mode)
    got=(fs[op](ctypes.byref(c),x,*([y] if op<4 else [])) if width==32 else sqrt64(ctypes.byref(c),x) if op==21 else f64(ctypes.byref(c),op,x,y))
    assert(got,c.flags)==(expected,flags.value),(width,mode,op,hex(x),hex(y),hex(got),hex(expected),c.flags,flags.value)
    if i< len(edges) or i==len(pairs)-1:vectors.append([width,mode,op,x,y,expected,flags.value])
   counts.append({'width':width,'rounding':mode,'operation':op,'cases':len(pairs)})
lines=['/* Integer SoftFloat oracle, all four rounding modes. */','struct sf_env_vector { unsigned int width,mode,op; unsigned long long a,b,result;unsigned int flags; };','static const struct sf_env_vector sf_env_vectors[]={']
lines += ['{%du,%du,%du,0x%xULL,0x%xULL,0x%xULL,%du},'%tuple(v) for v in vectors];lines+=['};','#define SF_ENV_VECTOR_COUNT %du'%len(vectors)]
(o/'environment-vectors.h').write_text('\n'.join(lines)+'\n')
report={'status':'passed','cases':sum(x['cases'] for x in counts),'guest_vectors':len(vectors),'operations':counts,'command':cmd,'runtime_sha256':{n:hashlib.sha256((r/n).read_bytes()).hexdigest() for n in ['context.c','context.h','fenv.c','fenv.h','binary32-add.c','binary32-convert.c','binary64.c','math-bits.c','oracle-environment.c']},'upstream_sha256':j}
(o/'summary.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'status':'passed','cases':report['cases'],'guest_vectors':len(vectors)}))
