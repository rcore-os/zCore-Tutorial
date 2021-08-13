//! Zircon syscall implementations

#![no_std]
#![deny(warnings, unsafe_code, unused_must_use, unreachable_patterns)]

extern crate alloc;

#[macro_use]
extern crate log;

use {
    core::convert::TryFrom,
    kernel_hal::user::*,
    zircon_object::object::*,
    zircon_object::task::{CurrentThread, ThreadFn},
};

mod channel;
mod consts;
mod debuglog;

use consts::SyscallType as Sys;

pub struct Syscall<'a> {
    pub thread: &'a CurrentThread,
    pub thread_fn: ThreadFn,
}

impl Syscall<'_> {
    pub async fn syscall(&mut self, num: u32, args: [usize; 8]) -> isize {
        let thread_name = self.thread.name();
        let proc_name = self.thread.proc().name();
        let sys_type = match Sys::try_from(num) {
            Ok(t) => t,
            Err(_) => {
                error!("invalid syscall number: {}", num);
                return ZxError::INVALID_ARGS as _;
            }
        };
        info!(
            "{}|{} {:?} => args={:x?}",
            proc_name, thread_name, sys_type, args
        );
        let [a0, a1, a2, a3, a4, a5, a6, a7] = args;
        let ret = match sys_type {
            Sys::CHANNEL_CREATE => self.sys_channel_create(a0 as _, a1.into(), a2.into()),
            Sys::CHANNEL_READ => self.sys_channel_read(
                a0 as _,
                a1 as _,
                a2.into(),
                a3 as _,
                a4 as _,
                a5 as _,
                a6.into(),
                a7.into(),
            ),
            Sys::CHANNEL_WRITE => {
                self.sys_channel_write(a0 as _, a1 as _, a2.into(), a3 as _, a4.into(), a5 as _)
            }
            Sys::DEBUGLOG_CREATE => self.sys_debuglog_create(a0 as _, a1 as _, a2.into()),
            Sys::DEBUGLOG_WRITE => self.sys_debuglog_write(a0 as _, a1 as _, a2.into(), a3 as _),
            Sys::DEBUGLOG_READ => self.sys_debuglog_read(a0 as _, a1 as _, a2.into(), a3 as _),
            _ => {
                error!("syscall unimplemented: {:?}", sys_type);
                Err(ZxError::NOT_SUPPORTED)
            }
        };
        info!("{}|{} {:?} <= {:?}", proc_name, thread_name, sys_type, ret);
        match ret {
            Ok(_) => 0,
            Err(err) => err as isize,
        }
    }
}
