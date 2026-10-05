/* Test-only wrapper around unmodified upstream integer SoftFloat sources.
   Ordinary globals are confined to this single-threaded reference process. */
#include "upstream/include/softfloat.h"
unsigned int cosmic_sf_reference_add32(unsigned int a,unsigned int b,unsigned int *flags) {
    float32_t x,y,z;
    x.v=a; y.v=b;
    softfloat_roundingMode=softfloat_round_near_even;
    softfloat_detectTininess=softfloat_tininess_afterRounding;
    softfloat_exceptionFlags=0;
    z=f32_add(x,y);
    *flags=softfloat_exceptionFlags;
    return z.v;
}
