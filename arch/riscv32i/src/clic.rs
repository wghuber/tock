//! Core Local Interrupt Control

use kernel::common::registers::{self, ReadOnly, ReadWrite, WriteOnly};
use kernel::common::StaticRef;

// use kernel::debug;
// use kernel::gpio;



//CLIC Hart Specific Region
#[repr(C)]
struct ClicRegisters {
    // CLIC Interrupt Pending Registers
    clicintip: IntPendRegisters,
    // CLIC Interrupt Enable Registers
    clicintie: IntEnableRegisters,
    // CLIC Interrupt Configuration Registers
    clicintcfg: IntConfigRegisters,
    // CLIC Configuration Registers
    cliccfg: ConfigRegisters
}

//Interrupt Pending Registers
#[repr(C)]
struct IntPendRegisters {
    //Reserved Section
    _reserved0: [u8; 3],
    //Machine Software Interrupt
    msip: ReadWrite<u8, intpend::Register>,
    _reserved1: [u8; 3],
    //Machine Timer Interrupt
    mtip: ReadWrite<u8, intpend::Register>,
    _reserved2: [u8; 3],
    //Machine External Interrupt
    meip: ReadWrite<u8, intpend::Register>,
    //CLIC Software Interrupt
    csip: ReadWrite<u8, intpend::Register>,
    _reserved3: [u8; 3],
    //Local Interrupt 0-127
    localintpend: [ReadWrite<u8, intpend::Register>; 128],
    // localintpend: [ReadWrite<u8, intpend::Register>; 1008],
    _reserved4: [u8; 880]




    // localintpend: [ReadWrite<u8, intpend::Register>; 1024],
}

//Interrupt Enable Registers
#[repr(C)]
struct IntEnableRegisters {
    //Reserved Section
    _reserved0: [u8; 3],
    //Machine Software Interrupt
    msip: ReadWrite<u8, inten::Register>,
    _reserved1: [u8; 3],
    //Machine Timer Interrupt
    mtip: ReadWrite<u8, inten::Register>,
    _reserved2: [u8; 3],
    //Machine External Interrupt
    meip: ReadWrite<u8, inten::Register>,
    //CLIC Software Interrupt
    csip: ReadWrite<u8, inten::Register>,
    _reserved3: [u8; 3],
    //Local Interrupt 0-127
    localint: [ReadWrite<u8, inten::Register>; 128],
    // localint: [ReadWrite<u8, inten::Register>; 1008],
    _reserved4: [u8; 880]


    // localint: [ReadWrite<u8, inten::Register>; 1024],
}

//Interrupt Configuration Registers
#[repr(C)]
struct IntConfigRegisters {
    //Reserved Section
    _reserved0: [u8; 3],
    //Machine Software Interrupt
    msip: ReadWrite<u8, intcon::Register>,
    _reserved1: [u8; 3],
    //Machine Timer Interrupt
    mtip: ReadWrite<u8, intcon::Register>,
    _reserved2: [u8; 3],
    //Machine External Interrupt
    meip: ReadWrite<u8, intcon::Register>,
    //CLIC Software Interrupt
    csip: ReadWrite<u8, intcon::Register>,
    _reserved3: [u8; 3],
    //Local Interrupt 0-127
    localint: [ReadWrite<u8, intcon::Register>; 1008],
    //_reserved4: [u8; 880]
}

//Configuration Register
#[repr(C)]
struct ConfigRegisters {
    cliccfg: ReadWrite<u8, conreg::Register>,
}



register_bitfields![u8,
      intpend [
          IntPend OFFSET(0) NUMBITS(1) []
      ]
  ];

register_bitfields![u8,
      inten [
          IntEn OFFSET(0) NUMBITS(1) []
      ]
  ];

//NOT SURE WHICH BITS PG28
register_bitfields![u8,
      intcon [
          IntCon OFFSET(3) NUMBITS(4) []
      ]
  ];

register_bitfields![u8,
      conreg [
          nvbits OFFSET(0) NUMBITS(1) [],
          nlbits OFFSET(1) NUMBITS(3) [],
          nmbits OFFSET(4) NUMBITS(3) []
      ]
  ];

const CLIC_BASE: StaticRef<ClicRegisters> =
    unsafe { StaticRef::new(0x0280_0000 as *const ClicRegisters) };

/// Clear all pending interrupts.
pub unsafe fn clear_all_pending() {
    let clic: &ClicRegisters = &*CLIC_BASE;

    clic.clicintip.msip.write(intpend::IntPend::CLEAR);
    clic.clicintip.mtip.write(intpend::IntPend::CLEAR);
    clic.clicintip.meip.write(intpend::IntPend::CLEAR);
    clic.clicintip.csip.write(intpend::IntPend::CLEAR);

    for pending in clic.clicintip.localintpend.iter() {
        pending.set(0);
        // pending.write(intpend::IntPend::CLEAR);
    }
}

