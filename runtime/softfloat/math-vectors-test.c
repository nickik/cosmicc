#include "binary32.h"
#include "math-vectors.h"
static unsigned long long mathcall(cosmic_sf_context *c,unsigned int width,unsigned int op,unsigned long long a,unsigned long long b,int n,int *e) {
    if(width==32) {
        switch(op) {
        case 0:return cosmic_sf_fmod32(c,(unsigned int)a,(unsigned int)b);
        case 1:return cosmic_sf_floor32(c,(unsigned int)a);case 2:return cosmic_sf_ceil32(c,(unsigned int)a);
        case 3:return cosmic_sf_trunc32(c,(unsigned int)a);case 4:return cosmic_sf_round32(c,(unsigned int)a);
        case 5:return cosmic_sf_fabs32(c,(unsigned int)a);case 6:return cosmic_sf_copysign32(c,(unsigned int)a,(unsigned int)b);
        case 7:return cosmic_sf_scalbn32(c,(unsigned int)a,n);case 8:return cosmic_sf_frexp32(c,(unsigned int)a,e);
        case 9:return cosmic_sf_remquo32(c,(unsigned int)a,(unsigned int)b,e);default:return cosmic_sf_nextafter32(c,(unsigned int)a,(unsigned int)b);
        }
    }
    switch(op) {
    case 0:return cosmic_sf_fmod64(c,a,b);case 1:return cosmic_sf_floor64(c,a);case 2:return cosmic_sf_ceil64(c,a);
    case 3:return cosmic_sf_trunc64(c,a);case 4:return cosmic_sf_round64(c,a);case 5:return cosmic_sf_fabs64(c,a);
    case 6:return cosmic_sf_copysign64(c,a,b);case 7:return cosmic_sf_scalbn64(c,a,n);case 8:return cosmic_sf_frexp64(c,a,e);
    case 9:return cosmic_sf_remquo64(c,a,b,e);default:return cosmic_sf_nextafter64(c,a,b);
    }
    return 0;
}
static unsigned int lowq(int q) { return (q<0 ? 0u-(unsigned int)q : (unsigned int)q)&7u; }
int main(void) {
    unsigned int i;cosmic_sf_context c;
    for(i=0;i<SF_MATH_VECTOR_COUNT;i++) {
        int e=0;unsigned long long v;
        cosmic_sf_init(&c);c.rounding=sf_math_vectors[i].mode;
        v=mathcall(&c,sf_math_vectors[i].width,sf_math_vectors[i].op,sf_math_vectors[i].a,sf_math_vectors[i].b,sf_math_vectors[i].n,&e);
        if(v!=sf_math_vectors[i].result || c.flags!=sf_math_vectors[i].flags) return (int)i+1;
        if(sf_math_vectors[i].op==9) {
            if(lowq(e)!=lowq(sf_math_vectors[i].exponent)) return (int)i+10000;
            if(lowq(e) && (e<0)!=(sf_math_vectors[i].exponent<0)) return (int)i+20000;
        } else if(e!=sf_math_vectors[i].exponent) return (int)i+30000;
    }
    return 0;
}
