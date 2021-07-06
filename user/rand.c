#include "user/rand.h"

static unsigned long next = 1;

// #ifdef NDEF_RAND
int rand(void) {
    next = next * 1103515245 + 12345;
    return((unsigned)(next/65536) % 32768);
}
// #endif

// #ifndef SRAND_DEF
// #define SRAND_DEF
void srand(unsigned seed) {
    next = seed;
}
// #endif