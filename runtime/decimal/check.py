#!/usr/bin/env python3
"""C-locale text conversion differential; guest proof is recorded separately."""
import argparse, ctypes, hashlib, json, random, struct, subprocess
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,required=True);p.add_argument('--random-cases',type=int,default=1000);a=p.parse_args()
root=Path(__file__).resolve().parents[2];out=a.output.resolve();out.mkdir(parents=True,exist_ok=False)
files=[root/n for n in ['runtime/decimal/parse.c','runtime/decimal/format.c','runtime/softfloat/context.c','runtime/softfloat/binary32-add.c','runtime/softfloat/binary64.c']]
cmd=['cc','-std=c99','-O2','-fno-builtin','-fPIC','-shared','-Wl,-Bsymbolic',*[str(f) for f in files],'-o',str(out/'decimal.so')]
r=subprocess.run(cmd,capture_output=True);(out/'build.log').write_bytes(r.stdout+r.stderr);r.check_returncode()
lib=ctypes.CDLL(str(out/'decimal.so'));ref=ctypes.CDLL('libc.so.6');m=ctypes.CDLL('libm.so.6')
class Context(ctypes.Structure):_fields_=[('flags',ctypes.c_uint),('rounding',ctypes.c_uint)]
ctx=Context(0,0);lib.cosmic_sf_bind_context.argtypes=[ctypes.POINTER(Context)];lib.cosmic_sf_bind_context(ctypes.byref(ctx))
parse=lib.cosmic_decimal_parse_bits;parse.argtypes=[ctypes.c_char_p,ctypes.POINTER(ctypes.c_void_p),ctypes.c_int,ctypes.POINTER(Context)];parse.restype=ctypes.c_ulonglong
for name,typ in [('strtod',ctypes.c_double),('strtof',ctypes.c_float)]:
 f=getattr(ref,name);f.argtypes=[ctypes.c_char_p,ctypes.POINTER(ctypes.c_void_p)];f.restype=typ
for x in [lib,ref]:x.snprintf.argtypes=[ctypes.c_void_p,ctypes.c_size_t,ctypes.c_char_p];x.snprintf.restype=ctypes.c_int
ref.setlocale.argtypes=[ctypes.c_int,ctypes.c_char_p];ref.setlocale(1,b'C')
rng=random.Random(20261006)
texts=['0','-0','1','.1','1e','+','0x1p-1074','1e-320','1e300','-0x1.fffffffffffffp1023','1.00000005960464477539062500000000001','0x','0x.',' .1e-45','1e400','1e-400','nan','-inf','Infinity!','-nan(payload)','nan(no)','nan(','0x1.','  garbage','1.00000000000000011102230246251565404236316680908203125'+'0'*1300+'1']
for i in range(a.random_cases):
 texts.append(('' if rng.randrange(2) else '-')+str(rng.randrange(1,10**rng.randrange(1,90)))+'e'+str(rng.randrange(-400,350)))
 texts.append(('' if rng.randrange(2) else '-')+'0x'+format(rng.getrandbits(64),'x')+'p'+str(rng.randrange(-1200,1100)))
values=[0.,-0.,1.,-1.,1.5,2.5,9.999999,1e-300,1e300,struct.unpack('<d',(1).to_bytes(8,'little'))[0],float('inf'),float('-inf'),float('nan')]
values += [struct.unpack('<d',rng.getrandbits(64).to_bytes(8,'little'))[0] for i in range(a.random_cases)]
formats=['%f','%e','%g','%a','%.0f','%.0e','%.0g','%.0a','%.1a','%.12a','%#.0f','%#.6g','%+020.3f','%020a','%-20.4e','%.17g','%.50f','%.900f','%F','%E','%G','%A','% 14.8g']
parse_count=format_count=boundary_count=0
for mode,hostmode in [(0,0),(1,0xc00),(2,0x400),(3,0x800)]:
 ctx.rounding=mode;m.fesetround(hostmode)
 for text in texts:
  buf=ctypes.create_string_buffer(text.encode());end=ctypes.c_void_p();end2=ctypes.c_void_p()
  for narrow in [0,1]:
   fresh=Context(0,mode);got=parse(buf,ctypes.byref(end),narrow,ctypes.byref(fresh));v=(ref.strtof if narrow else ref.strtod)(buf,ctypes.byref(end2));expected=int.from_bytes(struct.pack('<f' if narrow else '<d',v),'little')
   mask=0x7fffffff if narrow else 0x7fffffffffffffff;inf=0x7f800000 if narrow else 0x7ff0000000000000
   assert got==expected or ((got&mask)>inf and (expected&mask)>inf),(text,narrow,mode,hex(got),hex(expected))
   assert end.value==end2.value,(text,end.value,end2.value);parse_count+=1
 for v in values:
  for fmt in formats:
   x=ctypes.create_string_buffer(2000);y=ctypes.create_string_buffer(2000);n=lib.snprintf(x,2000,fmt.encode(),ctypes.c_double(v));n2=ref.snprintf(y,2000,fmt.encode(),ctypes.c_double(v))
   assert(n,x.value)==(n2,y.value),(mode,fmt,v,n,n2,x.value,y.value);format_count+=1
 # Truncation/termination and exact required-length results, including size zero.
 for size in [0,1,2,4,8,16,128]:
  x=ctypes.create_string_buffer(b'X'*130);y=ctypes.create_string_buffer(b'X'*130);n=lib.snprintf(x,size,b'%+09.2f:%#x:%lld',ctypes.c_double(-12.5),ctypes.c_uint(42),ctypes.c_longlong(-4294967297));n2=ref.snprintf(y,size,b'%+09.2f:%#x:%lld',ctypes.c_double(-12.5),ctypes.c_uint(42),ctypes.c_longlong(-4294967297));assert(n,x.raw)==(n2,y.raw);boundary_count+=1
m.fesetround(0)
# Avoid signed overflow or unbounded work on huge valid precision/width.
assert lib.snprintf(None,0,b'%.2147483647f',ctypes.c_double(1))==-1
assert lib.snprintf(None,0,b'%.2147483647g',ctypes.c_double(1))==1
assert lib.snprintf(None,0,b'%2147483647s',ctypes.c_char_p(b'x'))==2147483647
report={'status':'passed','seed':20261006,'parse_bit_and_end_checks':parse_count,'format_byte_and_length_checks':format_count,'truncation_boundary_checks':boundary_count,'rounding_modes':4,'reference':'glibc C locale; NaN payload implementation-defined (classification compared)','command':cmd,'source_hashes':{str(f.relative_to(root)):hashlib.sha256(f.read_bytes()).hexdigest() for f in [*files,root/'runtime/decimal/bigint.h']},'host_reference':'Host text conversion is used only as validation, never as guest implementation.'}
(out/'summary.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
