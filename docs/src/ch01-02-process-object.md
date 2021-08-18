#### 对象管理器：Process 对象

## 句柄——操作内核对象的桥梁

在1.1中我们用Rust语言实现了一个最核心的内核对象，在本小节我们将逐步了解与内核对象相关的三个重要概念中的其他两个：**句柄（Handle）和权限（Rights）**。

句柄是允许用户程序引用内核对象引用的一种内核结构，它可以被认为是与特定内核对象的会话或连接。

通常情况下，多个进程通过不同的句柄同时访问同一个对象。对象可能有多个句柄（在一个或多个进程中）引用它们。但单个句柄只能绑定到单个进程或绑定到内核。

### 定义句柄

在 object 模块下定义一个子模块：

```rust
// src/object/mod.rs
mod handle;

pub use self::handle::*;
```

定义句柄：

```rust
// src/object/handle.rs
use super::{KernelObject, Rights};
use alloc::sync::Arc;

/// 内核对象句柄
#[derive(Clone)]
pub struct Handle {
    pub object: Arc<dyn KernelObject>,
    pub rights: Rights,
}
```

一个Handle包含object和right两个字段，object是实现了`KernelObject`Trait的内核对象，Rights是该句柄的权限，我们将在下面提到它。

Arc<T>是一个可以在多线程上使用的引用计数类型，这个计数会随着 `Arc<T>` 的创建或复制而增加，并当 `Arc<T>` 生命周期结束被回收时减少。当这个计数变为零之后，这个计数变量本身以及被引用的变量都会从堆上被回收。

我们为什么要在这里使用Arc智能指针呢？

绝大多数内核对象的析构都发生在句柄数量为 0 时，也就是最后一个指向内核对象的Handle被关闭，该对象也随之消亡，抑或进入一种无法撤销的最终状态。很明显，这与Arc<T>天然的契合。

## 控制句柄的权限——Rights

上文的Handle中有一个字段是rights，也就是句柄的权限。顾名思义，权限规定该句柄对引用的对象可以进行何种操作。

当不同的权限和同一个对象绑定在一起时，也就形成了不同的句柄。

### 定义权限

在 object 模块下定义一个子模块：

````
// src/object/mod.rs
mod rights;

pub use self::rights::*;
````

权限就是u32的一个数字


```
// src/object/rights.rs
use bitflags::bitflags;

bitflags! {
    /// 句柄权限
    pub struct Rights: u32 {
        const DUPLICATE = 1 << 0;
        const TRANSFER = 1 << 1;
        const READ = 1 << 2;
        const WRITE = 1 << 3;
        const EXECUTE = 1 << 4;
		...
    }

```

