#include <stdlib.h>
#include <stdio.h>
#include <fenv.h>
static int compare(const char *a,const char *b){while(*a&&*a==*b){++a;++b;}return (unsigned char)*a-(unsigned char)*b;}
union Bits { double f; unsigned long long u; };
union Small { float f; unsigned int u; };
int main(void){
    char text[128],*end;union Bits d;union Small f;int n,wrote;
    d.f=strtod("  -12.5junk",&end);if(d.f!=-12.5||*end!='j')return 1;
    f.f=strtof("1.0000000596046447753906250000000001",&end);if(f.u!=0x3f800001u||*end)return 2;
    d.f=strtod("0x1p-1074",&end);if(d.u!=1||*end)return 3;
    fesetround(FE_UPWARD);d.f=strtod("1.00000000000000011102230246251565404236316680908203125",0);if(d.u!=0x3ff0000000000001ULL)return 4;
    fesetround(FE_TONEAREST);
    if(strtold("1.5",0)!=1.5L||atof("2.5")!=2.5)return 5;
    n=snprintf(text,sizeof text,"%+09.2f|%.3e|%.4g|%a",-12.5,1.25,12345.0,1.5);
    if(n!=38||compare(text,"-00012.50|1.250e+00|1.234e+04|0x1.8p+0"))return 6;
    if(snprintf(text,5,"hello%u",123u)!=8||compare(text,"hell"))return 7;
    if(snprintf(0,0,"%.1000f",1.0)!=1002)return 8;
    if(sprintf(text,"%#08x %lld%n",42u,-4294967297LL,&wrote)!=20||wrote!=20||compare(text,"0x00002a -4294967297"))return 9;
    fesetround(FE_DOWNWARD);if(snprintf(text,sizeof text,"%.0f %.0f",2.5,-2.5)!=4||compare(text,"2 -3"))return 10;
    fesetround(FE_TONEAREST);return 0;
}
