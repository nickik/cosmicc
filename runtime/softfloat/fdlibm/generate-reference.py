#!/usr/bin/env python3
"""Generate unchanged fdlibm host-arithmetic vectors, guest never uses host FP."""
import argparse,ctypes,hashlib,json,random,subprocess
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,required=True);p.add_argument('--random-cases',type=int,default=4);a=p.parse_args();r=Path(__file__).resolve().parent;o=a.output.resolve();o.mkdir(parents=True,exist_ok=False)
j=json.loads((r/'SHA256.json').read_text())
for n,h in j.items():assert hashlib.sha256((r/'upstream'/n).read_bytes()).hexdigest()==h,n
renames=['sin','cos','tan','atan','tanh','expm1','log1p','cbrt','erf','erfc','asinh','rint','nextafter','ilogb','logb','modf']
cmd=['cc','-std=c99','-O0','-fno-builtin','-fno-strict-aliasing','-frounding-math','-shared','-fPIC','-D__LITTLE_ENDIAN=1','-D__ieee754_sqrt=sqrt','-D__ieee754_fmod=fmod',*['-D'+n+'=cosmic_fd_'+n for n in renames],*[str(p) for p in sorted((r/'upstream').glob('*.c'))],str(r/'reference-test.c'),'-lm','-o',str(o/'reference.so')];subprocess.run(cmd,check=True)
l=ctypes.CDLL(str(o/'reference.so'));U=ctypes.c_uint;Q=ctypes.c_ulonglong;ref=l.cosmic_fd_reference;ref.argtypes=[U,Q,Q,ctypes.POINTER(U)];ref.restype=Q
edges=[0,0x8000000000000000,1,0x0010000000000000,0x3fe0000000000000,0x3ff0000000000000,0xbff0000000000000,0x4000000000000000,0x400921fb54442d18,0x4340000000000000,0x7fefffffffffffff,0x7ff0000000000000,0xfff0000000000000,0x7ff8000012345678]
rng=random.Random(20261010);vectors=[];names=['exp','log','log10','sin','cos','tan','asin','acos','atan','sinh','cosh','tanh','expm1','log1p','cbrt','pow','atan2','hypot','erf','erfc','lgamma','acosh','asinh','atanh','remainder']
for op,n in enumerate(names):
 pairs=[(x,0x4000000000000000) for x in edges]+[(rng.getrandbits(64),rng.getrandbits(64)) for _ in range(a.random_cases)]
 for x,y in pairs:
  f=U();v=ref(op,x,y,ctypes.byref(f));vectors.append([op,x,y,v,f.value])
lines=['/* Unchanged fdlibm host-arithmetic oracle; special NaNs compared by class. */','struct fd_vector {unsigned int op;unsigned long long a,b,result;unsigned int flags;};','static const struct fd_vector fd_vectors[]={']
lines+=['{%du,0x%xULL,0x%xULL,0x%xULL,%du},'%tuple(v) for v in vectors];lines+=['};','#define FD_VECTOR_COUNT %du'%len(vectors)];(o/'fdlibm-vectors.h').write_text('\n'.join(lines)+'\n')
(o/'summary.json').write_text(json.dumps({'status':'reference-generated','vectors':len(vectors),'operations':names,'seed':20261010,'upstream_sha256':j,'command':cmd,'reference_precision':'host binary64 nearest; flag comparison diagnostic (C integer casts can differ inexact)'},indent=2)+'\n');print('generated',len(vectors),'reference vectors')
