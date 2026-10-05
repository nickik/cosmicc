#include "binary32.h"
#include "binary64-dispatch-test.h"
#include "environment-vectors.h"
static unsigned int env32(cosmic_sf_context *c,unsigned int op,unsigned int a,unsigned int b) {
    switch(op) {
    case 0:return cosmic_sf_add32(c,a,b);
    case 1:return cosmic_sf_sub32(c,a,b);
    case 2:return cosmic_sf_mul32(c,a,b);
    case 3:return cosmic_sf_div32(c,a,b);
    case 4:return cosmic_sf_sqrt32(c,a);
    case 5:return cosmic_sf_from_i32(c,(int)a);
    default:return cosmic_sf_from_u32(c,a);
    }
    return 0;
}
int main(void) {
    unsigned int i;cosmic_sf_context c,other;
    cosmic_sf_init(&other);other.flags=COSMIC_SF_INVALID;
    for(i=0;i<SF_ENV_VECTOR_COUNT;i++) {
        unsigned long long got;
        cosmic_sf_init(&c);c.rounding=sf_env_vectors[i].mode;
        if(sf_env_vectors[i].width==32) got=env32(&c,sf_env_vectors[i].op,(unsigned int)sf_env_vectors[i].a,(unsigned int)sf_env_vectors[i].b);
        else if(sf_env_vectors[i].op==21) got=cosmic_sf_sqrt64(&c,sf_env_vectors[i].a);
        else got=cosmic_sf_test_binary64(&c,sf_env_vectors[i].op,sf_env_vectors[i].a,sf_env_vectors[i].b);
        if(got!=sf_env_vectors[i].result || c.flags!=sf_env_vectors[i].flags) return (int)i+1;
        if(other.flags!=COSMIC_SF_INVALID || other.rounding!=COSMIC_SF_NEAREST) return 20000;
    }
    return 0;
}
