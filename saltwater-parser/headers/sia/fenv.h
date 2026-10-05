/* Target-owned SIA software floating environment. */
#ifndef COSMIC_FENV_H
#define COSMIC_FENV_H
/* Explicit task-owned software floating-point environment. */
#ifndef COSMIC_SOFTFLOAT_CONTEXT_H
#define COSMIC_SOFTFLOAT_CONTEXT_H
#define COSMIC_SOFTFLOAT_CONTEXT_ABI 2
typedef struct { unsigned int flags; unsigned int rounding; } cosmic_sf_context;
#define COSMIC_SF_NEAREST 0u
#define COSMIC_SF_TOWARDZERO 1u
#define COSMIC_SF_DOWNWARD 2u
#define COSMIC_SF_UPWARD 3u
cosmic_sf_context *cosmic_sf_current_context(void);
cosmic_sf_context *cosmic_sf_bind_context(cosmic_sf_context *ctx);
/* Values match SoftFloat 3e's flag constants. */
#define COSMIC_SF_INEXACT 1u
#define COSMIC_SF_UNDERFLOW 2u
#define COSMIC_SF_OVERFLOW 4u
#define COSMIC_SF_DIVIDE_BY_ZERO 8u
#define COSMIC_SF_INVALID 16u
void cosmic_sf_init(cosmic_sf_context *ctx);
void cosmic_sf_raise(cosmic_sf_context *ctx, unsigned int flags);
unsigned int cosmic_sf_flags(const cosmic_sf_context *ctx);
void cosmic_sf_clear(cosmic_sf_context *ctx, unsigned int flags);
#endif

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
