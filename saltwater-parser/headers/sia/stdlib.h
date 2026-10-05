#ifndef __STDC_STDLIB_H
#define __STDC_STDLIB_H

#include <stddef.h>

void *malloc(size_t size);
void *calloc(size_t count, size_t size);
void *realloc(void *ptr, size_t size);
void free(void *ptr);
void abort(void);
void exit(int status);
int atexit(void (*function)(void));
unsigned long strtoul(const char *restrict str, char **restrict endptr, int base);

float strtof(const char *restrict,char **restrict);
double strtod(const char *restrict,char **restrict);
long double strtold(const char *restrict,char **restrict);
double atof(const char *);
#endif
