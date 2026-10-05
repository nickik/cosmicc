/* Compile-only target sysroot smoke: declarations do not supply a runtime. */
#include <limits.h>
#include <stdint.h>
#include <stddef.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>

#define REQUIRE(name, condition) typedef char name[(condition) ? 1 : -1]
REQUIRE(char_bits, CHAR_BIT == 8);
REQUIRE(char_width, sizeof(char) == 1);
REQUIRE(short_width, sizeof(short) == 2);
REQUIRE(int_width, sizeof(int) == 4);
REQUIRE(long_width, sizeof(long) == 4);
REQUIRE(longlong_width, sizeof(long long) == 8);
REQUIRE(pointer_width, sizeof(void *) == 4);
REQUIRE(long_boundary_width, sizeof(LONG_MAX) == 4);
REQUIRE(longlong_boundary_width, sizeof(LLONG_MAX) == 8);
REQUIRE(unsigned_longlong_boundary_width, sizeof(ULLONG_MAX) == 8);
REQUIRE(signed_char_bounds, SCHAR_MIN == -128 && SCHAR_MAX == 127);
REQUIRE(unsigned_char_bounds, UCHAR_MAX == 255U);
REQUIRE(short_bounds, SHRT_MIN == -32768 && SHRT_MAX == 32767);
REQUIRE(unsigned_short_bounds, USHRT_MAX == 65535U);
REQUIRE(int_bounds, INT_MIN == (-2147483647 - 1) && INT_MAX == 2147483647);
REQUIRE(unsigned_int_bounds, UINT_MAX == 4294967295U);
REQUIRE(long_bounds, LONG_MIN == (-2147483647L - 1L) && LONG_MAX == 2147483647L);
REQUIRE(unsigned_long_bounds, ULONG_MAX == 4294967295UL);
REQUIRE(longlong_bounds, LLONG_MIN == (-9223372036854775807LL - 1LL));
REQUIRE(unsigned_longlong_bounds, ULLONG_MAX == 18446744073709551615ULL);
REQUIRE(errno_range, ERANGE == 34);

signed char char_minimum = SCHAR_MIN;
unsigned char char_maximum = UCHAR_MAX;
short short_minimum = SHRT_MIN;
unsigned short short_maximum = USHRT_MAX;
int int_minimum = INT_MIN;
unsigned int int_maximum = UINT_MAX;
long long_minimum = LONG_MIN;
unsigned long long_maximum = ULONG_MAX;
long long longlong_minimum = LLONG_MIN;
unsigned long long longlong_maximum = ULLONG_MAX;

char *(*find_character)(const char *, int) = strchr;
unsigned long (*parse_unsigned)(const char *, char **, int) = strtoul;
int main(void) { return sizeof(longlong_minimum) == 8 ? 0 : 1; }
