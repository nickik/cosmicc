#ifndef __COSMIC_WCHAR_H
#define __COSMIC_WCHAR_H
#include <stddef.h>
/* Both supported targets use a signed 32-bit wide execution character. */
typedef int wchar_t;
typedef unsigned int wint_t;
#define WEOF ((wint_t)-1)
#define WCHAR_MIN (-2147483647-1)
#define WCHAR_MAX 2147483647
size_t wcslen(const wchar_t *);
int wcscmp(const wchar_t *, const wchar_t *);
#endif
