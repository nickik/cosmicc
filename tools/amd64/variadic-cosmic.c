/* Cosmic emits these callers; GCC/Clang emit the stdarg callees and harness. */
extern long host_integer(int,...);
extern long host_mixed(int,...);
extern long host_vectors(int,...);
extern long host_fixed(float,int,...);
extern int snprintf(char *, unsigned long, const char *,...);
extern int strcmp(const char *,const char *);
long cosmic_variadic_calls(long (*callback)(int,...), long seed) {
    int marker=17;
    unsigned char uc=255;
    signed char sc=-7;
    unsigned short us=65535;
    signed short ss=-32767;
    float fp=1.5f;
    char text[128];
    if(host_integer(0,sc,uc,ss,us,0xfedcba98u,seed,&marker,9,10)!=0) return 1;
    if(marker!=29) return 2;
    if(host_mixed(9,seed,fp,seed+1,2.5,seed+2,3.5,seed+3,4.5,seed+4,5.5,seed+5,6.5,seed+6,7.5,seed+7,8.5,seed+8,9.5)!=0) return 3;
    if(callback(9,seed,fp,seed+1,2.5,seed+2,3.5,seed+3,4.5,seed+4,5.5,seed+5,6.5,seed+6,7.5,seed+7,8.5,seed+8,9.5)!=0) return 4;
    if(snprintf(text,sizeof(text),"%d/%u/%ld/%.1f",sc,(unsigned)uc,seed,fp)!=22) return 5;
    if(strcmp(text,"-7/255/-4294967297/1.5")) return 6;
    if(host_vectors(1,fp)) return 7;
    if(host_vectors(7,fp,2.5,3.5,4.5,5.5,6.5,7.5)) return 8;
    if(host_vectors(8,fp,2.5,3.5,4.5,5.5,6.5,7.5,8.5)) return 9;
    if(host_vectors(9,fp,2.5,3.5,4.5,5.5,6.5,7.5,8.5,9.5)) return 10;
    if(host_fixed(1.25f,3,2.5,seed)) return 11;
    return 0;
}

long cosmic_named(int n,double fixed,...) { return n+(long)fixed; }
double cosmic_named_float(float fixed,int n,...) { return fixed+n; }
