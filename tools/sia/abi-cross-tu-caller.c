struct packet { long long value; double scale; unsigned char tag[3]; };
extern struct packet transform(int, struct packet, long long, double);
extern long long sum_words(int, ...);
extern double sum_promoted(int, ...);
int main(void)
{
    struct packet source = { 4294967297LL, 1.5, { 7, 9, 11 } };
    struct packet result;
    struct packet (*indirect)(int, struct packet, long long, double) = transform;
    float f = 1.25f;
    result = indirect(3, source, -4294967296LL, 2.0);
    if (result.value != 4 || result.scale != 3.0 || result.tag[0] != 8 || result.tag[1] != 9 || result.tag[2] != 11) return 1;
    if (source.value != 4294967297LL || source.scale != 1.5 || source.tag[0] != 7) return 2;
    if (sum_words(9, 1LL, -2LL, 4294967296LL, 4LL, 5LL, 6LL, 7LL, 8LL, 9LL) != 4294967334LL) return 3;
    if (sum_promoted(8, f, 2.5, -3.0, 4.0f, 0.25, 8.0, 16.0, -1.0) != 28.0) return 4;
    return 0;
}
