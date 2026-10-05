/* Actual C binary64 and cross-width conversions; no softfloat API calls. */
double global64=1.5;
double negative_zero64=-0.0;
struct Pair64 {double a,b;};
double relay64(double x){return x+global64;}
double sum64(double a,double b,double c,double d,double e,double f,double g,double h){return a+b+c+d+e+f+g+h;}
double mix64(int a,double b,int c,double d,int e,double f,int g,double h){return a+b+c+d+e+f+g+h;}
long long from_double(double x){return (long long)x;}
unsigned long long from_float(float x){return (unsigned long long)x;}
int main(void){
 double a=2.5,b=1.5;
 double (*fp)(double)=relay64;
 double (*many)(double,double,double,double,double,double,double,double)=sum64;
 double values[2]={a,b};struct Pair64 p={a,b};
 if(a+b!=4.0 || a-b!=1.0 || a*b!=3.75 || a/b<1.6 || a/b>1.7)return 1;
 if(!(a>b) || !(b<a) || !(a>=a) || !(a<=a) || a==b || !(a!=b))return 2;
 if(fp(a)!=4.0 || many(1.0,2.0,3.0,4.0,5.0,6.0,7.0,8.0)!=36.0)return 3;
 if(mix64(1,2.0,3,4.0,5,6.0,7,8.0)!=36.0)return 4;
 if(values[0]!=p.a || values[1]!=p.b)return 5;
 values[1]+=0.5; if(values[1]!=2.0)return 6;
 double old=a++; if(old!=2.5 || a!=3.5)return 7;
 --a; if(a!=2.5)return 8;
 if((int)a!=2 || (unsigned)a!=2U || (double)-3!=-3.0 || (double)4000000000U!=4000000000.0)return 9;
 if((unsigned char)255.0!=255 || (short)-32000.0!=-32000)return 10;
 long long signed_value=-4294967299LL;
 unsigned long long high=9223372036854775808ULL;
 if((double)signed_value!=-4294967299.0 || from_double((double)signed_value)!=signed_value)return 11;
 if((double)high!=9223372036854775808.0 || (unsigned long long)(double)high!=high)return 12;
 if((float)signed_value!=-4294967296.0f || (long long)(float)signed_value!=-4294967296LL)return 13;
 if((float)high!=9223372036854775808.0f || from_float((float)high)!=high)return 14;
 float narrow=(float)a; if(narrow!=2.5f || (double)narrow!=a)return 15;
 union Bits32 {float f;unsigned u;} rounded;rounded.f=(float)1.0000000596046448;
 if(rounded.u!=0x3f800000U)return 16;
 union Bits64 {double f;unsigned long long u;} bits;bits.f=negative_zero64;
 if(bits.u!=0x8000000000000000ULL || negative_zero64 || (_Bool)negative_zero64)return 17;
 double nan=0.0/0.0;
 if(!nan || !(_Bool)nan || nan==nan || !(nan!=nan) || nan<0.0 || nan>=0.0)return 18;
 double inf=1.0/0.0;if(!(inf>1.0e300))return 19;
 double *pointer=&values[0];*pointer=7.0;if(values[0]!=7.0)return 20;
 return 0;
}
