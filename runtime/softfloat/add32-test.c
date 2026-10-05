/* Integer-only guest checks. Including sources forms a freestanding one-TU
   fixture; production callers can instead link context.c and binary32-add.c. */
#include "context.c"
#include "binary32-add.c"
#include "add32-vectors.h"
int main(void) {
    cosmic_sf_context ctx, other;
    unsigned int i, got;
    cosmic_sf_init(&other);
    cosmic_sf_raise(&other,COSMIC_SF_INVALID);
    for(i=0;i<COSMIC_SF_ADD32_VECTOR_COUNT;i++) {
        cosmic_sf_init(&ctx);
        cosmic_sf_raise(&ctx,COSMIC_SF_DIVIDE_BY_ZERO);
        got=cosmic_sf_add32(&ctx,cosmic_sf_add32_vectors[i][0],cosmic_sf_add32_vectors[i][1]);
        if(got!=cosmic_sf_add32_vectors[i][2]) return (int)(1000u+i);
        if(cosmic_sf_flags(&ctx)!=(cosmic_sf_add32_vectors[i][3]|COSMIC_SF_DIVIDE_BY_ZERO)) return (int)(2000u+i);
        if(cosmic_sf_flags(&other)!=COSMIC_SF_INVALID) return 3;
    }
    cosmic_sf_init(&ctx);
    if(cosmic_sf_add32(&ctx,0x3f800000u,0x33800000u)!=0x3f800000u) return 4;
    if(cosmic_sf_add32(&ctx,0x7f800000u,0xff800000u)!=0x7fc00000u) return 5;
    if(ctx.flags!=(COSMIC_SF_INEXACT|COSMIC_SF_INVALID)) return 6;
    cosmic_sf_clear(&ctx,COSMIC_SF_INEXACT);
    if(ctx.flags!=COSMIC_SF_INVALID) return 7;
    return 0;
}
