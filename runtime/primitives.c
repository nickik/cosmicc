/* Target-native integer-only foundations; not a complete libc or FP runtime.
 * Buffers must designate valid guest memory. No host headers or pointers.
 */
typedef unsigned int cosmic_u32;
typedef struct { cosmic_u32 low; cosmic_u32 high; } cosmic_words64;
void *cosmic_memcpy(void *dest, const void *src, cosmic_u32 size) {
    unsigned char *d = (unsigned char *)dest;
    const unsigned char *s = (const unsigned char *)src;
    cosmic_u32 i;
    for (i = 0; i < size; i++) d[i] = s[i];
    return dest;
}
void *cosmic_memmove(void *dest, const void *src, cosmic_u32 size) {
    unsigned char *d = (unsigned char *)dest;
    const unsigned char *s = (const unsigned char *)src;
    cosmic_u32 i;
    /* Integer addresses avoid relational comparison of unrelated pointers. */
    if ((cosmic_u32)d <= (cosmic_u32)s) {
        for (i = 0; i < size; i++) d[i] = s[i];
    } else {
        i = size;
        while (i != 0) { i--; d[i] = s[i]; }
    }
    return dest;
}
void *cosmic_memset(void *dest, int value, cosmic_u32 size) {
    unsigned char *d = (unsigned char *)dest;
    cosmic_u32 i;
    for (i = 0; i < size; i++) d[i] = (unsigned char)value;
    return dest;
}
int cosmic_memcmp(const void *left, const void *right, cosmic_u32 size) {
    const unsigned char *a = (const unsigned char *)left;
    const unsigned char *b = (const unsigned char *)right;
    cosmic_u32 i;
    for (i = 0; i < size; i++) {
        if (a[i] < b[i]) return -1;
        if (a[i] > b[i]) return 1;
    }
    return 0;
}
/* IEEE class: 0=zero, 1=subnormal, 2=normal, 3=infinity, 4=NaN.
 * Preserve payloads; these raw-bit primitives do not raise exception flags.
 */
cosmic_u32 cosmic_f32_neg_bits(cosmic_u32 bits) { return bits ^ 0x80000000u; }
cosmic_u32 cosmic_f32_abs_bits(cosmic_u32 bits) { return bits & 0x7fffffffu; }
int cosmic_f32_class_bits(cosmic_u32 bits) {
    cosmic_u32 exp = bits & 0x7f800000u, frac = bits & 0x007fffffu;
    if (exp == 0) return frac == 0 ? 0 : 1;
    if (exp == 0x7f800000u) return frac == 0 ? 3 : 4;
    return 2;
}
int cosmic_f64_class_words(cosmic_u32 low, cosmic_u32 high) {
    cosmic_u32 exp = high & 0x7ff00000u, frac = (high & 0x000fffffu) | low;
    if (exp == 0) return frac == 0 ? 0 : 1;
    if (exp == 0x7ff00000u) return frac == 0 ? 3 : 4;
    return 2;
}
/* Right shift with sticky bit. Checks avoid shifts by the word width. */
cosmic_u32 cosmic_shift_right_jam32(cosmic_u32 value, cosmic_u32 distance) {
    if (distance == 0) return value;
    if (distance >= 32) return value != 0;
    return (value >> distance) | ((value << (32 - distance)) != 0);
}
void cosmic_shift_right_jam64(cosmic_words64 *out, cosmic_u32 low,
                              cosmic_u32 high, cosmic_u32 distance) {
    if (distance == 0) { out->low = low; out->high = high; }
    else if (distance < 32) {
        out->low = (low >> distance) | (high << (32 - distance)) |
                   ((low << (32 - distance)) != 0);
        out->high = high >> distance;
    } else if (distance == 32) {
        out->low = high | (low != 0); out->high = 0;
    } else if (distance < 64) {
        out->low = (high >> (distance - 32)) |
                   ((high << (64 - distance)) != 0 || low != 0);
        out->high = 0;
    } else { out->low = (low != 0 || high != 0); out->high = 0; }
}
/* I32-only arithmetic blocks; not yet compiler-generated I64 libcalls. */
void cosmic_mul32_wide(cosmic_words64 *out, cosmic_u32 a, cosmic_u32 b) {
    cosmic_u32 low = 0, high = 0, a_high = 0;
    while (b != 0) {
        if ((b & 1) != 0) {
            cosmic_u32 old = low;
            low += a;
            high += a_high + (low < old);
        }
        a_high = (a_high << 1) | (a >> 31);
        a <<= 1;
        b >>= 1;
    }
    out->low = low; out->high = high;
}
/* Unsigned restoring division; zero divisor returns 1, leaving outputs intact.
 * Output objects must be distinct. Carry retains the 65th remainder bit.
 */
int cosmic_udivmod64(cosmic_words64 *quotient, cosmic_words64 *remainder,
                     cosmic_u32 n_low, cosmic_u32 n_high,
                     cosmic_u32 d_low, cosmic_u32 d_high) {
    cosmic_u32 q_low = 0, q_high = 0, r_low = 0, r_high = 0;
    int i;
    if ((d_low | d_high) == 0) return 1;
    for (i = 0; i < 64; i++) {
        cosmic_u32 carry = r_high >> 31;
        r_high = (r_high << 1) | (r_low >> 31);
        r_low = (r_low << 1) | (n_high >> 31);
        n_high = (n_high << 1) | (n_low >> 31);
        n_low <<= 1;
        q_high = (q_high << 1) | (q_low >> 31);
        q_low <<= 1;
        if (carry || r_high > d_high || (r_high == d_high && r_low >= d_low)) {
            cosmic_u32 borrow = r_low < d_low;
            r_low -= d_low;
            r_high = r_high - d_high - borrow;
            q_low |= 1;
        }
    }
    quotient->low = q_low; quotient->high = q_high;
    remainder->low = r_low; remainder->high = r_high;
    return 0;
}
