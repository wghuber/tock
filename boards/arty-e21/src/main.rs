//! Board file for the SiFive E21 Bitstream running on the Arty FPGA

#![no_std]
#![no_main]
#![feature(asm)]
#![feature(const_fn, in_band_lifetimes)]

extern crate capsules;
#[allow(unused_imports)]
#[macro_use(create_capability, debug, debug_gpio, static_init)]
extern crate kernel;
extern crate arty_exx;
extern crate riscv32i;
extern crate sifive;

use capsules::virtual_alarm::{MuxAlarm, VirtualMuxAlarm};
// use capsules::virtual_i2c::{I2CDevice, MuxI2C};
use capsules::virtual_uart::{UartDevice, UartMux};
use kernel::capabilities;
use kernel::hil;
use kernel::Platform;

mod timer_test;

pub mod io;

// State for loading and holding applications.

// Number of concurrent processes this platform supports.
const NUM_PROCS: usize = 4;

// How should the kernel respond when a process faults.
const FAULT_RESPONSE: kernel::procs::FaultResponse = kernel::procs::FaultResponse::Panic;

// RAM to be shared by all application processes.
#[link_section = ".app_memory"]
static mut APP_MEMORY: [u8; 8192] = [0; 8192];

// Actual memory for holding the active process structures.
static mut PROCESSES: [Option<&'static kernel::procs::ProcessType>; NUM_PROCS] =
    [None, None, None, None];

/// Dummy buffer that causes the linker to reserve enough space for the stack.
#[no_mangle]
#[link_section = ".stack_buffer"]
pub static mut STACK_MEMORY: [u8; 0x1000] = [0; 0x1000];

/// A structure representing this platform that holds references to all
/// capsules for this platform.
struct ArtyE21 {
    console: &'static capsules::console::Console<'static, UartDevice<'static>>,
    gpio: &'static capsules::gpio::GPIO<'static, sifive::gpio::GpioPin>,
    alarm: &'static capsules::alarm::AlarmDriver<
        'static,
        VirtualMuxAlarm<'static, riscv32i::machine_timer::MachineTimer>,
    >,
    led: &'static capsules::led::LED<'static, sifive::gpio::GpioPin>,
    // button: &'static capsules::button::Button<'static, sam4l::gpio::GPIOPin>,
    // ipc: kernel::ipc::IPC,
}

/// Mapping of integer syscalls to objects that implement syscalls.
impl Platform for ArtyE21 {
    fn with_driver<F, R>(&self, driver_num: usize, f: F) -> R
    where
        F: FnOnce(Option<&kernel::Driver>) -> R,
    {
        match driver_num {
            capsules::console::DRIVER_NUM => f(Some(self.console)),
            capsules::gpio::DRIVER_NUM => f(Some(self.gpio)),

            capsules::alarm::DRIVER_NUM => f(Some(self.alarm)),
            capsules::led::DRIVER_NUM => f(Some(self.led)),
            // capsules::button::DRIVER_NUM => f(Some(self.button)),

            // kernel::ipc::DRIVER_NUM => f(Some(&self.ipc)),
            _ => f(None),
        }
    }
}

/// Reset Handler.
///
/// This function is called from the arch crate after some very basic RISC-V
/// setup.
#[no_mangle]
pub unsafe fn reset_handler() {
    // Basic setup of the platform.
    riscv32i::init_memory();
    riscv32i::configure_trap_handler();


    let process_mgmt_cap = create_capability!(capabilities::ProcessManagementCapability);
    let main_loop_cap = create_capability!(capabilities::MainLoopCapability);
    let memory_allocation_cap = create_capability!(capabilities::MemoryAllocationCapability);

    let board_kernel = static_init!(kernel::Kernel, kernel::Kernel::new(&PROCESSES));

    // Configure kernel debug gpios as early as possible
    kernel::debug::assign_gpios(
        Some(&arty_exx::gpio::PORT[0]), // Red
        Some(&arty_exx::gpio::PORT[1]),
        Some(&arty_exx::gpio::PORT[8]),
    );

    let chip = static_init!(arty_exx::chip::ArtyExx, arty_exx::chip::ArtyExx::new());

    // Create a shared UART channel for the console and for kernel debug.
    let uart_mux = static_init!(
        UartMux<'static>,
        UartMux::new(
            &arty_exx::uart::UART0,
            &mut capsules::virtual_uart::RX_BUF,
            115200
        )
    );
    hil::uart::UART::set_client(&arty_exx::uart::UART0, uart_mux);
    uart_mux.initialize();

    // Create a UartDevice for the console.
    let console_uart = static_init!(UartDevice, UartDevice::new(uart_mux, true));
    console_uart.setup();
    let console = static_init!(
        capsules::console::Console<UartDevice>,
        capsules::console::Console::new(
            console_uart,
            115200,
            &mut capsules::console::WRITE_BUF,
            &mut capsules::console::READ_BUF,
            board_kernel.create_grant(&memory_allocation_cap)
        )
    );
    hil::uart::UART::set_client(console_uart, console);

    // let ast = &sam4l::ast::AST;

    let mux_alarm = static_init!(
        MuxAlarm<'static, riscv32i::machine_timer::MachineTimer>,
        MuxAlarm::new(&riscv32i::machine_timer::MACHINETIMER)
    );
    riscv32i::machine_timer::MACHINETIMER.set_client(mux_alarm);

    // Alarm
    let virtual_alarm_user = static_init!(
        VirtualMuxAlarm<'static, riscv32i::machine_timer::MachineTimer>,
        VirtualMuxAlarm::new(mux_alarm)
    );
    let alarm = static_init!(
        capsules::alarm::AlarmDriver<'static, VirtualMuxAlarm<'static, riscv32i::machine_timer::MachineTimer>>,
        capsules::alarm::AlarmDriver::new(
            virtual_alarm_user,
            board_kernel.create_grant(&memory_allocation_cap)
        )
    );
    virtual_alarm_user.set_client(alarm);







    // let virtual_alarm_test = static_init!(
    //     VirtualMuxAlarm<'static, riscv32i::machine_timer::MachineTimer>,
    //     VirtualMuxAlarm::new(mux_alarm)
    // );
    // let timertest = static_init!(
    //     timer_test::TimerTest<'static, VirtualMuxAlarm<'static, riscv32i::machine_timer::MachineTimer>>,
    //     timer_test::TimerTest::new(virtual_alarm_test)
    // );
    // virtual_alarm_test.set_client(timertest);











    // // Initialize and enable SPI HAL
    // // Set up an SPI MUX, so there can be multiple clients
    // let mux_spi = static_init!(
    //     MuxSpiMaster<'static, sam4l::spi::SpiHw>,
    //     MuxSpiMaster::new(&sam4l::spi::SPI)
    // );

    // sam4l::spi::SPI.set_client(mux_spi);
    // sam4l::spi::SPI.init();

    // LEDs
    let led_pins = static_init!(
        [(
            &'static sifive::gpio::GpioPin,
            capsules::led::ActivationMode
        ); 3],
        [
            (
                // Red
                &arty_exx::gpio::PORT[0],
                capsules::led::ActivationMode::ActiveLow
            ),
            (
                // Green
                &arty_exx::gpio::PORT[1],
                capsules::led::ActivationMode::ActiveLow
            ),
            (
                // Blue
                &arty_exx::gpio::PORT[2],
                capsules::led::ActivationMode::ActiveLow
            ),
        ]
    );
    let led = static_init!(
        capsules::led::LED<'static, sifive::gpio::GpioPin>,
        capsules::led::LED::new(led_pins)
    );

    // // BUTTONs
    // let button_pins = static_init!(
    //     [(&'static sam4l::gpio::GPIOPin, capsules::button::GpioMode); 1],
    //     [(
    //         &sam4l::gpio::PA[16],
    //         capsules::button::GpioMode::LowWhenPressed
    //     )]
    // );
    // let button = static_init!(
    //     capsules::button::Button<'static, sam4l::gpio::GPIOPin>,
    //     capsules::button::Button::new(button_pins, board_kernel.create_grant())
    // );
    // for &(btn, _) in button_pins.iter() {
    //     btn.set_client(button);
    // }

    // set GPIO driver controlling remaining GPIO pins
    let gpio_pins = static_init!(
        [&'static sifive::gpio::GpioPin; 3],
        [
            &arty_exx::gpio::PORT[7],
            &arty_exx::gpio::PORT[5],
            &arty_exx::gpio::PORT[6],
        ]
    );
    let gpio = static_init!(
        capsules::gpio::GPIO<'static, sifive::gpio::GpioPin>,
        capsules::gpio::GPIO::new(gpio_pins)
    );
    for pin in gpio_pins.iter() {
        pin.set_client(gpio);
    }

    hil::gpio::Pin::make_output(&arty_exx::gpio::PORT[0]);
    hil::gpio::Pin::clear(&arty_exx::gpio::PORT[0]);

    hil::gpio::Pin::make_output(&arty_exx::gpio::PORT[1]);
    hil::gpio::Pin::clear(&arty_exx::gpio::PORT[1]);

    hil::gpio::Pin::make_output(&arty_exx::gpio::PORT[2]);
    hil::gpio::Pin::clear(&arty_exx::gpio::PORT[2]);

    hil::gpio::Pin::make_output(&arty_exx::gpio::PORT[8]);
    hil::gpio::Pin::clear(&arty_exx::gpio::PORT[8]);

    hil::gpio::Pin::make_input(&arty_exx::gpio::PORT[4]);
    hil::gpio::Pin::enable_interrupt(&arty_exx::gpio::PORT[4], 4, hil::gpio::InterruptMode::RisingEdge);

    hil::gpio::Pin::make_input(&arty_exx::gpio::PORT[3]);
    hil::gpio::Pin::enable_interrupt(&arty_exx::gpio::PORT[3], 3, hil::gpio::InterruptMode::RisingEdge);
    // hil::gpio::Pin::make_input(&arty_exx::gpio::PORT[16]);
    // hil::gpio::Pin::enable_interrupt(&arty_exx::gpio::PORT[16], 0, hil::gpio::InterruptMode::RisingEdge);
    // hil::gpio::Pin::clear(&arty_exx::gpio::PORT[8]);




    chip.enable_all_interrupts();




    let artye21 = ArtyE21 {
        console: console,
        gpio: gpio,
        alarm: alarm,
        led: led,
        // button: button,
        // ipc: kernel::ipc::IPC::new(board_kernel),
    };

    artye21.console.initialize();

    // Create virtual device for kernel debug.
    let debugger_uart = static_init!(UartDevice, UartDevice::new(uart_mux, false));
    debugger_uart.setup();
    let debugger = static_init!(
        kernel::debug::DebugWriter,
        kernel::debug::DebugWriter::new(
            debugger_uart,
            &mut kernel::debug::OUTPUT_BUF,
            &mut kernel::debug::INTERNAL_BUF,
        )
    );
    hil::uart::UART::set_client(debugger_uart, debugger);

    let debug_wrapper = static_init!(
        kernel::debug::DebugWriterWrapper,
        kernel::debug::DebugWriterWrapper::new(debugger)
    );
    kernel::debug::set_debug_writer_wrapper(debug_wrapper);

    // arty_exx::uart::UART0.initialize_gpio_pins(&arty_exx::gpio::PORT[17], &arty_exx::gpio::PORT[16]);

    debug!("wow cool Initialization complete. Entering main loop and does this matter at all");
    // debug!("Initialize have some more cool content here ok lets do it");


    // timertest.start();

    extern "C" {
        /// Beginning of the ROM region containing app images.
        ///
        /// This symbol is defined in the linker script.
        static _sapps: u8;
    }

    kernel::procs::load_processes(
        board_kernel,
        chip,
        &_sapps as *const u8,
        &mut APP_MEMORY,
        &mut PROCESSES,
        FAULT_RESPONSE,
        &process_mgmt_cap,
    );

    board_kernel.kernel_loop(&artye21, chip, None, &main_loop_cap);
}
