//! Utilities.

// Dead code is allowed in this file because not all components are used in the kernel.
#![allow(dead_code)]

pub mod branded;
pub mod etrace;
pub mod list;
pub mod list2;
pub mod pinned_array;
pub mod rc_cell;

pub fn spin_loop() -> ! {
    loop {
        ::core::hint::spin_loop();
    }
}
