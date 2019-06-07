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
#[repr(C)]
pub struct RiscvimacStoredState {
    /// Store all of the callee saved registers, plus ra for the process.
    ///
    /// ```
    /// index | register
    /// ----------------
    ///  0    | ra
    ///  1    | sp
    ///  2    | a0
    ///  3    | a1
    ///  4    | a2
    ///  5    | a3
    ///  6    | a4
    ///  7    | s0
    ///  8    | s1
    ///  9    | s2
    /// 10    | s3
    /// 11    | s4
    /// 12    | s5
    /// 13    | s6
    /// 14    | s7
    /// 15    | s8
    /// 16    | s9
    /// 17    | s10
    /// 18    | s11
    /// ```
    regs: [usize; 19],
    pc: usize,
    mcause: usize,
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

    unsafe fn set_syscall_return_value(
        &self,
        _stack_pointer: *const usize,
        state: &mut Self::StoredState,
        return_value: isize
    ) {
        // Just need to put the return value in the a0 register for when the
        // process resumes executing.
        state.regs[2] = return_value as usize; // a0 = regs[2] = return value


        debug!("r:{:#x} to:{:#x} ra:{:#x}", state.regs[2], state.pc, state.regs[0]);
    }

    unsafe fn set_process_function(
        &self,
        stack_pointer: *const usize,
        _remaining_stack_memory: usize,
        state: &mut RiscvimacStoredState,
        callback: kernel::procs::FunctionCall,
        first_function: bool,
        ) -> Result<*mut usize, *mut usize> {

        // Set the register state for the application when it starts
        // executing. These are the argument registers.
        state.regs[2] = callback.argument0; // a0 = x10 = regs[2]
        state.regs[3] = callback.argument1; // a1 = x11 = regs[3]
        state.regs[4] = callback.argument2; // a2 = x12 = regs[4]
        state.regs[5] = callback.argument3; // a3 = x13 = regs[5]

        // We also need to set the return address (ra) register so that the
        // new function that the process is running returns to the correct
        // location. However, if this is the first time the process is running
        // then there is nothing to return to so we skip this.
        if !first_function {
            state.regs[0] = state.pc;        // ra = x1 = regs[0]
        } else {
            // If this is the first execution we need to set the initial stack
            // pointer in the sp register.
            state.regs[1] = stack_pointer as usize;
        }

        // Save the PC we expect to execute.
        state.pc = callback.pc;

        debug!("going to {:#x}, ra: {:#x}, sp: {:#x}", callback.pc, state.regs[0], state.regs[1]);

        Ok(stack_pointer as *mut usize)
    }

