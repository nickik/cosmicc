#include <stdarg.h>
#include <stdio.h>
struct M { long a; double d; long v[2]; };
struct Odd { unsigned char b[17]; };
long host_take(int tag,struct M m,long x,double d,struct Odd odd,struct M n,long y) {
    if(tag!=13 || m.a!=7 || m.d!=2.5 || m.v[0]!=11 || m.v[1]!=19 || x!=-4294967297L || d!=4.5 || odd.b[0]!=3 || odd.b[16]!=251 || n.a!=17 || n.v[1]!=23 || y!=29) return 1;
    { volatile long *p=&m.a; volatile unsigned char *q=&odd.b[16]; *p=99; *q=0; }
    m.v[1]=101; n.a=33;
    return 0;
}
static volatile unsigned char aggregate_al __attribute__((used))=255;
__asm__(".text\n.globl host_va\nhost_va:\nmovb %al, aggregate_al(%rip)\njmp host_va_impl\n");
long host_va_impl(double fixed,int tag,...) {
    va_list ap; struct M m,n; struct Odd odd; long x; double d; int y;
    if(fixed!=1.25 || tag!=13 || aggregate_al!=2) return 1;
    va_start(ap,tag);
    m=va_arg(ap,struct M); x=va_arg(ap,long); odd=va_arg(ap,struct Odd);
    d=va_arg(ap,double); n=va_arg(ap,struct M); y=va_arg(ap,int);
    va_end(ap);
    return host_take(tag,m,x,d,odd,n,y);
}
extern long cosmic_take(int,struct M,double,struct Odd,struct M,long,struct M *);
extern long cosmic_calls(long (*)(int,struct M,long,double,struct Odd,struct M,long));
int main(void) {
    struct M m={7,2.5,{11,19}}, n={17,3.5,{21,23}};
    struct Odd odd={{3}}; long r;
    odd.b[16]=251;
    r=cosmic_take(13,m,4.5,odd,n,-4294967297L,&m);
    if(r || m.a!=7 || m.v[1]!=19 || odd.b[16]!=251 || n.a!=17) return 20+(int)r;
    r=cosmic_calls(host_take);
    if(r) fprintf(stderr,"MEMORY aggregate ABI failure %ld AL=%u\n",r,aggregate_al);
    return (int)r;
}
