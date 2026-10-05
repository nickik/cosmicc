#include "zlib.h"
static unsigned int used;
static voidpf allocate(voidpf opaque, uInt count, uInt size) {
    unsigned int n, i;
    unsigned char *p;
    (void)opaque;
    if(size && count > 0x50000u/size) return 0;
    n=count*size;
    if(n>0x50000u-used) return 0;
    p=(unsigned char *)(0x80000u+used);
    used+=(n+7u)&~7u;
    for(i=0;i<n;i++) p[i]=0;
    return p;
}
static void release(voidpf opaque, voidpf p) { (void)opaque; (void)p; }
int main(void) {
    z_stream s={0};
    unsigned char input[31],packed[128],output[31];
    unsigned int i, packed_size;
    int rc;
    for(i=0;i<31;i++) input[i]=(unsigned char)(i*37u);
    s.zalloc=allocate; s.zfree=release;
    rc=deflateInit2(&s,1,Z_DEFLATED,9,1,Z_DEFAULT_STRATEGY); if(rc!=Z_OK) return 10;
    s.next_in=input; s.avail_in=31; s.next_out=packed; s.avail_out=128;
    rc=deflate(&s,Z_FINISH); if(rc!=Z_STREAM_END) return 11;
    packed_size=(unsigned int)s.total_out;
    if(deflateEnd(&s)!=Z_OK) return 12;
    used=0;
    s.total_in=0; s.total_out=0; s.state=0;
    if(inflateInit(&s)!=Z_OK) return 13;
    s.next_in=packed; s.avail_in=packed_size; s.next_out=output; s.avail_out=31;
    if(inflate(&s,Z_FINISH)!=Z_STREAM_END) return 14;
    if(s.total_out!=31) return 15;
    if(inflateEnd(&s)!=Z_OK) return 16;
    for(i=0;i<31;i++) if(input[i]!=output[i]) return 17;
    return 0;
}
