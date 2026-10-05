/* Binary64/width/I64 conversion closure adapted from licensed SoftFloat3e.
   All inherited fragments preserve notices. Explicit context; no host FP. */
#include "binary32.h"
typedef unsigned int uint32_t;
typedef unsigned int uint_fast32_t;
typedef unsigned int uint_fast16_t;
typedef unsigned int uint_fast8_t;
typedef unsigned char uint_least8_t;
typedef int int_fast32_t;
typedef int int_fast16_t;
typedef int int_fast8_t;
typedef _Bool bool;
#define signF32UI(a) ((bool)((uint32_t)(a)>>31))
#define expF32UI(a) ((int_fast16_t)((a)>>23)&0xFF)
#define fracF32UI(a) ((a)&0x007FFFFF)
#define packToF32UI(sign,exp,sig) (((uint32_t)(sign)<<31)+((uint32_t)(exp)<<23)+(sig))
#define isNaNF32UI(a) (((~(a)&0x7F800000)==0)&&((a)&0x007FFFFF))
#define defaultNaNF32UI 0x7FC00000
#define cosmic_sf_private_isSigNaNF32UI(a) ((((a)&0x7FC00000)==0x7F800000)&&((a)&0x003FFFFF))
#define cosmic_sf_private_roundingMode (ctx->rounding)
#define cosmic_sf_private_detectTininess 1
#define cosmic_sf_private_round_near_even 0
#define cosmic_sf_private_round_minMag 1
#define cosmic_sf_private_round_min 2
#define cosmic_sf_private_round_max 3
#define cosmic_sf_private_round_near_maxMag 4
#define cosmic_sf_private_tininess_beforeRounding 0
#define cosmic_sf_private_flag_inexact COSMIC_SF_INEXACT
#define cosmic_sf_private_flag_underflow COSMIC_SF_UNDERFLOW
#define cosmic_sf_private_flag_overflow COSMIC_SF_OVERFLOW
#define cosmic_sf_private_flag_invalid COSMIC_SF_INVALID
static unsigned int cosmic_sf_private_roundPackToF32(cosmic_sf_context *,bool,int,unsigned int);

typedef unsigned short uint16_t;
typedef unsigned long long uint64_t;
typedef unsigned long long uint_fast64_t;
typedef long long int64_t;
typedef long long int_fast64_t;
typedef int int32_t;
#define UINT64_C(a) a##ULL
#define INT64_C(a) a##LL
#define true 1
#define false 0
#define INLINE_LEVEL 2
#define indexWord(total,n) (n)
#define signF64UI(a) ((bool)((uint64_t)(a)>>63))
#define expF64UI(a) ((int_fast16_t)((a)>>52)&0x7FF)
#define fracF64UI(a) ((a)&UINT64_C(0x000FFFFFFFFFFFFF))
#define packToF64UI(sign,exp,sig) (((uint64_t)(sign)<<63)+((uint64_t)(exp)<<52)+(sig))
#define isNaNF64UI(a) (((~(a)&UINT64_C(0x7FF0000000000000))==0)&&((a)&UINT64_C(0x000FFFFFFFFFFFFF)))
#define cosmic_sf_private_isSigNaNF64UI(a) ((((a)&UINT64_C(0x7FF8000000000000))==UINT64_C(0x7FF0000000000000))&&((a)&UINT64_C(0x0007FFFFFFFFFFFF)))
#define defaultNaNF64UI UINT64_C(0x7FF8000000000000)
#define cosmic_sf_private_flag_infinite COSMIC_SF_DIVIDE_BY_ZERO
#define i32_fromPosOverflow 0x7FFFFFFF
#define i32_fromNegOverflow (-0x7FFFFFFF-1)
#define i32_fromNaN 0
#define ui32_fromPosOverflow 0xFFFFFFFFu
#define ui32_fromNegOverflow 0
#define ui32_fromNaN 0
#define i64_fromPosOverflow INT64_C(0x7FFFFFFFFFFFFFFF)
#define i64_fromNegOverflow (-INT64_C(0x7FFFFFFFFFFFFFFF)-1)
#define i64_fromNaN 0
#define ui64_fromPosOverflow UINT64_C(0xFFFFFFFFFFFFFFFF)
#define ui64_fromNegOverflow 0
#define ui64_fromNaN 0
struct commonNaN { bool sign; uint64_t v0,v64; };
struct exp16_sig32 { int exp; unsigned int sig; };
struct exp16_sig64 { int exp; unsigned long long sig; };
static unsigned long long cosmic_sf_private_roundPackToF64(cosmic_sf_context *,bool,int,unsigned long long);
static unsigned long long cosmic_sf_private_normRoundPackToF64(cosmic_sf_context *,bool,int,unsigned long long);
#include "adapted/s_countLeadingZeros8.inc"
#include "adapted/s_shiftRightJam32.inc"
#include "adapted/s_countLeadingZeros32.inc"
#include "adapted/s_shortShiftRightJam64.inc"
#include "adapted/s_normSubnormalF32Sig.inc"
#include "adapted/s_roundPackToF32.inc"
#include "adapted/binary64-s_countLeadingZeros64.inc"
#include "adapted/binary64-s_shiftRightJam64.inc"
#include "adapted/binary64-s_approxRecip_1Ks.inc"
#include "adapted/binary64-s_approxRecip32_1.inc"
#include "adapted/binary64-s_mul64To128M.inc"
#include "adapted/binary64-s_normSubnormalF64Sig.inc"
#include "adapted/binary64-s_propagateNaNF64UI.inc"
#include "adapted/binary64-s_f32UIToCommonNaN.inc"
#include "adapted/binary64-s_f64UIToCommonNaN.inc"
#include "adapted/binary64-s_commonNaNToF32UI.inc"
#include "adapted/binary64-s_commonNaNToF64UI.inc"
#include "adapted/binary64-s_roundPackToF64.inc"
#include "adapted/binary64-s_normRoundPackToF64.inc"
#include "adapted/binary64-s_addMagsF64.inc"
#include "adapted/binary64-s_subMagsF64.inc"
#include "adapted/binary64-f64_add.inc"
#include "adapted/binary64-f64_sub.inc"
#include "adapted/binary64-f64_mul.inc"
#include "adapted/binary64-f64_div.inc"
#include "adapted/binary64-f64_eq.inc"
#include "adapted/binary64-f64_lt.inc"
#include "adapted/binary64-f64_le.inc"
#include "adapted/binary64-f32_to_f64.inc"
#include "adapted/binary64-f64_to_f32.inc"
#include "adapted/binary64-f32_to_i64_r_minMag.inc"
#include "adapted/binary64-f32_to_ui64_r_minMag.inc"
#include "adapted/binary64-f64_to_i64_r_minMag.inc"
#include "adapted/binary64-f64_to_ui64_r_minMag.inc"
#include "adapted/binary64-f64_to_i32_r_minMag.inc"
#include "adapted/binary64-f64_to_ui32_r_minMag.inc"
#include "adapted/binary64-i64_to_f32.inc"
#include "adapted/binary64-ui64_to_f32.inc"
#include "adapted/binary64-i64_to_f64.inc"
#include "adapted/binary64-ui64_to_f64.inc"
#include "adapted/binary64-i32_to_f64.inc"
#include "adapted/binary64-ui32_to_f64.inc"

