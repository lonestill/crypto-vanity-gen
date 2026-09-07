#include <metal_stdlib>
using namespace metal;

static inline uint64_t rotl64(uint64_t x, uint n) {
    return (x << n) | (x >> (64 - n));
}

constant uint64_t RC[24] = {
    0x0000000000000001ULL, 0x0000000000008082ULL, 0x800000000000808aULL,
    0x8000000080008000ULL, 0x000000000000808bULL, 0x0000000080000001ULL,
    0x8000000080008081ULL, 0x8000000000008009ULL, 0x000000000000008aULL,
    0x0000000000000088ULL, 0x0000000080008009ULL, 0x000000008000000aULL,
    0x000000008000808bULL, 0x800000000000008bULL, 0x8000000000008089ULL,
    0x8000000000008003ULL, 0x8000000000008002ULL, 0x8000000000000080ULL,
    0x000000000000800aULL, 0x800000008000000aULL, 0x8000000080008081ULL,
    0x8000000000008080ULL, 0x0000000080000001ULL, 0x8000000080008008ULL
};

constant uint RHO[24] = {
    1, 3, 6, 10, 15, 21, 28, 36, 45, 55, 2, 14, 27, 41, 56, 8, 25, 43, 62, 18, 39, 61, 20, 44
};

constant uint PI[24] = {
    10, 7, 11, 17, 18, 3, 5, 16, 8, 21, 24, 4, 15, 23, 19, 13, 12, 2, 20, 14, 22, 9, 6, 1
};

void keccak_f1600(thread uint64_t* a) {
    for (int round = 0; round < 24; ++round) {
        uint64_t c[5];
        c[0] = a[0] ^ a[5] ^ a[10] ^ a[15] ^ a[20];
        c[1] = a[1] ^ a[6] ^ a[11] ^ a[16] ^ a[21];
        c[2] = a[2] ^ a[7] ^ a[12] ^ a[17] ^ a[22];
        c[3] = a[3] ^ a[8] ^ a[13] ^ a[18] ^ a[23];
        c[4] = a[4] ^ a[9] ^ a[14] ^ a[19] ^ a[24];

        uint64_t d[5];
        d[0] = c[4] ^ rotl64(c[1], 1);
        d[1] = c[0] ^ rotl64(c[2], 1);
        d[2] = c[1] ^ rotl64(c[3], 1);
        d[3] = c[2] ^ rotl64(c[4], 1);
        d[4] = c[3] ^ rotl64(c[0], 1);

        for (int i = 0; i < 25; ++i) {
            a[i] ^= d[i % 5];
        }

        uint64_t last = a[1];
        for (int x = 0; x < 24; ++x) {
            uint64_t temp = a[PI[x]];
            a[PI[x]] = rotl64(last, RHO[x]);
            last = temp;
        }

        for (int y_step = 0; y_step < 5; ++y_step) {
            int y = y_step * 5;
            uint64_t row[5];
            for (int x = 0; x < 5; ++x) {
                row[x] = a[y + x];
            }
            for (int x = 0; x < 5; ++x) {
                a[y + x] = row[x] ^ ((~row[(x + 1) % 5]) & row[(x + 2) % 5]);
            }
        }

        a[0] ^= RC[round];
    }
}

struct GpuMatch {
    uint64_t salt_low;
    uint64_t salt_high;
    uint8_t address[20];
    uint32_t score;
    uint32_t tier;
    uint32_t pattern_type;
};

