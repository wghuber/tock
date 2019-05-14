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
pub struct RiscvimacStoredState {}

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
        let mut a0: u32;
        mstatus = 0;

        unsafe{
            asm! ("
              // CSR 0x300 mstatus
              csrr t3, 0x300
              mv $0, t3
              "
              : "=r" (mstatus)
              : 
              :
              : "volatile");
        }
        //debug!("{}", mstatus);

        mstatus = (mstatus  &! 0x00000100 &! 0x00000002) | 0x00000020;

        

        unsafe{
            asm! ("
              csrw 0x300, $0
              csrw 0x341, $1
              add x2, x0, $2
              li a0, 0x00000005
              li a1, 0x00000006
              li a2, 0x00000007
              li a3, 0x00000008
              //mret
              "
              : 
              : "r"(mstatus), "r"(0x40430060), "r"(stack_pointer)
              : "a0", "a1", "a2", "a3"
              : "volatile");
        }    

        unsafe{
            asm! ("
              mv $0, a2
              "
              : "=r" (a0)
              : 
              :
              : "volatile");
        }

        debug!("{}", a0);

        unsafe{
            asm! ("mret");
        }

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
