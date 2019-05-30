//! Core Local Interrupt Control

use kernel::common::registers::{self, ReadOnly, ReadWrite, WriteOnly};
use kernel::common::StaticRef;


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
    _reserved4: [u8; 880]
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
    _reserved4: [u8; 880]
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
    localint: [ReadWrite<u8, intcon::Register>; 128],
    _reserved4: [u8; 880]
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

// The data sheet isn't completely clear on this field, but it looks like there
// are four bits for priority and level, and the lowest for bits of the register
// are reserved.
register_bitfields![u8,
      intcon [
          IntCon OFFSET(4) NUMBITS(4) []
      ]
  ];

register_bitfields![u8,
      conreg [
          nvbits OFFSET(0) NUMBITS(1) [],
          nlbits OFFSET(1) NUMBITS(4) [],
          nmbits OFFSET(5) NUMBITS(2) []
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
        pending.write(intpend::IntPend::CLEAR);
    }
}

// Disable machine timer interrupt
pub unsafe fn disable_mtip() {
    let clic: &ClicRegisters = &*CLIC_BASE;

    clic.clicintie.mtip.write(inten::IntEn::CLEAR);
}

/// Enable ONLY the interrupts we actually want to use.
///
/// The CLIC allows disabled interrupts to still set the pending bit. Therefore
/// we have to be very careful about which interrupts we check.
pub unsafe fn enable_all() {
    let clic: &ClicRegisters = &*CLIC_BASE;

    // clic.clicintie.msip.write(inten::IntEn::SET);
    // clic.clicintie.mtip.write(inten::IntEn::SET);
    // clic.clicintie.meip.write(inten::IntEn::SET);
    // clic.clicintie.csip.write(inten::IntEn::SET);

    for (i,enable) in clic.clicintie.localint.iter().enumerate() {
        if i >= 3 && i <= 16 {
            enable.write(inten::IntEn::SET);
        }
    }
}

// Disable pending interrupts
pub unsafe fn disable_pending() {
    let clic: &ClicRegisters = &*CLIC_BASE;

    // Iterate through all interrupts. If the interrupt is enabled and it
    // is pending then disable the interrupt.
    for (i, pending) in clic.clicintip.localintpend.iter().enumerate() {
        if pending.is_set(intpend::IntPend) && clic.clicintie.localint[i].is_set(inten::IntEn) {
            clic.clicintie.localint[i].write(inten::IntEn::CLEAR);
        }
    }
}

/// Disable all interrupts.
pub unsafe fn disable_all() {
    let clic: &ClicRegisters = &*CLIC_BASE;

    clic.clicintie.msip.write(inten::IntEn::CLEAR);
    clic.clicintie.mtip.write(inten::IntEn::CLEAR);
    clic.clicintie.meip.write(inten::IntEn::CLEAR);
    clic.clicintie.csip.write(inten::IntEn::CLEAR);

    for enable in clic.clicintie.localint.iter() {
        enable.write(inten::IntEn::CLEAR);
    }
}

/// Get the index (0-256) of the lowest number pending interrupt, or `None` if
/// none is pending.
pub unsafe fn next_pending() -> Option<u32> {
    let clic: &ClicRegisters = &*CLIC_BASE;

    // HACK
    // Ignore these interrupts since we don't use them.
    // if clic.clicintip.msip.is_set(intpend::IntPend) {
    //     return Some(3);
    // } else if clic.clicintip.mtip.is_set(intpend::IntPend) {
    //     return Some(7);
    // } else if clic.clicintip.meip.is_set(intpend::IntPend) {
    //     return Some(11);
    // } else if clic.clicintip.csip.is_set(intpend::IntPend) {
    //     return Some(12);
    // }

    for (i, pending) in clic.clicintip.localintpend.iter().enumerate() {
        // HACK HACK
        // Skip these interrupt numbers. I'm not sure what they go to, but they
        // seem to always be pending.
        if i < 3 || i > 16 {
            continue;
        }

        if pending.is_set(intpend::IntPend) {
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
        16...144 => {
            clic.clicintip.localintpend[(index as usize) - 16].write(intpend::IntPend::CLEAR);
        },
        _ => {}
    }
}

/// Return `true` if there are any pending interrupts in the CLIC, `false`
/// otherwise.
pub unsafe fn has_pending() -> bool {
    let clic: &ClicRegisters = &*CLIC_BASE;

    // if clic.clicintip.csip.is_set(intpend::IntPend) {
    //     return true;
    // }

    for (i, pending) in clic.clicintip.localintpend.iter().enumerate() {
        // HACK HACK
        // Skip these interrupt numbers. I'm not sure what they go to, but they
        // seem to always be pending.
        if i < 3 || i > 16 {
            continue;
        }

        if pending.is_set(intpend::IntPend) {
            return true;
        }
    }

    return false;
}

