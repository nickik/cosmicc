/* Integer-only guest fixture generated against unmodified SoftFloat3e. */
#include "context.c"
#include "binary32-convert.c"
#include "convert32-vectors.h"
int main(void) {
    cosmic_sf_context ctx,other;
    unsigned int i,op,a,b,got,expected,flags,phase;
    cosmic_sf_init(&other);cosmic_sf_raise(&other,COSMIC_SF_OVERFLOW);
    for(i=0;i<COSMIC_SF_CONVERT_COUNT;i++) {
        op=cosmic_sf_convert_vectors[i][0];a=cosmic_sf_convert_vectors[i][1];b=cosmic_sf_convert_vectors[i][2];
        expected=cosmic_sf_convert_vectors[i][3];flags=cosmic_sf_convert_vectors[i][4];
        for(phase=0;phase<2;phase++) {
        cosmic_sf_init(&ctx);
        if(phase) cosmic_sf_raise(&ctx,COSMIC_SF_DIVIDE_BY_ZERO);
        switch(op) {
        case 0: got=(unsigned int)cosmic_sf_eq32(&ctx,a,b);break;
        case 1: got=(unsigned int)cosmic_sf_lt32(&ctx,a,b);break;
        case 2: got=(unsigned int)cosmic_sf_le32(&ctx,a,b);break;
        case 3: got=(unsigned int)cosmic_sf_to_i32(&ctx,a);break;
        case 4: got=cosmic_sf_to_u32(&ctx,a);break;
        case 5: got=cosmic_sf_from_i32(&ctx,(int)a);break;
        case 6: got=cosmic_sf_from_u32(&ctx,a);break;
        default:return 3;
        }
        if(got!=expected) return 1;
        if(cosmic_sf_flags(&ctx)!=(flags|(phase ? COSMIC_SF_DIVIDE_BY_ZERO : 0u))) return 2;
        if(cosmic_sf_flags(&other)!=COSMIC_SF_OVERFLOW) return 4;
        }
    }
    return 0;
}
