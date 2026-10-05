#!/usr/bin/env python3
"""Finite-input bit comparisons to independent host libm, used only as oracle."""
import argparse,ctypes,hashlib,json,random,subprocess
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,required=True);p.add_argument('--random-cases',type=int,default=10000);a=p.parse_args();r=Path(__file__).resolve().parent;o=a.output.resolve();o.mkdir(parents=True,exist_ok=False)
cmd=['cc','-std=c99','-O2','-fno-builtin','-frounding-math','-shared','-fPIC',*[str(r/n) for n in ['math-reference-test.c','math-bits.c','binary32-convert.c','binary32-add.c','binary64.c','context.c']],'-lm','-o',str(o/'oracle.so')];subprocess.run(cmd,check=True)
l=ctypes.CDLL(str(o/'oracle.so'));U=ctypes.c_uint;Q=ctypes.c_ulonglong;I=ctypes.c_int
class C(ctypes.Structure):_fields_=[('flags',U),('rounding',U)]
ref=l.cosmic_math_reference;ref.argtypes=[U,U,Q,Q,I,U,ctypes.POINTER(I)];ref.restype=Q
names=['fmod','floor','ceil','trunc','round','fabs','copysign','scalbn','frexp','remquo','nextafter'];rng=random.Random(20261009);counts=[];vectors=[]
for width in [32,64]:
 pbits,eb=(23,8) if width==32 else (52,11);maxexp=(1<<eb)-1
 edges=[0,1,1<<(width-1),(1<<pbits)-1,1<<pbits,((maxexp//2)<<pbits),((maxexp//2)<<pbits)|(1<<(pbits-1)),((maxexp-1)<<pbits)|((1<<pbits)-1)]
 edges+=[v|(1<<(width-1)) for v in edges]
 for op,name in enumerate(names):
  f=getattr(l,'cosmic_sf_'+name+str(width));f.restype=U if width==32 else Q;f.argtypes=[ctypes.POINTER(C),U if width==32 else Q]+([U if width==32 else Q] if op in [0,6,9,10] else [I] if op==7 else [ctypes.POINTER(I)] if op in [8,9] else [])
  if op==9:f.argtypes=[ctypes.POINTER(C),U if width==32 else Q,U if width==32 else Q,ctypes.POINTER(I)]
  count=0
  for mode in range(4):
   pairs=[(x,y,n) for x in edges for y in (edges if op in [0,6,9,10] else [0]) for n in ([-2000,-1,0,1,2000] if op==7 else [0])]
   for _ in range(a.random_cases):
    x=rng.getrandbits(width);y=rng.getrandbits(width)
    if((x>>pbits)&maxexp)==maxexp:x &= ~(1<<pbits)
    if((y>>pbits)&maxexp)==maxexp:y &= ~(1<<pbits)
    pairs.append((x,y,rng.randint(-2200,2200)))
   for i,(x,y,n) in enumerate(pairs):
    if op in [0,9] and (y&((1<<(width-1))-1))==0:continue
    e=I();expected=ref(width,op,x,y,n,mode,ctypes.byref(e));ce=I();c=C(0,mode)
    arg=[y,ctypes.byref(ce)] if op==9 else [y] if op in [0,6,9,10] else [n] if op==7 else [ctypes.byref(ce)] if op in [8,9] else []
    got=f(ctypes.byref(c),x,*arg)
    assert(got, (abs(ce.value)&7) if op==9 else ce.value)==(expected,(abs(e.value)&7) if op==9 else e.value),(width,mode,name,hex(x),hex(y),n,hex(got),hex(expected),ce.value,e.value)
    if op==9 and (abs(e.value)&7):assert (ce.value<0)==(e.value<0)
    count+=1
    if i<len(edges) or i==len(pairs)-1:vectors.append([width,mode,op,x,y,n,expected,e.value,c.flags])
  counts.append({'width':width,'operation':name,'cases':count})
lines=['/* Independent host libm oracle vectors; guest implementation integer-only. */','struct sf_math_vector { unsigned int width,mode,op;unsigned long long a,b;int n;unsigned long long result;int exponent;unsigned int flags; };','static const struct sf_math_vector sf_math_vectors[]={']
lines+=['{%du,%du,%du,0x%xULL,0x%xULL,%d,0x%xULL,%d,%du},'%tuple(v) for v in vectors];lines+=['};','#define SF_MATH_VECTOR_COUNT %du'%len(vectors)];(o/'math-vectors.h').write_text('\n'.join(lines)+'\n')
report={'status':'passed','cases':sum(x['cases'] for x in counts),'guest_vectors':len(vectors),'operations':counts,'reference':'host libm finite-input output bits; flags separately tested by directed fixtures','command':cmd,'source_sha256':{n:hashlib.sha256((r/n).read_bytes()).hexdigest() for n in ['math-reference-test.c','math-bits.c','binary32-convert.c','binary32-add.c','binary64.c','context.c']}}
(o/'summary.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'status':'passed','cases':report['cases'],'guest_vectors':len(vectors)}))