inline bool evaluate_address(
    thread const uint8_t* addr,
    constant uint8_t* target_nibbles,
    uint32_t target_len_nibbles,
    uint32_t min_zero_nibbles,
    thread uint32_t& match_score,
    thread uint32_t& match_tier,
    thread uint32_t& match_pattern
) {
    if (target_len_nibbles > 0) {
        uint32_t match_len = 0;
        for (uint32_t k = 0; k < target_len_nibbles; ++k) {
            uint8_t byte = addr[k >> 1];
            uint8_t nibble = (k & 1) ? (byte & 0x0f) : (byte >> 4);
            if (nibble == target_nibbles[k]) {
                match_len++;
            } else {
                break;
            }
        }
        uint32_t req_stage = (min_zero_nibbles >= 3) ? min_zero_nibbles : 3;
        if (match_len >= req_stage) {
            match_score = (match_len * 110 > 1000) ? 1000 : (match_len * 110);
            match_tier = (match_len >= target_len_nibbles) ? 5 : ((match_len >= 6) ? 4 : 3);
            match_pattern = match_len;
            return true;
        }
        return false;
    }

    uint32_t zero_nibbles = 0;
    for (int i = 0; i < 20; ++i) {
        uint8_t byte = addr[i];
        if ((byte >> 4) == 0) {
            zero_nibbles++;
        } else {
            break;
        }
        if ((byte & 0x0f) == 0) {
            zero_nibbles++;
        } else {
            break;
        }
    }

    if (zero_nibbles >= min_zero_nibbles) {
        match_score = (zero_nibbles >= 9) ? 1000 : ((zero_nibbles >= 8) ? 980 : 920);
        match_tier = (zero_nibbles >= 9) ? 5 : ((zero_nibbles >= 8) ? 4 : 3);
        match_pattern = 1;
        return true;
    }

    uint8_t n0 = addr[0] >> 4;
    uint8_t n1 = addr[0] & 0x0f;
    uint8_t n2 = addr[1] >> 4;
    uint8_t n3 = addr[1] & 0x0f;
    uint8_t n4 = addr[2] >> 4;
    uint8_t n5 = addr[2] & 0x0f;
    uint8_t n6 = addr[3] >> 4;
    uint8_t e1 = addr[18] & 0x0f;

    if (addr[0] == 0xde && addr[1] == 0xad && addr[2] == 0xbe && addr[3] == 0xef) {
        match_score = 1000;
        match_tier = 5;
        match_pattern = 6;
        return true;
    } else if (addr[0] == 0xca && addr[1] == 0xfe && addr[2] == 0xba && addr[3] == 0xbe) {
        match_score = 1000;
        match_tier = 5;
        match_pattern = 6;
        return true;
    } else if (addr[0] == 0xde && addr[1] == 0xad && addr[2] == 0xc0 && addr[3] == 0xde) {
        match_score = 995;
        match_tier = 5;
        match_pattern = 6;
        return true;
    } else if (addr[0] == 0xde && addr[1] == 0xf1 && addr[2] == 0x13 && addr[3] == 0x37) {
        match_score = 990;
        match_tier = 5;
        match_pattern = 6;
        return true;
    } else if (addr[0] == 0x13 && addr[1] == 0x37 && addr[2] == 0xc0 && addr[3] == 0xde) {
        match_score = 990;
        match_tier = 5;
        match_pattern = 6;
        return true;
    } else if (addr[0] == 0xba && addr[1] == 0xdb && addr[2] == 0xab && n6 == 0x0e) {
        match_score = 980;
        match_tier = 5;
        match_pattern = 6;
        return true;
    } else if (addr[0] == 0xc0 && addr[1] == 0xff && addr[2] == 0xee) {
        match_score = 940;
        match_tier = 4;
        match_pattern = 6;
        return true;
    } else if (addr[0] == 0xba && addr[1] == 0xda && addr[2] == 0x55) {
        match_score = 930;
        match_tier = 4;
        match_pattern = 6;
        return true;
    } else if (addr[0] == 0xde && addr[1] == 0xad && addr[18] == 0xbe && addr[19] == 0xef) {
        match_score = 999;
        match_tier = 5;
        match_pattern = 4;
        return true;
    } else if (addr[0] == 0xca && addr[1] == 0xfe && addr[18] == 0xba && addr[19] == 0xbe) {
        match_score = 999;
        match_tier = 5;
        match_pattern = 4;
        return true;
    } else if (addr[0] == 0xfa && addr[1] == 0xce && addr[18] == 0xde && addr[19] == 0xad) {
        match_score = 990;
        match_tier = 5;
        match_pattern = 4;
        return true;
    } else if (addr[0] == 0x77 && addr[1] == 0x77 && addr[18] == 0x77 && addr[19] == 0x77) {
        match_score = 999;
        match_tier = 5;
        match_pattern = 4;
        return true;
    } else if (addr[0] == 0x77 && n2 == 7 && e1 == 7 && addr[19] == 0x77) {
        match_score = 950;
        match_tier = 4;
        match_pattern = 4;
        return true;
    } else if (addr[0] == 0x00 && addr[1] == 0x00 && addr[18] == 0x00 && addr[19] == 0x00) {
        match_score = 999;
        match_tier = 5;
        match_pattern = 4;
        return true;
    } else if (addr[0] == 0x00 && n2 == 0 && e1 == 0 && addr[19] == 0x00) {
        match_score = 940;
        match_tier = 4;
        match_pattern = 4;
        return true;
    } else if (addr[0] == 0x88 && n2 == 8 && e1 == 8 && addr[19] == 0x88) {
        match_score = 940;
        match_tier = 4;
        match_pattern = 4;
        return true;
    } else if (addr[0] == 0x66 && n2 == 6 && e1 == 6 && addr[19] == 0x66) {
        match_score = 930;
        match_tier = 4;
        match_pattern = 4;
        return true;
    } else if (
        (addr[0] == 0x01 && addr[1] == 0x01 && addr[2] == 0x01 && addr[3] == 0x01) ||
        (addr[0] == 0x10 && addr[1] == 0x10 && addr[2] == 0x10 && addr[3] == 0x10) ||
        (addr[0] == 0x69 && addr[1] == 0x69 && addr[2] == 0x69 && addr[3] == 0x69) ||
        (addr[0] == 0x00 && addr[1] == 0x11 && addr[2] == 0x00 && addr[3] == 0x11) ||
        (addr[0] == 0x11 && addr[1] == 0x00 && addr[2] == 0x11 && addr[3] == 0x00)
    ) {
        match_score = 910;
        match_tier = 4;
        match_pattern = 5;
        return true;
    } else if (n0 != 0 && n0 == n1 && n1 == n2 && n2 == n3 && n3 == n4 && n4 == n5) {
        bool is_7 = (n5 == n6);
        match_score = is_7 ? 999 : 930;
        match_tier = is_7 ? 5 : 4;
        match_pattern = 2;
        return true;
    } else if (n1 == n0 + 1 && n2 == n1 + 1 && n3 == n2 + 1 && n4 == n3 + 1 && n5 == n4 + 1) {
        bool is_7 = (n6 == n5 + 1);
        match_score = is_7 ? 990 : 920;
        match_tier = is_7 ? 5 : 4;
        match_pattern = 3;
        return true;
    } else if (n0 >= 5 && n1 == n0 - 1 && n2 == n1 - 1 && n3 == n2 - 1 && n4 == n3 - 1 && n5 == n4 - 1) {
        bool is_7 = (n5 >= 1 && n6 == n5 - 1);
        match_score = is_7 ? 990 : 920;
        match_tier = is_7 ? 5 : 4;
        match_pattern = 3;
        return true;
    }

    return false;
}

