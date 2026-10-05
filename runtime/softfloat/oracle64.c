/* Integer-only, single-threaded test oracle; unchanged upstream arithmetic. */
#include "upstream/include/softfloat.h"
uint64_t cosmic_sf_reference_binary64(unsigned int op,uint64_t a,uint64_t b,unsigned int *flags) {
    float64_t x,y,z;float32_t f,g;uint64_t r;
    x.v=a;y.v=b;f.v=(uint32_t)a;
    softfloat_roundingMode=softfloat_round_near_even;
    softfloat_detectTininess=softfloat_tininess_afterRounding;
    softfloat_exceptionFlags=0;
    switch(op) {
    case 0:z=f64_add(x,y);r=z.v;break;
    case 1:z=f64_sub(x,y);r=z.v;break;
    case 2:z=f64_mul(x,y);r=z.v;break;
    case 3:z=f64_div(x,y);r=z.v;break;
    case 4:r=f64_eq(x,y);break;
    case 5:r=f64_lt(x,y);break;
    case 6:r=f64_le(x,y);break;
    case 7:z=f32_to_f64(f);r=z.v;break;
    case 8:g=f64_to_f32(x);r=g.v;break;
    case 9:r=(uint64_t)f32_to_i64_r_minMag(f,true);break;
    case 10:r=f32_to_ui64_r_minMag(f,true);break;
    case 11:r=(uint64_t)f64_to_i64_r_minMag(x,true);break;
    case 12:r=f64_to_ui64_r_minMag(x,true);break;
    case 13:r=(uint32_t)f64_to_i32_r_minMag(x,true);break;
    case 14:r=f64_to_ui32_r_minMag(x,true);break;
    case 15:g=i64_to_f32((int64_t)a);r=g.v;break;
    case 16:g=ui64_to_f32(a);r=g.v;break;
    case 17:z=i64_to_f64((int64_t)a);r=z.v;break;
    case 18:z=ui64_to_f64(a);r=z.v;break;
    case 19:z=i32_to_f64((int32_t)a);r=z.v;break;
    default:z=ui32_to_f64((uint32_t)a);r=z.v;break;
    }
    *flags=softfloat_exceptionFlags;return r;
}
