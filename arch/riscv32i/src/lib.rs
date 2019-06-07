#![crate_name = "riscv32i"]
#![crate_type = "rlib"]
#![feature(asm, const_fn, lang_items, global_asm)]
#![feature(crate_visibility_modifier)]
#![no_std]

#[macro_use(register_bitfields, register_bitmasks, debug_gpio, debug)]
extern crate kernel;

pub mod plic;
pub mod support;
pub mod syscall;
pub mod clic;
pub mod machine_timer;

extern "C" {
    // External function defined by the board main.rs.
    fn reset_handler();

    // Where the end of the stack region is (and hence where the stack should
    // start).
    static _estack: u32;

    // // Address of _start_trap.
    // static _start_trap: u32;

    // Boundaries of the .bss section.
    static mut _szero: u32;
    static mut _ezero: u32;

    // Where the .data section is stored in flash.
    static mut _etext: u32;

    // Boundaries of the .data section.
    static mut _srelocate: u32;
    static mut _erelocate: u32;
  }

/// Entry point of all programs (`_start`).
///
/// It initializes the stack pointer, the frame pointer (needed for closures to
/// work in start_rust) and the global pointer. Then it calls `reset_handler()`,
/// the main entry point for Tock boards.
#[link_section = ".riscv.start"]
#[export_name = "_start"]
pub extern "C" fn _start() {

    unsafe {
        asm! ("
            // Set the global pointer register using the variable defined in the
            // linker script. This register is only set once. The global pointer
            // is a method for sharing state between the linker and the CPU so
            // that the linker can emit code with offsets that are relative to
            // the gp register, and the CPU can successfully execute them.
            //
            // https://gnu-mcu-eclipse.github.io/arch/riscv/programmer/#the-gp-global-pointer-register
            // https://groups.google.com/a/groups.riscv.org/forum/#!msg/sw-dev/60IdaZj27dY/5MydPLnHAQAJ
            // https://www.sifive.com/blog/2017/08/28/all-aboard-part-3-linker-relaxation-in-riscv-toolchain/
            //
            lui  gp, %hi(__global_pointer$$)     // Set the global pointer.
            addi gp, gp, %lo(__global_pointer$$) // Value set in linker script.

            // Initialize the stack pointer register. This comes directly from
            // the linker script.
            lui  sp, %hi(_estack)     // Set the initial stack pointer.
            addi sp, sp, %lo(_estack) // Value from the linker script.

            // Set s0 (the frame pointer) to the start of the stack.
            add  s0, sp, zero

            // With that initial setup out of the way, we now branch to the main
            // code, likely defined in a board's main.rs.
            j    reset_handler
        "
        :
        :
        :
        : "volatile");
    }
}

// /// Setup memory for the kernel.
// ///
// /// This moves the data segment from flash to RAM and zeros out the BSS section.
// pub unsafe fn init_memory() {
//     tock_rt0::init_data(&mut _etext, &mut _srelocate, &mut _erelocate);
//     tock_rt0::zero_bss(&mut _szero, &mut _ezero);
// }

/// Setup memory for the kernel.
///
/// This moves the data segment from flash to RAM and zeros out the BSS section.
pub unsafe fn init_memory() {
    // Relocate data segment.
    // Assumes data starts right after text segment as specified by the linker
    // file.
    let mut pdest = &mut _srelocate as *mut u32;
    let pend = &mut _erelocate as *mut u32;
    let mut psrc = &_etext as *const u32;

    if psrc != pdest {
        while (pdest as *const u32) < pend {
            *pdest = *psrc;
            pdest = pdest.offset(1);
            psrc = psrc.offset(1);
        }
    }

    // Clear the zero segment (BSS)
    let pzero = &_ezero as *const u32;
    pdest = &mut _szero as *mut u32;

    while (pdest as *const u32) < pzero {
        *pdest = 0;
        pdest = pdest.offset(1);
    }
}

/// Tell the MCU what address the trap handler is located at. The trap handler
/// is called on exceptions and for interrupts.
///
/// This is a generic implementation. There may be board specific versions as
/// some platforms have added more bits to the `mtvec` register.
pub unsafe fn configure_trap_handler() {
    asm!("
        // The csrw instruction writes a Control and Status Register (CSR)
        // with a new value.
        //
        // CSR 0x305 (mtvec, 'Machine trap-handler base address.') sets the address
        // of the trap handler. We do not care about its old value, so we don't
        // bother reading it.
        csrw 0x305, $0        // Write the mtvec CSR.
    "
    :
    : "r"(&_start_trap)
    :
    : "volatile");
}

