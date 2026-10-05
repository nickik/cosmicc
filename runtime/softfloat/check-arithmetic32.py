#!/usr/bin/env python3
"""Integer-only binary32 sub/mul/div differential oracle and bounded guest vectors."""
import argparse,ctypes,hashlib,json,random,subprocess
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,required=True);p.add_argument('--random-cases',type=int,default=100000);a=p.parse_args()
root=Path(__file__).resolve().parent;out=a.output.resolve();out.mkdir(parents=True,exist_ok=False)
manifest=json.loads((root/'upstream/SHA256.json').read_text())
for n,h in manifest.items():
 if hashlib.sha256((root/'upstream'/n).read_bytes()).hexdigest()!=h:p.error('upstream hash mismatch: '+n)
files=['f32_sub.c','f32_mul.c','f32_div.c','s_addMagsF32.c','s_subMagsF32.c','s_roundPackToF32.c','s_normRoundPackToF32.c','s_normSubnormalF32Sig.c','s_shortShiftRightJam64.c','s_shiftRightJam32.c','s_countLeadingZeros32.c','s_countLeadingZeros8.c','softfloat_state.c','ARM-VFPv2/s_propagateNaNF32UI.c','ARM-VFPv2/softfloat_raiseFlags.c']
cmd=['cc','-std=c99','-O2','-fPIC','-shared','-DSOFTFLOAT_FAST_DIV64TO32=1','-I'+str(root/'upstream'),'-I'+str(root/'upstream/include'),'-I'+str(root/'upstream/ARM-VFPv2'),*[str(root/'upstream'/n) for n in files],str(root/'oracle-arithmetic.c'),str(root/'binary32-add.c'),str(root/'context.c'),'-o',str(out/'oracle.so')]
subprocess.run(cmd,check=True);lib=ctypes.CDLL(str(out/'oracle.so'));U=ctypes.c_uint
class Context(ctypes.Structure):_fields_=[('flags',U),('rounding',U)]
lib.cosmic_sf_reference_arithmetic32.argtypes=[U,U,U,ctypes.POINTER(U)];lib.cosmic_sf_reference_arithmetic32.restype=U
functions=[lib.cosmic_sf_sub32,lib.cosmic_sf_mul32,lib.cosmic_sf_div32]
for f in functions:f.argtypes=[ctypes.POINTER(Context),U,U];f.restype=U
anchors=[[(0x3f800000,0x3f800000,0,0),(0x3f800000,0xbf800000,0x40000000,0),(0x7f800000,0x7f800000,0x7fc00000,16),(0x7fc12345,0xff812345,0xffc12345,16),(0x80000000,0,0x80000000,0)],
 [(0x3fc00000,0x40000000,0x40400000,0),(0x00800000,0x3f7fffff,0x00800000,3),(1,0x3f000000,0,3),(0x7f7fffff,0x40000000,0x7f800000,5),(0,0x7f800000,0x7fc00000,16),(0x80000000,0x3f800000,0x80000000,0)],
 [(0x3f800000,0x40000000,0x3f000000,0),(0x3f800000,0,0x7f800000,8),(0,0,0x7fc00000,16),(1,0x40000000,0,3),(0x3f800000,0x40400000,0x3eaaaaab,1),(0x80000000,0x3f800000,0x80000000,0)]]
edges=[0,0x80000000,1,2,0x007fffff,0x00800000,0x00800001,0x3f000000,0x3f7fffff,0x3f800000,0x3f800001,0x40000000,0x33800000,0x33000000,0x7f7fffff,0xff7fffff,0x7f800000,0xff800000,0x7fc12345,0xffc12345,0x7f812345,0xff812345,0x7fffffff,0xffffffff,0xbf800000,0x80800000]
seed=20261006;rng=random.Random(seed);vectors=[];counts=[]
for op,fn in enumerate(functions):
 count=0;flagsets=set()
 def check(x,y,expected=None,guest=False):
  global count
  flags=U();ref=lib.cosmic_sf_reference_arithmetic32(op,x,y,ctypes.byref(flags))
  if expected is not None:assert(ref,flags.value)==expected,(op,hex(x),hex(y),'anchor',hex(ref),flags.value,expected)
  ctx=Context(0);got=fn(ctypes.byref(ctx),x,y)
  assert(got,ctx.flags)==(ref,flags.value),(op,hex(x),hex(y),hex(got),hex(ref),ctx.flags,flags.value)
  flagsets.add(flags.value);count+=1
  if guest:vectors.append([op,x,y,ref,flags.value])
 for x,y,r,f in anchors[op]:check(x,y,(r,f),True)
 for x in edges:
  for y in edges:check(x,y,guest=y in [0,0x3f800000,0xbf800000,0x7f800000])
 for i in range(a.random_cases):check(rng.getrandbits(32),rng.getrandbits(32),guest=i<64)
 counts.append({'op':['sub','mul','div'][op],'cases':count,'observed_flags':sorted(flagsets)})
lines=['/* Integer-only unchanged SoftFloat3e reference-generated vectors. */','static const unsigned int cosmic_sf_arithmetic32_vectors[][5] = {']
lines += ['    {'+', '.join('0x%08xu'%v for v in row)+'},' for row in vectors];lines+=['};','#define COSMIC_SF_ARITHMETIC32_VECTOR_COUNT %du'%len(vectors)]
(out/'arithmetic32-vectors.h').write_text('\n'.join(lines)+'\n')
report={'status':'passed','oracle':'unchanged SoftFloat3e ARM-VFPv2 integer closure, FAST_DIV64TO32','seed':seed,'random_cases_per_operation':a.random_cases,'operations':counts,'guest_vectors':len(vectors),'command':cmd,'upstream_sources':manifest,'runtime_source_sha256':{str(f.relative_to(root)):hashlib.sha256(f.read_bytes()).hexdigest() for f in [root/'binary32-add.c',root/'context.c',root/'context.h',root/'binary32.h',*sorted((root/'adapted').glob('*.inc'))]},'vector_sha256':hashlib.sha256((out/'arithmetic32-vectors.h').read_bytes()).hexdigest()}
(out/'summary.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:report[k] for k in ['status','operations','guest_vectors']}))
