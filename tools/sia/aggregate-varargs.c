#include <stdarg.h>
struct Small { int x; unsigned char c; };
struct Large { long long x; int a[5]; };
static struct Small small(struct Small x) { x.x += 7; x.c += 1; return x; }
static struct Large large(int a,int b,int c,int d,int e,int f,int g,struct Large x) {
    x.x += a+b+c+d+e+f+g; x.a[3] += 9; return x;
}
static long long consume(va_list *ap) {
    long long sum=0; int i;
    for(i=0;i<8;i++) sum += va_arg(*ap,int);
    sum += va_arg(*ap,long long);
    return sum;
}
static long long variadic(int a,int b,int c,int d,int e,int f,int g,...) {
    va_list ap,copy; long long first,second; struct Small x;
    va_start(ap,g); va_copy(copy,ap);
    first=consume(&ap); second=consume(&copy);
    if(first!=second) return -1;
    x=va_arg(ap,struct Small);
    va_end(ap); va_end(copy);
    return first+x.x+x.c+a+b+c+d+e+f+g;
}
int main(void) {
    struct Small s={11,5},t; struct Large l={100,{1,2,3,4,5}},r;
    struct Small (*sp)(struct Small)=small;
    struct Large (*lp)(int,int,int,int,int,int,int,struct Large)=large;
    long long (*vp)(int,int,int,int,int,int,int,...)=variadic;
    t=sp(s); if(t.x!=18||t.c!=6||s.x!=11||s.c!=5)return 1;
    r=lp(1,2,3,4,5,6,7,l);
    if(r.x!=128||r.a[3]!=13||l.x!=100||l.a[3]!=4)return 2;
    if(vp(1,2,3,4,5,6,7,1,2,3,4,5,6,7,8,1000LL,s)!=1080)return 3;
    if(variadic(1,2,3,4,5,6,7,1,2,3,4,5,6,7,8,1000LL,s)!=1080)return 4;
    return 0;
}
