// ANCHOR: handle
use super::{KernelObject, Rights};
use alloc::sync::Arc;

pub type HandleValue = u32;

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
