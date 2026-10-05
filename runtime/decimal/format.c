/* Original allocation-free C-locale integer decimal/hex snprintf core. */
#include <stdarg.h>
#include <stddef.h>
#include <stdint.h>
#include "../softfloat/context.h"
#include "bigint.h"
extern int errno;
struct cd_output { char *data; size_t size; unsigned long long count; };
static void cd_put(struct cd_output *o,int c){if(o->count+1<o->size)o->data[(size_t)o->count]=(char)c;++o->count;}
static void cd_repeat(struct cd_output *o,int c,int n){int emit=n;size_t room;if(n<=0)return;room=o->count<o->size?o->size-1-(size_t)o->count:0;if((unsigned long long)emit>room)emit=(int)room;while(emit>0){cd_put(o,c);--emit;--n;}o->count+=(unsigned int)n;}
struct cd_decimal { char digits[1200]; int n,point; };
static void cd_exact(struct cd_decimal *d,unsigned long long bits){
    struct cd_big a;unsigned long long sig=bits&0xfffffffffffffULL;int exp=(int)((bits>>52)&2047),i;char reverse[1200];
    if(exp){sig|=0x10000000000000ULL;exp-=1075;}else exp=-1074;
    if(!sig){d->digits[0]='0';d->n=1;d->point=1;return;}
    cd_u64(&a,sig);if(exp>=0)cd_shift(&a,exp);else for(i=0;i<-exp;i++)cd_mul(&a,5);
    d->n=0;while(!cd_empty(&a))reverse[d->n++]=(char)('0'+cd_div_small(&a,10));
    for(i=0;i<d->n;i++)d->digits[i]=reverse[d->n-1-i];d->point=d->n+(exp<0?exp:0);
}
static int cd_increment(int sign,unsigned int mode,int first,int rest,int odd){
    if(first==0 && !rest)return 0;
    if(mode==COSMIC_SF_UPWARD)return !sign;
    if(mode==COSMIC_SF_DOWNWARD)return sign;
    if(mode==COSMIC_SF_TOWARDZERO)return 0;
    return first>5 || (first==5 && (rest||odd));
}
static void cd_round(struct cd_decimal *d,int keep,int sign,unsigned int mode){
    int i,first,rest=0,inc;
    if(keep>=d->n)return;
    if(keep<0){first=0;rest=1;}else {first=d->digits[keep]-'0';for(i=keep+1;i<d->n;i++)if(d->digits[i]!='0')rest=1;}
    inc=cd_increment(sign,mode,first,rest,keep>0?((d->digits[keep-1]-'0')&1):0);
    if(keep<=0){d->digits[0]=inc?'1':'0';d->n=1;d->point=inc?d->point-keep+1:1;return;}
    d->n=keep;if(!inc)return;i=keep-1;
    while(i>=0&&d->digits[i]=='9'){d->digits[i]='0';--i;}
    if(i>=0)++d->digits[i];else {d->digits[0]='1';++d->point;}
}
static void cd_str(struct cd_output *o,const char *s,int n){int i;for(i=0;i<n;i++)cd_put(o,s[i]);}
static void cd_expout(struct cd_output *o,int exponent,int hex,int upper){
    char digits[16];unsigned int magnitude=exponent<0?(unsigned int)-exponent:(unsigned int)exponent;int n=0;
    cd_put(o,hex?(upper?'P':'p'):(upper?'E':'e'));cd_put(o,exponent<0?'-':'+');
    do{digits[n++]=(char)('0'+magnitude%10);magnitude/=10;}while(magnitude);
    if(!hex && n<2)digits[n++]='0';while(n)cd_put(o,digits[--n]);
}
static void cd_fixed(struct cd_output *o,const struct cd_decimal *d,int precision,int alternate){
    int i,n,start;
    if(d->point<=0)cd_put(o,'0');
    else{n=d->point<d->n?d->point:d->n;cd_str(o,d->digits,n);cd_repeat(o,'0',d->point-n);}
    if(precision||alternate)cd_put(o,'.');
    start=d->point;
    if(start<0){n=-start<precision?-start:precision;cd_repeat(o,'0',n);precision-=n;start=0;}
    n=d->n-start;if(n<0)n=0;if(n>precision)n=precision;
    for(i=0;i<n;i++)cd_put(o,d->digits[start+i]);cd_repeat(o,'0',precision-n);
}
static void cd_scientific(struct cd_output *o,const struct cd_decimal *d,int precision,int alternate,int upper){
    int n=d->n-1;if(n>precision)n=precision;
    cd_put(o,d->digits[0]);if(precision||alternate)cd_put(o,'.');cd_str(o,d->digits+1,n);cd_repeat(o,'0',precision-n);cd_expout(o,d->point-1,0,upper);
}
static void cd_hex(struct cd_output *o,unsigned long long bits,int precision,int alternate,int upper,unsigned int mode,int emit_prefix){
    const char *alphabet=upper?"0123456789ABCDEF":"0123456789abcdef";
    char digits[14];int exp=(int)((bits>>52)&2047),sign=(int)(bits>>63),i,first,rest=0,inc,keep;
    unsigned long long fraction=bits&0xfffffffffffffULL;
    digits[0]=exp?'1':'0';exp=exp?exp-1023:(fraction?-1022:0);
    for(i=0;i<13;i++)digits[i+1]=alphabet[(fraction>>(48-4*i))&15];
    if(precision<0){precision=13;while(precision>0&&digits[precision]=='0')--precision;}
    if(precision<13){keep=precision+1;first=(int)((fraction>>(48-4*precision))&15);for(i=keep+1;i<14;i++)if(digits[i]!='0')rest=1;
        inc=0;if(first||rest){if(mode==COSMIC_SF_UPWARD)inc=!sign;else if(mode==COSMIC_SF_DOWNWARD)inc=sign;else if(mode==COSMIC_SF_NEAREST)inc=first>8||(first==8&&(rest||((precision?((fraction>>(52-4*precision))&15):(unsigned int)(digits[0]-'0'))&1)));}
        if(inc){i=keep-1;while(i>0&&digits[i]==alphabet[15]){digits[i]='0';--i;}if(i==0)++digits[0];else{const char *p=alphabet;while(*p!=digits[i])++p;digits[i]=p[1];}}
    }
    if(emit_prefix){cd_put(o,'0');cd_put(o,upper?'X':'x');}cd_put(o,digits[0]);if(precision||alternate)cd_put(o,'.');i=precision<13?precision:13;cd_str(o,digits+1,i);cd_repeat(o,'0',precision-i);cd_expout(o,exp,1,upper);
}
static void cd_float(struct cd_output *o,unsigned long long bits,int conversion,int precision,int alternate,unsigned int mode){
    struct cd_decimal d;int upper=conversion>='A'&&conversion<='Z',kind=upper?conversion+32:conversion,exponent,sign=(int)(bits>>63);
    if(((bits>>52)&2047)==2047){cd_str(o,(bits&0xfffffffffffffULL)?(upper?"NAN":"nan"):(upper?"INF":"inf"),3);return;}
    if(kind=='a'){cd_hex(o,bits,precision,alternate,upper,mode,1);return;}
    if(precision<0)precision=6;if(kind=='g'&&!precision)precision=1;
    if(precision>2147482000){if(kind!='g'||alternate){o->count+=2147483648ULL;return;}precision=1200;}
    cd_exact(&d,bits);
    if(kind=='f'){if(precision<1200)cd_round(&d,d.point+precision,sign,mode);cd_fixed(o,&d,precision,alternate);}
    else{
        if(precision<1200)cd_round(&d,kind=='e'?precision+1:precision,sign,mode);exponent=d.point-1;
        if(kind=='g'){if(!alternate)while(d.n>1&&d.digits[d.n-1]=='0')--d.n;
            if(exponent<-4 || exponent>=precision){if(!alternate)precision=d.n-1;else --precision;cd_scientific(o,&d,precision,alternate,upper);}
            else {if(!alternate)precision=d.n-d.point;else precision-=d.point;if(precision<0)precision=0;cd_fixed(o,&d,precision,alternate);}
        }else cd_scientific(o,&d,precision,alternate,upper);
    }
}
static int cd_number(const char **p){int n=0;while(**p>='0'&&**p<='9'){if(n>214748364 || (n==214748364 && **p>'7'))return -1;n=n*10+(*(*p)++-'0');}return n;}
int vsnprintf(char *buffer,size_t size,const char *format,va_list args){
    struct cd_output out,measure;int left,plus,space,alternate,zero,width,precision,length,c,sign,prefix,n,pad,i;unsigned long long u;long long v;char digits[70],signchar;const char *s,*alphabet;union {double f;unsigned long long u;} fp;
    out.data=buffer;out.size=size;out.count=0;
    while(*format){if(*format!='%'){cd_put(&out,*format++);continue;}++format;left=plus=space=alternate=zero=0;
        while(*format=='-'||*format=='+'||*format==' '||*format=='#'||*format=='0'){c=*format++;if(c=='-')left=1;else if(c=='+')plus=1;else if(c==' ')space=1;else if(c=='#')alternate=1;else zero=1;}
        if(*format=='*'){width=va_arg(args,int);++format;if(width<0){if(width==(-2147483647-1))goto overflow;width=-width;left=1;}}
        else {width=cd_number(&format);if(width<0)goto overflow;}
        precision=-1;if(*format=='.'){++format;if(*format=='*'){precision=va_arg(args,int);++format;}else{precision=cd_number(&format);if(precision<0)goto overflow;}}
        length=0;if(*format=='h'){++format;length=1;if(*format=='h'){++format;length=2;}}else if(*format=='l'){++format;length=3;if(*format=='l'){++format;length=4;}}else if(*format=='L'){length=5;++format;}else if(*format=='j'){length=6;++format;}else if(*format=='z'){length=7;++format;}else if(*format=='t'){length=8;++format;}
        c=*format;if(!c)goto invalid;++format;
        if(c=='n'){if(length==1)*va_arg(args,short *)=(short)out.count;else if(length==2)*va_arg(args,signed char *)=(signed char)out.count;else if(length==3)*va_arg(args,long *)=(long)out.count;else if(length==4)*va_arg(args,long long *)=(long long)out.count;else if(length==6)*va_arg(args,intmax_t *)=(intmax_t)out.count;else if(length==7||length==8)*va_arg(args,ptrdiff_t *)=(ptrdiff_t)out.count;else *va_arg(args,int *)=(int)out.count;continue;}
        if(c=='s'||c=='c'||c=='%'){if(length)goto invalid;if(c=='s'){s=va_arg(args,const char *);if(!s)goto invalid;n=0;while((precision<0||n<precision)&&s[n]){if(n==2147483647)goto overflow;++n;}}else{digits[0]=c=='%'?'%':(char)va_arg(args,int);s=digits;n=1;}pad=width>n?width-n:0;if(!left)cd_repeat(&out,' ',pad);cd_str(&out,s,n);if(left)cd_repeat(&out,' ',pad);continue;}
        if(c=='f'||c=='F'||c=='e'||c=='E'||c=='g'||c=='G'||c=='a'||c=='A'){
            if(length==5)fp.f=(double)va_arg(args,long double);else if(length==0||length==3)fp.f=va_arg(args,double);else goto invalid;
            sign=(int)(fp.u>>63);signchar=sign?'-':plus?'+':space?' ':0;
            measure.data=(char *)0;measure.size=0;measure.count=0;
            cd_float(&measure,fp.u,c,precision,alternate,cosmic_sf_current_context()->rounding);
            if(measure.count>2147483647ULL-(signchar?1u:0u))goto overflow;n=(int)measure.count+(signchar?1:0);pad=width>n?width-n:0;
            if(((fp.u>>52)&2047)==2047)zero=0;
            if(!left&&!zero)cd_repeat(&out,' ',pad);if(signchar)cd_put(&out,signchar);
            if(!left&&zero){if(c=='a'||c=='A'){cd_put(&out,'0');cd_put(&out,c=='A'?'X':'x');/* Prefix is emitted by float core; move zeros after prefix below. */measure.data=0;}cd_repeat(&out,'0',pad);}
            if(!left&&zero&&(c=='a'||c=='A'))cd_hex(&out,fp.u,precision,alternate,c=='A',cosmic_sf_current_context()->rounding,0);
            else cd_float(&out,fp.u,c,precision,alternate,cosmic_sf_current_context()->rounding);
            if(left)cd_repeat(&out,' ',pad);continue;
        }
        prefix=0;signchar=0;
        if(c=='d'||c=='i'){
            if(length==4)v=va_arg(args,long long);else if(length==3)v=va_arg(args,long);else if(length==6)v=va_arg(args,intmax_t);else if(length==7||length==8)v=va_arg(args,ptrdiff_t);else{v=va_arg(args,int);if(length==1)v=(short)v;else if(length==2)v=(signed char)v;}
            if(v<0){signchar='-';u=0-(unsigned long long)v;}else{u=(unsigned long long)v;signchar=plus?'+':space?' ':0;}i=10;
        }else if(c=='u'||c=='o'||c=='x'||c=='X'||c=='p'){
            if(c=='p'){u=(uintptr_t)va_arg(args,void *);prefix=2;}else if(length==4)u=va_arg(args,unsigned long long);else if(length==3)u=va_arg(args,unsigned long);else if(length==6)u=va_arg(args,uintmax_t);else if(length==7||length==8)u=va_arg(args,size_t);else if(length==1)u=(unsigned short)va_arg(args,int);else if(length==2)u=(unsigned char)va_arg(args,int);else u=va_arg(args,unsigned int);
            i=c=='o'?8:c=='u'?10:16;if(alternate&&u&&(c=='x'||c=='X'))prefix=2;
        }else goto invalid;
        alphabet=c=='X'?"0123456789ABCDEF":"0123456789abcdef";n=0;
        if(u||precision!=0){do{digits[n++]=alphabet[u%(unsigned int)i];u/=(unsigned int)i;}while(u);}
        if(c=='o'&&alternate&&(n==0||digits[n-1]!='0')&&precision<=n)precision=n+1;
        if(precision<0)precision=n;else zero=0;
        if((unsigned long long)(precision>n?precision:n)+prefix+(signchar?1u:0u)>2147483647ULL)goto overflow;pad=width-(precision>n?precision:n)-prefix-(signchar?1:0);if(pad<0)pad=0;
        if(!left&&!zero)cd_repeat(&out,' ',pad);if(signchar)cd_put(&out,signchar);if(prefix){cd_put(&out,'0');cd_put(&out,c=='X'?'X':'x');}
        if(!left&&zero)cd_repeat(&out,'0',pad);cd_repeat(&out,'0',precision-n);while(n)cd_put(&out,digits[--n]);if(left)cd_repeat(&out,' ',pad);
    }
    if(size)buffer[(size_t)(out.count<size?out.count:size-1)]=0;
    if(out.count>2147483647ULL)goto overflow;return (int)out.count;
invalid: errno=22;if(size)buffer[(size_t)(out.count<size?out.count:size-1)]=0;return -1;
overflow: errno=75;if(size)buffer[(size_t)(out.count<size?out.count:size-1)]=0;return -1;
}
int snprintf(char *buffer,size_t size,const char *format,...){va_list args;int result;va_start(args,format);result=vsnprintf(buffer,size,format,args);va_end(args);return result;}
int vsprintf(char *buffer,const char *format,va_list args){return vsnprintf(buffer,(size_t)-1,format,args);}
int sprintf(char *buffer,const char *format,...){va_list args;int result;va_start(args,format);result=vsprintf(buffer,format,args);va_end(args);return result;}
