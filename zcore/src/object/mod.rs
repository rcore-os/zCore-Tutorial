use alloc::string::String;
// use core::fmt::Debug;
// use downcast_rs::{impl_downcast, DowncastSync};

// ANCHOR: object
/// 内核对象公共接口
pub trait KernelObject: Send + Sync {
    /// 获取对象 ID
    fn id(&self) -> KoID;
    /// 获取对象类型名
    fn type_name(&self) -> &str;
    /// 获取对象名称
    fn name(&self) -> String;
    /// 设置对象名称
    fn set_name(&self, name: &str);
}
// ANCHOR_END: object

// ANCHOR: koid
/// 对象 ID 类型
pub type KoID = u64;
// ANCHOR_END: koid

// ANCHOR: dummy_def
use spin::Mutex;

/// 空对象
pub struct DummyObject {
    id: KoID,
    inner: Mutex<DummyObjectInner>,
}

/// `DummyObject` 的内部可变部分
#[derive(Default)]
struct DummyObjectInner {
    name: String,
}
// ANCHOR_END: dummy_def

// ANCHOR: dummy_new
use alloc::sync::Arc;
use core::sync::atomic::*;

impl DummyObject {
    /// 创建一个新 `DummyObject`
    pub fn new() -> Arc<Self> {
        Arc::new(DummyObject {
            id: Self::new_koid(),
            inner: Default::default(),
        })
    }

    /// 生成一个唯一的 ID
    fn new_koid() -> KoID {
        static NEXT_KOID: AtomicU64 = AtomicU64::new(1024);
        NEXT_KOID.fetch_add(1, Ordering::SeqCst)
    }
}
// ANCHOR_END: dummy_new

// ANCHOR: dummy_impl
impl KernelObject for DummyObject {
    fn id(&self) -> KoID {
        self.id
    }
    fn type_name(&self) -> &str {
        "DummyObject"
    }
    fn name(&self) -> String {
        self.inner.lock().name.clone()
    }
    fn set_name(&self, name: &str) {
        self.inner.lock().name = String::from(name);
    }
}
// ANCHOR_END: dummy_impl

// ANCHOR: dummy_test
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dummy_object() {
        let o1 = DummyObject::new();
        let o2 = DummyObject::new();
        assert_eq!(o1.id(), 1024);
        assert_eq!(o2.id(), 1025);
        assert_eq!(o1.type_name(), "DummyObject");
        assert_eq!(o1.name(), "");
        o1.set_name("object1");
        assert_eq!(o1.name(), "object1");
    }
}
// ANCHOR_END: dummy_test

// ANCHOR: base
/// 内核对象核心结构
pub struct KObjectBase {
    /// 对象 ID
    pub id: KoID,
    inner: Mutex<KObjectBaseInner>,
}

/// `KObjectBase` 的内部可变部分
#[derive(Default)]
struct KObjectBaseInner {
    name: String,
}
// ANCHOR_END: base

// impl_downcast!(sync KernelObject);
