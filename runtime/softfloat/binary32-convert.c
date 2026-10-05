/* Integer-only binary32 comparisons and conversions. No host FP operations.
 * Conversion result policy matches the ARM-VFPv2 SoftFloat specialization.
 */
#include "binary32.h"
static int cosmic_sf_nan32(unsigned int a) {
    return (a & 0x7fffffffu) > 0x7f800000u;
}
static int cosmic_sf_snan32(unsigned int a) {
    return cosmic_sf_nan32(a) && !(a & 0x00400000u);
}
int cosmic_sf_eq32(cosmic_sf_context *ctx,unsigned int a,unsigned int b) {
    if(cosmic_sf_nan32(a)||cosmic_sf_nan32(b)) {
        if(cosmic_sf_snan32(a)||cosmic_sf_snan32(b)) cosmic_sf_raise(ctx,COSMIC_SF_INVALID);
        return 0;
    }
    return a==b || ((a|b)&0x7fffffffu)==0;
}
int cosmic_sf_lt32(cosmic_sf_context *ctx,unsigned int a,unsigned int b) {
    unsigned int sa=a>>31,sb=b>>31;
    if(cosmic_sf_nan32(a)||cosmic_sf_nan32(b)) {
        cosmic_sf_raise(ctx,COSMIC_SF_INVALID); return 0;
    }
    if(sa!=sb) return sa && ((a|b)&0x7fffffffu)!=0;
    return a!=b && (sa ? a>b : a<b);
}
int cosmic_sf_le32(cosmic_sf_context *ctx,unsigned int a,unsigned int b) {
    unsigned int sa=a>>31,sb=b>>31;
    if(cosmic_sf_nan32(a)||cosmic_sf_nan32(b)) {
        cosmic_sf_raise(ctx,COSMIC_SF_INVALID); return 0;
    }
    if(sa!=sb) return sa || ((a|b)&0x7fffffffu)==0;
    return a==b || (sa ? a>b : a<b);
}
static unsigned int cosmic_sf_trunc_to_integer32(cosmic_sf_context *ctx,unsigned int a,int is_signed) {
    unsigned int sign=a>>31,exp=(a>>23)&255u,frac=a&0x007fffffu,sig,mag,discard;
    int e=(int)exp-127;
    if(e<0) {
        if(exp||frac) cosmic_sf_raise(ctx,COSMIC_SF_INEXACT);
        return 0;
    }
    if(exp==255u || e>31 || (is_signed && e==31 && a!=0xcf000000u) || (!is_signed && sign)) {
        cosmic_sf_raise(ctx,COSMIC_SF_INVALID);
        if(exp==255u && frac) return 0;
        if(is_signed) return sign ? 0x80000000u : 0x7fffffffu;
        return sign ? 0u : 0xffffffffu;
    }
    sig=frac|0x00800000u;
    if(e>=23) mag=sig<<(e-23);
    else {
        discard=(1u<<(23-e))-1u;
        if(sig&discard) cosmic_sf_raise(ctx,COSMIC_SF_INEXACT);
        mag=sig>>(23-e);
    }
    return sign ? 0u-mag : mag;
}
int cosmic_sf_to_i32(cosmic_sf_context *ctx,unsigned int a) {
    return (int)cosmic_sf_trunc_to_integer32(ctx,a,1);
}
unsigned int cosmic_sf_to_u32(cosmic_sf_context *ctx,unsigned int a) {
    return cosmic_sf_trunc_to_integer32(ctx,a,0);
}
static unsigned int cosmic_sf_from_mag32(cosmic_sf_context *ctx,unsigned int a,unsigned int sign) {
    unsigned int shifted=a,exp=0,sig,rem,half,dist;
    if(!a) return 0;
    while(shifted>>1) { shifted>>=1; exp++; }
    if(exp<=23) sig=a<<(23-exp);
    else {
        dist=exp-23;
        sig=a>>dist;
        rem=a&((1u<<dist)-1u);
        half=1u<<(dist-1);
        if(rem) cosmic_sf_raise(ctx,COSMIC_SF_INEXACT);
        if((ctx->rounding==COSMIC_SF_NEAREST && (rem>half || (rem==half && (sig&1u)))) ||
           (rem && ((ctx->rounding==COSMIC_SF_UPWARD && !sign) ||
                    (ctx->rounding==COSMIC_SF_DOWNWARD && sign)))) sig++;
        if(sig==0x01000000u) { sig>>=1;exp++; }
    }
    return ((exp+127u)<<23)|(sig&0x007fffffu);
}
unsigned int cosmic_sf_from_u32(cosmic_sf_context *ctx,unsigned int a) {
    return cosmic_sf_from_mag32(ctx,a,0);
}
unsigned int cosmic_sf_from_i32(cosmic_sf_context *ctx,int a) {
    unsigned int mag=(unsigned int)a,result;
    if(a<0) mag=0u-mag;
    result=cosmic_sf_from_mag32(ctx,mag,a<0);
    return result|(a<0 ? 0x80000000u : 0u);
}
