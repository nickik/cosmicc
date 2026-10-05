int main(void) {
 unsigned char a[4]={128,255,127,1}; unsigned long hold=0; unsigned bits=0;
 unsigned char *p=a; int mode=0, sum=0;
 hold |= (unsigned long)(*p++) << bits; bits+=8;
 hold |= (unsigned long)(*p++) << bits; bits+=8;
 if (hold != 65408UL || p-a != 2) return 1;
 switch(mode) { case 0: sum++; case 1: sum+=2; case 2: sum+=4; break; default: return 2; }
 if(sum!=7) return 3;
 mode=1; switch(mode) {case 0: return 4; case 1: if(a[2]==127) sum++; else sum+=99; case 2: sum+=4; break; default: return 5;}
 if(sum!=12) return 6;
 if ((unsigned int)0x80000000UL >> 31 != 1) return 7;
 if ((unsigned char)(hold >> 8) != 255) return 8;
 return 0;
}
