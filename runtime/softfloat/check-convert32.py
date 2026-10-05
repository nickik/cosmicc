#!/usr/bin/env python3
"""Bit/flag-exact integer-only binary32 comparison and conversion oracle gate."""
import argparse, ctypes, hashlib, json, random, subprocess
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--output',type=Path,required=True)
p.add_argument('--random-cases',type=int,default=100000)
a=p.parse_args();root=Path(__file__).resolve().parent;out=a.output.resolve();out.mkdir(parents=True,exist_ok=False)
manifest=json.loads((root/'upstream/SHA256.json').read_text())
for name,h in manifest.items():
 if hashlib.sha256((root/'upstream'/name).read_bytes()).hexdigest()!=h:p.error('upstream source changed: '+name)
files=['f32_eq.c','f32_lt.c','f32_le.c','f32_to_i32_r_minMag.c','f32_to_ui32_r_minMag.c','i32_to_f32.c','ui32_to_f32.c','s_normRoundPackToF32.c','s_roundPackToF32.c','s_countLeadingZeros32.c','s_countLeadingZeros8.c','s_shiftRightJam32.c','softfloat_state.c','ARM-VFPv2/softfloat_raiseFlags.c']
cmd=['cc','-std=c99','-O2','-fPIC','-shared','-DINLINE_LEVEL=1','-I'+str(root/'upstream'),'-I'+str(root/'upstream/include'),'-I'+str(root/'upstream/ARM-VFPv2'),*[str(root/'upstream'/f) for f in files],str(root/'conversion-oracle.c'),str(root/'binary32-convert.c'),str(root/'context.c'),'-o',str(out/'oracle.so')]
subprocess.run(cmd,check=True);lib=ctypes.CDLL(str(out/'oracle.so'));U=ctypes.c_uint;I=ctypes.c_int
class Context(ctypes.Structure):_fields_=[('flags',U),('rounding',U)]
ref=lib.cosmic_sf_reference_convert;ref.argtypes=[U,U,U,ctypes.POINTER(U)];ref.restype=U
names=['eq32','lt32','le32','to_i32','to_u32','from_i32','from_u32'];functions=[]
for op,name in enumerate(names):
 f=getattr(lib,'cosmic_sf_'+name);f.argtypes=[ctypes.POINTER(Context),I if op==5 else U]+([U] if op<3 else []);f.restype=I if op in [0,1,2,3] else U;functions.append(f)
edges=[0,0x80000000,1,0x007fffff,0x00800000,0x3f7fffff,0x3f800000,0x3fc00000,0xbf800000,0xbfc00000,0x4effffff,0x4f000000,0xcf000000,0xcf000001,0x4f7fffff,0x4f800000,0x7f7fffff,0xff7fffff,0x7f800000,0xff800000,0x7fc12345,0x7f812345,0xffc12345,0x7fffffff,0xffffffff,0x01000001,0x01ffffff]
pairs=[(x,y) for x in edges for y in edges];seed=20261005;rng=random.Random(seed);pairs += [(rng.getrandbits(32),rng.getrandbits(32)) for _ in range(a.random_cases)]
vectors=[]
for i,(x,y) in enumerate(pairs):
 for op,f in enumerate(functions):
  flags=U();expected=ref(op,x,y,ctypes.byref(flags));ctx=Context(0)
  actual=f(ctypes.byref(ctx),x,y) if op<3 else f(ctypes.byref(ctx),x)
  if (actual&0xffffffff)!=expected or ctx.flags!=flags.value:raise RuntimeError((op,hex(x),hex(y),actual,expected,ctx.flags,flags.value))
  ctx=Context(8)
  sticky=f(ctypes.byref(ctx),x,y) if op<3 else f(ctypes.byref(ctx),x)
  if (sticky&0xffffffff)!=expected or ctx.flags!=(flags.value|8):raise RuntimeError('sticky context mismatch')
  if i<len(edges)**2 or i<len(edges)**2+32:vectors.append((op,x,y,expected,flags.value))
lines=['/* Generated integer-only SoftFloat3e reference vectors. */','static const unsigned int cosmic_sf_convert_vectors[][5]={']
lines+=['{'+','.join('0x%08xu'%v for v in row)+'},' for row in vectors];lines+=['};','#define COSMIC_SF_CONVERT_COUNT %du'%len(vectors)]
(out/'convert32-vectors.h').write_text('\n'.join(lines)+'\n')
report={'status':'passed','seed':seed,'operations':names,'edge_pairs':len(edges)**2,'random_pairs':a.random_cases,'comparisons':len(pairs)*7,'guest_vectors':len(vectors),'vector_sha256':hashlib.sha256((out/'convert32-vectors.h').read_bytes()).hexdigest(),'command':cmd,'upstream_sources':manifest,'source_sha256':hashlib.sha256((root/'binary32-convert.c').read_bytes()).hexdigest()}
(out/'summary.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:report[k] for k in ['status','comparisons','guest_vectors']}))
