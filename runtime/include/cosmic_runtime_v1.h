#ifndef COSMIC_RUNTIME_V1_H
#define COSMIC_RUNTIME_V1_H
#define COSMIC_RUNTIME_ABI_VERSION 1u
typedef unsigned int cosmic_size_t;
/* SIA32 only: 32-bit guest pointers and unsigned int, eight-bit bytes. */
typedef struct {
    unsigned char *base;
    cosmic_size_t capacity;
    cosmic_size_t used;
} cosmic_arena;
int cosmic_arena_init(cosmic_arena *, void *, cosmic_size_t);
void *cosmic_arena_alloc(cosmic_arena *, cosmic_size_t);
void *cosmic_arena_realloc(cosmic_arena *, void *, cosmic_size_t);
int cosmic_arena_free(cosmic_arena *, void *);
void *cosmic_arena_calloc(cosmic_arena *, cosmic_size_t, cosmic_size_t);
#define COSMIC_PARSE_OK 0u
#define COSMIC_PARSE_NO_DIGITS 1u
#define COSMIC_PARSE_RANGE 2u
#define COSMIC_PARSE_INVALID_BASE 3u
unsigned int cosmic_strtoul32(const char *, char **, int, unsigned int *);
void cosmic_arena_reset(cosmic_arena *);
cosmic_size_t cosmic_strlen(const char *);
cosmic_size_t cosmic_strnlen(const char *, cosmic_size_t);
int cosmic_strcmp(const char *, const char *);
int cosmic_strncmp(const char *, const char *, cosmic_size_t);
char *cosmic_strchr(const char *, int);
char *cosmic_strrchr(const char *, int);
#endif
