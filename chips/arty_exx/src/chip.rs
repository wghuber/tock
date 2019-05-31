use riscv32i;
use riscv32i::clic;
use riscv32i::machine_timer;
use kernel;
use gpio;
use interrupts;
use uart;



pub struct ArtyExx {
    userspace_kernel_boundary: riscv32i::syscall::SysCall,
    clic: riscv32i::clic::Clic,
}

impl ArtyExx {
    pub unsafe fn new() -> ArtyExx {

        // Make a bit-vector of all interrupt locations that we actually intend
        // to use on this chip.
        // 0001 1111 1111 1111 1111 0000 0000 1000 0000
        let in_use_interrupts: u64 = 0x1FFFF0080;

        ArtyExx {
            userspace_kernel_boundary: riscv32i::syscall::SysCall::new(),
            clic: riscv32i::clic::Clic::new(in_use_interrupts),
        }
    }

    pub fn enable_all_interrupts(&self) {
        self.clic.enable_all();
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
            while let Some(interrupt) = self.clic.next_pending() {
                match interrupt {
                    interrupts::MTIP => machine_timer::MACHINETIMER.handle_interrupt(),

                    interrupts::GPIO0 => gpio::PORT[3].handle_interrupt(),
                    interrupts::GPIO1 => gpio::PORT[3].handle_interrupt(),
                    interrupts::GPIO2 => gpio::PORT[3].handle_interrupt(),
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
                self.clic.complete(interrupt);
            }
        }
    }

    fn has_pending_interrupts(&self) -> bool {
        self.clic.has_pending()
    }

    fn sleep(&self) {
        unsafe {
            riscv32i::support::wfi();
        }
    }

    unsafe fn atomic<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        riscv32i::support::atomic(f)
    }
}
