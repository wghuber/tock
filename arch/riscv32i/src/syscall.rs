//! Pseudo kernel-userland system call interface.
//!
//! This is for platforms that only include the "Machine Mode" privilege level.
//! Since these chips don't have hardware support for user mode, we have to fake
//! it. This means the apps have to be highly trusted as there is no real separation
//! between the kernel and apps.
//!
//! Note: this really only exists so we can demonstrate Tock running on actual
//! RISC-V hardware. Really, this is very undesirable for Tock as it violates
//! the safety properties of the OS. As hardware starts to exist that supports M
//! and U modes we will remove this.

use core::fmt::Write;
use core::ptr::{read_volatile, write_volatile};

use kernel;

#[allow(improper_ctypes)]
extern "C" {
    pub fn switch_to_user(user_stack: *const u8, process_regs: &mut [usize; 8]) -> *mut u8;
}

/// This holds all of the state that the kernel must keep for the process when
/// the process is not executing.
#[derive(Copy, Clone, Default)]
pub struct RiscvimacStoredState {
    regs: [usize; 32],
    pc: usize,
}

/// Implementation of the `UserspaceKernelBoundary` for the RISC-V architecture.
pub struct SysCall();

impl SysCall {
    pub const unsafe fn new() -> SysCall {
        SysCall()
    }
}

impl kernel::syscall::UserspaceKernelBoundary for SysCall {
    type StoredState = RiscvimacStoredState;

    /// Get the syscall that the process called.
    unsafe fn get_syscall(&self, _stack_pointer: *const usize) -> Option<kernel::syscall::Syscall> {
        None
    }

    unsafe fn set_syscall_return_value(&self, _stack_pointer: *const usize, _return_value: isize) {}

    unsafe fn pop_syscall_stack_frame(
        &self,
        stack_pointer: *const usize,
        _state: &mut RiscvimacStoredState,
        ) -> *mut usize {
        stack_pointer as *mut usize
    }

    unsafe fn push_function_call(
        &self,
        stack_pointer: *const usize,
        _remaining_stack_memory: usize,
        _callback: kernel::procs::FunctionCall,
        _state: &RiscvimacStoredState,
        ) -> Result<*mut usize, *mut usize> {

        Ok(stack_pointer as *mut usize)
    }

    unsafe fn switch_to_process(
        &self,
        stack_pointer: *const usize,
        _state: &mut RiscvimacStoredState,
        ) -> (*mut usize, kernel::syscall::ContextSwitchReason) {

        let mut mstatus: u32;
        mstatus = 0;


        asm! ("
          // save kernel registers, and sp in mscratch (0x340)

          addi sp, sp, -31*4  // Move the stack pointer down to make room.

          sw x1,0*4(sp)       // Save all of the registers on the kernel stack.
          sw x3,1*4(sp)
          sw x4,2*4(sp)
          sw x5,3*4(sp)
          sw x6,4*4(sp)
          sw x7,5*4(sp)
          sw x8,6*4(sp)
          sw x9,7*4(sp)
          sw x10,8*4(sp)
          sw x11,9*4(sp)
          sw x12,10*4(sp)
          sw x13,11*4(sp)
          sw x14,12*4(sp)
          sw x15,13*4(sp)
          sw x16,14*4(sp)
          sw x17,15*4(sp)
          sw x18,16*4(sp)
          sw x19,17*4(sp)
          sw x20,18*4(sp)
          sw x21,19*4(sp)
          sw x22,20*4(sp)
          sw x23,21*4(sp)
          sw x24,22*4(sp)
          sw x25,23*4(sp)
          sw x26,24*4(sp)
          sw x27,25*4(sp)
          sw x28,26*4(sp)
          sw x29,27*4(sp)
          sw x30,28*4(sp)
          sw x31,29*4(sp)

          sw $0, 30*4(sp)     // Store process state pointer on stack as well.

          csrw 0x340, sp      // Save stack pointer in mscratch. This allows
                              // us to find it when the app returns back to
                              // the kernel.
          "
          :
          :"r"(_state)
          :
          :"volatile");


        // Read current mstatus CSR and then modify it so we switch to
        // user mode when running the app.
        asm! ("
          csrr $0, 0x300  // Read mstatus CSR
          "
          : "=r" (mstatus)
          :
          :
              : "volatile");

        // (read_csr(mstatus) &~ MSTATUS_MPP &~ MSTATUS_MIE) | MSTATUS_MPIE
        mstatus = (mstatus  & !0x00001800 & !0x00000008) | 0x00000080;
        // mstatus = 0x00000080;

        asm! ("
          // Write mstatus, write app location to mepc, load stack pointer,
          // set parameters.
          csrw 0x300, $0     // Set mstatus CSR
          csrw 0x341, $1     // Set mepc CSR. This is the PC we want to go to.
          add x2, x0, $2     // Set sp register with app stack pointer.
          li a0, 0x00000005  // Arg0: `void* app_start`
          li a1, 0x00000006  // Arg1: `void* mem_start`
          li a2, 0x00000007  // Arg2: `void* memory_len`
          li a3, 0x00000008  // Arg3: `void* app_heap_break`
          mret
          "
          :
          : "r"(mstatus), "r"(0x40430060), "r"(stack_pointer)
          : "a0", "a1", "a2", "a3"
          : "volatile");



        asm!("
        _return_to_kernel:

          // mcause is stored in mscratch at this point since we have exited
          // the fault handler.
          csrr t0, 0x340              // CSR=0x340=mscratch
          // If mcause < 0 then we encountered an interrupt.
          blt  t0, x0, _app_interrupt // If negative, this was an interrupt.


          // Check the various exception codes and handle them properly.

          andi  t0, t0, 0x1ff         // `and` mcause with 9 lower bits of zero
                                      // to mask off just the cause. This is
                                      // needed because the E21 core uses
                                      // several of the upper bits for other
                                      // flags.

        _check_ecall_umode:
          li    t1, 8             // 8 is the index of ECALL from U mode.
          beq   t0, t1, _done     // Check if we did an ECALL and handle it correctly.



          // Fall through to error.
          j _go_red



          // An interrupt occurred while the app was running.
          // TODO
        _app_interrupt:
          nop


          // Stop here if we get here. This means there was some other exception that
          // we are not handling. The red LED will come on.
        _go_red:
          lui t5, 0x20002
          addi t5, t5, 0x00000008
          li t6, 0x00000007
          sw t6, 0(t5)
          lui t5, 0x20002
          addi t5, t5, 0x0000000c
          li t6, 0x1
          sw t6, 0(t5)
          j _go_red





        _done:
          nop



          "
          :
          :
          :
          : "volatile");


        debug!("yay!! wow does this actually print a lot or what??");

        (
            stack_pointer as *mut usize,
            kernel::syscall::ContextSwitchReason::Fault,
            )
    }

    unsafe fn fault_fmt(&self, writer: &mut Write) {}

    unsafe fn process_detail_fmt(
        &self,
        stack_pointer: *const usize,
        state: &RiscvimacStoredState,
        writer: &mut Write,
        ) {
    }
}
