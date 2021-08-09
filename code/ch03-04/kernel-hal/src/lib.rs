#![no_std]
#![feature(linkage)]
#![deny(warnings)]

extern crate alloc;

pub use trapframe::{GeneralRegs, UserContext};

use {
    alloc::{boxed::Box, vec::Vec},
    core::{future::Future, pin::Pin, time::Duration},
};

#[derive(Debug)]
pub struct HalError;
/// The result type returned by HAL functions.
pub type Result<T> = core::result::Result<T, HalError>;

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

pub trait PageTableTrait: Sync + Send {
    /// Map the page of `vaddr` to the frame of `paddr` with `flags`.
    fn map(&mut self, _vaddr: VirtAddr, _paddr: PhysAddr, _flags: MMUFlags) -> Result<()>;

    /// Unmap the page of `vaddr`.
    fn unmap(&mut self, _vaddr: VirtAddr) -> Result<()>;

    /// Change the `flags` of the page of `vaddr`.
    fn protect(&mut self, _vaddr: VirtAddr, _flags: MMUFlags) -> Result<()>;

    /// Query the physical address which the page of `vaddr` maps to.
    fn query(&mut self, _vaddr: VirtAddr) -> Result<PhysAddr>;

    /// Get the physical address of root page table.
    fn table_phys(&self) -> PhysAddr;

    #[cfg(target_arch = "riscv64")]
    /// Activate this page table
    fn activate(&self);

    fn map_many(
        &mut self,
        mut vaddr: VirtAddr,
        paddrs: &[PhysAddr],
        flags: MMUFlags,
    ) -> Result<()> {
        for &paddr in paddrs {
            self.map(vaddr, paddr, flags)?;
            vaddr += PAGE_SIZE;
        }
        Ok(())
    }

    fn map_cont(
        &mut self,
        mut vaddr: VirtAddr,
        paddr: PhysAddr,
        pages: usize,
        flags: MMUFlags,
    ) -> Result<()> {
        for i in 0..pages {
            let paddr = paddr + i * PAGE_SIZE;
            self.map(vaddr, paddr, flags)?;
            vaddr += PAGE_SIZE;
        }
        Ok(())
    }

    fn unmap_cont(&mut self, vaddr: VirtAddr, pages: usize) -> Result<()> {
        for i in 0..pages {
            self.unmap(vaddr + i * PAGE_SIZE)?;
        }
        Ok(())
    }
}

/// Page Table
#[repr(C)]
pub struct PageTable {
    table_phys: PhysAddr,
}

impl PageTable {
    /// Get current page table
    #[linkage = "weak"]
    #[export_name = "hal_pt_current"]
    pub fn current() -> Self {
        unimplemented!()
    }

    /// Create a new `PageTable`.
    #[allow(clippy::new_without_default)]
    #[linkage = "weak"]
    #[export_name = "hal_pt_new"]
    pub fn new() -> Self {
        unimplemented!()
    }
}

impl PageTableTrait for PageTable {
    /// Map the page of `vaddr` to the frame of `paddr` with `flags`.
    #[linkage = "weak"]
    #[export_name = "hal_pt_map"]
    fn map(&mut self, _vaddr: VirtAddr, _paddr: PhysAddr, _flags: MMUFlags) -> Result<()> {
        unimplemented!()
    }
    /// Unmap the page of `vaddr`.
    #[linkage = "weak"]
    #[export_name = "hal_pt_unmap"]
    fn unmap(&mut self, _vaddr: VirtAddr) -> Result<()> {
        unimplemented!()
    }
    /// Change the `flags` of the page of `vaddr`.
    #[linkage = "weak"]
    #[export_name = "hal_pt_protect"]
    fn protect(&mut self, _vaddr: VirtAddr, _flags: MMUFlags) -> Result<()> {
        unimplemented!()
    }
    /// Query the physical address which the page of `vaddr` maps to.
    #[linkage = "weak"]
    #[export_name = "hal_pt_query"]
    fn query(&mut self, _vaddr: VirtAddr) -> Result<PhysAddr> {
        unimplemented!()
    }
    /// Get the physical address of root page table.
    #[linkage = "weak"]
    #[export_name = "hal_pt_table_phys"]
    fn table_phys(&self) -> PhysAddr {
        self.table_phys
    }

    /// Activate this page table
    #[cfg(target_arch = "riscv64")]
    #[linkage = "weak"]
    #[export_name = "hal_pt_activate"]
    fn activate(&self) {
        unimplemented!()
    }

    #[linkage = "weak"]
    #[export_name = "hal_pt_unmap_cont"]
    fn unmap_cont(&mut self, vaddr: VirtAddr, pages: usize) -> Result<()> {
        for i in 0..pages {
            self.unmap(vaddr + i * PAGE_SIZE)?;
        }
        Ok(())
    }
}
