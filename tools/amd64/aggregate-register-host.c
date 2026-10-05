#include <stdio.h>
#include <stdarg.h>
struct C { unsigned char a[3]; };struct I {long a,b;};struct F{float a;};struct FF{float a,b,c,d;};struct D{double a,b;};struct M{double a;long b;};struct R{long a;double b;};struct N{short tag;float a[2];};union U{double d;unsigned long i;};
extern struct C cosmic_c(struct C);extern struct I cosmic_i(struct I);extern struct F cosmic_f(struct F);extern struct FF cosmic_ff(struct FF);extern struct D cosmic_d(struct D);extern struct M cosmic_m(struct M);extern struct R cosmic_r(struct R);extern struct N cosmic_n(struct N);extern union U cosmic_u(union U);
extern long cosmic_gp(long,long,long,long,long,struct I,long);extern double cosmic_fp(double,double,double,double,double,double,double,struct D,double);extern long cosmic_mixed(double,double,double,double,double,double,double,double,struct M,long);
extern struct M cosmic_callback(struct M,struct M(*)(struct M));extern long cosmic_va(int,...);extern double cosmic_va_spill(int,...);
struct M host_m(struct M x){x.a*=2;x.b+=10;return x;}
extern long cosmic_send(struct I,struct M,struct D);
extern long cosmic_va_gp(int,...);
long host_va(int n,...){va_list ap;struct I i;struct M m;struct D d;va_start(ap,n);i=va_arg(ap,struct I);m=va_arg(ap,struct M);d=va_arg(ap,struct D);va_end(ap);return i.a+i.b+(long)m.a+m.b+(long)d.a+(long)d.b+n;}
int main(void){
 struct C c={{1,2,3}};struct I i={10,20};struct F f={1.5f};struct FF ff={1,2,3,4};struct D d={10,20};struct M m={10,20};struct R r={10,2};struct N n={5,{2,3}};union U u;u.i=123;
 c=cosmic_c(c);if(c.a[0]!=1||c.a[2]!=10)return 1;
 i=cosmic_i(i);if(i.a!=13||i.b!=18)return 2;
 f=cosmic_f(f);if(f.a!=2)return 3;
 ff=cosmic_ff(ff);if(ff.a!=2||ff.b!=2||ff.c!=3||ff.d!=8)return 4;
 d=cosmic_d(d);if(d.a!=11||d.b!=40)return 5;
 m=cosmic_m(m);if(m.a!=11||m.b!=23)return 6;
 r=cosmic_r(r);if(r.a!=15||r.b!=6)return 7;
 n=cosmic_n(n);if(n.tag!=7||n.a[0]!=3||n.a[1]!=6)return 8;
 u=cosmic_u(u);if(u.i!=0x800000000000007BUL)return 9;
 if(cosmic_gp(1,2,3,4,5,i,100)!=146)return 10;
 if(cosmic_fp(1,2,3,4,5,6,7,d,100)!=179)return 11;
 if(cosmic_mixed(1,2,3,4,5,6,7,8,m,100)!=170)return 12;
 m=cosmic_callback(m,cosmic_m);if(m.a!=24||m.b!=36)return 13;
 if(cosmic_va(1,i,m,d)!=143)return 14;
 if(cosmic_va_spill(1,1.,2.,3.,4.,5.,6.,7.,d,100.)!=180)return 15;
 if(cosmic_send(i,m,d)!=143)return 16;
 if(cosmic_va_gp(1,1L,2L,3L,4L,i,100L,m,200.)!=402)return 17;
 puts("aggregate-register-pass");return 0;
}
