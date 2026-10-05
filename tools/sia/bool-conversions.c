/* Full scalar truth conversion; high-only bits cannot be truncated away. */
_Bool from32(unsigned x){return (_Bool)x;}
_Bool from64(unsigned long long x){return (_Bool)x;}
_Bool from_pointer(int *p){return (_Bool)p;}
int main(void){
 unsigned word=256; unsigned long long pair=1ULL<<51;
 _Bool a=word,b=pair;
 if(!a || !b || !from32(word) || !from64(pair))return 1;
 if(from32(0) || from64(0) || from_pointer((int*)0))return 2;
 /* Converting an integer to a pointer is implementation-defined; the SIA
    target preserves the address bits. No dereference is performed. */
 if(!from_pointer((int*)256))return 3;
 if(!(pair&&word) || !(pair||0) || (pair&&0))return 4;
 return 0;
}
