#ifndef COSMIC_SOFTFLOAT_BINARY32_H
#define COSMIC_SOFTFLOAT_BINARY32_H
#include "context.h"
/* Raw IEEE binary32 bits; fixed nearest ties-even, gradual underflow,
   tininess after rounding, ARM-VFPv2 NaN payload/default policy.
   Context is nonnull, aligned guest storage with sticky exception flags. */
unsigned int cosmic_sf_add32(cosmic_sf_context *ctx, unsigned int a, unsigned int b);
unsigned int cosmic_sf_sub32(cosmic_sf_context *ctx, unsigned int a, unsigned int b);
unsigned int cosmic_sf_mul32(cosmic_sf_context *ctx, unsigned int a, unsigned int b);
unsigned int cosmic_sf_div32(cosmic_sf_context *ctx, unsigned int a, unsigned int b);
int cosmic_sf_eq32(cosmic_sf_context *ctx, unsigned int a, unsigned int b);
int cosmic_sf_lt32(cosmic_sf_context *ctx, unsigned int a, unsigned int b);
int cosmic_sf_le32(cosmic_sf_context *ctx, unsigned int a, unsigned int b);
/* Float-to-integer truncates toward zero, sets inexact/invalid, and uses
   ARM-VFPv2 saturation/NaN results. C out-of-range casts remain undefined. */
int cosmic_sf_to_i32(cosmic_sf_context *ctx, unsigned int a);
unsigned int cosmic_sf_to_u32(cosmic_sf_context *ctx, unsigned int a);
unsigned int cosmic_sf_from_i32(cosmic_sf_context *ctx, int a);
unsigned int cosmic_sf_from_u32(cosmic_sf_context *ctx, unsigned int a);
unsigned long long cosmic_sf_add64(cosmic_sf_context *,unsigned long long,unsigned long long);
unsigned long long cosmic_sf_sub64(cosmic_sf_context *,unsigned long long,unsigned long long);
unsigned long long cosmic_sf_mul64(cosmic_sf_context *,unsigned long long,unsigned long long);
unsigned long long cosmic_sf_div64(cosmic_sf_context *,unsigned long long,unsigned long long);
int cosmic_sf_eq64(cosmic_sf_context *,unsigned long long,unsigned long long);
int cosmic_sf_lt64(cosmic_sf_context *,unsigned long long,unsigned long long);
int cosmic_sf_le64(cosmic_sf_context *,unsigned long long,unsigned long long);
unsigned long long cosmic_sf_f32_to_f64(cosmic_sf_context *,unsigned int);
unsigned int cosmic_sf_f64_to_f32(cosmic_sf_context *,unsigned long long);
long long cosmic_sf_f32_to_i64(cosmic_sf_context *,unsigned int);
unsigned long long cosmic_sf_f32_to_u64(cosmic_sf_context *,unsigned int);
long long cosmic_sf_f64_to_i64(cosmic_sf_context *,unsigned long long);
unsigned long long cosmic_sf_f64_to_u64(cosmic_sf_context *,unsigned long long);
int cosmic_sf_f64_to_i32(cosmic_sf_context *,unsigned long long);
unsigned int cosmic_sf_f64_to_u32(cosmic_sf_context *,unsigned long long);
unsigned int cosmic_sf_i64_to_f32(cosmic_sf_context *,long long);
unsigned int cosmic_sf_u64_to_f32(cosmic_sf_context *,unsigned long long);
unsigned long long cosmic_sf_i64_to_f64(cosmic_sf_context *,long long);
unsigned long long cosmic_sf_u64_to_f64(cosmic_sf_context *,unsigned long long);
unsigned long long cosmic_sf_i32_to_f64(cosmic_sf_context *,int);
unsigned long long cosmic_sf_u32_to_f64(cosmic_sf_context *,unsigned int);
unsigned int cosmic_sf_sqrt32(cosmic_sf_context *,unsigned int);
unsigned long long cosmic_sf_sqrt64(cosmic_sf_context *,unsigned long long);
unsigned int cosmic_sf_trunc32(cosmic_sf_context *,unsigned int);
unsigned int cosmic_sf_floor32(cosmic_sf_context *,unsigned int);
unsigned int cosmic_sf_ceil32(cosmic_sf_context *,unsigned int);
unsigned int cosmic_sf_round32(cosmic_sf_context *,unsigned int);
unsigned int cosmic_sf_fabs32(cosmic_sf_context *,unsigned int);
unsigned int cosmic_sf_fmod32(cosmic_sf_context *,unsigned int,unsigned int);
unsigned int cosmic_sf_copysign32(cosmic_sf_context *,unsigned int,unsigned int);
unsigned int cosmic_sf_frexp32(cosmic_sf_context *,unsigned int,int *);
unsigned int cosmic_sf_scalbn32(cosmic_sf_context *,unsigned int,int);
unsigned int cosmic_sf_pack32(cosmic_sf_context *,int,int,unsigned int);
unsigned long long cosmic_sf_trunc64(cosmic_sf_context *,unsigned long long);
unsigned long long cosmic_sf_floor64(cosmic_sf_context *,unsigned long long);
unsigned long long cosmic_sf_ceil64(cosmic_sf_context *,unsigned long long);
unsigned long long cosmic_sf_round64(cosmic_sf_context *,unsigned long long);
unsigned long long cosmic_sf_fabs64(cosmic_sf_context *,unsigned long long);
unsigned long long cosmic_sf_fmod64(cosmic_sf_context *,unsigned long long,unsigned long long);
unsigned long long cosmic_sf_copysign64(cosmic_sf_context *,unsigned long long,unsigned long long);
unsigned long long cosmic_sf_frexp64(cosmic_sf_context *,unsigned long long,int *);
unsigned long long cosmic_sf_scalbn64(cosmic_sf_context *,unsigned long long,int);
unsigned long long cosmic_sf_pack64(cosmic_sf_context *,int,int,unsigned long long);
unsigned int cosmic_sf_fma32(cosmic_sf_context *,unsigned int,unsigned int,unsigned int);
unsigned long long cosmic_sf_fma64(cosmic_sf_context *,unsigned long long,unsigned long long,unsigned long long);
unsigned int cosmic_sf_rint32(cosmic_sf_context *,unsigned int,int);
unsigned long long cosmic_sf_rint64(cosmic_sf_context *,unsigned long long,int);
unsigned int cosmic_sf_remquo32(cosmic_sf_context *,unsigned int,unsigned int,int *);
unsigned int cosmic_sf_nextafter32(cosmic_sf_context *,unsigned int,unsigned int);
unsigned long long cosmic_sf_remquo64(cosmic_sf_context *,unsigned long long,unsigned long long,int *);
unsigned long long cosmic_sf_nextafter64(cosmic_sf_context *,unsigned long long,unsigned long long);
unsigned int cosmic_sf_nexttoward32(cosmic_sf_context *,unsigned int,unsigned long long);
#endif
