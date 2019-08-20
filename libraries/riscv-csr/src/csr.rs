//! CSR




// use core::fmt;
use core::marker::PhantomData;
// use core::ops::{Add, AddAssign, BitAnd, BitOr, Not, Shl, Shr};

use tock_registers::registers::{RegisterLongName, IntLike, FieldValue, Field,
TryFromValue};

/// Read/Write registers.
pub struct RiscvCsr<T: IntLike, R: RegisterLongName = ()> {
    value: T,
    associated_register: PhantomData<R>,
}

impl<T: IntLike, R: RegisterLongName> RiscvCsr<T, R> {
    pub const fn new(value: T) -> Self {
        RiscvCsr {
            value: value,
            associated_register: PhantomData,
        }
    }

    #[inline]
    pub fn get(&self) -> T {
        // unsafe { ::core::ptr::read_volatile(&self.value) }
        let r: T;
        unsafe { asm!("csrr $0, $1" : "=r"(r) : "i"(self.value) :: "volatile") }
        r
    }

    #[inline]
    pub fn set(&self, value: T) {
        // unsafe { ::core::ptr::write_volatile(&self.value as *const T as *mut T, value) }
        unsafe { asm!("csrw $1, $0" :: "r"(value), "i"(self.value) :: "volatile") }
    }

    #[inline]
    pub fn read(&self, field: Field<T, R>) -> T {
        (self.get() & (field.mask << field.shift)) >> field.shift
    }

    #[inline]
    pub fn read_as_enum<E: TryFromValue<T, EnumType = E>>(&self, field: Field<T, R>) -> Option<E> {
        let val: T = self.read(field);

        E::try_from(val)
    }

    // #[inline]
    // pub fn extract(&self) -> LocalRegisterCopy<T, R> {
    //     LocalRegisterCopy::new(self.get())
    // }

    #[inline]
    pub fn write(&self, field: FieldValue<T, R>) {
        self.set(field.value);
    }

    #[inline]
    pub fn modify(&self, field: FieldValue<T, R>) {
        let reg: T = self.get();
        self.set((reg & !field.mask) | field.value);
    }

    #[inline]
    // pub fn modify_no_read(&self, original: LocalRegisterCopy<T, R>, field: FieldValue<T, R>) {
    //     self.set((original.get() & !field.mask) | field.value);
    // }

    #[inline]
    pub fn is_set(&self, field: Field<T, R>) -> bool {
        self.read(field) != T::zero()
    }

    #[inline]
    pub fn matches_any(&self, field: FieldValue<T, R>) -> bool {
        self.get() & field.mask != T::zero()
    }

    #[inline]
    pub fn matches_all(&self, field: FieldValue<T, R>) -> bool {
        self.get() & field.mask == field.value
    }
}