#[link_section = ".riscv.trap"]
#[export_name = "_start_trap"]
pub extern "C" fn _start_trap() {
    unsafe {
        asm! ("

            // First check which privilege level we came from. If we came from
            // user mode then we need to handle that differently from if we came
            // from kernel mode. Luckily in the E21, the MPP bits are included
            // in the mcause register.
            csrr t0, 0x342              // CSR=0x342=mcause
            srli t1, t0, 28             // Shift the mcause 28 bits to the right (MPP bits)
            andi t1, t1, 0x3            // `and` to get only the bottom two MPP bits
            beq  t1, x0, _from_app      // If MPP=00 then we came from user mode



            // If we came from mcause.MPP=11 then we came from the kernel.


        _from_kernel:
            addi sp, sp, -16*4

            sw ra, 0*4(sp)
            sw t0, 1*4(sp)
            sw t1, 2*4(sp)
            sw t2, 3*4(sp)
            sw t3, 4*4(sp)
            sw t4, 5*4(sp)
            sw t5, 6*4(sp)
            sw t6, 7*4(sp)
            sw a0, 8*4(sp)
            sw a1, 9*4(sp)
            sw a2, 10*4(sp)
            sw a3, 11*4(sp)
            sw a4, 12*4(sp)
            sw a5, 13*4(sp)
            sw a6, 14*4(sp)
            sw a7, 15*4(sp)

            jal ra, _start_trap_rust

            lw ra, 0*4(sp)
            lw t0, 1*4(sp)
            lw t1, 2*4(sp)
            lw t2, 3*4(sp)
            lw t3, 4*4(sp)
            lw t4, 5*4(sp)
            lw t5, 6*4(sp)
            lw t6, 7*4(sp)
            lw a0, 8*4(sp)
            lw a1, 9*4(sp)
            lw a2, 10*4(sp)
            lw a3, 11*4(sp)
            lw a4, 12*4(sp)
            lw a5, 13*4(sp)
            lw a6, 14*4(sp)
            lw a7, 15*4(sp)

            addi sp, sp, 16*4

            // set mstatus how we expect
            lui t4, %hi(0x00001800)
            addi t4, t4, %lo(0x00001800)
            csrw 0x300, t4

            mret


        _from_app:

            // Save the app registers to the StoredState array.
            // Kernel stack pointer was saved in mscratch, and stored state
            // pointer is on kernel stack.
            csrr t0, 0x340
            lw t1, 30*4(t0)

            // Need to save the excepting PC value. This will be stored in mepc.
            csrr t0, 0x341              // CSR=0x341=mepc
            sw   t0, 32*4(t1)           // Save the PC to the stored state struct

            sw   x1, 0*4(t1)
            sw   x2, 1*4(t1)
            sw   x3, 2*4(t1)
            sw   x4, 3*4(t1)
            sw   x5, 4*4(t1)
            sw   x6, 5*4(t1)
            sw   x7, 6*4(t1)
            sw   x8, 7*4(t1)
            sw   x9, 8*4(t1)
            sw   x10, 9*4(t1)
            sw   x11, 10*4(t1)
            sw   x12, 11*4(t1)
            sw   x13, 12*4(t1)
            sw   x14, 13*4(t1)
            sw   x15, 14*4(t1)
            sw   x16, 15*4(t1)
            sw   x17, 16*4(t1)
            sw   x18, 17*4(t1)
            sw   x19, 18*4(t1)
            sw   x20, 19*4(t1)
            sw   x21, 20*4(t1)
            sw   x22, 21*4(t1)
            sw   x23, 22*4(t1)
            sw   x24, 23*4(t1)
            sw   x25, 24*4(t1)
            sw   x26, 25*4(t1)
            sw   x27, 26*4(t1)
            sw   x28, 27*4(t1)
            sw   x29, 28*4(t1)
            sw   x30, 29*4(t1)
            sw   x31, 30*4(t1)

            // Restore kernel sp and registers.

            csrr sp, 0x340
            lw  x1,0*4(sp)
            lw  x3,1*4(sp)
            lw  x4,2*4(sp)
            lw  x5,3*4(sp)
            lw  x6,4*4(sp)
            lw  x7,5*4(sp)
            lw  x8,6*4(sp)
            lw  x9,7*4(sp)
            lw  x10,8*4(sp)
            lw  x11,9*4(sp)
            lw  x12,10*4(sp)
            lw  x13,11*4(sp)
            lw  x14,12*4(sp)
            lw  x15,13*4(sp)
            lw  x16,14*4(sp)
            lw  x17,15*4(sp)
            lw  x18,16*4(sp)
            lw  x19,17*4(sp)
            lw  x20,18*4(sp)
            lw  x21,19*4(sp)
            lw  x22,20*4(sp)
            lw  x23,21*4(sp)
            lw  x24,22*4(sp)
            lw  x25,23*4(sp)
            lw  x26,24*4(sp)
            lw  x27,25*4(sp)
            lw  x28,26*4(sp)
            lw  x29,27*4(sp)
            lw  x30,28*4(sp)
            lw  x31,29*4(sp)

            addi sp, sp, 31*4

            //get pc
            // lw  t0, 32*4(sp)
            // csrw 0x341, t0

            //save mcause in mscratch
            csrr t3, 0x342
            csrw 0x340, t3


            // Load the location in syscall.rs that we want to return to.
            lui t1, %hi(_return_to_kernel)
            addi t1, t1, %lo(_return_to_kernel)
            csrw 0x341, t1


            // set mstatus how we expect
            lui t4, %hi(0x00001808)
            addi t4, t4, %lo(0x00001808)
            csrw 0x300, t4

            mret
        "
        :
        :
        :
        : "volatile");
    }
}

/// Ensure an abort symbol exists.
#[link_section = ".init"]
#[export_name = "abort"]
pub extern "C" fn abort() {
    unsafe {
        asm! ("
            // Simply go back to the start as if we had just booted.
            j    _start
        "
        :
        :
        :
        : "volatile");
    }
}
