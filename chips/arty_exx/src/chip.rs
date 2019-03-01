use riscv32i;
use riscv32i::clic;
use kernel;
use gpio;
use interrupts;
use uart;



pub struct ArtyExx {
    userspace_kernel_boundary: riscv32i::syscall::SysCall,
}

impl ArtyExx {
    pub unsafe fn new() -> ArtyExx {
        ArtyExx {
            userspace_kernel_boundary: riscv32i::syscall::SysCall::new(),
        }
    }
}

impl kernel::Chip for ArtyExx {
    type MPU = ();
    type UserspaceKernelBoundary = riscv32i::syscall::SysCall;
    type SysTick = ();

    fn mpu(&self) -> &Self::MPU {
        &()
    }

    fn systick(&self) -> &Self::SysTick {
        &()
    }

    fn userspace_kernel_boundary(&self) -> &riscv32i::syscall::SysCall {
        &self.userspace_kernel_boundary
    }

    fn service_pending_interrupts(&self) {
        unsafe {
            //debug_gpio!(0, set);
            while let Some(interrupt) = clic::next_pending() {
                uart::UART0.handle_interrupt();
                // match interrupt {

                //     // interrupts::UART0 => uart::UART0.handle_interrupt(),
                //     // index @ interrupts::GPIO0..interrupts::GPIO31 => gpio::PORT[index as usize].handle_interrupt(),
                //     // _ => debug!("Pidx {}", interrupt),
                // }

                // Mark that we are done with this interrupt and the hardware
                // can clear it.
                clic::complete(interrupt);
            }
        }
    }

    fn has_pending_interrupts(&self) -> bool {
        unsafe { clic::has_pending() }
        
        //false
    }

    fn sleep(&self) {
        // unsafe {
        // riscv32i::support::wfi();
        riscv32i::support::nop();
        // }
    }

    unsafe fn atomic<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        riscv32i::support::atomic(f)
    }
}
