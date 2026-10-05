/* Additional C99 interfaces. Composite transcendentals inherit fdlibm accuracy. */
#include "math.h"
#include "binary32.h"
extern double cosmic_fd_erf(double),cosmic_fd_erfc(double),cosmic_fd_rint(double),cosmic_fd_modf(double,double *);
extern double __ieee754_remainder(double,double),__ieee754_lgamma_r(double,int *);
#define WRAP1(name,inner) double name(double x) { return inner(x); } float name##f(float x) { return (float)inner((double)x); } long double name##l(long double x) { return (long double)inner((double)x); }
WRAP1(erf,cosmic_fd_erf) WRAP1(erfc,cosmic_fd_erfc)
double remainder(double x,double y) { return __ieee754_remainder(x,y); }
long double remainderl(long double x,long double y) { return (long double)__ieee754_remainder((double)x,(double)y); }
float remainderf(float x,float y) { return (float)__ieee754_remainder((double)x,(double)y); }
double rint(double x) { union { double f;unsigned long long u; } a;a.f=x;a.u=cosmic_sf_rint64(cosmic_sf_current_context(),a.u,1);return a.f; }
float rintf(float x) { union { float f;unsigned int u; } a;a.f=x;a.u=cosmic_sf_rint32(cosmic_sf_current_context(),a.u,1);return a.f; }
long double rintl(long double x) { return (long double)rint((double)x); }
double nearbyint(double x) { union { double f;unsigned long long u; } a;a.f=x;a.u=cosmic_sf_rint64(cosmic_sf_current_context(),a.u,0);return a.f; }
float nearbyintf(float x) { union { float f;unsigned int u; } a;a.f=x;a.u=cosmic_sf_rint32(cosmic_sf_current_context(),a.u,0);return a.f; }
long double nearbyintl(long double x) { return (long double)nearbyint((double)x); }
long lrint(double x) { return (long)rint(x); } long lrintf(float x) { return (long)rintf(x); } long lrintl(long double x) { return (long)rintl(x); }
long long llrint(double x) { return (long long)rint(x); } long long llrintf(float x) { return (long long)rintf(x); } long long llrintl(long double x) { return (long long)rintl(x); }
long lround(double x) { return (long)round(x); } long lroundf(float x) { return (long)roundf(x); } long lroundl(long double x) { return (long)roundl(x); }
long long llround(double x) { return (long long)round(x); } long long llroundf(float x) { return (long long)roundf(x); } long long llroundl(long double x) { return (long long)roundl(x); }
double modf(double x,double *ip) { return cosmic_fd_modf(x,ip); }
float modff(float x,float *ip) { double i;double f=cosmic_fd_modf((double)x,&i);*ip=(float)i;return (float)f; }
long double modfl(long double x,long double *ip) { double i;double f=cosmic_fd_modf((double)x,&i);*ip=(long double)i;return (long double)f; }
static int bitexp(unsigned long long a,unsigned int p,unsigned int eb) {
    unsigned int exp=(unsigned int)((a>>p)&((1u<<eb)-1u));unsigned long long sig=a&((1ULL<<p)-1ULL);int e;
    if(exp==((1u<<eb)-1u) || (!exp&&!sig)) { cosmic_sf_raise(cosmic_sf_current_context(),COSMIC_SF_INVALID);return exp ? 2147483647 : (-2147483647-1); }
    e=(int)exp-((1<<(eb-1))-1);
    if(!exp) { e++;while(!(sig&(1ULL<<p))) { sig<<=1;e--; } }
    return e;
}
int ilogb(double x) { union { double f;unsigned long long u; } a;a.f=x;return bitexp(a.u,52,11); }
int ilogbf(float x) { union { float f;unsigned int u; } a;a.f=x;return bitexp(a.u,23,8); }
int ilogbl(long double x) { return ilogb((double)x); }
double logb(double x) { if(isnan(x)) return x+x;if(isinf(x)) return fabs(x);if(x==0.0) return -1.0/fabs(x);return (double)ilogb(x); }
float logbf(float x) { return (float)logb((double)x); }
long double logbl(long double x) { return (long double)logb((double)x); }
double exp2(double x) { double n;
    if(isnan(x)) return x+x;if(x==0.0) return 1.0;if(isinf(x)) return signbit(x) ? 0.0 : x;
    if(x>=1024.0) return scalbn(1.0,2000);if(x<= -1075.0) return scalbn(1.0,-2000);
    n=floor(x);return scalbn(exp((x-n)*0x1.62e42fefa39efp-1),(int)n);
}
float exp2f(float x) { return (float)exp2((double)x); } long double exp2l(long double x) { return (long double)exp2((double)x); }
double log2(double x) { int n;double f;if(isnan(x)) return x+x;if(x>0.0&&isfinite(x)) { f=frexp(x,&n);if(f==0.5) return (double)(n-1); }return log(x)*0x1.71547652b82fep0; }
float log2f(float x) { return (float)log2((double)x); } long double log2l(long double x) { return (long double)log2((double)x); }
double lgamma(double x) { int sign;return __ieee754_lgamma_r(x,&sign); }
float lgammaf(float x) { return (float)lgamma((double)x); } long double lgammal(long double x) { return (long double)lgamma((double)x); }
double tgamma(double x) { int sign;double y,z;cosmic_sf_context *c=cosmic_sf_current_context();unsigned int mode=c->rounding;
    if(isnan(x)) return x+x;
    if(x==0.0) return 1.0/x;
    if(x<0.0 && (isinf(x)||floor(x)==x)) { cosmic_sf_raise(c,COSMIC_SF_INVALID);return 0.0/0.0; }
    y=__ieee754_lgamma_r(x,&sign);
    if(sign<0 && mode==COSMIC_SF_UPWARD) c->rounding=COSMIC_SF_DOWNWARD;
    else if(sign<0 && mode==COSMIC_SF_DOWNWARD) c->rounding=COSMIC_SF_UPWARD;
    z=exp(y);c->rounding=mode;return sign<0 ? -z : z;
}
float tgammaf(float x) { return (float)tgamma((double)x); } long double tgammal(long double x) { return (long double)tgamma((double)x); }

