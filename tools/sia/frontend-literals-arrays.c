#include <stdint.h>
#include <wchar.h>
#define CAT2(a,b) a##b
#define CAT(a,b) CAT2(a,b)
#define AB(x) CAT(x,y)
int xy=42;
union F64 { double value; unsigned long long bits; } minimum64={0x1p-1074};
union F32 { float value; unsigned int bits; } minimum32={0x1p-149f};
union F64 infinity64={__builtin_inf()};
union F32 nan32={__builtin_nanf("")};
wchar_t text[]=L"hello$$你好¢¢世界€€world";
wchar_t *literal=L"a'b\u20ac\U0001F600";
struct Entry {char name[3];int n;};
struct Entry entries[] = {"abc", 4, "xy", 7};
char terminated[3]={"ab"};
int main(void) {
    struct Entry local={"pq",9};
    wchar_t wide[3]={L"xy"};
    char exact[3]={"abc"};
    int32_t x=~0;
    if(x!=0xffffffff || x!=0xffffffffffffffff) return 1;
    if(CAT(A,B)(x)!=42) return 2;
    if(entries->n!=4 || entries[1].n!=7 || sizeof(entries)!=16) return 3;
    if(entries[0].name[2]!='c' || local.name[0]!='p' || terminated[2]!=0 || exact[2]!='c') return 4;
    if(sizeof(text)!=84 || text[7]!=0x4f60 || text[8]!=0x597d || text[13]!=0x20ac) return 5;
    if(literal[1]!='\'' || literal[3]!=0x20ac || literal[4]!=0x1f600 || literal[5]!=0) return 6;
    if(wide[0]!='x' || wide[2]!=0) return 7;
    if(minimum64.bits!=1 || minimum32.bits!=1) return 8;
    if(infinity64.bits!=0x7ff0000000000000ULL || (nan32.bits&0x7fc00000U)!=0x7fc00000U) return 9;
    return 0;
}
