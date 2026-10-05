/* Portable fixed-work kernels. No host calls, undefined signed overflow, or I/O. */
unsigned long bench_hash(const unsigned int *input, unsigned long count) {
    unsigned long hash = 1469598103934665603UL;
    unsigned long i;
    for (i = 0; i < count; i++) {
        hash ^= input[i];
        hash *= 1099511628211UL;
    }
    return hash;
}
unsigned long bench_branch(unsigned long count, unsigned long state) {
    unsigned long i, sum = 0;
    for (i = 0; i < count; i++) {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        if (state & 1UL) sum += state ^ i;
        else sum ^= state + i;
    }
    return sum;
}
unsigned long bench_divide(unsigned long count, unsigned long state) {
    unsigned long i, denominator, total = 0;
    for (i = 0; i < count; i++) {
        state = state * 6364136223846793005UL + 1442695040888963407UL;
        denominator = (state >> 37) | 3UL;
        total += state / denominator;
        total ^= state % denominator;
    }
    return total;
}
void bench_matrix(const int *a, const int *b, long *c, unsigned long size) {
    unsigned long i, j, k;
    long sum;
    for (i = 0; i < size; i++) for (j = 0; j < size; j++) {
        sum = 0;
        for (k = 0; k < size; k++) sum += (long)a[i*size+k] * b[k*size+j];
        c[i*size+j] = sum;
    }
}
