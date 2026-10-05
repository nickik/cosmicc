#include "context.c"
#include "binary64.c"
#include "binary64-dispatch-test.h"
#include "binary64-vectors.h"
int main(void) {
    cosmic_sf_context ctx,other;
    unsigned int i;unsigned long long result;
    cosmic_sf_init(&other);cosmic_sf_raise(&other,COSMIC_SF_INVALID);
    for(i=0;i<COSMIC_SF_VECTOR64_COUNT;i++) {
        cosmic_sf_init(&ctx);
        result=cosmic_sf_test_binary64(&ctx,cosmic_sf_vectors64[i].op,cosmic_sf_vectors64[i].a,cosmic_sf_vectors64[i].b);
        if(result!=cosmic_sf_vectors64[i].result)return (int)(1000u+i);
        if(ctx.flags!=cosmic_sf_vectors64[i].flags)return (int)(3000u+i);
        if(other.flags!=COSMIC_SF_INVALID)return 3;
    }
    cosmic_sf_init(&ctx);
    if(cosmic_sf_div64(&ctx,0x3ff0000000000000ULL,0)!=0x7ff0000000000000ULL)return 4;
    if(cosmic_sf_mul64(&ctx,0,0x7ff0000000000000ULL)!=0x7ff8000000000000ULL)return 5;
    if(ctx.flags!=(COSMIC_SF_DIVIDE_BY_ZERO|COSMIC_SF_INVALID))return 6;
    return 0;
}
