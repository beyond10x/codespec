/* codespec_rt.h: the runtime of C written by codespec-write-c. Header-only, C11, libc only.
 *
 * It holds what cir/1 has and C lacks: a string, a slice, a map, a channel without goroutines,
 * an error value, an `any` with the type it holds, and integer arithmetic that wraps as cir/1's
 * does (sem.int.wrap). Memory is allocated and never freed: a written program is short-lived. */
#ifndef CODESPEC_RT_H
#define CODESPEC_RT_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* A run-time error the source language panics on: printed to stderr, exit status 3. */
_Noreturn static inline void cs_panic(const char *msg) {
    fflush(stdout);
    fprintf(stderr, "panic: %s\n", msg);
    exit(3);
}

static inline void *cs_new(size_t n) {
    void *p = calloc(1, n ? n : 1);
    if (p == NULL) {
        cs_panic("out of memory");
    }
    return p;
}

static inline void *cs_dup(const void *v, size_t n) {
    void *p = cs_new(n);
    if (n) {
        memcpy(p, v, n);
    }
    return p;
}

/* ---- integers ------------------------------------------------------------------------------
 * `+`, `-`, `*` and negation of a signed type are written through uint64_t and converted back,
 * so they wrap instead of overflowing. Division, remainder and shifts go through these. */

static inline uint64_t cs_shift_count(int64_t n) {
    if (n < 0) {
        cs_panic("runtime error: negative shift amount");
    }
    return (uint64_t)n;
}

#define CS_SIGNED_OPS(T)                                                                        \
    static inline T cs_div_##T(T x, T y) {                                                    \
        if (y == 0) {                                                                         \
            cs_panic("runtime error: integer divide by zero");                                \
        }                                                                                     \
        if (y == -1) {                                                                        \
            return (T)((uint64_t)0 - (uint64_t)x);                                            \
        }                                                                                     \
        return (T)(x / y);                                                                    \
    }                                                                                         \
    static inline T cs_rem_##T(T x, T y) {                                                    \
        if (y == 0) {                                                                         \
            cs_panic("runtime error: integer divide by zero");                                \
        }                                                                                     \
        if (y == -1) {                                                                        \
            return 0;                                                                         \
        }                                                                                     \
        return (T)(x % y);                                                                    \
    }                                                                                         \
    static inline T cs_shl_##T(T x, uint64_t n) {                                             \
        return n >= sizeof(T) * 8 ? 0 : (T)((uint64_t)x << n);                                \
    }                                                                                         \
    static inline T cs_shr_##T(T x, uint64_t n) {                                             \
        return n >= sizeof(T) * 8 ? (T)(x < 0 ? -1 : 0) : (T)(x >> n);                        \
    }

#define CS_UNSIGNED_OPS(T)                                                                      \
    static inline T cs_div_##T(T x, T y) {                                                    \
        if (y == 0) {                                                                         \
            cs_panic("runtime error: integer divide by zero");                                \
        }                                                                                     \
        return (T)(x / y);                                                                    \
    }                                                                                         \
    static inline T cs_rem_##T(T x, T y) {                                                    \
        if (y == 0) {                                                                         \
            cs_panic("runtime error: integer divide by zero");                                \
        }                                                                                     \
        return (T)(x % y);                                                                    \
    }                                                                                         \
    static inline T cs_shl_##T(T x, uint64_t n) {                                             \
        return n >= sizeof(T) * 8 ? 0 : (T)((uint64_t)x << n);                                \
    }                                                                                         \
    static inline T cs_shr_##T(T x, uint64_t n) {                                             \
        return n >= sizeof(T) * 8 ? 0 : (T)(x >> n);                                          \
    }

CS_SIGNED_OPS(int8_t)
CS_SIGNED_OPS(int16_t)
CS_SIGNED_OPS(int32_t)
CS_SIGNED_OPS(int64_t)
CS_UNSIGNED_OPS(uint8_t)
CS_UNSIGNED_OPS(uint16_t)
CS_UNSIGNED_OPS(uint32_t)
CS_UNSIGNED_OPS(uint64_t)

/* ---- strings: immutable bytes and their length ---------------------------------------------- */

