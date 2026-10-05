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
int cosmic_sf_flt_rounds(void);
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