    unsafe fn switch_to_process(
        &self,
        _stack_pointer: *const usize,
        state: &mut RiscvimacStoredState,
        ) -> (*mut usize, kernel::syscall::ContextSwitchReason) {
        let mut switch_reason: u32;
        let mut syscall_args: [u32; 5] = [0; 5];
        let mut new_stack_pointer: u32;

        asm! ("
          // Before switching to the app we need to save the kernel registers to
          // the kernel stack. We then save the stack pointer in the mscratch
          // CSR (0x340) so we can retrieve it after returning to the kernel
          // from the app.

          addi sp, sp, -32*4  // Move the stack pointer down to make room.

          sw   x1, 0*4(sp)    // Save all of the registers on the kernel stack.
          sw   x3, 1*4(sp)
          sw   x4, 2*4(sp)
          sw   x5, 3*4(sp)
          sw   x6, 4*4(sp)
          sw   x7, 5*4(sp)
          sw   x8, 6*4(sp)
          sw   x9, 7*4(sp)
          sw   x10, 8*4(sp)
          sw   x11, 9*4(sp)
          sw   x12, 10*4(sp)
          sw   x13, 11*4(sp)
          sw   x14, 12*4(sp)
          sw   x15, 13*4(sp)
          sw   x16, 14*4(sp)
          sw   x17, 15*4(sp)
          sw   x18, 16*4(sp)
          sw   x19, 17*4(sp)
          sw   x20, 18*4(sp)
          sw   x21, 19*4(sp)
          sw   x22, 20*4(sp)
          sw   x23, 21*4(sp)
          sw   x24, 22*4(sp)
          sw   x25, 23*4(sp)
          sw   x26, 24*4(sp)
          sw   x27, 25*4(sp)
          sw   x28, 26*4(sp)
          sw   x29, 27*4(sp)
          sw   x30, 28*4(sp)
          sw   x31, 29*4(sp)

          sw   $2, 30*4(sp)   // Store process state pointer on stack as well.
                              // We need to have the available for after the app
                              // returns to the kernel so we can store its
                              // registers.

          // Store the address to jump back to on the stack so that the trap
          // handler knows where to return to after the app stops executing.
          lui  t0, %hi(_return_to_kernel)
          addi t0, t0, %lo(_return_to_kernel)
          sw   t0, 31*4(sp)

          csrw 0x340, sp      // Save stack pointer in mscratch. This allows
                              // us to find it when the app returns back to
                              // the kernel.

          // Read current mstatus CSR and then modify it so we switch to
          // user mode when running the app.
          csrr t0, 0x300      // Read mstatus=0x300 CSR
          // Set the mode to user mode and set MPIE.
          li   t1, 0x1808     // t1 = MSTATUS_MPP & MSTATUS_MIE
          not  t1, t1         // t1 = ~(MSTATUS_MPP & MSTATUS_MIE)
          and  t0, t0, t1     // t0 = mstatus & ~(MSTATUS_MPP & MSTATUS_MIE)
          ori  t0, t0, 0x80   // t0 = t0 | MSTATUS_MPIE
          csrw 0x300, t0      // Set mstatus CSR so that we switch to user mode.

          // We have to set the mepc CSR with the PC we want the app to start
          // executing at. This has been saved in RiscvimacStoredState for us
          // (either when the app returned back to the kernel or in the
          // `set_process_function()` function).
          lw   t0, 19*4($2)   // Retrieve the PC from RiscvimacStoredState
          csrw 0x341, t0      // Set mepc CSR. This is the PC we want to go to.

          // Restore all of the app registers from what we saved. If this is the
          // first time running the app then most of these values are
          // irrelevant, However we do need to set the four arguments to the
          // `_start_ function in the app. If the app has been executing then this
          // allows the app to correctly resume.
          lw   ra,   0*4($2)
          lw   sp,   1*4($2)
          lw   a0,   2*4($2)
          lw   a1,   3*4($2)
          lw   a2,   4*4($2)
          lw   a3,   5*4($2)
          lw   a4,   6*4($2)
          lw   s0,   7*4($2)
          lw   s1,   8*4($2)
          lw   s2,   9*4($2)
          lw   s3,  10*4($2)
          lw   s4,  11*4($2)
          lw   s5,  12*4($2)
          lw   s6,  13*4($2)
          lw   s7,  14*4($2)
          lw   s8,  15*4($2)
          lw   s9,  16*4($2)
          lw   s10, 17*4($2)
          lw   s11, 18*4($2)

          // Call mret to jump to where mepc points, switch to user mode, and
          // start running the app.
          mret




          // This is where the trap handler jumps back to after the app stops
          // executing.
        _return_to_kernel:


          // First thing we have to do is store the app registers. The kernel
          // stack pointer was saved in mscratch which has been loaded into sp
          // in the trap handler. We use that to find the pointer to the stored
          // state struct.
          lw   t0, 30*4(sp) // Load the stored state pointer into t0.

          // Store all of the callee saved registers to the stored state
          // struct. We also save a0-a4 since we use those as syscall
          // arguments.
          sw   ra,   0*4(t0) // Save ra
          sw   a0,   2*4(t0) // Save a0-a4
          sw   a1,   3*4(t0)
          sw   a2,   4*4(t0)
          sw   a3,   5*4(t0)
          sw   a4,   6*4(t0)
          sw   s0,   7*4(t0) // Save s0-s11
          sw   s1,   8*4(t0)
          sw   s2,   9*4(t0)
          sw   s3,  10*4(t0)
          sw   s4,  11*4(t0)
          sw   s5,  12*4(t0)
          sw   s6,  13*4(t0)
          sw   s7,  14*4(t0)
          sw   s8,  15*4(t0)
          sw   s9,  16*4(t0)
          sw   s10, 17*4(t0)
          sw   s11, 18*4(t0)


          // Now we can restore the kernel registers before resuming kernel
          // code.
          lw   x1,  0*4(sp)
          lw   x3,  1*4(sp)
          lw   x4,  2*4(sp)
          lw   x5,  3*4(sp)
          lw   x6,  4*4(sp)
          lw   x7,  5*4(sp)
          lw   x8,  6*4(sp)
          lw   x9,  7*4(sp)
          lw   x10, 8*4(sp)
          lw   x11, 9*4(sp)
          lw   x12, 10*4(sp)
          lw   x13, 11*4(sp)
          lw   x14, 12*4(sp)
          lw   x15, 13*4(sp)
          lw   x16, 14*4(sp)
          lw   x17, 15*4(sp)
          lw   x18, 16*4(sp)
          lw   x19, 17*4(sp)
          lw   x20, 18*4(sp)
          lw   x21, 19*4(sp)
          lw   x22, 20*4(sp)
          lw   x23, 21*4(sp)
          lw   x24, 22*4(sp)
          lw   x25, 23*4(sp)
          lw   x26, 24*4(sp)
          lw   x27, 25*4(sp)
          lw   x28, 26*4(sp)
          lw   x29, 27*4(sp)
          lw   x30, 28*4(sp)
          lw   x31, 29*4(sp)

          addi sp, sp, 32*4




          // Load mcause from the stored value in the RiscvimacStoredState
          // struct.
          lw   t0, 20*4($2)
          // If mcause < 0 then we encountered an interrupt.
          blt  t0, x0, _app_interrupt // If negative, this was an interrupt.


          // Check the various exception codes and handle them properly.

          andi  t0, t0, 0x1ff // `and` mcause with 9 lower bits of zero
                              // to mask off just the cause. This is needed
                              // because the E21 core uses several of the upper
                              // bits for other flags.

        _check_ecall_umode:
          li   t1, 8          // 8 is the index of ECALL from U mode.
          beq  t0, t1, _ecall // Check if we did an ECALL and handle it
                              // correctly.

        _check_exception:
          li   $0, 2          // If we get here, the only other option is an
          j    _done          // exception happened. We don't differentiate.

        _app_interrupt:
          li   $0, 1          // Mark that an interrupt occurred while the app
                              // was running.
          j    _done


        _ecall:
          li   $0, 0          // Mark that the process did a syscall.
          // Need to increment the PC so when we return we start at the correct
          // instruction. The hardware does not do this for us.
          lw   t0, 19*4($2)   // Get the PC from RiscvimacStoredState
          addi t0, t0, 4      // Add 4 to increment the PC past ecall instruction
          sw   t0, 19*4($2)   // Save the new PC back to RiscvimacStoredState

          // We have to get the values that the app passed to us in registers
          // (these are stored in RiscvimacStoredState) and copy them to
          // registers so we can use them when returning to the kernel loop.
          lw   t0, 2*4($2)    // Fetch a0
          sw   t0, 0*4($3)
          lw   t0, 3*4($2)    // Fetch a1
          sw   t0, 1*4($3)
          lw   t0, 4*4($2)    // Fetch a2
          sw   t0, 2*4($3)
          lw   t0, 5*4($2)    // Fetch a3
          sw   t0, 3*4($3)
          lw   t0, 6*4($2)    // Fetch a4
          sw   t0, 4*4($3)
          lw   $1, 1*4($2)    // Fetch sp

        _done:
          nop
          "
          : "=r"(switch_reason), "=r" (new_stack_pointer)
          : "r"(state), "r"(&mut syscall_args)
          : "a0", "a1", "a2", "a3"
          : "volatile");


        debug!("syscall: {:#x} {:#x} {:#x} {:#x} {:#x} {:#x} {:#x}",
            syscall_args[0],
            syscall_args[1],
            syscall_args[2],
            syscall_args[3],
            syscall_args[4],
            new_stack_pointer, switch_reason);


        // debug!("syscall: {:#x} {:#x} {:#x} {:#x} {:#x} {:#x} {:#x}",
        //     aaaa[0], aaaa[1], aaaa[2], aaaa[3], aaaa[4], newsp, switch_reason);

        // debug!("syscall: {:#x} {:#x} {:#x} {:#x} {:#x} {:#x} {:#x}",
        //     syscall0, 1,1,1,1, newsp, switch_reason);




        // Prepare the return type that marks why the app stopped executing.
        let ret = match switch_reason {
            0 => {
                let syscall = kernel::syscall::arguments_to_syscall(
                    syscall_args[0] as u8,
                    syscall_args[1] as usize,
                    syscall_args[2] as usize,
                    syscall_args[3] as usize,
                    syscall_args[4] as usize);
                match syscall {
                    Some(s) => kernel::syscall::ContextSwitchReason::SyscallFired{
                        syscall: s
                    },
                    None => kernel::syscall::ContextSwitchReason::Fault
                }

            }
            1 => kernel::syscall::ContextSwitchReason::Interrupted,
            2 => kernel::syscall::ContextSwitchReason::Fault,
            _ => kernel::syscall::ContextSwitchReason::Fault,
        };








        // let mut ret: kernel::syscall::ContextSwitchReason;
        // if (switchReason == 1){
        //     //debug_gpio!(1, set);
        //     ret = kernel::syscall::ContextSwitchReason::Interrupted;
        //     switchReason = 0;
        // }
        // else if (switchReason == 2){
        //     ret = kernel::syscall::ContextSwitchReason::Fault;
        //     switchReason = 0;
        // }
        // // // else if(syscall.is_some()){
        // //     ret = kernel::syscall::ContextSwitchReason::SyscallFired{syscall: syscall};
        // // }
        // // else{
        // //     ret = kernel::syscall::ContextSwitchReason::Fault;
        // // }
        // else{
        //     ret = match syscall {
        //     Some(s) => kernel::syscall::ContextSwitchReason::SyscallFired{
        //         syscall: s
        //     },
        //     None => kernel::syscall::ContextSwitchReason::Fault
        // };

        // }


        (new_stack_pointer as *mut usize, ret)
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