typedef struct {
    const char *p;
    int64_t n;
} cs_string;

static inline bool cs_string_eq(cs_string a, cs_string b) {
    return a.n == b.n && (a.n == 0 || memcmp(a.p, b.p, (size_t)a.n) == 0);
}

/* Byte order, as Go compares strings. */
static inline int cs_string_cmp(cs_string a, cs_string b) {
    size_t n = (size_t)(a.n < b.n ? a.n : b.n);
    int c = n ? memcmp(a.p, b.p, n) : 0;
    if (c != 0) {
        return c < 0 ? -1 : 1;
    }
    return a.n < b.n ? -1 : (a.n > b.n ? 1 : 0);
}

static inline cs_string cs_string_cat(cs_string a, cs_string b) {
    char *p = cs_new((size_t)(a.n + b.n));
    if (a.n) {
        memcpy(p, a.p, (size_t)a.n);
    }
    if (b.n) {
        memcpy(p + a.n, b.p, (size_t)b.n);
    }
    return (cs_string){p, a.n + b.n};
}

static inline uint8_t cs_string_at(cs_string s, int64_t i) {
    if (i < 0 || i >= s.n) {
        cs_panic("runtime error: index out of range");
    }
    return (uint8_t)s.p[i];
}

/* ---- errors: nil is NULL ---------------------------------------------------------------------
 * A sentinel or formatted error holds its message; a value of an error type holds the function
 * that gives its message and a copy of the value. */

typedef struct cs_error_s {
    cs_string msg;
    cs_string (*fn)(const void *self);
    const void *self;
} *cs_error;

static inline cs_error cs_error_new(cs_string msg) {
    cs_error e = cs_new(sizeof(struct cs_error_s));
    e->msg = msg;
    return e;
}

static inline cs_error cs_error_of(cs_string (*fn)(const void *), const void *self) {
    cs_error e = cs_new(sizeof(struct cs_error_s));
    e->fn = fn;
    e->self = self;
    return e;
}

static inline cs_string cs_error_message(cs_error e) {
    if (e == NULL) {
        return (cs_string){"<nil>", 5};
    }
    return e->fn ? e->fn(e->self) : e->msg;
}

/* ---- a string builder, for sprintf, errorf and println --------------------------------------- */

typedef struct {
    char *p;
    int64_t n, cap;
} cs_sb;

static inline cs_sb *cs_sb_new(void) {
    return cs_new(sizeof(cs_sb));
}

static inline cs_sb *cs_sb_bytes(cs_sb *b, const char *p, int64_t n) {
    if (b->n + n > b->cap) {
        int64_t cap = b->cap ? b->cap * 2 : 32;
        while (cap < b->n + n) {
            cap *= 2;
        }
        char *q = cs_new((size_t)cap);
        if (b->n) {
            memcpy(q, b->p, (size_t)b->n);
        }
        b->p = q;
        b->cap = cap;
    }
    if (n) {
        memcpy(b->p + b->n, p, (size_t)n);
    }
    b->n += n;
    return b;
}

static inline cs_sb *cs_fmt_str(cs_sb *b, cs_string s) {
    return cs_sb_bytes(b, s.p, s.n);
}

static inline cs_sb *cs_fmt_uint(cs_sb *b, uint64_t v) {
    char d[24];
    int i = 24;
    do {
        d[--i] = (char)('0' + v % 10);
        v /= 10;
    } while (v);
    return cs_sb_bytes(b, d + i, 24 - i);
}

static inline cs_sb *cs_fmt_int(cs_sb *b, int64_t v) {
    if (v < 0) {
        cs_sb_bytes(b, "-", 1);
        return cs_fmt_uint(b, (uint64_t)0 - (uint64_t)v);
    }
    return cs_fmt_uint(b, (uint64_t)v);
}

static inline cs_sb *cs_fmt_bool(cs_sb *b, bool v) {
    return v ? cs_sb_bytes(b, "true", 4) : cs_sb_bytes(b, "false", 5);
}

static inline cs_sb *cs_fmt_error(cs_sb *b, cs_error e) {
    return cs_fmt_str(b, cs_error_message(e));
}