[**bitflags**](https://docs.rs/bitflags/1.2.1/bitflags/) 是一个 Rust 中常用来比特标志位的 crate 。它提供了 一个 `bitflags!` 宏，如上面的代码段所展示的那样，借助 `bitflags!` 宏我们将一个 `u32` 的 rights 包装为一个 `Rights` 结构体。注意，在使用之前我们需要引入该 crate 的依赖：

```
# Cargo.toml

[dependencies]
bitflags = "1.2"
```

定义好权限之后，我们回到句柄相关方法的实现。

首先是最简单的部分，创建一个handle，很显然我们需要提供两个参数，分别是句柄关联的内核对象和句柄的权限。

```
impl Handle {
    /// 创建一个新句柄
    pub fn new(object: Arc<dyn KernelObject>, rights: Rights) -> Self {
        Handle { object, rights }
    }
}
```

### 测试

好啦，让我们来测试一下！

```
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
```

## 句柄存储的载体——Process

实现完了句柄之后，我们开始考虑，句柄是存储在哪里的呢？

通过前面的讲解，很明显Process拥有内核对象句柄，也就是说，句柄存储在Process中，所以我们先来实现一个Process：

### 实现空的process对象

```
// src/task/process.rs
/// 进程对象
pub struct Process {
    base: KObjectBase,
    inner: Mutex<ProcessInner>,
}
// 宏的作用：补充
impl_kobject!(Process);

struct ProcessInner {
    handles: BTreeMap<HandleValue, Handle>,
}

pub type HandleValue = u32;
```

handles使用BTreeMap存储的key是HandleValue，value就是句柄。通过HandleValue实现对句柄的增删操作。HandleValue实际上就是u32类型是别名。

把内部对象ProcessInner用自旋锁Mutex包起来，保证了互斥访问，因为Mutex会帮我们处理好并发问题，这一点已经在1.1节中详细说明。

接下来我们实现创建一个Process的方法：

```
impl Process {
    /// 创建一个新的进程对象
    pub fn new() -> Arc<Self> {
        Arc::new(Process {
            base: KObjectBase::default(),
            inner: Mutex::new(ProcessInner {
                handles: BTreeMap::default(),
            }),
        })
    }
}
```

#### 单元测试

我们已经实现了创建一个Process的方法，下面我们写一个单元测试：

```
#[test]
    fn new_proc() {
        let proc = Process::new();
        assert_eq!(proc.type_name(), "Process");
        assert_eq!(proc.name(), "");
        proc.set_name("proc1");
        assert_eq!(proc.name(), "proc1");
        assert_eq!(
            format!("{:?}", proc),
            format!("Process({}, \"proc1\")", proc.id())
        );

        let obj: Arc<dyn KernelObject> = proc;
        assert_eq!(obj.type_name(), "Process");
        assert_eq!(obj.name(), "proc1");
        obj.set_name("proc2");
        assert_eq!(obj.name(), "proc2");
        assert_eq!(
            format!("{:?}", obj),
            format!("Process({}, \"proc2\")", obj.id())
        );
    }
```

### Process相关方法

#### 插入句柄

在Process中添加一个新的handle，返回值是一个handleValue，也就是u32：

```
pub fn add_handle(&self, handle: Handle) -> HandleValue {

    let mut inner = self.inner.lock();
    let value = (0 as HandleValue..)
    	.find(|idx| !inner.handles.contains_key(idx))
    	.unwrap();
    // 插入BTreeMap
    inner.handles.insert(value, handle);
    value
 }
```

#### 移除句柄

删除Process中的一个句柄：

```
pub fn remove_handle(&self, handle_value: HandleValue) {
	self.inner.lock().handles.remove(&handle_value);
}
```

#### 根据句柄查找内核对象

```
// src/task/process.rs
impl Process {
    /// 根据句柄值查找内核对象，并检查权限
    pub fn get_object_with_rights<T: KernelObject>(
        &self,
        handle_value: HandleValue,
        desired_rights: Rights,
    ) -> ZxResult<Arc<T>> {
        let handle = self
            .inner
            .lock()
            .handles
            .get(&handle_value)
            .ok_or(ZxError::BAD_HANDLE)?
            .clone();
        // check type before rights
        let object = handle
            .object
            .downcast_arc::<T>()
            .map_err(|_| ZxError::WRONG_TYPE)?;
        if !handle.rights.contains(desired_rights) {
            return Err(ZxError::ACCESS_DENIED);
        }
        Ok(object)
    }
}
```

#### ZxResult

ZxResult是表示Zircon状态的i32值，值空间划分如下：

- 0:ok
- 负值：由系统定义（也就是这个文件）
- 正值：被保留，用于协议特定的错误值，永远不会被系统定义。

```
pub type ZxResult<T> = Result<T, ZxError>;

#[allow(non_camel_case_types, dead_code)]
#[repr(i32)]
#[derive(Debug, Clone, Copy)]
pub enum ZxError {
    OK = 0,
   	...
   	
    /// 一个不指向handle的特定的handle value
    BAD_HANDLE = -11,
    
    /// 操作主体对于执行这个操作来说是错误的类型
    /// 例如： 尝试执行 message_read 在 thread handle.
    WRONG_TYPE = -12,
    
    // 权限检查错误
    // 调用者没有执行该操作的权限
    ACCESS_DENIED = -30,
}
```

ZxResult<T>相当于Result<T, ZxError>，也就相当于我们自己定义了一种错误。

### 单元测试

目前为止，我们已经实现了Process最基础的方法，下面我们来运行一个单元测试：

```
fn proc_handle() {
        let proc = Process::new();
        let handle = Handle::new(proc.clone(), Rights::DEFAULT_PROCESS);
        let handle_value = proc.add_handle(handle);

        let object1: Arc<Process> = proc
            .get_object_with_rights(handle_value, Rights::DEFAULT_PROCESS)
            .expect("failed to get object");
        assert!(Arc::ptr_eq(&object1, &proc));

        proc.remove_handle(handle_value);
    }
```

## 总结

在这一节中我们实现了内核对象的两个重要的概念，句柄（Handle）和权限（Rights），同时实现了句柄存储的载体——Process，并且实现了Process的基本方法，这将是我们继续探索zCore的基础。

在下一节中，我们将介绍内核对象的传输器——管道（Channel）。
