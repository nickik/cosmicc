#!/usr/bin/env python3
"""Preserve notices, namespace public functions, fix the SIA little-endian word ABI."""
from pathlib import Path
import hashlib,json,re
r=Path(__file__).resolve().parent;u=r/'upstream';a=r/'adapted';a.mkdir(exist_ok=True)
manifest={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(u.iterdir()) if p.is_file()}
(r/'SHA256.json').write_text(json.dumps(manifest,indent=2)+'\n')
public=['sin','cos','tan','atan','expm1','log1p','tanh','cbrt','erf','erfc','rint','nextafter','ilogb','logb','modf','asinh']
for p in u.iterdir():
 if p.suffix!='.c':continue
 s=p.read_text()
 # Exponent/word shifts use defined unsigned modulo arithmetic (upstream assumes it).
 s=re.sub(r"\b(k|n|j|e0)<<(\d+|\([^)]*\))",r"((unsigned int)\1)<<\2",s)
 s=s.replace("((k+1000)<<20)","((unsigned int)(k+1000)<<20)")
 for n in public:s=re.sub(r'\b'+n+r'\b','cosmic_fd_'+n,s)
 if p.name in ['e_atan2.c','s_cos.c','s_sin.c']:
  i=s.rfind('}');s=s[:i]+'    return 0.0; /* Exhaustive switch above; explicit terminal return for SIA. */\n'+s[i:]
 (a/p.name).write_text(s)
s=(u/'fdlibm.h').read_text();s='#define __LITTLE_ENDIAN 1\n'+s
for n in public:s=re.sub(r'\b'+n+r'\b','cosmic_fd_'+n,s)
s+='\n#define __ieee754_sqrt sqrt\n#define __ieee754_fmod fmod\n'
(a/'fdlibm.h').write_text(s)
print('Adapted',len(list(a.glob('*.c'))),'fdlibm translation units.')
