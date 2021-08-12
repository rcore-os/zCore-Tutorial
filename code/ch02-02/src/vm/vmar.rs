use {super::*, crate::object::*, alloc::sync::Arc};

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
