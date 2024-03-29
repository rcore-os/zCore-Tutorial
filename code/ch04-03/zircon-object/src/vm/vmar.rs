use {
    super::*,
    crate::object::*,
    alloc::sync::Arc,
    alloc::vec,
    alloc::vec::Vec,
    bitflags::bitflags,
    kernel_hal::{MMUFlags, PageTableTrait},
    spin::Mutex,
};

bitflags! {
    /// Creation flags for VmAddressRegion.
    pub struct VmarFlags: u32 {
        #[allow(clippy::identity_op)]
        /// When randomly allocating subregions, reduce sprawl by placing allocations
        /// near each other.
        const COMPACT               = 1 << 0;
        /// Request that the new region be at the specified offset in its parent region.
        const SPECIFIC              = 1 << 1;
        /// Like SPECIFIC, but permits overwriting existing mappings.  This
        /// flag will not overwrite through a subregion.
        const SPECIFIC_OVERWRITE    = 1 << 2;
        /// Allow VmMappings to be created inside the new region with the SPECIFIC or
        /// OFFSET_IS_UPPER_LIMIT flag.
        const CAN_MAP_SPECIFIC      = 1 << 3;
        /// Allow VmMappings to be created inside the region with read permissions.
        const CAN_MAP_READ          = 1 << 4;
        /// Allow VmMappings to be created inside the region with write permissions.
        const CAN_MAP_WRITE         = 1 << 5;
        /// Allow VmMappings to be created inside the region with execute permissions.
        const CAN_MAP_EXECUTE       = 1 << 6;
        /// Require that VMO backing the mapping is non-resizable.
        const REQUIRE_NON_RESIZABLE = 1 << 7;
        /// Treat the offset as an upper limit when allocating a VMO or child VMAR.
        const ALLOW_FAULTS          = 1 << 8;

        /// Allow VmMappings to be created inside the region with read, write and execute permissions.
        const CAN_MAP_RXW           = Self::CAN_MAP_READ.bits | Self::CAN_MAP_EXECUTE.bits | Self::CAN_MAP_WRITE.bits;
        /// Creation flags for root VmAddressRegion
        const ROOT_FLAGS            = Self::CAN_MAP_RXW.bits | Self::CAN_MAP_SPECIFIC.bits;
    }
}

/// Virtual Memory Address Regions
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

impl_kobject!(VmAddressRegion);

