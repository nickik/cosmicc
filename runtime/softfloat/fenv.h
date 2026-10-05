#ifndef COSMIC_FENV_H
#define COSMIC_FENV_H
#include "context.h"
typedef cosmic_sf_context fenv_t;
typedef unsigned int fexcept_t;
#define FE_INEXACT COSMIC_SF_INEXACT
#define FE_UNDERFLOW COSMIC_SF_UNDERFLOW
#define FE_OVERFLOW COSMIC_SF_OVERFLOW
#define FE_DIVBYZERO COSMIC_SF_DIVIDE_BY_ZERO
#define FE_INVALID COSMIC_SF_INVALID
#define FE_ALL_EXCEPT 31u
#define FE_TONEAREST COSMIC_SF_NEAREST
#define FE_TOWARDZERO COSMIC_SF_TOWARDZERO
#define FE_DOWNWARD COSMIC_SF_DOWNWARD
#define FE_UPWARD COSMIC_SF_UPWARD
#define FE_DFL_ENV ((const fenv_t *)0)
int feclearexcept(int);
int fegetexceptflag(fexcept_t *,int);
int feraiseexcept(int);
int fesetexceptflag(const fexcept_t *,int);
int fetestexcept(int);
int fegetround(void);
int fesetround(int);
int fegetenv(fenv_t *);
int feholdexcept(fenv_t *);
int fesetenv(const fenv_t *);
int feupdateenv(const fenv_t *);
#endif
