from pathlib import Path
import re,hashlib,json
r=Path(__file__).resolve().parent;u=r/'upstream'
public=['f64_add','f64_sub','f64_mul','f64_div','f64_eq','f64_lt','f64_le','f32_to_f64','f64_to_f32','f32_to_i64_r_minMag','f32_to_ui64_r_minMag','f64_to_i64_r_minMag','f64_to_ui64_r_minMag','f64_to_i32_r_minMag','f64_to_ui32_r_minMag','i64_to_f32','ui64_to_f32','i64_to_f64','ui64_to_f64','i32_to_f64','ui32_to_f64']
helpers=['s_addMagsF64','s_subMagsF64','s_roundPackToF64','s_normRoundPackToF64','s_normSubnormalF64Sig','s_shiftRightJam64','s_countLeadingZeros64','s_mul64To128M','s_approxRecip32_1','s_approxRecip_1Ks']
special=['s_propagateNaNF64UI','s_f32UIToCommonNaN','s_f64UIToCommonNaN','s_commonNaNToF32UI','s_commonNaNToF64UI']
files=[n+'.c' for n in public+helpers]+['ARM-VFPv2/'+n+'.c' for n in special]
j=json.loads((r/'upstream/SHA256.json').read_text())
j=json.loads((r/'upstream/SHA256.json').read_text())
for n in files:
 assert hashlib.sha256((r/'upstream'/n).read_bytes()).hexdigest()==j[n],n
names={n:'cosmic_sf_'+n.replace('ui','u').replace('_r_minMag','') for n in public}
for op in ['add','sub','mul','div','eq','lt','le']:names['f64_'+op]='cosmic_sf_'+op+'64'
state=['addMagsF64','subMagsF64','roundPackToF64','normRoundPackToF64','propagateNaNF64UI','f32UIToCommonNaN','f64UIToCommonNaN','roundPackToF32','normRoundPackToF32']
for n in files:
 t=(u/n).read_text();t=re.sub(r'^#include.*\n','',t,flags=re.M)
 t=t.replace('float32_t','unsigned int').replace('float64_t','unsigned long long')
 t=re.sub(r'    union ui(?:32_f32|64_f64) (u[A-Z]?);\n','',t)
 for v,param in [('uA','a'),('uB','b')]:t=re.sub(r'    '+v+r'\.f = '+param+r';\s*ui'+v[1:]+r' = '+v+r'\.ui;', '    ui'+v[1:]+' = '+param+';',t)
 t=re.sub(r'\b(uZ|u)\.ui\s*=\s*(.*?);\s*return \1\.f;',r'return \2;',t,flags=re.S)
 for fn in state:
  def inject(m):
   typed=re.match(r'(?:bool|uint_fast32_t|uint_fast64_t)\s',t[m.end():])
   return 'softfloat_'+fn+'( '+('cosmic_sf_context *ctx, ' if typed else 'ctx, ')
  t=re.sub(r'\bsoftfloat_'+fn+r'\(\s*',inject,t)
 t=t.replace('softfloat_raiseFlags(','cosmic_sf_raise( ctx,').replace('softfloat_exceptionFlags','ctx->flags')
 t=re.sub(r'\bsoftfloat_([A-Za-z0-9_]+)',r'cosmic_sf_private_\1',t)
 stem=Path(n).stem
 if stem in public:
  original=stem
  t=re.sub(r'\b'+original+r'\(\s*', names[original]+'( cosmic_sf_context *ctx, ',t)
  if '_r_minMag' in original:t=t.replace(', bool exact','');t=re.sub(r'\bexact\b','true',t)
  if original in ['f64_eq','f64_lt','f64_le']:t=re.sub(r'^bool ', 'int ',t,flags=re.M)
  # Some exact conversions/comparisons do not touch context except at NaN.
 else:
  t=re.sub(r'^(unsigned long long|unsigned int|uint64_t|uint32_t|uint_fast8_t|uint_fast32_t|uint_fast64_t|void|const uint16_t)\b',r'static \1',t,flags=re.M)
  t=re.sub(r'^extern const uint16_t [^;]+;\n','',t,flags=re.M)
 if stem=='s_normSubnormalF64Sig':
  t=t.replace('struct exp16_sig64 cosmic_sf_private_normSubnormalF64Sig( uint_fast64_t sig )','static void cosmic_sf_private_normSubnormalF64Sig( uint_fast64_t sig, struct exp16_sig64 *z )')
  t=t.replace('    struct exp16_sig64 z;\n','').replace('z.exp','z->exp').replace('z.sig','z->sig').replace('    return z;','    return;')
 for bits in [32,64]:
  for arg in ['sigA','sigB','frac']:t=t.replace('normExpSig = cosmic_sf_private_normSubnormalF'+str(bits)+'Sig( '+arg+' );','cosmic_sf_private_normSubnormalF'+str(bits)+'Sig( '+arg+', &normExpSig );')
 expected=(r/'adapted'/('binary64-'+stem+'.inc')).read_text()
 assert re.sub(r'\s+',' ',t)==re.sub(r'\s+',' ',expected),n
print('Verified binary64 arithmetic/conversion adaptations against preserved upstream hashes.')
