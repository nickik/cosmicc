#include <math.h>
#include <fenv.h>
static double constant_inf=HUGE_VAL;
static float constant_nan=NAN;
static long double constant_ldinf=HUGE_VALL;
int main(void) {
    int q,e,mode;double ip,d;float fi;long double li;
    if(math_errhandling!=MATH_ERREXCEPT) return 1;
    feclearexcept(FE_ALL_EXCEPT);
    if(!isinf(constant_inf)||!isnan(constant_nan)||!isinf(constant_ldinf)||fetestexcept(FE_ALL_EXCEPT)) return 2;
    if(isgreater(constant_nan,1.0)||isless(constant_nan,1.0)||!isunordered(constant_nan,1.0)||fetestexcept(FE_INVALID)) return 3;
    for(mode=0;mode<4;mode++) {
        double positive=mode==0||mode==3 ? 2.0 : 1.0;
        double negative=mode==0||mode==2 ? -2.0 : -1.0;
        if(fesetround(mode)) return 4;
        feclearexcept(FE_ALL_EXCEPT);
        if(rint(1.5)!=positive || rintf(-1.5f)!=(float)negative || rintl(1.5L)!=(long double)positive || !(fetestexcept(FE_INEXACT))) return 5;
        feclearexcept(FE_ALL_EXCEPT);
        if(nearbyint(1.5)!=positive || nearbyintf(-1.5f)!=(float)negative || nearbyintl(1.5L)!=(long double)positive || fetestexcept(FE_INEXACT)) return 6;
        if(lrint(1.5)!=(long)positive || llrintf(-1.5f)!=(long long)negative || llrintl(1.5L)!=(long long)positive) return 7;
        if(lround(-1.5)!= -2 || llroundf(1.5f)!=2 || llroundl(-1.5L)!= -2) return 8;
    }
    fesetround(FE_TONEAREST);feclearexcept(FE_ALL_EXCEPT);
    if(fma(0x1.0000000000001p0,0x1.ffffffffffffep-1,-1.0)!= -0x1p-104) return 9;
    if(fmaf(0x1.000002p0f,0x1.fffffcp-1f,-1.0f)!= -0x1p-46f) return 10;
    if(fmal(0x1.0000000000001p0L,0x1.ffffffffffffep-1L,-1.0L)!= -0x1p-104L) return 11;
    if(fetestexcept(FE_ALL_EXCEPT)) return 12;
    if(remainder(5.5,2.0)!= -0.5 || remquo(5.5,2.0,&q)!= -0.5 || q!=3) return 13;
    if(remquof(-5.5f,2.0f,&q)!=0.5f || q!= -3 || remainderl(5.5L,2.0L)!= -0.5L) return 14;
    if(nextafter(1.0,2.0)!=0x1.0000000000001p0 || nextafterf(1.0f,2.0f)!=0x1.000002p0f) return 15;
    if(nexttowardf(1.0f,0x1.0000000000001p0L)!=0x1.000002p0f || nexttowardl(1.0L,2.0L)!=0x1.0000000000001p0L) return 16;
    if(ilogb(0x1p-1074)!= -1074 || ilogbf(0x1p-149f)!= -149 || logbl(8.0L)!=3.0L) return 17;
    if(modf(-1.75,&ip)!= -0.75 || ip!= -1.0 || modff(2.5f,&fi)!=0.5f || fi!=2.0f || modfl(3.25L,&li)!=0.25L || li!=3.0L) return 18;
    if(exp2(10.0)!=1024.0 || exp2f(-149.0f)!=0x1p-149f || log2(0x1p-1074)!= -1074.0 || log2l(1024.0L)!=10.0L) return 19;
    if(erf(0.0)!=0.0 || erfcf(0.0f)!=1.0f || erfcl(0.0L)!=1.0L) return 20;
    if(lgamma(1.0)!=0.0 || tgamma(1.0)!=1.0 || fabs(tgamma(4.0)-6.0)>0x1p-40 || fabs(tgammal(0.5L)-1.772453850905516)>0x1p-40) return 21;
    if(acosh(1.0)!=0.0 || asinhf(0.0f)!=0.0f || atanhl(0.0L)!=0.0L) return 22;
    if(fdim(2.0,1.0)!=1.0 || fdimf(1.0f,2.0f)!=0.0f || fmaxl(1.0L,2.0L)!=2.0L || fmin(1.0,2.0)!=1.0) return 23;
    if(signbit(fmax(-0.0,0.0)) || !signbit(fmin(-0.0,0.0))) return 24;
    if(scalbln(1.5,3)!=12.0 || scalblnf(1.0f,-149)!=0x1p-149f || scalblnl(1.0L,-1074)!=0x1p-1074L) return 25;
    if(!isnan(nan("")) || !isnan(nanf("payload")) || !isnan(nanl(""))) return 26;
    feclearexcept(FE_ALL_EXCEPT);d=nextafter(0.0,1.0);
    if(d!=0x1p-1074 || fetestexcept(FE_ALL_EXCEPT)!=(FE_UNDERFLOW|FE_INEXACT)) return 27;
    feclearexcept(FE_ALL_EXCEPT);d=tgamma(-2.0);
    if(!isnan(d)||fetestexcept(FE_ALL_EXCEPT)!=FE_INVALID) return 28;
    feclearexcept(FE_ALL_EXCEPT);e=ilogb(0.0);
    if(e!=FP_ILOGB0 || fetestexcept(FE_ALL_EXCEPT)!=FE_INVALID) return 29;
    return 0;
}
