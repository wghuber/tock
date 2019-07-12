#![no_std]
#![feature(in_band_lifetimes)]

#[macro_use]
pub mod led;
#[macro_use]
pub mod isl29035;
pub mod rng;
#[macro_use]
pub mod crc;
#[macro_use]
pub mod alarm;
pub mod process_console;
pub mod console;
pub mod nrf51822;
