# 对象传送器：Channel 对象

## 概要

通道（Channel）是由一定数量的字节数据和一定数量的句柄组成的双向消息传输。

## 用于IPC的内核对象

Zircon中用于IPC的内核对象主要有Channel、Socket和FIFO。这里我们主要介绍一下前两个。

> **进程间通信**（**IPC**，*Inter-Process Communication*），指至少两个进程或线程间传送数据或信号的一些技术或方法。进程是计算机系统分配资源的最小单位(进程是分配资源最小的单位，而线程是调度的最小单位，线程共用进程资源)。每个进程都有自己的一部分独立的系统资源，彼此是隔离的。为了能使不同的进程互相访问资源并进行协调工作，才有了进程间通信。举一个典型的例子，使用进程间通信的两个应用可以被分类为客户端和服务器，客户端进程请求数据，服务端回复客户端的数据请求。有一些应用本身既是服务器又是客户端，这在分布式计算中，时常可以见到。这些进程可以运行在同一计算机上或网络连接的不同计算机上。

`Socket`和`Channel`都是双向和双端的IPC相关的`Object`。创建`Socket`或`Channel`将返回两个不同的`Handle`，分别指向`Socket`或`Channel`的两端。与channel的不同之处在于，socket仅能传输数据（而不移动句柄），而channel可以传递句柄。

- `Socket`是面向流的对象，可以通过它读取或写入以一个或多个字节为单位的数据。
-  `Channel`是面向数据包的对象，并限制消息的大小最多为64K（如果有改变，可能会更小），以及最多1024个`Handle`挂载到同一消息上（如果有改变，同样可能会更小）。

当`Handle`被写入到`Channel`中时，在发送端`Process`中将会移除这些`Handle`。同时携带`Handle`的消息从`Channel`中被读取时，该`Handle`也将被加入到接收端`Process`中。在这两个时间点之间时，`Handle`将同时存在于两端（以保证它们指向的`Object`继续存在而不被销毁），除非`Channel`写入方向一端被关闭，这种情况下，指向该端点的正在发送的消息将被丢弃，并且它们包含的任何句柄都将被关闭。

## Channel

Channel是唯一一个能传递handle的IPC，其他只能传递消息。通道有两个端点`endpoints`，对于代码实现来说，**通道是虚拟的，我们实际上是用通道的两个端点来描述一个通道**。两个端点各自要维护一个消息队列，在一个端点写消息，实际上是把消息写入**另一个端点**的消息队列队尾；在一个端点读消息，实际上是从**当前端点**的消息队列的队头读出一个消息。

消息通常含有`data`和`handles`两部分，我们这里将消息封装为`MessagePacket`结构体，结构体中含有上述两个字段：

```
#[derive(Default)]
pub struct MessagePacket {
    /// message packet携带的数据data
    pub data: Vec<u8>,
    /// message packet携带的句柄Handle
    pub handles: Vec<Handle>,
}
```

### 实现空的Channel对象

在`src`目录下创建一个`ipc`目录，在`ipc`模块下定义一个子模块`channel`：

```
// src/ipc/mod.rs
use super::*;

mod channel;
pub use self::channel::*;
```

在`ipc.rs`中引入`crate`：

```
// src/ipc/channel.rs

use {
    super::*,
    crate::error::*,
    crate::object::*,
    alloc::collections::VecDeque,
    alloc::sync::{Arc, Weak},
    spin::Mutex,
};
```

把在上面提到的`MessagePacket`结构体添加到该文件中。

下面我们添加Channel结构体：

```
// src/ipc/channel.rs
pub struct Channel {
    base: KObjectBase,
    peer: Weak<Channel>,
    recv_queue: Mutex<VecDeque<T>>,
}

type T = MessagePacket;
```

`peer`代表当前端点所在管道的另一个端点，两端的结构体分别持有对方的`Weak`引用，并且两端的结构体将分别通过`Arc`引用，作为内核对象而被内核中的其他数据结构引用，这一部分我们将在创建Channel实例时提到。

`recv_queue`代表当前端点维护的消息队列，它使用`VecDeque`来存放`MessagePacket`，可以通过`pop_front()`、`push_back`等方法在队头弹出数据和在队尾压入数据。

用使用宏自动实现 `KernelObject` trait ，使用channel类型名，并添加两个函数。

```
impl_kobject!(Channel
    fn peer(&self) -> ZxResult<Arc<dyn KernelObject>> {
        let peer = self.peer.upgrade().ok_or(ZxError::PEER_CLOSED)?;
        Ok(peer)
    }
    fn related_koid(&self) -> KoID {
        self.peer.upgrade().map(|p| p.id()).unwrap_or(0)
    }
);
```

### 实现创建Channel的方法

下面我们来实现创建一个`Channel`的方法：

```
impl Channel {

    #[allow(unsafe_code)]
    pub fn create() -> (Arc<Self>, Arc<Self>) {
        let mut channel0 = Arc::new(Channel {
            base: KObjectBase::default(),
            peer: Weak::default(),
            recv_queue: Default::default(),
        });
        let channel1 = Arc::new(Channel {
            base: KObjectBase::default(),
            peer: Arc::downgrade(&channel0),
            recv_queue: Default::default(),
        });
        // no other reference of `channel0`
        unsafe {
            Arc::get_mut_unchecked(&mut channel0).peer = Arc::downgrade(&channel1);
        }
        (channel0, channel1)
}
```

