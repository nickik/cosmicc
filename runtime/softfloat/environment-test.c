/* Real C operations must observe persistent, task-bound software environment. */
#include <fenv.h>
#include <math.h>
#include <float.h>
extern void cosmic_sf_init(fenv_t *);
extern fenv_t *cosmic_sf_bind_context(fenv_t *);
int main(void) {
    fenv_t first,second,saved;fexcept_t flag;
    volatile double one=1.0,half=0x1p-53,zero=0.0;
    volatile float fone=1.0f,fhalf=0x1p-24f;
    double d;float f;
    cosmic_sf_init(&first);cosmic_sf_init(&second);cosmic_sf_bind_context(&first);
    if(fegetround()!=FE_TONEAREST || FLT_ROUNDS!=1) return 1;
    if(fesetround(FE_UPWARD) || FLT_ROUNDS!=2) return 2;
    d=one+half;f=fone+fhalf;
    if(d!=0x1.0000000000001p0 || f!=0x1.000002p0f || !(fetestexcept(FE_INEXACT))) return 3;
    if(fegetexceptflag(&flag,FE_INEXACT) || flag!=FE_INEXACT) return 4;
    if(feholdexcept(&saved) || fetestexcept(FE_ALL_EXCEPT)) return 5;
    d=one/zero;
    if(!isinf(d) || fetestexcept(FE_ALL_EXCEPT)!=FE_DIVBYZERO) return 6;
    if(feupdateenv(&saved) || fetestexcept(FE_ALL_EXCEPT)!=(FE_INEXACT|FE_DIVBYZERO)) return 7;
    cosmic_sf_bind_context(&second);
    if(fegetround()!=FE_TONEAREST || fetestexcept(FE_ALL_EXCEPT)) return 8;
    if(fesetround(FE_DOWNWARD)) return 9;
    d= -one-half; if(d!= -0x1.0000000000001p0) return 10;
    cosmic_sf_bind_context(&first);
    if(fegetround()!=FE_UPWARD || fetestexcept(FE_ALL_EXCEPT)!=(FE_INEXACT|FE_DIVBYZERO)) return 11;
    if(feclearexcept(FE_ALL_EXCEPT) || fesetround(FE_TONEAREST)) return 12;
    if(sqrt(4.0)!=2.0 || sqrtf(9.0f)!=3.0f || sqrtl(16.0L)!=4.0L) return 13;
    if(fmod(5.5,2.0)!=1.5 || fmodf(-5.5f,2.0f)!= -1.5f) return 14;
    if(floor(-1.25)!= -2.0 || ceil(-1.25)!= -1.0 || trunc(-1.75)!= -1.0 || round(-1.5)!= -2.0) return 15;
    if(fabs(-0.0)!=0.0 || signbit(fabs(-0.0)) || !signbit(copysign(0.0,-1.0))) return 16;
    if(scalbn(0x1p-1022,-1)!=0x1p-1023 || ldexp(1.5,3)!=12.0) return 17;
    { int e=0;if(frexp(12.0,&e)!=0.75 || e!=4) return 18; }
    if(fpclassify(0x1p-1074)!=FP_SUBNORMAL || !isnormal(1.0) || !isfinite(0.0)) return 19;
    if(fesetround(99)==0 || fegetround()!=FE_TONEAREST) return 20;
    if(fesetenv(FE_DFL_ENV) || fetestexcept(FE_ALL_EXCEPT) || fegetround()!=FE_TONEAREST) return 21;
    cosmic_sf_bind_context((fenv_t *)0);
    return 0;
}
