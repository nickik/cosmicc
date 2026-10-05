/* Test stored _Bool conversion after arithmetic, plus old expression values. */
_Bool global;
_Bool elements[2];
struct State { _Bool field; int sentinel; } state={0,77};
_Bool side_values[2];
int visits;
_Bool *pick(void) { return &side_values[visits++]; }
int cycle(_Bool *p) {
    if((int)(*p)++!=0 || (int)*p!=1) return 1;
    if((int)(*p)++!=1 || (int)*p!=1) return 2;
    if((int)(*p)--!=1 || (int)*p!=0) return 3;
    if((int)(*p)--!=0 || (int)*p!=1) return 4;
    if(*(unsigned char *)p!=1) return 5;
    return 0;
}
int main(void) {
    _Bool local=0, escaped=0;
    int i;
    if((int)local++!=0 || (int)local!=1) return 10;
    if((int)local++!=1 || (int)local!=1) return 11;
    if((int)local--!=1 || (int)local!=0) return 12;
    if((int)local--!=0 || (int)local!=1) return 13;
    for(i=0;i<20;i++) { local--; if((int)local!=(i%2)) return 14; }
    if(cycle(&escaped)) return 20;
    if(cycle(&global)) return 21;
    if(cycle(&elements[1])) return 22;
    if(cycle(&state.field) || state.sentinel!=77) return 23;
    if(elements[0]) return 24;
    if((int)(*pick())++!=0 || visits!=1 || (int)side_values[0]!=1 || side_values[1]) return 25;
    if((int)(*pick())++!=0 || visits!=2 || (int)side_values[1]!=1) return 26;
    return 0;
}
