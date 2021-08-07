#![no_std]
#![feature(linkage)]
#![deny(warnings)]

extern crate alloc;

pub use trapframe::{GeneralRegs, UserContext};

use {
    alloc::{boxed::Box, vec::Vec},
    core::{future::Future, pin::Pin, time::Duration},
};

pub mod defs {
    use bitflags::bitflags;
    use numeric_enum_macro::numeric_enum;

    bitflags! {
        pub struct MMUFlags: usize {
            #[allow(clippy::identity_op)]
            const CACHE_1   = 1 << 0;
            const CACHE_2   = 1 << 1;
            const READ      = 1 << 2;
            const WRITE     = 1 << 3;
            const EXECUTE   = 1 << 4;
            const USER      = 1 << 5;
            const RXW = Self::READ.bits | Self::WRITE.bits | Self::EXECUTE.bits;
        }
    }
    numeric_enum! {
        #[repr(u32)]
        #[derive(Debug, PartialEq, Clone, Copy)]
        pub enum CachePolicy {
            Cached = 0,
            Uncached = 1,
            UncachedDevice = 2,
            WriteCombining = 3,
        }
    }
    pub const CACHE_POLICY_MASK: u32 = 3;

    impl Default for CachePolicy {
        fn default() -> Self {
            Self::Cached
        }
    }

    pub type PhysAddr = usize;
    pub type VirtAddr = usize;
    pub type DevVAddr = usize;
    pub const PAGE_SIZE: usize = 0x1000;
}

pub use self::defs::*;

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

#[repr(C)]
pub struct PhysFrame {
    paddr: PhysAddr,
}

impl PhysFrame {
    #[linkage = "weak"]
    #[export_name = "hal_frame_alloc"]
    pub fn alloc() -> Option<Self> {
        unimplemented!()
    }

    #[linkage = "weak"]
    #[export_name = "hal_frame_alloc_contiguous"]
    pub fn alloc_contiguous_base(_size: usize, _align_log2: usize) -> Option<PhysAddr> {
        unimplemented!()
    }

    pub fn alloc_contiguous(size: usize, align_log2: usize) -> Vec<Self> {
        PhysFrame::alloc_contiguous_base(size, align_log2).map_or(Vec::new(), |base| {
            (0..size)
                .map(|i| PhysFrame {
                    paddr: base + i * PAGE_SIZE,
                })
                .collect()
        })
    }

    pub fn alloc_zeroed() -> Option<Self> {
        Self::alloc().map(|f| {
            pmem_zero(f.addr(), PAGE_SIZE);
            f
        })
    }

    pub fn alloc_contiguous_zeroed(size: usize, align_log2: usize) -> Vec<Self> {
        PhysFrame::alloc_contiguous_base(size, align_log2).map_or(Vec::new(), |base| {
            pmem_zero(base, size * PAGE_SIZE);
            (0..size)
                .map(|i| PhysFrame {
                    paddr: base + i * PAGE_SIZE,
                })
                .collect()
        })
    }

    pub fn addr(&self) -> PhysAddr {
        self.paddr
    }

    #[linkage = "weak"]
    #[export_name = "hal_zero_frame_paddr"]
    pub fn zero_frame_addr() -> PhysAddr {
        unimplemented!()
    }
}

impl Drop for PhysFrame {
    #[linkage = "weak"]
    #[export_name = "hal_frame_dealloc"]
    fn drop(&mut self) {
        unimplemented!()
    }
}

/// Read physical memory from `paddr` to `buf`.
#[linkage = "weak"]
#[export_name = "hal_pmem_read"]
pub fn pmem_read(_paddr: PhysAddr, _buf: &mut [u8]) {
    unimplemented!()
}

/// Write physical memory to `paddr` from `buf`.
#[linkage = "weak"]
#[export_name = "hal_pmem_write"]
pub fn pmem_write(_paddr: PhysAddr, _buf: &[u8]) {
    unimplemented!()
}

/// Zero physical memory at `[paddr, paddr + len)`
#[linkage = "weak"]
#[export_name = "hal_pmem_zero"]
pub fn pmem_zero(_paddr: PhysAddr, _len: usize) {
    unimplemented!()
}

/// Copy content of `src` frame to `target` frame.
#[linkage = "weak"]
#[export_name = "hal_frame_copy"]
pub fn frame_copy(_src: PhysAddr, _target: PhysAddr) {
    unimplemented!()
}

/// Flush the physical frame.
#[linkage = "weak"]
#[export_name = "hal_frame_flush"]
pub fn frame_flush(_target: PhysAddr) {
    unimplemented!()
}
