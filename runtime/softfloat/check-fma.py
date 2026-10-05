#!/usr/bin/env python3
"""Fused multiply-add and rint, all modes; unchanged integer SoftFloat oracle."""
import argparse,ctypes,hashlib,json,random,subprocess
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,required=True);p.add_argument('--random-cases',type=int,default=10000);a=p.parse_args();r=Path(__file__).resolve().parent;o=a.output.resolve();o.mkdir(parents=True,exist_ok=False)
j=json.loads((r/'upstream/SHA256.json').read_text())
for n,h in j.items():assert hashlib.sha256((r/'upstream'/n).read_bytes()).hexdigest()==h,n
names=set(json.loads((r/'BINARY64_CLOSURE.json').read_text())['files']);names.update(['f32_mulAdd.c','f64_mulAdd.c','s_mulAddF32.c','s_mulAddF64.c','f32_roundToInt.c','f64_roundToInt.c','s_addM.c','s_subM.c','s_negXM.c','s_shortShiftLeftM.c','s_shortShiftRightM.c','s_shortShiftRightJamM.c','s_shiftLeftM.c','s_shiftRightJamM.c','s_addMagsF32.c','s_subMagsF32.c','s_roundPackToF32.c','s_normRoundPackToF32.c','s_normSubnormalF32Sig.c','s_shiftRightJam32.c','s_countLeadingZeros32.c','s_countLeadingZeros8.c','s_shortShiftRightJam64.c','softfloat_state.c','ARM-VFPv2/s_propagateNaNF32UI.c','ARM-VFPv2/softfloat_raiseFlags.c'])
cmd=['cc','-std=c99','-O2','-fPIC','-shared','-DINLINE_LEVEL=2','-I'+str(r/'upstream'),'-I'+str(r/'upstream/include'),'-I'+str(r/'upstream/ARM-VFPv2'),*[str(r/'upstream'/n) for n in sorted(names)],*[str(r/n) for n in ['oracle-fma.c','binary32-add.c','binary64.c','context.c']],'-o',str(o/'oracle.so')];subprocess.run(cmd,check=True)
l=ctypes.CDLL(str(o/'oracle.so'));U=ctypes.c_uint;Q=ctypes.c_ulonglong;I=ctypes.c_int
class C(ctypes.Structure):_fields_=[('flags',U),('rounding',U)]
ref=l.cosmic_sf_reference_fma;ref.argtypes=[U,U,U,Q,Q,Q,ctypes.POINTER(U)];ref.restype=Q
rng=random.Random(20261011);counts=[];vectors=[]
for width in [32,64]:
 edges=([0,1,0x80000000,0x007fffff,0x00800000,0x3f800000,0x3f000000,0x3f800001,0xbf800000,0x7f7fffff,0x7f800000,0xff800000,0x7fc12345,0x7f812345] if width==32 else [0,1,0x8000000000000000,0x000fffffffffffff,0x0010000000000000,0x3ff0000000000000,0x3fe0000000000000,0x3ff0000000000001,0xbff0000000000000,0x7fefffffffffffff,0x7ff0000000000000,0xfff0000000000000,0x7ff8000012345678,0x7ff0000012345678])
 anchors=([(0x3f800001,0x3f7ffffe,0xbf800000),(0x7f7fffff,0x40000000,0xff7fffff),(1,0x3f000000,1)] if width==32 else [(0x3ff0000000000001,0x3feffffffffffffe,0xbff0000000000000),(0x7fefffffffffffff,0x4000000000000000,0xffefffffffffffff),(1,0x3fe0000000000000,1)])
 T=U if width==32 else Q
 for mode in range(4):
  for op in [0,1]:
   f=getattr(l,'cosmic_sf_'+('fma' if op==0 else 'rint')+str(width));f.restype=T;f.argtypes=[ctypes.POINTER(C),T]+([T,T] if op==0 else [I])
   pairs=(anchors if op==0 else [])+[(x,y,z) for x in edges for y in (edges if op==0 else [0]) for z in (edges if op==0 else [0])]+[(rng.getrandbits(width),rng.getrandbits(width),rng.getrandbits(width)) for _ in range(a.random_cases)]
   for i,(x,y,z) in enumerate(pairs):
    flags=U();expected=ref(width,mode,op,x,y,z,ctypes.byref(flags))
    for initial in [0,8]:
     c=C(initial,mode);got=f(ctypes.byref(c),x,*([y,z] if op==0 else [1]));assert(got,c.flags)==(expected,flags.value|initial),(width,mode,op,hex(x),hex(y),hex(z),hex(got),hex(expected),c.flags,flags.value,initial)
    if i<len(edges) or i==len(pairs)-1:vectors.append([width,mode,op,x,y,z,expected,flags.value])
   counts.append({'width':width,'mode':mode,'operation':'fma' if op==0 else 'rint','cases':len(pairs),'fresh_and_sticky_checks':2*len(pairs)})
lines=['/* Unchanged integer SoftFloat fused operation oracle. */','struct sf_fma_vector {unsigned int width,mode,op;unsigned long long a,b,c,result;unsigned int flags;};','static const struct sf_fma_vector sf_fma_vectors[]={']
lines+=['{%du,%du,%du,0x%xULL,0x%xULL,0x%xULL,0x%xULL,%du},'%tuple(v) for v in vectors];lines+=['};','#define SF_FMA_VECTOR_COUNT %du'%len(vectors)];(o/'fma-vectors.h').write_text('\n'.join(lines)+'\n')
report={'status':'passed','cases':sum(c['cases'] for c in counts),'fresh_and_sticky_checks':sum(c['fresh_and_sticky_checks'] for c in counts),'guest_vectors':len(vectors),'operations':counts,'command':cmd,'upstream_sha256':j,'runtime_sha256':{n:hashlib.sha256((r/n).read_bytes()).hexdigest() for n in ['binary32-add.c','binary64.c','context.c','context.h','oracle-fma.c']}}
(o/'summary.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:report[k] for k in ['status','cases','fresh_and_sticky_checks','guest_vectors']}))
