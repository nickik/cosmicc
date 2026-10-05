#include <float.h>
#include <fenv.h>
#pragma STDC FENV_ACCESS ON
long double extended_global=1.5L;
long double extended(long double x){return x+extended_global;}
int main(void){
#if LDBL_MANT_DIG == 53
 if(sizeof(long double)!=8 || LDBL_DIG!=DBL_DIG)return 1;
#endif
 long double (*fp)(long double)=extended;
 long double a=2.5L;
 if(fp(a)!=4.0L || (double)a!=2.5 || (float)a!=2.5f || (long long)a!=2LL)return 2;
 long double old=a++;if(old!=2.5L || a!=3.5L)return 3;
 if(fesetround(FE_TONEAREST) || feclearexcept(FE_ALL_EXCEPT))return 4;
 volatile double one=1.0, half=0x1p-53;
 if(fesetround(FE_UPWARD))return 5;
 double up=one+half;
 if(up!=0x1.0000000000001p0 || !(fetestexcept(FE_INEXACT)&FE_INEXACT))return 6;
#ifdef COSMIC_SIA_FLOAT_H
 if(FLT_ROUNDS!=2)return 7;
#endif
 if(fesetround(FE_DOWNWARD) || feclearexcept(FE_ALL_EXCEPT))return 8;
 double down=one+half;if(down!=1.0)return 9;
#ifdef COSMIC_SIA_FLOAT_H
 if(FLT_ROUNDS!=3)return 9;
#endif
 volatile unsigned integer=16777217U;
 float low=(float)integer;if(low!=16777216.0f)return 10;
 if(fesetround(FE_UPWARD))return 11;
 float high=(float)integer;if(high!=16777218.0f)return 12;
 volatile double positive=1.9; if((int)positive!=1)return 13;
 if(fesetround(FE_TONEAREST) || feclearexcept(FE_ALL_EXCEPT))return 14;
 volatile double zero=0.0;
 double nan=zero/zero;if(nan==nan || !(fetestexcept(FE_INVALID)&FE_INVALID))return 15;
 double infinity=one/zero;if(!(infinity>1.0e300) || !(fetestexcept(FE_DIVBYZERO)&FE_DIVBYZERO))return 16;
 if((fetestexcept(FE_ALL_EXCEPT)&(FE_INVALID|FE_DIVBYZERO))!=(FE_INVALID|FE_DIVBYZERO))return 17;
 fenv_t saved;if(feholdexcept(&saved) || fetestexcept(FE_ALL_EXCEPT))return 18;
 if(feraiseexcept(FE_INEXACT) || feupdateenv(&saved))return 19;
 if((fetestexcept(FE_ALL_EXCEPT)&(FE_INVALID|FE_DIVBYZERO|FE_INEXACT))!=(FE_INVALID|FE_DIVBYZERO|FE_INEXACT))return 20;
#ifdef COSMIC_SOFTFLOAT_CONTEXT_H
 cosmic_sf_context task;cosmic_sf_init(&task);
 cosmic_sf_context *previous=cosmic_sf_bind_context(&task);
 double task_nan=zero/zero;if(task_nan==task_nan || !(task.flags&FE_INVALID))return 21;
 cosmic_sf_bind_context(previous);
#endif
 fesetround(FE_TONEAREST);feclearexcept(FE_ALL_EXCEPT);
 return 0;
}
