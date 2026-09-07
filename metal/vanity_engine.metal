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

        bool matched = false;
        uint32_t match_score = 0;
        uint32_t match_tier = 0;
        uint32_t match_pattern = 0;

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
                matched = true;
                match_score = (match_len * 110 > 1000) ? 1000 : (match_len * 110);
                match_tier = (match_len >= target_len_nibbles) ? 5 : ((match_len >= 6) ? 4 : 3);
                match_pattern = match_len;
            }
        } else {
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
                matched = true;
                match_score = (zero_nibbles >= 9) ? 1000 : ((zero_nibbles >= 8) ? 980 : 920);
                match_tier = (zero_nibbles >= 9) ? 5 : ((zero_nibbles >= 8) ? 4 : 3);
                match_pattern = 1;
            } else {
                uint8_t n0 = addr[0] >> 4;
                uint8_t n1 = addr[0] & 0x0f;
                uint8_t n2 = addr[1] >> 4;
                uint8_t n3 = addr[1] & 0x0f;
                uint8_t n4 = addr[2] >> 4;
                uint8_t n5 = addr[2] & 0x0f;
                uint8_t n6 = addr[3] >> 4;
                uint8_t n7 = addr[3] & 0x0f;

                uint8_t e0 = addr[18] >> 4;
                uint8_t e1 = addr[18] & 0x0f;
                uint8_t e2 = addr[19] >> 4;
                uint8_t e3 = addr[19] & 0x0f;

                if (addr[0] == 0xde && addr[1] == 0xad && addr[2] == 0xbe && addr[3] == 0xef) {
                    matched = true;
                    match_score = 1000;
                    match_tier = 5;
                    match_pattern = 6;
                } else if (addr[0] == 0xca && addr[1] == 0xfe && addr[2] == 0xba && addr[3] == 0xbe) {
                    matched = true;
                    match_score = 1000;
                    match_tier = 5;
                    match_pattern = 6;
                } else if (addr[0] == 0xde && addr[1] == 0xad && addr[2] == 0xc0 && addr[3] == 0xde) {
                    matched = true;
                    match_score = 995;
                    match_tier = 5;
                    match_pattern = 6;
                } else if (addr[0] == 0xde && addr[1] == 0xf1 && addr[2] == 0x13 && addr[3] == 0x37) {
                    matched = true;
                    match_score = 990;
                    match_tier = 5;
                    match_pattern = 6;
                } else if (addr[0] == 0x13 && addr[1] == 0x37 && addr[2] == 0xc0 && addr[3] == 0xde) {
                    matched = true;
                    match_score = 990;
                    match_tier = 5;
                    match_pattern = 6;
                } else if (addr[0] == 0xba && addr[1] == 0xdb && addr[2] == 0xab && n6 == 0x0e) {
                    matched = true;
                    match_score = 980;
                    match_tier = 5;
                    match_pattern = 6;
                } else if (addr[0] == 0xc0 && addr[1] == 0xff && addr[2] == 0xee) {
                    matched = true;
                    match_score = 940;
                    match_tier = 4;
                    match_pattern = 6;
                } else if (addr[0] == 0xba && addr[1] == 0xda && addr[2] == 0x55) {
                    matched = true;
                    match_score = 930;
                    match_tier = 4;
                    match_pattern = 6;
                } else if (addr[0] == 0xde && addr[1] == 0xad && addr[18] == 0xbe && addr[19] == 0xef) {
                    matched = true;
                    match_score = 999;
                    match_tier = 5;
                    match_pattern = 4;
                } else if (addr[0] == 0xca && addr[1] == 0xfe && addr[18] == 0xba && addr[19] == 0xbe) {
                    matched = true;
                    match_score = 999;
                    match_tier = 5;
                    match_pattern = 4;
                } else if (addr[0] == 0xfa && addr[1] == 0xce && addr[18] == 0xde && addr[19] == 0xad) {
                    matched = true;
                    match_score = 990;
                    match_tier = 5;
                    match_pattern = 4;
                } else if (addr[0] == 0x77 && addr[1] == 0x77 && addr[18] == 0x77 && addr[19] == 0x77) {
                    matched = true;
                    match_score = 999;
                    match_tier = 5;
                    match_pattern = 4;
                } else if (addr[0] == 0x77 && n2 == 7 && e1 == 7 && addr[19] == 0x77) {
                    matched = true;
                    match_score = 950;
                    match_tier = 4;
                    match_pattern = 4;
                } else if (addr[0] == 0x00 && addr[1] == 0x00 && addr[18] == 0x00 && addr[19] == 0x00) {
                    matched = true;
                    match_score = 999;
                    match_tier = 5;
                    match_pattern = 4;
                } else if (addr[0] == 0x00 && n2 == 0 && e1 == 0 && addr[19] == 0x00) {
                    matched = true;
                    match_score = 940;
                    match_tier = 4;
                    match_pattern = 4;
                } else if (addr[0] == 0x88 && n2 == 8 && e1 == 8 && addr[19] == 0x88) {
                    matched = true;
                    match_score = 940;
                    match_tier = 4;
                    match_pattern = 4;
                } else if (addr[0] == 0x66 && n2 == 6 && e1 == 6 && addr[19] == 0x66) {
                    matched = true;
                    match_score = 930;
                    match_tier = 4;
                    match_pattern = 4;
                } else if (
                    (addr[0] == 0x01 && addr[1] == 0x01 && addr[2] == 0x01 && addr[3] == 0x01) ||
                    (addr[0] == 0x10 && addr[1] == 0x10 && addr[2] == 0x10 && addr[3] == 0x10) ||
                    (addr[0] == 0x69 && addr[1] == 0x69 && addr[2] == 0x69 && addr[3] == 0x69) ||
                    (addr[0] == 0x00 && addr[1] == 0x11 && addr[2] == 0x00 && addr[3] == 0x11) ||
                    (addr[0] == 0x11 && addr[1] == 0x00 && addr[2] == 0x11 && addr[3] == 0x00)
                ) {
                    matched = true;
                    match_score = 910;
                    match_tier = 4;
                    match_pattern = 5;
                } else if (n0 != 0 && n0 == n1 && n1 == n2 && n2 == n3 && n3 == n4 && n4 == n5) {
                    bool is_7 = (n5 == n6);
                    matched = true;
                    match_score = is_7 ? 999 : 930;
                    match_tier = is_7 ? 5 : 4;
                    match_pattern = 2;
                } else if (n1 == n0 + 1 && n2 == n1 + 1 && n3 == n2 + 1 && n4 == n3 + 1 && n5 == n4 + 1) {
                    bool is_7 = (n6 == n5 + 1);
                    matched = true;
                    match_score = is_7 ? 990 : 920;
                    match_tier = is_7 ? 5 : 4;
                    match_pattern = 3;
                } else if (n0 >= 5 && n1 == n0 - 1 && n2 == n1 - 1 && n3 == n2 - 1 && n4 == n3 - 1 && n5 == n4 - 1) {
                    bool is_7 = (n5 >= 1 && n6 == n5 - 1);
                    matched = true;
                    match_score = is_7 ? 990 : 920;
                    match_tier = is_7 ? 5 : 4;
                    match_pattern = 3;
                }
            }
        }

        if (matched) {
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
