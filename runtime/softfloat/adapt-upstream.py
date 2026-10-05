#!/usr/bin/env python3
"""Reproduce bounded 3e arithmetic fragment adaptations from vendored originals."""
import hashlib,json,re
from pathlib import Path
root=Path(__file__).resolve().parent
manifest=json.loads((root/'upstream/SHA256.json').read_text())
for name,h in manifest.items():
 assert hashlib.sha256((root/'upstream'/name).read_bytes()).hexdigest()==h,name
stateful=['addMagsF32','subMagsF32','roundPackToF32','normRoundPackToF32','propagateNaNF32UI']
files=['s_countLeadingZeros8.c','s_shiftRightJam32.c','s_countLeadingZeros32.c','ARM-VFPv2/s_propagateNaNF32UI.c','s_roundPackToF32.c','s_normRoundPackToF32.c','s_addMagsF32.c','s_subMagsF32.c']
for name in files:
 t=(root/'upstream'/name).read_text();t=re.sub(r'^#include.*\n','',t,flags=re.M)
 t=t.replace('float32_t','unsigned int').replace('    union ui32_f32 uZ;\n','')
 t=t.replace('    uZ.ui = uiZ;\n    return uZ.f;','    return uiZ;')
 t=t.replace('        uZ.ui = packToF32UI( sign, sig ? exp : 0, sig<<(shiftDist - 7) );\n        return uZ.f;','        return packToF32UI( sign, sig ? exp : 0, sig<<(shiftDist - 7) );')
 for fn in stateful:
  # One replacement distinguishes a function definition's typed first parameter
  # from a call expression, avoiding changes to arithmetic itself.
  pattern=r'\bsoftfloat_'+fn+r'\(\s*'
  def insert(m):
   typed=re.match(r'(?:bool|uint_fast32_t)\s',t[m.end():])
   return 'softfloat_'+fn+'( '+('cosmic_sf_context *ctx, ' if typed else 'ctx, ')
  t=re.sub(pattern,insert,t)
 t=t.replace('softfloat_raiseFlags(','cosmic_sf_raise( ctx,')
 t=t.replace('softfloat_exceptionFlags','ctx->flags')
 t=re.sub(r'\bsoftfloat_([A-Za-z0-9_]+)',r'cosmic_sf_private_\1',t)
 t=re.sub(r'^(unsigned int|uint32_t|uint_fast8_t|uint_fast32_t|const uint_least8_t)\b',r'static \1',t,flags=re.M)
 # Ignore harmless spacing when checking a reproduced inherited source fragment.
 path=root/'adapted'/(Path(name).stem+'.inc')
 assert re.sub(r'\s+',' ',t)==re.sub(r'\s+',' ',path.read_text()),str(path)
print('Verified all adapted fragments against preserved source transformations.')
# Extension: preserve multiplication/division arithmetic, replace aggregate
# wrappers and the internal normalization struct-return ABI with an output ptr.
for name in ['s_shortShiftRightJam64.c','s_normSubnormalF32Sig.c','f32_mul.c','f32_div.c']:
 t=(root/'upstream'/name).read_text();t=re.sub(r'^#include.*\n','',t,flags=re.M)
 t=t.replace('float32_t','unsigned int')
 for union in ['uA','uB','uZ']:t=t.replace('    union ui32_f32 '+union+';\n','')
 t=t.replace('    uA.f = a;\n    uiA = uA.ui;','    uiA = a;').replace('    uB.f = b;\n    uiB = uB.ui;','    uiB = b;')
 t=t.replace('    uZ.ui = uiZ;\n    return uZ.f;','    return uiZ;')
 for op in ['mul','div']:t=t.replace('f32_'+op+'( unsigned int a, unsigned int b )','cosmic_sf_'+op+'32( cosmic_sf_context *ctx, unsigned int a, unsigned int b )')
 for fn in ['roundPackToF32','propagateNaNF32UI']:t=re.sub(r'\bsoftfloat_'+fn+r'\(\s*','softfloat_'+fn+'( ctx, ',t)
 t=t.replace('softfloat_raiseFlags(','cosmic_sf_raise( ctx,')
 t=re.sub(r'\bsoftfloat_([A-Za-z0-9_]+)',r'cosmic_sf_private_\1',t)
 if name=='s_normSubnormalF32Sig.c':
  t=t.replace('struct exp16_sig32 cosmic_sf_private_normSubnormalF32Sig( uint_fast32_t sig )','static void cosmic_sf_private_normSubnormalF32Sig( uint_fast32_t sig, struct exp16_sig32 *z )')
  t=t.replace('    struct exp16_sig32 z;\n','').replace('z.exp','z->exp').replace('z.sig','z->sig').replace('    return z;','    return;')
 else:
  for arg in ['sigA','sigB']:t=t.replace('normExpSig = cosmic_sf_private_normSubnormalF32Sig( '+arg+' );','cosmic_sf_private_normSubnormalF32Sig( '+arg+', &normExpSig );')
 if name=='s_shortShiftRightJam64.c':t=re.sub(r'^uint64_t','static uint64_t',t,flags=re.M)
 path=root/'adapted'/(Path(name).stem+'.inc')
 assert re.sub(r'\s+',' ',t)==re.sub(r'\s+',' ',path.read_text()),str(path)
print('Verified multiplication/division adaptation fragments and normalization ABI change.')