/// The mutable part of `VmAddressRegion`.
#[derive(Default)]
struct VmarInner {
    children: Vec<Arc<VmAddressRegion>>,
    mappings: Vec<Arc<VmMapping>>,
}

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

    /// Create a child VMAR at the `offset`.
    pub fn allocate_at(
        self: &Arc<Self>,
        offset: usize,
        len: usize,
        flags: VmarFlags,
        align: usize,
    ) -> ZxResult<Arc<Self>> {
        self.allocate(Some(offset), len, flags, align)
    }

    /// Create a child VMAR with optional `offset`.
    pub fn allocate(
        self: &Arc<Self>,
        offset: Option<usize>,
        len: usize,
        flags: VmarFlags,
        align: usize,
    ) -> ZxResult<Arc<Self>> {
        let mut guard = self.inner.lock();
        let inner = guard.as_mut().ok_or(ZxError::BAD_STATE)?;
        let offset = self.determine_offset(inner, offset, len, align)?;
        let child = Arc::new(VmAddressRegion {
            flags,
            base: KObjectBase::new(),
            addr: self.addr + offset,
            size: len,
            parent: Some(self.clone()),
            page_table: self.page_table.clone(),
            inner: Mutex::new(Some(VmarInner::default())),
        });
        inner.children.push(child.clone());
        Ok(child)
    }

    /// Map the `vmo` into this VMAR at given `offset`.
    pub fn map_at(
        &self,
        vmar_offset: usize,
        vmo: Arc<VmObject>,
        vmo_offset: usize,
        len: usize,
        flags: MMUFlags,
    ) -> ZxResult<VirtAddr> {
        self.map(Some(vmar_offset), vmo, vmo_offset, len, flags)
    }

    /// Map the `vmo` into this VMAR.
    pub fn map(
        &self,
        vmar_offset: Option<usize>,
        vmo: Arc<VmObject>,
        vmo_offset: usize,
        len: usize,
        flags: MMUFlags,
    ) -> ZxResult<VirtAddr> {
        self.map_ext(
            vmar_offset,
            vmo,
            vmo_offset,
            len,
            MMUFlags::RXW,
            flags,
            false,
            true,
        )
    }

    /// Map the `vmo` into this VMAR.
    #[allow(clippy::too_many_arguments)]
    pub fn map_ext(
        &self,
        vmar_offset: Option<usize>,
        vmo: Arc<VmObject>,
        vmo_offset: usize,
        len: usize,
        permissions: MMUFlags,
        flags: MMUFlags,
        overwrite: bool,
        map_range: bool,
    ) -> ZxResult<VirtAddr> {
        if !page_aligned(vmo_offset) || !page_aligned(len) || vmo_offset.overflowing_add(len).1 {
            return Err(ZxError::INVALID_ARGS);
        }
        if !permissions.contains(flags & MMUFlags::RXW) {
            return Err(ZxError::ACCESS_DENIED);
        }
        if vmo_offset > vmo.len() || len > vmo.len() - vmo_offset {
            return Err(ZxError::INVALID_ARGS);
        }
        // Simplify: overwrite == false && map_range == true
        if overwrite || !map_range {
            warn!("Simplify: overwrite == false && map_range == true");
            return Err(ZxError::INVALID_ARGS);
        }
        let mut guard = self.inner.lock();
        let inner = guard.as_mut().ok_or(ZxError::BAD_STATE)?;
        let offset = self.determine_offset(inner, vmar_offset, len, PAGE_SIZE)?;
        let addr = self.addr + offset;
        let flags = flags | MMUFlags::from_bits_truncate(vmo.cache_policy() as u32 as usize);
        // align = 1K? 2K? 4K? 8K? ...
        if !self.test_map(inner, offset, len, PAGE_SIZE) {
            return Err(ZxError::NO_MEMORY);
        }
        let mapping = VmMapping::new(
            addr,
            len,
            vmo,
            vmo_offset,
            permissions,
            flags,
            self.page_table.clone(),
        );
        mapping.map()?;
        inner.mappings.push(mapping);
        Ok(addr)
    }

    /// Unmaps all VMO mappings and destroys all sub-regions within the absolute range
    /// including `addr` and ending before exclusively at `addr + len`.
    /// Any sub-region that is in the range must be fully in the range
    /// (i.e. partial overlaps are an error).
    /// NOT SUPPORT:
    /// If a mapping is only partially in the range, the mapping is split and the requested
    /// portion is unmapped.
    pub fn unmap(&self, addr: VirtAddr, len: usize) -> ZxResult {
        if !page_aligned(addr) || !page_aligned(len) || len == 0 {
            return Err(ZxError::INVALID_ARGS);
        }
        let mut guard = self.inner.lock();
        let inner = guard.as_mut().ok_or(ZxError::BAD_STATE)?;
        let begin = addr;
        let end = addr + len;
        // check partial overlapped sub-regions
        if inner
            .children
            .iter()
            .any(|vmar| vmar.partial_overlap(begin, end))
        {
            return Err(ZxError::INVALID_ARGS);
        }

        if inner
            .mappings
            .iter()
            .any(|map| map.partial_overlap(begin, end))
        {
            warn!("Simplify: Not support partial unmap.");
            return Err(ZxError::INVALID_ARGS);
        }
        inner.mappings.drain_filter(|map| map.within(begin, end));

        for vmar in inner.children.drain_filter(|vmar| vmar.within(begin, end)) {
            vmar.destroy_internal()?;
        }
        Ok(())
    }

    /// Change protections on a subset of the region of memory in the containing
    /// address space.  If the requested range overlaps with a subregion,
    /// protect() will fail.
    pub fn protect(&self, addr: usize, len: usize, flags: MMUFlags) -> ZxResult {
        if !page_aligned(addr) || !page_aligned(len) {
            return Err(ZxError::INVALID_ARGS);
        }
        let mut guard = self.inner.lock();
        let inner = guard.as_mut().ok_or(ZxError::BAD_STATE)?;
        let end_addr = addr + len;
        // check if there are overlapping subregion
        if inner
            .children
            .iter()
            .any(|child| child.overlap(addr, end_addr))
        {
            return Err(ZxError::INVALID_ARGS);
        }
        let length = inner.mappings.iter().fold(0, |acc, map| {
            acc + end_addr
                .min(map.end_addr())
                .saturating_sub(addr.max(map.addr()))
        });
        if length != len {
            return Err(ZxError::NOT_FOUND);
        }
        // check if protect flags is valid
        if inner
            .mappings
            .iter()
            .filter(|map| map.overlap(addr, end_addr)) // get mappings in range: [addr, end_addr]
            .any(|map| !map.is_valid_mapping_flags(flags))
        {
            return Err(ZxError::ACCESS_DENIED);
        }
        inner
            .mappings
            .iter()
            .filter(|map| map.overlap(addr, end_addr))
            .for_each(|map| {
                let start_index = pages(addr.max(map.addr()) - map.addr());
                let end_index = pages(end_addr.min(map.end_addr()) - map.addr());
                map.protect(flags, start_index, end_index);
            });
        Ok(())
    }

    /// Unmap all mappings and destroy all sub-regions of VMAR.
    pub fn clear(&self) -> ZxResult {
        let mut guard = self.inner.lock();
        let inner = guard.as_mut().ok_or(ZxError::BAD_STATE)?;
        for vmar in inner.children.drain(..) {
            vmar.destroy_internal()?;
        }
        inner.mappings.clear();
        Ok(())
    }

    /// Destroy but do not remove self from parent.
    fn destroy_internal(&self) -> ZxResult {
        let mut guard = self.inner.lock();
        let inner = guard.as_mut().ok_or(ZxError::BAD_STATE)?;
        for vmar in inner.children.drain(..) {
            vmar.destroy_internal()?;
        }
        inner.mappings.clear();
        *guard = None;
        Ok(())
    }

    /// Unmap all mappings within the VMAR, and destroy all sub-regions of the region.
    pub fn destroy(self: &Arc<Self>) -> ZxResult {
        self.destroy_internal()?;
        // remove from parent
        if let Some(parent) = &self.parent {
            let mut guard = parent.inner.lock();
            let inner = guard.as_mut().ok_or(ZxError::BAD_STATE)?;
            inner.children.retain(|vmar| !Arc::ptr_eq(self, vmar));
        }
        Ok(())
    }

    /// Get physical address of the underlying page table.
    pub fn table_phys(&self) -> PhysAddr {
        self.page_table.lock().table_phys()
    }

    /// Get start address of this VMAR.
    pub fn addr(&self) -> usize {
        self.addr
    }

    /// Whether this VMAR is dead.
    pub fn is_dead(&self) -> bool {
        self.inner.lock().is_none()
    }

    /// Whether this VMAR is alive.
    pub fn is_alive(&self) -> bool {
        !self.is_dead()
    }

    /// Determine final address with given input `offset` and `len`.
    fn determine_offset(
        &self,
        inner: &VmarInner,
        offset: Option<usize>,
        len: usize,
        align: usize,
    ) -> ZxResult<VirtAddr> {
        if !check_aligned(len, align) {
            Err(ZxError::INVALID_ARGS)
        } else if let Some(offset) = offset {
            if check_aligned(offset, align) && self.test_map(inner, offset, len, align) {
                Ok(offset)
            } else {
                Err(ZxError::INVALID_ARGS)
            }
        } else if len > self.size {
            Err(ZxError::INVALID_ARGS)
        } else {
            match self.find_free_area(inner, 0, len, align) {
                Some(offset) => Ok(offset),
                None => Err(ZxError::NO_MEMORY),
            }
        }
    }

    /// Test if can create a new mapping at `offset` with `len`.
    fn test_map(&self, inner: &VmarInner, offset: usize, len: usize, align: usize) -> bool {
        debug_assert!(check_aligned(offset, align));
        debug_assert!(check_aligned(len, align));
        let begin = self.addr + offset;
        let end = begin + len;
        if end > self.addr + self.size {
            return false;
        }
        // brute force
        if inner.children.iter().any(|vmar| vmar.overlap(begin, end)) {
            return false;
        }
        if inner.mappings.iter().any(|map| map.overlap(begin, end)) {
            return false;
        }
        true
    }

    /// Find a free area with `len`.
    fn find_free_area(
        &self,
        inner: &VmarInner,
        offset_hint: usize,
        len: usize,
        align: usize,
    ) -> Option<usize> {
        // TODO: randomize
        debug_assert!(check_aligned(offset_hint, align));
        debug_assert!(check_aligned(len, align));
        // brute force:
        // try each area's end address as the start
        core::iter::once(offset_hint)
            .chain(inner.children.iter().map(|map| map.end_addr() - self.addr))
            .chain(inner.mappings.iter().map(|map| map.end_addr() - self.addr))
            .find(|&offset| self.test_map(inner, offset, len, align))
    }

    fn end_addr(&self) -> VirtAddr {
        self.addr + self.size
    }

    fn overlap(&self, begin: VirtAddr, end: VirtAddr) -> bool {
        !(self.addr >= end || self.end_addr() <= begin)
    }

    fn within(&self, begin: VirtAddr, end: VirtAddr) -> bool {
        begin <= self.addr && self.end_addr() <= end
    }

    fn partial_overlap(&self, begin: VirtAddr, end: VirtAddr) -> bool {
        self.overlap(begin, end) && !self.within(begin, end)
    }

    fn contains(&self, vaddr: VirtAddr) -> bool {
        self.addr <= vaddr && vaddr < self.end_addr()
    }

    /// Get information of this VmAddressRegion
    pub fn get_info(&self) -> VmarInfo {
        // pub fn get_info(&self, va: usize) -> VmarInfo {
        // let _r = self.page_table.lock().query(va);
        VmarInfo {
            base: self.addr(),
            len: self.size,
        }
    }

    /// Get VmarFlags of this VMAR.
    pub fn get_flags(&self) -> VmarFlags {
        self.flags
    }

    #[cfg(test)]
    fn count(&self) -> usize {
        let mut guard = self.inner.lock();
        let inner = guard.as_mut().unwrap();
        println!("m = {}, c = {}", inner.mappings.len(), inner.children.len());
        inner.mappings.len() + inner.children.len()
    }

    #[cfg(test)]
    fn used_size(&self) -> usize {
        let mut guard = self.inner.lock();
        let inner = guard.as_mut().unwrap();
        let map_size: usize = inner.mappings.iter().map(|map| map.size()).sum();
        let vmar_size: usize = inner.children.iter().map(|vmar| vmar.size).sum();
        println!("size = {:#x?}", map_size + vmar_size);
        map_size + vmar_size
    }
}