kernel void create2_search(
    constant uint8_t* factory [[buffer(0)]],
    constant uint8_t* init_hash [[buffer(1)]],
    constant uint64_t& base_salt [[buffer(2)]],
    constant uint32_t& iterations_per_thread [[buffer(3)]],
    constant uint32_t& min_zero_nibbles [[buffer(4)]],
    device atomic_uint* match_count [[buffer(5)]],
    device GpuMatch* matches [[buffer(6)]],
    constant uint32_t& max_matches [[buffer(7)]],
    constant uint8_t* target_nibbles [[buffer(8)]],
    constant uint32_t& target_len_nibbles [[buffer(9)]],
    uint id [[thread_position_in_grid]]
) {
    uint8_t block[136];
    for (int i = 0; i < 136; ++i) block[i] = 0;

    block[0] = 0xff;
    for (int i = 0; i < 20; ++i) {
        block[1 + i] = factory[i];
    }
    for (int i = 0; i < 32; ++i) {
        block[53 + i] = init_hash[i];
    }
    block[85] = 0x01;
    block[135] = 0x80;

    uint64_t thread_salt = base_salt + ((uint64_t)id * (uint64_t)iterations_per_thread);

    for (uint32_t iter = 0; iter < iterations_per_thread; ++iter) {
        uint64_t current_salt = thread_salt + iter;
        for (int i = 0; i < 8; ++i) {
            block[21 + i] = (uint8_t)(current_salt >> (i * 8));
        }

        uint64_t state[25];
        for (int i = 0; i < 17; ++i) {
            uint64_t word = 0;
            int offset = i * 8;
            for (int b = 0; b < 8; ++b) {
                word |= ((uint64_t)block[offset + b]) << (b * 8);
            }
            state[i] = word;
        }
        for (int i = 17; i < 25; ++i) {
            state[i] = 0;
        }

        keccak_f1600(state);

        uint8_t hash[32];
        for (int i = 0; i < 4; ++i) {
            uint64_t word = state[i];
            int offset = i * 8;
            for (int b = 0; b < 8; ++b) {
                hash[offset + b] = (uint8_t)(word >> (b * 8));
            }
        }

        uint8_t addr[20];
        for (int i = 0; i < 20; ++i) {
            addr[i] = hash[12 + i];
        }

        uint32_t match_score = 0;
        uint32_t match_tier = 0;
        uint32_t match_pattern = 0;

        if (evaluate_address(addr, target_nibbles, target_len_nibbles, min_zero_nibbles, match_score, match_tier, match_pattern)) {
            uint slot = atomic_fetch_add_explicit(match_count, 1, memory_order_relaxed);
            if (slot < max_matches) {
                matches[slot].salt_low = current_salt;
                matches[slot].salt_high = 0;
                for (int i = 0; i < 20; ++i) {
                    matches[slot].address[i] = addr[i];
                }
                matches[slot].score = match_score;
                matches[slot].tier = match_tier;
                matches[slot].pattern_type = match_pattern;
            }
        }
    }
}

