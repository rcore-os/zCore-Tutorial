use crate::error::*;
use alloc::string::String;
use alloc::sync::Arc;
use core::fmt::Debug;
use core::sync::atomic::*;
use downcast_rs::{impl_downcast, DowncastSync};
use spin::Mutex;

mod handle;
mod rights;

pub use self::handle::*;
pub use self::rights::*;

// ANCHOR: trait
/// 内核对象公共接口
pub trait KernelObject: DowncastSync + Debug {
    /// 获取对象 ID
    fn id(&self) -> KoID;
    /// 获取对象类型名
    fn type_name(&self) -> &str;
    /// 获取对象名称
    fn name(&self) -> String;
    /// 设置对象名称
    fn set_name(&self, name: &str);
    /// 尝试获取对象伙伴
    ///
    /// 当前该对象必须是 `Channel`
    fn peer(&self) -> ZxResult<Arc<dyn KernelObject>> {
        Err(ZxError::NOT_SUPPORTED)
    }
    /// 尝试获取关联对象 id，否则返回 0
    ///
    /// 当前该对象必须是 `Channel`
    fn related_koid(&self) -> KoID {
        0
    }
}
// ANCHOR_END: trait

impl_downcast!(sync KernelObject);

/// 对象 ID 类型
pub type KoID = u64;

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

impl Default for KObjectBase {
    /// 创建一个新 `KObjectBase`
    fn default() -> Self {
        KObjectBase {
            id: Self::new_koid(),
            inner: Default::default(),
        }
    }
}

impl KObjectBase {
    /// 生成一个唯一的 ID
    fn new_koid() -> KoID {
        static NEXT_KOID: AtomicU64 = AtomicU64::new(1024);
        NEXT_KOID.fetch_add(1, Ordering::SeqCst)
    }
    /// 获取对象名称
    pub fn name(&self) -> String {
        self.inner.lock().name.clone()
    }
    /// 设置对象名称
    pub fn set_name(&self, name: &str) {
        self.inner.lock().name = String::from(name);
    }
}

/// 为内核对象 struct 自动实现 `KernelObject` trait 的宏。
#[macro_export] // 导出宏，可在 crate 外部使用
macro_rules! impl_kobject {
    // 匹配类型名，并可以提供函数覆盖默认实现
    ($class:ident $( $fn:tt )*) => {
        // 为对象实现 KernelObject trait，方法直接转发到内部 struct
        impl KernelObject for $class {
            fn id(&self) -> KoID {
                // 直接访问内部的 pub 属性
                self.base.id
            }
            fn type_name(&self) -> &str {
                // 用 stringify! 宏将输入转成字符串
                stringify!($class)
            }
            // 注意宏里面的类型要写完整路径，例如：alloc::string::String
            fn name(&self) -> alloc::string::String {
                self.base.name()
            }
            fn set_name(&self, name: &str){
                // 直接访问内部的 pub 方法
                self.base.set_name(name)
            }
            // 可以传入任意数量的函数，覆盖 trait 的默认实现
            $( $fn )*
        }
        // 为对象实现 Debug trait
        impl core::fmt::Debug for $class {
            fn fmt(
                &self,
                f: &mut core::fmt::Formatter<'_>,
            ) -> core::result::Result<(), core::fmt::Error> {
                // 输出对象类型、ID 和名称
                f.debug_tuple(&stringify!($class))
                    .field(&self.id())
                    .field(&self.name())
                    .finish()
            }
        }
    };
}

/// 空对象
pub struct DummyObject {
    // 其中必须包含一个名为 `base` 的 `KObjectBase`
    base: KObjectBase,
}

// 使用刚才的宏，声明其为内核对象，自动生成必要的代码
impl_kobject!(DummyObject);

impl DummyObject {
    /// 创建一个新 `DummyObject`
    pub fn new() -> Arc<Self> {
        Arc::new(DummyObject {
            base: KObjectBase::default(),
        })
    }
}

#[cfg(test)]
#[test]
fn impl_kobject() {
    use alloc::format;
    let dummy = DummyObject::new();
    let object: Arc<dyn KernelObject> = dummy;
    assert_eq!(object.type_name(), "DummyObject");
    assert_eq!(object.name(), "");
    object.set_name("dummy");
    assert_eq!(object.name(), "dummy");
    assert_eq!(
        format!("{:?}", object),
        format!("DummyObject({}, \"dummy\")", object.id())
    );
    let _result: Arc<DummyObject> = object.downcast_arc::<DummyObject>().unwrap();
}
