#include <stdarg.h>
struct Big { long x[4]; double f; unsigned char tail; };
extern struct Big host_return(struct Big,int);
struct Big cosmic_return(struct Big a,int n) { a.x[0]+=n;a.f*=2;a.tail+=1;return a; }
struct Big cosmic_pressure(long a,long b,long c,long d,long e,long f,long g,struct Big x) {
 x.x[1]+=a+b+c+d+e+f+g;return x;
}
struct Big cosmic_variadic(int n,...) {
 va_list ap;struct Big x;long value;va_start(ap,n);value=va_arg(ap,long);x=va_arg(ap,struct Big);va_end(ap);
 x.x[2]+=value+n;return x;
}
struct Big cosmic_callback(struct Big x,struct Big(*cb)(struct Big,int)) {
 struct Big y=cb(x,3);return host_return(y,5);
}
