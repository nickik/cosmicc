#include <stddef.h>
#include <stdint.h>
struct Padded { char tag; long value; long *pointer; };
static long values[4] = {11L, 22L, 33L, 44L};
static struct Padded record = {7, 0x200000003L, &values[2]};
static long *member_pointer = &record.value;
static long next(void) { static long count = 0x100000000L; return ++count; }
int reviewed_target(double nan_value, double negative_zero) {
    long *p = values;
    if (sizeof(long) != 8 || sizeof(void *) != 8 || sizeof(size_t) != 8) return 1;
    if (sizeof(record) != 24 || (char *)&record.value - (char *)&record != 8) return 2;
    if (*member_pointer != 0x200000003L || *record.pointer != 33L) return 3;
    if (*(p++) != 11L || *p != 22L || ++p != &values[2]) return 4;
    if (&values[3] - &values[0] != 3 || &values[0] - &values[3] != -3) return 5;
    if (next() != 0x100000001L || next() != 0x100000002L) return 6;
    if ((signed char)255 != -1 || (unsigned char)-1 != 255) return 7;
    if ((long)(signed char)-100 != -100L || (unsigned long)(unsigned int)-1 != 4294967295UL) return 8;
    if ((unsigned long)4294967297.0 != 4294967297UL || (double)4294967297UL != 4294967297.0) return 9;
    if (!nan_value || !(_Bool)nan_value || nan_value == nan_value || !(nan_value != nan_value)) return 10;
    if (nan_value < 0.0 || nan_value >= 0.0) return 11;
    if (negative_zero || (_Bool)negative_zero || negative_zero != 0.0) return 12;
    return 0;
}
