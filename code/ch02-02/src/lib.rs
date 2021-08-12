#![no_std]
#![deny(unused_imports)]
#![allow(dead_code)]
#![feature(get_mut_unchecked)]

extern crate alloc;

#[cfg(test)]
#[macro_use]
extern crate std;

mod error;
mod ipc;
mod object;
mod task;
mod vm;

pub use self::error::*;
