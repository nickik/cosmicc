#include <stdio.h>
struct Big { long x[4]; double f; unsigned char tail; };
extern struct Big cosmic_return(struct Big,int);
extern struct Big cosmic_pressure(long,long,long,long,long,long,long,struct Big);
extern struct Big cosmic_variadic(int,...);
extern struct Big cosmic_callback(struct Big,struct Big(*)(struct Big,int));
struct Big host_return(struct Big x,int n){x.x[3]+=n;return x;}
int main(void){
 struct Big a={{1,2,3,4},1.5,7},b;
 struct Big(*p)(struct Big,int)=cosmic_return;
 b=p(a,9);if(b.x[0]!=10||b.f!=3||b.tail!=8||a.x[0]!=1||a.f!=1.5)return 1;
 b=cosmic_pressure(1,2,3,4,5,6,7,a);if(b.x[1]!=30||a.x[1]!=2)return 2;
 b=cosmic_variadic(2,100L,a);if(b.x[2]!=105||a.x[2]!=3)return 3;
 b=cosmic_callback(a,cosmic_return);if(b.x[0]!=4||b.x[3]!=9||b.f!=3||b.tail!=8)return 4;
 puts("aggregate-return-pass");return 0;
}
