#include <stdarg.h>
#include <stdio.h>
extern long cosmic_consume(va_list);
extern long cosmic_incoming(int,...);
extern long cosmic_named(long,long,long,long,long,long,long,long,double,double,double,double,double,double,double,double,double,...);
long host_consume(va_list ap) {long sum=0;int i;for(i=0;i<8;i++)sum+=va_arg(ap,long);for(i=0;i<10;i++)sum+=(long)va_arg(ap,double);return sum;}
static long host_incoming(int tag,...) {va_list ap;long r;va_start(ap,tag);r=cosmic_consume(ap);va_end(ap);return r+tag;}
#define VALUES 1L,2L,3L,4L,5L,6L,7L,8L,1.0,2.0,3.0,4.0,5.0,6.0,7.0,8.0,9.0,10.0
struct Big { long a,b,c; unsigned char tag; };
extern long cosmic_memory(struct Big,int,...);
extern long cosmic_scalars(int,...);
int main(void){
 struct Big a={1,2,3,4},b={10,20,30,5};
 signed char small=-127;unsigned short u=65535;float f=1.5f;
 if(cosmic_incoming(7,VALUES)!=98)return 1;
 if(host_incoming(7,VALUES)!=98)return 2;
 if(cosmic_named(1,2,3,4,5,6,7,8,1.,2.,3.,4.,5.,6.,7.,8.,9.,100L,200.)!=346)return 3;
 if(cosmic_memory(a,2,b,100L)!=180 || b.a!=10)return 4;
 if(cosmic_scalars(9,small,u,0x8000000000000001UL,"x",f)!=9)return 5;
 puts("stdarg-pass");return 0;
}
