/* Single-threaded test-only integer reference using unchanged SoftFloat3e. */
#include "upstream/include/softfloat.h"
unsigned int cosmic_sf_reference_arithmetic32(unsigned int op,unsigned int a,unsigned int b,unsigned int *flags) {
    float32_t x,y,z;
    x.v=a;y.v=b;
    softfloat_roundingMode=softfloat_round_near_even;
    softfloat_detectTininess=softfloat_tininess_afterRounding;
    softfloat_exceptionFlags=0;
    if(op==0) z=f32_sub(x,y);
    else if(op==1) z=f32_mul(x,y);
    else z=f32_div(x,y);
    *flags=softfloat_exceptionFlags;
    return z.v;
}
