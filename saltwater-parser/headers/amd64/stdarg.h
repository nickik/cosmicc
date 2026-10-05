#ifndef __COSMIC_AMD64_STDARG_H
#define __COSMIC_AMD64_STDARG_H
typedef struct {
    unsigned int gp_offset;
    unsigned int fp_offset;
    void *overflow_arg_area;
    void *reg_save_area;
} __cosmic_va_state;
typedef __cosmic_va_state va_list[1];
typedef va_list __gnuc_va_list;
void __cosmic_va_start(void *, void *);
void *__cosmic_va_arg(void *, ...);
void __cosmic_va_copy(void *, const void *);
void __cosmic_va_end(void *);
#define va_start(ap, last) __cosmic_va_start((ap), &(last))
#define va_arg(ap, T) (*(T *)__cosmic_va_arg((ap), (T *)0))
#define va_copy(dst, src) __cosmic_va_copy((dst), (src))
#define va_end(ap) __cosmic_va_end((ap))
#endif
