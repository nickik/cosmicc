/* I32-only low-word-first arithmetic. Include after primitives.c.
 * Explicit helpers, not automatic CLIF legalization or an I64 return ABI.
 */
void cosmic_mul64(cosmic_words64 *out, cosmic_u32 a_low,
                  cosmic_u32 a_high, cosmic_u32 b_low, cosmic_u32 b_high) {
    cosmic_words64 product;
    cosmic_mul32_wide(&product, a_low, b_low);
    out->low = product.low;
    out->high = product.high + a_high * b_low + a_low * b_high;
}
/* Returns 1 for division by zero, 2 for INT64_MIN/-1 overflow.
 * Failure leaves both outputs unchanged. Outputs must be distinct.
 * Success truncates toward zero; remainder has the numerator's sign.
 * All negation is unsigned, including INT64_MIN, avoiding C signed UB.
 */
int cosmic_sdivmod64(cosmic_words64 *quotient, cosmic_words64 *remainder,
                     cosmic_u32 n_low, cosmic_u32 n_high,
                     cosmic_u32 d_low, cosmic_u32 d_high) {
    cosmic_u32 n_negative = n_high >> 31, d_negative = d_high >> 31;
    cosmic_words64 q, r;
    if ((d_low | d_high) == 0) return 1;
    if (n_low == 0 && n_high == 0x80000000u &&
        d_low == 0xffffffffu && d_high == 0xffffffffu) return 2;
    if (n_negative) {
        n_low = ~n_low + 1;
        n_high = ~n_high + (n_low == 0);
    }
    if (d_negative) {
        d_low = ~d_low + 1;
        d_high = ~d_high + (d_low == 0);
    }
    cosmic_udivmod64(&q, &r, n_low, n_high, d_low, d_high);
    if (n_negative != d_negative) {
        q.low = ~q.low + 1;
        q.high = ~q.high + (q.low == 0);
    }
    if (n_negative) {
        r.low = ~r.low + 1;
        r.high = ~r.high + (r.low == 0);
    }
    quotient->low = q.low; quotient->high = q.high;
    remainder->low = r.low; remainder->high = r.high;
    return 0;
}
