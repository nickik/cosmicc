#ifndef __COSMIC_AMD64_MATH_H
#define __COSMIC_AMD64_MATH_H
double sin(double); double cos(double); double tan(double);
double asin(double); double acos(double); double atan(double); double atan2(double,double);
double sinh(double); double cosh(double); double tanh(double);
double exp(double); double exp2(double); double log(double); double log2(double); double log10(double);
double pow(double,double); double sqrt(double); double cbrt(double);
double ceil(double); double floor(double); double trunc(double); double round(double);
double fabs(double); double fmod(double,double); double remainder(double,double);
double frexp(double,int*); double ldexp(double,int); double modf(double,double*);
float sinf(float); float cosf(float); float tanf(float); float sqrtf(float);
float powf(float,float); float fabsf(float); float floorf(float); float ceilf(float);
#endif
