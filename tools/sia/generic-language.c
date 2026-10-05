/* C11 selection semantics; no headers, I/O, or host callbacks. */
int identity(int x) { return x; }
int main(void) {
    char c=0; signed char s=0; unsigned char u=0;
    unsigned short narrow=0;
    const int ci=0; int value=0; int * const p=&value; const int *q=&ci;
    int array[4]; int calls=0;
    if (sizeof('a')!=4 || _Generic('a',int:1,default:0)!=1) return 1;
    if (_Generic(c,char:1,signed char:2,unsigned char:3)!=1) return 2;
    if (_Generic(s,char:1,signed char:2,unsigned char:3)!=2) return 3;
    if (_Generic(u,char:1,signed char:2,unsigned char:3)!=3) return 4;
    if (_Generic(narrow,unsigned short:1,int:0)!=1) return 5;
    if (_Generic(ci,int:1,const int:0)!=1) return 6;
    if (_Generic(p,int*:1,int*const:0)!=1) return 7;
    if (_Generic(q,int*:0,const int*:1)!=1) return 8;
    if (_Generic(array,int*:1,int[4]:0)!=1) return 9;
    if (_Generic("abc",char*:1,const char*:0)!=1) return 10;
    if (_Generic(identity,int(*)(int):identity,default:identity)(12)!=12) return 11;
    if (_Generic(++calls,int:7,default:++calls)!=7 || calls!=0) return 12;
    if (_Generic(ci,int:++calls,default:++calls)!=1 || calls!=1) return 13;
    _Generic(ci,int:value,default:ci)=19;
    if(value!=19) return 14;
    struct A {int x;}; struct B {int x;}; struct A a;
    if(_Generic(a,struct A:1,struct B:2)!=1) return 15;
    return 0;
}
