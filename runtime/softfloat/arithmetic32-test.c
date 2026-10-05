#include "context.c"
#include "binary32-add.c"
#include "arithmetic32-vectors.h"
int main(void) {
    cosmic_sf_context ctx,other;
    unsigned int i,op,got,a,b;
    cosmic_sf_init(&other);
    cosmic_sf_raise(&other,COSMIC_SF_OVERFLOW);
    for(i=0;i<COSMIC_SF_ARITHMETIC32_VECTOR_COUNT;i++) {
        op=cosmic_sf_arithmetic32_vectors[i][0];
        a=cosmic_sf_arithmetic32_vectors[i][1];
        b=cosmic_sf_arithmetic32_vectors[i][2];
        cosmic_sf_init(&ctx);
        if(op==0)got=cosmic_sf_sub32(&ctx,a,b);
        else if(op==1)got=cosmic_sf_mul32(&ctx,a,b);
        else got=cosmic_sf_div32(&ctx,a,b);
        if(got!=cosmic_sf_arithmetic32_vectors[i][3])return (int)(1000u+i);
        if(ctx.flags!=cosmic_sf_arithmetic32_vectors[i][4])return (int)(2000u+i);
        if(other.flags!=COSMIC_SF_OVERFLOW)return 3;
    }
    cosmic_sf_init(&ctx);
    if(cosmic_sf_div32(&ctx,0x3f800000u,0)!=0x7f800000u)return 4;
    if(cosmic_sf_mul32(&ctx,0,0x7f800000u)!=0x7fc00000u)return 5;
    if(cosmic_sf_mul32(&ctx,1,0x3f000000u)!=0)return 6;
    if(ctx.flags!=(COSMIC_SF_DIVIDE_BY_ZERO|COSMIC_SF_INVALID|COSMIC_SF_UNDERFLOW|COSMIC_SF_INEXACT))return 7;
    return 0;
}