该方法的返回值是两端点结构体（Channel）的`Arc`引用，这将作为内核对象被内核中的其他数据结构引用。两个端点互相持有对方`Weak`指针，这是因为一个端点无需引用计数为0，只要`strong_count`为0就可以被清理掉，即使另一个端点指向它。

> rust 语言并没有提供垃圾回收 (GC, Garbage Collection ) 的功能， 不过它提供了最简单的引用计数包装类型 `Rc`，这种引用计数功能也是早期 GC 常用的方法， 但是引用计数不能解决循环引用。那么如何 fix 这个循环引用呢？答案是 `Weak` 指针，只增加引用逻辑，不共享所有权，即不增加 strong reference count。由于 `Weak` 指针指向的对象可能析构了，所以不能直接解引用，要模式匹配，再 upgrade。

下面我们来分析一下这个`unsafe`代码块：

```
unsafe {
            Arc::get_mut_unchecked(&mut channel0).peer = Arc::downgrade(&channel1);
        }
```

由于两端的结构体将分别通过 `Arc` 引用，作为内核对象而被内核中的其他数据结构使用。因此，在同时初始化两端的同时，将必须对某一端的 Arc 指针进行获取可变引用的操作，即`get_mut_unchecked`接口。当 `Arc` 指针的引用计数不为 `1` 时，这一接口是非常不安全的，但是在当前情境下，我们使用这一接口进行`IPC` 对象的初始化，安全性是可以保证的。

### 单元测试

下面我们写一个单元测试，来验证我们写的`create`方法：

```
#[test]
    fn test_basics() {
        let (end0, end1) = Channel::create();
        assert!(Arc::ptr_eq(
            &end0.peer().unwrap().downcast_arc().unwrap(),
            &end1
        ));
        assert_eq!(end0.related_koid(), end1.id());

        drop(end1);
        assert_eq!(end0.peer().unwrap_err(), ZxError::PEER_CLOSED);
        assert_eq!(end0.related_koid(), 0);
    }
```

### 实现数据传输

Channel中的数据传输，可以理解为`MessagePacket`在两个端点之间的传输，那么谁可以读写消息呢？

有一个句柄与通道端点相关联，持有该句柄的进程被视为所有者（owner）。所以是（持有与通道端点关联句柄的）进程可以读取或写入消息，或将通道端点发送到另一个进程。

当`MessagePacket`被写入通道时，它们会从发送进程中删除。当从通道读取`MessagePacket`时，`MessagePacket`的句柄被添加到接收进程中。

#### read

获取当前端点的`recv_queue`，从队头中读取一条消息，如果能读取到消息，返回`Ok`，否则返回错误信息。

```
pub fn read(&self) -> ZxResult<T> {
        let mut recv_queue = self.recv_queue.lock();
        if let Some(_msg) = recv_queue.front() {
            let msg = recv_queue.pop_front().unwrap();
            return Ok(msg);
        }
        if self.peer_closed() {
            Err(ZxError::PEER_CLOSED)
        } else {
            Err(ZxError::SHOULD_WAIT)
        }
    }
```

#### write

先获取当前端点对应的另一个端点的`Weak`指针，通过`upgrade`接口升级为`Arc`指针，从而获取到对应的结构体对象。在它的`recv_queue`队尾push一个`MessagePacket`。

```
pub fn write(&self, msg: T) -> ZxResult {
        let peer = self.peer.upgrade().ok_or(ZxError::PEER_CLOSED)?;
        peer.push_general(msg);
        Ok(())
    }
fn push_general(&self, msg: T) {
        let mut send_queue = self.recv_queue.lock();
        send_queue.push_back(msg);
    }
```

### 单元测试

下面我们写一个单元测试，验证我们上面写的`read`和`write`两个方法：

```
#[test]
    fn read_write() {
        let (channel0, channel1) = Channel::create();
        // write a message to each other
        channel0
            .write(MessagePacket {
                data: Vec::from("hello 1"),
                handles: Vec::new(),
            })
            .unwrap();

        channel1
            .write(MessagePacket {
                data: Vec::from("hello 0"),
                handles: Vec::new(),
            })
            .unwrap();

		// read message should success
        let recv_msg = channel1.read().unwrap();
        assert_eq!(recv_msg.data.as_slice(), b"hello 1");
        assert!(recv_msg.handles.is_empty());

        let recv_msg = channel0.read().unwrap();
        assert_eq!(recv_msg.data.as_slice(), b"hello 0");
        assert!(recv_msg.handles.is_empty());

        // read more message should fail.
        assert_eq!(channel0.read().err(), Some(ZxError::SHOULD_WAIT));
        assert_eq!(channel1.read().err(), Some(ZxError::SHOULD_WAIT));
    }
```

## 总结

在这一节中我们实现了唯一一个可以传递句柄的对象传输器——Channel，我们先了解的Zircon中主要的IPC内核对象，再介绍了Channel如何创建和实现read和write函数的细节。

本章我们学习了中最核心的几个内核对象，在下一章中，我们将学习`Zircon`的任务管理体系和进程、线程管理的对象。
