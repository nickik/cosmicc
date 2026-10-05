struct M { long a; double d; long v[2]; };
struct Odd { unsigned char b[17]; };
extern long host_take(int,struct M,long,double,struct Odd,struct M,long);
extern long host_va(double,int,...);
long cosmic_take(int tag,struct M m,double d,struct Odd odd,struct M n,long x,struct M *original) {
    if(tag!=13 || m.a!=7 || m.d!=2.5 || m.v[0]!=11 || m.v[1]!=19 || d!=4.5 || odd.b[0]!=3 || odd.b[16]!=251 || n.a!=17 || n.v[1]!=23 || x!=-4294967297L) return 1;
    m.a=99; m.v[1]=101; odd.b[16]=0; n.a=33;
    if(original->a!=7 || original->v[1]!=19) return 2;
    return 0;
}
long cosmic_calls(long (*callback)(int,struct M,long,double,struct Odd,struct M,long)) {
    struct M m={7,2.5,{11,19}}, n={17,3.5,{21,23}};
    struct Odd odd={{3}};
    odd.b[16]=251;
    if(host_take(13,m,-4294967297L,4.5,odd,n,29)) return 10;
    if(m.a!=7 || m.v[1]!=19 || odd.b[16]!=251 || n.a!=17) return 11;
    if(callback(13,m,-4294967297L,4.5,odd,n,29)) return 12;
    if(m.a!=7 || m.v[1]!=19 || odd.b[16]!=251 || n.a!=17) return 13;
    if(host_va(1.25,13,m,-4294967297L,odd,4.5,n,29)) return 14;
    if(m.a!=7 || m.v[1]!=19 || odd.b[16]!=251 || n.a!=17) return 15;
    return 0;
}
