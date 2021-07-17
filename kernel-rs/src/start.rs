use crate::{
    arch::memlayout::{clint_mtimecmp, CLINT_MTIME},
    arch::riscv::{
        r_mhartid, w_medeleg, w_mepc, w_mideleg, w_mscratch, w_mtvec, w_satp, w_tp, Mstatus, MIE,
        SIE,
    },
    arch::arm::*,
    kernel::main,
    param::NCPU,
    arch::arm_virt::*,
    uart::Uart,
};

use cortex_a::{asm::barrier, registers::*};
use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};

extern "C" {
    // assembly code in kernelvec.S for machine-mode timer interrupt.
    fn timervec();
    fn vectors();
}

/// entry.S needs one stack per CPU.
#[repr(C, align(16))]
pub struct Stack([[u8; 4096]; NCPU]);

impl Stack {
    const fn new() -> Self {
        Self([[0; 4096]; NCPU])
    }
}

#[no_mangle]
pub static mut stack0: Stack = Stack::new();

/// A scratch area per CPU for machine-mode timer interrupts.
static mut TIMER_SCRATCH: [[usize; NCPU]; 5] = [[0; NCPU]; 5];

/// entry.S jumps here in machine mode on stack0.
#[no_mangle]
pub unsafe fn start() {
    let cur_el = r_currentel();
    
    match cur_el {
        0 => _puts("current el: 0\n"),
        1 => _puts("current el: 1\n"),
        2 => _puts("current el: 2\n"),
        3 => _puts("current el: 3\n"),
        _ => _puts("current el: unknown\n"),
    }

    // flush TLB and cache
    _puts("Flushing TLB and instr cache\n");

    // flush Instr Cache
    ic_ialluis();

    // flush TLB
    tlbi_vmalle1();
    unsafe { barrier::dsb(barrier::SY) } ;
    
    // no trapping on FP/SIMD instructions
    unsafe { w_cpacr_el1(0x03 << 20) };
    
    // monitor debug: all disabled
    unsafe { w_mdscr_el1(0) };
    
    // set_up_mair
    // TODO: This setting might be problematic.
    MAIR_EL1.write(
        // Attribute 1 - Cacheable normal DRAM.
        MAIR_EL1::Attr1_Normal_Outer::WriteBack_NonTransient_ReadWriteAlloc +
        MAIR_EL1::Attr1_Normal_Inner::WriteBack_NonTransient_ReadWriteAlloc +
        // Attribute 0 - Device.
        MAIR_EL1::Attr0_Device::nonGathering_nonReordering_EarlyWriteAck,
    );
    
    // configure transaltion control register
    TCR_EL1.write(
        TCR_EL1::TBI1::Used
        + TCR_EL1::IPS::Bits_44 // intermediate physical address = 44bits
        + TCR_EL1::TG1::KiB_4 // transaltion granule = 4KB
        + TCR_EL1::SH0::Inner
        + TCR_EL1::SH1::Inner // Inner Shareable
        + TCR_EL1::IRGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
        + TCR_EL1::ORGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
        + TCR_EL1::IRGN1::WriteBack_ReadAlloc_WriteAlloc_Cacheable
        + TCR_EL1::ORGN1::WriteBack_ReadAlloc_WriteAlloc_Cacheable
        + TCR_EL1::EPD0::EnableTTBR0Walks
        + TCR_EL1::EPD1::EnableTTBR1Walks
        + TCR_EL1::A1::TTBR0 // use TTBR0_EL1's ASID as an ASID
        + TCR_EL1::T0SZ.val(32) // this can be changed, possible up to 44
        + TCR_EL1::T1SZ.val(32) // this can be changed, possible up to 44
        + TCR_EL1::AS::ASID16Bits // the upper 16 bits of TTBR0_EL1 and TTBR1_EL1 are used for allocation and matching in the TLB.
        + TCR_EL1::TBI0::Ignored // this may not be needed
    );
    
    // set vector base address register
    _puts("Setting Vector Base Addcress Register (VBAR_EL1)\n");
    VBAR_EL1.set(vectors as _);

    // set system contol register
    // Enable the MMU and turn on data and instruction caching.
    _puts("Setting System Control Register (SCTLR_EL1)\n");
    SCTLR_EL1.modify(SCTLR_EL1::M::Enable + SCTLR_EL1::C::Cacheable + SCTLR_EL1::I::Cacheable);

    // Force MMU init to complete before next instruction.
    unsafe { barrier::isb(barrier::SY) } ;

    unsafe {
        main();
    }
}

/// set up to receive timer interrupts in machine mode,
/// which arrive at timervec in kernelvec.S,
/// which turns them into software interrupts for devintr() in trap.c.
unsafe fn timerinit() {
    // each CPU has a separate source of timer interrupts.
    let id = r_mhartid();

    // ask the CLINT for a timer interrupt.
    let interval: usize = 1_000_000; // cycles; about 1/10th second in qemu.
    unsafe { *(clint_mtimecmp(id) as *mut usize) = (*(CLINT_MTIME as *mut usize)) + interval };

    // prepare information in scratch[] for timervec.
    // scratch[0..2] : space for timervec to save registers.
    // scratch[3] : address of CLINT MTIMECMP register.
    // scratch[4] : desired interval (in cycles) between timer interrupts.
    let scratch = unsafe { &mut TIMER_SCRATCH[id][..] };
    *unsafe { scratch.get_unchecked_mut(3) } = clint_mtimecmp(id);
    *unsafe { scratch.get_unchecked_mut(4) } = interval;
    unsafe { w_mscratch(&scratch[0] as *const _ as usize) };

    // set the machine-mode trap handler.
    unsafe { w_mtvec(timervec as _) };

    // enable machine-mode interrupts.
    let mut x = Mstatus::read();
    x.insert(Mstatus::MIE);
    unsafe { x.write() };

    // enable machine-mode timer interrupts.
    let mut y = MIE::read();
    y.insert(MIE::MTIE);
    unsafe { y.write() };
}

fn _puts(s: &str)
{
    for c in s.chars() {
        uart_putc(c as u8);
    }
}

fn uart_putc(c: u8)
{
    let ptr: *mut u8 = UART0 as *mut u8;

    let u_art = unsafe { Uart::new(UART0) } ;
    u_art.putc(c);
}
