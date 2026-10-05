#include <fenv.h>
#pragma STDC FENV_ACCESS ON
/* Automatic runtime expressions must observe the dynamic environment even
   when their operands are constants. Static initializers are separate. */
int main(void){
 fesetround(FE_UPWARD);feclearexcept(FE_ALL_EXCEPT);
 double value=1.0+0x1p-53;
 if(value!=0x1.0000000000001p0)return 1;
 if(!(fetestexcept(FE_INEXACT)&FE_INEXACT))return 2;
 feclearexcept(FE_ALL_EXCEPT);
 double nan=0.0/0.0;
 if(nan==nan || !(fetestexcept(FE_INVALID)&FE_INVALID))return 3;
 fesetround(FE_TONEAREST);feclearexcept(FE_ALL_EXCEPT);return 0;
}
