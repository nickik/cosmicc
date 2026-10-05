/* Test-only unmodified SoftFloat wrappers. Reference state is single-threaded. */
#include "upstream/include/softfloat.h"
unsigned int cosmic_sf_reference_convert(unsigned int op,unsigned int a,unsigned int b,unsigned int *flags) {
    float32_t x,y,z;unsigned int result=0;
    x.v=a;y.v=b;
    softfloat_roundingMode=softfloat_round_near_even;
    softfloat_detectTininess=softfloat_tininess_afterRounding;
    softfloat_exceptionFlags=0;
    switch(op) {
    case 0: result=f32_eq(x,y);break;
    case 1: result=f32_lt(x,y);break;
    case 2: result=f32_le(x,y);break;
    case 3: result=(unsigned int)f32_to_i32_r_minMag(x,1);break;
    case 4: result=(unsigned int)f32_to_ui32_r_minMag(x,1);break;
    case 5: z=i32_to_f32((int32_t)a);result=z.v;break;
    case 6: z=ui32_to_f32(a);result=z.v;break;
    }
    *flags=softfloat_exceptionFlags;return result;
}