/// Enable all interrupts.
pub unsafe fn enable_all() {
    let clic: &ClicRegisters = &*CLIC_BASE;



    clic.clicintie.msip.write(inten::IntEn::SET);
    clic.clicintie.mtip.write(inten::IntEn::SET);
    clic.clicintie.meip.write(inten::IntEn::SET);
    clic.clicintie.csip.write(inten::IntEn::SET);


    for (i,enable) in clic.clicintie.localint.iter().enumerate() {
        // enable.set(enable.get() | 0x01);
        // enable.write(inten::IntEn::SET);
        // enable.write(inten::IntEn::CLEAR);

        // if i != 0 {
            enable.set(1);
        // }
    }









    // debug_gpio!(0, set);

    // Set some default priority for each interrupt. This is not really used
    // at this point.
    //for priority in clic.priority.iter() {
    //    priority.write(priority::Priority.val(4));
    //}

    // Accept all interrupts.
    //clic.threshold.write(priority::Priority.val(0));

    // clic.clicintip.csip.write(intpend::IntPend::SET);


    // clic.clicintip.localintpend[32].set(1);
    // clic.clicintip.localintpend[12].set(1);


    // if clic.clicintip.localintpend[12].get() != 0 {


    // // if clic.clicintie.localint[12].get() != 0 {
    //     debug_gpio!(1, set);
    // }
    // debug_gpio!(1, set);
}

/// Disable all interrupts.
pub unsafe fn disable_all() {
    let clic: &ClicRegisters = &*CLIC_BASE;



    clic.clicintie.msip.write(inten::IntEn::CLEAR);
    clic.clicintie.mtip.write(inten::IntEn::CLEAR);
    clic.clicintie.meip.write(inten::IntEn::CLEAR);
    clic.clicintie.csip.write(inten::IntEn::CLEAR);

    for enable in clic.clicintie.localint.iter() {
        // enable.set(0);
        enable.write(inten::IntEn::CLEAR);
    }
}

/// Get the index (0-256) of the lowest number pending interrupt, or `None` if
/// none is pending.
pub unsafe fn next_pending() -> Option<u32> {
    let clic: &ClicRegisters = &*CLIC_BASE;
    // let mut i = 0;
    //


    if clic.clicintip.msip.is_set(intpend::IntPend) {
        return Some(3);
    }
    if clic.clicintip.mtip.is_set(intpend::IntPend) {
        return Some(7);
    }
    if clic.clicintip.meip.is_set(intpend::IntPend) {
        return Some(11);
    }
    if clic.clicintip.csip.is_set(intpend::IntPend) {
        return Some(12);
    }

    for (i, pending) in clic.clicintip.localintpend.iter().enumerate() {
            // i += 1;
            // if pending.get() != 0x10 {
            if pending.is_set(intpend::IntPend) {
                //debug_gpio!(0, set);
                return Some((i+16) as u32);
        }
    }
    return None;
}

/// Signal that an interrupt is finished being handled. In Tock, this should be
/// called from the normal main loop (not the interrupt handler).
pub unsafe fn complete(index: u32) {
    let clic: &ClicRegisters = &*CLIC_BASE;

    match index {
        3 => clic.clicintip.msip.write(intpend::IntPend::CLEAR),
        7 => clic.clicintip.mtip.write(intpend::IntPend::CLEAR),
        11 => clic.clicintip.meip.write(intpend::IntPend::CLEAR),
        12 => clic.clicintip.csip.write(intpend::IntPend::CLEAR),
        16...128 => {
            clic.clicintip.localintpend[(index as usize) - 16].write(intpend::IntPend::CLEAR);
        },
        _ => {}
    }

    // if index < 16 {

    //     if index == 12 {
    //         clic.clicintip.csip.write(intpend::IntPend::CLEAR);
    //     }

    //     if index == 12 {
    //         clic.clicintip.csip.write(intpend::IntPend::CLEAR);
    //     }

    // } else {

    //     clic.clicintip.localintpend[(index as usize) - 16].write(intpend::IntPend::CLEAR);

    // }

    // for (i, pending) in clic.clicintip.localintpend.iter().enumerate() {
    //     if i == (index as usize){
    //         pending.set(0);
    //     }
    // }
}

/// Return `true` if there are any pending interrupts in the CLIC, `false`
/// otherwise.
pub unsafe fn has_pending() -> bool {
    let clic: &ClicRegisters = &*CLIC_BASE;

    // return true;

    if clic.clicintip.csip.is_set(intpend::IntPend) {
        return true;
    }

    for (i, pending) in clic.clicintip.localintpend.iter().enumerate() {
        if pending.is_set(intpend::IntPend)  {
            return true;
        }
    }

    return false;

    // clic.clicintip.localintpend.iter().fold(0, |i, localintpend| localintpend.get() | i) != 0
}
