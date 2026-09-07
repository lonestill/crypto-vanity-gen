#import <Metal/Metal.h>
#import <Foundation/Foundation.h>
#include "metal_bridge.h"

static id<MTLDevice> g_device = nil;
static id<MTLCommandQueue> g_queue = nil;
static id<MTLComputePipelineState> g_create2_pipeline = nil;
static id<MTLComputePipelineState> g_eoa_pipeline = nil;
static id<MTLBuffer> g_buf_count = nil;
static id<MTLBuffer> g_buf_matches = nil;
static id<MTLBuffer> g_buf_table1 = nil;
static id<MTLBuffer> g_buf_table2 = nil;
static id<MTLBuffer> g_buf_table3 = nil;
static NSLock* g_lock = nil;
static int g_ref_count = 0;
static char g_device_name[128] = {0};

int metal_bridge_is_supported(void) {
    @autoreleasepool {
        id<MTLDevice> device = MTLCreateSystemDefaultDevice();
        if (device != nil) {
            return 1;
        }
        return 0;
    }
}

const char* metal_bridge_get_device_name(void) {
    if (g_device_name[0] != 0) {
        return g_device_name;
    }
    @autoreleasepool {
        id<MTLDevice> device = MTLCreateSystemDefaultDevice();
        if (device != nil) {
            strncpy(g_device_name, [[device name] UTF8String], sizeof(g_device_name) - 1);
            return g_device_name;
        }
        return "Unknown Metal Device";
    }
}

int metal_bridge_init_engine(const char* msl_source) {
    @autoreleasepool {
        if (g_lock == nil) {
            g_lock = [[NSLock alloc] init];
        }
        [g_lock lock];

        if (g_ref_count > 0 && g_device != nil && g_create2_pipeline != nil && g_eoa_pipeline != nil) {
            g_ref_count++;
            [g_lock unlock];
            return 0;
        }

        if (g_device == nil) {
            g_device = MTLCreateSystemDefaultDevice();
        }
        if (g_device == nil) {
            [g_lock unlock];
            return -1;
        }

        strncpy(g_device_name, [[g_device name] UTF8String], sizeof(g_device_name) - 1);

        if (g_queue == nil) {
            g_queue = [g_device newCommandQueue];
        }

        NSString* sourceStr = [NSString stringWithUTF8String:msl_source];
        NSError* err = nil;
        id<MTLLibrary> library = [g_device newLibraryWithSource:sourceStr options:nil error:&err];
        if (library == nil) {
            [g_lock unlock];
            return -2;
        }

        id<MTLFunction> create2_fn = [library newFunctionWithName:@"create2_search"];
        if (create2_fn == nil) {
            [g_lock unlock];
            return -3;
        }

        g_create2_pipeline = [g_device newComputePipelineStateWithFunction:create2_fn error:&err];
        if (g_create2_pipeline == nil) {
            [g_lock unlock];
            return -4;
        }

        id<MTLFunction> eoa_fn = [library newFunctionWithName:@"eoa_search"];
        if (eoa_fn == nil) {
            [g_lock unlock];
            return -5;
        }

        g_eoa_pipeline = [g_device newComputePipelineStateWithFunction:eoa_fn error:&err];
        if (g_eoa_pipeline == nil) {
            [g_lock unlock];
            return -6;
        }

        if (g_buf_count == nil) {
            g_buf_count = [g_device newBufferWithLength:sizeof(uint32_t) options:MTLResourceStorageModeShared];
        }
        if (g_buf_matches == nil) {
            g_buf_matches = [g_device newBufferWithLength:256 * sizeof(MetalMatchRecord) options:MTLResourceStorageModeShared];
        }

        g_ref_count = 1;
        [g_lock unlock];
        return 0;
    }
}