struct fe256 {
    uint32_t d[8];
};

struct PointAff {
    fe256 x;
    fe256 y;
};

struct PointJac {
    fe256 x;
    fe256 y;
    fe256 z;
};

constant PointAff G_AFFINE = {
    { { 0x16f81798, 0x59f2815b, 0x2dce28d9, 0x029bfcdb, 0xce870b07, 0x55a06295, 0xf9dcbbac, 0x79be667e } },
    { { 0xfb10d4b8, 0x9c47d08f, 0xa6855419, 0xfd17b448, 0x0e1108a8, 0x5da4fbfc, 0x26a3c465, 0x483ada77 } }
};

fe256 fe_add(fe256 a, fe256 b) {
    fe256 r;
    uint64_t c = 0;
    for (int i = 0; i < 8; ++i) {
        c = (uint64_t)a.d[i] + (uint64_t)b.d[i] + (c >> 32);
        r.d[i] = (uint32_t)c;
    }
    uint64_t carry = c >> 32;

    uint64_t t = (uint64_t)r.d[0] + 0x3D1;
    uint32_t tr[8];
    tr[0] = (uint32_t)t;
    t = (uint64_t)r.d[1] + 1 + (t >> 32);
    tr[1] = (uint32_t)t;
    for (int i = 2; i < 8; ++i) {
        t = (uint64_t)r.d[i] + (t >> 32);
        tr[i] = (uint32_t)t;
    }
    if ((t >> 32) != 0 || carry != 0) {
        for (int i = 0; i < 8; ++i) r.d[i] = tr[i];
    }
    return r;
}

fe256 fe_sub(fe256 a, fe256 b) {
    fe256 r;
    uint32_t borrow = 0;
    for (int i = 0; i < 8; ++i) {
        uint64_t diff = (uint64_t)a.d[i] - (uint64_t)b.d[i] - borrow;
        r.d[i] = (uint32_t)diff;
        borrow = (diff >> 63) & 1;
    }
    if (borrow != 0) {
        uint64_t t = (uint64_t)r.d[0] - 0x3D1;
        r.d[0] = (uint32_t)t;
        uint32_t b2 = (t >> 63) & 1;
        t = (uint64_t)r.d[1] - 1 - b2;
        r.d[1] = (uint32_t)t;
        b2 = (t >> 63) & 1;
        for (int i = 2; i < 8; ++i) {
            t = (uint64_t)r.d[i] - b2;
            r.d[i] = (uint32_t)t;
            b2 = (t >> 63) & 1;
        }
    }
    return r;
}

