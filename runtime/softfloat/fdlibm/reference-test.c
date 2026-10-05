/* Reference process only: original fdlibm, host arithmetic; never linked guest. */
#include <stdint.h>
#include <fenv.h>
extern double __ieee754_exp(double),__ieee754_log(double),__ieee754_log10(double),__ieee754_asin(double),__ieee754_acos(double),__ieee754_sinh(double),__ieee754_cosh(double);
extern double __ieee754_pow(double,double),__ieee754_atan2(double,double),__ieee754_hypot(double,double);
extern double cosmic_fd_sin(double),cosmic_fd_cos(double),cosmic_fd_tan(double),cosmic_fd_atan(double),cosmic_fd_tanh(double),cosmic_fd_expm1(double),cosmic_fd_log1p(double),cosmic_fd_cbrt(double);
extern double cosmic_fd_erf(double),cosmic_fd_erfc(double),cosmic_fd_asinh(double),__ieee754_acosh(double),__ieee754_atanh(double),__ieee754_lgamma_r(double,int *),__ieee754_remainder(double,double);
uint64_t cosmic_fd_reference(unsigned int op,uint64_t a,uint64_t b,unsigned int *flags) {
    union { double f;uint64_t u; } x,y,z;int f,sign;
    x.u=a;y.u=b;fesetround(FE_TONEAREST);feclearexcept(FE_ALL_EXCEPT);
    switch(op) {
    case 0:z.f=__ieee754_exp(x.f);break;case 1:z.f=__ieee754_log(x.f);break;
    case 2:z.f=__ieee754_log10(x.f);break;case 3:z.f=cosmic_fd_sin(x.f);break;
    case 4:z.f=cosmic_fd_cos(x.f);break;case 5:z.f=cosmic_fd_tan(x.f);break;
    case 6:z.f=__ieee754_asin(x.f);break;case 7:z.f=__ieee754_acos(x.f);break;
    case 8:z.f=cosmic_fd_atan(x.f);break;case 9:z.f=__ieee754_sinh(x.f);break;
    case 10:z.f=__ieee754_cosh(x.f);break;case 11:z.f=cosmic_fd_tanh(x.f);break;
    case 12:z.f=cosmic_fd_expm1(x.f);break;case 13:z.f=cosmic_fd_log1p(x.f);break;
    case 14:z.f=cosmic_fd_cbrt(x.f);break;case 15:z.f=__ieee754_pow(x.f,y.f);break;
    case 16:z.f=__ieee754_atan2(x.f,y.f);break;case 17:z.f=__ieee754_hypot(x.f,y.f);break;
    case 18:z.f=cosmic_fd_erf(x.f);break;case 19:z.f=cosmic_fd_erfc(x.f);break;
    case 20:z.f=__ieee754_lgamma_r(x.f,&sign);break;case 21:z.f=__ieee754_acosh(x.f);break;
    case 22:z.f=cosmic_fd_asinh(x.f);break;case 23:z.f=__ieee754_atanh(x.f);break;
    default:z.f=__ieee754_remainder(x.f,y.f);break;
    }
    f=fetestexcept(FE_ALL_EXCEPT);*flags=0;
    if(f&FE_INEXACT)*flags|=1;if(f&FE_UNDERFLOW)*flags|=2;if(f&FE_OVERFLOW)*flags|=4;
    if(f&FE_DIVBYZERO)*flags|=8;if(f&FE_INVALID)*flags|=16;
    return z.u;
}
