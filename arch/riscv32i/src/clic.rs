//! Core Local Interrupt Control

use kernel::common::registers::{self, ReadOnly, ReadWrite, WriteOnly};
use kernel::common::StaticRef;

// use kernel::debug;
// use kernel::gpio;

#[repr(C)]

//CLIC Hart Specific Region
struct ClicRegisters {
    //CLIC Interrupt Pending Registers
    clicintip: IntPendRegisters,
    //CLIC Interrupt Enable Registers
    clicintie: IntEnableRegisters,
    //CLIC Interrupt Configuration Registers
    clicintcfg: IntConfigRegisters,
    //CLIC Configuration Registers
    cliccfg: ConfigRegisters
}

//Interrupt Pending Registers
struct IntPendRegisters {
    //Reserved Section
    _reserved0: [u8; 3],
    //Machine Software Interrupt
    msip: [ReadWrite<u8, intpend::Register>; 1],
    _reserved1: [u8; 3],
    //Machine Timer Interrupt
    mtip: [ReadWrite<u8, intpend::Register>; 1],
    _reserved2: [u8; 3],
    //Machine External Interrupt
    meip: [ReadWrite<u8, intpend::Register>; 1],
    //CLIC Software Interrupt
    csip: [ReadWrite<u8, intpend::Register>; 1],
    _reserved3: [u8; 3],
    //Local Interrupt 0-127
    localintpend: [ReadWrite<u8, intpend::Register>; 128],
    _reserved4: [u8; 880]
}

//Interrupt Enable Registers
struct IntEnableRegisters {
    //Reserved Section
    _reserved0: [u8; 3],
    //Machine Software Interrupt
    msip: [ReadWrite<u8, inten::Register>; 1],
    _reserved1: [u8; 3],
    //Machine Timer Interrupt
    mtip: [ReadWrite<u8, inten::Register>; 1],
    _reserved2: [u8; 3],
    //Machine External Interrupt
    meip: [ReadWrite<u8, inten::Register>; 1],
    //CLIC Software Interrupt
    csip: [ReadWrite<u8, inten::Register>; 1],
    _reserved3: [u8; 3],
    //Local Interrupt 0-127
    localint: [ReadWrite<u8, inten::Register>; 128],
    _reserved4: [u8; 880]
}

//Interrupt Configuration Registers
struct IntConfigRegisters {
    //Reserved Section
    _reserved0: [u8; 3],
    //Machine Software Interrupt
    msip: [ReadWrite<u8, intcon::Register>; 1],
    _reserved1: [u8; 3],
    //Machine Timer Interrupt
    mtip: [ReadWrite<u8, intcon::Register>; 1],
    _reserved2: [u8; 3],
    //Machine External Interrupt
    meip: [ReadWrite<u8, intcon::Register>; 1],
    //CLIC Software Interrupt
    csip: [ReadWrite<u8, intcon::Register>; 1],
    _reserved3: [u8; 3],
    //Local Interrupt 0-127
    localint: [ReadWrite<u8, intcon::Register>; 128],
    _reserved4: [u8; 880]
}

//Configuration Register
struct ConfigRegisters {
    //Reserved Section
    _reserved0: [u8; 3],
    //Machine Software Interrupt
    msip: [ReadWrite<u8, conreg::Register>; 1],
    _reserved1: [u8; 3],
    //Machine Timer Interrupt
    mtip: [ReadWrite<u8, conreg::Register>; 1],
    _reserved2: [u8; 3],
    //Machine External Interrupt
    meip: [ReadWrite<u8, conreg::Register>; 1],
    //CLIC Software Interrupt
    csip: [ReadWrite<u8, conreg::Register>; 1],
    _reserved3: [u8; 3],
    //Local Interrupt 0-127
    localint: [ReadWrite<u8, conreg::Register>; 128],
    _reserved4: [u8; 880]
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
          IntCon OFFSET(4) NUMBITS(4) []
      ]
  ];

register_bitfields![u8,
      conreg [
          nvbits OFFSET(0) NUMBITS(1) [],
          nlbits OFFSET(1) NUMBITS(3) [],
          nmbits OFFSET(1) NUMBITS(3) []
      ]
  ];

const CLIC_BASE: StaticRef<ClicRegisters> =
    unsafe { StaticRef::new(0x0280_0000 as *const ClicRegisters) };

/// Clear all pending interrupts.
pub unsafe fn clear_all_pending() {
    let clic: &ClicRegisters = &*CLIC_BASE;
    for pending in clic.clicintip.localintpend.iter() {
        pending.set(0);
    }
}

/// Enable all interrupts.
pub unsafe fn enable_all() {
    let clic: &ClicRegisters = &*CLIC_BASE;
    for enable in clic.clicintie.localint.iter() {
        enable.set(0xFFFF_FFFF);
    }

    // Set some default priority for each interrupt. This is not really used
    // at this point.
    //for priority in clic.priority.iter() {
    //    priority.write(priority::Priority.val(4));
    //}

    // Accept all interrupts.
    //clic.threshold.write(priority::Priority.val(0));
}

/// Disable all interrupts.
pub unsafe fn disable_all() {
    let clic: &ClicRegisters = &*CLIC_BASE;
    for enable in clic.clicintie.localint.iter() {
        enable.set(0);
    }
}

/// Get the index (0-256) of the lowest number pending interrupt, or `None` if
/// none is pending.
pub unsafe fn next_pending() -> Option<u32> {
    let clic: &ClicRegisters = &*CLIC_BASE;
    let mut i = 0;
    for pending in clic.clicintip.localintpend.iter() {
            i += 1;
            if pending.get() = 0x00 {
                debug!("{}", pending.get());
                debug_gpio!(0, set);
                return Some(i);
        }
    }
    return None;
}

/// Signal that an interrupt is finished being handled. In Tock, this should be
/// called from the normal main loop (not the interrupt handler).
pub unsafe fn complete(index: u32) {
    let clic: &ClicRegisters = &*CLIC_BASE;
    for (i, pending) in clic.clicintip.localintpend.iter().enumerate() {
        if i == (index as usize){
            pending.set(0);
        }
    }
}

/// Return `true` if there are any pending interrupts in the CLIC, `false`
/// otherwise.
pub unsafe fn has_pending() -> bool {
    let clic: &ClicRegisters = &*CLIC_BASE;

    clic.clicintip.localintpend.iter().fold(0, |i, localintpend| localintpend.get() | i) != 0
}