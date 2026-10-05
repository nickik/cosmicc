unsigned int convert(double x) { return (unsigned int)x; }
unsigned long convert_long(double x) { return (unsigned long)x; }
unsigned int implicit(double x) { return x; }
unsigned long implicit_long(double x) { return x; }
unsigned int identity(unsigned int x) {return x;}
int main(void) { unsigned int y=3000000000.0; unsigned long z=9223372036854775808.0; return convert(3000000000.0)!=3000000000U || convert_long(9223372036854775808.0)!=9223372036854775808UL || implicit(3000000000.0)!=3000000000U || implicit_long(9223372036854775808.0)!=9223372036854775808UL || identity(3000000000.0)!=3000000000U || y!=3000000000U || z!=9223372036854775808UL; }
