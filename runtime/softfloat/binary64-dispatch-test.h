/* Test-only dispatch normalizes integer result bits without any host FP. */
unsigned long long cosmic_sf_test_binary64(cosmic_sf_context *ctx,unsigned int op,unsigned long long a,unsigned long long b) {
    switch(op) {
    case 0:return cosmic_sf_add64(ctx,a,b);
    case 1:return cosmic_sf_sub64(ctx,a,b);
    case 2:return cosmic_sf_mul64(ctx,a,b);
    case 3:return cosmic_sf_div64(ctx,a,b);
    case 4:return (unsigned int)cosmic_sf_eq64(ctx,a,b);
    case 5:return (unsigned int)cosmic_sf_lt64(ctx,a,b);
    case 6:return (unsigned int)cosmic_sf_le64(ctx,a,b);
    case 7:return cosmic_sf_f32_to_f64(ctx,(unsigned int)a);
    case 8:return cosmic_sf_f64_to_f32(ctx,a);
    case 9:return (unsigned long long)cosmic_sf_f32_to_i64(ctx,(unsigned int)a);
    case 10:return cosmic_sf_f32_to_u64(ctx,(unsigned int)a);
    case 11:return (unsigned long long)cosmic_sf_f64_to_i64(ctx,a);
    case 12:return cosmic_sf_f64_to_u64(ctx,a);
    case 13:return (unsigned int)cosmic_sf_f64_to_i32(ctx,a);
    case 14:return cosmic_sf_f64_to_u32(ctx,a);
    case 15:return cosmic_sf_i64_to_f32(ctx,(long long)a);
    case 16:return cosmic_sf_u64_to_f32(ctx,a);
    case 17:return cosmic_sf_i64_to_f64(ctx,(long long)a);
    case 18:return cosmic_sf_u64_to_f64(ctx,a);
    case 19:return cosmic_sf_i32_to_f64(ctx,(int)a);
    default:return cosmic_sf_u32_to_f64(ctx,(unsigned int)a);
    }
    return 0;
}
