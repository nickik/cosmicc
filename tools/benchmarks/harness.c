/* System-GCC-built measurement/validation harness shared by every variant. */
#define _POSIX_C_SOURCE 200809L
#include <stdio.h>
#include <stdlib.h>
#include <time.h>
extern unsigned long bench_hash(const unsigned int *, unsigned long);
extern unsigned long bench_branch(unsigned long, unsigned long);
extern unsigned long bench_divide(unsigned long, unsigned long);
extern void bench_matrix(const int *,const int *,long *,unsigned long);
static double now(void) {
    struct timespec t;
    if(clock_gettime(CLOCK_MONOTONIC,&t)) exit(90);
    return (double)t.tv_sec+(double)t.tv_nsec*1e-9;
}
int main(int argc,char **argv) {
    unsigned long repetitions=argc>1?strtoul(argv[1],0,10):20000UL;
    unsigned int input[4096];
    int a[256],b[256];
    long c[256];
    unsigned long i,r,checksum;
    double start,elapsed;
    if(!repetitions || sizeof(unsigned long)!=8) return 91;
    for(i=0;i<4096;i++) input[i]=(unsigned int)((i*37UL)^(i>>3));
    for(i=0;i<256;i++) { a[i]=(int)(i%17)-8; b[i]=(int)(i%13)-6; }
    bench_matrix(a,b,c,16);
    checksum=1;
    for(i=0;i<256;i++) { checksum^=(unsigned long)c[i]; checksum*=1099511628211UL; }
    printf("{\"iterations\":%lu,\"validation\":{\"hash\":%lu,\"branch\":%lu,\"divide\":%lu,\"matrix\":%lu},\"kernels\":{",
        repetitions,bench_hash(input,4096),bench_branch(4096,1),bench_divide(4096,1),checksum);
    checksum=0; start=now();
    for(r=0;r<repetitions;r++) checksum+=bench_hash(input,4096);
    elapsed=now()-start;
    printf("\"hash\":{\"seconds\":%.9f,\"checksum\":%lu},",elapsed,checksum);
    checksum=0; start=now();
    for(r=0;r<repetitions;r++) checksum+=bench_branch(4096,r+1);
    elapsed=now()-start;
    printf("\"branch\":{\"seconds\":%.9f,\"checksum\":%lu},",elapsed,checksum);
    checksum=0; start=now();
    for(r=0;r<repetitions;r++) checksum+=bench_divide(4096,r+1);
    elapsed=now()-start;
    printf("\"divide\":{\"seconds\":%.9f,\"checksum\":%lu},",elapsed,checksum);
    checksum=0; start=now();
    for(r=0;r<repetitions;r++) {
        bench_matrix(a,b,c,16);
        checksum+=(unsigned long)c[r%256];
    }
    elapsed=now()-start;
    printf("\"matrix\":{\"seconds\":%.9f,\"checksum\":%lu}}}\n",elapsed,checksum);
    return 0;
}
