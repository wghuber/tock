//! CSRs

use kernel::common::registers::register_bitfields;
use kernel::common::StaticRef;

use riscv_csr::csr::RiscvCsr;

#[repr(C)]
struct CSR {
    mscratch: RiscvCsr<u32, mscratch::Register>,
}


register_bitfields![u32,
    mscratch [
        scratch OFFSET(0) NUMBITS(32) []
    ]
];



const CSR_BASE: StaticRef<CSR> =
    unsafe { StaticRef::new(0x340 as *const CSR) };


