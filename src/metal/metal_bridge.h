#ifndef METAL_BRIDGE_H
#define METAL_BRIDGE_H

#include <stdint.h>

#ifdef __cplusplus
extern C {
#endif

typedef struct {
    uint64_t salt_low;
    uint64_t salt_high;
    uint8_t address[20];
    uint32_t score;
    uint32_t tier;
    uint32_t pattern_type;
} MetalMatchRecord;

typedef struct {
    uint32_t x[8];
    uint32_t y[8];
} MetalPointAff;

int metal_bridge_is_supported(void);
const char* metal_bridge_get_device_name(void);
int metal_bridge_init_engine(const char* msl_source);
int metal_bridge_init_tables(
    const MetalPointAff* table1,
    const MetalPointAff* table2,
    const MetalPointAff* table3
);
int metal_bridge_run_create2_batch(
    const uint8_t* factory,
    const uint8_t* init_hash,
    uint64_t base_salt,
    uint32_t total_threads,
    uint32_t iters_per_thread,
    uint32_t min_zeros,
    const uint8_t* target_nibbles,
    uint32_t target_len_nibbles,
    MetalMatchRecord* out_matches,
    uint32_t max_matches,
    uint32_t* out_match_count
);
int metal_bridge_run_eoa_batch(
    const MetalPointAff* base_point,
    uint32_t total_threads,
    uint32_t min_zeros,
    const uint8_t* target_nibbles,
    uint32_t target_len_nibbles,
    MetalMatchRecord* out_matches,
    uint32_t max_matches,
    uint32_t* out_match_count
);
void metal_bridge_release(void);

#ifdef __cplusplus
}
#endif

#endif
