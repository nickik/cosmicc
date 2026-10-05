#include <stdarg.h>
struct C { unsigned char a[3]; };
struct I { long a,b; };
struct F { float a; };
struct FF { float a,b,c,d; };
struct D { double a,b; };
struct M { double a; long b; };
struct R { long a; double b; };
struct N { short tag; float a[2]; };
union U { double d; unsigned long i; };
struct C cosmic_c(struct C x){x.a[2]+=7;return x;}
struct I cosmic_i(struct I x){x.a+=3;x.b-=2;return x;}
struct F cosmic_f(struct F x){x.a+=0.5f;return x;}
struct FF cosmic_ff(struct FF x){x.a+=1;x.d*=2;return x;}
struct D cosmic_d(struct D x){x.a+=1;x.b*=2;return x;}
struct M cosmic_m(struct M x){x.a+=1;x.b+=3;return x;}
struct R cosmic_r(struct R x){x.a+=5;x.b*=3;return x;}
struct N cosmic_n(struct N x){x.tag+=2;x.a[0]+=1;x.a[1]*=2;return x;}
union U cosmic_u(union U x){x.i^=0x8000000000000000UL;return x;}
long cosmic_gp(long a,long b,long c,long d,long e,struct I x,long tail){return a+b+c+d+e+x.a+x.b+tail;}
double cosmic_fp(double a,double b,double c,double d,double e,double f,double g,struct D x,double tail){return a+b+c+d+e+f+g+x.a+x.b+tail;}
long cosmic_mixed(double a,double b,double c,double d,double e,double f,double g,double h,struct M x,long tail){return (long)(a+b+c+d+e+f+g+h+x.a)+x.b+tail;}
extern struct M host_m(struct M);
struct M cosmic_callback(struct M x,struct M(*p)(struct M)){return host_m(p(x));}
long cosmic_va(int n,...){va_list ap,cp;struct I i;struct M m;struct D d;long result;va_start(ap,n);va_copy(cp,ap);i=va_arg(ap,struct I);m=va_arg(ap,struct M);d=va_arg(ap,struct D);result=i.a+i.b+(long)m.a+m.b+(long)d.a+(long)d.b;va_end(cp);va_end(ap);return result+n;}
double cosmic_va_spill(int n,...){va_list ap;int i;double total=0;struct D d;va_start(ap,n);for(i=0;i<7;i++)total+=va_arg(ap,double);d=va_arg(ap,struct D);total+=d.a+d.b+va_arg(ap,double);va_end(ap);return total+n;}
extern long host_va(int,...);
long cosmic_send(struct I i,struct M m,struct D d){return host_va(1,i,m,d);}
long cosmic_va_gp(int n,...){va_list ap;int j;long total=0;struct I i;struct M m;va_start(ap,n);for(j=0;j<4;j++)total+=va_arg(ap,long);i=va_arg(ap,struct I);total+=i.a+i.b+va_arg(ap,long);m=va_arg(ap,struct M);total+=(long)m.a+m.b+(long)va_arg(ap,double);va_end(ap);return total+n;}
