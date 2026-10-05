/* Original exact-rational strtof/strtod parser, fixed C locale. */
#include "../softfloat/binary32.h"
#include "bigint.h"
#define CD_DIGITS 1200
int errno;
static int cd_space(int c){return c==' '||c=='\t'||c=='\n'||c=='\r'||c=='\f'||c=='\v';}
static int cd_digit(int c){if(c>='0'&&c<='9')return c-'0';if(c>='a'&&c<='f')return c-'a'+10;if(c>='A'&&c<='F')return c-'A'+10;return -1;}
static int cd_lower(int c){return c>='A'&&c<='Z'?c+32:c;}
static int cd_word(const char *s,const char *word){while(*word){if(cd_lower(*s++)!=*word++)return 0;}return 1;}
static long long cd_exp(const char **p,int marker){
    const char *s=*p,*q;long long e=0;int sign=1;
    if(cd_lower(*s)!=marker)return 0;q=s+1;if(*q=='+'||*q=='-'){if(*q=='-')sign=-1;++q;}
    if(*q<'0'||*q>'9')return 0;
    do{if(e<10000000000LL)e=e*10+(*q-'0');++q;}while(*q>='0'&&*q<='9');*p=q;return sign*e;
}
unsigned long long cosmic_decimal_parse_bits(const char *text,char **end,int narrow,cosmic_sf_context *ctx){
    struct cd_big num,den,copy; cosmic_sf_context operation;const char *s=text,*digits,*q;int sign=0,base=10,seen=0,dot=0,started=0,kept=0,sticky=0,d,i,e,scale;
    long long frac=0,dropped=0,exp,order;unsigned long long sig,result;
    while(cd_space(*s))++s;if(*s=='+'||*s=='-'){sign=*s=='-';++s;}
    if(cd_word(s,"inf")){s+=3;if(cd_word(s,"inity"))s+=5;if(end)*end=(char *)s;return narrow?((unsigned long long)sign<<31)|0x7f800000ULL:((unsigned long long)sign<<63)|0x7ff0000000000000ULL;}
    if(cd_word(s,"nan")){s+=3;if(*s=='('){q=s+1;while((*q>='0'&&*q<='9')||(*q>='a'&&*q<='z')||(*q>='A'&&*q<='Z')||*q=='_')++q;if(*q==')')s=q+1;}if(end)*end=(char *)s;return narrow?((unsigned long long)sign<<31)|0x7fc00000ULL:((unsigned long long)sign<<63)|0x7ff8000000000000ULL;}
    digits=s;cd_zero(&num);cd_u64(&den,1);
    if(s[0]=='0'&&(s[1]=='x'||s[1]=='X')){q=s+2;if(*q=='.')++q;if(cd_digit(*q)>=0){base=16;s+=2;}}
    while(1){d=cd_digit(*s);if(d>=0&&d<base){seen=1;if(dot)++frac;if(d||started){started=1;if(kept<CD_DIGITS){cd_mul(&num,(unsigned int)base);cd_add(&num,(unsigned int)d);++kept;}else{++dropped;if(d)sticky=1;}}++s;}else if(*s=='.'&&!dot){dot=1;++s;}else break;}
    if(!seen){if(end)*end=(char *)text;return 0;}
    exp=cd_exp(&s,base==16?'p':'e');if(end)*end=(char *)s;
    if(cd_empty(&num))return narrow?(unsigned long long)sign<<31:(unsigned long long)sign<<63;
    exp+=(dropped-frac)*(base==16?4:1);
    order=base==16?(long long)cd_bits(&num)+exp:kept+exp;
    if(order>(base==16?2000:400)){e=2000;cd_u64(&num,1);}
    else if(order<(base==16?-2000:-400)){e=-2000;cd_u64(&num,1);}
    else{
        if(exp>0){if(base==16)cd_shift(&num,(int)exp);else for(i=0;i<(int)exp;i++)cd_mul(&num,10);}
        else if(exp<0){if(base==16)cd_shift(&den,(int)-exp);else for(i=0;i<(int)-exp;i++)cd_mul(&den,10);}
        e=cd_bits(&num)-cd_bits(&den);copy=num;
        if(e>=0){copy=den;cd_shift(&copy,e);if(cd_cmp(&num,&copy)<0)--e;}
        else{cd_shift(&copy,-e);if(cd_cmp(&copy,&den)<0)--e;}
    }
    if(e>1100 || e<-1200){sig=narrow?1ULL<<30:1ULL<<62;}
    else{scale=(narrow?30:62)-e;if(scale>=0)cd_shift(&num,scale);else cd_shift(&den,-scale);sig=cd_quotient(&num,&den);if(!cd_empty(&num)||sticky)sig|=1;}
    cosmic_sf_init(&operation);operation.rounding=ctx->rounding;
    result=narrow?cosmic_sf_pack32(&operation,sign,e+126,(unsigned int)sig):cosmic_sf_pack64(&operation,sign,e+1022,sig);
    ctx->flags |= operation.flags;
    if(operation.flags&(COSMIC_SF_UNDERFLOW|COSMIC_SF_OVERFLOW))errno=34;
    return result;
}
float strtof(const char *text,char **end){union{float f;unsigned int u;}v;v.u=(unsigned int)cosmic_decimal_parse_bits(text,end,1,cosmic_sf_current_context());return v.f;}
double strtod(const char *text,char **end){union{double f;unsigned long long u;}v;v.u=cosmic_decimal_parse_bits(text,end,0,cosmic_sf_current_context());return v.f;}
long double strtold(const char *text,char **end){return (long double)strtod(text,end);}
double atof(const char *text){return strtod(text,(char **)0);}
