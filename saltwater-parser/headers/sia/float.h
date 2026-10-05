#ifndef COSMIC_SIA_FLOAT_H
#define COSMIC_SIA_FLOAT_H
/* SIA policy: distinct long double C type with binary64 precision/storage. */
int cosmic_sf_flt_rounds(void);
#define FLT_ROUNDS (cosmic_sf_flt_rounds())
#define FLT_RADIX 2
#define FLT_EVAL_METHOD 0
#define DECIMAL_DIG 17
#define FLT_MANT_DIG 24
#define FLT_DIG 6
#define FLT_DECIMAL_DIG 9
#define FLT_MIN_EXP (-125)
#define FLT_MAX_EXP 128
#define FLT_MIN_10_EXP (-37)
#define FLT_MAX_10_EXP 38
#define FLT_MAX 0x1.fffffep127f
#define FLT_EPSILON 0x1p-23f
#define FLT_MIN 0x1p-126f
#define FLT_TRUE_MIN 0x1p-149f
#define FLT_HAS_SUBNORM 1
#define DBL_MANT_DIG 53
#define DBL_DIG 15
#define DBL_DECIMAL_DIG 17
#define DBL_MIN_EXP (-1021)
#define DBL_MAX_EXP 1024
#define DBL_MIN_10_EXP (-307)
#define DBL_MAX_10_EXP 308
#define DBL_MAX 0x1.fffffffffffffp1023
#define DBL_EPSILON 0x1p-52
#define DBL_MIN 0x1p-1022
#define DBL_TRUE_MIN 0x1p-1074
#define DBL_HAS_SUBNORM 1
#define LDBL_MANT_DIG DBL_MANT_DIG
#define LDBL_DIG DBL_DIG
#define LDBL_DECIMAL_DIG DBL_DECIMAL_DIG
#define LDBL_MIN_EXP DBL_MIN_EXP
#define LDBL_MAX_EXP DBL_MAX_EXP
#define LDBL_MIN_10_EXP DBL_MIN_10_EXP
#define LDBL_MAX_10_EXP DBL_MAX_10_EXP
#define LDBL_MAX 0x1.fffffffffffffp1023L
#define LDBL_EPSILON 0x1p-52L
#define LDBL_MIN 0x1p-1022L
#define LDBL_TRUE_MIN 0x1p-1074L
#define LDBL_HAS_SUBNORM 1
#endif
