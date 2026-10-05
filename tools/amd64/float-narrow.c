signed char sc(double x) { return x; }
unsigned char uc(float x) { return x; }
short ss(double x) { short local=x; return local; }
unsigned short us(double x) { return (unsigned short)x; }
int consume(signed char x) { return x; }
int main(void) {
    double values[5]={-127.9,-1.75,-0.5,0.0,126.99};
    int expected[5]={-127,-1,0,0,126};
    int i;
    for(i=0;i<5;i++) if((int)sc(values[i])!=expected[i]) return 1+i;
    if((unsigned)uc(255.75f)!=255) return 10;
    if((unsigned)uc(-0.75f)!=0) return 11;
    if((int)ss(-32767.9)!=-32767) return 12;
    if((int)ss(32767.9)!=32767) return 13;
    if((unsigned)us(65535.75)!=65535) return 14;
    if((unsigned)us(-0.75)!=0) return 15;
    if(consume(99.75)!=99) return 16;
    return 0;
}