#include "adapted/sqrt-s_approxRecipSqrt_1Ks.inc"
#include "adapted/sqrt-s_approxRecipSqrt32_1.inc"
#include "adapted/sqrt-f64_sqrt.inc"
unsigned long long cosmic_sf_pack64(cosmic_sf_context *ctx,int sign,int exponent,unsigned long long sig) {
    return cosmic_sf_private_roundPackToF64(ctx,sign!=0,exponent,sig);
}
unsigned long long cosmic_sf_scalbn64(cosmic_sf_context *ctx,unsigned long long a,int n) {
    unsigned int sign=(unsigned int)(a>>63);
    int exp=(int)((a>>52)&2047ULL);
    unsigned long long sig=a&0xfffffffffffffULL;
    struct exp16_sig64 z;
    if(exp==2047) return sig ? cosmic_sf_private_propagateNaNF64UI(ctx,a,0) : a;
    if(!exp) { if(!sig) return a;cosmic_sf_private_normSubnormalF64Sig(sig,&z);exp=z.exp;sig=z.sig; }
    else sig|=0x10000000000000ULL;
    if(n>10000) n=10000; if(n< -10000) n= -10000;
    return cosmic_sf_private_roundPackToF64(ctx,sign!=0,exp+n-1,sig<<10);
}
#define cosmic_sf_private_mulAdd_subC 1
#define cosmic_sf_private_mulAdd_subProd 2
#define wordIncr 1
#define indexWordHi(total) ((total)-1)
#define indexWordLo(total) 0
#define indexMultiwordHi(total,n) ((total)-(n))
#define indexMultiwordLo(total,n) 0
#define indexMultiwordHiBut(total,n) (n)
#define indexMultiwordLoBut(total,n) 0
#include "adapted/fma-s_addM.inc"
#include "adapted/fma-s_subM.inc"
#include "adapted/fma-s_negXM.inc"
#include "adapted/fma-s_shortShiftLeftM.inc"
#include "adapted/fma-s_shortShiftRightM.inc"
#include "adapted/fma-s_shortShiftRightJamM.inc"
#include "adapted/fma-s_shiftLeftM.inc"
#include "adapted/fma-s_shiftRightJamM.inc"
#define cosmic_sf_private_add128M(a,b,z) cosmic_sf_private_addM(4,a,b,z)
#define cosmic_sf_private_sub128M(a,b,z) cosmic_sf_private_subM(4,a,b,z)
#define cosmic_sf_private_negX128M(z) cosmic_sf_private_negXM(4,z)
#define cosmic_sf_private_shortShiftRight128M(a,d,z) cosmic_sf_private_shortShiftRightM(4,a,d,z)
#define cosmic_sf_private_shiftRightJam128M(a,d,z) cosmic_sf_private_shiftRightJamM(4,a,d,z)
#define cosmic_sf_private_shiftLeft128M(a,d,z) cosmic_sf_private_shiftLeftM(4,a,d,z)
#include "adapted/fma-s_mulAddF64.inc"
#include "adapted/fma-f64_roundToInt.inc"
unsigned long long cosmic_sf_fma64(cosmic_sf_context *ctx,unsigned long long a,unsigned long long b,unsigned long long c) { return cosmic_sf_private_mulAddF64(ctx,a,b,c,0); }
unsigned long long cosmic_sf_rint64(cosmic_sf_context *ctx,unsigned long long a,int exact) { return cosmic_sf_private_roundToInt64(ctx,a,ctx->rounding,exact!=0); }