int metal_bridge_init_tables(
    const MetalPointAff* table1,
    const MetalPointAff* table2,
    const MetalPointAff* table3
) {
    if (g_device == nil) {
        return -1;
    }
    @autoreleasepool {
        [g_lock lock];
        if (g_buf_table1 == nil) {
            NSUInteger table_size = 256 * sizeof(MetalPointAff);
            g_buf_table1 = [g_device newBufferWithBytes:table1 length:table_size options:MTLResourceStorageModeShared];
            g_buf_table2 = [g_device newBufferWithBytes:table2 length:table_size options:MTLResourceStorageModeShared];
            g_buf_table3 = [g_device newBufferWithBytes:table3 length:table_size options:MTLResourceStorageModeShared];
        }
        [g_lock unlock];
        if (g_buf_table1 == nil || g_buf_table2 == nil || g_buf_table3 == nil) {
            return -2;
        }
        return 0;
    }
}

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
) {
    if (g_device == nil || g_queue == nil || g_create2_pipeline == nil || g_buf_count == nil || g_buf_matches == nil) {
        return -1;
    }

    @autoreleasepool {
        [g_lock lock];
        *(uint32_t*)[g_buf_count contents] = 0;

        uint32_t nibble_len = (target_len_nibbles > 0) ? target_len_nibbles : 1;

        id<MTLCommandBuffer> cmd = [g_queue commandBuffer];
        id<MTLComputeCommandEncoder> enc = [cmd computeCommandEncoder];

        [enc setComputePipelineState:g_create2_pipeline];
        [enc setBytes:factory length:20 atIndex:0];
        [enc setBytes:init_hash length:32 atIndex:1];
        [enc setBytes:&base_salt length:sizeof(uint64_t) atIndex:2];
        [enc setBytes:&iters_per_thread length:sizeof(uint32_t) atIndex:3];
        [enc setBytes:&min_zeros length:sizeof(uint32_t) atIndex:4];
        [enc setBuffer:g_buf_count offset:0 atIndex:5];
        [enc setBuffer:g_buf_matches offset:0 atIndex:6];
        [enc setBytes:&max_matches length:sizeof(uint32_t) atIndex:7];
        [enc setBytes:target_nibbles length:nibble_len atIndex:8];
        [enc setBytes:&target_len_nibbles length:sizeof(uint32_t) atIndex:9];

        MTLSize grid = MTLSizeMake(total_threads, 1, 1);
        NSUInteger threadGroupSize = g_create2_pipeline.maxTotalThreadsPerThreadgroup;
        if (threadGroupSize > 256) {
            threadGroupSize = 256;
        }
        MTLSize group = MTLSizeMake(threadGroupSize, 1, 1);

        [enc dispatchThreads:grid threadsPerThreadgroup:group];
        [enc endEncoding];

        [cmd commit];
        [cmd waitUntilCompleted];

        uint32_t found = *(uint32_t*)[g_buf_count contents];
        if (found > max_matches) {
            found = max_matches;
        }
        *out_match_count = found;

        if (found > 0) {
            memcpy(out_matches, [g_buf_matches contents], found * sizeof(MetalMatchRecord));
        }

        [g_lock unlock];
        return 0;
    }
}

int metal_bridge_run_eoa_batch(
    const MetalPointAff* base_point,
    uint32_t total_threads,
    uint32_t min_zeros,
    const uint8_t* target_nibbles,
    uint32_t target_len_nibbles,
    MetalMatchRecord* out_matches,
    uint32_t max_matches,
    uint32_t* out_match_count
) {
    if (g_device == nil || g_queue == nil || g_eoa_pipeline == nil || g_buf_count == nil || g_buf_matches == nil || g_buf_table1 == nil || g_buf_table2 == nil || g_buf_table3 == nil) {
        return -1;
    }

    @autoreleasepool {
        [g_lock lock];
        *(uint32_t*)[g_buf_count contents] = 0;

        uint32_t nibble_len = (target_len_nibbles > 0) ? target_len_nibbles : 1;

        id<MTLCommandBuffer> cmd = [g_queue commandBuffer];
        id<MTLComputeCommandEncoder> enc = [cmd computeCommandEncoder];

        [enc setComputePipelineState:g_eoa_pipeline];
        [enc setBytes:base_point length:sizeof(MetalPointAff) atIndex:0];
        [enc setBuffer:g_buf_table1 offset:0 atIndex:1];
        [enc setBuffer:g_buf_table2 offset:0 atIndex:2];
        [enc setBuffer:g_buf_table3 offset:0 atIndex:3];
        [enc setBytes:&total_threads length:sizeof(uint32_t) atIndex:4];
        [enc setBytes:&min_zeros length:sizeof(uint32_t) atIndex:5];
        [enc setBuffer:g_buf_count offset:0 atIndex:6];
        [enc setBuffer:g_buf_matches offset:0 atIndex:7];
        [enc setBytes:&max_matches length:sizeof(uint32_t) atIndex:8];
        [enc setBytes:target_nibbles length:nibble_len atIndex:9];
        [enc setBytes:&target_len_nibbles length:sizeof(uint32_t) atIndex:10];

        MTLSize grid = MTLSizeMake(total_threads, 1, 1);
        NSUInteger threadGroupSize = g_eoa_pipeline.maxTotalThreadsPerThreadgroup;
        if (threadGroupSize > 256) {
            threadGroupSize = 256;
        }
        MTLSize group = MTLSizeMake(threadGroupSize, 1, 1);

        [enc dispatchThreads:grid threadsPerThreadgroup:group];
        [enc endEncoding];

        [cmd commit];
        [cmd waitUntilCompleted];

        uint32_t found = *(uint32_t*)[g_buf_count contents];
        if (found > max_matches) {
            found = max_matches;
        }
        *out_match_count = found;

        if (found > 0) {
            memcpy(out_matches, [g_buf_matches contents], found * sizeof(MetalMatchRecord));
        }

        [g_lock unlock];
        return 0;
    }
}

void metal_bridge_release(void) {
    if (g_lock == nil) return;
    [g_lock lock];
    g_ref_count--;
    if (g_ref_count <= 0) {
        g_ref_count = 0;
        g_buf_table3 = nil;
        g_buf_table2 = nil;
        g_buf_table1 = nil;
        g_buf_matches = nil;
        g_buf_count = nil;
        g_eoa_pipeline = nil;
        g_create2_pipeline = nil;
        g_queue = nil;
        g_device = nil;
    }
    [g_lock unlock];
}
