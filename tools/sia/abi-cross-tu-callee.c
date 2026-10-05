#include <stdarg.h>
struct packet { long long value; double scale; unsigned char tag[3]; };
struct packet transform(int prefix, struct packet input, long long bias, double factor)
{
    input.value += bias + prefix;
    input.scale *= factor;
    input.tag[0] += 1;
    return input;
}
long long sum_words(int count, ...)
{
    va_list args, copy;
    long long total = 0, again = 0;
    int i;
    va_start(args, count);
    va_copy(copy, args);
    for (i = 0; i < count; ++i) total += va_arg(args, long long);
    for (i = 0; i < count; ++i) again += va_arg(copy, long long);
    va_end(copy);
    va_end(args);
    return total == again ? total : -1;
}
double sum_promoted(int count, ...)
{
    va_list args;
    double total = 0;
    int i;
    va_start(args, count);
    for (i = 0; i < count; ++i) total += va_arg(args, double);
    va_end(args);
    return total;
}
