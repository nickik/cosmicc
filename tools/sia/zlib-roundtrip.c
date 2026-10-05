#include "zlib.h"
#ifdef COSMIC_ZLIB_REFERENCE
#include <stdio.h>
#else
#include "zlib-oracle.h"
#endif

/* A bounded guest allocator, also compiled by GCC for the byte oracle. */
#define ARENA_SIZE 0x50000u
static unsigned long long arena_words[ARENA_SIZE / 8u];
static unsigned int used;
static voidpf allocate(voidpf opaque, uInt count, uInt size) {
    unsigned int n, rounded, i;
    unsigned char *p;
    (void)opaque;
    if (size && count > ARENA_SIZE / size) return 0;
    n = count * size;
    if (n > ARENA_SIZE - 7u) return 0;
    rounded = (n + 7u) & ~7u;
    if (rounded > ARENA_SIZE - used) return 0;
    p = (unsigned char *)arena_words + used;
    used += rounded;
    for (i = 0; i < n; i++) p[i] = 0;
    return p;
}
static void release(voidpf opaque, voidpf p) { (void)opaque; (void)p; }
static const unsigned int sizes[8] = {0, 1, 31, 257, 1024, 257, 1024, 1024};
static const int levels[8] = {0, 1, 1, 6, 9, 6, 1, 9};
static const int strategies[8] = {
    Z_DEFAULT_STRATEGY, Z_DEFAULT_STRATEGY, Z_DEFAULT_STRATEGY,
    Z_DEFAULT_STRATEGY, Z_DEFAULT_STRATEGY, Z_FIXED, Z_HUFFMAN_ONLY, Z_RLE
};
int main(void) {
    unsigned char input[1024], packed[2048], output[1024];
    unsigned int index, i, packed_size, crc, adler, state;
    int rc;
    for (index = 0; index < 8; index++) {
        z_stream s = {0};
        /* Startup must clear this storage even when initial RAM is dirty. */
        if (used != 0) return 1;
        state = 123456789u + index;
        for (i = 0; i < sizes[index]; i++) {
            state = state * 1664525u + 1013904223u;
            input[i] = (unsigned char)(index % 3u == 0 ? i % 7u :
                         index % 3u == 1 ? state >> 24 : i * 37u);
        }
        s.zalloc = allocate; s.zfree = release;
        rc = deflateInit2(&s, levels[index], Z_DEFLATED, 9, 1, strategies[index]);
        if (rc != Z_OK) return 10 + index;
        s.next_in = input; s.avail_in = sizes[index];
        s.next_out = packed; s.avail_out = sizeof(packed);
        if (deflate(&s, Z_FINISH) != Z_STREAM_END) return 20 + index;
        packed_size = (unsigned int)s.total_out;
        if (deflateEnd(&s) != Z_OK) return 30 + index;
        crc = (unsigned int)crc32(0, input, sizes[index]);
        adler = (unsigned int)adler32(1, input, sizes[index]);
#ifndef COSMIC_ZLIB_REFERENCE
        if (packed_size != expected_size[index] || crc != expected_crc[index] ||
            adler != expected_adler[index]) return 40 + index;
        for (i = 0; i < packed_size; i++)
            if (packed[i] != expected_bytes[expected_offset[index] + i]) return 50 + index;
#endif
        used = 0;
        s.total_in = 0; s.total_out = 0; s.state = 0;
        if (inflateInit(&s) != Z_OK) return 60 + index;
        s.next_in = packed; s.avail_in = packed_size;
        s.next_out = output; s.avail_out = sizeof(output);
        if (inflate(&s, Z_FINISH) != Z_STREAM_END) return 70 + index;
        if (s.total_out != sizes[index] || (unsigned int)s.adler != adler) return 80 + index;
        if (inflateEnd(&s) != Z_OK) return 90 + index;
        for (i = 0; i < sizes[index]; i++) if (input[i] != output[i]) return 100 + index;
#ifdef COSMIC_ZLIB_REFERENCE
        printf("%u %u %08x %08x ", index, packed_size, crc, adler);
        for (i = 0; i < packed_size; i++) printf("%02x", (unsigned int)packed[i]);
        printf("\n");
#endif
        /* Invalid zlib header must fail rather than silently return data. */
        used = 0; s.total_in = 0; s.total_out = 0; s.state = 0;
        if (inflateInit(&s) != Z_OK) return 110 + index;
        packed[0] ^= 0xffu;
        s.next_in = packed; s.avail_in = packed_size;
        s.next_out = output; s.avail_out = sizeof(output);
        if (inflate(&s, Z_FINISH) != Z_DATA_ERROR) return 120 + index;
        if (inflateEnd(&s) != Z_OK) return 130 + index;
        used = 0;
    }
    return 0;
}
