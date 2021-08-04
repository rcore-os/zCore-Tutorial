#![feature(asm)]
#![feature(linkage)]
#![deny(warnings)]

extern crate alloc;

use {
    alloc::boxed::Box,
    core::time::Duration,
    core::{future::Future, pin::Pin},
    std::time::SystemTime,
};

pub use trapframe::{GeneralRegs, UserContext};

#[repr(C)]
pub struct Thread {
    thread: usize,
}

impl Thread {
    #[export_name = "hal_thread_spawn"]
    pub fn spawn(
        future: Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
        _vmtoken: usize,
    ) -> Self {
        async_std::task::spawn(future);
        Thread { thread: 0 }
    }
}

/// Get current time.
#[export_name = "hal_timer_now"]
pub fn timer_now() -> Duration {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
}

/// Initialize the HAL.
///
/// This function must be called at the beginning.
pub fn init() {
    #[cfg(target_os = "macos")]
    unimplemented!()
}
