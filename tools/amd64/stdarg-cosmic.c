#include <stdarg.h>
long cosmic_consume(va_list ap) {
 long sum=0;int i;for(i=0;i<8;i++)sum+=va_arg(ap,long);
 for(i=0;i<10;i++)sum+=(long)va_arg(ap,double);
 return sum;
}
extern long host_consume(va_list ap);
long cosmic_incoming(int tag,...) {
 va_list ap,cp;long a,b,c;va_start(ap,tag);va_copy(cp,ap);
 a=cosmic_consume(ap);b=host_consume(cp);va_end(cp);va_end(ap);
 return a==b?a+tag:-1;
}
long cosmic_named(long a,long b,long c,long d,long e,long f,long g,long h,
 double x0,double x1,double x2,double x3,double x4,double x5,double x6,double x7,double x8,...) {
 va_list ap;long result;va_start(ap,x8);result=va_arg(ap,long)+(long)va_arg(ap,double);va_end(ap);
 return result+a+b+c+d+e+f+g+h+(long)x0+(long)x8;
}
struct Big { long a,b,c; unsigned char tag; };
long cosmic_memory(struct Big named,int n,...) {
 va_list ap;struct Big extra;long tail;va_start(ap,n);
 extra=va_arg(ap,struct Big);tail=va_arg(ap,long);va_end(ap);
 extra.a+=7;return named.a+named.b+named.c+extra.a+extra.b+extra.c+extra.tag+tail+n;
}
long cosmic_scalars(int n,...) {
 va_list ap;int a,b;unsigned long bits;char *p;double d;va_start(ap,n);
 a=va_arg(ap,int);b=va_arg(ap,int);bits=va_arg(ap,unsigned long);p=va_arg(ap,char*);d=va_arg(ap,double);va_end(ap);
 return (a==-127 && b==65535 && bits==0x8000000000000001UL && p[0]=='x' && d==1.5)?n:0;
}