float fmaf(float x,float y,float z) { union { float f;unsigned int u; } a,b,d;a.f=x;b.f=y;d.f=z;a.u=cosmic_sf_fma32(cosmic_sf_current_context(),a.u,b.u,d.u);return a.f; }
double fma(double x,double y,double z) { union { double f;unsigned long long u; } a,b,d;a.f=x;b.f=y;d.f=z;a.u=cosmic_sf_fma64(cosmic_sf_current_context(),a.u,b.u,d.u);return a.f; }
long double fmal(long double x,long double y,long double z) { return (long double)fma((double)x,(double)y,(double)z); }

double remquo(double x,double y,int *q) { union { double f;unsigned long long u; } a,b;a.f=x;b.f=y;a.u=cosmic_sf_remquo64(cosmic_sf_current_context(),a.u,b.u,q);return a.f; }
float remquof(float x,float y,int *q) { union { float f;unsigned int u; } a,b;a.f=x;b.f=y;a.u=cosmic_sf_remquo32(cosmic_sf_current_context(),a.u,b.u,q);return a.f; }
long double remquol(long double x,long double y,int *q) { return (long double)remquo((double)x,(double)y,q); }
double nextafter(double x,double y) { union { double f;unsigned long long u; } a,b;a.f=x;b.f=y;a.u=cosmic_sf_nextafter64(cosmic_sf_current_context(),a.u,b.u);return a.f; }
float nextafterf(float x,float y) { union { float f;unsigned int u; } a,b;a.f=x;b.f=y;a.u=cosmic_sf_nextafter32(cosmic_sf_current_context(),a.u,b.u);return a.f; }
long double nextafterl(long double x,long double y) { return (long double)nextafter((double)x,(double)y); }
double nexttoward(double x,long double y) { return nextafter(x,(double)y); }
long double nexttowardl(long double x,long double y) { return nextafterl(x,y); }
float nexttowardf(float x,long double y) { union { float f;unsigned int u; } a;union { long double f;unsigned long long u; } b;a.f=x;b.f=y;a.u=cosmic_sf_nexttoward32(cosmic_sf_current_context(),a.u,b.u);return a.f; }
double fdim(double x,double y) { if(isnan(x)||isnan(y)) return x+y;return x>y ? x-y : 0.0; }
float fdimf(float x,float y) { if(isnan(x)||isnan(y)) return x+y;return x>y ? x-y : 0.0f; }
long double fdiml(long double x,long double y) { if(isnan(x)||isnan(y)) return x+y;return x>y ? x-y : 0.0L; }
double fmax(double x,double y) { if(isnan(x)&&isnan(y)) return x+y;if(isnan(x)) { (void)(x+x);return y; }if(isnan(y)) { (void)(y+y);return x; }if(x==y) return signbit(x) ? y : x;return x>y ? x : y; }
double fmin(double x,double y) { if(isnan(x)&&isnan(y)) return x+y;if(isnan(x)) { (void)(x+x);return y; }if(isnan(y)) { (void)(y+y);return x; }if(x==y) return signbit(x) ? x : y;return x<y ? x : y; }
float fmaxf(float x,float y) { return (float)fmax((double)x,(double)y); }long double fmaxl(long double x,long double y) { return (long double)fmax((double)x,(double)y); }
float fminf(float x,float y) { return (float)fmin((double)x,(double)y); }long double fminl(long double x,long double y) { return (long double)fmin((double)x,(double)y); }
double scalbln(double x,long n) { if(n>10000) n=10000;if(n< -10000) n= -10000;return scalbn(x,(int)n); }
float scalblnf(float x,long n) { if(n>1000) n=1000;if(n< -1000) n= -1000;return scalbnf(x,(int)n); }
long double scalblnl(long double x,long n) { return (long double)scalbln((double)x,n); }
double nan(const char *tag) { union { double f;unsigned long long u; } a;(void)tag;a.u=0x7ff8000000000000ULL;return a.f; }
float nanf(const char *tag) { union { float f;unsigned int u; } a;(void)tag;a.u=0x7fc00000u;return a.f; }
long double nanl(const char *tag) { return (long double)nan(tag); }
int cosmic_isgreater(double a,double b) { return !isnan(a)&&!isnan(b)&&a>b; }
int cosmic_isgreaterequal(double a,double b) { return !isnan(a)&&!isnan(b)&&a>=b; }
int cosmic_isless(double a,double b) { return !isnan(a)&&!isnan(b)&&a<b; }
int cosmic_islessequal(double a,double b) { return !isnan(a)&&!isnan(b)&&a<=b; }
int cosmic_islessgreater(double a,double b) { return !isnan(a)&&!isnan(b)&&a!=b; }
int cosmic_isunordered(double a,double b) { return isnan(a)||isnan(b); }

extern double __ieee754_acosh(double),__ieee754_atanh(double),cosmic_fd_asinh(double);
WRAP1(acosh,__ieee754_acosh) WRAP1(asinh,cosmic_fd_asinh) WRAP1(atanh,__ieee754_atanh)
