/* Licensed fdlibm algorithms execute through SIA software FP lowering. */
#include "math.h"
extern double __ieee754_exp(double);
double exp(double x) { return __ieee754_exp(x); }
float expf(float x) { return (float)__ieee754_exp((double)x); }
long double expl(long double x) { return (long double)__ieee754_exp((double)x); }
extern double __ieee754_log(double);
double log(double x) { return __ieee754_log(x); }
float logf(float x) { return (float)__ieee754_log((double)x); }
long double logl(long double x) { return (long double)__ieee754_log((double)x); }
extern double __ieee754_log10(double);
double log10(double x) { return __ieee754_log10(x); }
float log10f(float x) { return (float)__ieee754_log10((double)x); }
long double log10l(long double x) { return (long double)__ieee754_log10((double)x); }
extern double cosmic_fd_sin(double);
double sin(double x) { return cosmic_fd_sin(x); }
float sinf(float x) { return (float)cosmic_fd_sin((double)x); }
long double sinl(long double x) { return (long double)cosmic_fd_sin((double)x); }
extern double cosmic_fd_cos(double);
double cos(double x) { return cosmic_fd_cos(x); }
float cosf(float x) { return (float)cosmic_fd_cos((double)x); }
long double cosl(long double x) { return (long double)cosmic_fd_cos((double)x); }
extern double cosmic_fd_tan(double);
double tan(double x) { return cosmic_fd_tan(x); }
float tanf(float x) { return (float)cosmic_fd_tan((double)x); }
long double tanl(long double x) { return (long double)cosmic_fd_tan((double)x); }
extern double __ieee754_asin(double);
double asin(double x) { return __ieee754_asin(x); }
float asinf(float x) { return (float)__ieee754_asin((double)x); }
long double asinl(long double x) { return (long double)__ieee754_asin((double)x); }
extern double __ieee754_acos(double);
double acos(double x) { return __ieee754_acos(x); }
float acosf(float x) { return (float)__ieee754_acos((double)x); }
long double acosl(long double x) { return (long double)__ieee754_acos((double)x); }
extern double cosmic_fd_atan(double);
double atan(double x) { return cosmic_fd_atan(x); }
float atanf(float x) { return (float)cosmic_fd_atan((double)x); }
long double atanl(long double x) { return (long double)cosmic_fd_atan((double)x); }
extern double __ieee754_sinh(double);
double sinh(double x) { return __ieee754_sinh(x); }
float sinhf(float x) { return (float)__ieee754_sinh((double)x); }
long double sinhl(long double x) { return (long double)__ieee754_sinh((double)x); }
extern double __ieee754_cosh(double);
double cosh(double x) { return __ieee754_cosh(x); }
float coshf(float x) { return (float)__ieee754_cosh((double)x); }
long double coshl(long double x) { return (long double)__ieee754_cosh((double)x); }
extern double cosmic_fd_tanh(double);
double tanh(double x) { return cosmic_fd_tanh(x); }
float tanhf(float x) { return (float)cosmic_fd_tanh((double)x); }
long double tanhl(long double x) { return (long double)cosmic_fd_tanh((double)x); }
extern double cosmic_fd_expm1(double);
double expm1(double x) { return cosmic_fd_expm1(x); }
float expm1f(float x) { return (float)cosmic_fd_expm1((double)x); }
long double expm1l(long double x) { return (long double)cosmic_fd_expm1((double)x); }
extern double cosmic_fd_log1p(double);
double log1p(double x) { return cosmic_fd_log1p(x); }
float log1pf(float x) { return (float)cosmic_fd_log1p((double)x); }
long double log1pl(long double x) { return (long double)cosmic_fd_log1p((double)x); }
extern double cosmic_fd_cbrt(double);
double cbrt(double x) { return cosmic_fd_cbrt(x); }
float cbrtf(float x) { return (float)cosmic_fd_cbrt((double)x); }
long double cbrtl(long double x) { return (long double)cosmic_fd_cbrt((double)x); }
extern double __ieee754_pow(double,double);
double pow(double x,double y) { return __ieee754_pow(x,y); }
float powf(float x,float y) { return (float)__ieee754_pow((double)x,(double)y); }
long double powl(long double x,long double y) { return (long double)__ieee754_pow((double)x,(double)y); }
extern double __ieee754_atan2(double,double);
double atan2(double x,double y) { return __ieee754_atan2(x,y); }
float atan2f(float x,float y) { return (float)__ieee754_atan2((double)x,(double)y); }
long double atan2l(long double x,long double y) { return (long double)__ieee754_atan2((double)x,(double)y); }
extern double __ieee754_hypot(double,double);
double hypot(double x,double y) { return __ieee754_hypot(x,y); }
float hypotf(float x,float y) { return (float)__ieee754_hypot((double)x,(double)y); }
long double hypotl(long double x,long double y) { return (long double)__ieee754_hypot((double)x,(double)y); }
