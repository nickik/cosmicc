#include "context.h"
/* ctx must point to writable guest storage owned by the calling task.
 * No global flags, TLS assumption, allocation, or arithmetic implementation.
 */
void cosmic_sf_init(cosmic_sf_context *ctx) { ctx->flags = 0; ctx->rounding = COSMIC_SF_NEAREST; }
void cosmic_sf_raise(cosmic_sf_context *ctx, unsigned int flags) {
    ctx->flags |= flags & 31u;
}
unsigned int cosmic_sf_flags(const cosmic_sf_context *ctx) {
    return ctx->flags & 31u;
}
void cosmic_sf_clear(cosmic_sf_context *ctx, unsigned int flags) {
    ctx->flags &= ~(flags & 31u);
}

/* Scheduler must bind before executing a task; this is not TLS. */
static cosmic_sf_context cosmic_sf_boot_context;
static cosmic_sf_context *cosmic_sf_bound_context;
cosmic_sf_context *cosmic_sf_current_context(void) {
    return cosmic_sf_bound_context ? cosmic_sf_bound_context : &cosmic_sf_boot_context;
}
cosmic_sf_context *cosmic_sf_bind_context(cosmic_sf_context *ctx) {
    cosmic_sf_context *prior=cosmic_sf_current_context();
    cosmic_sf_bound_context=ctx;
    return prior;
}
int cosmic_sf_flt_rounds(void) {
    unsigned int mode=cosmic_sf_current_context()->rounding;
    if(mode==COSMIC_SF_NEAREST) return 1;
    if(mode==COSMIC_SF_TOWARDZERO) return 0;
    if(mode==COSMIC_SF_UPWARD) return 2;
    if(mode==COSMIC_SF_DOWNWARD) return 3;
    return -1;
}
