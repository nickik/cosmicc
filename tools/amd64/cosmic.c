extern long host_global;
extern long host_callback(int, long, int *, long, long, long, long, long);
extern long separate_helper(long);
static long same_local_name = 11;
long cosmic_table[3] = { 0x100000001L, -7L, 99L };
unsigned long cosmic_widths(void) {
    return sizeof(void *) + 16UL * sizeof(long) + 256UL * sizeof(int);
}
long cosmic_mixed(int a, long b, int *out, long c, long d, long e, long f, long g) {
    return host_callback(a,b,out,c,d,e,f,g) + separate_helper(b) +
           host_global + same_local_name;
}
long cosmic_array(long *p, unsigned long count) {
    unsigned long i;
    long total = 0;
    for (i = 0; i < count; i++) total += p[i];
    return total;
}
long cosmic_indirect(long (*function)(long), long value) { return function(value) + 7; }
long cosmic_signed(long a, long b) { return a / b + a % b; }
unsigned long cosmic_unsigned(unsigned long a, unsigned long b) {
    return (a / b) ^ (a % b) ^ (a >> 37);
}
long cosmic_narrow_sum(signed char a, short b, unsigned char c, unsigned short d) {
    return (long)a + b + c + d;
}
signed char cosmic_char_return(int value) { return (signed char)value; }
short cosmic_short_return(int value) { return (short)value; }
extern signed char host_char(void);
extern short host_short(void);
extern unsigned char host_uchar(void);
extern unsigned short host_ushort(void);
long cosmic_narrow_returns(void) {
    return (long)host_char() + host_short() + host_uchar() + host_ushort();
}
double cosmic_double_sum(double a,double b,double c,double d,double e,
                        double f,double g,double h,double i) {
    return a+b+c+d+e+f+g+h+i;
}
extern double host_double_mix(int,double,long,double,double,double,double,double,double,double,double);
double cosmic_double_mix(int a,double b,long c,double d,double e,double f,
                        double g,double h,double i,double j,double k) {
    return host_double_mix(a,b,c,d,e,f,g,h,i,j,k)+0.5;
}
