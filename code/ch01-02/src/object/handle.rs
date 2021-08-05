// ANCHOR: handle
use super::{DummyObject, KernelObject, Rights};
use alloc::sync::Arc;

/// 内核对象句柄
#[derive(Clone)]
pub struct Handle {
    pub object: Arc<dyn KernelObject>,
    pub rights: Rights,
}

impl Handle {
    /// 创建一个新句柄
    pub fn new(object: Arc<dyn KernelObject>, rights: Rights) -> Self {
        Handle { object, rights }
    }
}
// ANCHOR_END: handle

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::DummyObject;
    
    #[test]
    fn new_obj_handle() {
        let obj = DummyObject::new();
        let handle1 = Handle::new(obj.clone(), Rights::BASIC);
    }
}