static inline cs_string cs_sb_string(cs_sb *b) {
    return (cs_string){b->p, b->n};
}

static inline cs_error cs_sb_error(cs_sb *b) {
    return cs_error_new(cs_sb_string(b));
}

/* The line and a newline on standard output, as Go's fmt.Println writes it. */
static inline void cs_sb_println(cs_sb *b) {
    cs_sb_bytes(b, "\n", 1);
    fwrite(b->p, 1, (size_t)b->n, stdout);
}

/* ---- slices: nil is {NULL, 0, 0} ---------------------------------------------------------------- */

typedef struct {
    void *p;
    int64_t len, cap;
} cs_slice;

/* An empty slice that is not nil. */
static inline cs_slice cs_slice_make(void) {
    return (cs_slice){cs_new(1), 0, 0};
}

static inline void *cs_slice_at(cs_slice s, int64_t i, size_t esz) {
    if (i < 0 || i >= s.len) {
        cs_panic("runtime error: index out of range");
    }
    return (char *)s.p + (size_t)i * esz;
}

static inline cs_slice cs_append(cs_slice s, const void *v, size_t esz) {
    if (s.len == s.cap) {
        int64_t cap = s.cap ? s.cap * 2 : 4;
        void *p = cs_new((size_t)cap * esz);
        if (s.len) {
            memcpy(p, s.p, (size_t)s.len * esz);
        }
        s.p = p;
        s.cap = cap;
    }
    memcpy((char *)s.p + (size_t)s.len * esz, v, esz);
    s.len++;
    return s;
}

/* ---- maps: nil is NULL. Entries in insertion order, each key and value in its own allocation,
 * so a pointer to one stays valid while the map grows. ------------------------------------------ */

enum {
    CS_KEY_I8,
    CS_KEY_I16,
    CS_KEY_I32,
    CS_KEY_I64,
    CS_KEY_U8,
    CS_KEY_U16,
    CS_KEY_U32,
    CS_KEY_U64,
    CS_KEY_BOOL,
    CS_KEY_STRING,
    CS_KEY_PTR
};

typedef struct cs_map {
    int kind;
    size_t ksz, vsz;
    int64_t len, cap;
    void **keys, **vals;
} cs_map;

static inline cs_map *cs_map_new(size_t ksz, size_t vsz, int kind) {
    cs_map *m = cs_new(sizeof(cs_map));
    m->kind = kind;
    m->ksz = ksz;
    m->vsz = vsz;
    return m;
}

static inline int64_t cs_map_len(const cs_map *m) {
    return m ? m->len : 0;
}

static inline bool cs_map_key_eq(const cs_map *m, const void *a, const void *b) {
    if (m->kind == CS_KEY_STRING) {
        return cs_string_eq(*(const cs_string *)a, *(const cs_string *)b);
    }
    return memcmp(a, b, m->ksz) == 0;
}

/* The value of `k`, or NULL when the map does not hold it. */
static inline void *cs_map_find(const cs_map *m, const void *k) {
    if (m == NULL) {
        return NULL;
    }
    for (int64_t i = 0; i < m->len; i++) {
        if (cs_map_key_eq(m, m->keys[i], k)) {
            return m->vals[i];
        }
    }
    return NULL;
}

/* The value of `k`, or a fresh zero value of `vsz` bytes. */
static inline void *cs_map_get(const cs_map *m, const void *k, size_t vsz) {
    void *v = cs_map_find(m, k);
    return v ? v : cs_new(vsz);
}

/* The value of `k`, inserted as zero when absent, to be assigned. */
static inline void *cs_map_put(cs_map *m, const void *k) {
    if (m == NULL) {
        cs_panic("assignment to entry in nil map");
    }
    void *v = cs_map_find(m, k);
    if (v) {
        return v;
    }
    if (m->len == m->cap) {
        int64_t cap = m->cap ? m->cap * 2 : 8;
        void **keys = cs_new((size_t)cap * sizeof(void *));
        void **vals = cs_new((size_t)cap * sizeof(void *));
        if (m->len) {
            memcpy(keys, m->keys, (size_t)m->len * sizeof(void *));
            memcpy(vals, m->vals, (size_t)m->len * sizeof(void *));
        }
        m->keys = keys;
        m->vals = vals;
        m->cap = cap;
    }
    m->keys[m->len] = cs_dup(k, m->ksz);
    m->vals[m->len] = cs_new(m->vsz);
    return m->vals[m->len++];
}

