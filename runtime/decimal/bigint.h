/* Original bounded integer arithmetic for IEEE decimal text conversion.
 * 8192 bits cover binary64 exact decimals and the 1200 retained input digits.
 * No allocation and no host floating point. */
#ifndef COSMIC_DECIMAL_BIGINT_H
#define COSMIC_DECIMAL_BIGINT_H
#define CD_WORDS 256
struct cd_big { unsigned int w[CD_WORDS]; int n; };
static void cd_zero(struct cd_big *a) { a->n=1; a->w[0]=0; }
static void cd_u64(struct cd_big *a,unsigned long long v) {
    a->w[0]=(unsigned int)v; a->w[1]=(unsigned int)(v>>32);a->n=a->w[1]?2:1;
}
static int cd_empty(const struct cd_big *a) { return a->n==1 && a->w[0]==0; }
static void cd_trim(struct cd_big *a) {while(a->n>1 && !a->w[a->n-1])--a->n;}
static void cd_mul(struct cd_big *a,unsigned int b) {
    unsigned int carry=0,lo,hi;int i;
    for(i=0;i<a->n;i++){lo=(a->w[i]&65535u)*b+carry;hi=(a->w[i]>>16)*b+(lo>>16);a->w[i]=(lo&65535u)|(hi<<16);carry=hi>>16;}
    if(carry){a->w[a->n]=carry;++a->n;}
}
static void cd_add(struct cd_big *a,unsigned int b) {
    unsigned int old;int i=0;
    do{if(i==a->n){a->w[i]=0;++a->n;}old=a->w[i];a->w[i]+=b;b=a->w[i]<old;++i;}while(b);
}
static int cd_cmp(const struct cd_big *a,const struct cd_big *b) {
    int i;if(a->n!=b->n)return a->n>b->n?1:-1;
    for(i=a->n-1;i>=0;--i)if(a->w[i]!=b->w[i])return a->w[i]>b->w[i]?1:-1;
    return 0;
}
static void cd_sub(struct cd_big *a,const struct cd_big *b) {
    unsigned int old,word,borrow=0,next;int i;
    for(i=0;i<a->n;i++){word=i<b->n?b->w[i]:0;old=a->w[i];next=old<word || (borrow && old==word);a->w[i]=old-word-borrow;borrow=next;}cd_trim(a);
}
static int cd_bits(const struct cd_big *a) {
    int r=(a->n-1)*32;unsigned int top=a->w[a->n-1];while(top){++r;top>>=1;}return r;
}
static void cd_shift(struct cd_big *a,int bits) {
    int words=bits/32,part=bits%32,i;unsigned int carry=0,next;
    if(cd_empty(a))return;
    for(i=a->n-1;i>=0;--i)a->w[i+words]=a->w[i];
    for(i=0;i<words;i++)a->w[i]=0;a->n+=words;
    if(part){for(i=words;i<a->n;i++){next=a->w[i]>>(32-part);a->w[i]=(a->w[i]<<part)|carry;carry=next;}if(carry){a->w[a->n]=carry;++a->n;}}
}
static void cd_right1(struct cd_big *a) {
    unsigned int carry=0,next;int i;for(i=a->n-1;i>=0;--i){next=a->w[i]<<31;a->w[i]=(a->w[i]>>1)|carry;carry=next;}cd_trim(a);
}
static unsigned long long cd_quotient(struct cd_big *a,const struct cd_big *b) {
    struct cd_big shifted;unsigned long long q=0;int shift=cd_bits(a)-cd_bits(b),i;
    if(shift<0)return 0;shifted=*b;cd_shift(&shifted,shift);
    for(i=shift;i>=0;--i){if(cd_cmp(a,&shifted)>=0){cd_sub(a,&shifted);q|=1ULL<<i;}cd_right1(&shifted);}return q;
}
static unsigned int cd_div_small(struct cd_big *a,unsigned int divisor) {
    unsigned int rem=0,hi,lo,v;int i;
    for(i=a->n-1;i>=0;--i){v=(rem<<16)|(a->w[i]>>16);hi=v/divisor;rem=v%divisor;v=(rem<<16)|(a->w[i]&65535u);lo=v/divisor;rem=v%divisor;a->w[i]=(hi<<16)|lo;}cd_trim(a);return rem;
}
#endif
