#![no_std]
#![deny(unused_imports)]
#![allow(dead_code)]
#![feature(get_mut_unchecked)]
#![feature(drain_filter)]

extern crate alloc;

#[cfg(test)]
#[macro_use]
extern crate std;

#[macro_use]
extern crate log;

mod error;
mod ipc;
mod object;
mod task;
mod util;
mod vm;

pub use self::error::*;
