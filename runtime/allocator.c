#include "include/cosmic_runtime_v1.h"
/* Eight-byte size/span headers. Size zero denotes a free block.
 * Metadata is private; callers must not overwrite it or arena state.
 */
int cosmic_arena_init(cosmic_arena *a, void *memory, cosmic_size_t capacity) {
    unsigned int address = (unsigned int)memory;
    unsigned int padding = (0u - address) & 7u;
    a->base = 0; a->capacity = 0; a->used = 0;
    if (memory == 0 || capacity < padding || capacity > 0xffffffffu - address)
        return 1;
    a->base = (unsigned char *)memory + padding;
    a->capacity = capacity - padding;
    return 0;
}
static unsigned int *cosmic_arena_header(cosmic_arena *a, cosmic_size_t offset) {
    unsigned int *h;
    if (a->used > a->capacity || offset > a->used || a->used - offset < 8u)
        return 0;
    h = (unsigned int *)(a->base + offset);
    if (h[1] < 16u || (h[1] & 7u) != 0 || h[1] > a->used - offset
        || h[0] > h[1] - 8u) return 0;
    return h;
}
void *cosmic_arena_alloc(cosmic_arena *a, cosmic_size_t size) {
    cosmic_size_t span, offset = 0, remainder;
    unsigned int *header;
    unsigned int *next;
    if (size == 0 || size > 0xfffffff0u || a->used > a->capacity) return 0;
    span = (size + 15u) & ~7u;
    while (offset < a->used) {
        header = cosmic_arena_header(a, offset);
        if (header == 0) return 0;
        if (header[0] == 0 && header[1] >= span) {
            remainder = header[1] - span;
            if (remainder >= 16u) {
                next = (unsigned int *)(a->base + offset + span);
                next[0] = 0; next[1] = remainder;
                header[1] = span;
            }
            header[0] = size;
            return (unsigned char *)header + 8;
        }
        offset += header[1];
    }
    if (span > a->capacity - a->used) return 0;
    header = (unsigned int *)(a->base + a->used);
    header[0] = size; header[1] = span;
    a->used += span;
    return (unsigned char *)header + 8;
}
int cosmic_arena_free(cosmic_arena *a, void *pointer) {
    cosmic_size_t offset = 0, found = 0, next_offset;
    unsigned int *header;
    unsigned int *next;
    if (pointer == 0) return 0;
    /* Validate the entire metadata chain before mutation. */
    while (offset < a->used) {
        header = cosmic_arena_header(a, offset);
        if (header == 0) return 1;
        if ((unsigned char *)header + 8 == (unsigned char *)pointer) {
            if (header[0] == 0) return 1;
            found = offset + 1u;
        }
        offset += header[1];
    }
    if (found == 0) return 1;
    header = (unsigned int *)(a->base + found - 1u);
    header[0] = 0;
    /* Merge adjacent holes and release trailing space. */
    offset = 0;
    while (offset < a->used) {
        header = (unsigned int *)(a->base + offset);
        next_offset = offset + header[1];
        if (header[0] == 0) {
            while (next_offset < a->used) {
                next = (unsigned int *)(a->base + next_offset);
                if (next[0] != 0) break;
                header[1] += next[1];
                next_offset = offset + header[1];
            }
            if (next_offset == a->used) { a->used = offset; break; }
        }
        offset = next_offset;
    }
    return 0;
}
void *cosmic_arena_realloc(cosmic_arena *a, void *old, cosmic_size_t size) {
    cosmic_size_t offset = 0, old_size, i;
    unsigned int *header;
    unsigned char *result;
    if (old == 0) return cosmic_arena_alloc(a, size);
    while (offset < a->used) {
        header = cosmic_arena_header(a, offset);
        if (header == 0) return 0;
        if ((unsigned char *)header + 8 == (unsigned char *)old) {
            old_size = header[0];
            if (old_size == 0) return 0;
            if (size == 0) { cosmic_arena_free(a, old); return 0; }
            if (size <= header[1] - 8u) { header[0] = size; return old; }
            result = (unsigned char *)cosmic_arena_alloc(a, size);
            if (result == 0) return 0;
            for (i = 0; i < old_size; i++) result[i] = ((unsigned char *)old)[i];
            cosmic_arena_free(a, old);
            return result;
        }
        offset += header[1];
    }
    return 0;
}
void *cosmic_arena_calloc(cosmic_arena *a, cosmic_size_t count, cosmic_size_t size) {
    cosmic_size_t bytes, i;
    unsigned char *p;
    if (size != 0 && count > 0xffffffffu / size) return 0;
    bytes = count * size;
    p = (unsigned char *)cosmic_arena_alloc(a, bytes);
    if (p != 0) for (i = 0; i < bytes; i++) p[i] = 0;
    return p;
}
void cosmic_arena_reset(cosmic_arena *a) { a->used = 0; }
