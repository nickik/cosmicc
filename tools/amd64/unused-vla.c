static int calls;
static int bound(int n){calls++;return n+1;}
static int probe(void){
 int k;
 for(k=0;k<3;k++){unsigned char unused[bound(k)];}
 goto label;
 {int array[2]={99,99};
 label:array[1]=39;return array[1]+calls;}
}
int main(void){return probe()==42 && calls==3 ? 0 : 1;}
