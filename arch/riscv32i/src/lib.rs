#![crate_name = "riscv32i"]
#![crate_type = "rlib"]
#![feature(asm, const_fn, lang_items, global_asm)]
#![no_std]

#[macro_use(register_bitfields, register_bitmasks, debug_gpio, debug)]
extern crate kernel;


//pub mod plic;
pub mod support;
pub mod syscall;
pub mod clic;

extern "C" {
    // External function defined by the board main.rs.
    fn reset_handler();

    // Where the end of the stack region is (and hence where the stack should
    // start).
    static _estack: u32;

    // Address of _start_trap.
    static _start_trap: u32;

    // Boundaries of the .bss section.
    static mut _szero: u32;
    static mut _ezero: u32;

    // Where the .data section is stored in flash.
    static mut _etext: u32;

    // Boundaries of the .data section.
    static mut _srelocate: u32;
    static mut _erelocate: u32;
}

/// Entry point of all programs (_start).
///
/// It initializes DWARF call frame information, the stack pointer, the
/// frame pointer (needed for closures to work in start_rust) and the global
/// pointer. Then it calls _start_rust.
#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
global_asm!(
    r#"
.section .riscv.start, "ax"
.globl _start
_start:
  .cfi_startproc
  .cfi_undefined ra

  // Set the global pointer register using the variable defined in the linker
  // script. This register is only set once. The global pointer is a method
  // for sharing state between the linker and the CPU so that the linker can
  // emit code with offsets that are relative to the gp register, and the CPU
  // can successfully execute them.
  //
  // https://gnu-mcu-eclipse.github.io/arch/riscv/programmer/#the-gp-global-pointer-register
  // https://groups.google.com/a/groups.riscv.org/forum/#!msg/sw-dev/60IdaZj27dY/5MydPLnHAQAJ
  // https://www.sifive.com/blog/2017/08/28/all-aboard-part-3-linker-relaxation-in-riscv-toolchain/
  //
  lui gp, %hi(__global_pointer$)
  addi gp, gp, %lo(__global_pointer$)

  // Initialize the stack pointer register. This comes directly from the linker
  // script.
  lui sp, %hi(_estack)
  addi sp, sp, %lo(_estack)

  // Set s0 (the frame pointer) to the start of the stack.
  add s0, sp, zero


  // PMP PMP PMP
  // PMP PMP PMP
  // PMP PMP PMP
  // PMP PMP PMP
  // TODO: Add a real PMP driver!!
  // Take some time to disable the PMP.

  // Set the first region address to 0xFFFFFFFF. When using top-of-range mode
  // this will include the entire address space.
  lui t0, %hi(0xFFFFFFFF)
  addi t0, t0, %lo(0xFFFFFFFF)
  csrw 0x3b0, t0   // CSR=pmpaddr0

  // Set the first region to use top-of-range and allow everything.
  // This is equivalent to:
  // R=1, W=1, X=1, A=01, L=0
  li t0, 0x0F
  csrw 0x3a0, t0   // CSR=pmpcfg0



  // Initialize machine timer mtimecmp to disable the machine timer interrupt.
  li t0, -1                    // Set mtimecmp to 0xFFFFFFFF
  lui t1, %hi(0x02004000)      // Load the address of mtimecmp to t1
  addi t1, t1, %lo(0x02004000) // Load the address of mtimecmp to t1
  sw t0, 0(t1)                 // mtimecmp is 64 bits, set to all ones
  sw t0, 4(t1)                 // mtimecmp is 64 bits, set to all ones







  // With that initial setup out of the way, we now branch to the main code,
  // likely defined in a board's main.rs.
  jal zero, reset_handler

  .cfi_endproc
"#
);

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

/// Tell the MCU what address the trap handler is located at.
///
/// The trap handler is called on exceptions and for interrupts.
pub unsafe fn configure_trap_handler() {
    asm!("
    // The csrw instruction writes a Control and Status Register (CSR)
    // with a new value.
    //
    // CSR 0x305 (mtvec, 'Machine trap-handler base address.') sets the address
    // of the trap handler. We do not care about its old value, so we don't
    // bother reading it. We want to enable direct CLIC mode so we set the
    // second lowest bit.
    ori  $0, $0, 0x02  // Set CLIC direct mode
    csrw 0x305, $0     // Write the mtvec CSR.
    "
       :
       : "r"(&_start_trap)
       :
       : "volatile");
}


