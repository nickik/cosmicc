#include <stdarg.h>
#include <stdio.h>
static volatile unsigned char vectors_al __attribute__((used))=255,fixed_al __attribute__((used))=255;
static volatile unsigned char integer_al __attribute__((used))=255,mixed_al __attribute__((used))=255;
/* Capture at the exact public ABI entry before any compiler-generated code. */
__asm__(".text\n.globl host_integer\nhost_integer:\nmovb %al, integer_al(%rip)\njmp host_integer_impl\n.globl host_mixed\nhost_mixed:\nmovb %al, mixed_al(%rip)\njmp host_mixed_impl\n");
long host_integer_impl(int tag,...) {
    va_list ap; int *marker; int bad=tag || integer_al!=0;
    va_start(ap,tag);
    bad |= va_arg(ap,int)!=-7;
    bad |= va_arg(ap,int)!=255;
    bad |= va_arg(ap,int)!=-32767;
    bad |= va_arg(ap,int)!=65535;
    bad |= va_arg(ap,unsigned int)!=0xfedcba98u;
    bad |= va_arg(ap,long)!=-4294967297L;
    marker=va_arg(ap,int *); bad |= *marker!=17; *marker=29;
    bad |= va_arg(ap,int)!=9; bad |= va_arg(ap,int)!=10;
    va_end(ap); return bad;
}
long host_mixed_impl(int count,...) {
    va_list ap; int i; int bad=count!=9 || mixed_al!=8;
    va_start(ap,count);
    for(i=0;i<count;i++) {
        bad |= va_arg(ap,long)!=-4294967297L+i;
        bad |= va_arg(ap,double)!=(double)i+1.5;
    }
    va_end(ap); return bad;
}
__asm__(".text\n.globl host_vectors\nhost_vectors:\nmovb %al, vectors_al(%rip)\njmp host_vectors_impl\n.globl host_fixed\nhost_fixed:\nmovb %al, fixed_al(%rip)\njmp host_fixed_impl\n");
long host_vectors_impl(int count,...) {
    va_list ap; int i; int bad=vectors_al!=(count<8?count:8);
    va_start(ap,count);
    for(i=0;i<count;i++) bad |= va_arg(ap,double)!=(double)i+1.5;
    va_end(ap); return bad;
}
long host_fixed_impl(float fixed,int tag,...) {
    va_list ap; int bad=fixed_al!=2 || fixed!=1.25f || tag!=3;
    va_start(ap,tag); bad |= va_arg(ap,double)!=2.5; bad |= va_arg(ap,long)!=-4294967297L;
    va_end(ap); return bad;
}
extern long host_mixed(int,...);
extern long cosmic_variadic_calls(long (*)(int,...),long);
extern long cosmic_named(int,double,...);
extern double cosmic_named_float(float,int,...);
int main(void) {
    long result=cosmic_variadic_calls(host_mixed,-4294967297L);
    if(cosmic_named(7,1.5,-3,4L,5.5,6L,7.5,8L,9.5,10L,11.5,12L,13.5,14L,15.5,16L,17.5,18L,19.5)!=8) return 20;
    if(cosmic_named_float(1.25f,3,4.5,6L,7.5,8L,9.5,10L,11.5,12L,13.5,14L,15.5,16L,17.5,18L,19.5)!=4.25) return 21;
    if(result) fprintf(stderr,"variadic ABI failure %ld (ALinteger=%u ALmixed=%u)\n",result,integer_al,mixed_al);
    return (int)result;
}