/// Information of a VmAddressRegion.
#[repr(C)]
#[derive(Debug)]
pub struct VmarInfo {
    base: usize,
    len: usize,
}

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

impl core::fmt::Debug for VmMapping {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let inner = self.inner.lock();
        f.debug_struct("VmMapping")
            .field("addr", &inner.addr)
            .field("size", &inner.size)
            .field("permissions", &self.permissions)
            .field("flags", &inner.flags)
            .field("vmo_id", &self.vmo.id())
            .field("vmo_offset", &inner.vmo_offset)
            .finish()
    }
}

impl VmMapping {
    fn new(
        addr: VirtAddr,
        size: usize,
        vmo: Arc<VmObject>,
        vmo_offset: usize,
        permissions: MMUFlags,
        flags: MMUFlags,
        page_table: Arc<Mutex<dyn PageTableTrait>>,
    ) -> Arc<Self> {
        let mapping = Arc::new(VmMapping {
            inner: Mutex::new(VmMappingInner {
                flags: vec![flags; pages(size)],
                addr,
                size,
                vmo_offset,
            }),
            permissions,
            page_table,
            vmo: vmo.clone(),
        });
        vmo.append_mapping(Arc::downgrade(&mapping));
        mapping
    }

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

    fn overlap(&self, begin: VirtAddr, end: VirtAddr) -> bool {
        let inner = self.inner.lock();
        !(inner.addr >= end || inner.end_addr() <= begin)
    }

