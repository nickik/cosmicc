/* SIA wrappers: the compiler lowers these FP values through integer helpers. */
#include "binary32.h"
#include "math.h"
#define UNARY32(name) float name##f(float x) { union { float f;unsigned int u; } a;a.f=x;a.u=cosmic_sf_##name##32(cosmic_sf_current_context(),a.u);return a.f; }
#define UNARY64(name) double name(double x) { union { double f;unsigned long long u; } a;a.f=x;a.u=cosmic_sf_##name##64(cosmic_sf_current_context(),a.u);return a.f; } \
long double name##l(long double x) { union { long double f;unsigned long long u; } a;a.f=x;a.u=cosmic_sf_##name##64(cosmic_sf_current_context(),a.u);return a.f; }
#define BINARY32(name) float name##f(float x,float y) { union { float f;unsigned int u; } a,b;a.f=x;b.f=y;a.u=cosmic_sf_##name##32(cosmic_sf_current_context(),a.u,b.u);return a.f; }
#define BINARY64(name) double name(double x,double y) { union { double f;unsigned long long u; } a,b;a.f=x;b.f=y;a.u=cosmic_sf_##name##64(cosmic_sf_current_context(),a.u,b.u);return a.f; } \
long double name##l(long double x,long double y) { union { long double f;unsigned long long u; } a,b;a.f=x;b.f=y;a.u=cosmic_sf_##name##64(cosmic_sf_current_context(),a.u,b.u);return a.f; }
UNARY32(sqrt) UNARY64(sqrt)
UNARY32(fabs) UNARY64(fabs)
UNARY32(floor) UNARY64(floor)
UNARY32(ceil) UNARY64(ceil)
UNARY32(trunc) UNARY64(trunc)
UNARY32(round) UNARY64(round)
BINARY32(fmod) BINARY64(fmod)
BINARY32(copysign) BINARY64(copysign)
float scalbnf(float x,int n) { union { float f;unsigned int u; } a;a.f=x;a.u=cosmic_sf_scalbn32(cosmic_sf_current_context(),a.u,n);return a.f; }
double scalbn(double x,int n) { union { double f;unsigned long long u; } a;a.f=x;a.u=cosmic_sf_scalbn64(cosmic_sf_current_context(),a.u,n);return a.f; }
long double scalbnl(long double x,int n) { union { long double f;unsigned long long u; } a;a.f=x;a.u=cosmic_sf_scalbn64(cosmic_sf_current_context(),a.u,n);return a.f; }
float ldexpf(float x,int n) { return scalbnf(x,n); }
double ldexp(double x,int n) { return scalbn(x,n); }
long double ldexpl(long double x,int n) { return scalbnl(x,n); }
float frexpf(float x,int *n) { union { float f;unsigned int u; } a;a.f=x;a.u=cosmic_sf_frexp32(cosmic_sf_current_context(),a.u,n);return a.f; }
double frexp(double x,int *n) { union { double f;unsigned long long u; } a;a.f=x;a.u=cosmic_sf_frexp64(cosmic_sf_current_context(),a.u,n);return a.f; }
long double frexpl(long double x,int *n) { union { long double f;unsigned long long u; } a;a.f=x;a.u=cosmic_sf_frexp64(cosmic_sf_current_context(),a.u,n);return a.f; }
static int classify(unsigned long long x,unsigned int p,unsigned int eb) { unsigned long long f=x&((1ULL<<p)-1ULL);unsigned int e=(unsigned int)((x>>p)&((1u<<eb)-1u));if(e==((1u<<eb)-1u)) return f ? FP_NAN : FP_INFINITE;if(!e) return f ? FP_SUBNORMAL : FP_ZERO;return FP_NORMAL; }
int cosmic_fpclassifyf(float x) { union { float f;unsigned int u; } a;a.f=x;return classify(a.u,23,8); }
int cosmic_fpclassify(double x) { union { double f;unsigned long long u; } a;a.f=x;return classify(a.u,52,11); }
int cosmic_fpclassifyl(long double x) { union { long double f;unsigned long long u; } a;a.f=x;return classify(a.u,52,11); }
int cosmic_signbitf(float x) { union { float f;unsigned int u; } a;a.f=x;return (int)(a.u>>31); }
int cosmic_signbit(double x) { union { double f;unsigned long long u; } a;a.f=x;return (int)(a.u>>63); }
int cosmic_signbitl(long double x) { union { long double f;unsigned long long u; } a;a.f=x;return (int)(a.u>>63); }