fe256 fe_mul(fe256 a, fe256 b) {
    uint32_t prod[16];
    for (int i = 0; i < 16; ++i) prod[i] = 0;

    for (int i = 0; i < 8; ++i) {
        uint64_t carry = 0;
        for (int j = 0; j < 8; ++j) {
            uint64_t s = (uint64_t)a.d[i] * (uint64_t)b.d[j] + (uint64_t)prod[i + j] + carry;
            prod[i + j] = (uint32_t)s;
            carry = s >> 32;
        }
        int k = i + 8;
        while (carry > 0 && k < 16) {
            uint64_t s = (uint64_t)prod[k] + carry;
            prod[k] = (uint32_t)s;
            carry = s >> 32;
            k++;
        }
    }

    uint32_t acc[10];
    for (int i = 0; i < 8; ++i) acc[i] = prod[i];
    acc[8] = 0;
    acc[9] = 0;

    uint64_t c = 0;
    for (int i = 0; i < 8; ++i) {
        uint64_t h = (uint64_t)prod[8 + i];
        uint64_t s = (uint64_t)acc[i] + h * 977 + c;
        acc[i] = (uint32_t)s;
        c = s >> 32;
    }
    for (int i = 8; i < 10; ++i) {
        uint64_t s = (uint64_t)acc[i] + c;
        acc[i] = (uint32_t)s;
        c = s >> 32;
    }

    c = 0;
    for (int i = 0; i < 8; ++i) {
        uint64_t h = (uint64_t)prod[8 + i];
        uint64_t s = (uint64_t)acc[1 + i] + h + c;
        acc[1 + i] = (uint32_t)s;
        c = s >> 32;
    }
    uint64_t s9 = (uint64_t)acc[9] + c;
    acc[9] = (uint32_t)s9;

    uint64_t h2 = ((uint64_t)acc[9] << 32) | (uint64_t)acc[8];
    uint32_t acc2[8];
    for (int i = 0; i < 8; ++i) acc2[i] = acc[i];

    c = 0;
    uint64_t s0 = (uint64_t)acc2[0] + (h2 * 977) + c;
    acc2[0] = (uint32_t)s0;
    c = s0 >> 32;
    for (int i = 1; i < 8; ++i) {
        uint64_t s = (uint64_t)acc2[i] + c;
        acc2[i] = (uint32_t)s;
        c = s >> 32;
    }
    uint64_t extra_c = c;

    c = 0;
    uint64_t s1 = (uint64_t)acc2[1] + (h2 & 0xffffffffULL) + c;
    acc2[1] = (uint32_t)s1;
    c = s1 >> 32;
    for (int i = 2; i < 8; ++i) {
        uint64_t add_val = (i == 2) ? (h2 >> 32) : 0;
        uint64_t s = (uint64_t)acc2[i] + add_val + c;
        acc2[i] = (uint32_t)s;
        c = s >> 32;
    }
    extra_c += c;

    if (extra_c > 0) {
        c = 0;
        uint64_t se0 = (uint64_t)acc2[0] + extra_c * 977 + c;
        acc2[0] = (uint32_t)se0;
        c = se0 >> 32;
        uint64_t se1 = (uint64_t)acc2[1] + extra_c + c;
        acc2[1] = (uint32_t)se1;
        c = se1 >> 32;
        for (int i = 2; i < 8; ++i) {
            uint64_t s = (uint64_t)acc2[i] + c;
            acc2[i] = (uint32_t)s;
            c = s >> 32;
        }
    }

    uint64_t t = (uint64_t)acc2[0] + 0x3D1;
    uint32_t tr[8];
    tr[0] = (uint32_t)t;
    t = (uint64_t)acc2[1] + 1 + (t >> 32);
    tr[1] = (uint32_t)t;
    for (int i = 2; i < 8; ++i) {
        t = (uint64_t)acc2[i] + (t >> 32);
        tr[i] = (uint32_t)t;
    }
    if ((t >> 32) != 0) {
        for (int i = 0; i < 8; ++i) acc2[i] = tr[i];
    }

    fe256 res;
    for (int i = 0; i < 8; ++i) res.d[i] = acc2[i];
    return res;
}

inline fe256 fe_sqr(fe256 a) {
    return fe_mul(a, a);
}

