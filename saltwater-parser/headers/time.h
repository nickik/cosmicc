#ifndef __STDC_TIME_H
#define __STDC_TIME_H

#include <sys/types.h>

typedef long clock_t;

struct timespec {
    time_t tv_sec;
    long tv_nsec;
};

time_t time(time_t *result);

#endif
