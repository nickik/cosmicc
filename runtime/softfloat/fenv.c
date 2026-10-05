#include "fenv.h"
int feclearexcept(int e) { cosmic_sf_clear(cosmic_sf_current_context(),(unsigned int)e); return 0; }
int fegetexceptflag(fexcept_t *p,int e) { *p=cosmic_sf_flags(cosmic_sf_current_context()) & (unsigned int)e; return 0; }
int feraiseexcept(int e) { cosmic_sf_raise(cosmic_sf_current_context(),(unsigned int)e); return 0; }
int fesetexceptflag(const fexcept_t *p,int e) {
    cosmic_sf_context *c=cosmic_sf_current_context();
    cosmic_sf_clear(c,(unsigned int)e); cosmic_sf_raise(c,*p & (unsigned int)e); return 0;
}
int fetestexcept(int e) { return (int)(cosmic_sf_flags(cosmic_sf_current_context()) & (unsigned int)e); }
int fegetround(void) { return (int)cosmic_sf_current_context()->rounding; }
int fesetround(int mode) {
    if(mode<0 || mode>3) return 1;
    cosmic_sf_current_context()->rounding=(unsigned int)mode; return 0;
}
int fegetenv(fenv_t *p) { *p=*cosmic_sf_current_context(); return 0; }
int feholdexcept(fenv_t *p) { fegetenv(p); feclearexcept(FE_ALL_EXCEPT); return 0; }
int fesetenv(const fenv_t *p) {
    cosmic_sf_context *c=cosmic_sf_current_context();
    if(!p) { cosmic_sf_init(c); return 0; }
    if(p->rounding>3) return 1;
    c->flags=p->flags & FE_ALL_EXCEPT; c->rounding=p->rounding; return 0;
}
int feupdateenv(const fenv_t *p) {
    unsigned int flags=cosmic_sf_flags(cosmic_sf_current_context());
    int status=fesetenv(p); if(status) return status;
    cosmic_sf_raise(cosmic_sf_current_context(),flags); return 0;
}
