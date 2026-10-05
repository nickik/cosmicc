#include <stdio.h>
extern unsigned long cosmic_widths(void);
extern long cosmic_mixed(int,long,int*,long,long,long,long,long);
extern long cosmic_array(long *,unsigned long);
extern long cosmic_indirect(long (*)(long),long);
extern long cosmic_signed(long,long);
extern unsigned long cosmic_unsigned(unsigned long,unsigned long);
extern long cosmic_table[3];
extern long cosmic_narrow_sum(signed char,short,unsigned char,unsigned short);
extern signed char cosmic_char_return(int);
extern short cosmic_short_return(int);
extern long cosmic_narrow_returns(void);
extern double cosmic_double_sum(double,double,double,double,double,double,double,double,double);
extern double cosmic_double_mix(int,double,long,double,double,double,double,double,double,double,double);
signed char host_char(void) { return -100; }
short host_short(void) { return -300; }
unsigned char host_uchar(void) { return 250; }
unsigned short host_ushort(void) { return 60000; }
double host_double_mix(int a,double b,long c,double d,double e,double f,
                       double g,double h,double i,double j,double k) {
    return a+b+c+d+e+f+g+h+i+j+k;
}
long host_global = 0x200000003L;
long host_callback(int a,long b,int *out,long c,long d,long e,long f,long g) {
    *out = a - 3;
    return a + b + c + d + e + f + g;
}
static long twice(long value) { return value * 2; }
int main(void) {
    int out = 0;
    long b = 0x100000005L;
    unsigned long u = 0xfedcba9876543210UL, v = 12345UL;
    if(cosmic_widths() != 1160UL) return 1;
    if(cosmic_mixed(-9,b,&out,3,-4,5,-6,7) !=
       -9+b+3-4+5-6+7 + b*3+13+host_global+11 || out != -12) return 2;
    if(cosmic_array(cosmic_table,3) != 0x100000001L-7+99) return 3;
    cosmic_table[1] = -100;
    if(cosmic_array(cosmic_table,3) != 0x100000001L-100+99) return 4;
    if(cosmic_indirect(twice,b) != b*2+7) return 5;
    if(cosmic_signed(-0x100000005L,37) != -0x100000005L/37 + -0x100000005L%37) return 6;
    if(cosmic_unsigned(u,v) != ((u/v)^(u%v)^(u>>37))) return 7;
    if(cosmic_narrow_sum(-120,-300,250,60000) != 59830) return 8;
    if(cosmic_char_return(-127) != -127 || cosmic_short_return(-30000) != -30000) return 9;
    if(cosmic_narrow_returns() != 59850) return 10;
    if(cosmic_double_sum(0.5,1,2,3,4,5,6,7,8) != 36.5) return 11;
    if(cosmic_double_mix(-3,0.5,9,1,2,3,4,5,6,7,8) != 43.0) return 12;
    puts("PASS amd64 LP64 native interoperability and separate translation units");
    return 0;
}
