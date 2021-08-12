use {
    super::*, crate::object::*, alloc::sync::Arc, alloc::vec::Vec, kernel_hal::MMUFlags,
    spin::Mutex,
};

/// Virtual Memory Address Regions
pub struct VmAddressRegion {
    base: KObjectBase,
}

impl_kobject!(VmAddressRegion);

impl VmAddressRegion {
    /// Create a new root VMAR.
    pub fn new_root() -> Arc<Self> {
        Arc::new(VmAddressRegion {
            base: KObjectBase::new(),
        })
    }
}

/// Virtual Memory Mapping
pub struct VmMapping {
    /// The permission limitation of the vmar
    permissions: MMUFlags,
    vmo: Arc<VmObject>,
    // page_table: Arc<Mutex<dyn PageTableTrait>>,
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
