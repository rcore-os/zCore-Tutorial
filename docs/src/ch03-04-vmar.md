# 虚拟内存：VMAR 对象

## VMAR 简介

虚拟内存地址区域（Virtual Memory Address Regions ，VMARs）为管理进程的地址空间提供了一种抽象。在进程创建时，将Root VMAR 的句柄提供给进程创建者。该句柄指的是跨越整个地址空间的 VMAR。这个空间可以通过[`zx_vmar_map()`](https://fuchsia.dev/docs/reference/syscalls/vmar_map)和 [`zx_vmar_allocate()`](https://fuchsia.dev/docs/reference/syscalls/vmar_allocate)接口来划分 。 [`zx_vmar_allocate()`](https://fuchsia.dev/docs/reference/syscalls/vmar_allocate)可用于生成新的 VMAR（称为子区域或子区域），可用于将地址空间的各个部分组合在一起。

## 实现 VMAR 对象框架

> 定义 VmAddressRange，VmMapping
>
> 实现 create_child, map, unmap, destroy 函数，并做单元测试验证地址空间分配

### VmAddressRegion
```rust
pub struct VmAddressRegion {
    flags: VmarFlags,
    base: KObjectBase,
    addr: VirtAddr,
    size: usize,
    parent: Option<Arc<VmAddressRegion>>,
    page_table: Arc<Mutex<dyn PageTableTrait>>,
    /// If inner is None, this region is destroyed, all operations are invalid.
    inner: Mutex<Option<VmarInner>>,
}

#[derive(Default)]
struct VmarInner {
    children: Vec<Arc<VmAddressRegion>>,
    mappings: Vec<Arc<VmMapping>>,
}
```
构造一个根节点 VMAR，这个 VMAR 是每个进程都拥有的。
```rust
impl VmAddressRegion {
    /// Create a new root VMAR.
    pub fn new_root() -> Arc<Self> {
        let (addr, size) = {
            use core::sync::atomic::*;
            static VMAR_ID: AtomicUsize = AtomicUsize::new(0);
            let i = VMAR_ID.fetch_add(1, Ordering::SeqCst);
            (0x2_0000_0000 + 0x100_0000_0000 * i, 0x100_0000_0000)
        };
        Arc::new(VmAddressRegion {
            flags: VmarFlags::ROOT_FLAGS,
            base: KObjectBase::new(),
            addr,
            size,
            parent: None,
            page_table: Arc::new(Mutex::new(kernel_hal::PageTable::new())), //hal PageTable
            inner: Mutex::new(Some(VmarInner::default())),
        })
    }
}
```
我们的内核同样需要一个根 VMAR
```rust
/// The base of kernel address space
/// In x86 fuchsia this is 0xffff_ff80_0000_0000 instead
pub const KERNEL_ASPACE_BASE: u64 = 0xffff_ff02_0000_0000;
/// The size of kernel address space
pub const KERNEL_ASPACE_SIZE: u64 = 0x0000_0080_0000_0000;
/// The base of user address space
pub const USER_ASPACE_BASE: u64 = 0;
// pub const USER_ASPACE_BASE: u64 = 0x0000_0000_0100_0000;
/// The size of user address space
pub const USER_ASPACE_SIZE: u64 = (1u64 << 47) - 4096 - USER_ASPACE_BASE;

impl VmAddressRegion {
		/// Create a kernel root VMAR.
    pub fn new_kernel() -> Arc<Self> {
        let kernel_vmar_base = KERNEL_ASPACE_BASE as usize;
        let kernel_vmar_size = KERNEL_ASPACE_SIZE as usize;
        Arc::new(VmAddressRegion {
            flags: VmarFlags::ROOT_FLAGS,
            base: KObjectBase::new(),
            addr: kernel_vmar_base,
            size: kernel_vmar_size,
            parent: None,
            page_table: Arc::new(Mutex::new(kernel_hal::PageTable::new())),
            inner: Mutex::new(Some(VmarInner::default())),
        })
    }
}
```
### VmAddressMapping
VmAddressMapping 用于建立 VMO 和 VMAR 之间的映射。
```rust
/// Virtual Memory Mapping
pub struct VmMapping {
    /// The permission limitation of the vmar
    permissions: MMUFlags,
    vmo: Arc<VmObject>,
    page_table: Arc<Mutex<dyn PageTableTrait>>,
    inner: Mutex<VmMappingInner>,
}

#[derive(Debug, Clone)]
struct VmMappingInner {
    /// The actual flags used in the mapping of each page
    flags: Vec<MMUFlags>,
    addr: VirtAddr,
    size: usize,
    vmo_offset: usize,
}
```
map 和 unmap 实现内存映射和解映射
```rust
impl VmMapping {
	/// Map range and commit.
    /// Commit pages to vmo, and map those to frames in page_table.
    /// Temporarily used for development. A standard procedure for
    /// vmo is: create_vmo, op_range(commit), map
    fn map(self: &Arc<Self>) -> ZxResult {
        self.vmo.commit_pages_with(&mut |commit| {
            let inner = self.inner.lock();
            let mut page_table = self.page_table.lock();
            let page_num = inner.size / PAGE_SIZE;
            let vmo_offset = inner.vmo_offset / PAGE_SIZE;
            for i in 0..page_num {
                let paddr = commit(vmo_offset + i, inner.flags[i])?;
                //通过 PageTableTrait 的 hal_pt_map 进行页表映射
                //调用 kernel-hal的方法进行映射
            }
            Ok(())
        })
    }

    fn unmap(&self) {
        let inner = self.inner.lock();
        let pages = inner.size / PAGE_SIZE;
        // TODO inner.vmo_offset unused?
        // 调用 kernel-hal的方法进行解映射
    }
}
```
## HAL：用 mmap 模拟页表

> 实现页表接口 map, unmap, protect

在 kernel-hal 中定义了一个页表和这个页表具有的方法。
```rust
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
```
在 kernel-hal-unix 中实现了 PageTableTrait，在 map 中调用了 mmap。
```rust
impl PageTableTrait for PageTable {
    /// Map the page of `vaddr` to the frame of `paddr` with `flags`.
    #[export_name = "hal_pt_map"]
    fn map(&mut self, vaddr: VirtAddr, paddr: PhysAddr, flags: MMUFlags) -> Result<()> {
        debug_assert!(page_aligned(vaddr));
        debug_assert!(page_aligned(paddr));
        let prot = flags.to_mmap_prot();
        mmap(FRAME_FILE.as_raw_fd(), paddr, PAGE_SIZE, vaddr, prot);
        Ok(())
    }

    /// Unmap the page of `vaddr`.
    #[export_name = "hal_pt_unmap"]
    fn unmap(&mut self, vaddr: VirtAddr) -> Result<()> {
        self.unmap_cont(vaddr, 1)
    }
}
```
## 实现内存映射

> 用 HAL 实现上面 VMAR 留空的部分，并做单元测试验证内存映射
```rust
impl VmMapping {
	/// Map range and commit.
    /// Commit pages to vmo, and map those to frames in page_table.
    /// Temporarily used for development. A standard procedure for
    /// vmo is: create_vmo, op_range(commit), map
    fn map(self: &Arc<Self>) -> ZxResult {
        self.vmo.commit_pages_with(&mut |commit| {
            let inner = self.inner.lock();
            let mut page_table = self.page_table.lock();
            let page_num = inner.size / PAGE_SIZE;
            let vmo_offset = inner.vmo_offset / PAGE_SIZE;
            for i in 0..page_num {
                let paddr = commit(vmo_offset + i, inner.flags[i])?;
                //通过 PageTableTrait 的 hal_pt_map 进行页表映射
                page_table
                    .map(inner.addr + i * PAGE_SIZE, paddr, inner.flags[i])
                    .expect("failed to map");
            }
            Ok(())
        })
    }

    fn unmap(&self) {
        let inner = self.inner.lock();
        let pages = inner.size / PAGE_SIZE;
        // TODO inner.vmo_offset unused?
        self.page_table
            .lock()
            .unmap_cont(inner.addr, pages)
            .expect("failed to unmap")
    }
}
```