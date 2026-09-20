#ifndef __STDC_STDIO_H
#define __STDC_STDIO_H

#include <stddef.h>
#include <sys/types.h>

typedef struct __cosmic_FILE FILE;
typedef long fpos_t;

#define EOF (-1)
#define SEEK_SET 0
#define SEEK_CUR 1
#define SEEK_END 2

int printf(const char *format, ...);
int fprintf(FILE *stream, const char *format, ...);
int snprintf(char *buffer, size_t size, const char *format, ...);

#endif
