/* Host libm is a test oracle only. No target code calls this file. */
#include <math.h>
#include <fenv.h>
#include <stdint.h>
uint64_t cosmic_math_reference(unsigned int width,unsigned int op,uint64_t a,uint64_t b,int n,unsigned int mode,int *e) {
    union { double f;uint64_t u; } x,y,z;
    union { float f;uint32_t u; } xf,yf,zf;
    static const int modes[]={FE_TONEAREST,FE_TOWARDZERO,FE_DOWNWARD,FE_UPWARD};
    fesetround(modes[mode]); *e=0;x.u=a;y.u=b;xf.u=(uint32_t)a;yf.u=(uint32_t)b;
    if(width==32) {
        switch(op) {
        case 0:zf.f=fmodf(xf.f,yf.f);break;
        case 1:zf.f=floorf(xf.f);break;
        case 2:zf.f=ceilf(xf.f);break;
        case 3:zf.f=truncf(xf.f);break;
        case 4:zf.f=roundf(xf.f);break;
        case 5:zf.f=fabsf(xf.f);break;
        case 6:zf.f=copysignf(xf.f,yf.f);break;
        case 7:zf.f=scalbnf(xf.f,n);break;
        case 8:zf.f=frexpf(xf.f,e);break;case 9:zf.f=remquof(xf.f,yf.f,e);break;default:zf.f=nextafterf(xf.f,yf.f);break;
        }return zf.u;
    }
    switch(op) {
    case 0:z.f=fmod(x.f,y.f);break;
    case 1:z.f=floor(x.f);break;
    case 2:z.f=ceil(x.f);break;
    case 3:z.f=trunc(x.f);break;
    case 4:z.f=round(x.f);break;
    case 5:z.f=fabs(x.f);break;
    case 6:z.f=copysign(x.f,y.f);break;
    case 7:z.f=scalbn(x.f,n);break;
    case 8:z.f=frexp(x.f,e);break;case 9:z.f=remquo(x.f,y.f,e);break;default:z.f=nextafter(x.f,y.f);break;
    }return z.u;
}
