#ifndef __COSMIC_SIA_STDARG_H
#define __COSMIC_SIA_STDARG_H
typedef unsigned char *va_list;
typedef va_list __gnuc_va_list;
void __cosmic_sia_va_start(void *, void *);
void *__cosmic_sia_va_arg(void *, unsigned long);
void __cosmic_sia_va_copy(void *, void *);
void __cosmic_sia_va_end(void *);
#define va_start(ap,last) __cosmic_sia_va_start(&(ap), &(last))
#define va_arg(ap,T) (*(T *)__cosmic_sia_va_arg(&(ap),sizeof(T)))
#define va_copy(dst,src) __cosmic_sia_va_copy(&(dst), &(src))
#define va_end(ap) __cosmic_sia_va_end(&(ap))
#endif
