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

        unsafe{

            asm! ("
              // save kernel registers, and sp in mscratch (0x340)
              sw x1,1*4(x2)
              sw x3,3*4(x2)
              sw x4,4*4(x2)
              sw x5,5*4(x2)
              sw x6,6*4(x2)
              sw x7,7*4(x2)
              sw x8,8*4(x2)
              sw x9,9*4(x2)
              sw x10,10*4(x2)
              sw x11,11*4(x2)
              sw x12,12*4(x2)
              sw x13,13*4(x2)
              sw x14,14*4(x2)
              sw x15,15*4(x2)
              sw x16,16*4(x2)
              sw x17,17*4(x2)
              sw x18,18*4(x2)
              sw x19,19*4(x2)
              sw x20,20*4(x2)
              sw x21,21*4(x2)
              sw x22,22*4(x2)
              sw x23,23*4(x2)
              sw x24,24*4(x2)
              sw x25,25*4(x2)
              sw x26,26*4(x2)
              sw x27,27*4(x2)
              sw x28,28*4(x2)
              sw x29,29*4(x2)
              sw x30,30*4(x2)
              sw x31,31*4(x2)

              //store process state pointer on stack
              add t0, x0, $0
              sw t0, 32*4(x2)
              csrw 0x340, x2
              "
              :
              :"r"(_state)
              :
              :"volatile");
        }

        unsafe{
            asm! ("
              // Read mstatus (0x300) into mstatus var
              csrr t3, 0x300
              mv $0, t3
              "
              : "=r" (mstatus)
              : 
              :
              : "volatile");
        }

        // (read_csr(mstatus) &~ MSTATUS_MPP &~ MSTATUS_MIE) | MSTATUS_MPIE
        mstatus = (mstatus  &! 0x00000100 &! 0x00000002) | 0x00000020;

        unsafe{
            asm! ("
              // Write mstatus, write app location to mepc, load stack pointer, set parameters  
              lui t1, %hi(0x40430060)
              addi t1, t1, %lo(0x40430060)
              csrw 0x300, $0
              csrw 0x341, t1
              add x2, x0, $2
              li a0, 0x00000005
              li a1, 0x00000006
              li a2, 0x00000007
              li a3, 0x00000008
              mret
              "
              : 
              : "r"(mstatus), "r"(0x40430060), "r"(stack_pointer)
              : "a0", "a1", "a2", "a3"
              : "volatile");

        }

        unsafe{
            asm!("
             _return_to_kernel:
                nop
                nop
                ");
        }   
        //debug_gpio!(0, set);
          
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
