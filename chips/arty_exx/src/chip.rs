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
            while let Some(interrupt) = clic::next_pending() {
                match interrupt {

                    interrupts::GPIO3 => gpio::PORT[3].handle_interrupt(),
                    interrupts::GPIO4 => gpio::PORT[4].handle_interrupt(),
                    interrupts::GPIO5 => gpio::PORT[5].handle_interrupt(),
                    interrupts::GPIO6 => gpio::PORT[6].handle_interrupt(),
                    interrupts::GPIO7 => gpio::PORT[7].handle_interrupt(),
                    interrupts::GPIO8 => gpio::PORT[8].handle_interrupt(),
                    interrupts::GPIO9 => gpio::PORT[9].handle_interrupt(),
                    interrupts::GPIO10 => gpio::PORT[10].handle_interrupt(),
                    interrupts::GPIO11 => gpio::PORT[11].handle_interrupt(),
                    interrupts::GPIO12 => gpio::PORT[12].handle_interrupt(),
                    interrupts::GPIO13 => gpio::PORT[13].handle_interrupt(),
                    interrupts::GPIO14 => gpio::PORT[14].handle_interrupt(),
                    interrupts::GPIO15 => gpio::PORT[15].handle_interrupt(),

                    interrupts::UART0 => uart::UART0.handle_interrupt(),

                    _ => debug!("Pidx {}", interrupt),
                }

                // Mark that we are done with this interrupt and the hardware
                // can clear it.
                clic::complete(interrupt);
            }
        }
    }

    fn has_pending_interrupts(&self) -> bool {
        unsafe {
            clic::has_pending()
        }
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
