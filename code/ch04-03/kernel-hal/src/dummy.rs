use super::*;
use crate::vdso::VdsoConstants;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::time::Duration;

#[derive(Debug)]
pub struct HalError;
/// The result type returned by HAL functions.
pub type Result<T> = core::result::Result<T, HalError>;

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

    /// Set tid and pid of current task.
    #[linkage = "weak"]
    #[export_name = "hal_thread_set_tid"]
    pub fn set_tid(_tid: u64, _pid: u64) {
        unimplemented!()
    }

    /// Get tid and pid of current task.
    #[linkage = "weak"]
    #[export_name = "hal_thread_get_tid"]
    pub fn get_tid() -> (u64, u64) {
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

#[linkage = "weak"]
#[export_name = "hal_context_run"]
pub fn context_run(_context: &mut UserContext) {
    unimplemented!()
}

/// Get platform specific information.
#[linkage = "weak"]
#[export_name = "hal_vdso_constants"]
pub fn vdso_constants() -> VdsoConstants {
    unimplemented!()
}

/// Read a string from console.
#[linkage = "weak"]
#[export_name = "hal_serial_read"]
pub fn serial_read(_buf: &mut [u8]) -> usize {
    unimplemented!()
}

/// Output a string to console.
#[linkage = "weak"]
#[export_name = "hal_serial_write"]
pub fn serial_write(_s: &str) {
    unimplemented!()
}
