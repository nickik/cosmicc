struct Entry { unsigned short count; unsigned short code; };
static struct Entry table[256];
int main(void) {
 unsigned char index; unsigned short wide;
 for(wide=0;wide<256;wide++) {index=(unsigned char)wide;table[index].count=(unsigned short)(wide+1);}
 for(wide=0;wide<256;wide++) if(table[wide].count!=wide+1)return 1;
 index=255; if(&table[index]-table!=255)return 2;
 signed char negative=-1; struct Entry *p=table+1;
 if(p+negative!=table)return 3;
 return 0;
}
