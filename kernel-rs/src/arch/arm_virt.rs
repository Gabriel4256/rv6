//
// Board specific information for the ARM Virtual Machine
//

// We assume it has 128MB instead. During boot, the lower
// 64MB memory is mapped to the flash, needs to be remapped
// the the SDRAM. We skip this for QEMU
pub const PHY_START: usize = 0x40000000;
pub const PHY_STOP: usize = 0x08000000 + PHY_START;

pub const DEVBASE1:usize = 0x08000000;
pub const DEVBASE2:usize = 0x09000000;
pub const DEVBASE3:usize = 0x0a000000;
pub const DEV_MEM_SZ:usize = 0x01000000;


pub const UART0:usize = 0x09000000;
pub const UART_CLK:usize = 24000000;    // Clock rate for pub const

pub const TIMER0:usize = 0x1c110000;
pub const TIMER1:usize = 0x1c120000;
pub const CLK_HZ:usize = 1000000;     // the clock is pub const

pub const VIC_BASE:usize = 0x08000000;
pub const PIC_TIMER01:usize = 13;
pub const PIC_TIMER23:usize = 11;
pub const PIC_UART0:usize = 1;
pub const PIC_GRAPHIC:usize = 19;