    fn within(&self, begin: VirtAddr, end: VirtAddr) -> bool {
        let inner = self.inner.lock();
        begin <= inner.addr && inner.end_addr() <= end
    }

    fn partial_overlap(&self, begin: VirtAddr, end: VirtAddr) -> bool {
        self.overlap(begin, end) && !self.within(begin, end)
    }

    fn contains(&self, vaddr: VirtAddr) -> bool {
        let inner = self.inner.lock();
        inner.addr <= vaddr && vaddr < inner.end_addr()
    }

    fn is_valid_mapping_flags(&self, flags: MMUFlags) -> bool {
        self.permissions.contains(flags & MMUFlags::RXW)
    }

    fn protect(&self, flags: MMUFlags, start_index: usize, end_index: usize) {
        let mut inner = self.inner.lock();
        let mut pg_table = self.page_table.lock();
        for i in start_index..end_index {
            inner.flags[i] = (inner.flags[i] & !MMUFlags::RXW) | (flags & MMUFlags::RXW);
            pg_table
                .protect(inner.addr + i * PAGE_SIZE, inner.flags[i])
                .unwrap();
        }
    }

    fn size(&self) -> usize {
        self.inner.lock().size
    }

    fn addr(&self) -> VirtAddr {
        self.inner.lock().addr
    }

