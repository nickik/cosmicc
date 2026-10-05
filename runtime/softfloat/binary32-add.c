
/*============================================================================

This C source file is part of the SoftFloat IEEE Floating-Point Arithmetic
Package, Release 3e, by John R. Hauser.

Copyright 2011, 2012, 2013, 2014, 2015, 2016 The Regents of the University of
California.  All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

 1. Redistributions of source code must retain the above copyright notice,
    this list of conditions, and the following disclaimer.

 2. Redistributions in binary form must reproduce the above copyright notice,
    this list of conditions, and the following disclaimer in the documentation
    and/or other materials provided with the distribution.

 3. Neither the name of the University nor the names of its contributors may
    be used to endorse or promote products derived from this software without
    specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS "AS IS", AND ANY
EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE, ARE
DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE FOR ANY
DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
(INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND
ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

=============================================================================*/

/* Bounded SoftFloat 3e adaptation. Inherited arithmetic and license notices are
   retained in the adapted include fragments. See ADAPTATION.md for exact changes/provenance. */
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
static unsigned int cosmic_sf_private_normRoundPackToF32(cosmic_sf_context *,bool,int,unsigned int);
#include "adapted/s_countLeadingZeros8.inc"
#include "adapted/s_shiftRightJam32.inc"
#include "adapted/s_countLeadingZeros32.inc"
#include "adapted/s_propagateNaNF32UI.inc"
#include "adapted/s_roundPackToF32.inc"
#include "adapted/s_normRoundPackToF32.inc"
#include "adapted/s_addMagsF32.inc"
#include "adapted/s_subMagsF32.inc"
unsigned int cosmic_sf_add32(cosmic_sf_context *ctx,unsigned int a,unsigned int b) {
    if(signF32UI(a^b)) return cosmic_sf_private_subMagsF32(ctx,a,b);
    return cosmic_sf_private_addMagsF32(ctx,a,b);
}

/* Additional bounded upstream arithmetic closure, with integer 64-bit operations. */
typedef unsigned long long uint64_t;
typedef unsigned long long uint_fast64_t;
#define SOFTFLOAT_FAST_DIV64TO32 1
#define cosmic_sf_private_flag_infinite COSMIC_SF_DIVIDE_BY_ZERO
struct exp16_sig32 { int exp; unsigned int sig; };
#include "adapted/s_shortShiftRightJam64.inc"
#include "adapted/s_normSubnormalF32Sig.inc"
unsigned int cosmic_sf_sub32(cosmic_sf_context *ctx,unsigned int a,unsigned int b) {
    /* Upstream f32_sub dispatch retains original NaN operand bits. */
    if(signF32UI(a^b)) return cosmic_sf_private_addMagsF32(ctx,a,b);
    return cosmic_sf_private_subMagsF32(ctx,a,b);
}
#include "adapted/f32_mul.inc"
#include "adapted/f32_div.inc"

typedef unsigned short uint16_t;
#include "adapted/sqrt-s_approxRecipSqrt_1Ks.inc"
#include "adapted/sqrt-s_approxRecipSqrt32_1.inc"
#include "adapted/sqrt-f32_sqrt.inc"
unsigned int cosmic_sf_pack32(cosmic_sf_context *ctx,int sign,int exponent,unsigned int sig) {
    return cosmic_sf_private_roundPackToF32(ctx,sign!=0,exponent,sig);
}
unsigned int cosmic_sf_scalbn32(cosmic_sf_context *ctx,unsigned int a,int n) {
    unsigned int sign=a>>31,sig=a&0x7fffffu;
    int exp=(int)((a>>23)&255u);
    struct exp16_sig32 z;
    if(exp==255u) return sig ? cosmic_sf_private_propagateNaNF32UI(ctx,a,0) : a;
    if(!exp) { if(!sig) return a; cosmic_sf_private_normSubnormalF32Sig(sig,&z);exp=z.exp;sig=z.sig; }
    else sig|=0x800000u;
    if(n>1000) n=1000; if(n< -1000) n= -1000;
    return cosmic_sf_private_roundPackToF32(ctx,sign!=0,(int)exp+n-1,sig<<7);
}
#define UINT64_C(a) a##ULL
#define cosmic_sf_private_mulAdd_subC 1
#define cosmic_sf_private_mulAdd_subProd 2
#include "adapted/binary64-s_shiftRightJam64.inc"
#include "adapted/binary64-s_countLeadingZeros64.inc"
#include "adapted/fma-s_mulAddF32.inc"
#include "adapted/fma-f32_roundToInt.inc"
unsigned int cosmic_sf_fma32(cosmic_sf_context *ctx,unsigned int a,unsigned int b,unsigned int c) { return cosmic_sf_private_mulAddF32(ctx,a,b,c,0); }
unsigned int cosmic_sf_rint32(cosmic_sf_context *ctx,unsigned int a,int exact) { return cosmic_sf_private_roundToInt32(ctx,a,ctx->rounding,exact!=0); }
