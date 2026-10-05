#include "upstream/include/softfloat.h"
uint64_t cosmic_sf_reference_fma(unsigned int width,unsigned int mode,unsigned int op,uint64_t a,uint64_t b,uint64_t c,unsigned int *flags) {
    float64_t x,y,z,r;float32_t xf,yf,zf,rf;uint64_t bits;
    x.v=a;y.v=b;z.v=c;xf.v=(uint32_t)a;yf.v=(uint32_t)b;zf.v=(uint32_t)c;
    softfloat_roundingMode=mode;softfloat_detectTininess=softfloat_tininess_afterRounding;softfloat_exceptionFlags=0;
    if(width==32) { rf=op ? f32_roundToInt(xf,mode,true) : f32_mulAdd(xf,yf,zf);bits=rf.v; }
    else { r=op ? f64_roundToInt(x,mode,true) : f64_mulAdd(x,y,z);bits=r.v; }
    *flags=softfloat_exceptionFlags;return bits;
}
