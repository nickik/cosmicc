#ifndef _FCNTL_H
#define _FCNTL_H
#include <sys/types.h>
/* Declarations only: runtime-specific open flags are supplied by the target adapter. */
int open(const char *path, int flags, ...);
int fcntl(int fd, int command, ...);
#endif
