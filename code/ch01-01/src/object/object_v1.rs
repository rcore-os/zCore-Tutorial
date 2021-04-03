#![allow(dead_code)]
use super::{KernelObject, KoID, String};

// ANCHOR: dummy_def
use spin::Mutex;

/// 空对象
#[derive(Debug)]
pub struct DummyObject {
    id: KoID,
    inner: Mutex<DummyObjectInner>,
}

/// `DummyObject` 的内部可变部分
#[derive(Default, Debug)]
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
        assert_ne!(o1.id(), o2.id());
        assert_eq!(o1.type_name(), "DummyObject");
        assert_eq!(o1.name(), "");
        o1.set_name("object1");
        assert_eq!(o1.name(), "object1");
    }
}
// ANCHOR_END: dummy_test

#[cfg(test)]
// ANCHOR: downcast_test
#[test]
fn downcast() {
    let dummy = DummyObject::new();
    let object: Arc<dyn KernelObject> = dummy;
    let _result: Arc<DummyObject> = object.downcast_arc::<DummyObject>().unwrap();
}
// ANCHOR_END: downcast_test
