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

pub mod debuglog;
pub mod dev;
pub mod error;
pub mod ipc;
pub mod object;
pub mod task;
pub mod util;
pub mod vm;

pub use self::error::*;
