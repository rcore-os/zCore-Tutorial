#![no_std]
#![feature(linkage)]
#![deny(warnings)]

extern crate alloc;

pub use trapframe::{GeneralRegs, UserContext};

use {
    alloc::boxed::Box,
    core::{future::Future, pin::Pin, time::Duration},
};

#[repr(C)]
pub struct Thread {
    id: usize,
}

impl Thread {
    /// Spawn a new thread.
    #[linkage = "weak"]
    #[export_name = "hal_thread_spawn"]
    pub fn spawn(
        _future: Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
        _vmtoken: usize,
    ) -> Self {
        unimplemented!()
    }
}

#[linkage = "weak"]
#[export_name = "hal_timer_now"]
pub fn timer_now() -> Duration {
    unimplemented!()
}
