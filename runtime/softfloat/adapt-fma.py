from pathlib import Path
import json,hashlib,re
r=Path(__file__).resolve().parent;u=r/'upstream';j=json.loads((r/'upstream/SHA256.json').read_text())
names=['s_mulAddF32','s_mulAddF64','s_addM','s_subM','s_negXM','s_shortShiftLeftM','s_shortShiftRightM','s_shortShiftRightJamM','s_shiftLeftM','s_shiftRightJamM','f32_roundToInt','f64_roundToInt']
state=['roundPackToF32','roundPackToF64','propagateNaNF32UI','propagateNaNF64UI']
for n in names:
 data=(u/(n+'.c')).read_bytes();assert j[n+'.c']==hashlib.sha256(data).hexdigest(),n;t=data.decode().replace('\r\n','\n');t=re.sub(r'^#include.*\n','',t,flags=re.M)
 for fn in state:
  t=t.replace('softfloat_'+fn+'( ','softfloat_'+fn+'( ctx, ')
 if n.startswith('s_mulAdd'):
  t=t.replace('softfloat_mulAddF'+n[-2:]+'(\n','softfloat_mulAddF'+n[-2:]+'( cosmic_sf_context *ctx,\n')
 else:t=re.sub(r'^(void|uint32_t) ',r'static \1 ',t,flags=re.M)
 for bits in [32,64]:
  for arg in ['sigA','sigB','sigC']:t=t.replace('normExpSig = softfloat_normSubnormalF'+str(bits)+'Sig( '+arg+' );','softfloat_normSubnormalF'+str(bits)+'Sig( '+arg+', &normExpSig );')
 for bits in [32,64]:
  t=t.replace('float'+str(bits)+'_t','unsigned int' if bits==32 else 'unsigned long long')
  t=re.sub(r'    union ui'+str(bits)+r'_f'+str(bits)+r' uZ;\n','',t)
  t=t.replace('uZ.ui = uiZ;\n    return uZ.f;','return uiZ;')
 t=t.replace('softfloat_raiseFlags(','cosmic_sf_raise( ctx,').replace('softfloat_exceptionFlags','ctx->flags')
 t=re.sub(r'\bsoftfloat_([A-Za-z0-9_]+)',r'cosmic_sf_private_\1',t)
 if n.endswith('roundToInt'):
  bits=n[1:3];typ='unsigned int' if bits=='32' else 'unsigned long long'
  t=t.replace('float'+bits+'_t',typ).replace(n+'( ', 'cosmic_sf_private_roundToInt'+bits+'( cosmic_sf_context *ctx, ')
  t=re.sub(r'    union ui'+bits+r'_f'+bits+r' uA;\n','',t).replace('uA.f = a;\n    uiA = uA.ui;','uiA = a;').replace('uA.ui = uiZ;\n    return uA.f;','return uiZ;')
 if n.startswith('s_mulAdd'):t=t.replace('\nunsigned int\n','\nstatic unsigned int\n').replace('\nunsigned long long\n','\nstatic unsigned long long\n')
 if n.endswith('roundToInt'):t=re.sub(r'^(unsigned int|unsigned long long) ',r'static \1 ',t,flags=re.M)
 assert (r/'adapted'/('fma-'+n+'.inc')).read_text()==t,n
print('Verified fused multiply-add and integral rounding adaptations.')
