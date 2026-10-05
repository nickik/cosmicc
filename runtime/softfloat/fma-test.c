#include "binary32.h"
#include "fma-vectors.h"
int main(void) {
    unsigned int i,k;cosmic_sf_context c;
    for(i=0;i<SF_FMA_VECTOR_COUNT;i++) for(k=0;k<2;k++) {
        unsigned long long v;unsigned int old=k ? COSMIC_SF_DIVIDE_BY_ZERO : 0;
        cosmic_sf_init(&c);c.flags=old;c.rounding=sf_fma_vectors[i].mode;
        if(sf_fma_vectors[i].width==32) {
            if(sf_fma_vectors[i].op) v=cosmic_sf_rint32(&c,(unsigned int)sf_fma_vectors[i].a,1);
            else v=cosmic_sf_fma32(&c,(unsigned int)sf_fma_vectors[i].a,(unsigned int)sf_fma_vectors[i].b,(unsigned int)sf_fma_vectors[i].c);
        } else {
            if(sf_fma_vectors[i].op) v=cosmic_sf_rint64(&c,sf_fma_vectors[i].a,1);
            else v=cosmic_sf_fma64(&c,sf_fma_vectors[i].a,sf_fma_vectors[i].b,sf_fma_vectors[i].c);
        }
        if(v!=sf_fma_vectors[i].result || c.flags!=(sf_fma_vectors[i].flags|old)) return (int)i+1;
    }
    return 0;
}
