#include <stdio.h>
#include <stdint.h>

int main() {
    uint32_t l = 0xFFFFFFFE; // 4294967294
    uint8_t* p = (uint8_t*)0x100000;
    uint8_t* end = (uint8_t*)0x101000; // 4096 bytes buffer
    
    // The exact expression from cw_pack_str (line 276)
    // and cw_pack_reserve_space (line 138)
    uint8_t* nyp = p + (l + 5);
    
    printf("sizeof(p)=%zu, sizeof(l+5)=%zu\n", sizeof(p), sizeof(l+5)); printf("l = %u (0x%X)\n", l, l);
    printf("p = %p\n", (void*)p);
    printf("end = %p\n", (void*)end);
    printf("nyp = p + (l + 5) = %p\n", (void*)nyp);
    
    if (nyp > end) {
        printf("Check: nyp > end -> TRUE. Handler called.\n");
    } else {
        printf("Check: nyp > end -> FALSE. Handler bypassed!\n");
    }
    return 0;
}
