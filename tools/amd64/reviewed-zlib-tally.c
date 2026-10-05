struct S {unsigned char b[9]; unsigned next;};
int main(void) {struct S s={{0},0}; unsigned short dist=1025; unsigned char len=255;
s.b[s.next++]=(unsigned char)dist; s.b[s.next++]=(unsigned char)(dist>>8); s.b[s.next++]=len;
if(s.next!=3||s.b[0]!=1||s.b[1]!=4||s.b[2]!=255)return 1;
unsigned char a[12]={1,2,3,4,5,6,7,8,9,10,11,12}; unsigned char b[12]={1,2,3,4,5,99,7,8,9,10,11,12};
unsigned char *p=a,*q=b; do {}while(*++p==*++q && *++p==*++q && *++p==*++q && *++p==*++q && *++p==*++q && *++p==*++q && *++p==*++q && *++p==*++q);
return p-a!=5||q-b!=5;
}
