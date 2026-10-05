#include "include/cosmic_runtime_v1.h"
cosmic_size_t cosmic_strlen(const char *s) {
    cosmic_size_t n = 0; while (s[n] != 0) n++; return n;
}
cosmic_size_t cosmic_strnlen(const char *s, cosmic_size_t limit) {
    cosmic_size_t n = 0; while (n < limit && s[n] != 0) n++; return n;
}
int cosmic_strcmp(const char *a, const char *b) {
    while (*a != 0 && *a == *b) { a++; b++; }
    return (int)(unsigned char)*a - (int)(unsigned char)*b;
}
int cosmic_strncmp(const char *a, const char *b, cosmic_size_t limit) {
    cosmic_size_t i;
    for (i = 0; i < limit; i++) {
        if (a[i] != b[i] || a[i] == 0)
            return (int)(unsigned char)a[i] - (int)(unsigned char)b[i];
    }
    return 0;
}
char *cosmic_strchr(const char *s, int character) {
    unsigned int c = (unsigned int)character & 255u;
    while ((unsigned char)*s != c) { if (*s == 0) return 0; s++; }
    return (char *)s;
}
char *cosmic_strrchr(const char *s, int character) {
    unsigned int c = (unsigned int)character & 255u;
    const char *last = 0;
    do { if ((unsigned char)*s == c) last = s; } while (*s++ != 0);
    return (char *)last;
}
/* Explicit 32-bit unsigned conversion: status 0=ok, 1=no digits,
 * 2=range overflow, 3=invalid base. No process-global errno.
 */
static int cosmic_ascii_digit(unsigned int c) {
    if (c >= '0' && c <= '9') return (int)(c - '0');
    if (c >= 'a' && c <= 'z') return (int)(c - 'a') + 10;
    if (c >= 'A' && c <= 'Z') return (int)(c - 'A') + 10;
    return -1;
}
unsigned int cosmic_strtoul32(const char *text, char **end, int base,
                             unsigned int *status) {
    const char *s = text;
    unsigned int value = 0, negative = 0, any = 0, overflow = 0;
    unsigned int radix, cutoff, cutlim;
    int digit;
    if (end != 0) *end = (char *)text;
    if (status != 0) *status = 0;
    if (base != 0 && (base < 2 || base > 36)) {
        if (status != 0) *status = 3;
        return 0;
    }
    while (*s == ' ' || *s == '\t' || *s == '\n' || *s == '\r'
           || *s == '\f' || *s == '\v') s++;
    if (*s == '+' || *s == '-') { negative = *s == '-'; s++; }
    if ((base == 0 || base == 16) && s[0] == '0'
        && (s[1] == 'x' || s[1] == 'X')) {
        digit = cosmic_ascii_digit((unsigned char)s[2]);
        if (digit >= 0 && digit < 16) { base = 16; s += 2; }
    }
    if (base == 0) base = *s == '0' ? 8 : 10;
    radix = (unsigned int)base;
    cutoff = 0xffffffffu / radix;
    cutlim = 0xffffffffu % radix;
    while (1) {
        digit = cosmic_ascii_digit((unsigned char)*s);
        if (digit < 0 || (unsigned int)digit >= radix) break;
        any = 1;
        if (value > cutoff || (value == cutoff && (unsigned int)digit > cutlim))
            overflow = 1;
        if (overflow == 0) value = value * radix + (unsigned int)digit;
        s++;
    }
    if (any == 0) {
        if (status != 0) *status = 1;
        return 0;
    }
    if (end != 0) *end = (char *)s;
    if (overflow != 0) {
        if (status != 0) *status = 2;
        return 0xffffffffu;
    }
    return negative != 0 ? 0u - value : value;
}
