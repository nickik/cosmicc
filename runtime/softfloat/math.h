#ifndef COSMIC_MATH_H
#define COSMIC_MATH_H
#define MATH_ERRNO 1
#define MATH_ERREXCEPT 2
#define math_errhandling MATH_ERREXCEPT
#define HUGE_VAL (__builtin_inf())
#define HUGE_VALF (__builtin_inff())
#define HUGE_VALL (__builtin_infl())
#define INFINITY (__builtin_inff())
#define NAN (__builtin_nanf(""))
#define FP_NAN 0
#define FP_INFINITE 1
#define FP_ZERO 2
#define FP_SUBNORMAL 3
#define FP_NORMAL 4
float sqrtf(float); double sqrt(double); long double sqrtl(long double);
float fmodf(float,float); double fmod(double,double); long double fmodl(long double,long double);
float fabsf(float); double fabs(double); long double fabsl(long double);
float copysignf(float,float); double copysign(double,double); long double copysignl(long double,long double);
float floorf(float); double floor(double); long double floorl(long double);
float ceilf(float); double ceil(double); long double ceill(long double);
float truncf(float); double trunc(double); long double truncl(long double);
float roundf(float); double round(double); long double roundl(long double);
float scalbnf(float,int); double scalbn(double,int); long double scalbnl(long double,int);
float ldexpf(float,int); double ldexp(double,int); long double ldexpl(long double,int);
float frexpf(float,int *); double frexp(double,int *); long double frexpl(long double,int *);
int cosmic_fpclassifyf(float); int cosmic_fpclassify(double); int cosmic_fpclassifyl(long double);
int cosmic_signbitf(float); int cosmic_signbit(double); int cosmic_signbitl(long double);
/* sizeof selects by format; only the selected branch evaluates its argument.
   SIA long double shares binary64, so double conversion is exact. */
#define fpclassify(x) (sizeof(x)==sizeof(float) ? cosmic_fpclassifyf(x) : cosmic_fpclassify(x))
#define signbit(x) (sizeof(x)==sizeof(float) ? cosmic_signbitf(x) : cosmic_signbit(x))
#define isnan(x) (fpclassify(x)==FP_NAN)
#define isinf(x) (fpclassify(x)==FP_INFINITE)
#define isfinite(x) (fpclassify(x)>=FP_ZERO)
#define isnormal(x) (fpclassify(x)==FP_NORMAL)
double exp(double); float expf(float); long double expl(long double);
double log(double); float logf(float); long double logl(long double);
double log10(double); float log10f(float); long double log10l(long double);
double sin(double); float sinf(float); long double sinl(long double);
double cos(double); float cosf(float); long double cosl(long double);
double tan(double); float tanf(float); long double tanl(long double);
double asin(double); float asinf(float); long double asinl(long double);
double acos(double); float acosf(float); long double acosl(long double);
double atan(double); float atanf(float); long double atanl(long double);
double sinh(double); float sinhf(float); long double sinhl(long double);
double cosh(double); float coshf(float); long double coshl(long double);
double tanh(double); float tanhf(float); long double tanhl(long double);
double expm1(double); float expm1f(float); long double expm1l(long double);
double log1p(double); float log1pf(float); long double log1pl(long double);
double cbrt(double); float cbrtf(float); long double cbrtl(long double);
double pow(double,double); float powf(float,float); long double powl(long double,long double);
double atan2(double,double); float atan2f(float,float); long double atan2l(long double,long double);
double hypot(double,double); float hypotf(float,float); long double hypotl(long double,long double);
#define FP_ILOGB0 (-2147483647-1)
#define FP_ILOGBNAN 2147483647
double erf(double);float erff(float);long double erfl(long double);
double erfc(double);float erfcf(float);long double erfcl(long double);
double rint(double);float rintf(float);long double rintl(long double);
double nearbyint(double);float nearbyintf(float);long double nearbyintl(long double);
double logb(double);float logbf(float);long double logbl(long double);
double exp2(double);float exp2f(float);long double exp2l(long double);
double log2(double);float log2f(float);long double log2l(long double);
double lgamma(double);float lgammaf(float);long double lgammal(long double);
double tgamma(double);float tgammaf(float);long double tgammal(long double);
long lrint(double);long lrintf(float);long lrintl(long double);
long lround(double);long lroundf(float);long lroundl(long double);
long long llrint(double);long long llrintf(float);long long llrintl(long double);
long long llround(double);long long llroundf(float);long long llroundl(long double);
int ilogb(double);int ilogbf(float);int ilogbl(long double);
double remainder(double,double);float remainderf(float,float);long double remainderl(long double,long double);
double modf(double,double *);float modff(float,float *);long double modfl(long double,long double *);
double fma(double,double,double);float fmaf(float,float,float);long double fmal(long double,long double,long double);
double remquo(double,double,int *);float remquof(float,float,int *);long double remquol(long double,long double,int *);
double nextafter(double,double);float nextafterf(float,float);long double nextafterl(long double,long double);
double nexttoward(double,long double);float nexttowardf(float,long double);long double nexttowardl(long double,long double);
double fdim(double,double);float fdimf(float,float);long double fdiml(long double,long double);
double fmax(double,double);float fmaxf(float,float);long double fmaxl(long double,long double);
double fmin(double,double);float fminf(float,float);long double fminl(long double,long double);
double scalbln(double,long);float scalblnf(float,long);long double scalblnl(long double,long);
double nan(const char *);float nanf(const char *);long double nanl(const char *);
int cosmic_isgreater(double,double);
#define isgreater(a,b) cosmic_isgreater((a),(b))
int cosmic_isgreaterequal(double,double);
#define isgreaterequal(a,b) cosmic_isgreaterequal((a),(b))
int cosmic_isless(double,double);
#define isless(a,b) cosmic_isless((a),(b))
int cosmic_islessequal(double,double);
#define islessequal(a,b) cosmic_islessequal((a),(b))
int cosmic_islessgreater(double,double);
#define islessgreater(a,b) cosmic_islessgreater((a),(b))
int cosmic_isunordered(double,double);
#define isunordered(a,b) cosmic_isunordered((a),(b))
double acosh(double);float acoshf(float);long double acoshl(long double);
double asinh(double);float asinhf(float);long double asinhl(long double);
double atanh(double);float atanhf(float);long double atanhl(long double);
#endif