    fn end_addr(&self) -> VirtAddr {
        self.inner.lock().end_addr()
    }

    /// Get MMUFlags of this VmMapping.
    pub fn get_flags(&self, vaddr: usize) -> ZxResult<MMUFlags> {
        if self.contains(vaddr) {
            let page_id = (vaddr - self.addr()) / PAGE_SIZE;
            Ok(self.inner.lock().flags[page_id])
        } else {
            Err(ZxError::NO_MEMORY)
        }
    }
}

impl VmMappingInner {
    fn end_addr(&self) -> VirtAddr {
        self.addr + self.size
    }
}

impl Drop for VmMapping {
    fn drop(&mut self) {
        self.unmap();
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_child() {
        let root_vmar = VmAddressRegion::new_root();
        let child = root_vmar
            .allocate_at(0, 0x2000, VmarFlags::CAN_MAP_RXW, PAGE_SIZE)
            .expect("failed to create child VMAR");

        // test invalid argument
        assert_eq!(
            root_vmar
                .allocate_at(0x2001, 0x1000, VmarFlags::CAN_MAP_RXW, PAGE_SIZE)
                .err(),
            Some(ZxError::INVALID_ARGS)
        );
        assert_eq!(
            root_vmar
                .allocate_at(0x2000, 1, VmarFlags::CAN_MAP_RXW, PAGE_SIZE)
                .err(),
            Some(ZxError::INVALID_ARGS)
        );
        assert_eq!(
            root_vmar
                .allocate_at(0, 0x1000, VmarFlags::CAN_MAP_RXW, PAGE_SIZE)
                .err(),
            Some(ZxError::INVALID_ARGS)
        );
        assert_eq!(
            child
                .allocate_at(0x1000, 0x2000, VmarFlags::CAN_MAP_RXW, PAGE_SIZE)
                .err(),
            Some(ZxError::INVALID_ARGS)
        );
    }

    /// A valid virtual address base to mmap.
    const MAGIC: usize = 0xdead_beaf;

    #[test]
    #[allow(unsafe_code)]
    fn map() {
        let vmar = VmAddressRegion::new_root();
        let vmo = VmObject::new_paged(4);
        let flags = MMUFlags::READ | MMUFlags::WRITE;

        // invalid argument
        assert_eq!(
            vmar.map_at(0, vmo.clone(), 0x4000, 0x1000, flags),
            Err(ZxError::INVALID_ARGS)
        );
        assert_eq!(
            vmar.map_at(0, vmo.clone(), 0, 0x5000, flags),
            Err(ZxError::INVALID_ARGS)
        );
        assert_eq!(
            vmar.map_at(0, vmo.clone(), 0x1000, 1, flags),
            Err(ZxError::INVALID_ARGS)
        );
        assert_eq!(
            vmar.map_at(0, vmo.clone(), 1, 0x1000, flags),
            Err(ZxError::INVALID_ARGS)
        );
        vmar.map_at(0, vmo.clone(), 0, 0x4000, flags).unwrap();
        vmar.map_at(0x12000, vmo.clone(), 0x2000, 0x1000, flags)
            .unwrap();
        unsafe {
            ((vmar.addr() + 0x2000) as *mut usize).write(MAGIC);
            assert_eq!(((vmar.addr() + 0x12000) as *const usize).read(), MAGIC);
        }
    }

    /// ```text
    /// +--------+--------+--------+--------+
    /// |           root              ....  |
    /// +--------+--------+--------+--------+
    /// |      child1     | child2 |
    /// +--------+--------+--------+
    /// | g-son1 | g-son2 |
    /// +--------+--------+
    /// ```
    struct Sample {
        root: Arc<VmAddressRegion>,
        child1: Arc<VmAddressRegion>,
        child2: Arc<VmAddressRegion>,
        grandson1: Arc<VmAddressRegion>,
        grandson2: Arc<VmAddressRegion>,
    }

    impl Sample {
        fn new() -> Self {
            let root = VmAddressRegion::new_root();
            let child1 = root
                .allocate_at(0, 0x2000, VmarFlags::CAN_MAP_RXW, PAGE_SIZE)
                .unwrap();
            let child2 = root
                .allocate_at(0x2000, 0x1000, VmarFlags::CAN_MAP_RXW, PAGE_SIZE)
                .unwrap();
            let grandson1 = child1
                .allocate_at(0, 0x1000, VmarFlags::CAN_MAP_RXW, PAGE_SIZE)
                .unwrap();
            let grandson2 = child1
                .allocate_at(0x1000, 0x1000, VmarFlags::CAN_MAP_RXW, PAGE_SIZE)
                .unwrap();
            Sample {
                root,
                child1,
                child2,
                grandson1,
                grandson2,
            }
        }
    }

    #[test]
    fn unmap_vmar() {
        let s = Sample::new();
        let base = s.root.addr();
        s.child1.unmap(base, 0x1000).unwrap();
        assert!(s.grandson1.is_dead());
        assert!(s.grandson2.is_alive());

        // partial overlap sub-region should fail.
        let s = Sample::new();
        let base = s.root.addr();
        assert_eq!(
            s.root.unmap(base + 0x1000, 0x2000),
            Err(ZxError::INVALID_ARGS)
        );

        // unmap nothing should success.
        let s = Sample::new();
        let base = s.root.addr();
        s.child1.unmap(base + 0x8000, 0x1000).unwrap();
    }

    #[test]
    fn destroy() {
        let s = Sample::new();
        s.child1.destroy().unwrap();
        assert!(s.child1.is_dead());
        assert!(s.grandson1.is_dead());
        assert!(s.grandson2.is_dead());
        assert!(s.child2.is_alive());
        // address space should be released
        assert!(s
            .root
            .allocate_at(0, 0x1000, VmarFlags::CAN_MAP_RXW, PAGE_SIZE)
            .is_ok());
    }

    #[test]
    fn unmap_mapping() {
        //   +--------+--------+--------+--------+--------+
        // 1 [--------------------------|xxxxxxxx|--------]
        // 2 [xxxxxxxx|-----------------]
        // 3          [--------|xxxxxxxx]
        // 4          [xxxxxxxx]
        let vmar = VmAddressRegion::new_root();
        let base = vmar.addr();
        let vmo = VmObject::new_paged(5);
        let flags = MMUFlags::READ | MMUFlags::WRITE;
        vmar.map_at(0, vmo, 0, 0x5000, flags).unwrap();
        assert_eq!(vmar.count(), 1);
        assert_eq!(vmar.used_size(), 0x5000);

        // 0. unmap none.
        vmar.unmap(base + 0x5000, 0x1000).unwrap();
        assert_eq!(vmar.count(), 1);
        assert_eq!(vmar.used_size(), 0x5000);

        // // 1. unmap middle.
        // vmar.unmap(base + 0x3000, 0x1000).unwrap();
        // assert_eq!(vmar.count(), 2);
        // assert_eq!(vmar.used_size(), 0x4000);

        // // 2. unmap prefix.
        // vmar.unmap(base, 0x1000).unwrap();
        // assert_eq!(vmar.count(), 2);
        // assert_eq!(vmar.used_size(), 0x3000);

        // // 3. unmap postfix.
        // vmar.unmap(base + 0x2000, 0x1000).unwrap();
        // assert_eq!(vmar.count(), 2);
        // assert_eq!(vmar.used_size(), 0x2000);

        // 4. unmap all.
        vmar.unmap(base, 0x5000).unwrap();
        assert_eq!(vmar.count(), 0);
        assert_eq!(vmar.used_size(), 0x0);
    }
}