/// Enable all PLIC interrupts so that individual peripheral drivers do not have
/// to manage these.
pub unsafe fn enable_clic_interrupts() {

    clic::disable_all();
    clic::clear_all_pending();
    clic::enable_all();

    // let m: u32;
    // let METAL_MIE_INTERRUPT: u32 = 0x00000008;

    // asm! ("csrrs %0, mstatus, %1" : "=r"(m) : "r"(METAL_MIE_INTERRUPT));





    // // enable mie 1
    // asm! ("
    //   // CSR 0x304 mie
    //   csrw 0x304, $0
    //   "
    //   :
    //   : "r"(0x00000001)
    //   :
    //   : "volatile");

    // enable machine mode interrupts
    asm! ("
      lui t0, %hi(0x0001808)       // Load the value we want mstatus to be.
      addi t0, t0, %lo(0x0001808)  // This should keep the core in M-Mode and
                                   // enable machine mode interrupts.
      csrw 0x300, t0               // Save to the mstatus CSR.
      "
      :
      :
      :
      : "volatile");
}



/// Trap entry point (_start_trap)
///
/// Saves caller saved registers ra, t0..6, a0..7, calls _start_trap_rust,
/// restores caller saved registers and then returns.
#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
global_asm!(
    r#"
  .section .riscv.trap, "ax"
  .align 6
  //.p2align 6
  .global _start_trap

_start_trap:


  //     //turn on purple LED
  // lui t5, 0x20002
  // addi t5, t5, 0x00000008
  // li t6, 0x00000007
  // sw t6, 0(t5)
  // lui t5, 0x20002
  // addi t5, t5, 0x0000000c
  // li t6, 0x5
  // sw t6, 0(t5)


  // // some mcause check code
  // csrr t0, 0x342
  // li t1, 0x00000008
  // beq  t1, t0, _from_app


  // First check which privilege level we came from. If we came from user mode
  // then we need to handle that differently from if we came from kernel mode.
  // Luckily in the E21, the MPP bits are included in the mcause register.
  csrr t0, 0x342              // CSR=0x342=mcause
  srli t1, t0, 28             // Shift the mcause 28 bits to the right (MPP bits)
  andi t1, t1, 0x3            // `and` to get only the bottom two MPP bits
  beq  t1, x0, _from_app      // If MPP=00 then we came from user mode



  // If we came from mcause.MPP=11 then we came from the kernel.



  // lui t1, %hi(0xfFFFFFFF)
  // addi t1, t1, %lo(0xfFFFFFFF)
  // // li t1, 0x000000ff
  // bgt t0, t1, _from_app
  // // j _from_app

  // Check if it came from the kernel (0x00001800 is 11 for machine mode)
  // csrr t0, 0x300
  // lui t1, %hi(0x00001800)
  // addi t1, t1, %lo(0x00001800)
  // or  t2, t0, t1
  // beq  t0, t2, _from_kernel


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
"#
);


// /// Trap entry point (_start_trap)
// ///
// /// Saves caller saved registers ra, t0..6, a0..7, calls _start_trap_rust,
// /// restores caller saved registers and then returns.
// #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
// global_asm!(
//     r#"
//   .section .riscv.trap, "ax"
//   .p2align 6
//   .global _start_trap

// _start_trap:
//   mret
// "#
// );

/// Trap entry point rust (_start_trap_rust)
///
/// mcause is read to determine the cause of the trap. XLEN-1 bit indicates
/// if it's an interrupt or an exception. The result is converted to an element
/// of the Interrupt or Exception enum and passed to handle_interrupt or
/// handle_exception.
// #[link_section = ".trap.rust"]
#[export_name = "_start_trap_rust"]
pub extern "C" fn start_trap_rust() {
  unsafe {clic::disable_mtip();}
  unsafe {clic::disable_pending();}
    // while(true){};
    // // dispatch trap to handler
    // trap_handler(mcause::read().cause());
    // // mstatus, remain in M-mode after mret
    // unsafe {
    //     mstatus::set_mpp(mstatus::MPP::Machine);
    // }

    unsafe{
    asm! ("
      // CSR 0x300 mstatus
      csrw 0x300, $0
      "
      :
      : "r"(0x00001808)
      :
      : "volatile");
  }
}

// Make sure there is an abort when linking.
//
// I don't know why we need this, or why cortex-m doesn't seem to have it.
#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
global_asm!(
    r#"
.section .init
.globl abort
abort:
  jal zero, _start
"#
);