fe256 fe_inv(fe256 x) {
    fe256 x2 = fe_mul(fe_sqr(x), x);
    fe256 x3 = fe_mul(fe_sqr(x2), x);
    fe256 x6 = fe_mul(fe_sqr(fe_sqr(fe_sqr(x3))), x3);
    fe256 x9 = fe_mul(fe_sqr(fe_sqr(fe_sqr(x6))), x3);
    fe256 x11 = fe_mul(fe_sqr(fe_sqr(x9)), x2);

    fe256 t = x11;
    for (int i = 0; i < 11; ++i) t = fe_sqr(t);
    fe256 x22 = fe_mul(t, x11);

    t = x22;
    for (int i = 0; i < 22; ++i) t = fe_sqr(t);
    fe256 x44 = fe_mul(t, x22);

    t = x44;
    for (int i = 0; i < 44; ++i) t = fe_sqr(t);
    fe256 x88 = fe_mul(t, x44);

    t = x88;
    for (int i = 0; i < 88; ++i) t = fe_sqr(t);
    fe256 x176 = fe_mul(t, x88);

    t = x176;
    for (int i = 0; i < 44; ++i) t = fe_sqr(t);
    fe256 x220 = fe_mul(t, x44);

    t = x220;
    for (int i = 0; i < 3; ++i) t = fe_sqr(t);
    fe256 x223 = fe_mul(t, x3);

    t = fe_sqr(x223);
    for (int i = 0; i < 22; ++i) t = fe_sqr(t);
    t = fe_mul(t, x22);

    for (int i = 0; i < 4; ++i) t = fe_sqr(t);
    t = fe_mul(fe_sqr(t), x);
    t = fe_sqr(t);
    t = fe_mul(fe_sqr(fe_sqr(t)), x2);
    t = fe_sqr(t);
    t = fe_mul(fe_sqr(t), x);
    return t;
}

PointJac point_add_mixed(PointJac P, PointAff Q) {
    bool p_is_inf = true;
    for (int i = 0; i < 8; ++i) {
        if (P.z.d[i] != 0) {
            p_is_inf = false;
            break;
        }
    }
    if (p_is_inf) {
        PointJac res;
        res.x = Q.x;
        res.y = Q.y;
        res.z.d[0] = 1;
        for (int i = 1; i < 8; ++i) res.z.d[i] = 0;
        return res;
    }

    fe256 z1z1 = fe_sqr(P.z);
    fe256 u2 = fe_mul(Q.x, z1z1);
    fe256 s2 = fe_mul(fe_mul(Q.y, P.z), z1z1);
    fe256 h = fe_sub(u2, P.x);
    fe256 r = fe_sub(s2, P.y);

    fe256 hh = fe_sqr(h);
    fe256 hhh = fe_mul(hh, h);
    fe256 v = fe_mul(P.x, hh);

    fe256 r2 = fe_sqr(r);
    fe256 v2 = fe_add(v, v);
    fe256 x3 = fe_sub(fe_sub(r2, hhh), v2);

    fe256 v_sub_x3 = fe_sub(v, x3);
    fe256 r_v_x3 = fe_mul(r, v_sub_x3);
    fe256 y1_hhh = fe_mul(P.y, hhh);
    fe256 y3 = fe_sub(r_v_x3, y1_hhh);

    fe256 z3 = fe_mul(P.z, h);

    PointJac res;
    res.x = x3;
    res.y = y3;
    res.z = z3;
    return res;
}

void eoa_pubkey_to_address(fe256 qx, fe256 qy, thread uint8_t* out_addr) {
    uint8_t block[136];
    for (int i = 0; i < 136; ++i) block[i] = 0;

    for (int i = 0; i < 8; ++i) {
        uint32_t limb_x = qx.d[7 - i];
        block[i * 4 + 0] = (uint8_t)(limb_x >> 24);
        block[i * 4 + 1] = (uint8_t)(limb_x >> 16);
        block[i * 4 + 2] = (uint8_t)(limb_x >> 8);
        block[i * 4 + 3] = (uint8_t)(limb_x);

        uint32_t limb_y = qy.d[7 - i];
        block[32 + i * 4 + 0] = (uint8_t)(limb_y >> 24);
        block[32 + i * 4 + 1] = (uint8_t)(limb_y >> 16);
        block[32 + i * 4 + 2] = (uint8_t)(limb_y >> 8);
        block[32 + i * 4 + 3] = (uint8_t)(limb_y);
    }

    block[64] = 0x01;
    block[135] = 0x80;

    uint64_t state[25];
    for (int i = 0; i < 17; ++i) {
        uint64_t word = 0;
        int offset = i * 8;
        for (int b = 0; b < 8; ++b) {
            word |= ((uint64_t)block[offset + b]) << (b * 8);
        }
        state[i] = word;
    }
    for (int i = 17; i < 25; ++i) {
        state[i] = 0;
    }

    keccak_f1600(state);

    uint8_t hash[32];
    for (int i = 0; i < 4; ++i) {
        uint64_t word = state[i];
        int offset = i * 8;
        for (int b = 0; b < 8; ++b) {
            hash[offset + b] = (uint8_t)(word >> (b * 8));
        }
    }

    for (int i = 0; i < 20; ++i) {
        out_addr[i] = hash[12 + i];
    }
}

