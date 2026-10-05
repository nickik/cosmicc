/* Acceptance wrapper; crc32.c is included unmodified from the upstream tree. */
#include "crc32.c"
int main(void) {
    return crc32(0, (const unsigned char *)"123456789", 9) == 0xcbf43926UL ? 0 : 1;
}