static inline void *cs_map_key(const cs_map *m, int64_t i) {
    return m->keys[i];
}

static inline void *cs_map_val(const cs_map *m, int64_t i) {
    return m->vals[i];
}

/* The UTF-16 code units of the code point starting at s[*i], advancing *i; a byte that does
 * not start a well-formed sequence is one unit of its own value. */
static inline int cs_utf16_units(cs_string s, int64_t *i, uint16_t u[2]) {
    const unsigned char *p = (const unsigned char *)s.p;
    uint32_t c = p[*i];
    int64_t n = 1;
    if (c >= 0xF0 && *i + 3 < s.n + 0 && (p[*i + 1] & 0xC0) == 0x80 &&
        (p[*i + 2] & 0xC0) == 0x80 && (p[*i + 3] & 0xC0) == 0x80) {
        c = ((c & 0x07) << 18) | ((uint32_t)(p[*i + 1] & 0x3F) << 12) |
            ((uint32_t)(p[*i + 2] & 0x3F) << 6) | (uint32_t)(p[*i + 3] & 0x3F);
        n = 4;
    } else if (c >= 0xE0 && *i + 2 < s.n && (p[*i + 1] & 0xC0) == 0x80 &&
               (p[*i + 2] & 0xC0) == 0x80) {
        c = ((c & 0x0F) << 12) | ((uint32_t)(p[*i + 1] & 0x3F) << 6) |
            (uint32_t)(p[*i + 2] & 0x3F);
        n = 3;
    } else if (c >= 0xC0 && *i + 1 < s.n && (p[*i + 1] & 0xC0) == 0x80) {
        c = ((c & 0x1F) << 6) | (uint32_t)(p[*i + 1] & 0x3F);
        n = 2;
    }
    *i += n;
    if (c >= 0x10000) {
        c -= 0x10000;
        u[0] = (uint16_t)(0xD800 + (c >> 10));
        u[1] = (uint16_t)(0xDC00 + (c & 0x3FF));
        return 2;
    }
    u[0] = (uint16_t)c;
    return 1;
}

/* By UTF-16 code unit, as Java's String.compareTo orders strings (cir/1 `sorted_keys`). */
static inline int cs_string_cmp_utf16(cs_string a, cs_string b) {
    int64_t i = 0, j = 0;
    uint16_t ua[2], ub[2];
    int na = 0, nb = 0, ia = 0, ib = 0;
    for (;;) {
        if (ia == na) {
            if (i >= a.n) {
                na = 0;
            } else {
                na = cs_utf16_units(a, &i, ua);
            }
            ia = 0;
        }
        if (ib == nb) {
            if (j >= b.n) {
                nb = 0;
            } else {
                nb = cs_utf16_units(b, &j, ub);
            }
            ib = 0;
        }
        if (na == 0 || nb == 0) {
            return na == nb ? 0 : (na == 0 ? -1 : 1);
        }
        if (ua[ia] != ub[ib]) {
            return ua[ia] < ub[ib] ? -1 : 1;
        }
        ia++;
        ib++;
    }
}

static inline int cs_map_key_cmp(int kind, const void *a, const void *b) {
    switch (kind) {
    case CS_KEY_I8: return *(const int8_t *)a < *(const int8_t *)b ? -1 : *(const int8_t *)a > *(const int8_t *)b;
    case CS_KEY_I16: return *(const int16_t *)a < *(const int16_t *)b ? -1 : *(const int16_t *)a > *(const int16_t *)b;
    case CS_KEY_I32: return *(const int32_t *)a < *(const int32_t *)b ? -1 : *(const int32_t *)a > *(const int32_t *)b;
    case CS_KEY_I64: return *(const int64_t *)a < *(const int64_t *)b ? -1 : *(const int64_t *)a > *(const int64_t *)b;
    case CS_KEY_U8: return *(const uint8_t *)a < *(const uint8_t *)b ? -1 : *(const uint8_t *)a > *(const uint8_t *)b;
    case CS_KEY_U16: return *(const uint16_t *)a < *(const uint16_t *)b ? -1 : *(const uint16_t *)a > *(const uint16_t *)b;
    case CS_KEY_U32: return *(const uint32_t *)a < *(const uint32_t *)b ? -1 : *(const uint32_t *)a > *(const uint32_t *)b;
    case CS_KEY_U64: return *(const uint64_t *)a < *(const uint64_t *)b ? -1 : *(const uint64_t *)a > *(const uint64_t *)b;
    case CS_KEY_STRING: return cs_string_cmp_utf16(*(const cs_string *)a, *(const cs_string *)b);
    default: cs_panic("sorted_keys: keys that have no order");
    }
}

