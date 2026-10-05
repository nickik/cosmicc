/* Integer-only exact bit operations and remainder reduction. */
#include "binary32.h"
static unsigned long long sf_mask(unsigned int p) { return (1ULL<<p)-1ULL; }
static unsigned long long sf_quiet(cosmic_sf_context *c,unsigned long long x,unsigned int p) {
    unsigned long long q=1ULL<<(p-1);
    if(!(x&q)) cosmic_sf_raise(c,COSMIC_SF_INVALID);
    return x|q;
}
static unsigned long long sf_integral(cosmic_sf_context *c,unsigned long long x,unsigned int p,unsigned int eb,int how) {
    unsigned int sign=(unsigned int)(x>>(p+eb));
    unsigned int exp=(unsigned int)((x>>p)&((1u<<eb)-1u));
    int bias=(1<<(eb-1))-1,e=(int)exp-bias;
    unsigned long long frac=x&sf_mask(p),mask,half,unit,mag=x&((1ULL<<(p+eb))-1ULL);
    if(exp==((1u<<eb)-1u)) return frac ? sf_quiet(c,x,p) : x;
    if(e>=(int)p || !mag) return x;
    if(e<0) {
        if((how==2 && !sign) || (how==1 && sign) || (how==3 && e==-1))
            return ((unsigned long long)sign<<(p+eb))|((unsigned long long)bias<<p);
        return (unsigned long long)sign<<(p+eb);
    }
    unit=1ULL<<(p-(unsigned int)e); mask=unit-1ULL;half=unit>>1;
    frac=x&mask;mag=x&~mask;
    if(frac && ((how==2 && !sign)||(how==1 && sign)||(how==3 && frac>=half))) mag+=unit;
    return mag;
}
static unsigned long long sf_mod(cosmic_sf_context *c,unsigned long long x,unsigned long long y,unsigned int p,unsigned int eb) {
    unsigned long long sign=x&(1ULL<<(p+eb)),top=1ULL<<p;
    unsigned long long mask=(1ULL<<(p+eb))-1ULL,fx=x&sf_mask(p),fy=y&sf_mask(p),a=x&mask,b=y&mask;
    int ex=(int)(a>>p),ey=(int)(b>>p),max=(1<<eb)-1;
    if((ex==max && fx) || (ey==max && fy))
        return p==23 ? cosmic_sf_add32(c,(unsigned int)x,(unsigned int)y) : cosmic_sf_add64(c,x,y);
    if(!b || ex==max) { cosmic_sf_raise(c,COSMIC_SF_INVALID); return ((unsigned long long)max<<p)|(top>>1); }
    if(a<b) return x;
    if(a==b) return sign;
    if(!ex) { ex=1;while(!(a&top)) { a<<=1;ex--; } } else a=fx|top;
    if(!ey) { ey=1;while(!(b&top)) { b<<=1;ey--; } } else b=fy|top;
    while(ex>ey) { if(a>=b) a-=b; if(!a) return sign; a<<=1;ex--; }
    if(a>=b) a-=b;
    if(!a) return sign;
    while(!(a&top)) { a<<=1;ex--; }
    if(ex>0) a=(a-top)|((unsigned long long)ex<<p);
    else a>>=(1-ex);
    return a|sign;
}
unsigned int cosmic_sf_fmod32(cosmic_sf_context *c,unsigned int a,unsigned int b) { return (unsigned int)sf_mod(c,a,b,23,8); }
unsigned long long cosmic_sf_fmod64(cosmic_sf_context *c,unsigned long long a,unsigned long long b) { return sf_mod(c,a,b,52,11); }
#define SF_INT32(name,mode) unsigned int cosmic_sf_##name##32(cosmic_sf_context *c,unsigned int a) { return (unsigned int)sf_integral(c,a,23,8,mode); }
#define SF_INT64(name,mode) unsigned long long cosmic_sf_##name##64(cosmic_sf_context *c,unsigned long long a) { return sf_integral(c,a,52,11,mode); }
SF_INT32(trunc,0) SF_INT32(floor,1) SF_INT32(ceil,2) SF_INT32(round,3)
SF_INT64(trunc,0) SF_INT64(floor,1) SF_INT64(ceil,2) SF_INT64(round,3)
unsigned int cosmic_sf_fabs32(cosmic_sf_context *c,unsigned int a) { (void)c; return a&0x7fffffffu; }
unsigned long long cosmic_sf_fabs64(cosmic_sf_context *c,unsigned long long a) { (void)c;return a&0x7fffffffffffffffULL; }
unsigned int cosmic_sf_copysign32(cosmic_sf_context *c,unsigned int a,unsigned int b) { (void)c;return (a&0x7fffffffu)|(b&0x80000000u); }
unsigned long long cosmic_sf_copysign64(cosmic_sf_context *c,unsigned long long a,unsigned long long b) { (void)c;return (a&0x7fffffffffffffffULL)|(b&0x8000000000000000ULL); }
static unsigned long long sf_frexp(cosmic_sf_context *c,unsigned long long a,int *e,unsigned int p,unsigned int eb) {
    unsigned int exp=(unsigned int)((a>>p)&((1u<<eb)-1u));
    unsigned long long sig=a&sf_mask(p),sign=a&(1ULL<<(p+eb));
    int bias=(1<<(eb-1))-1,ee=(int)exp;
    *e=0;
    if(exp==((1u<<eb)-1u)) return sig ? sf_quiet(c,a,p) : a;
    if(!exp) { if(!sig) return a;ee=1;while(!(sig&(1ULL<<p))) { sig<<=1;ee--; } }
    *e=ee-bias+1;
    return sign|((unsigned long long)(bias-1)<<p)|(sig&sf_mask(p));
}
unsigned int cosmic_sf_frexp32(cosmic_sf_context *c,unsigned int a,int *e) { return (unsigned int)sf_frexp(c,a,e,23,8); }
unsigned long long cosmic_sf_frexp64(cosmic_sf_context *c,unsigned long long a,int *e) { return sf_frexp(c,a,e,52,11); }
static void sf_unpack(unsigned long long a,unsigned int p,unsigned int eb,int *e,unsigned long long *sig) {
    *e=(int)((a>>p)&((1u<<eb)-1u));*sig=a&sf_mask(p);
    if(!*e) { *e=1;while(!(*sig&(1ULL<<p))) { *sig<<=1;--*e; } }
    else *sig|=1ULL<<p;
}
static unsigned long long sf_remquo(cosmic_sf_context *c,unsigned long long x,unsigned long long y,int *quo,unsigned int p,unsigned int eb) {
    unsigned long long signbit=1ULL<<(p+eb),a=x&(signbit-1ULL),b=y&(signbit-1ULL),r,sx,sy,sr;
    unsigned int q=0;int ex,ey,er,greater=0,tie=0;*quo=0;
    r=sf_mod(c,x,y,p,eb);
    if(!b || a>=((unsigned long long)((1u<<eb)-1u)<<p) || b>((unsigned long long)((1u<<eb)-1u)<<p)) return r;
    if(b==((unsigned long long)((1u<<eb)-1u)<<p) || !a) return r;
    sf_unpack(a,p,eb,&ex,&sx);sf_unpack(b,p,eb,&ey,&sy);
    if(ex>=ey) {
        while(ex>ey) { q=(q<<1)&127u;if(sx>=sy) { sx-=sy;q|=1u; }sx<<=1;ex--; }
        q=(q<<1)&127u;if(sx>=sy) q|=1u;
    }
    if(r&(signbit-1ULL)) {
        sf_unpack(r&(signbit-1ULL),p,eb,&er,&sr);
        if(er+1>ey) greater=1;
        else if(er+1==ey) { greater=sr>sy;tie=sr==sy; }
        if(greater || (tie&&(q&1u))) {
            q=(q+1u)&127u;
            if(p==23) r=cosmic_sf_sub32(c,(unsigned int)r,(unsigned int)(b|(x&signbit)));
            else r=cosmic_sf_sub64(c,r,b|(x&signbit));
        }
    }
    *quo=((x^y)&signbit) ? -(int)q : (int)q;return r;
}
unsigned int cosmic_sf_remquo32(cosmic_sf_context *c,unsigned int a,unsigned int b,int *q) { return (unsigned int)sf_remquo(c,a,b,q,23,8); }
unsigned long long cosmic_sf_remquo64(cosmic_sf_context *c,unsigned long long a,unsigned long long b,int *q) { return sf_remquo(c,a,b,q,52,11); }
static unsigned long long sf_next(cosmic_sf_context *c,unsigned long long a,unsigned long long b,unsigned int p,unsigned int eb) {
    unsigned long long sign=1ULL<<(p+eb),mask=sign-1ULL,inf=(unsigned long long)((1u<<eb)-1u)<<p,r;
    int less;
    if((a&mask)>inf || (b&mask)>inf) return p==23 ? cosmic_sf_add32(c,(unsigned int)a,(unsigned int)b) : cosmic_sf_add64(c,a,b);
    if(a==b || !((a|b)&mask)) return b;
    less=p==23 ? cosmic_sf_lt32(c,(unsigned int)a,(unsigned int)b) : cosmic_sf_lt64(c,a,b);
    if(!(a&mask)) r=(b&sign)|1ULL;
    else if(less==!(a&sign)) r=a+1ULL;
    else r=a-1ULL;
    if((r&mask)==inf && (a&mask)!=inf) cosmic_sf_raise(c,COSMIC_SF_OVERFLOW|COSMIC_SF_INEXACT);
    else if((r&mask)<(1ULL<<p)) cosmic_sf_raise(c,COSMIC_SF_UNDERFLOW|COSMIC_SF_INEXACT);
    return r;
}
unsigned int cosmic_sf_nextafter32(cosmic_sf_context *c,unsigned int a,unsigned int b) { return (unsigned int)sf_next(c,a,b,23,8); }
unsigned long long cosmic_sf_nextafter64(cosmic_sf_context *c,unsigned long long a,unsigned long long b) { return sf_next(c,a,b,52,11); }
unsigned int cosmic_sf_nexttoward32(cosmic_sf_context *c,unsigned int a,unsigned long long b) {
    unsigned long long wide=cosmic_sf_f32_to_f64(c,a);
    if((wide&0x7fffffffffffffffULL)>0x7ff0000000000000ULL || (b&0x7fffffffffffffffULL)>0x7ff0000000000000ULL)
        return cosmic_sf_f64_to_f32(c,cosmic_sf_add64(c,wide,b));
    if(cosmic_sf_eq64(c,wide,b)) return (a&0x7fffffffu)|(unsigned int)(b>>32)&0x80000000u;
    return cosmic_sf_nextafter32(c,a,cosmic_sf_lt64(c,wide,b) ? 0x7f800000u : 0xff800000u);
}
