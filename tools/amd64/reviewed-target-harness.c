#include <stdio.h>
#include <math.h>
extern int reviewed_target(double, double);
int main(void) {
    int result = reviewed_target(NAN, -0.0);
    if (result) fprintf(stderr, "reviewed target case %d failed\n", result);
    return result;
}
