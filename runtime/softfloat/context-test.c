#include "context.c"
int main(void) {
    cosmic_sf_context first, second;
    cosmic_sf_init(&first);
    cosmic_sf_init(&second);
    cosmic_sf_raise(&first, COSMIC_SF_INVALID | COSMIC_SF_INEXACT);
    cosmic_sf_raise(&second, COSMIC_SF_DIVIDE_BY_ZERO);
    if (cosmic_sf_flags(&first) != 17u) return 1;
    if (cosmic_sf_flags(&second) != 8u) return 2;
    cosmic_sf_clear(&first, COSMIC_SF_INVALID);
    if (cosmic_sf_flags(&first) != 1u) return 3;
    cosmic_sf_raise(&first, 0xffffffe0u);
    if (cosmic_sf_flags(&first) != 1u) return 4;
    cosmic_sf_clear(&second, 31u);
    return cosmic_sf_flags(&second) != 0u;
}