kernel void eoa_search(
    constant PointAff& base_point [[buffer(0)]],
    constant PointAff* table1 [[buffer(1)]],
    constant PointAff* table2 [[buffer(2)]],
    constant PointAff* table3 [[buffer(3)]],
    constant uint32_t& total_threads [[buffer(4)]],
    constant uint32_t& min_zero_nibbles [[buffer(5)]],
    device atomic_uint* match_count [[buffer(6)]],
    device GpuMatch* matches [[buffer(7)]],
    constant uint32_t& max_matches [[buffer(8)]],
    constant uint8_t* target_nibbles [[buffer(9)]],
    constant uint32_t& target_len_nibbles [[buffer(10)]],
    uint id [[thread_position_in_grid]]
) {
    if (id >= total_threads) return;

    uint32_t idx = id * 8;
    uint32_t a = idx & 0xff;
    uint32_t b = (idx >> 8) & 0xff;
    uint32_t c = (idx >> 16) & 0xff;

    PointJac p0;
    p0.x = base_point.x;
    p0.y = base_point.y;
    p0.z.d[0] = 1;
    for (int i = 1; i < 8; ++i) p0.z.d[i] = 0;

    p0 = point_add_mixed(p0, table1[a]);
    p0 = point_add_mixed(p0, table2[b]);
    p0 = point_add_mixed(p0, table3[c]);

    PointJac pts[8];
    pts[0] = p0;
    for (int j = 1; j < 8; ++j) {
        pts[j] = point_add_mixed(pts[j - 1], G_AFFINE);
    }

    fe256 prod_z[8];
    prod_z[0] = pts[0].z;
    for (int j = 1; j < 8; ++j) {
        prod_z[j] = fe_mul(prod_z[j - 1], pts[j].z);
    }

    fe256 inv_all = fe_inv(prod_z[7]);

    fe256 z_inv[8];
    fe256 cur_inv = inv_all;
    for (int i = 7; i > 0; --i) {
        z_inv[i] = fe_mul(cur_inv, prod_z[i - 1]);
        cur_inv = fe_mul(cur_inv, pts[i].z);
    }
    z_inv[0] = cur_inv;

    for (int j = 0; j < 8; ++j) {
        fe256 zi2 = fe_sqr(z_inv[j]);
        fe256 zi3 = fe_mul(zi2, z_inv[j]);
        fe256 qx = fe_mul(pts[j].x, zi2);
        fe256 qy = fe_mul(pts[j].y, zi3);

        uint8_t addr[20];
        eoa_pubkey_to_address(qx, qy, addr);

        uint32_t match_score = 0;
        uint32_t match_tier = 0;
        uint32_t match_pattern = 0;

        if (evaluate_address(addr, target_nibbles, target_len_nibbles, min_zero_nibbles, match_score, match_tier, match_pattern)) {
            uint slot = atomic_fetch_add_explicit(match_count, 1, memory_order_relaxed);
            if (slot < max_matches) {
                matches[slot].salt_low = (uint64_t)idx + 65793ULL + (uint64_t)j;
                matches[slot].salt_high = 0;
                for (int i = 0; i < 20; ++i) {
                    matches[slot].address[i] = addr[i];
                }
                matches[slot].score = match_score;
                matches[slot].tier = match_tier;
                matches[slot].pattern_type = match_pattern;
            }
        }
    }
}
