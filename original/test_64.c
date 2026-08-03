#include <stdio.h>
#include <stdint.h>

int main() {
    uint32_t l = 0xFFFFFFFE; 
    uint64_t p_val = 0x10000000000ULL;
    uint64_t end_val = 0x10000001000ULL;
    
    // Simulate 64-bit pointer arithmetic
    uint8_t* p = (uint8_t*)p_val;
    uint8_t* end = (uint8_t*)end_val;
    
    uint8_t* nyp = p + (l + 5);
    
    printf("sizeof(void*) simulated as: 8, sizeof(l+5)=%zu\n", sizeof(l+5));
    printf("p = %llx\n", (unsigned long long)p);
    printf("end = %llx\n", (unsigned long long)end);
    printf("nyp = %llx\n", (unsigned long long)nyp);
    
    if (nyp > end) {
        printf("Check: nyp > end -> TRUE. Handler called.\n");
    } else {
        printf("Check: nyp > end -> FALSE. Handler bypassed!\n");
    }
    return 0;
}
