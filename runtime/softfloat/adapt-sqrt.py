from pathlib import Path
import re,hashlib,json
r=Path(__file__).resolve().parent;u=r/'upstream'
j=json.loads((r/'upstream/SHA256.json').read_text())
for name in ['f32_sqrt','f64_sqrt','s_approxRecipSqrt32_1','s_approxRecipSqrt_1Ks']:
 data=(u/(name+'.c')).read_bytes();assert j[name+'.c']==hashlib.sha256(data).hexdigest(),name
 t=data.decode().replace('\r\n','\n');t=re.sub(r'^#include.*\n','',t,flags=re.M)
 if name.startswith('f'):
  bits=name[1:3];t=t.replace('float'+bits+'_t','unsigned int' if bits=='32' else 'unsigned long long')
  t=re.sub(r'    union ui'+bits+r'_f'+bits+r' u[A-Z];\n','',t)
  t=t.replace('uA.f = a;\n    uiA = uA.ui;','uiA = a;').replace('uZ.ui = uiZ;\n    return uZ.f;','return uiZ;')
  t=t.replace(name+'( ', 'cosmic_sf_sqrt'+bits+'( cosmic_sf_context *ctx, ')
  for fn in ['propagateNaNF'+bits+'UI','roundPackToF'+bits]:t=t.replace('softfloat_'+fn+'( ', 'softfloat_'+fn+'( ctx, ')
  t=t.replace('normExpSig = softfloat_normSubnormalF'+bits+'Sig( sigA );','softfloat_normSubnormalF'+bits+'Sig( sigA, &normExpSig );')
  t=t.replace('softfloat_raiseFlags( ','cosmic_sf_raise( ctx, ')
 else:
  t=re.sub(r'^extern const uint16_t [^;]+;\n','',t,flags=re.M)
  t=re.sub(r'^(uint32_t|const uint16_t) ',r'static \1 ',t,flags=re.M)
 t=re.sub(r'\bsoftfloat_([A-Za-z0-9_]+)',r'cosmic_sf_private_\1',t)
 assert (r/'adapted'/('sqrt-'+name+'.inc')).read_text()==t,name
print('Verified licensed square-root adaptations against upstream hashes.')
