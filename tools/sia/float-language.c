/* Actual C binary32 operators; no raw-bit softfloat API calls here. */
float global=1.5f;
float negative_zero=-0.0f;
float folded=(16777216.0f+1.0f)-16777216.0f;
float rounded=1.000000059604644775390625000001f;
struct Pair {float a,b;};
float relay(float x){return x+global;}
float many(float a,float b,float c,float d,float e,float f,float g,float h){return a+b+c+d+e+f+g+h;}
int main(void){
 float a=2.5f,b=1.5f;
 float (*fp)(float)=relay;
 float values[2]={a,b};struct Pair p={a,b};
 if(a+b!=4.0f || a-b!=1.0f || a*b!=3.75f || a/b<1.6f || a/b>1.7f)return 1;
 if(!(a>b) || !(b<a) || !(a>=a) || !(a<=a) || a==b || !(a!=b))return 2;
 if(fp(a)!=4.0f || many(1.0f,2.0f,3.0f,4.0f,5.0f,6.0f,7.0f,8.0f)!=36.0f)return 3;
 if(values[0]!=p.a || values[1]!=p.b)return 4;
 values[1]+=0.5f; if(values[1]!=2.0f)return 5;
 float old=a++; if(old!=2.5f || a!=3.5f)return 6;
 --a; if(a!=2.5f)return 7;
 if((int)a!=2 || (unsigned)a!=2U || (float)-3!=-3.0f || (float)4000000000U<3999999000.0f)return 8;
 if((unsigned char)255.0f!=255 || (short)-32000.0f!=-32000)return 9;
 if(negative_zero || !(-negative_zero==0.0f) || (_Bool)negative_zero)return 10;
 float nan=0.0f/0.0f;
 if(!nan || !(_Bool)nan || nan==nan || !(nan!=nan) || nan<0.0f || nan>=0.0f)return 11;
 float inf=1.0f/0.0f; if(!(inf>1000.0f))return 12;
 union Bits {float f;unsigned u;} bits;bits.f=negative_zero;
 if(bits.u!=0x80000000U || folded!=0.0f) return 13;
 bits.f=rounded;if(bits.u!=0x3f800001U)return 14;
 float *pointer=&values[0];*pointer=7.0f;if(values[0]!=7.0f)return 15;
 return 0;
}
