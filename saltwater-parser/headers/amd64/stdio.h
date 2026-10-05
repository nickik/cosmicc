#ifndef __COSMIC_AMD64_STDIO_H
#define __COSMIC_AMD64_STDIO_H
#include <stddef.h>
#include <stdarg.h>
typedef struct __cosmic_FILE FILE;

extern FILE *stdin; extern FILE *stdout; extern FILE *stderr;
#define EOF (-1)
#define SEEK_SET 0
#define SEEK_CUR 1
#define SEEK_END 2
#define BUFSIZ 8192
#define FILENAME_MAX 4096
FILE *fopen(const char *restrict,const char *restrict);
FILE *freopen(const char *restrict,const char *restrict,FILE *restrict);
int fclose(FILE*); int fflush(FILE*);
size_t fread(void *restrict,size_t,size_t,FILE *restrict);
size_t fwrite(const void *restrict,size_t,size_t,FILE *restrict);
int fgetc(FILE*); int getc(FILE*); int getchar(void);
int fputc(int,FILE*); int putc(int,FILE*); int putchar(int);
char *fgets(char *restrict,int,FILE *restrict);
int fputs(const char *restrict,FILE *restrict); int puts(const char*);
int printf(const char *restrict,...); int fprintf(FILE *restrict,const char *restrict,...);
int sprintf(char *restrict,const char *restrict,...);
int snprintf(char *restrict,size_t,const char *restrict,...);
int vprintf(const char *restrict,va_list); int vfprintf(FILE *restrict,const char *restrict,va_list);
int vsprintf(char *restrict,const char *restrict,va_list);
int vsnprintf(char *restrict,size_t,const char *restrict,va_list);
int fseek(FILE*,long,int); long ftell(FILE*); void rewind(FILE*);
int feof(FILE*); int ferror(FILE*); void clearerr(FILE*);
int remove(const char*); int rename(const char*,const char*); void perror(const char*);
FILE *tmpfile(void);
#endif
