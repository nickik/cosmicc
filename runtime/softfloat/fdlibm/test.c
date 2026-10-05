#include "../math.h"
#include "../context.h"
#include "fdlibm-vectors.h"
static double call(unsigned int op,double a,double b) {
    switch(op) {
    case 0:return exp(a);case 1:return log(a);case 2:return log10(a);
    case 3:return sin(a);case 4:return cos(a);case 5:return tan(a);
    case 6:return asin(a);case 7:return acos(a);case 8:return atan(a);
    case 9:return sinh(a);case 10:return cosh(a);case 11:return tanh(a);
    case 12:return expm1(a);case 13:return log1p(a);case 14:return cbrt(a);
    case 15:return pow(a,b);case 16:return atan2(a,b);case 17:return hypot(a,b);
    case 18:return erf(a);case 19:return erfc(a);case 20:return lgamma(a);
    case 21:return acosh(a);case 22:return asinh(a);case 23:return atanh(a);default:return remainder(a,b);
    }
    return 0.0;
}
static int nanbits(unsigned long long a) { return (a&0x7fffffffffffffffULL)>0x7ff0000000000000ULL; }
int main(void) {
    unsigned int i;cosmic_sf_context c;
    cosmic_sf_init(&c);cosmic_sf_bind_context(&c);
    for(i=0;i<FD_VECTOR_COUNT;i++) {
        union { double f;unsigned long long u; } a,b,z;
        a.u=fd_vectors[i].a;b.u=fd_vectors[i].b;c.flags=0;
        z.f=call(fd_vectors[i].op,a.f,b.f);
        if(z.u!=fd_vectors[i].result && !(nanbits(z.u)&&nanbits(fd_vectors[i].result))) return (int)i+1;
        /* Target cast/conversion helpers record inexact; test mandatory special flags. */
        if((c.flags&30u)!=(fd_vectors[i].flags&30u)) return (int)i+10000;
    }
    cosmic_sf_bind_context((cosmic_sf_context *)0);return 0;
}