/* The keys in ascending order: integers numerically, strings by UTF-16 code unit. */
static inline cs_slice cs_map_sorted_keys(const cs_map *m) {
    cs_slice s = cs_slice_make();
    if (m == NULL) {
        return s;
    }
    for (int64_t i = 0; i < m->len; i++) {
        s = cs_append(s, m->keys[i], m->ksz);
    }
    char *base = s.p;
    char *tmp = cs_new(m->ksz);
    for (int64_t i = 1; i < s.len; i++) {
        memcpy(tmp, base + (size_t)i * m->ksz, m->ksz);
        int64_t j = i - 1;
        while (j >= 0 && cs_map_key_cmp(m->kind, base + (size_t)j * m->ksz, tmp) > 0) {
            memcpy(base + (size_t)(j + 1) * m->ksz, base + (size_t)j * m->ksz, m->ksz);
            j--;
        }
        memcpy(base + (size_t)(j + 1) * m->ksz, tmp, m->ksz);
    }
    return s;
}

/* ---- channels: a buffer of fixed room, no goroutines. A send on a full channel and a receive
 * from an empty one can never complete, and panic as Go's deadlock does. ------------------------- */

typedef struct cs_chan {
    size_t esz;
    int64_t cap, len, head;
    char *buf;
} cs_chan;

static inline cs_chan *cs_chan_make(int64_t cap, size_t esz) {
    cs_chan *c = cs_new(sizeof(cs_chan));
    c->esz = esz;
    c->cap = cap;
    c->buf = cs_new((size_t)(cap ? cap : 1) * esz);
    return c;
}

static inline int64_t cs_chan_len(const cs_chan *c) {
    return c ? c->len : 0;
}

static inline void cs_chan_send(cs_chan *c, const void *v) {
    if (c == NULL || c->len == c->cap) {
        cs_panic("all goroutines are asleep - deadlock!");
    }
    memcpy(c->buf + (size_t)((c->head + c->len) % c->cap) * c->esz, v, c->esz);
    c->len++;
}

/* A copy of the oldest value, removed from the channel. */
static inline void *cs_chan_recv(cs_chan *c) {
    if (c == NULL || c->len == 0) {
        cs_panic("all goroutines are asleep - deadlock!");
    }
    void *v = cs_dup(c->buf + (size_t)c->head * c->esz, c->esz);
    c->head = (c->head + 1) % c->cap;
    c->len--;
    return v;
}

/* ---- any: a value and the type it holds; nil is {NULL, NULL} ---------------------------------- */

enum {
    CS_K_INT8,
    CS_K_INT16,
    CS_K_INT32,
    CS_K_INT64,
    CS_K_UINT8,
    CS_K_UINT16,
    CS_K_UINT32,
    CS_K_UINT64,
    CS_K_BOOL,
    CS_K_STRING,
    CS_K_ERROR,
    CS_K_SLICE,
    CS_K_NAMED,
    CS_K_STRUCT,
    CS_K_OTHER
};

/* A type as an `any` records it: its kind, its name, and for a slice its element type, for a
 * named type that is not a struct the type it is defined over. */
typedef struct cs_type {
    int kind;
    const char *name;
    const struct cs_type *elem;
} cs_type;

typedef struct {
    const cs_type *t;
    void *p;
} cs_any;

static inline cs_any cs_box(const cs_type *t, const void *v, size_t n) {
    return (cs_any){t, cs_dup(v, n)};
}

#endif
