#include "arm.h"
#include "memlayout.h"

.equ _EL2, 0x8
.equ _core_id_mask, 0b11

        # qemu -kernel loads the kernel at 0x80000000
        # and causes each CPU to jump there.
        # kernel.ld causes the following code to
        # be placed at 0x80000000.
.section .text
_entry:
        # initialize stack pointers for svc modes
	mov     x0, #1     // select SP_EL1
	msr     spsel, x0
	isb

	# set up a stack for C.
        # stack0 is declared in start.c,
        # with a 4096-byte stack per CPU.
        # sp = stack0 + (hartid * 4096)
        // la sp, stack0
        adrp x0, stack0
        mov sp, x0
        // li a0, 1024*4
        mov x0, #4096
	// csrr a1, mhartid
        mrs x1, MPIDR_EL1
        and x1, x1, _core_id_mask
        // addi a1, a1, 1
        add x1, x1, #1
        // mul a0, a0, a1
        mul x0, x0, x1
        // add sp, sp, a0
        add sp, sp, x0
	# jump to start() in start.c
        b start
spin:
        b spin
