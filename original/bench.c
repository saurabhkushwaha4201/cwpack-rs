#include <stdio.h>
#include <time.h>
#include "src/cwpack.h"

int main() {
    uint8_t buf[1024];
    cw_pack_context ctx;
    clock_t start = clock();
    
    for (volatile int i = 0; i < 10000000; i++) {
        cw_pack_context_init(&ctx, buf, sizeof(buf), NULL);
        cw_pack_array_size(&ctx, 3);
        cw_pack_unsigned(&ctx, 42);
        cw_pack_str(&ctx, "hello world", 11);
        cw_pack_map_size(&ctx, 1);
        cw_pack_str(&ctx, "key", 3);
        cw_pack_boolean(&ctx, true);
    }
    
    clock_t end = clock();
    double time_spent = (double)(end - start) / CLOCKS_PER_SEC;
    printf("C packing time (10M iters): %f seconds (last byte: %d)\n", time_spent, buf[(ctx.current - ctx.start) - 1]);
    
    cw_pack_context_init(&ctx, buf, sizeof(buf), NULL);
    cw_pack_array_size(&ctx, 3);
    cw_pack_unsigned(&ctx, 42);
    cw_pack_str(&ctx, "hello world", 11);
    cw_pack_map_size(&ctx, 1);
    cw_pack_str(&ctx, "key", 3);
    cw_pack_boolean(&ctx, true);
    size_t packed_len = ctx.current - ctx.start;
    
    cw_unpack_context uctx;
    start = clock();
    volatile int dummy = 0;
    for (volatile int i = 0; i < 10000000; i++) {
        cw_unpack_context_init(&uctx, buf, packed_len, NULL);
        cw_unpack_next(&uctx);
        cw_unpack_next(&uctx);
        cw_unpack_next(&uctx);
        cw_unpack_next(&uctx);
        cw_unpack_next(&uctx);
        cw_unpack_next(&uctx);
        dummy = uctx.return_code;
    }
    end = clock();
    time_spent = (double)(end - start) / CLOCKS_PER_SEC;
    printf("C unpacking time (10M iters): %f seconds (ret: %d)\n", time_spent, dummy);

    return 0;
}
