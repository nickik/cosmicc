/* Defined integer-only ABI stress; return zero iff every check succeeds. */
typedef unsigned long long U;
U mix(int a,U b,int c,U d,int e,U f,int g,U h,int i,U j,int k,U l,int m,U n,int o,U p) {
 return b+2*d+3*f+4*h+5*j+6*l+7*n+8*p+(U)(a+c+e+g+i+k+m+o);
}
U many(U a,U b,U c,U d,U e,U f,U g,U h,U i,U j,U k,U l) {
 return a+2*b+3*c+4*d+5*e+6*f+7*g+8*h+9*i+10*j+11*k+12*l;
}
U bump(U x){return x+0x100000003ULL;}
U nested(U x){return bump(bump(x));}
U live(U x){
 U a=x+1,b=x+2,c=x+3,d=x+4,e=x+5,f=x+6,g=x+7,h=x+8;
 U i=x+9,j=x+10,k=x+11,l=x+12,m=x+13,n=x+14,o=x+15,p=x+16;
 U r=nested(x);
 return a+b+c+d+e+f+g+h+i+j+k+l+m+n+o+p+r;
}
long long signed_boundary(int a,int b,int c,int d,int e,long long f,int g,long long h){return f+h+a+b+c+d+e+g;}
int main(void){
 U x=0x1234567800000000ULL;
 U (*fp)(U,U,U,U,U,U,U,U,U,U,U,U)=many;
 if(many(x+1,x+2,x+3,x+4,x+5,x+6,x+7,x+8,x+9,x+10,x+11,x+12)!=78*x+650) return 1;
 if(fp(x+1,x+2,x+3,x+4,x+5,x+6,x+7,x+8,x+9,x+10,x+11,x+12)!=78*x+650) return 2;
 if(mix(1,x+1,2,x+2,3,x+3,4,x+4,5,x+5,6,x+6,7,x+7,8,x+8)!=36*x+240) return 3;
 if(live(x)!=17*x+136+2*0x100000003ULL) return 4;
 if(signed_boundary(1,2,3,4,5,-0x1234567800000000LL,6,0x1234567800000010LL)!=37) return 5;
 return 0;
}
